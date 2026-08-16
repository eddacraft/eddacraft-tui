//! #3913: parse/schema failures of discovered project config use exit 4.
//!
//! Top-level help documents `EXIT_CONFIG_ERROR` (4) for invalid config files.
//! These process tests pin `check`, `gate`, `gate-config`, `watch` startup,
//! and `architecture` to that contract. Runtime/tool failures stay on 1.

use std::path::Path;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

const SCHEMA_INVALID: &str = "gate:\n  checks: \"not-an-object\"\n";
const PARSE_INVALID: &str = "gate: [\n  this is not yaml\n";

fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create workspace");
    let root = dir.path();
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .status()
        .expect("run git init");
    assert!(status.success(), "git init failed");
    std::fs::create_dir_all(root.join("src")).expect("create src");
    std::fs::write(root.join("src/lib.rs"), "pub fn ok() {}\n").expect("write source file");
    dir
}

fn write_config(root: &Path, body: &str) {
    std::fs::write(root.join(".anvil.yaml"), body).expect("write .anvil.yaml");
}

fn run_anvil(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(ANVIL_BIN)
        .args(["--no-tui"])
        .args(args)
        .current_dir(root)
        .env("ANVIL_HOME", root)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join(".config"))
        .env("XDG_CACHE_HOME", root.join(".cache"))
        .env("XDG_DATA_HOME", root.join(".local/share"))
        .env("USERPROFILE", root)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env_remove("ANVIL_TOUCH_PROJECT_STATE")
        .env_remove("TRACEPARENT")
        .output()
        .unwrap_or_else(|err| panic!("invoke anvil {}: {err}", args.join(" ")))
}

fn assert_invalid_config_exit(output: &std::process::Output, command: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(4),
        "{command} must exit 4 (EXIT_CONFIG_ERROR) on invalid project config\nstdout={stdout}\nstderr={stderr}"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("invalid config")
            || combined.contains("parse")
            || combined.contains(".anvil.yaml")
            || combined.contains("gate.checks"),
        "{command} must keep actionable path/context\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn check_schema_failure_exits_config_error() {
    let dir = workspace();
    write_config(dir.path(), SCHEMA_INVALID);
    let output = run_anvil(dir.path(), &["check", "src/lib.rs"]);
    assert_invalid_config_exit(&output, "check");
}

#[test]
fn gate_schema_failure_exits_config_error() {
    let dir = workspace();
    write_config(dir.path(), SCHEMA_INVALID);
    let output = run_anvil(dir.path(), &["gate"]);
    assert_invalid_config_exit(&output, "gate");
}

#[test]
fn gate_config_schema_failure_exits_config_error() {
    let dir = workspace();
    write_config(dir.path(), SCHEMA_INVALID);
    let output = run_anvil(dir.path(), &["gate-config", "--list"]);
    assert_invalid_config_exit(&output, "gate-config --list");
}

#[test]
fn watch_parse_failure_exits_config_error() {
    let dir = workspace();
    write_config(dir.path(), PARSE_INVALID);
    let output = run_anvil(dir.path(), &["watch", "--action", "none"]);
    assert_invalid_config_exit(&output, "watch --action none");
}

#[test]
fn architecture_parse_failure_exits_config_error() {
    let dir = workspace();
    write_config(dir.path(), PARSE_INVALID);
    let output = run_anvil(dir.path(), &["architecture", "validate"]);
    assert_invalid_config_exit(&output, "architecture validate");
}

#[test]
fn check_json_schema_failure_keeps_path_on_stderr() {
    let dir = workspace();
    write_config(dir.path(), SCHEMA_INVALID);
    let output = run_anvil(dir.path(), &["--json", "check", "src/lib.rs"]);
    assert_invalid_config_exit(&output, "check --json");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed: serde_json::Value = serde_json::from_str(&stderr)
        .unwrap_or_else(|err| panic!("JSON error envelope on stderr ({err})\nstderr={stderr}"));
    let error = parsed["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("gate.checks") || error.contains("invalid config"),
        "JSON error must keep schema path/context: {error}"
    );
}

#[test]
fn architecture_missing_file_stays_runtime_error() {
    let dir = workspace();
    let output = run_anvil(
        dir.path(),
        &["architecture", "validate", "--file", "missing.yaml"],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "missing --file is a runtime/tool failure, not invalid project config\nstdout={stdout}\nstderr={stderr}"
    );
}
