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

/// Run an arbitrary `anvil` subcommand in `project` with `ANVIL_HOME` (when
/// `Some`) set on the child env, plus the auth-bypass/hermetic env. Returns
/// `(exit_ok, combined stdout+stderr)`.
fn run_anvil(project: &Path, anvil_home: Option<&Path>, args: &[&str]) -> (bool, String) {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(args)
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
    let out = cmd.output().expect("spawn anvil");
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), combined)
}

#[test]
fn start_under_gated_anvil_home_does_not_seed_project_state() {
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("anvil home");

    // `anvil start` on a fresh repo would normally seed project state; under a
    // gated ANVIL_HOME it must run read-only and leave the real project clean.
    let (_ok, _out) = run_anvil(project.path(), Some(home.path()), &["start"]);

    for seeded in ["anvil/project-id", ".anvilrc", ".gitattributes"] {
        assert!(
            !project.path().join(seeded).exists(),
            "{seeded} must NOT be seeded into the real project under a gated ANVIL_HOME"
        );
    }
}

#[test]
fn init_refused_under_gated_anvil_home_leaves_no_anvilrc() {
    // Representative of the secondary mutation commands (init / doctor --fix /
    // migrate --apply / drift snapshot) gated after Council review.
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("anvil home");

    let (ok, out) = run_anvil(project.path(), Some(home.path()), &["init"]);

    assert!(
        !ok,
        "init must be refused under a gated ANVIL_HOME; out: {out}"
    );
    assert!(
        out.contains("--touch-project-state"),
        "refusal must name the opt-in flag; out: {out}"
    );
    assert!(
        !project.path().join(".anvilrc").exists() && !project.path().join(".anvil").exists(),
        "init must not seed .anvilrc / .anvil/ into the real project under the gate"
    );
}

#[test]
fn hook_bootstrap_refused_under_gated_anvil_home() {
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("anvil home");

    let (ok, out) = run_anvil(project.path(), Some(home.path()), &["hook", "bootstrap"]);

    assert!(
        !ok,
        "hook bootstrap must be refused under a gated ANVIL_HOME; out: {out}"
    );
    assert!(
        out.contains("--touch-project-state"),
        "refusal must name the opt-in flag; out: {out}"
    );
    assert!(
        !project
            .path()
            .join(".git")
            .join("hooks")
            .join("pre-commit")
            .exists(),
        "no git hook may be installed into the real project under the gate"
    );
}

#[test]
fn baseline_new_identity_refused_under_gate_leaves_project_id() {
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("anvil home");
    let project_id = project.path().join("anvil").join("project-id");

    let (ok, out) = run_anvil(
        project.path(),
        Some(home.path()),
        &["baseline", "--new-identity"],
    );

    assert!(
        !ok,
        "baseline --new-identity must be refused under the gate; out: {out}"
    );
    assert!(
        !project_id.exists(),
        "the project identity anchor must NOT be minted under a gated ANVIL_HOME \
         (the gate must precede the identity mint)"
    );
}

#[test]
fn anvil_home_flag_gates_like_env_var_via_reexec() {
    // The `--anvil-home` flag (which triggers the re-exec round-trip) must
    // deliver the same gating as setting ANVIL_HOME in the environment.
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("anvil home");
    let baseline_path = project.path().join("anvil").join("baseline.json");

    let home_arg = home.path().to_str().expect("utf8 home path");
    // Note: ANVIL_HOME is NOT set in the env here — only the flag is passed.
    let (ok, out) = run_anvil(
        project.path(),
        None,
        &["baseline", "--anvil-home", home_arg],
    );

    assert!(
        !ok,
        "--anvil-home flag must gate the baseline write; out: {out}"
    );
    assert!(
        out.contains("--touch-project-state"),
        "flag-driven refusal must name the opt-in; out: {out}"
    );
    assert!(
        !baseline_path.exists(),
        "project baseline must be untouched when gated via the --anvil-home flag"
    );
}
