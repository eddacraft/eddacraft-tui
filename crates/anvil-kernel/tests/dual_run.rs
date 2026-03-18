use std::fs;
use std::path::PathBuf;

use anvil_kernel::embedded::{EmbeddedConfig, EmbeddedResult, run_embedded};
use anvil_kernel::policy::engine::Violation;
use tempfile::TempDir;

/// Comparison result from running both engines on the same input.
struct DualRunResult {
    rust_violations: Vec<Violation>,
    matches: bool,
    discrepancies: Vec<String>,
}

/// Run the Rust kernel in embedded mode and capture results.
/// The TS engine side is a placeholder for now -- returns empty results.
fn run_dual(root: &std::path::Path, arch_config: Option<PathBuf>) -> DualRunResult {
    let config = EmbeddedConfig {
        root: root.to_path_buf(),
        architecture_config: arch_config,
        filter: None,
    };

    let rust_result = run_embedded(&config).expect("rust kernel should succeed");

    // Placeholder: TS engine results would come from a subprocess or FFI
    let ts_violations: Vec<ViolationRecord> = Vec::new();
    let rust_records = serialise_violations(&rust_result);

    let (matches, discrepancies) = compare(&rust_records, &ts_violations);

    DualRunResult {
        rust_violations: rust_result.violations,
        matches,
        discrepancies,
    }
}

/// Serialisable violation record for cross-engine comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ViolationRecord {
    policy_id: String,
    file: String,
    symbol: String,
}

fn serialise_violations(result: &EmbeddedResult) -> Vec<ViolationRecord> {
    result
        .violations
        .iter()
        .map(|v| ViolationRecord {
            policy_id: v.policy_id.clone(),
            file: v.file.clone(),
            symbol: v.symbol.clone(),
        })
        .collect()
}

fn compare(rust: &[ViolationRecord], ts: &[ViolationRecord]) -> (bool, Vec<String>) {
    let mut discrepancies = Vec::new();

    for r in rust {
        if !ts.contains(r) {
            discrepancies.push(format!(
                "rust-only: {} in {} ({})",
                r.policy_id, r.file, r.symbol
            ));
        }
    }

    for t in ts {
        if !rust.contains(t) {
            discrepancies.push(format!(
                "ts-only: {} in {} ({})",
                t.policy_id, t.file, t.symbol
            ));
        }
    }

    (discrepancies.is_empty(), discrepancies)
}

fn write_file(dir: &std::path::Path, name: &str, content: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn harness_runs_embedded_and_captures_results() {
    let tmp = TempDir::new().unwrap();
    write_file(
        tmp.path(),
        "src/main.ts",
        "export function main() { return 'hello'; }",
    );

    let result = run_dual(tmp.path(), None);

    // With no architecture config, public-api-expansion should fire
    assert!(
        !result.rust_violations.is_empty(),
        "should detect at least the public API expansion"
    );
}

#[test]
fn results_can_be_serialised_for_comparison() {
    let tmp = TempDir::new().unwrap();
    write_file(
        tmp.path(),
        "src/util.ts",
        "function internal() { return 42; }",
    );

    let config = EmbeddedConfig {
        root: tmp.path().to_path_buf(),
        architecture_config: None,
        filter: None,
    };

    let result = run_embedded(&config).unwrap();
    let records = serialise_violations(&result);

    // Serialisation should work and produce deterministic records
    for record in &records {
        assert!(!record.policy_id.is_empty());
        assert!(!record.file.is_empty());
    }
}

#[test]
fn comparison_detects_discrepancies() {
    let rust = vec![ViolationRecord {
        policy_id: "cross-layer-violation".to_string(),
        file: "src/a.ts".to_string(),
        symbol: "foo".to_string(),
    }];
    let ts = vec![ViolationRecord {
        policy_id: "cross-layer-violation".to_string(),
        file: "src/b.ts".to_string(),
        symbol: "bar".to_string(),
    }];

    let (matches, discrepancies) = compare(&rust, &ts);
    assert!(!matches);
    assert_eq!(discrepancies.len(), 2);
}

#[test]
fn comparison_matches_on_identical_results() {
    let violations = vec![ViolationRecord {
        policy_id: "public-api-expansion".to_string(),
        file: "src/api.ts".to_string(),
        symbol: "handler".to_string(),
    }];

    let (matches, discrepancies) = compare(&violations, &violations);
    assert!(matches);
    assert!(discrepancies.is_empty());
}

#[test]
fn empty_project_matches_trivially() {
    let tmp = TempDir::new().unwrap();
    let result = run_dual(tmp.path(), None);

    // Both engines produce nothing => match
    assert!(result.matches);
    assert!(result.discrepancies.is_empty());
    assert!(result.rust_violations.is_empty());
}

#[test]
fn ts_placeholder_always_empty() {
    // Until TS engine integration is added, TS side is always empty.
    // The harness should still run cleanly and report rust-only findings.
    let tmp = TempDir::new().unwrap();
    write_file(tmp.path(), "src/main.ts", "export function hello() {}");

    let result = run_dual(tmp.path(), None);

    // Rust finds violations but TS placeholder returns nothing
    if !result.rust_violations.is_empty() {
        assert!(!result.matches);
        assert!(!result.discrepancies.is_empty());
    }
}
