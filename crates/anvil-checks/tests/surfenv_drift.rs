//! Integration tests for SURFENV-004 (`.env.example` drift).

use std::path::PathBuf;

use anvil_checks::surface::env::{DriftKind, SURFENV_004_RULE_ID, check_env_drift};

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/surfenv")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} unreadable: {err}", path.display()))
}

#[test]
fn drift_fixture_reports_both_directions() {
    let example = fixture("drift-example.env.fixture");
    let concrete = fixture("drift-concrete.env.fixture");

    let findings = check_env_drift(".env.example", &example, ".env.local", &concrete);
    assert_eq!(findings.len(), 2, "got {findings:#?}");

    // Stable ordering puts MissingFromExample first.
    assert_eq!(findings[0].kind, DriftKind::MissingFromExample);
    assert_eq!(findings[0].key, "NEW_FLAG");
    assert_eq!(findings[0].concrete_file, ".env.local");

    assert_eq!(findings[1].kind, DriftKind::MissingFromConcrete);
    assert_eq!(findings[1].key, "LEGACY_FEATURE_FLAG");
    assert_eq!(findings[1].example_file, ".env.example");
}

#[test]
fn rule_id_constant_matches_directive_form() {
    assert_eq!(SURFENV_004_RULE_ID, "SURFENV-004");
}
