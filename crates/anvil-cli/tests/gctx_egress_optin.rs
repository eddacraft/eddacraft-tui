//! GCTX-024: the `anvil gctx egress` opt-in command.
//!
//! Exercises the consent-gated enable/disable/status round-trip end-to-end
//! through the built binary: the CE-1 default is identity-only, `enable --yes`
//! persists consent under operator-owned state, `status` reports state + source,
//! `disable` reverts, and a non-interactive `enable` without `--yes` fails
//! closed (CE-12). A repository-controlled worktree file is never treated as
//! consent.

use std::path::{Path, PathBuf};
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Operator-state consent dir when `HOME` is rerooted to `home` (no `ANVIL_HOME`
/// / `XDG_STATE_HOME`).
fn operator_consent_dir(home: &Path) -> PathBuf {
    home.join(".local/state/anvil/gctx-egress")
}

/// True when any consent JSON was written under the operator state dir.
fn operator_consent_present(home: &Path) -> bool {
    let dir = operator_consent_dir(home);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .any(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
}

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
        .env_remove("ANVIL_GCTX_EGRESS")
        .env_remove("ANVIL_HOME")
        .env_remove("XDG_STATE_HOME");
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
    assert!(
        operator_consent_present(dir.path()),
        "enable must write under operator state, not the worktree"
    );
    assert!(
        !dir.path().join("anvil/witness/gctx-egress.json").exists(),
        "consent must not be written into the repository worktree"
    );

    let status = run_egress(dir.path(), None, &["status"]);
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(
        stdout.contains("enabled") && stdout.contains("workspace consent"),
        "status after enable must report config-sourced enablement: {stdout}",
    );

    let disabled = run_egress(dir.path(), None, &["disable"]);
    assert!(disabled.status.success());
    assert!(!operator_consent_present(dir.path()));

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
        !operator_consent_present(dir.path()),
        "no consent record may be written without acknowledgement",
    );
    assert!(
        !dir.path().join("anvil/witness/gctx-egress.json").exists(),
        "no worktree consent record may be written without acknowledgement",
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--yes"),
        "error must point the operator at --yes: {stderr}",
    );
}

#[test]
fn enable_from_subdirectory_keys_consent_to_repo_root() {
    // Regression guard: consent must be keyed to the git top-level (where the
    // daemon reads it), not the invocation CWD. Run `enable` from a subdir of a
    // real git repo and assert status is enabled for the root, with no worktree
    // plant and no subdir-keyed record.
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
        .env_remove("ANVIL_HOME")
        .env_remove("XDG_STATE_HOME")
        .output()
        .expect("invoke anvil");
    assert!(
        out.status.success(),
        "enable from subdir failed: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    assert!(
        operator_consent_present(root),
        "consent must land under operator state keyed to the repo root"
    );
    assert!(
        !root.join("anvil/witness/gctx-egress.json").exists(),
        "consent must not be written at the repo worktree path",
    );
    assert!(
        !sub.join("anvil/witness/gctx-egress.json").exists(),
        "consent must NOT be written in the invocation subdirectory",
    );

    // Status from the root and from the subdir both report the same opt-in.
    let status_root = run_egress(root, None, &["status"]);
    let status_sub = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["gctx", "egress", "status"])
        .current_dir(&sub)
        .env("HOME", root)
        .env("NONINTERACTIVE", "1")
        .env_remove("ANVIL_GCTX_EGRESS")
        .env_remove("ANVIL_HOME")
        .env_remove("XDG_STATE_HOME")
        .output()
        .expect("status from sub");
    for (label, out) in [("root", &status_root), ("sub", &status_sub)] {
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success()
                && stdout.contains("enabled")
                && stdout.contains("workspace consent"),
            "status from {label} must report enabled workspace consent: {stdout}",
        );
    }
}

#[test]
fn planted_worktree_consent_file_does_not_enable_status() {
    // Security regression: a hostile checkout planting the legacy worktree
    // path must not open snippet egress.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("anvil/witness")).unwrap();
    std::fs::write(
        dir.path().join("anvil/witness/gctx-egress.json"),
        br#"{"snippet_egress":true,"consent_version":1,"workspace_root":"planted"}"#,
    )
    .unwrap();

    let status = run_egress(dir.path(), None, &["status"]);
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(
        status.status.success(),
        "status must still succeed: {}",
        String::from_utf8_lossy(&status.stderr),
    );
    assert!(
        stdout.contains("identity-only") && stdout.contains("default (no opt-in)"),
        "planted worktree consent must not enable egress: {stdout}",
    );
}

#[cfg(unix)]
#[test]
fn env_sourced_status_skips_unreadable_persisted_state() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    // Plant a broken symlink under operator state so a naive read would error.
    let consent_dir = operator_consent_dir(dir.path());
    std::fs::create_dir_all(&consent_dir).unwrap();
    let other = tempfile::tempdir().unwrap();
    // Any leaf name is fine — env-sourced status must not need to open it.
    symlink(
        other.path().join("missing-consent.json"),
        consent_dir.join("deadbeef.json"),
    )
    .unwrap();

    let killed = run_egress(dir.path(), Some("0"), &["status"]);
    assert!(
        killed.status.success(),
        "env kill-switch status should not read persisted state: {}",
        String::from_utf8_lossy(&killed.stderr),
    );
    let stdout = String::from_utf8_lossy(&killed.stdout);
    assert!(
        stdout.contains("identity-only") && stdout.contains("environment"),
        "env=0 must remain decisive: {stdout}",
    );

    let forced = run_egress(dir.path(), Some("1"), &["status"]);
    assert!(
        forced.status.success(),
        "env force-on status should not read persisted state: {}",
        String::from_utf8_lossy(&forced.stderr),
    );
    let stdout = String::from_utf8_lossy(&forced.stdout);
    assert!(
        stdout.contains("enabled") && stdout.contains("environment"),
        "env=1 must remain decisive: {stdout}",
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
