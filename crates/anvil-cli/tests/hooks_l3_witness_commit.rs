//! CIB-363 / CIB-346 follow-on: stock file-mode hooks plus one real
//! `git commit` must move `anvil audit-chain --json` to `witnessed: 1`
//! with HEAD absent from `unwitnessed`. A gate-only pre-commit control
//! stays at `witnessed: 0`.
//!
//! PR #3982 pinned installed hook *text*; this file runs the installed
//! scripts. Unix-only: file hooks are `#!/bin/sh`.
//!
//! Fixture is *not* `anvil start`: that writes MCP config and probes the
//! daemon. Identity + a start-like three-check `.anvil.yaml` + `hooks
//! install` is the hermetic equivalent of the activated hook bodies.

#![cfg(unix)]

use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const PROJECT_UUID: &str = "01997e4a-1b2c-7345-8901-abcdef123456";

/// Matches the three-check set `anvil start` writes so the hook's
/// `anvil gate --progress` can pass without lint/test/pnpm.
const START_LIKE_CONFIG: &str = r#"schemaVersion: "1.0.0"
planningDir: plans
format: yaml
checks:
  - secret-detection
  - import-boundaries
  - antipattern-scan
"#;

/// Pre-CIB-346 stock pre-commit: quality gate only, no L3 witness and
/// no post-commit SHA bind. The regression control.
const GATE_ONLY_PRE_COMMIT: &str = r#"#!/bin/sh
# @anvil-managed
# old gate-only hook (pre-CIB-346)
[ "$ANVIL_SKIP_HOOKS" = "1" ] && exit 0
command -v anvil >/dev/null 2>&1 || { echo "anvil not found on PATH, skipping hook"; exit 0; }
ANVIL_HOOK=1 anvil gate --progress || {
  echo "anvil gate checks failed"
  exit 1
}
"#;

struct Fixture {
    _tmp: TempDir,
    root: PathBuf,
    home: PathBuf,
    bin_dir: PathBuf,
}

/// Git only honours a ceiling that is a proper ancestor of the probed
/// directory. Passing the tempdir itself is silently ignored.
fn discovery_ceiling(dir: &Path) -> &Path {
    dir.parent().expect("tempdir has a parent")
}

fn fixture_repo() -> Fixture {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let home = root.join(".home");
    let bin_dir = home.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("create isolated bin dir");
    symlink(ANVIL_BIN, bin_dir.join("anvil")).expect("symlink anvil onto PATH");

    let global_gitconfig = home.join(".gitconfig");
    std::fs::write(
        &global_gitconfig,
        "[user]\n\tname = cib363\n\temail = cib363@example.test\n[commit]\n\tgpgsign = false\n",
    )
    .expect("write isolated gitconfig");

    git_init(&root, &global_gitconfig);

    std::fs::write(root.join(".anvil.yaml"), START_LIKE_CONFIG).expect("write .anvil.yaml");
    std::fs::create_dir_all(root.join("anvil")).expect("create anvil/");
    std::fs::write(
        root.join("anvil").join("project-id"),
        format!("project_uuid: {PROJECT_UUID}\n"),
    )
    .expect("write anvil/project-id");
    std::fs::write(root.join("README.md"), "hello\n").expect("write README.md");

    Fixture {
        _tmp: tmp,
        root,
        home,
        bin_dir,
    }
}

fn git_init(root: &Path, global_gitconfig: &Path) {
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", global_gitconfig)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("invoking git init");
    assert!(status.success(), "git init must succeed");
    for (key, value) in [
        ("user.name", "cib363"),
        ("user.email", "cib363@example.test"),
        ("commit.gpgsign", "false"),
    ] {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["config", key, value])
            .env("GIT_CONFIG_GLOBAL", global_gitconfig)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("invoking git config");
        assert!(status.success(), "git config {key} must succeed");
    }
}

fn apply_hermetic<'a>(cmd: &'a mut Command, fx: &Fixture) -> &'a mut Command {
    let path = format!(
        "{}:{}",
        fx.bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    cmd.current_dir(&fx.root)
        .env("HOME", &fx.home)
        .env("USERPROFILE", &fx.home)
        .env("LOCALAPPDATA", &fx.home)
        .env("XDG_RUNTIME_DIR", &fx.home)
        .env("PATH", path)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(&fx.root))
        .env("GIT_CONFIG_GLOBAL", fx.home.join(".gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("ANVIL_HOME")
        .env_remove("ANVIL_SKIP_HOOKS")
        .env_remove("ANVIL_LICENSE")
        .env_remove("ANVIL_LOG")
        .env_remove("RUST_LOG")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("ANVIL_TOUCH_PROJECT_STATE")
}

fn hooks_install(fx: &Fixture) {
    let output = apply_hermetic(&mut Command::new(ANVIL_BIN), fx)
        .args(["--no-tui", "hooks", "install"])
        .output()
        .expect("invoking anvil hooks install");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "hooks install must exit 0\nstdout: {stdout}\nstderr: {stderr}",
    );
}

fn install_gate_only_pre_commit(fx: &Fixture) {
    let hooks_dir = fx.root.join(".git").join("hooks");
    std::fs::create_dir_all(&hooks_dir).expect("create .git/hooks");
    let path = hooks_dir.join("pre-commit");
    std::fs::write(&path, GATE_ONLY_PRE_COMMIT).expect("write gate-only pre-commit");
    let mut perms = std::fs::metadata(&path)
        .expect("stat pre-commit")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("chmod pre-commit");
    let _ = std::fs::remove_file(hooks_dir.join("post-commit"));
}

fn git_commit(fx: &Fixture) -> std::process::Output {
    let add = apply_hermetic(&mut Command::new("git"), fx)
        .args(["add", "README.md"])
        .status()
        .expect("git add");
    assert!(add.success(), "git add must succeed");
    apply_hermetic(&mut Command::new("git"), fx)
        .args(["commit", "-m", "cib363 first commit"])
        .output()
        .expect("git commit")
}

fn head_sha(fx: &Fixture) -> String {
    let output = apply_hermetic(&mut Command::new("git"), fx)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse HEAD");
    assert!(
        output.status.success(),
        "git rev-parse HEAD must succeed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn audit_chain_json(fx: &Fixture) -> (i32, Value, String) {
    let output = apply_hermetic(&mut Command::new(ANVIL_BIN), fx)
        .args(["--no-tui", "audit-chain", "--json", "--threshold", "1"])
        .output()
        .expect("invoking anvil audit-chain --json");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    let parsed: Value = serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!("audit-chain stdout must be JSON: {err}\nstdout: {stdout}\nstderr: {stderr}");
    });
    (code, parsed, stderr)
}

fn unwitnessed_shas(report: &Value) -> Vec<String> {
    report["unwitnessed"]
        .as_array()
        .expect("unwitnessed array")
        .iter()
        .map(|v| v.as_str().expect("unwitnessed sha string").to_string())
        .collect()
}

#[test]
fn stock_hooks_install_then_commit_witnesses_head() {
    let fx = fixture_repo();
    hooks_install(&fx);

    let commit = git_commit(&fx);
    let stdout = String::from_utf8_lossy(&commit.stdout);
    let stderr = String::from_utf8_lossy(&commit.stderr);
    assert!(
        commit.status.success(),
        "stock hooks must allow the commit\nstdout: {stdout}\nstderr: {stderr}",
    );

    let head = head_sha(&fx);
    let (code, report, audit_stderr) = audit_chain_json(&fx);
    assert_eq!(
        report["commits_walked"].as_u64(),
        Some(1),
        "one commit walked; stderr={audit_stderr}",
    );
    assert_eq!(
        report["witnessed"].as_u64(),
        Some(1),
        "stock install must witness HEAD; report={report} stderr={audit_stderr}",
    );
    assert_eq!(
        code, 0,
        "clean coverage must keep audit-chain exit 0; stderr={audit_stderr}",
    );
    let unwitnessed = unwitnessed_shas(&report);
    assert!(
        unwitnessed.is_empty(),
        "HEAD must be absent from unwitnessed: {unwitnessed:?}",
    );
    assert!(
        !unwitnessed.iter().any(|sha| sha == &head),
        "HEAD {head} must not appear in unwitnessed",
    );
}

#[test]
fn gate_only_pre_commit_leaves_head_unwitnessed() {
    let fx = fixture_repo();
    install_gate_only_pre_commit(&fx);

    let commit = git_commit(&fx);
    let stdout = String::from_utf8_lossy(&commit.stdout);
    let stderr = String::from_utf8_lossy(&commit.stderr);
    assert!(
        commit.status.success(),
        "gate-only hook must still allow a passing gate commit\nstdout: {stdout}\nstderr: {stderr}",
    );

    let head = head_sha(&fx);
    let (code, report, audit_stderr) = audit_chain_json(&fx);
    assert_eq!(
        report["commits_walked"].as_u64(),
        Some(1),
        "one commit walked; stderr={audit_stderr}",
    );
    assert_eq!(
        report["witnessed"].as_u64(),
        Some(0),
        "gate-only pre-commit must leave audit-chain dark; report={report} stderr={audit_stderr}",
    );
    assert_eq!(
        code, 1,
        "threshold-1 drift must fail audit-chain; stderr={audit_stderr}",
    );
    let unwitnessed = unwitnessed_shas(&report);
    assert!(
        unwitnessed.iter().any(|sha| sha == &head),
        "HEAD {head} must be in unwitnessed: {unwitnessed:?}",
    );
}
