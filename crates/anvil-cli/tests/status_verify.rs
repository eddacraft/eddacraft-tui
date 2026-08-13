//! LAUNCH-008 / LAUNCH-012 integration: `anvil status --verify` is a
//! non-mutating activation probe that prints the literal protection
//! state vocabulary, in both human and JSON forms, on a fresh repo.
//!
//! These tests are guard-rails for the council-locked truthfulness
//! requirement: surfaces must NEVER claim `protecting` unless the
//! diagnostic literally backs it.
//!
//! ## HOME isolation
//!
//! Every test overrides `HOME` (and `USERPROFILE` on Windows) to a
//! per-test tempdir so the MCP probe sees an empty home regardless
//! of what's on the developer's machine. Without this, the tests
//! would silently pass or fail depending on whether the developer
//! happens to have `~/.cursor/mcp.json` configured for anvil.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run_status_verify_with_home(workdir: &Path, home: &Path, extra_args: &[&str]) -> Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("status")
        .arg("--verify")
        .args(extra_args)
        .current_dir(workdir)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

#[cfg(not(target_os = "windows"))]
#[test]
fn status_verify_on_fresh_repo_renders_needs_action() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_status_verify_with_home(dir.path(), home.path(), &[]);
    assert!(
        out.status.success(),
        "anvil status --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: needs_action"),
        "expected `state: needs_action` on empty repo, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("state: protecting"),
        "fresh empty repo MUST NOT claim protection, got:\n{stdout}"
    );
}

#[test]
fn status_verify_on_repo_with_invalid_config_renders_error() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvil.yaml"),
        "{this is not valid in any format::",
    )
    .unwrap();

    let home = tempfile::tempdir().unwrap();
    let out = run_status_verify_with_home(dir.path(), home.path(), &[]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: error"),
        "expected `state: error` on invalid config, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("state: protecting") && !stdout.contains("state: watching"),
        "error state must not also claim coverage, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn status_verify_json_keys_are_stable() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();

    // Both human and JSON modes must agree on `needs_action` for a
    // repo with valid config but no MCP install — the test covers
    // both invocations so a regression in either path is caught.
    let human_out = run_status_verify_with_home(dir.path(), home.path(), &[]);
    assert!(human_out.status.success());
    let human_stdout = String::from_utf8_lossy(&human_out.stdout);
    assert!(
        human_stdout.contains("state: needs_action"),
        "human render expected `state: needs_action`, got:\n{human_stdout}"
    );

    let json_out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("--json")
        .arg("status")
        .arg("--verify")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil");
    assert!(json_out.status.success());

    let stdout = String::from_utf8_lossy(&json_out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim_start_matches('\u{feff}'))
        .unwrap_or_else(|e| panic!("JSON parse failed: {e}\nstdout:\n{stdout}"));

    for key in [
        "state",
        "headline",
        "config",
        "mcp",
        "watch",
        "baseline_present",
        "last_error",
        "all_languages_unsupported",
    ] {
        assert!(
            parsed.get(key).is_some(),
            "missing key `{key}` in --verify JSON: {parsed}"
        );
    }
    assert_eq!(parsed["config"], "valid");
    // Fresh repo with valid config but no MCP install yet: state must
    // be `needs_action`, never `protecting`.
    assert_eq!(parsed["state"], "needs_action");

    // Sanity-check that the human render didn't bleed into JSON mode.
    assert!(
        !stdout.contains("ACTIVATION\n"),
        "human header leaked into JSON output:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn status_default_json_embeds_activation_block() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();
    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("--json")
        .arg("status")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        parsed.get("activation").is_some(),
        "activation block missing from `anvil --json status`: {parsed}"
    );
    assert_eq!(parsed["activation"]["state"], "needs_action");
}

#[test]
fn status_verify_is_idempotent_and_does_not_mutate_workdir() {
    // LAUNCH-012: re-running verification performs no writes and
    // leaves existing state unchanged. We snapshot the entire
    // workdir's path → mtime map so a future regression that writes
    // to `.anvil/`, a sibling file, or a freshly-created path is
    // caught — not just the config file.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join(".anvil")).unwrap();

    let snapshot = |root: &std::path::Path| -> Vec<(std::path::PathBuf, std::time::SystemTime)> {
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry.unwrap();
            let m = entry.metadata().unwrap();
            // Use modified time only; atime can be touched by reads
            // on non-noatime mounts, which is allowed for a probe.
            let mtime = m.modified().unwrap();
            out.push((entry.path().to_path_buf(), mtime));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    };

    let before = snapshot(dir.path());
    let _ = run_status_verify_with_home(dir.path(), home.path(), &[]);
    let _ = run_status_verify_with_home(dir.path(), home.path(), &[]);
    let after = snapshot(dir.path());

    assert_eq!(
        before.len(),
        after.len(),
        "status --verify created or removed entries: before={before:?}, after={after:?}"
    );
    for ((bp, bt), (ap, at)) in before.iter().zip(after.iter()) {
        assert_eq!(bp, ap, "path drift: {bp:?} vs {ap:?}");
        assert_eq!(
            bt, at,
            "status --verify mutated mtime of {bp:?}: {bt:?} → {at:?}"
        );
    }
}
