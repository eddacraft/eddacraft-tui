//! GTAO-003: the CLI follow-up command (`anvil check` on the changed path)
//! reports AST findings the save-time verdict does not.

use std::process::{Command, Stdio};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
fn changed_path_check_reports_rs001_on_unwrap() {
    let workspace = tempfile::tempdir().expect("workspace");
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
        .stdin(Stdio::null())
        .output()
        .expect("anvil check runs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("RS-001"),
        "follow-up `anvil check` must emit RS-001 for unwrap; stdout={stdout}\nstderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !stdout.contains("\"command\":\"gate\"") && !stdout.contains("--all"),
        "follow-up output must not look like a full gate"
    );
}
