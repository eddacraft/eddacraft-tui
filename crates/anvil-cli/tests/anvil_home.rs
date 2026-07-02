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

    // Parse the JSON rather than substring-matching the raw path: on Windows
    // the install_root separators are backslashes, which JSON escapes (`\\`),
    // so `stdout.contains(<raw path>)` spuriously fails even when the reported
    // install_root is exactly right. Comparing as `Path` is separator- and
    // escaping-agnostic.
    let status: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("status --json must emit valid JSON ({e}); got: {stdout}"));
    let install_root = status["install_root"].as_str().unwrap_or_else(|| {
        panic!("status --json must carry an install_root field under ANVIL_HOME; got: {stdout}")
    });
    assert_eq!(
        Path::new(install_root),
        home.path(),
        "status --json must report the resolved install_root"
    );
    assert_eq!(
        status["project_writes_gated"],
        serde_json::Value::Bool(true),
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

fn run_anvil_raw(
    project: &Path,
    home: &Path,
    anvil_home: Option<&Path>,
    args: &[&str],
) -> (bool, String, String) {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(args)
        .current_dir(project)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.env_remove("ANVIL_HOME");
    cmd.env_remove("ANVIL_TOUCH_PROJECT_STATE");
    if let Some(root) = anvil_home {
        cmd.env("ANVIL_HOME", root);
    }
    let out = cmd.output().expect("spawn anvil");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
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
fn anvil_home_flag_takes_precedence_over_env_var() {
    // #1726 acceptance: "`--anvil-home <path>` flag overrides the env var when
    // both are set." Set ANVIL_HOME to one prefix in the environment AND pass
    // `--anvil-home` pointing at a *different* prefix; `status --json` must
    // report the flag's prefix as the resolved install_root, proving the flag
    // wins over the env.
    let project = tempdir().expect("project dir");
    let env_home = tempdir().expect("env anvil home");
    let flag_home = tempdir().expect("flag anvil home");
    let flag_arg = flag_home.path().to_str().expect("utf8 flag path");

    // `run_status_json`'s `Some(..)` sets ANVIL_HOME on the child env; the extra
    // `--anvil-home` arg supplies the competing flag value.
    let (_ok, stdout) = run_status_json(
        project.path(),
        Some(env_home.path()),
        &["--anvil-home", flag_arg],
    );

    let status: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("status --json must emit valid JSON ({e}); got: {stdout}"));
    let install_root = status["install_root"].as_str().unwrap_or_else(|| {
        panic!("status --json must carry install_root when ANVIL_HOME/--anvil-home is set; got: {stdout}")
    });
    assert_eq!(
        Path::new(install_root),
        flag_home.path(),
        "the --anvil-home flag must win over the ANVIL_HOME env var",
    );
    assert_ne!(
        Path::new(install_root),
        env_home.path(),
        "the env-var prefix must not be the resolved root when the flag is also set",
    );
}

#[test]
fn anvil_home_pointing_at_a_nonexistent_path_is_used_as_is() {
    // #1726 acceptance names a "fallback when the path doesn't exist" case. The
    // resolver has no such fallback by design: any non-blank ANVIL_HOME is taken
    // verbatim (state is created under it on first write) rather than silently
    // reverting to the platform default. Pin that so a future "helpfully fall
    // back to default" change is a conscious one — a silent revert would send a
    // candidate's writes to the prod install the operator was trying to avoid.
    let project = tempdir().expect("project dir");
    let nonexistent = project.path().join("does").join("not").join("exist");
    assert!(
        !nonexistent.exists(),
        "precondition: the prefix does not exist yet"
    );

    let (ok, stdout) = run_status_json(project.path(), Some(nonexistent.as_path()), &[]);
    assert!(
        ok,
        "status must succeed under a not-yet-created ANVIL_HOME; got: {stdout}"
    );

    let status: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("status --json must emit valid JSON ({e}); got: {stdout}"));
    let install_root = status["install_root"].as_str().unwrap_or_else(|| {
        panic!("status --json must report the resolved install_root; got: {stdout}")
    });
    assert_eq!(
        Path::new(install_root),
        nonexistent.as_path(),
        "a non-existent ANVIL_HOME is used verbatim, not replaced by the default",
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

#[test]
fn uninstall_global_under_anvil_home_removes_prefix_user_dir_only() {
    let project = tempdir().expect("project dir");
    let prod_home = tempdir().expect("production home");
    let candidate = tempdir().expect("candidate anvil home");
    let user_dir = candidate.path().join("user");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(user_dir.join("credentials.json"), "{}").unwrap();
    let prod_state = prod_home.path().join(".anvil");
    std::fs::create_dir_all(&prod_state).unwrap();
    std::fs::write(prod_state.join("keep.txt"), "keep").unwrap();

    let (ok, stdout, stderr) = run_anvil_raw(
        project.path(),
        prod_home.path(),
        Some(candidate.path()),
        &[
            "uninstall",
            "--global",
            "--yes",
            "--keep-daemon",
            "--keep-mcp",
        ],
    );

    assert!(
        ok,
        "uninstall should succeed; stdout={stdout}; stderr={stderr}"
    );
    assert!(
        !user_dir.exists(),
        "<ANVIL_HOME>/user/ must be removed by global uninstall"
    );
    assert!(
        prod_state.join("keep.txt").exists(),
        "production ~/.anvil/ must be preserved under ANVIL_HOME"
    );
}

#[test]
fn uninstall_global_under_anvil_home_dry_run_json_reports_prefix_user_dir() {
    let project = tempdir().expect("project dir");
    let prod_home = tempdir().expect("production home");
    let candidate = tempdir().expect("candidate anvil home");
    let user_dir = candidate.path().join("user");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::write(user_dir.join("credentials.json"), "{}").unwrap();
    let prod_state = prod_home.path().join(".anvil");
    std::fs::create_dir_all(&prod_state).unwrap();
    std::fs::write(prod_state.join("keep.txt"), "keep").unwrap();

    let (ok, stdout, stderr) = run_anvil_raw(
        project.path(),
        prod_home.path(),
        Some(candidate.path()),
        &[
            "--json",
            "uninstall",
            "--global",
            "--dry-run",
            "--keep-daemon",
            "--keep-mcp",
        ],
    );

    assert!(
        ok,
        "dry-run uninstall should succeed; stdout={stdout}; stderr={stderr}"
    );
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("dry-run must emit JSON ({err}); stdout={stdout}"));
    let actions = value["actions"].as_array().expect("actions array");
    assert!(
        actions.iter().any(|action| {
            action["kind"] == "remove_user_anvil"
                && action["path"]
                    .as_str()
                    .is_some_and(|p| Path::new(p) == user_dir)
                && action["install_root_scoped"] == serde_json::Value::Bool(true)
        }),
        "dry-run JSON should name scoped <ANVIL_HOME>/user action: {stdout}"
    );
    assert!(
        user_dir.exists(),
        "dry-run must not delete <ANVIL_HOME>/user/"
    );
    assert!(
        prod_state.join("keep.txt").exists(),
        "dry-run must not delete production ~/.anvil/"
    );
}

#[test]
fn uninstall_refused_under_gated_anvil_home_when_project_has_dot_anvil() {
    let project = tempdir().expect("project dir");
    let home = tempdir().expect("home");
    let candidate = tempdir().expect("candidate anvil home");
    let dot_anvil = project.path().join(".anvil");
    std::fs::create_dir_all(&dot_anvil).unwrap();
    std::fs::write(dot_anvil.join("keep.txt"), "keep").unwrap();

    let (ok, _stdout, stderr) = run_anvil_raw(
        project.path(),
        home.path(),
        Some(candidate.path()),
        &["uninstall", "--yes", "--keep-daemon", "--keep-mcp"],
    );

    assert!(
        !ok,
        "uninstall must be refused under a gated ANVIL_HOME when the project has .anvil/"
    );
    assert!(
        stderr.contains("--touch-project-state"),
        "refusal must name the opt-in flag; stderr: {stderr}"
    );
    assert!(
        dot_anvil.join("keep.txt").exists(),
        "the real project's .anvil/ must not be removed under the gate"
    );
}

// --- Two-daemon coexistence (#1726 acceptance: concurrent daemons per prefix) ---
//
// Unix-only: the coexistence claim is about the per-prefix Unix **socket**
// (`<ANVIL_HOME>/intercept.sock`) and PID file. Windows IPC uses named pipes
// (no socket file), so its coexistence is a separate concern out of scope here.

/// Kill-on-drop guard so a failing assertion never leaks a live daemon.
#[cfg(unix)]
struct DaemonChild(std::process::Child);

#[cfg(unix)]
impl Drop for DaemonChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(unix)]
fn spawn_intercept_daemon(anvil_home: &Path) -> DaemonChild {
    use std::process::Stdio;
    let child = Command::new(ANVIL_BIN)
        .args(["intercept", "start", "--foreground"])
        .env("ANVIL_HOME", anvil_home)
        // Pin HOME/USERPROFILE to the prefix so no home-dir lookup can reach
        // host-scoped paths — same hermetic hygiene as the other helpers.
        .env("HOME", anvil_home)
        .env("USERPROFILE", anvil_home)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_DISABLE_UPDATE_HINT", "1")
        .env_remove("ANVIL_TOUCH_PROJECT_STATE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn anvil intercept start --foreground");
    DaemonChild(child)
}

#[cfg(unix)]
fn wait_for_socket(path: &Path, attempts: u32) -> bool {
    for _ in 0..attempts {
        if path.exists() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    path.exists()
}

#[cfg(unix)]
#[test]
fn two_daemons_under_different_anvil_home_prefixes_coexist() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Stdio;

    // #1726 acceptance: "A second daemon under a different ANVIL_HOME can run
    // concurrently without socket clash." Each prefix derives its own
    // intercept.sock / intercept.pid (ADR-060 §1 re-root + ADR-036 keying), so
    // two candidate daemons coexist — while the single-instance rule still
    // holds *within* a prefix.
    let home_a = tempdir().expect("anvil home a");
    let home_b = tempdir().expect("anvil home b");
    std::fs::set_permissions(home_a.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::set_permissions(home_b.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    let mut daemon_a = spawn_intercept_daemon(home_a.path());
    let mut daemon_b = spawn_intercept_daemon(home_b.path());

    let sock_a = home_a.path().join("intercept.sock");
    let sock_b = home_b.path().join("intercept.sock");
    assert!(
        wait_for_socket(&sock_a, 60),
        "daemon A must bind its socket under prefix A"
    );
    assert!(
        wait_for_socket(&sock_b, 60),
        "daemon B must bind its socket under prefix B"
    );
    assert_ne!(
        sock_a, sock_b,
        "the two prefixes derive distinct socket paths"
    );

    // Both alive at once — no single-instance collision across distinct prefixes.
    assert!(
        daemon_a.0.try_wait().unwrap().is_none(),
        "daemon A must stay alive alongside B"
    );
    assert!(
        daemon_b.0.try_wait().unwrap().is_none(),
        "daemon B must stay alive alongside A"
    );

    // The single-instance rule still holds *within* a prefix: a third daemon
    // under prefix A is refused (PID lock). `--foreground` normally blocks, but
    // a refused start exits immediately, so `output()` returns promptly.
    let dup = Command::new(ANVIL_BIN)
        .args(["intercept", "start", "--foreground"])
        .env("ANVIL_HOME", home_a.path())
        .env("HOME", home_a.path())
        .env("USERPROFILE", home_a.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_DISABLE_UPDATE_HINT", "1")
        .env_remove("ANVIL_TOUCH_PROJECT_STATE")
        .stdin(Stdio::null())
        .output()
        .expect("spawn duplicate daemon under prefix A");
    assert!(
        !dup.status.success(),
        "a second daemon under the SAME prefix must be refused"
    );
    let dup_err = String::from_utf8_lossy(&dup.stderr);
    assert!(
        dup_err.contains("already running") || dup_err.contains("PID file is locked"),
        "duplicate-under-same-prefix refusal must explain the single-instance lock; got: {dup_err}"
    );

    // Guards kill both daemons on drop.
    let _ = (&mut daemon_a, &mut daemon_b);
}
