//! CIB-087: `anvil report-fp --list` reads the local false-positive sidecar.

use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn run_anvil(home: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(args)
        .current_dir(home)
        .env("ANVIL_HOME", home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.env_remove("ANVIL_TOUCH_PROJECT_STATE");
    cmd.env_remove("TRACEPARENT");
    cmd.output().expect("spawn anvil")
}

#[test]
fn report_fp_list_reads_local_reports_without_plaintext_path_or_snippet() {
    let home = tempdir().expect("anvil home");
    let source = home.path().join("src").join("secret.rs");
    std::fs::create_dir_all(source.parent().expect("parent")).expect("mkdir");
    std::fs::write(&source, "let secret = \"sk-test\";\n").expect("write source");
    let source_arg = format!("{}:1", source.display());

    let record = run_anvil(
        home.path(),
        &[
            "--no-tui",
            "report-fp",
            "ANV-CORE-001",
            &source_arg,
            "--include-snippet",
        ],
    );
    assert!(
        record.status.success(),
        "record report-fp failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&record.stdout),
        String::from_utf8_lossy(&record.stderr)
    );

    let listed = run_anvil(home.path(), &["--json", "report-fp", "--list"]);
    assert!(
        listed.status.success(),
        "list report-fp failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&listed.stdout),
        String::from_utf8_lossy(&listed.stderr)
    );

    let stdout = String::from_utf8_lossy(&listed.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("json list output: {err}: {stdout}"));
    assert_eq!(payload["count"], 1);
    let report = &payload["reports"][0];
    assert_eq!(report["check_id"], "ANV-CORE-001");
    assert_eq!(report["line"], 1);
    assert!(
        report["hashed_path"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "hashed path should be present: {payload}"
    );
    assert!(
        !stdout.contains(&source.to_string_lossy().to_string()),
        "plaintext path leaked in list output: {stdout}"
    );
    assert!(
        !stdout.contains("sk-test"),
        "opt-in snippet leaked in list output: {stdout}"
    );
}

#[test]
fn report_fp_list_empty_sidecar_is_clean() {
    let home = tempdir().expect("anvil home");
    let listed = run_anvil(home.path(), &["--json", "report-fp", "--list"]);
    assert!(
        listed.status.success(),
        "list report-fp failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&listed.stdout),
        String::from_utf8_lossy(&listed.stderr)
    );
    let stdout = String::from_utf8_lossy(&listed.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("json list output: {err}: {stdout}"));
    assert_eq!(payload["count"], 0);
    assert_eq!(
        payload["reports"].as_array().expect("reports array").len(),
        0
    );
}
