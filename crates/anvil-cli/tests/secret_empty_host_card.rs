//! #3917: `anvil --json check` must report SECRET-CREDIT-CARD for a
//! malformed empty-host URL, and must keep the CIB-323 exemption for a
//! valid host. Digits are assembled at runtime so the fixture source never
//! contains a 16-digit run.

use std::fs;
use std::process::Command;

use serde_json::Value;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn visa_test_pan() -> String {
    ["4111", "1111", "1111", "1111"].concat()
}

fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create workspace");
    let root = dir.path();
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .status()
        .expect("run git init");
    assert!(status.success(), "git init failed");
    fs::create_dir_all(root.join("src")).expect("create src");
    dir
}

fn check_file(root: &std::path::Path, relative: &str) -> (bool, Value, String, String) {
    let output = Command::new(ANVIL_BIN)
        .args(["--no-tui", "--json", "check", relative])
        .current_dir(root)
        .env("ANVIL_HOME", root)
        .env("HOME", root)
        .env("USERPROFILE", root)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env_remove("ANVIL_TOUCH_PROJECT_STATE")
        .env_remove("TRACEPARENT")
        .output()
        .expect("invoke anvil --json check");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let json = serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!("check {relative} did not emit JSON ({err})\nstdout: {stdout}\nstderr: {stderr}")
    });
    (output.status.success(), json, stdout, stderr)
}

fn warning_ids(value: &Value) -> Vec<String> {
    value["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .map(|w| w["id"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[test]
fn empty_host_url_reports_secret_credit_card_over_json_check() {
    let digits = visa_test_pan();
    let workspace = workspace();
    let root = workspace.path();
    fs::write(
        root.join("src/empty-host.ts"),
        format!("const emptyHost = \"https:///accounts/{digits}/events\";\n"),
    )
    .expect("write empty-host fixture");

    let (ok, json, stdout, stderr) = check_file(root, "src/empty-host.ts");
    assert!(
        !ok,
        "empty-host card must be blocking; stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        json["summary"]["total"].as_u64().unwrap_or(0) >= 1,
        "summary.total must be non-zero, got {json}"
    );
    let ids = warning_ids(&json);
    assert!(
        ids.iter().any(|id| id == "SECRET-CREDIT-CARD"),
        "must report SECRET-CREDIT-CARD, got {ids:?}"
    );
    assert!(
        !stdout.contains(&digits) && !stderr.contains(&digits),
        "JSON/human/hook streams must not leak the raw PAN\nstdout={stdout}\nstderr={stderr}"
    );
    let message = json["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|w| w["id"] == "SECRET-CREDIT-CARD")
        .and_then(|w| w["message"].as_str())
        .unwrap_or("");
    assert!(
        message.contains("[REDACTED]"),
        "JSON message must keep redaction: {message:?}"
    );
}

#[test]
fn valid_host_url_path_stays_clean_over_json_check() {
    let digits = visa_test_pan();
    let workspace = workspace();
    let root = workspace.path();
    fs::write(
        root.join("src/valid-host.ts"),
        format!("const reel = 'https://www.facebook.com/reel/{digits}';\n"),
    )
    .expect("write valid-host fixture");

    let (ok, json, stdout, stderr) = check_file(root, "src/valid-host.ts");
    assert!(
        ok,
        "valid-host URL path must stay clean; stdout={stdout}\nstderr={stderr}"
    );
    let ids = warning_ids(&json);
    assert!(
        !ids.iter().any(|id| id == "SECRET-CREDIT-CARD"),
        "valid host must keep the CIB-323 exemption, got {ids:?}"
    );
    assert!(
        !stdout.contains(&digits) && !stderr.contains(&digits),
        "output must not leak the raw PAN\nstdout={stdout}\nstderr={stderr}"
    );
}
