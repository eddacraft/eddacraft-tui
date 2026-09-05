//! EMBERRS-001: the inactive Ember surface cannot touch historical data.

use std::process::Command;

fn ember(root: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_anvil"));
    cmd.current_dir(root)
        .args(["ember", "list"])
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1")
        .env("ANVIL_LOG", "off")
        .env_remove("ANVIL_EMBER")
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_ADMIN_KEY");
    cmd
}

#[test]
fn ember_closed_gate_precedes_database_access_and_preserves_data() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".anvil")).unwrap();
    let db = root.path().join(".anvil/ember.db");
    let sentinel = b"historical database deliberately unreadable as SQLite";
    std::fs::write(&db, sentinel).unwrap();
    for value in [None, Some("0"), Some("true"), Some(" 1"), Some("1 ")] {
        let mut cmd = ember(root.path());
        cmd.arg("--json").env("ANVIL_DEV", "1");
        if let Some(value) = value {
            cmd.env("ANVIL_EMBER", value);
        }
        let output = cmd.output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(json["error"], "feature_disabled");
        assert_eq!(json["flag"], "ember.enabled");
        assert_eq!(std::fs::read(&db).unwrap(), sentinel);
    }
}

#[test]
fn ember_global_json_and_plain_refusal_are_truthful() {
    let root = tempfile::tempdir().unwrap();
    let output = ember(root.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ember.enabled"));
    let output = Command::new(env!("CARGO_BIN_EXE_anvil"))
        .current_dir(root.path())
        .args(["--json", "ember", "list"])
        .env_remove("ANVIL_EMBER")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["error"], "feature_disabled");
    assert!(!root.path().join(".anvil/ember.db").exists());
}

#[test]
fn ember_explicit_opt_in_reaches_rust_reader_without_creating_database() {
    let root = tempfile::tempdir().unwrap();
    let output = ember(root.path())
        .arg("--json")
        .env("ANVIL_EMBER", "1")
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["database_found"], false);
    assert_ne!(json["error"], "feature_disabled");
    assert!(!root.path().join(".anvil/ember.db").exists());
}

#[test]
fn ember_is_hidden_from_root_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_anvil"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        !String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.trim_start().starts_with("ember "))
    );
}
