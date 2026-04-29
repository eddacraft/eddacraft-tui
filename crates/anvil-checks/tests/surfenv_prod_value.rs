//! Integration tests for SURFENV-003 (production-shaped values in
//! non-production `.env` files). Fixture lives at
//! `tests/fixtures/surfenv/prod-in-local.env.fixture` — the `.fixture`
//! suffix keeps the repo's `.env*` ignore rule from masking it.

use std::path::PathBuf;

use anvil_checks::surface::env::{ProdIndicator, SURFENV_003_RULE_ID, scan_prod_values};

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/surfenv")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} unreadable: {err}", path.display()))
}

#[test]
fn prod_in_local_fixture_yields_expected_findings() {
    let content = fixture("prod-in-local.env.fixture");
    let findings = scan_prod_values(".env.local", &content);

    // Expect 4 findings: prod-db host, FEATURE_FLAGS_ENV=production,
    // SECRET_PROD key suffix, plus the LEGACY_HOST line which is
    // suppressed via the directive immediately above it. The staging
    // host is short-circuited and contributes no finding.
    assert_eq!(
        findings.len(),
        4,
        "expected 4 SURFENV-003 findings, got {findings:#?}"
    );

    let suppressed: Vec<_> = findings.iter().filter(|f| f.suppressed).collect();
    assert_eq!(suppressed.len(), 1, "exactly one suppressed finding");
    assert_eq!(suppressed[0].key, "LEGACY_HOST");

    let staging_finding = findings.iter().find(|f| f.key == "SECONDARY_HOST");
    assert!(
        staging_finding.is_none(),
        "staging host must short-circuit"
    );

    let key_suffix = findings
        .iter()
        .find(|f| f.indicator == ProdIndicator::KeySuffixProd)
        .expect("SECRET_PROD key-suffix finding");
    assert_eq!(key_suffix.key, "SECRET_PROD");
    assert!(
        !key_suffix.redacted_excerpt.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        "redaction must hide the raw value"
    );
}

#[test]
fn rule_id_constant_matches_directive_form() {
    assert_eq!(SURFENV_003_RULE_ID, "SURFENV-003");
}
