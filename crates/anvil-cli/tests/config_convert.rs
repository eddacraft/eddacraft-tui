//! Issues #3914/#3962: `anvil config convert` and `anvil migrate format`
//! rewrite embedded `format` metadata for the destination. Requested
//! `.yml`/`.yaml` filenames stay distinct, while either YAML spelling uses
//! canonical `yaml` metadata.

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

/// Issue #3962: `yml` remains a supported destination filename spelling,
/// while embedded and machine-readable metadata use canonical `yaml`.
#[test]
fn yaml_destination_tokens_use_canonical_metadata_across_owned_writers() {
    for (context, requested, args) in [
        (
            "config convert --to yml --json",
            "yml",
            vec!["--no-tui", "--json", "config", "convert", "--to", "yml"],
        ),
        (
            "config convert --to yaml --json",
            "yaml",
            vec!["--no-tui", "--json", "config", "convert", "--to", "yaml"],
        ),
        (
            "migrate format --format yml --json",
            "yml",
            vec!["--no-tui", "--json", "migrate", "format", "--format", "yml"],
        ),
        (
            "migrate format --format yaml --json",
            "yaml",
            vec![
                "--no-tui", "--json", "migrate", "format", "--format", "yaml",
            ],
        ),
    ] {
        let root = tempdir().unwrap();
        std::fs::write(
            root.path().join(".anvil.json"),
            "{\n  \"format\": \"json\",\n  \"checks\": []\n}\n",
        )
        .unwrap();

        let output = run_anvil(root.path(), &args);
        assert_success(&output, context);

        let destination = root.path().join(format!(".anvil.{requested}"));
        assert!(
            destination.is_file(),
            "{context} must retain the requested destination extension"
        );

        let body = std::fs::read_to_string(&destination).unwrap();
        let converted: serde_json::Value = serde_yaml::from_str(&body).unwrap();
        assert_eq!(converted["format"], "yaml", "{context}: {body}");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let object = envelope
            .as_object()
            .unwrap_or_else(|| panic!("{context} envelope must be an object: {envelope}"));
        assert_eq!(
            object.len(),
            3,
            "{context} must retain exactly the stable three-field write envelope: {envelope}"
        );

        let reported_source = object["source"]
            .as_str()
            .unwrap_or_else(|| panic!("{context} source must be a string: {envelope}"));
        assert!(
            reported_source.ends_with(".anvil.json"),
            "{context}: {envelope}"
        );

        let reported_destination = object["destination"]
            .as_str()
            .unwrap_or_else(|| panic!("{context} destination must be a string: {envelope}"));
        assert!(
            reported_destination.ends_with(&format!(".anvil.{requested}")),
            "{context}: {envelope}"
        );

        let source_removed = object["source_removed"]
            .as_bool()
            .unwrap_or_else(|| panic!("{context} source_removed must be a bool: {envelope}"));
        assert!(!source_removed, "{context}: {envelope}");
    }

    for requested in ["yml", "yaml"] {
        let root = tempdir().unwrap();
        std::fs::write(
            root.path().join(".anvil.json"),
            "{\n  \"format\": \"json\",\n  \"checks\": []\n}\n",
        )
        .unwrap();

        let output = run_anvil(
            root.path(),
            &[
                "--no-tui", "--json", "config", "convert", "--to", requested, "--stdout",
            ],
        );
        assert_success(
            &output,
            &format!("config convert --to {requested} --stdout --json"),
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(envelope["format"], "yaml", "{requested}: {envelope}");

        let converted: serde_json::Value =
            serde_yaml::from_str(envelope["converted"].as_str().unwrap()).unwrap();
        assert_eq!(converted["format"], "yaml", "{requested}: {converted}");
        assert!(
            !root.path().join(format!(".anvil.{requested}")).exists(),
            "--stdout must not write the requested destination"
        );
    }
}
