//! Integration tests for the SURFENV `.env` file secret scan (SURFENV-001).
//!
//! Drives the public `anvil_checks::surface::env::*` API against the
//! checked-in fixture so future refactors can't quietly regress the
//! end-to-end "AWS key in `.env` is caught" promise.

use std::path::PathBuf;

use anvil_checks::secret::SecretCheckConfig;
use anvil_checks::surface::env::{is_env_file, scan_env_file};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("surfenv")
        .join(name)
}

fn config_no_entropy() -> SecretCheckConfig {
    // Entropy adds noise on short fixture values; pattern matches alone
    // are enough to prove the SURFENV-001 contract.
    SecretCheckConfig {
        enable_entropy: false,
        ..SecretCheckConfig::default()
    }
}

#[test]
fn aws_key_fixture_yields_structured_findings() {
    let path = fixture_path("aws-key.env");
    let content = std::fs::read_to_string(&path).expect("fixture readable");
    let findings = scan_env_file(
        path.to_str().expect("fixture path is utf-8"),
        &content,
        &config_no_entropy(),
    );

    // Three secrets in the fixture: two AWS keys + one GitHub token.
    // The second AWS key is suppressed via @anvil-ignore — it is still
    // returned (consumers tally suppressed counts) but flagged.
    assert_eq!(
        findings.len(),
        3,
        "expected 3 findings (2 AWS + 1 GitHub), got {findings:#?}"
    );

    let aws_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.finding.pattern_name == "AWS Key")
        .collect();
    assert_eq!(aws_findings.len(), 2, "two AWS Key findings");

    let suppressed: Vec<_> = findings.iter().filter(|f| f.suppressed).collect();
    assert_eq!(
        suppressed.len(),
        1,
        "exactly one finding should be suppressed via the ADR-029 directive"
    );
    assert_eq!(
        suppressed[0].suppression_reason.as_deref(),
        Some("rotated key kept for replay test")
    );

    let github_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.finding.pattern_name == "GitHub Token")
        .collect();
    assert_eq!(github_findings.len(), 1, "one GitHub Token finding");
    // Quoted-value parsing — the finding's key must point at the env
    // variable, not at the surrounding quotes.
    assert_eq!(github_findings[0].key, "GITHUB_TOKEN");
}

#[test]
fn redacted_line_does_not_leak_the_match() {
    let path = fixture_path("aws-key.env");
    let content = std::fs::read_to_string(&path).expect("fixture readable");
    let findings = scan_env_file(
        path.to_str().expect("fixture path is utf-8"),
        &content,
        &config_no_entropy(),
    );

    for finding in &findings {
        assert!(
            !finding.finding.redacted_line.contains("AKIAEXAMPLE"),
            "AWS key fragment leaked through redaction: {}",
            finding.finding.redacted_line
        );
    }
}

#[test]
fn discovery_routes_canonical_env_filenames_through_scanner() {
    // The fixture is named `aws-key.env` so it stays untracked by the repo's
    // `.env*` gitignore rule — discovery routing is keyed on real `.env`
    // filenames, which we assert directly so the contract is unambiguous.
    use std::path::PathBuf;
    assert!(is_env_file(&PathBuf::from(".env")));
    assert!(is_env_file(&PathBuf::from("packages/api/.env.production")));
    assert!(is_env_file(&PathBuf::from("services/.envrc")));
    assert!(!is_env_file(&PathBuf::from("config/env.ts")));
}
