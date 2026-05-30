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

use std::path::PathBuf;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const SKIP_EXIT_CODE: i32 = 77;

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
    // Match the rest of the test suite's hygiene env. ANVIL_DEV
    // suppresses dev-only welcome flows; ANVIL_SKIP_WELCOME guards
    // against the welcome screen reaching for state. Neither of
    // these is a network-touching path on its own; they exist to
    // keep the test bounded.
    cmd.env("ANVIL_DEV", "1");
    cmd.env("ANVIL_SKIP_WELCOME", "1");
    let out = cmd.output().expect("failed to spawn harness");
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
    cmd.env_remove("ANVIL_DEV");
    cmd.env_remove("ANVIL_LICENSE");
    cmd.env("ANVIL_SKIP_WELCOME", "1");
    cmd.env("XDG_CONFIG_HOME", xdg_config_home);
    let out = cmd.output().expect("failed to spawn harness");
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
