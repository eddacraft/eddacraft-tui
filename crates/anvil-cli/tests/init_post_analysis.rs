//! LAUNCH-004 validation: `anvil init --force` lands on a real first
//! signal of value (the sample analysis section) rather than a flat
//! "Run `anvil doctor`" stub.

use std::fs;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
fn init_force_prints_post_init_analysis_section() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("sample.ts"),
        "export const x = 1;\nexport const y = 2;\n",
    )
    .unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("init")
        .arg("--force")
        .current_dir(dir.path())
        // Skip the welcome chain so the run terminates cleanly.
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil binary");

    assert!(
        output.status.success(),
        "anvil init exited with {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("First scan"),
        "expected 'First scan' section in stdout, got:\n{stdout}",
    );
    assert!(
        stdout.contains("Scanned"),
        "expected 'Scanned N file(s)' line in stdout, got:\n{stdout}",
    );
}

#[test]
fn init_force_post_analysis_silent_for_empty_tree() {
    // No source files in the temp dir — analysis should produce no
    // section rather than printing "0 files" noise. The init command
    // itself must still succeed.
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("init")
        .arg("--force")
        .current_dir(dir.path())
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil binary");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("First scan"),
        "expected no analysis section for an empty tree, got:\n{stdout}",
    );
}

#[test]
fn init_json_mode_skips_post_analysis() {
    // JSON mode preserves the existing config schema — the analysis is
    // a human-facing wow surface, not part of the machine contract.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("sample.ts"), "export const x = 1;\n").unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("--json")
        .arg("init")
        .arg("--force")
        .current_dir(dir.path())
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil binary");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("First scan"),
        "JSON mode should not emit the human analysis section, got:\n{stdout}",
    );
    // Sanity: JSON output is still a valid config document.
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("JSON mode stdout must parse as JSON");
    assert!(parsed.get("schemaVersion").is_some());
}
