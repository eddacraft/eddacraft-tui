//! Issue #3914: `anvil config convert` and `anvil migrate format` must
//! rewrite embedded `format` metadata to the destination spelling.

use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run_anvil(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(ANVIL_BIN)
        .args(args)
        .current_dir(root)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("spawn anvil")
}

fn write_yml_with_format_meta(root: &Path) {
    std::fs::write(
        root.join(".anvil.yml"),
        "schema_version: \"1.0.0\"\nplanning_dir: \"plans\"\nformat: yml\nchecks: []\n",
    )
    .unwrap();
}

fn assert_success(output: &std::process::Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn config_convert_yml_to_json_rewrites_format_metadata() {
    let root = tempdir().unwrap();
    write_yml_with_format_meta(root.path());

    let output = run_anvil(
        root.path(),
        &["--no-tui", "config", "convert", "--to", "json"],
    );
    assert_success(&output, "config convert --to json");

    let body = std::fs::read_to_string(root.path().join(".anvil.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["format"], "json", "{body}");
}

#[test]
fn migrate_format_yml_to_json_rewrites_format_metadata() {
    let root = tempdir().unwrap();
    write_yml_with_format_meta(root.path());

    let output = run_anvil(
        root.path(),
        &["--no-tui", "migrate", "format", "--format", "json"],
    );
    assert_success(&output, "migrate format --format json");

    let body = std::fs::read_to_string(root.path().join(".anvil.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["format"], "json", "{body}");
}

#[test]
fn config_convert_stdout_rewrites_format_metadata() {
    let root = tempdir().unwrap();
    write_yml_with_format_meta(root.path());

    let output = run_anvil(
        root.path(),
        &["--no-tui", "config", "convert", "--to", "json", "--stdout"],
    );
    assert_success(&output, "config convert --stdout --to json");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["format"], "json", "{stdout}");
    assert!(
        !root.path().join(".anvil.json").exists(),
        "--stdout must not write .anvil.json"
    );
}

#[test]
fn config_convert_remove_old_rewrites_format_metadata() {
    let root = tempdir().unwrap();
    std::fs::write(
        root.path().join(".anvil.json"),
        "{\n  \"format\": \"json\",\n  \"checks\": []\n}\n",
    )
    .unwrap();

    let output = run_anvil(
        root.path(),
        &[
            "--no-tui",
            "config",
            "convert",
            "--to",
            "toml",
            "--remove-old",
        ],
    );
    assert_success(&output, "config convert --to toml --remove-old");

    let body = std::fs::read_to_string(root.path().join(".anvil.toml")).unwrap();
    assert!(body.contains("format = \"toml\""), "{body}");
    assert!(!body.contains("format = \"json\""), "{body}");
    assert!(!root.path().join(".anvil.json").exists());
}
