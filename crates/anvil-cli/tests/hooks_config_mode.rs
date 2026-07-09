//! GHOOK-002 — `anvil hooks install --config` / `uninstall --config` are
//! exercised end-to-end against a real `git init`'d repo. Each test skips
//! when the host's Git is older than 2.54 because that is the floor the
//! `--config` opt-in defends; verifying the refusal path on older Git is
//! covered by the unit tests in `commands/hooks.rs`.
//!
//! See `plans/modules/git-config-hooks.aps.md#GHOOK-002` and
//! `docs/guides/git-hook-compatibility.md` for the rollout policy.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Ceiling for git repo discovery. Git only honours a ceiling that is a
/// proper ancestor of the probed directory, so this must be the tempdir's
/// parent — passing the tempdir itself is silently ignored and the walk
/// could escape into a stray ancestor repo (e.g. a leftover /tmp/.git).
/// The repo `git init`'d at the tempdir itself is still found: the
/// ceiling only stops the walk from ascending past it.
fn discovery_ceiling(dir: &std::path::Path) -> &std::path::Path {
    dir.parent().expect("tempdir has a parent")
}

/// Returns `Some((major, minor, patch))` when `git --version` parses, or
/// `None` when the binary is missing. Tests bail (skip) on `None` and on
/// any version below 2.54.
fn host_git_version() -> Option<(u32, u32, u32)> {
    let out = Command::new("git").arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout).into_owned();
    let stripped = raw.trim().strip_prefix("git version ")?;
    let core = stripped.split_whitespace().next().unwrap_or(stripped);
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next()?.parse::<u32>().ok()?;
    let patch = parts
        .next()
        .and_then(|p| {
            let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<u32>().ok()
        })
        .unwrap_or(0);
    Some((major, minor, patch))
}

fn skip_if_git_too_old() -> Option<()> {
    let (major, minor, patch) = host_git_version()?;
    if (major, minor) < (2, 54) {
        eprintln!(
            "skipping GHOOK-002 integration test: host git is {major}.{minor}.{patch} (< 2.54)",
        );
        return None;
    }
    Some(())
}

fn git_init(dir: &std::path::Path) {
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .status()
        .expect("invoking git init");
    assert!(status.success(), "git init must succeed");
}

fn config_get_all(dir: &std::path::Path, key: &str) -> Vec<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["config", "--get-all", key])
        .output()
        .expect("invoking git config --get-all");
    if out.status.success() {
        return String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(ToString::to_string)
            .collect();
    }
    // `git config --get-all` exits 1 specifically when the key is not
    // set. Any other exit code is a real error (bad repo, invalid key,
    // permission issue, etc.) and must fail the test loudly rather
    // than silently hand back an empty vec — that would let an
    // uninstall-path assertion pass for the wrong reason.
    match out.status.code() {
        Some(1) => Vec::new(),
        other => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            panic!(
                "git config --get-all {key} failed unexpectedly: exit={other:?} stderr={stderr}"
            );
        }
    }
}

#[test]
fn hooks_install_config_writes_command_for_pre_commit_and_pre_push() {
    if skip_if_git_too_old().is_none() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["hooks", "install", "--config"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
        .output()
        .expect("invoking anvil hooks install --config");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "install --config must exit 0\nstdout: {stdout}\nstderr: {stderr}",
    );

    let pre_commit = config_get_all(dir.path(), "hook.pre-commit.command");
    assert_eq!(
        pre_commit,
        vec!["ANVIL_HOOK=1 anvil gate --progress".to_string()],
        "pre-commit config-mode entry must round-trip",
    );

    let pre_push = config_get_all(dir.path(), "hook.pre-push.command");
    assert_eq!(
        pre_push,
        vec!["ANVIL_HOOK=1 anvil gate".to_string()],
        "pre-push config-mode entry must round-trip",
    );

    // No file-mode hooks should have been written when --config is used.
    assert!(
        !dir.path().join(".git/hooks/pre-commit").exists(),
        "config mode must not write .git/hooks/pre-commit",
    );
    assert!(
        !dir.path().join(".git/hooks/pre-push").exists(),
        "config mode must not write .git/hooks/pre-push",
    );
    assert!(
        !dir.path().join(".husky").exists(),
        "config mode must not create .husky/",
    );
}

#[test]
fn hooks_uninstall_config_clears_command_entries() {
    if skip_if_git_too_old().is_none() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());

    // Install first.
    let install = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["hooks", "install", "--config"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
        .output()
        .expect("invoking anvil hooks install --config");
    assert!(install.status.success(), "install --config must exit 0");
    assert!(!config_get_all(dir.path(), "hook.pre-commit.command").is_empty());

    // Uninstall.
    let uninstall = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["hooks", "uninstall", "--config"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
        .output()
        .expect("invoking anvil hooks uninstall --config");
    let stdout = String::from_utf8_lossy(&uninstall.stdout);
    let stderr = String::from_utf8_lossy(&uninstall.stderr);
    assert!(
        uninstall.status.success(),
        "uninstall --config must exit 0\nstdout: {stdout}\nstderr: {stderr}",
    );

    assert!(
        config_get_all(dir.path(), "hook.pre-commit.command").is_empty(),
        "pre-commit config-mode entry must be removed",
    );
    assert!(
        config_get_all(dir.path(), "hook.pre-push.command").is_empty(),
        "pre-push config-mode entry must be removed",
    );
}

#[test]
fn hooks_install_config_does_not_stack_duplicates_on_repeat() {
    if skip_if_git_too_old().is_none() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());

    for _ in 0..3 {
        let out = Command::new(ANVIL_BIN)
            .arg("--no-tui")
            .args(["hooks", "install", "--config"])
            .current_dir(dir.path())
            .env("HOME", dir.path())
            .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
            .output()
            .expect("invoking anvil hooks install --config");
        assert!(
            out.status.success(),
            "repeated install --config must exit 0"
        );
    }

    let pre_commit = config_get_all(dir.path(), "hook.pre-commit.command");
    assert_eq!(pre_commit.len(), 1, "install must be idempotent");
}

#[test]
fn hooks_install_config_refuses_when_host_git_too_old() {
    let Some((major, minor, patch)) = host_git_version() else {
        eprintln!("skipping: git --version unavailable");
        return;
    };
    if (major, minor) >= (2, 54) {
        eprintln!(
            "skipping: host git is {major}.{minor}.{patch} (>= 2.54); refusal path \
             is exercised by unit tests in commands/hooks.rs",
        );
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["hooks", "install", "--config"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
        .output()
        .expect("invoking anvil hooks install --config");

    assert!(
        !output.status.success(),
        "install --config must refuse on Git < 2.54",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("2.54"),
        "refusal must mention 2.54: {stderr}",
    );
    assert!(
        stderr.contains("docs/guides/git-hook-compatibility.md"),
        "refusal must point at policy doc: {stderr}",
    );
}

#[test]
fn hooks_install_config_warns_when_file_mode_hook_already_exists() {
    if skip_if_git_too_old().is_none() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());

    // Plant an existing file-mode pre-commit hook.
    let hooks_dir = dir.path().join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap();
    std::fs::write(hooks_dir.join("pre-commit"), "#!/bin/sh\necho old\n").unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["hooks", "install", "--config", "--pre-commit-only"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
        .output()
        .expect("invoking anvil hooks install --config");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "install --config must warn rather than block per scope-guard",
    );
    assert!(
        stdout.contains("Duplicate-execution risk"),
        "expected coexistence warning in stdout, got:\n{stdout}",
    );
    // And the config entry must still be written.
    let pre_commit = config_get_all(dir.path(), "hook.pre-commit.command");
    assert_eq!(pre_commit.len(), 1);
}
