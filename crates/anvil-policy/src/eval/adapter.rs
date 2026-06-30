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

impl PolicyEvalRunner for SubprocessRunner {
    fn eval_json(&self, suite: &EvalSuite) -> Result<String, EvalHarnessError> {
        let exec_err =
            |source: Box<dyn std::error::Error + Send + Sync>| EvalHarnessError::Execution {
                suite: suite.name.clone(),
                source,
            };

        let mut child = Command::new(&self.program)
            .args(Self::args(suite))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| exec_err(Box::new(e)))?;

        // `anvil policy eval` exits non-zero on a *blocking* finding — that is a
        // valid result, not a runner failure — so the exit status is ignored
        // here; the verdict comes from the parsed `exit_code` field.
        let status = child
            .wait_timeout(self.timeout)
            .map_err(|e| exec_err(Box::new(e)))?;
        if status.is_none() {
            let _ = child.kill();
            return Err(exec_err(
                format!("suite timed out after {:?}", self.timeout).into(),
            ));
        }

        let output = child
            .wait_with_output()
            .map_err(|e| exec_err(Box::new(e)))?;
        String::from_utf8(output.stdout).map_err(|e| exec_err(Box::new(e)))
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
}
