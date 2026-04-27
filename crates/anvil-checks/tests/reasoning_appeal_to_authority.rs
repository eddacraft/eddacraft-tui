//! Integration tests for the AI-001 appeal-to-authority reasoning rule.
//!
//! These tests exercise the public API
//! (`anvil_checks::reasoning::*`) end-to-end against a representative
//! fixture, alongside focused suppression and comment-region cases that
//! complement the per-module unit tests.

use std::path::PathBuf;

use anvil_checks::reasoning::{
    APPEAL_TO_AUTHORITY_RULE_ID, ReasoningCheckConfig, run_reasoning_check,
    scan_appeal_to_authority,
};
use anvil_kernel_types::{Category, Severity};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("reasoning")
        .join(name)
}

#[test]
fn fixture_flags_three_appeals_and_honours_suppression() {
    let path = fixture_path("appeal_to_authority.ts");
    let content = std::fs::read_to_string(&path).expect("fixture readable");
    let findings = scan_appeal_to_authority(path.to_str().unwrap(), &content);

    let lines: Vec<u32> = findings
        .iter()
        .filter_map(|finding| finding.location.line)
        .collect();

    assert_eq!(
        lines,
        vec![10, 21, 27],
        "expected AI-001 to fire on the three unsuppressed appeal lines",
    );
    assert!(
        findings.iter().all(|f| f.id == APPEAL_TO_AUTHORITY_RULE_ID),
        "every fixture finding must carry the AI-001 id",
    );
    assert!(
        findings.iter().all(|f| f.category == Category::Reasoning),
        "every fixture finding must use Category::Reasoning",
    );
    assert!(
        findings.iter().all(|f| f.severity == Severity::Info),
        "AI-001 ships as info severity",
    );
}

#[test]
fn run_reasoning_check_aggregates_fixture_findings() {
    let path = fixture_path("appeal_to_authority.ts");
    let content = std::fs::read_to_string(&path).expect("fixture readable");
    let files = [(path.to_str().unwrap(), content.as_str())];
    let result = run_reasoning_check(&files, &ReasoningCheckConfig::default());

    assert!(!result.passed);
    assert_eq!(result.findings.len(), 3);
    assert!(result.message.contains('3'));
}
