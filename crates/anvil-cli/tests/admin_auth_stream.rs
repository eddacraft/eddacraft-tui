//! CIB-052: admin `--json` auth-error envelope stream contract.
//!
//! `admin.rs::print_auth_required` emitted its JSON-shaped auth error to
//! **stderr** (as did the `plan dashboard` gate refusal that mirrors it —
//! pinned in `plan_dashboard_gate.rs`). The stream policy
//! (`docs/guides/cli-output-streams.md`) puts machine-readable output on
//! stdout — the same contract CIB-049 (PR #2474) applied to the main
//! pre-dispatch auth gate's `--json` envelopes. These tests pin the admin
//! surface to that policy:
//!
//! - under `--json`, the auth-error envelope lands on stdout and stderr
//!   carries no JSON;
//! - in plain-text mode, the human-readable message stays on stderr and
//!   stdout stays empty;
//! - the exit code (`EXIT_AUTH_REQUIRED` = 3) is unchanged in both modes.
//!
//! ## Environment isolation
//!
//! Admin authenticates via `ANVIL_ADMIN_KEY` or a stored credential source
//! at `dirs::config_dir()/anvil/admin-auth.json` (XDG on Linux, `HOME`
//! fallback elsewhere). Each test removes the env var and points `HOME` /
//! `XDG_CONFIG_HOME` at an empty tempdir so no developer-machine
//! credential can leak in and the auth-required path is deterministic.

use std::process::{Command, Output};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Exit code for the auth-required sentinel (`EXIT_AUTH_REQUIRED` in
/// `main.rs`).
const EXIT_AUTH_REQUIRED: i32 = 3;

/// Run `anvil admin list` (optionally with `--json`) in an environment
/// where no admin credential can resolve.
fn run_admin_list_unauthenticated(json: bool) -> Output {
    let home = tempfile::tempdir().unwrap();

    let mut cmd = Command::new(ANVIL_BIN);
    if json {
        cmd.arg("--json");
    }
    cmd.arg("admin")
        .arg("list")
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("xdg"))
        .env_remove("ANVIL_ADMIN_KEY")
        .env_remove("ANVIL_DEV")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

/// CIB-052: the `--json` auth-error envelope is structured output and
/// must land on stdout per the stream policy, with the existing exit
/// code and no JSON on stderr.
#[test]
fn admin_json_auth_error_envelope_lands_on_stdout() {
    let output = run_admin_list_unauthenticated(true);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(EXIT_AUTH_REQUIRED),
        "exit code must stay EXIT_AUTH_REQUIRED: stdout=\n{stdout}\nstderr=\n{stderr}",
    );

    let envelope: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!(
            "expected the auth-error JSON envelope on stdout ({err}): \
             stdout=\n{stdout}\nstderr=\n{stderr}"
        )
    });
    assert_eq!(
        envelope["error"], "authentication_required",
        "unexpected envelope shape: stdout=\n{stdout}",
    );
    assert!(
        envelope["detail"].is_string(),
        "envelope must carry a detail string: stdout=\n{stdout}",
    );

    assert!(
        !stderr.contains("\"error\""),
        "stderr must carry no JSON envelope under --json: stderr=\n{stderr}",
    );
}

/// The plain-text auth-error message is human-readable diagnostics and
/// stays on stderr (stream policy unchanged for text mode), with the
/// same exit code and nothing on stdout.
#[test]
fn admin_plain_auth_error_stays_on_stderr() {
    let output = run_admin_list_unauthenticated(false);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(EXIT_AUTH_REQUIRED),
        "exit code must stay EXIT_AUTH_REQUIRED: stdout=\n{stdout}\nstderr=\n{stderr}",
    );
    assert!(
        stderr.contains("Authentication required"),
        "plain-text message must stay on stderr: stderr=\n{stderr}",
    );
    assert!(
        stdout.trim().is_empty(),
        "plain-text mode must not write to stdout: stdout=\n{stdout}",
    );
}
