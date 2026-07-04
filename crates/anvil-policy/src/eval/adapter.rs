//! EVAL-002 — the concrete harness adapter.
//!
//! [`PolicyEvalAdapter`] is the framework adapter that runs a suite and
//! normalises its output into an [`EvalRunSummary`]. It binds to the **frozen**
//! `anvil policy eval --json` v1 contract: it reads only the gate-critical
//! fields (`schema_version`, `policy`, `query`, `findings`, `exit_code`) and
//! ignores the diagnostic fields (`value`, `coverage`, `trace`, `why`), which
//! the contract explicitly allows to change shape without a major bump.
//!
//! Execution is pluggable via [`PolicyEvalRunner`]: the adapter owns the
//! normalisation, the runner owns *how* the raw document is produced. The
//! default [`SubprocessRunner`] shells out to the `anvil` binary; tests inject a
//! fake runner so normalisation is verified without a process.

use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use serde::Deserialize;
use wait_timeout::ChildExt;

use super::port::{EvalHarnessError, EvalHarnessPort, EvalRunSummary, EvalSuite};

/// The major version of the `policy-eval-output-v1` contract this build speaks.
const SUPPORTED_SCHEMA_MAJOR: u64 = 1;

/// Produces the raw `anvil policy eval --json` document for a suite. Splitting
/// this from normalisation keeps the adapter's parsing unit-testable without a
/// subprocess, and lets a CI harness swap in an in-process runner.
pub trait PolicyEvalRunner {
    /// Run the suite and return the raw JSON document (stdout of
    /// `anvil policy eval --json`).
    fn eval_json(&self, suite: &EvalSuite) -> Result<String, EvalHarnessError>;
}

/// The concrete harness adapter: a [`PolicyEvalRunner`] plus the v1
/// normalisation. Generic over the runner so the execution mechanism is
/// swappable.
pub struct PolicyEvalAdapter<R: PolicyEvalRunner> {
    runner: R,
}

impl<R: PolicyEvalRunner> PolicyEvalAdapter<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R: PolicyEvalRunner> EvalHarnessPort for PolicyEvalAdapter<R> {
    fn run_suite(&self, suite: &EvalSuite) -> Result<EvalRunSummary, EvalHarnessError> {
        let raw = self.runner.eval_json(suite)?;
        normalise(&suite.name, &raw)
    }
}

/// The frozen subset of the v1 wire document. `#[serde(deny_unknown_fields)]`
/// is deliberately **not** used: the contract reserves the right to add
/// diagnostic fields, and a forward-compatible consumer ignores them.
#[derive(Debug, Deserialize)]
struct RawEvalOutput {
    schema_version: String,
    policy: String,
    query: String,
    /// The contract guarantees an empty array (never omitted) when there are no
    /// findings; `default` is belt-and-braces for a malformed producer.
    #[serde(default)]
    findings: Vec<super::port::EvalFinding>,
    exit_code: i32,
}

/// Normalise a raw `anvil policy eval --json` document into an
/// [`EvalRunSummary`], enforcing the frozen v1 contract.
pub fn normalise(suite: &str, raw: &str) -> Result<EvalRunSummary, EvalHarnessError> {
    let parsed: RawEvalOutput =
        serde_json::from_str(raw).map_err(|e| EvalHarnessError::Contract {
            suite: suite.to_string(),
            detail: e.to_string(),
        })?;

    let major = schema_major(&parsed.schema_version).ok_or_else(|| EvalHarnessError::Contract {
        suite: suite.to_string(),
        detail: format!("unparseable schema_version `{}`", parsed.schema_version),
    })?;
    if major != SUPPORTED_SCHEMA_MAJOR {
        return Err(EvalHarnessError::UnsupportedSchema {
            suite: suite.to_string(),
            version: parsed.schema_version,
        });
    }

    Ok(EvalRunSummary {
        suite: suite.to_string(),
        schema_version: parsed.schema_version,
        policy: parsed.policy,
        query: parsed.query,
        findings: parsed.findings,
        exit_code: parsed.exit_code,
    })
}

/// Parse the leading major component of a `"X.Y.Z"` semver string.
fn schema_major(version: &str) -> Option<u64> {
    version.split('.').next()?.parse().ok()
}

/// Default per-suite wall-clock budget: a hung evaluation past this is an
/// execution error rather than a silent stall.
const DEFAULT_SUITE_TIMEOUT: Duration = Duration::from_mins(1);

/// A [`PolicyEvalRunner`] that shells out to an `anvil` executable, invoking
/// `anvil policy eval --json` for the suite. This is the production execution
/// path; the args mirror the `EvalArgs` clap surface in
/// `anvil-cli/src/commands/policy/eval.rs`.
pub struct SubprocessRunner {
    /// The `anvil` executable to invoke (a path or a PATH-resolved name).
    program: PathBuf,
    /// Wall-clock budget per suite; a hung evaluation is an execution error.
    timeout: Duration,
}

impl SubprocessRunner {
    /// Run via the given `anvil` executable.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            timeout: DEFAULT_SUITE_TIMEOUT,
        }
    }

    /// Override the per-suite timeout (default 60s).
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Build the argument vector for a suite — extracted so the wiring is
    /// testable without spawning a process.
    fn args(suite: &EvalSuite) -> Vec<String> {
        let mut args = vec![
            "policy".into(),
            "eval".into(),
            "--json".into(),
            suite.policy.display().to_string(),
            "--query".into(),
            suite.query.clone(),
        ];
        if let Some(input) = &suite.input {
            args.push("--input".into());
            args.push(input.display().to_string());
        }
        args
    }
}

/// Render a non-`{0,1}` process exit for the EVALCI-003 error detail. On Unix a
/// signal death (`code()` is `None`) surfaces the signal number so an infra/OPA
/// crash is actionable from the logs, rather than an opaque `signal`.
#[cfg(unix)]
fn describe_exit(status: std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    status.code().map_or_else(
        || {
            status
                .signal()
                .map_or_else(|| "signal".to_string(), |s| format!("signal {s}"))
        },
        |c| c.to_string(),
    )
}

#[cfg(not(unix))]
fn describe_exit(status: std::process::ExitStatus) -> String {
    status
        .code()
        .map_or_else(|| "signal".to_string(), |c| c.to_string())
}

/// Retry budget on `ETXTBSY` before giving up. Bounds worst-case added latency
/// to ~2+4+8+16+32 = ~62 ms — negligible beside the per-suite timeout.
const ETXTBSY_MAX_RETRIES: u32 = 5;

/// Spawn a child, retrying on `ETXTBSY` ("Text file busy", os error 26).
///
/// Guards the classic multithreaded fork+exec race: `fork()` clones the whole
/// process fd table, so a sibling thread mid-`write` on *any* freshly-created
/// executable can leak a write-mode fd into another thread's forked child. The
/// kernel's `deny_write_access` check is inode-scoped (the target file's
/// `i_writecount`), not path-scoped, so this trips even across different paths,
/// and a write-then-rename does *not* help — rename preserves the inode and its
/// write-count. The condition is transient and self-healing (the sibling child
/// reaches its own exec in microseconds, closing the inherited CLOEXEC fd), so a
/// bounded retry is the ecosystem-standard fix here, not a mask for a real bug.
/// Mirrors golang/go#22315 and rust-lang/rust#114554.
fn spawn_with_etxtbsy_retry(
    mut spawn: impl FnMut() -> std::io::Result<std::process::Child>,
    max_attempts: u32,
) -> std::io::Result<std::process::Child> {
    /// `ETXTBSY` — "Text file busy". A raw constant avoids a `libc` dependency
    /// for a single integer.
    const ETXTBSY: i32 = 26;
    let mut attempt = 0;
    loop {
        match spawn() {
            Err(e) if e.raw_os_error() == Some(ETXTBSY) && attempt < max_attempts => {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(1u64 << attempt));
            }
            other => return other,
        }
    }
}

impl PolicyEvalRunner for SubprocessRunner {
    fn eval_json(&self, suite: &EvalSuite) -> Result<String, EvalHarnessError> {
        let exec_err =
            |source: Box<dyn std::error::Error + Send + Sync>| EvalHarnessError::Execution {
                suite: suite.name.clone(),
                source,
            };

        // Reconstruct the `Command` per attempt so each spawn gets fresh stdio
        // pipes. The retry guards the ETXTBSY fork+exec race (see
        // `spawn_with_etxtbsy_retry`), which is why the fixture-exec tests below
        // no longer flake under parallel runs.
        let mut child = spawn_with_etxtbsy_retry(
            || {
                Command::new(&self.program)
                    .args(Self::args(suite))
                    // EVALCI-002: null the child's stdin so a future upstream
                    // prompt (an auth or licence gate on `anvil policy eval`)
                    // reads EOF and fails fast, rather than blocking on an
                    // unanswerable prompt until the per-suite timeout burns the
                    // whole 60s budget.
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
            },
            ETXTBSY_MAX_RETRIES,
        )
        .map_err(|e| exec_err(Box::new(e)))?;

        // Drain stdout/stderr on dedicated threads so a full OS pipe buffer
        // cannot wedge the child (a findings-heavy policy easily exceeds the
        // 64 KiB pipe buffer), and so we use a *single* wait path. Calling
        // `wait_timeout` then `wait_with_output` on the same `Child` is
        // undefined — on Linux the second wait can return a bogus exit status,
        // on macOS it surfaces ECHILD — exactly the trap `opa.rs` documents and
        // avoids.
        let mut stdout_handle = child
            .stdout
            .take()
            .expect("stdout is piped above; taken once");
        let mut stderr_handle = child
            .stderr
            .take()
            .expect("stderr is piped above; taken once");
        let stdout_reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
            let mut buf = Vec::new();
            stdout_handle.read_to_end(&mut buf)?;
            Ok(buf)
        });
        let stderr_reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
            let mut buf = Vec::new();
            stderr_handle.read_to_end(&mut buf)?;
            Ok(buf)
        });

        // `anvil policy eval` exits `0` (clean) or `1` (a *blocking* finding —
        // a valid gate verdict, not a runner failure) and prints the JSON
        // document in both cases; the verdict comes from the parsed `exit_code`
        // field. Any *other* code is classified as an execution error below
        // (EVALCI-003).
        let Some(status) = child
            .wait_timeout(self.timeout)
            .map_err(|e| exec_err(Box::new(e)))?
        else {
            let _ = child.kill();
            let _ = child.wait();
            // The child's death (from `kill`) closes its pipe write ends,
            // unblocking the readers; join to avoid dangling threads but discard
            // the payloads. A grandchild (e.g. OPA) that inherited the write end
            // and survives the kill can delay the join until it too exits.
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(exec_err(
                format!("suite timed out after {:?}", self.timeout).into(),
            ));
        };

        let stdout_bytes = stdout_reader
            .join()
            .map_err(|e| exec_err(format!("stdout reader thread panicked: {e:?}").into()))?
            .map_err(|e| exec_err(Box::new(e)))?;
        let stderr_bytes = stderr_reader
            .join()
            .map_err(|e| exec_err(format!("stderr reader thread panicked: {e:?}").into()))?
            .map_err(|e| exec_err(Box::new(e)))?;

        // EVALCI-003: the frozen v1 contract defines exactly two *process* exit
        // verdicts — `0` (clean) and `1` (blocking finding). Any other code — or
        // a signal death (`code()` is `None`) — is an OPA/infra failure: an
        // execution error, not a trust regression. Classifying it as such stops
        // a broken toolchain (for example an `opa` invocation exiting 2) from
        // masquerading as a passing or failing gate and false-blocking main.
        //
        // NB this is the *process* status, not the JSON `exit_code` field.
        // `EvalRegressionReport::regressed` treats a `1 -> 2` change in the
        // parsed `exit_code` field as an escalation regression, but under this
        // contract a producer that exits 2 never yields a parsed summary — the
        // council settled process exit >=2 as infra, not a trust escalation. If
        // a future contract major assigns meaning to exit 2, revisit this.
        if !matches!(status.code(), Some(0 | 1)) {
            let stderr = String::from_utf8_lossy(&stderr_bytes);
            let stderr = stderr.trim();
            let code = describe_exit(status);
            let detail = if stderr.is_empty() {
                format!("suite exited with status {code} (expected 0 or 1)")
            } else {
                format!("suite exited with status {code} (expected 0 or 1); stderr: {stderr}")
            };
            return Err(exec_err(detail.into()));
        }

        // Empty stdout means the process produced no document (a crash/panic):
        // surface that with the captured stderr rather than letting it decay
        // into an opaque "EOF while parsing" contract error downstream.
        if stdout_bytes.is_empty() {
            let stderr = String::from_utf8_lossy(&stderr_bytes);
            let stderr = stderr.trim();
            let detail = if stderr.is_empty() {
                "produced no output".to_string()
            } else {
                format!("produced no output; stderr: {stderr}")
            };
            return Err(exec_err(detail.into()));
        }

        String::from_utf8(stdout_bytes).map_err(|e| exec_err(Box::new(e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::port::EvalSeverity;

    /// The frozen v1 snapshot fixture from `docs/specs/policy-eval-output-v1.md`
    /// — a blocking evaluation with two findings, diagnostic fields omitted.
    const V1_FIXTURE: &str = r#"{
        "schema_version": "1.0.0",
        "policy": "policies/arch_boundary.rego",
        "query": "data.anvil.arch.findings",
        "findings": [
            {
                "severity": "error",
                "message": "ui imports db directly",
                "from": "src/ui.rs",
                "to": "src/db.rs",
                "fingerprint": "abc123",
                "is_new_edge": true,
                "baselined": false
            },
            {
                "severity": "warning",
                "message": "consider extracting a port"
            }
        ],
        "exit_code": 1
    }"#;

    fn suite() -> EvalSuite {
        EvalSuite {
            name: "arch".into(),
            policy: "policies/arch_boundary.rego".into(),
            input: None,
            query: "data.anvil.arch.findings".into(),
        }
    }

    struct FakeRunner(String);
    impl PolicyEvalRunner for FakeRunner {
        fn eval_json(&self, _suite: &EvalSuite) -> Result<String, EvalHarnessError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn eval_harness_adapter_normalises_v1_fixture() {
        let adapter = PolicyEvalAdapter::new(FakeRunner(V1_FIXTURE.into()));
        let summary = adapter.run_suite(&suite()).expect("normalise");

        assert_eq!(summary.suite, "arch");
        assert_eq!(summary.schema_version, "1.0.0");
        assert_eq!(summary.policy, "policies/arch_boundary.rego");
        assert_eq!(summary.exit_code, 1);
        assert!(!summary.passed());
        assert_eq!(summary.error_count(), 1);
        assert_eq!(summary.warning_count(), 1);

        let first = &summary.findings[0];
        assert_eq!(first.severity, EvalSeverity::Error);
        assert_eq!(first.from.as_deref(), Some("src/ui.rs"));
        assert_eq!(first.fingerprint.as_deref(), Some("abc123"));
        // The second finding's absent edge/fingerprint normalise to None.
        assert_eq!(summary.findings[1].from, None);
        assert_eq!(summary.findings[1].fingerprint, None);
    }

    #[test]
    fn eval_harness_adapter_ignores_diagnostic_fields() {
        // Diagnostic fields are not part of the contract; their presence must
        // not break normalisation.
        let raw = r#"{
            "schema_version": "1.0.0",
            "policy": "p.rego",
            "query": "data",
            "findings": [],
            "exit_code": 0,
            "value": {"anything": [1,2,3]},
            "coverage": {"covered": [1]},
            "trace": {"steps": 9},
            "why": 0
        }"#;
        let summary = normalise("s", raw).expect("normalise");
        assert!(summary.passed());
        assert!(summary.findings.is_empty());
    }

    #[test]
    fn eval_harness_adapter_clean_run_passes() {
        let raw =
            r#"{"schema_version":"1.0.0","policy":"p","query":"q","findings":[],"exit_code":0}"#;
        let summary = normalise("s", raw).expect("normalise");
        assert!(summary.passed());
        assert_eq!(summary.error_count(), 0);
    }

    #[test]
    fn eval_harness_adapter_rejects_future_major_schema() {
        let raw =
            r#"{"schema_version":"2.0.0","policy":"p","query":"q","findings":[],"exit_code":0}"#;
        let err = normalise("s", raw).expect_err("unsupported");
        assert!(
            matches!(err, EvalHarnessError::UnsupportedSchema { version, .. } if version == "2.0.0")
        );
    }

    #[test]
    fn eval_harness_adapter_accepts_forward_minor_schema() {
        // A 1.x minor bump is additive — still v1, still consumable.
        let raw =
            r#"{"schema_version":"1.4.0","policy":"p","query":"q","findings":[],"exit_code":0}"#;
        assert!(normalise("s", raw).is_ok());
    }

    #[test]
    fn eval_harness_adapter_rejects_malformed_output() {
        let err = normalise("s", "not json at all").expect_err("contract");
        assert!(matches!(err, EvalHarnessError::Contract { .. }));
    }

    #[test]
    fn eval_harness_adapter_missing_frozen_field_is_a_contract_error() {
        // No `exit_code` — a frozen field — must be a contract violation.
        let raw = r#"{"schema_version":"1.0.0","policy":"p","query":"q","findings":[]}"#;
        let err = normalise("s", raw).expect_err("contract");
        assert!(matches!(err, EvalHarnessError::Contract { .. }));
    }

    #[test]
    fn spawn_retry_gives_up_after_max_attempts() {
        // A spawn that *always* fails with ETXTBSY (os error 26) must exhaust
        // exactly `max_attempts` retries and then surface the last error, not
        // loop forever.
        let mut calls = 0u32;
        let err = spawn_with_etxtbsy_retry(
            || {
                calls += 1;
                Err::<std::process::Child, _>(std::io::Error::from_raw_os_error(26))
            },
            3,
        )
        .expect_err("persistent ETXTBSY should surface after exhausting retries");
        assert_eq!(err.raw_os_error(), Some(26), "last error is preserved");
        // Initial attempt + 3 retries.
        assert_eq!(calls, 4, "one initial call plus max_attempts retries");
    }

    #[test]
    fn spawn_retry_does_not_retry_non_etxtbsy() {
        // A permanent, unrelated error (ENOENT, os error 2 — "file not found")
        // must propagate on the first attempt: retrying it would only slow real
        // failures down for no benefit.
        let mut calls = 0u32;
        let err = spawn_with_etxtbsy_retry(
            || {
                calls += 1;
                Err::<std::process::Child, _>(std::io::Error::from_raw_os_error(2))
            },
            5,
        )
        .expect_err("ENOENT must not be retried");
        assert_eq!(err.raw_os_error(), Some(2));
        assert_eq!(calls, 1, "a non-ETXTBSY error is not retried");
    }

    #[cfg(unix)]
    #[test]
    fn spawn_retry_recovers_after_transient_etxtbsy() {
        // The whole point: a transient ETXTBSY that clears must resolve to a
        // successful spawn. Fail twice with os error 26, then spawn a real
        // (trivial) child on the third attempt.
        let mut calls = 0u32;
        let mut child = spawn_with_etxtbsy_retry(
            || {
                calls += 1;
                if calls < 3 {
                    Err(std::io::Error::from_raw_os_error(26))
                } else {
                    Command::new("true").spawn()
                }
            },
            5,
        )
        .expect("a transient ETXTBSY should recover once it clears");
        let _ = child.wait();
        assert_eq!(calls, 3, "retried twice, then succeeded");
    }

    #[test]
    fn eval_harness_adapter_subprocess_arg_wiring() {
        let with_input = EvalSuite {
            name: "s".into(),
            policy: "p.rego".into(),
            input: Some("in.json".into()),
            query: "data.x".into(),
        };
        let args = SubprocessRunner::args(&with_input);
        assert_eq!(
            args,
            vec![
                "policy", "eval", "--json", "p.rego", "--query", "data.x", "--input", "in.json"
            ]
        );
        // Without an input the `--input` pair is omitted.
        let no_input = suite();
        let args = SubprocessRunner::args(&no_input);
        assert!(!args.iter().any(|a| a == "--input"));
    }

    /// Write an executable shell script that ignores its args and runs `body`,
    /// returning its path (kept alive by the returned `TempDir`).
    #[cfg(unix)]
    fn script(body: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::TempDir::new().expect("tmp");
        let path = dir.path().join("fake-anvil.sh");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        (dir, path)
    }

    #[cfg(unix)]
    #[test]
    fn eval_harness_adapter_subprocess_drains_large_output_without_deadlock() {
        // A document far larger than the 64 KiB pipe buffer must not wedge the
        // child — the regression guard for the wait/drain rewrite.
        let big_message = "x".repeat(200_000);
        let json = format!(
            r#"{{"schema_version":"1.0.0","policy":"p","query":"q","findings":[{{"severity":"warning","message":"{big_message}"}}],"exit_code":0}}"#
        );
        // `printf %s` avoids the shell mangling the JSON; single-quote-escape.
        let (_dir, path) = script(&format!("cat <<'EOF'\n{json}\nEOF"));
        let runner = SubprocessRunner::new(path).with_timeout(Duration::from_secs(10));
        let out = runner.eval_json(&suite()).expect("eval_json");
        assert!(out.len() > 64 * 1024, "got {} bytes", out.len());
        let summary = normalise("arch", &out).expect("normalise large output");
        assert_eq!(summary.findings.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn eval_harness_adapter_subprocess_times_out_on_hanging_child() {
        // `exec` so the shell replaces itself with `sleep` — otherwise `sleep`
        // is a grandchild that survives killing the shell and keeps the stdout
        // pipe open, blocking the reader thread until it exits.
        let (_dir, path) = script("exec sleep 30");
        let runner = SubprocessRunner::new(path).with_timeout(Duration::from_millis(200));
        let err = runner.eval_json(&suite()).expect_err("should time out");
        assert!(matches!(err, EvalHarnessError::Execution { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn eval_harness_adapter_subprocess_empty_stdout_surfaces_stderr() {
        // A crashing child (empty stdout, message on stderr) yields a
        // diagnosable execution error, not an opaque contract error.
        let (_dir, path) = script("echo 'boom: engine panicked' 1>&2\nexit 101");
        let runner = SubprocessRunner::new(path);
        let err = runner.eval_json(&suite()).expect_err("crash");
        let msg = err.to_string();
        assert!(msg.contains("boom: engine panicked"), "stderr lost: {msg}");
    }

    #[cfg(unix)]
    #[test]
    fn eval_harness_adapter_subprocess_null_stdin() {
        // EVALCI-002: `cat` copies stdin to stdout; with stdin nulled it reads
        // immediate EOF and exits producing no output — proving a child that
        // waits on stdin (a future auth/licence prompt) cannot hang the suite.
        // `exec` so the shell is replaced by `cat` (no surviving grandchild).
        // Without the null redirect an inherited terminal stdin would instead
        // block `cat` until the timeout, surfacing as "timed out".
        let (_dir, path) = script("exec cat");
        let runner = SubprocessRunner::new(path).with_timeout(Duration::from_secs(2));
        let err = runner
            .eval_json(&suite())
            .expect_err("cat with null stdin yields empty output");
        let msg = err.to_string();
        assert!(
            msg.contains("produced no output"),
            "expected clean EOF: {msg}"
        );
        assert!(
            !msg.contains("timed out"),
            "stdin not nulled — child hung: {msg}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn eval_harness_exit_code_classification() {
        // EVALCI-003: a valid JSON document paired with a non-{0,1} process exit
        // (an OPA/infra failure) is an execution error, not a trust regression.
        let clean =
            r#"{"schema_version":"1.0.0","policy":"p","query":"q","findings":[],"exit_code":0}"#;
        let (_dir, path) = script(&format!("printf '%s' '{clean}'\nexit 2"));
        let err = SubprocessRunner::new(path)
            .eval_json(&suite())
            .expect_err("exit 2 is an execution error");
        assert!(matches!(err, EvalHarnessError::Execution { .. }));
        assert!(
            err.to_string().contains("status 2"),
            "status surfaced: {err}"
        );

        // The two gate verdicts still parse cleanly.
        let (_d0, p0) = script(&format!("printf '%s' '{clean}'\nexit 0"));
        assert!(
            SubprocessRunner::new(p0).eval_json(&suite()).is_ok(),
            "exit 0"
        );
        let blocking = r#"{"schema_version":"1.0.0","policy":"p","query":"q","findings":[{"severity":"error","message":"x"}],"exit_code":1}"#;
        let (_d1, p1) = script(&format!("printf '%s' '{blocking}'\nexit 1"));
        assert!(
            SubprocessRunner::new(p1).eval_json(&suite()).is_ok(),
            "exit 1"
        );

        // A signal death surfaces the signal number so an infra crash is
        // debuggable (SIGTERM = 15).
        let (_ds, ps) = script("kill -TERM $$");
        let err = SubprocessRunner::new(ps)
            .eval_json(&suite())
            .expect_err("signal death is an execution error");
        assert!(
            err.to_string().contains("signal 15"),
            "signal surfaced: {err}"
        );
    }
}
