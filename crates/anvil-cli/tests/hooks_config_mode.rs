//! GHOOK-002 — `anvil hooks install --config` / `uninstall --config` are
//! exercised end-to-end against a real `git init`'d repo. Each test skips
//! when the host's Git is older than 2.54 because that is the floor the
//! `--config` opt-in defends; verifying the refusal path on older Git is
//! covered by the unit tests in `commands/hooks.rs`.
//! A missing or unparsable Git executable is a failed suite precondition.
//!
//! See `plans/modules/git-config-hooks.aps.md#GHOOK-002` and
//! `docs/guides/git-hook-compatibility.md` for the rollout policy.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
#[should_panic(expected = "unrecognised version")]
fn malformed_git_version_fails_the_suite_precondition() {
    parse_git_version("not git version output");
}

/// Ceiling for git repo discovery. Git only honours a ceiling that is a
/// proper ancestor of the probed directory, so this must be the tempdir's
/// parent — passing the tempdir itself is silently ignored and the walk
/// could escape into a stray ancestor repo (e.g. a leftover /tmp/.git).
/// The repo `git init`'d at the tempdir itself is still found: the
/// ceiling only stops the walk from ascending past it.
fn discovery_ceiling(dir: &std::path::Path) -> &std::path::Path {
    dir.parent().expect("tempdir has a parent")
}

#[test]
#[should_panic(expected = "Git is required for config-mode integration tests")]
fn missing_git_fails_the_suite_precondition() {
    host_git_version_from("__anvil_missing_git_for_test__");
}

fn parse_git_version(raw: &str) -> (u32, u32, u32) {
    let stripped = raw.trim().strip_prefix("git version ").unwrap_or_else(|| {
        panic!("Git is required for config-mode integration tests: unrecognised version {raw:?}")
    });
    let core = stripped.split_whitespace().next().unwrap_or(stripped);
    let mut parts = core.split('.');
    let major = parts
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or_else(|| {
            panic!("Git is required for config-mode integration tests: invalid version {raw:?}")
        });
    let minor = parts
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or_else(|| {
            panic!("Git is required for config-mode integration tests: invalid version {raw:?}")
        });
    let patch = parts
        .next()
        .and_then(|part| {
            let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<u32>().ok()
        })
        .unwrap_or(0);
    (major, minor, patch)
}

fn host_git_version_from(program: &str) -> (u32, u32, u32) {
    let out = Command::new(program)
        .arg("--version")
        .output()
        .unwrap_or_else(|error| {
            panic!("Git is required for config-mode integration tests: {error}")
        });
    assert!(
        out.status.success(),
        "Git is required for config-mode integration tests: `{program} --version` exited {}",
        out.status
    );
    let raw = String::from_utf8_lossy(&out.stdout);
    parse_git_version(&raw)
}

fn host_git_version() -> (u32, u32, u32) {
    host_git_version_from("git")
}

fn skip_if_git_too_old() -> bool {
    let (major, minor, patch) = host_git_version();
    if (major, minor) < (2, 54) {
        eprintln!(
            "skipping GHOOK-002 integration test: host git is {major}.{minor}.{patch} (< 2.54)",
        );
        return true;
    }
    false
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
    if skip_if_git_too_old() {
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
        vec![
            "ANVIL_HOOK=1 anvil gate --progress".to_string(),
            "ANVIL_HOOK=1 anvil hook pre-commit".to_string(),
        ],
        "pre-commit config-mode must install gate + L3 witness",
    );

    let pre_push = config_get_all(dir.path(), "hook.pre-push.command");
    assert_eq!(
        pre_push,
        vec!["ANVIL_HOOK=1 anvil hook pre-push".to_string()],
        "pre-push config-mode entry must round-trip",
    );

    let post_commit = config_get_all(dir.path(), "hook.post-commit.command");
    assert_eq!(
        post_commit,
        vec!["ANVIL_HOOK=1 anvil hook post-commit".to_string()],
        "post-commit config-mode must bind HEAD via L3 witness",
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
        !dir.path().join(".git/hooks/post-commit").exists(),
        "config mode must not write .git/hooks/post-commit",
    );
    assert!(
        !dir.path().join(".husky").exists(),
        "config mode must not create .husky/",
    );
}

#[test]
fn hooks_uninstall_config_clears_command_entries() {
    if skip_if_git_too_old() {
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
    assert!(
        config_get_all(dir.path(), "hook.post-commit.command").is_empty(),
        "post-commit config-mode entry must be removed",
    );
}

#[test]
fn hooks_install_config_does_not_stack_duplicates_on_repeat() {
    if skip_if_git_too_old() {
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
    assert_eq!(
        pre_commit.len(),
        2,
        "install must be idempotent (gate + L3 witness, no extras)"
    );
}

#[test]
fn hooks_install_config_refuses_when_host_git_too_old() {
    let (major, minor, patch) = host_git_version();
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
    if skip_if_git_too_old() {
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
    // And the config entries must still be written.
    let pre_commit = config_get_all(dir.path(), "hook.pre-commit.command");
    assert_eq!(
        pre_commit.len(),
        2,
        "pre-commit must install gate + witness"
    );
    let post_commit = config_get_all(dir.path(), "hook.post-commit.command");
    assert_eq!(
        post_commit.len(),
        1,
        "--pre-commit-only still installs post-commit"
    );
}

/// File-mode `hooks install` writes gate + L3 witness on pre-commit and a
/// managed post-commit. A full `git commit` + audit-chain fixture is left
/// to manual/operator verification — this pins the installed hook bodies.
#[test]
fn hooks_install_file_mode_writes_gate_and_l3_witness() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["hooks", "install"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
        .output()
        .expect("invoking anvil hooks install");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "hooks install must exit 0\nstdout: {stdout}\nstderr: {stderr}",
    );

    let pre_commit = std::fs::read_to_string(dir.path().join(".git/hooks/pre-commit"))
        .expect("file-mode pre-commit must be written");
    assert!(
        pre_commit.contains("anvil gate --progress"),
        "file-mode pre-commit must keep the quality gate: {pre_commit}",
    );
    assert!(
        pre_commit.contains("hook pre-commit"),
        "file-mode pre-commit must run L3 witness: {pre_commit}",
    );

    let post_commit = std::fs::read_to_string(dir.path().join(".git/hooks/post-commit"))
        .expect("file-mode post-commit must be written");
    assert!(
        post_commit.contains("hook post-commit"),
        "file-mode post-commit must bind HEAD: {post_commit}",
    );

    let pre_push = std::fs::read_to_string(dir.path().join(".git/hooks/pre-push"))
        .expect("file-mode pre-push must be written");
    assert!(
        pre_push.contains("hook pre-push"),
        "file-mode pre-push must stay on the L4 runtime: {pre_push}",
    );

    assert!(
        stdout.contains("L3 witness") || stdout.contains("hook pre-commit"),
        "install summary must name the witness step:\n{stdout}",
    );

    let status = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["hooks", "status"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("GIT_CEILING_DIRECTORIES", discovery_ceiling(dir.path()))
        .output()
        .expect("invoking anvil hooks status");
    let status_out = String::from_utf8_lossy(&status.stdout);
    assert!(status.status.success(), "hooks status must exit 0");
    assert!(
        status_out.contains("L3 witness") || status_out.contains("SHA-binding"),
        "hooks status must name the witness step:\n{status_out}",
    );
}
