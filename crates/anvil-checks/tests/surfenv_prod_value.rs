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
    // CLAWP-039: pin the suppressed finding's indicator too, so the full
    // 4-finding set is asserted by key + indicator. `classify_entry`
    // checks `value_has_prod_host_segment` before `value_mentions_production`,
    // so `api.production.acme.io` (a `.production.` host segment) resolves
    // to ProdHostSegment, not ValueMentionsProduction.
    assert_eq!(suppressed[0].indicator, ProdIndicator::ProdHostSegment);

    let staging_finding = findings.iter().find(|f| f.key == "SECONDARY_HOST");
    assert!(staging_finding.is_none(), "staging host must short-circuit");

    // CLAWP-039: pin the full expected finding set by key + indicator,
    // not just the count and two of the four. Asserting each fixture
    // line's (key, indicator) triple catches a regression that drops one
    // finding while spuriously adding another — which would keep the
    // count at 4 and pass the looser checks. (LEGACY_HOST is covered by
    // the suppressed-finding assertion above.)
    let by_key = |k: &str| {
        findings
            .iter()
            .find(|f| f.key == k)
            .unwrap_or_else(|| panic!("no SURFENV-003 finding for `{k}`: {findings:#?}"))
    };

    let database_url = by_key("DATABASE_URL");
    assert_eq!(
        database_url.indicator,
        ProdIndicator::ProdHostSegment,
        "DATABASE_URL=postgres://prod-db... is a prod host segment"
    );
    assert!(
        !database_url.suppressed,
        "DATABASE_URL is an active finding"
    );

    let feature_flags = by_key("FEATURE_FLAGS_ENV");
    assert_eq!(
        feature_flags.indicator,
        ProdIndicator::ValueMentionsProduction,
        "FEATURE_FLAGS_ENV=production mentions production"
    );
    assert!(
        !feature_flags.suppressed,
        "FEATURE_FLAGS_ENV is an active finding"
    );

    let key_suffix = by_key("SECRET_PROD");
    assert_eq!(key_suffix.indicator, ProdIndicator::KeySuffixProd);
    assert!(!key_suffix.suppressed, "SECRET_PROD is an active finding");
    assert!(
        !key_suffix
            .redacted_excerpt
            .contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        "redaction must hide the raw value"
    );
}

#[test]
fn rule_id_constant_matches_directive_form() {
    assert_eq!(SURFENV_003_RULE_ID, "SURFENV-003");
}
