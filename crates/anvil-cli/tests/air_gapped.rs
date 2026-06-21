//! MLP-017: air-gapped operation guarantee scaffold.
//!
//! Confirms that the read-only / activation-adjacent commands ship
//! today survive being run with no network access. This is the v1
//! scaffold for the release-gate guarantee that ADR-036 §D-3 calls
//! out: `anvil start`, `anvil baseline`, `anvil intercept ensure`,
//! all `anvil hook` subcommands, and `anvil audit` MUST make zero
//! network calls in normal operation.
//!
//! The scaffold lives here and grows as each command lands:
//!
//! - **v1 (this PR):** the `anvil version --offline` subcommand
//!   succeeds under the network-blocked harness, and the read-only
//!   `anvil status --verify --json` path exits cleanly (any code;
//!   not killed by signal). These are the currently-shipping
//!   commands in the MLP/INTL slate that legitimately run without
//!   network access. They give the harness real exercise and make
//!   the rule visible to anyone adding new commands.
//! - **As MLP-002 → -008 land:** each new command (`anvil hook
//!   pre-commit`, `anvil baseline`, etc.) adds a `#[test]` here.
//!   Reviewers should reject hook PRs that don't extend this test.
//!
//! The harness lives at `tools/test-harness/network-blocked/run.sh`
//! and uses Linux `unshare -n -r` to strip the network namespace.
//! On platforms where the primitive isn't available the script
//! exits 77 (skip) and the test treats that as "platform skip" so
//! local macOS development isn't blocked.

#![cfg(target_os = "linux")]

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const SKIP_EXIT_CODE: i32 = 77;

/// Maximum wall-clock for a single air-gapped harness invocation.
/// Generous enough for cargo/init/file-walk overhead on slow CI, yet
/// bounded so a command that wedges on a network resolver — the failure
/// this suite guards against — fails the test instead of hanging it
/// indefinitely (CLAWP-035).
const AIR_GAP_TIMEOUT: Duration = Duration::from_mins(1);

/// Spawn `cmd` and wait up to [`AIR_GAP_TIMEOUT`]. On timeout, kill the
/// child and panic so a wedged command under test surfaces as a clear,
/// bounded failure rather than an indefinite hang.
///
/// stdout/stderr are drained in dedicated threads (the same pattern as
/// `anvil-policy`'s OPA runner) so a child that fills the OS pipe buffer
/// can never wedge on `write(2)` — which would otherwise masquerade as a
/// timeout — and so there is a single wait path: the status comes from
/// `try_wait`, and `wait_with_output` is never called afterwards (calling
/// a second wait on an already-reaped child is the footgun documented at
/// `crates/anvil-policy/src/opa.rs`).
fn output_within_timeout(cmd: &mut Command) -> Output {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("failed to spawn harness");

    let mut out_pipe = child.stdout.take().expect("stdout piped above");
    let mut err_pipe = child.stderr.take().expect("stderr piped above");
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out_pipe.read_to_end(&mut buf);
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err_pipe.read_to_end(&mut buf);
        buf
    });

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait on harness child") {
            break status;
        }
        if start.elapsed() >= AIR_GAP_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "air-gapped harness exceeded {AIR_GAP_TIMEOUT:?} and was killed — \
                 the command under test likely wedged (e.g. on a network resolver)"
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let stdout = out_reader.join().expect("stdout reader thread");
    let stderr = err_reader.join().expect("stderr reader thread");
    Output {
        status,
        stdout,
        stderr,
    }
}

// KEEP-IN-SYNC: `harness_path` + `run_air_gapped` are mirrored in
// `crates/anvil-run/tests/air_gapped_run.rs` (anvil-run is a separate
// binary, so its air-gap test cannot share this crate's helper). If
// the harness protocol changes (skip code, env hygiene, path walk),
// update both. The duplication is ~25 lines — too small to justify a
// shared test-util crate.

/// Resolve the workspace-root path to the network-blocked harness.
///
/// `CARGO_MANIFEST_DIR` points at `crates/anvil-cli/`; the harness
/// lives at `<repo-root>/tools/test-harness/network-blocked/run.sh`,
/// so we go up two levels.
fn harness_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/anvil-cli -> crates
    p.pop(); // crates -> repo root
    p.push("tools");
    p.push("test-harness");
    p.push("network-blocked");
    p.push("run.sh");
    p
}

/// Run `args` through the air-gapped harness. Returns `None` if the
/// harness chose to skip (exit 77); otherwise the `Output` from the
/// command under test.
fn run_air_gapped(args: &[&str]) -> Option<std::process::Output> {
    let harness = harness_path();
    assert!(
        harness.exists(),
        "harness script not found at {}; expected to be checked in",
        harness.display(),
    );
    let mut cmd = Command::new(&harness);
    cmd.arg(ANVIL_BIN);
    cmd.args(args);
    // Close stdin explicitly. `Command::output` already nulls stdin,
    // but pinning it here means a command that reads stdin (e.g.
    // `hook pre-push`, which consumes git's pre-push contract) hits
    // EOF immediately and can never hang the test waiting for input.
    cmd.stdin(std::process::Stdio::null());
    // Match the rest of the test suite's hygiene env. ANVIL_DEV
    // suppresses dev-only welcome flows; ANVIL_SKIP_WELCOME guards
    // against the welcome screen reaching for state. Neither of
    // these is a network-touching path on its own; they exist to
    // keep the test bounded.
    cmd.env("ANVIL_DEV", "1");
    cmd.env("ANVIL_SKIP_WELCOME", "1");
    let out = output_within_timeout(&mut cmd);
    if out.status.code() == Some(SKIP_EXIT_CODE) {
        eprintln!(
            "air-gapped harness skipped:\nstderr={}",
            String::from_utf8_lossy(&out.stderr),
        );
        return None;
    }
    Some(out)
}

fn run_air_gapped_without_dev(
    args: &[&str],
    xdg_config_home: &std::path::Path,
) -> Option<std::process::Output> {
    let harness = harness_path();
    assert!(
        harness.exists(),
        "harness script not found at {}; expected to be checked in",
        harness.display(),
    );
    let mut cmd = Command::new(&harness);
    cmd.arg(ANVIL_BIN);
    cmd.args(args);
    cmd.stdin(std::process::Stdio::null());
    cmd.env_remove("ANVIL_DEV");
    cmd.env_remove("ANVIL_LICENSE");
    cmd.env("ANVIL_SKIP_WELCOME", "1");
    cmd.env("XDG_CONFIG_HOME", xdg_config_home);
    let out = output_within_timeout(&mut cmd);
    if out.status.code() == Some(SKIP_EXIT_CODE) {
        eprintln!(
            "air-gapped harness skipped:\nstderr={}",
            String::from_utf8_lossy(&out.stderr),
        );
        return None;
    }
    Some(out)
}

#[test]
fn anvil_version_offline_succeeds_with_no_network() {
    let Some(out) = run_air_gapped(&["--no-tui", "version", "--offline"]) else {
        return;
    };
    assert!(
        out.status.success(),
        "anvil version --offline failed under air-gap: stderr={}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("anvil "), "missing version line: {stdout}");
}

/// CLAWP-025: error markers that prove a network / auth / update
/// dependency was hit. `status --verify --json` runs the LOCAL
/// activation probe (see `commands/status.rs::run_verify` →
/// `activation::render_json`), so none of these may appear on either
/// stream — their presence means the air-gap contract was violated by
/// a code path reaching for the network or an auth refresh.
///
/// - `authentication_required` / `auth_check_failed` — the JSON
///   auth-gate envelopes (`main.rs`). Either means the auth path ran
///   instead of the local probe.
/// - `auth_error` — the generic auth marker the issue (#1754) names as
///   forbidden in the local-diagnostic shape.
/// - `device-flow` — a device-flow / token-refresh attempt that only
///   happens when the auth path reaches outward.
///
/// Broader network/timeout words (`network`, `connection`, `no response`,
/// …) are deliberately NOT scanned across the whole stream here: they can
/// appear benignly in normal output (e.g. "no network access"). They are
/// instead checked narrowly inside the structured `last_error` field (see
/// the `last_error` guard in the local-diagnostic-shape assertion below).
const NETWORK_AUTH_UPDATE_MARKERS: &[&str] = &[
    "authentication_required",
    "auth_check_failed",
    "auth_error",
    "device-flow",
];

/// Closed-set activation-state labels emitted by
/// `activation::state::ProtectionState::label` (the `state` field of
/// `status --verify --json`). The local probe can only produce one of
/// these; an off-vocabulary value would mean the JSON came from a
/// non-diagnostic code path.
const ACTIVATION_STATES: &[&str] = &[
    "protecting",
    "ready_restart_required",
    "watching",
    "needs_action",
    "unsupported",
    "error",
];

/// Assert that neither stdout nor stderr names a network / auth /
/// update error marker. Centralised so both air-gap tests pin the
/// same forbidden-marker set.
fn assert_no_network_auth_update_markers(stdout: &str, stderr: &str) {
    for marker in NETWORK_AUTH_UPDATE_MARKERS {
        assert!(
            !stdout.contains(marker),
            "air-gap violation: stdout contains network/auth/update marker `{marker}`; \
             status --verify must run the local probe only. stdout=\n{stdout}",
        );
        assert!(
            !stderr.contains(marker),
            "air-gap violation: stderr contains network/auth/update marker `{marker}`; \
             status --verify must run the local probe only. stderr=\n{stderr}",
        );
    }
}

/// Parse `stdout` as the `anvil status --verify --json` activation
/// diagnostic and assert the expected local-diagnostic shape, proving
/// the local probe path ran (rather than a network/auth code path that
/// times out before the budget yet still exits cleanly).
///
/// The shape is the stable contract from
/// `activation::render::render_json`: a JSON object whose `state` is
/// one of the closed-set activation-state labels, plus the
/// `config`/`watch`/`baseline_present` local-signal keys. A network
/// attempt cannot synthesise this object — it comes from local state
/// only — so a parse + shape match is positive evidence the air-gap
/// contract held.
fn assert_local_activation_diagnostic_shape(stdout: &str) {
    let value: serde_json::Value = serde_json::from_str(stdout).unwrap_or_else(|e| {
        panic!("status --verify --json stdout is not valid JSON ({e}): stdout=\n{stdout}")
    });
    let obj = value
        .as_object()
        .unwrap_or_else(|| panic!("status --verify --json output is not a JSON object: {value}"));

    // The local activation diagnostic always carries these keys (the
    // `render_json` contract). Their presence proves the diagnostic
    // was assembled from local state, not a degraded network/auth
    // fallthrough.
    for key in ["state", "config", "watch", "baseline_present"] {
        assert!(
            obj.contains_key(key),
            "local activation diagnostic missing `{key}` key — output is not the \
             local-probe shape: {value}",
        );
    }

    // `state` must be one of the closed-set activation-state labels.
    // An off-vocabulary value would mean the JSON came from somewhere
    // other than `ProtectionState::label`.
    let state = obj["state"]
        .as_str()
        .unwrap_or_else(|| panic!("`state` is not a string: {value}"));
    assert!(
        ACTIVATION_STATES.contains(&state),
        "`state` = `{state}` is not a closed-set activation state {ACTIVATION_STATES:?}; \
         output is not the local-probe shape: {value}",
    );

    // The local probe must not surface a network/auth error in its
    // structured `last_error` field. `null` (no error) or a non-
    // network local cause is acceptable; a network/auth/update token
    // there is the structured form of the air-gap violation.
    if let Some(last_error) = obj.get("last_error").and_then(serde_json::Value::as_str) {
        // "no response" catches `ProbeError::Timeout`, which renders as
        // "no response within Xs" rather than the literal word "timeout".
        for marker in [
            "network",
            "timed out",
            "timeout",
            "no response",
            "connection",
            "auth",
        ] {
            assert!(
                !last_error.to_lowercase().contains(marker),
                "local diagnostic `last_error` names a network/auth cause `{marker}`: \
                 {last_error}",
            );
        }
    }
}

#[test]
fn anvil_status_verify_json_exits_cleanly_with_no_network() {
    // `anvil status --verify --json` is the read-only activation
    // probe (LAUNCH-012). It must not depend on the network in
    // normal operation — the diagnostic comes from local state.
    let Some(out) = run_air_gapped(&["--no-tui", "--json", "status", "--verify"]) else {
        return;
    };
    // We don't enforce success here: `status --verify` legitimately
    // returns a non-zero exit when activation is incomplete (e.g.
    // running outside a workspace). What we DO enforce is that the
    // process exits cleanly under the air-gap rather than hanging on
    // a network resolver. Cleanly = any exit status; not segfault,
    // not killed-by-signal. Test name reflects that contract.
    assert!(
        out.status.code().is_some(),
        "anvil status --verify --json was killed by signal under air-gap: {:?}",
        out.status,
    );
    // CLAWP-025: "not killed by signal" passes even if a network
    // attempt was made and timed out before the test budget. Pin the
    // air-gap contract by inspecting OUTPUT CONTENT: the stdout must
    // be the local activation-diagnostic shape, and neither stream may
    // name a network/auth/update error marker.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_local_activation_diagnostic_shape(&stdout);
    assert_no_network_auth_update_markers(&stdout, &stderr);
}

#[test]
fn anvil_status_verify_json_skips_auth_refresh_with_expired_credentials() {
    let config_home = tempfile::tempdir().unwrap();
    let anvil_dir = config_home.path().join("anvil");
    std::fs::create_dir(&anvil_dir).expect("create anvil config dir");
    std::fs::write(
        anvil_dir.join("credentials.json"),
        r#"{
  "license": "expired-local-token",
  "refreshToken": "refresh-token-that-must-not-be-used",
  "email": "airgap@example.test",
  "expiresAt": "2000-01-01T00:00:00Z"
}
"#,
    )
    .expect("write expired credentials");

    let Some(out) = run_air_gapped_without_dev(
        &["--no-tui", "--json", "status", "--verify"],
        config_home.path(),
    ) else {
        return;
    };

    assert!(
        out.status.code().is_some(),
        "anvil status --verify --json was killed by signal under air-gap: {:?}",
        out.status,
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("authentication_required"),
        "status --verify should run the local probe instead of failing auth first: {stderr}",
    );
    // CLAWP-025: with expired credentials present, the temptation is
    // for the auth path to attempt a token refresh against the
    // network. Prove instead that stdout carries the local activation
    // diagnostic and that no stream names a network/auth/update error
    // marker — the only honest evidence the local probe ran without
    // reaching outward.
    assert_local_activation_diagnostic_shape(&stdout);
    assert_no_network_auth_update_markers(&stdout, &stderr);
}

/// CIB-049 (1): `anvil start --verify` is the read-only activation
/// probe sibling of `anvil status --verify` (`StartArgs::verify` is
/// documented as producing the same output). It must skip the auth
/// wall and run the local probe unauthenticated — air-gapped and
/// scripted consumers depend on it.
#[test]
fn anvil_start_verify_json_runs_local_probe_unauthenticated() {
    let config_home = tempfile::tempdir().unwrap();
    let Some(out) = run_air_gapped_without_dev(
        &["--no-tui", "--json", "start", "--verify"],
        config_home.path(),
    ) else {
        return;
    };

    assert!(
        out.status.code().is_some(),
        "anvil start --verify --json was killed by signal under air-gap: {:?}",
        out.status,
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // The local activation diagnostic on stdout is the positive
    // evidence the probe ran instead of the auth gate: the auth path
    // cannot synthesise this shape, and unauthenticated it would have
    // emitted an `authRequired` envelope and returned early.
    assert_local_activation_diagnostic_shape(&stdout);
    assert_no_network_auth_update_markers(&stdout, &stderr);
}

// #1705 (Council C-011): the air-gap runbook commits every core
// MLP/INTL protection command to making zero network calls, and its
// "How to extend the gate" section makes adding a `#[test]` here
// mandatory for each such command. The three below exercise the
// v0.7.0-beta surfaces that landed without coverage. Like the
// `status --verify` case above, success is not enforced — these
// commands legitimately return non-zero outside an activated
// workspace. What is enforced is that the process EXITS under the
// air-gap (`status.code().is_some()`) rather than hanging on a
// network resolver or being killed by signal.

/// `anvil audit-chain` (MLP-015 + Group K) walks commit history and
/// consults the local witness chain. It reads local git + witness
/// state only and must run off-network.
#[test]
fn anvil_audit_chain_exits_cleanly_with_no_network() {
    let Some(out) = run_air_gapped(&["--no-tui", "audit-chain"]) else {
        return;
    };
    assert!(
        out.status.code().is_some(),
        "anvil audit-chain was killed by signal under air-gap: {:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
}

/// `anvil l4-validate <range>` (MLP2-046) is the dedicated L4-policy
/// validator extracted from the pre-push hook. It walks the commit
/// range with `git rev-list` and runs the local rule engine — no
/// network. We pass the empty range `HEAD..HEAD` rather than
/// `HEAD~1..HEAD`: it is always valid (even on a shallow `--depth 1`
/// CI checkout where `HEAD~1` does not exist, which would make the
/// command fail for an unrelated git reason and pass this assertion
/// vacuously), keeps the walk bounded, and still exercises the
/// startup → git → rule-engine path that the air-gap contract covers.
#[test]
fn anvil_l4_validate_exits_cleanly_with_no_network() {
    let Some(out) = run_air_gapped(&["--no-tui", "l4-validate", "HEAD..HEAD"]) else {
        return;
    };
    assert!(
        out.status.code().is_some(),
        "anvil l4-validate was killed by signal under air-gap: {:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
}

/// `anvil report-fp <check-id> <file:line>` (OPSUP-007 / ADR-089) records a
/// false-positive report to the **local** Kindling sidecar only. ADR-089
/// makes the local record the destination — nothing leaves the machine — so
/// the command must complete off-network.
#[test]
fn anvil_report_fp_exits_cleanly_with_no_network() {
    let Some(out) = run_air_gapped(&["--no-tui", "report-fp", "ANV-CORE-001", "src/x.rs:1"]) else {
        return;
    };
    assert!(
        out.status.code().is_some(),
        "anvil report-fp was killed by signal under air-gap: {:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
}

/// `anvil hook pre-push` (MLP-004) reads git's pre-push stdin contract
/// and validates the pushed range against local policy. With no stdin
/// supplied (`Command::output` closes it) there are zero refs to walk,
/// so it returns quickly — and entirely off-network.
#[test]
fn anvil_hook_pre_push_exits_cleanly_with_no_network() {
    let Some(out) = run_air_gapped(&["--no-tui", "hook", "pre-push"]) else {
        return;
    };
    assert!(
        out.status.code().is_some(),
        "anvil hook pre-push was killed by signal under air-gap: {:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn harness_is_executable_and_checked_in() {
    // Defensive: if a reviewer accidentally drops the executable
    // bit on the harness, every subsequent test would silently
    // exit 126 (perms). Detect that early with an unambiguous
    // failure here.
    let harness = harness_path();
    assert!(
        harness.exists(),
        "harness missing at {}; MLP-017 scaffold has been damaged",
        harness.display(),
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&harness).expect("harness metadata");
        let mode = meta.permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "harness at {} is not executable (mode={mode:o})",
            harness.display(),
        );
    }
}
