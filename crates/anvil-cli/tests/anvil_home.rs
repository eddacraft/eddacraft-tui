//! DISTRIB-006 (ADR-060): `ANVIL_HOME` install-root override + project-state
//! write-guard, proven end-to-end against the built `anvil` binary.
//!
//! These tests set `ANVIL_HOME` on the **child process only** (`Command::env`),
//! never on the test process, so they are hermetic and race-free regardless of
//! test parallelism.
//!
//! The headline mitigation from ADR-060's "Risks / Mitigations": under a
//! non-default `ANVIL_HOME` without `--touch-project-state`, a durable
//! per-project mutation (`anvil baseline`) is **refused** and the real project's
//! `anvil/baseline.json` is left **untouched** — while the same command succeeds
//! with the opt-in and under the platform default.

use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Run `anvil baseline` in `project` with the given extra args and `ANVIL_HOME`
/// (when `Some`) set on the child environment only. Returns `(exit_ok, stderr)`.
fn run_baseline(project: &Path, anvil_home: Option<&Path>, extra: &[&str]) -> (bool, String) {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("baseline").args(extra).current_dir(project);
    // Keep the run hermetic and offline regardless of the host environment.
    cmd.env_remove("ANVIL_HOME");
    cmd.env_remove("ANVIL_TOUCH_PROJECT_STATE");
    if let Some(home) = anvil_home {
        cmd.env("ANVIL_HOME", home);
    }
    let out = cmd.output().expect("spawn anvil baseline");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Run `anvil status --json` in `project` with `ANVIL_HOME` (when `Some`) and the
/// given extra args set on the child environment only. Returns `(exit_ok, stdout)`.
///
/// Mirrors `status_json_contract.rs`: `ANVIL_DEV=1` bypasses the auth gate so the
/// full `StatusOutput` is emitted, and a temp `HOME` keeps default path
/// resolution hermetic.
fn run_status_json(project: &Path, anvil_home: Option<&Path>, extra: &[&str]) -> (bool, String) {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("status")
        .arg("--json")
        .args(extra)
        .current_dir(project)
        .env("HOME", project)
        .env("USERPROFILE", project)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.env_remove("ANVIL_HOME");
    cmd.env_remove("ANVIL_TOUCH_PROJECT_STATE");
    if let Some(home) = anvil_home {
        cmd.env("ANVIL_HOME", home);
    }
    let out = cmd.output().expect("spawn anvil status --json");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn status_json_reports_install_root_and_gated_under_anvil_home() {
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("anvil home");

    let (_ok, stdout) = run_status_json(project.path(), Some(home.path()), &[]);

    let home_str = home.path().display().to_string();
    assert!(
        stdout.contains(&home_str),
        "status --json must report the resolved install_root ({home_str}); got: {stdout}"
    );
    assert!(
        stdout.contains("\"install_root\""),
        "status --json must carry an install_root field under ANVIL_HOME; got: {stdout}"
    );
    assert!(
        stdout.contains("\"project_writes_gated\": true"),
        "project_writes_gated must be true without --touch-project-state; got: {stdout}"
    );
}

#[test]
fn status_json_reports_writes_ungated_with_opt_in() {
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("anvil home");

    let (_ok, stdout) = run_status_json(
        project.path(),
        Some(home.path()),
        &["--touch-project-state"],
    );

    assert!(
        stdout.contains("\"project_writes_gated\": false"),
        "project_writes_gated must be false with --touch-project-state; got: {stdout}"
    );
}

#[test]
fn status_json_omits_install_fields_under_platform_default() {
    let project = tempdir().expect("project dir");

    let (_ok, stdout) = run_status_json(project.path(), None, &[]);

    assert!(
        !stdout.contains("install_root"),
        "default status --json must NOT carry install_root (byte-for-byte v1); got: {stdout}"
    );
    assert!(
        !stdout.contains("project_writes_gated"),
        "default status --json must NOT carry project_writes_gated; got: {stdout}"
    );
}

#[test]
fn baseline_is_refused_under_gated_anvil_home_and_leaves_project_untouched() {
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("anvil home");
    let baseline_path = project.path().join("anvil").join("baseline.json");

    let (ok, stderr) = run_baseline(project.path(), Some(home.path()), &[]);

    assert!(
        !ok,
        "baseline must be refused under a gated ANVIL_HOME; stderr: {stderr}"
    );
    assert!(
        stderr.contains("--touch-project-state"),
        "refusal must name the opt-in flag; stderr: {stderr}"
    );
    assert!(
        !baseline_path.exists(),
        "the real project's anvil/baseline.json must NOT be written under the gate"
    );
    // The override must not have leaked any baseline under the prefix either.
    assert!(
        !home.path().join("anvil").join("baseline.json").exists(),
        "baseline must not be written under the ANVIL_HOME prefix (per-project \
         state stays at the project root and the write is gated)"
    );
}

#[test]
fn baseline_writes_with_touch_project_state_opt_in() {
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("anvil home");
    let baseline_path = project.path().join("anvil").join("baseline.json");

    let (ok, stderr) = run_baseline(
        project.path(),
        Some(home.path()),
        &["--touch-project-state"],
    );

    assert!(
        ok,
        "baseline must succeed with --touch-project-state; stderr: {stderr}"
    );
    assert!(
        baseline_path.exists(),
        "anvil/baseline.json must be written when the operator opts in"
    );
}

#[test]
fn baseline_writes_under_platform_default_without_anvil_home() {
    let project = tempdir().expect("project dir");
    let baseline_path = project.path().join("anvil").join("baseline.json");

    let (ok, stderr) = run_baseline(project.path(), None, &[]);

    assert!(
        ok,
        "baseline must succeed under the platform default; stderr: {stderr}"
    );
    assert!(
        baseline_path.exists(),
        "anvil/baseline.json must be written in the default (non-overridden) case"
    );
}
