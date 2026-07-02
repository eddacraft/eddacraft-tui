//! CIB-046: binary-level contract for the `anvil plan dashboard`
//! internal-developer gate.
//!
//! The unit tests in `feature_flags.rs` cover the gate *decision*; these
//! integration tests pin the full dispatch + exit-code wiring that the unit
//! tests cannot reach:
//! - gate closed (no `ANVIL_DEV`, no `ANVIL_ADMIN_KEY`) → exit 3
//!   (`EXIT_AUTH_REQUIRED`), so a refactor of the dispatch path or the
//!   `AuthRequired` error type cannot silently break the contract;
//! - gate open via `ANVIL_DEV=1` or a non-empty `ANVIL_ADMIN_KEY` → the gate
//!   is passed (the command does not exit 3).

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
/// `EXIT_AUTH_REQUIRED` in `main.rs`. Duplicated as a literal because the bin
/// crate exposes no library target for tests to import the constant from.
const EXIT_AUTH_REQUIRED: i32 = 3;

/// A command rooted inside the repo tree (so workspace discovery succeeds when
/// the gate is open) with a hygienic, prompt-free environment. The gate-state
/// env vars are removed by default; each test sets only what it needs.
fn plan_dashboard() -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(["plan", "dashboard", "--no-tui"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1")
        .env("ANVIL_LOG", "off")
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_ADMIN_KEY");
    cmd
}

#[test]
fn plan_dashboard_refused_when_gate_closed() {
    let output = plan_dashboard()
        .output()
        .expect("failed to invoke anvil binary");

    assert_eq!(
        output.status.code(),
        Some(EXIT_AUTH_REQUIRED),
        "closed gate must exit EXIT_AUTH_REQUIRED (3); stderr:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("internal-developer"),
        "refusal must explain the surface is internal-developer-only: {stderr}",
    );
}

#[test]
fn plan_dashboard_opens_with_dev_override() {
    let output = plan_dashboard()
        .env("ANVIL_DEV", "1")
        .output()
        .expect("failed to invoke anvil binary");

    // The gate is open; workspace discovery succeeds from the crate dir so the
    // read-only summary renders and the command exits 0.
    assert!(
        output.status.success(),
        "ANVIL_DEV=1 must open the gate and render the dashboard; code={:?} stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_ne!(
        output.status.code(),
        Some(EXIT_AUTH_REQUIRED),
        "an open gate must never report auth-required",
    );
}

/// CIB-052: under `--json` the closed-gate refusal envelope is structured
/// output and must land on **stdout** per the stream policy
/// (`docs/guides/cli-output-streams.md`) — the same contract as the admin
/// auth-error envelope it mirrors — with the existing exit code and no
/// JSON on stderr.
#[test]
fn plan_dashboard_json_refusal_envelope_lands_on_stdout() {
    let output = plan_dashboard()
        .arg("--json")
        .output()
        .expect("failed to invoke anvil binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(EXIT_AUTH_REQUIRED),
        "exit code must stay EXIT_AUTH_REQUIRED: stdout=\n{stdout}\nstderr=\n{stderr}",
    );

    let envelope: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!(
            "expected the refusal JSON envelope on stdout ({err}): \
             stdout=\n{stdout}\nstderr=\n{stderr}"
        )
    });
    assert_eq!(
        envelope["error"], "authentication_required",
        "unexpected envelope shape: stdout=\n{stdout}",
    );
    assert!(
        envelope["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("internal-developer")),
        "envelope detail must explain the surface is internal-developer-only: \
         stdout=\n{stdout}",
    );

    assert!(
        !stderr.contains("\"error\""),
        "stderr must carry no JSON envelope under --json: stderr=\n{stderr}",
    );
}

#[test]
fn plan_dashboard_opens_with_admin_key() {
    let output = plan_dashboard()
        .env("ANVIL_ADMIN_KEY", "any-non-empty-token")
        .output()
        .expect("failed to invoke anvil binary");

    assert!(
        output.status.success(),
        "a non-empty ANVIL_ADMIN_KEY must open the gate; code={:?} stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_ne!(
        output.status.code(),
        Some(EXIT_AUTH_REQUIRED),
        "an open gate must never report auth-required",
    );
}
