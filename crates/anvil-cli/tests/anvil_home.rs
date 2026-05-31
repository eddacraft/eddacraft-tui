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
/// (when `Some`) set on the child environment only. Returns (exit_ok, stderr).
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

    let (ok, stderr) =
        run_baseline(project.path(), Some(home.path()), &["--touch-project-state"]);

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
