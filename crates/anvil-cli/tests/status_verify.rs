//! LAUNCH-008 / LAUNCH-012 integration: `anvil status --verify` is a
//! non-mutating activation probe that prints the literal protection
//! state vocabulary, in both human and JSON forms, on a fresh repo.
//!
//! These tests are guard-rails for the council-locked truthfulness
//! requirement: surfaces must NEVER claim `protecting` unless the
//! diagnostic literally backs it. Until LAUNCH-009 / LAUNCH-011 land,
//! a fresh repo can only legitimately reach `needs_action`.

use std::fs;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run_status_verify(workdir: &std::path::Path, extra_args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("status")
        .arg("--verify")
        .args(extra_args)
        .current_dir(workdir)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.output().expect("failed to invoke anvil binary")
}

#[test]
fn status_verify_on_fresh_repo_renders_needs_action() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_status_verify(dir.path(), &[]);
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
        dir.path().join(".anvilrc"),
        "{this is not valid in any format::",
    )
    .unwrap();

    let out = run_status_verify(dir.path(), &[]);
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

#[test]
fn status_verify_json_keys_are_stable() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();

    let out = run_status_verify(dir.path(), &[]);
    let mut cmd = Command::new(ANVIL_BIN);
    let json_out = cmd
        .arg("--no-tui")
        .arg("--json")
        .arg("status")
        .arg("--verify")
        .current_dir(dir.path())
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

    // Also ensure default (non-verify) status JSON now embeds activation.
    let _ = out; // silence unused warning if first invocation wasn't used here
}

#[test]
fn status_default_json_embeds_activation_block() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvilrc"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();
    let out = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("--json")
        .arg("status")
        .current_dir(dir.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.get("activation").is_some(), "activation block missing from `anvil --json status`: {parsed}");
    assert_eq!(parsed["activation"]["state"], "needs_action");
}

#[test]
fn status_verify_is_idempotent_and_does_not_mutate_config() {
    // LAUNCH-012: re-running verification performs no writes and
    // leaves existing config unchanged on repeated runs.
    let dir = tempfile::tempdir().unwrap();
    let rc = dir.path().join(".anvilrc");
    fs::write(&rc, "profile: default\nchecks: []\n").unwrap();
    let before = fs::metadata(&rc).unwrap().modified().unwrap();
    let _ = run_status_verify(dir.path(), &[]);
    let _ = run_status_verify(dir.path(), &[]);
    let after = fs::metadata(&rc).unwrap().modified().unwrap();
    assert_eq!(
        before, after,
        "status --verify must not mutate `.anvilrc` mtime"
    );
}
