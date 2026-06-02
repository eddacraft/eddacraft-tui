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

    // Empty workspace — the curated AI guardrail set runs:
    // secret-detection, import-boundaries, antipattern-scan, policy,
    // command-safety. Under `--profile ai`, strict_config marks the
    // three config-gap checks (import-boundaries, policy,
    // command-safety — the canonical names shown in JSON output;
    // import-boundaries dispatches through the `architecture`
    // internal check) as `requires_config = true` rather than
    // blocking diagnostics (CIB-011 / #1803 — the pre-CIB-011
    // behaviour scored 1/5 = 20% on a fresh repo, which made Anvil
    // look broken on day one). The remaining two checks
    // (secret-detection, antipattern-scan) actually run; both pass
    // on an empty workspace, so the gate exits 0 and the envelope
    // reports `summary.config_gaps = 3` alongside an empty
    // diagnostics array.
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

    // CIB-011 / #1803 — empty workspace under strict-mode must PASS
    // (config gaps are not failures). CLAWP-011 still applies in
    // spirit: the in-band `exit_code` MUST match the process status
    // so a regression that drifts one without the other is caught.
    assert!(
        output.status.success(),
        "ai-profile gate on empty workspace must succeed under CIB-011 (config gaps are not failures); got status={:?}, stdout={stdout}",
        output.status
    );
    let process_code = output
        .status
        .code()
        .expect("ai-profile gate process exited via signal, not a normal exit code");

    // Outer envelope identifies as `anvil.gate-result.v1` per the
    // diagnostic-envelope coordination spec.
    assert_eq!(parsed["schema"], "anvil.gate-result.v1");
    let envelope_code = parsed["exit_code"]
        .as_u64()
        .expect("envelope `exit_code` must be a u64");
    let envelope_code_i32 = i32::try_from(envelope_code)
        .unwrap_or_else(|_| panic!("envelope `exit_code` ({envelope_code}) does not fit in i32"));
    assert_eq!(
        envelope_code_i32, process_code,
        "in-band envelope exit_code ({envelope_code}) must match the process status code ({process_code}); \
         stdout={stdout}"
    );
    assert!(parsed["summary"].is_object());
    assert!(parsed["diagnostics"].is_array());

    // CIB-011 contract: no diagnostics whose summary contains
    // "Skipping" as its reason on a fresh repo. The pre-CIB-011
    // behaviour produced three such diagnostics ("Strict mode
    // (profile=ai): X requires configuration. No X config found ...
    // Skipping.").
    let diagnostics = parsed["diagnostics"].as_array().unwrap();
    for diag in diagnostics {
        let summary = diag["summary"].as_str().unwrap_or("");
        let remediation = diag["remediation_hint"].as_str().unwrap_or("");
        assert!(
            !summary.contains("Skipping") && !remediation.contains("Skipping"),
            "CIB-011: no FAIL diagnostic should carry `Skipping` as its reason; got: {diag}"
        );
    }

    // The strict-mode skips are surfaced via `summary.config_gaps`
    // rather than via `diagnostics[]`. On an empty workspace we expect
    // 3 gaps (architecture, policy, command-safety).
    let summary = &parsed["summary"];
    let config_gaps = summary["config_gaps"].as_u64().unwrap_or(0);
    assert_eq!(
        config_gaps, 3,
        "expected 3 config gaps (architecture, policy, command-safety) on empty workspace; summary: {summary}"
    );

    // overall_passed mirrors the process exit; the gate is vacuously
    // green when only config-gap checks would have failed.
    assert_eq!(summary["overall_passed"], true);

    // If any diagnostic DOES surface (e.g. the empty-workspace
    // antipattern scan produces one in a future regression), the
    // envelope shape must still be valid — for EVERY diagnostic, not
    // just the first (CLAWP-040: validating only `diagnostics.first()`
    // let a malformed tail diagnostic pass while the head looked fine).
    for (idx, diag) in diagnostics.iter().enumerate() {
        assert_eq!(
            diag["schema_version"], "anvil.diagnostic.v1",
            "diagnostic[{idx}]: {diag}"
        );
        assert_eq!(diag["mode"], "gate", "diagnostic[{idx}]: {diag}");
        assert!(
            diag["id"].is_string(),
            "diagnostic[{idx}].id not a string: {diag}"
        );
        assert!(
            diag["severity"].is_string(),
            "diagnostic[{idx}].severity not a string: {diag}"
        );
        assert!(
            diag["summary"].is_string(),
            "diagnostic[{idx}].summary not a string: {diag}"
        );
        assert!(
            diag["category"].is_string(),
            "diagnostic[{idx}].category not a string: {diag}"
        );
        assert!(
            diag["source"]["rule_id"].is_string(),
            "diagnostic[{idx}].source.rule_id not a string: {diag}"
        );
        assert!(
            diag["source"]["source_module"].is_string(),
            "diagnostic[{idx}].source.source_module not a string: {diag}"
        );
        assert!(
            diag["location"]["file"].is_string(),
            "diagnostic[{idx}].location.file not a string: {diag}"
        );
    }

    // Every AI-guardrail diagnostic must route to a dedicated Category —
    // a diagnostic landing in `other` means `summary.by_category` lost
    // signal.
    let categories: Vec<String> = diagnostics
        .iter()
        .filter_map(|d| d["category"].as_str().map(str::to_string))
        .collect();
    assert!(
        !categories.iter().any(|c| c == "other"),
        "no AI-guardrail diagnostic should route to `other`; saw: {categories:?}"
    );
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
