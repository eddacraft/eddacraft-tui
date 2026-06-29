//! GCTX-024: the `anvil gctx egress` opt-in command.
//!
//! Exercises the consent-gated enable/disable/status round-trip end-to-end
//! through the built binary: the CE-1 default is identity-only, `enable --yes`
//! persists consent, `status` reports state + source, `disable` reverts, and a
//! non-interactive `enable` without `--yes` fails closed (CE-12).

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Run `anvil gctx egress <args...>` in `cwd`, with a clean env: `HOME` rerooted,
/// `ANVIL_GCTX_EGRESS` unset unless `egress_env` is `Some`, and
/// `NONINTERACTIVE=1` so the consent prompt never blocks the test.
fn run_egress(
    cwd: &std::path::Path,
    egress_env: Option<&str>,
    args: &[&str],
) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("gctx")
        .arg("egress")
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd)
        .env("NONINTERACTIVE", "1")
        .env_remove("ANVIL_GCTX_EGRESS");
    if let Some(value) = egress_env {
        cmd.env("ANVIL_GCTX_EGRESS", value);
    }
    cmd.output().expect("failed to invoke anvil binary")
}

#[test]
fn status_defaults_to_identity_only() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_egress(dir.path(), None, &["status"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "status must exit 0: {stdout}");
    assert!(
        stdout.contains("identity-only"),
        "default must be identity-only: {stdout}",
    );
    assert!(
        stdout.contains("default (no opt-in)"),
        "source must read as the default: {stdout}",
    );
}

#[test]
fn enable_then_status_then_disable_round_trip() {
    let dir = tempfile::tempdir().unwrap();

    let enabled = run_egress(dir.path(), None, &["enable", "--yes"]);
    assert!(
        enabled.status.success(),
        "enable --yes must succeed: {}",
        String::from_utf8_lossy(&enabled.stderr),
    );
    assert!(dir.path().join("anvil/gctx-egress.json").is_file());

    let status = run_egress(dir.path(), None, &["status"]);
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(
        stdout.contains("enabled") && stdout.contains("workspace consent"),
        "status after enable must report config-sourced enablement: {stdout}",
    );

    let disabled = run_egress(dir.path(), None, &["disable"]);
    assert!(disabled.status.success());
    assert!(!dir.path().join("anvil/gctx-egress.json").exists());

    let after = run_egress(dir.path(), None, &["status"]);
    assert!(
        String::from_utf8_lossy(&after.stdout).contains("identity-only"),
        "disable must revert to identity-only",
    );
}

#[test]
fn enable_without_consent_fails_closed_non_interactive() {
    let dir = tempfile::tempdir().unwrap();
    // No `--yes`, NONINTERACTIVE=1 → must refuse rather than auto-enable (CE-12).
    let out = run_egress(dir.path(), None, &["enable"]);
    assert!(
        !out.status.success(),
        "non-interactive enable without --yes must fail closed",
    );
    assert!(
        !dir.path().join("anvil/gctx-egress.json").exists(),
        "no consent record may be written without acknowledgement",
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--yes"),
        "error must point the operator at --yes: {stderr}",
    );
}

#[test]
fn enable_from_subdirectory_writes_consent_at_repo_root() {
    // Regression guard: consent must be keyed to the git top-level (where the
    // daemon reads it), not the invocation CWD. Run `enable` from a subdir of a
    // real git repo and assert the record lands at the root, not the subdir.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git");
    };
    git(&["init", "-q"]);
    let sub = root.join("crates").join("inner");
    std::fs::create_dir_all(&sub).unwrap();

    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["gctx", "egress", "enable", "--yes"])
        .current_dir(&sub)
        .env("HOME", root)
        .env("NONINTERACTIVE", "1")
        .env_remove("ANVIL_GCTX_EGRESS")
        .output()
        .expect("invoke anvil");
    assert!(
        out.status.success(),
        "enable from subdir failed: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let canonical_root = std::fs::canonicalize(root).unwrap();
    assert!(
        canonical_root.join("anvil/gctx-egress.json").is_file(),
        "consent must be written at the repo root",
    );
    assert!(
        !sub.join("anvil/gctx-egress.json").exists(),
        "consent must NOT be written in the invocation subdirectory",
    );
}

#[test]
fn env_var_overrides_persisted_state_in_status() {
    let dir = tempfile::tempdir().unwrap();
    run_egress(dir.path(), None, &["enable", "--yes"]);

    // env=0 kill-switch overrides the persisted opt-in.
    let killed = run_egress(dir.path(), Some("0"), &["status"]);
    let stdout = String::from_utf8_lossy(&killed.stdout);
    assert!(
        stdout.contains("identity-only") && stdout.contains("environment"),
        "env=0 must override config and report an env source: {stdout}",
    );

    // env=1 forces on and is reported as env-sourced even with no persisted flag.
    let dir2 = tempfile::tempdir().unwrap();
    let forced = run_egress(dir2.path(), Some("1"), &["status"]);
    let stdout2 = String::from_utf8_lossy(&forced.stdout);
    assert!(
        stdout2.contains("enabled") && stdout2.contains("environment"),
        "env=1 must force on with an env source: {stdout2}",
    );
}
