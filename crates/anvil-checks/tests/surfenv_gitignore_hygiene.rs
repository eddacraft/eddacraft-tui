//! Integration tests for SURFENV-002 (`.gitignore` hygiene).
//!
//! Drives the public `check_gitignore_hygiene` API against the
//! checked-in fixture. Fixtures live under
//! `tests/fixtures/surfenv/gitignore-unprotected/` with `.fixture`
//! suffixes so the repo's own `.env*` rule doesn't ignore them
//! (`.gitignore` rules apply to filenames, not to extensions).

use std::path::{Path, PathBuf};

use anvil_checks::surface::env::{
    GitignoreFindingKind, SURFENV_002_RULE_ID, check_gitignore_hygiene,
};

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/surfenv/gitignore-unprotected")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} unreadable: {err}", path.display()))
}

#[test]
fn fixture_repo_yields_one_unignored_finding_and_one_suppressed() {
    let gitignore = fixture(".gitignore.fixture");
    let env_local = fixture(".env.local.fixture");
    let env_prod = fixture(".env.production.fixture");

    let env_files = vec![
        (PathBuf::from(".env.local"), env_local),
        (PathBuf::from(".env.production"), env_prod),
    ];
    let findings = check_gitignore_hygiene(&env_files, Some(&gitignore));

    assert_eq!(findings.len(), 2, "two unignored env files in fixture");
    let local = findings
        .iter()
        .find(|f| f.file.as_path() == Path::new(".env.local"))
        .expect(".env.local finding");
    assert_eq!(local.kind, GitignoreFindingKind::UnignoredEnvFile);
    assert!(!local.suppressed);
    assert_eq!(local.suggested_pattern, ".env.local");

    let prod = findings
        .iter()
        .find(|f| f.file.as_path() == Path::new(".env.production"))
        .expect(".env.production finding");
    assert!(
        prod.suppressed,
        "the .env.production fixture has a SURFENV-002 directive"
    );
    assert_eq!(
        prod.suppression_reason.as_deref(),
        Some("frozen replay fixture for the gitignore hygiene test")
    );
    // CLAWP-053: the suppressed finding's kind and suggested_pattern were
    // unchecked — only the `.env.local` finding had them asserted. A
    // suppressed finding that drifted to the wrong kind/pattern (e.g. a
    // bad suggested `.gitignore` line a user would paste) would slip
    // through. Mirror the `.env.local` assertions on the suppressed one.
    assert_eq!(prod.kind, GitignoreFindingKind::UnignoredEnvFile);
    assert_eq!(prod.suggested_pattern, ".env.production");
}

#[test]
fn rule_id_constant_matches_directive_form() {
    // Trip-wire: if anyone changes the constant, the fixture needs the
    // matching directive. Catch it here rather than in the fixture test.
    assert_eq!(SURFENV_002_RULE_ID, "SURFENV-002");
}
