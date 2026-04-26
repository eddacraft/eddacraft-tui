//! AIGUARD-003 validation: `anvil gate --profile ai` runs the curated
//! AI guardrail rule set as an allow-list, applies the strict-config
//! default, pins JSON output, and emits the `anvil.gate-result.v1`
//! envelope wrapping `anvil.diagnostic.v1` payloads.
//!
//! Companion to the colocated unit tests in
//! `crates/anvil-cli/src/commands/gate.rs`. End-to-end coverage lives
//! here because we need an empty workspace temp dir to ensure the
//! strict-config path fires deterministically.

use std::fs;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
fn ai_profile_emits_diagnostic_envelope_in_json_mode() {
    let dir = tempfile::tempdir().unwrap();

    // Empty workspace — architecture, policy, and command-safety all
    // hit their "no config" path. Under `--profile ai` strict_config
    // converts that into blocking diagnostics, which is exactly what
    // we want to assert: the envelope is well-formed even (especially)
    // when checks fail.
    let output = Command::new(ANVIL_BIN)
        .arg("gate")
        .arg("--profile")
        .arg("ai")
        .current_dir(dir.path())
        // Bypass the local licence pre-check; gate doesn't make API
        // calls, this just lets the test run without a live token.
        .env("ANVIL_DEV", "1")
        .output()
        .expect("failed to invoke anvil binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!("expected JSON envelope on stdout under --profile ai, got: {stdout}\nerror: {err}")
    });

    // Outer envelope identifies as `anvil.gate-result.v1` per the
    // diagnostic-envelope coordination spec.
    assert_eq!(parsed["schema"], "anvil.gate-result.v1");
    assert!(parsed["exit_code"].is_u64());
    assert!(parsed["summary"].is_object());
    assert!(parsed["diagnostics"].is_array());

    // Strict-config default elevates the missing-config skip into a
    // blocking diagnostic for at least one check; verify diagnostics
    // carry the inner-shape fields and the gate mode discriminator.
    let diagnostics = parsed["diagnostics"].as_array().unwrap();
    assert!(
        !diagnostics.is_empty(),
        "expected at least one diagnostic under strict-config: {parsed}"
    );
    let first = &diagnostics[0];
    assert_eq!(first["schema_version"], "anvil.diagnostic.v1");
    assert_eq!(first["mode"], "gate");
    assert!(first["id"].is_string());
    assert!(first["severity"].is_string());
    assert!(first["summary"].is_string());
    assert!(first["category"].is_string());
    assert!(first["source"]["rule_id"].is_string());
    assert!(first["source"]["source_module"].is_string());
    assert!(first["location"]["file"].is_string());
}

#[test]
fn ai_profile_check_set_excludes_toolchain_checks() {
    // `--profile ai` runs the curated allow-list — lint/test/coverage/
    // dependency are deliberately not in it. With `--progress` the
    // runner logs `▶ <name> running...` lines for each check it
    // executes; assert the toolchain checks never appear.
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(ANVIL_BIN)
        .arg("gate")
        .arg("--profile")
        .arg("ai")
        .arg("--progress")
        .arg("--no-tui")
        .current_dir(dir.path())
        .env("ANVIL_DEV", "1")
        .output()
        .expect("failed to invoke anvil binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    for excluded in [
        "lint running",
        "test running",
        "coverage running",
        "dependency running",
    ] {
        assert!(
            !stderr.contains(excluded),
            "ai profile must not run {excluded}; stderr was: {stderr}"
        );
    }

    // And it should run at least one curated check.
    assert!(
        stderr.contains("running")
            || stderr.contains("\u{25b6}")
            || stderr.contains("antipattern-scan"),
        "expected at least one curated check to run; stderr: {stderr}"
    );

    // Sanity: keep the temp dir variable alive until end of scope.
    let _ = fs::metadata(dir.path()).unwrap();
}
