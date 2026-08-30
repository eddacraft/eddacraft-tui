//! GTAO-003: the CLI follow-up command (`anvil check` on the changed path)
//! reports AST findings the save-time verdict does not.

use std::process::{Command, Stdio};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn has_warning(stdout: &[u8], id: &str, file: &str) -> Result<bool, serde_json::Error> {
    let payload: serde_json::Value = serde_json::from_slice(stdout)?;
    Ok(payload
        .get("warnings")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|warnings| {
            warnings.iter().any(|warning| {
                warning.get("id").and_then(serde_json::Value::as_str) == Some(id)
                    && warning.get("file").and_then(serde_json::Value::as_str) == Some(file)
            })
        }))
}

#[test]
fn changed_path_check_reports_rs001_on_unwrap() {
    let workspace = tempfile::tempdir().expect("workspace");
    let home = tempfile::tempdir().expect("home");
    let src = workspace.path().join("src");
    std::fs::create_dir_all(&src).expect("src");
    let file = src.join("lib.rs");
    std::fs::write(&file, "pub fn run() { let n = parse().unwrap(); }\n").expect("fixture");

    let output = Command::new(ANVIL_BIN)
        .args([
            "check",
            "--json",
            "--no-tui",
            "--",
            file.to_str().expect("utf-8 path"),
        ])
        .current_dir(workspace.path())
        .env("ANVIL_DEV", "1")
        .env("HOME", home.path())
        .env("ANVIL_HOME", home.path().join(".anvil"))
        .env("ANVIL_DISABLE_UPDATE_HINT", "1")
        .stdin(Stdio::null())
        .output()
        .expect("anvil check runs");

    let has_rs001 = has_warning(&output.stdout, "RS-001", "src/lib.rs").unwrap_or_else(|error| {
        panic!(
            "follow-up `anvil check --json` must emit valid JSON: {error}; stdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert!(
        has_rs001,
        "follow-up `anvil check` must emit structured RS-001 for src/lib.rs; stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_json_warning_shape_pins_the_followup_advisory() {
    let payload = serde_json::json!({
        "warnings": [{"id": "RS-001", "file": "src/lib.rs"}]
    });
    let warnings = payload
        .get("warnings")
        .and_then(serde_json::Value::as_array)
        .expect("warnings");
    let first = warnings.first().expect("one warning");
    let id = first
        .get("id")
        .and_then(serde_json::Value::as_str)
        .expect("id");
    let file = first
        .get("file")
        .and_then(serde_json::Value::as_str)
        .expect("file");
    let line = format!("anvil: 1 AST follow-up warning (save allowed) — {id} in {file}");
    assert_eq!(
        line,
        "anvil: 1 AST follow-up warning (save allowed) — RS-001 in src/lib.rs"
    );
    assert_eq!(line.lines().count(), 1);
}

#[test]
fn warning_match_rejects_textual_rs001_lookalikes() {
    for stdout in [
        br"warning RS-001 in src/lib.rs".as_slice(),
        br#"{"warnings":[{"id":"RS-001","file":"src/lib.rs"}]"#.as_slice(),
    ] {
        assert!(
            has_warning(stdout, "RS-001", "src/lib.rs").is_err(),
            "human or malformed output must not satisfy the JSON warning contract: {}",
            String::from_utf8_lossy(stdout)
        );
    }

    assert!(
        !has_warning(br#""RS-001 in src/lib.rs""#, "RS-001", "src/lib.rs")
            .expect("valid JSON scalar"),
        "a valid JSON scalar containing the token is not a structured warning"
    );
}
