//! EXCEPT-004: `anvil exception` integration tests.
//!
//! Pin the CLI contract end-to-end through the real binary: grant an
//! attributed record into the tracked store, list/show it with its
//! verdict, revoke it soft-delete-style, and keep the store honest
//! about attribution (a grant with no `--owner` and no git identity
//! refuses).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Fresh git repo with an identity, isolated HOME (no global git
/// config, no real anvil state).
fn repo_with_identity() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .env("HOME", &root)
            .output()
            .expect("git available");
        assert!(out.status.success(), "git {args:?}: {out:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "operator@example.test"]);
    git(&["config", "user.name", "Operator"]);
    (tmp, root)
}

fn anvil(root: &Path, args: &[&str]) -> Output {
    Command::new(ANVIL_BIN)
        .args(args)
        .current_dir(root)
        .env("HOME", root)
        .env("ANVIL_HOME", root.join(".anvil-home"))
        .output()
        .expect("spawn anvil")
}

fn stdout_json(output: &Output) -> Value {
    assert!(output.status.success(), "expected success: {output:?}");
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn grant_list_revoke_round_trip() {
    let (_tmp, root) = repo_with_identity();

    let granted = anvil(
        &root,
        &[
            "--json",
            "exception",
            "grant",
            "--policy",
            "AP-001",
            "--reason",
            "legacy module scheduled for removal",
            "--scope",
            "src/legacy/**",
            "--expires-in-days",
            "30",
        ],
    );
    let granted = stdout_json(&granted);
    let id = granted["id"]
        .as_str()
        .expect("grant reports id")
        .to_string();
    assert_eq!(granted["verdict"], "active");

    // The tracked store is what got written — not the legacy path.
    assert!(root.join("anvil/exceptions/store.json").exists());
    assert!(!root.join(".anvil/exceptions.json").exists());

    let listed = stdout_json(&anvil(&root, &["--json", "exception", "list"]));
    let rows = listed.as_array().expect("list is an array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"].as_str(), Some(id.as_str()));
    assert_eq!(rows[0]["verdict"], "active");
    assert_eq!(rows[0]["created_by"], "operator@example.test");

    let shown = stdout_json(&anvil(&root, &["--json", "exception", "show", &id]));
    assert_eq!(shown["policy_id"], "AP-001");
    assert_eq!(shown["scope"], "src/legacy/**");

    let revoked = anvil(
        &root,
        &[
            "--json",
            "exception",
            "revoke",
            &id,
            "--reason",
            "module removed",
        ],
    );
    assert!(revoked.status.success(), "{revoked:?}");

    let after = stdout_json(&anvil(&root, &["--json", "exception", "verify"]));
    let rows = after.as_array().expect("verify is an array");
    assert_eq!(rows[0]["verdict"], "revoked");
    assert_eq!(rows[0]["revoked_by"], "operator@example.test");
}

#[test]
fn grant_without_identity_or_owner_refuses() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["init", "-q", "-b", "main"])
        .env("HOME", &root)
        .output()
        .expect("git available");
    assert!(out.status.success());
    // No user.email/user.name in the repo, HOME isolated → no identity.
    let output = anvil(
        &root,
        &["exception", "grant", "--policy", "AP-001", "--reason", "x"],
    );
    assert!(!output.status.success(), "unattributed grant must refuse");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("attribution"), "stderr: {stderr}");
    assert!(!root.join("anvil/exceptions/store.json").exists());
}

#[test]
fn invalid_scope_refuses_with_nonzero_exit() {
    let (_tmp, root) = repo_with_identity();
    let output = anvil(
        &root,
        &[
            "exception",
            "grant",
            "--policy",
            "AP-001",
            "--reason",
            "x",
            "--scope",
            "src/[oops",
        ],
    );
    assert!(!output.status.success(), "invalid glob must refuse");
    assert!(!root.join("anvil/exceptions/store.json").exists());
}
