//! SURFENV-006 baseline run against the anvil repository itself.
//!
//! Walks the workspace's own `.env*` files and runs every SURFENV rule
//! against them. The repo is currently expected to be **clean** — no
//! unsuppressed findings on any rule. A regression flips this test red
//! and gives the operator the file/rule that drifted.
//!
//! Why an integration test instead of a `bin/` walker:
//!
//! - The build is hermetic — anyone running `cargo test` validates the
//!   baseline without an extra step.
//! - When SURFENV gains a CLI surface (Phase 4 work), this test stays
//!   useful as the smoke check for the rule pack itself.
//!
//! When anvil legitimately introduces a new `.env*` file, update the
//! [`ANVIL_ENV_FILES`] list and (if a finding is intentional) annotate
//! the file with the appropriate `# @anvil-ignore SURFENV-NNN -- ...`
//! directive — the same workflow operators downstream will use.

use std::path::{Path, PathBuf};

use anvil_checks::secret::SecretCheckConfig;
use anvil_checks::surface::env::{
    check_env_drift, check_gitignore_hygiene, is_env_file, scan_env_file, scan_prod_values,
};

/// Repo-relative paths of env files anvil cares about for baseline
/// scanning. Hard-coded so a new committed env file forces an explicit
/// decision rather than silently widening the baseline (we deliberately
/// don't walk the tree — the surface is small and a globber would also
/// pick up uncommitted local-only `.env`s on a contributor's machine).
///
/// The list is split into committed and gitignored files. The
/// gitignored ones still need to pass the SURFENV-002 `.gitignore`
/// hygiene rule (i.e. they must actually be covered by the gitignore)
/// — adding them here is the trip-wire that catches a regression where
/// the gitignore drifts and stops protecting them. Operations review
/// flagged the original list as having a hole at
/// `.github/actions-runner/.env`; closed by including it below.
const ANVIL_ENV_FILES: &[&str] = &[
    // Committed env templates — intentionally tracked.
    "apps/anvil-api/.env.example",
    "apps/website/.env.local.example",
    // Local-only runner config — gitignored. Listed here so the
    // baseline asserts the gitignore actually covers it. If the file
    // is absent on this machine (most contributors), `read_repo_file`
    // returns None and the per-file tests skip cleanly.
    ".github/actions-runner/.env",
];

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/anvil-checks; walk up two
    // levels to land at the workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("workspace root resolves")
}

fn read_repo_file(relative: &str) -> Option<String> {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path).ok()
}

fn config_no_entropy() -> SecretCheckConfig {
    SecretCheckConfig {
        enable_entropy: false,
        ..SecretCheckConfig::default()
    }
}

#[test]
fn anvil_env_files_are_routed_through_the_scanner() {
    // Trip-wire on `is_env_file` semantics — if someone narrows the
    // discovery rule, the SURFENV pipeline silently stops processing
    // anvil's own files. Catch that here.
    for relative in ANVIL_ENV_FILES {
        let path = PathBuf::from(relative);
        assert!(
            is_env_file(&path),
            "anvil env file {relative} no longer recognised by is_env_file"
        );
    }
}

#[test]
fn surfenv_001_secret_scan_is_clean_on_anvil() {
    for relative in ANVIL_ENV_FILES {
        let Some(content) = read_repo_file(relative) else {
            // File not present in this checkout (e.g. shallow clone) —
            // skip rather than fail. The discovery test above is the
            // strict gate.
            continue;
        };
        let findings = scan_env_file(relative, &content, &config_no_entropy());
        let unsuppressed: Vec<_> = findings.iter().filter(|f| !f.suppressed).collect();
        assert!(
            unsuppressed.is_empty(),
            "{relative} grew an unsuppressed SURFENV-001 finding: {unsuppressed:#?}"
        );
    }
}

#[test]
fn surfenv_002_gitignore_hygiene_is_clean_on_anvil() {
    let Some(gitignore) = read_repo_file(".gitignore") else {
        return;
    };

    let env_files: Vec<(PathBuf, String)> = ANVIL_ENV_FILES
        .iter()
        .filter_map(|relative| {
            let content = read_repo_file(relative)?;
            Some((PathBuf::from(relative), content))
        })
        .collect();

    let findings = check_gitignore_hygiene(&env_files, Some(&gitignore));
    let unsuppressed: Vec<_> = findings.iter().filter(|f| !f.suppressed).collect();
    assert!(
        unsuppressed.is_empty(),
        "anvil grew an unsuppressed SURFENV-002 finding: {unsuppressed:#?}"
    );
}

#[test]
fn surfenv_003_prod_value_scan_is_clean_on_anvil() {
    // SURFENV-003 only fires on non-prod env files; anvil currently
    // commits only `.env.example` templates so this should always be
    // empty. The test still runs the scanner to assert "no findings"
    // remains the contract.
    for relative in ANVIL_ENV_FILES {
        let Some(content) = read_repo_file(relative) else {
            continue;
        };
        let findings = scan_prod_values(relative, &content);
        assert!(
            findings.is_empty(),
            "{relative} grew a SURFENV-003 finding: {findings:#?}"
        );
    }
}

#[test]
fn surfenv_004_drift_check_template_only_pairwise_run_is_missing_from_concrete_only() {
    // Anvil only commits templates; the higher-level repo walk
    // (`run_surfenv_check::pair_template_with_concrete`) avoids
    // pairing those against a missing concrete sibling — so the
    // aggregator surfaces zero drift findings here. This test
    // exercises the lower-level pairwise helper directly and records
    // its narrower contract: when given a committed template and an
    // empty concrete file, every unsuppressed finding must be
    // `MissingFromConcrete` (every example-side key is "missing"
    // from the empty concrete). Copilot review caught the prior
    // version asserting nothing useful — fixed by filtering to
    // template files and tripping if zero templates are checked.
    let mut checked_templates = 0;
    for relative in ANVIL_ENV_FILES
        .iter()
        .copied()
        .filter(|relative| relative.ends_with(".example"))
    {
        let Some(content) = read_repo_file(relative) else {
            continue;
        };
        checked_templates += 1;
        let findings = check_env_drift(relative, &content, "<no-concrete>", "");
        let unsuppressed: Vec<_> = findings.iter().filter(|f| !f.suppressed).collect();
        for finding in &unsuppressed {
            assert_eq!(
                finding.kind,
                anvil_checks::surface::env::DriftKind::MissingFromConcrete,
                "unexpected drift kind on a template-only pairwise run"
            );
        }
    }
    assert!(
        checked_templates > 0,
        "expected at least one committed template file in ANVIL_ENV_FILES"
    );
}

#[test]
fn external_validation_smoke_against_synthetic_repo() {
    // Stand-in for "external candidate validation" until a real
    // external repo lands as a SURFENV-006 follow-up. The synthetic
    // case exercises every rule end-to-end on adversarial input that
    // exercises the rule interactions — the kind of file an external
    // operator is most likely to point at.
    let gitignore = "node_modules/\ndist/\n# .env intentionally NOT ignored — bug we're catching\n";
    let env_local = "DATABASE_URL=postgres://prod-db.acme.io/app\nAPI_KEY=local-dev-key\n";
    let env_example = "DATABASE_URL=\nAPI_KEY=\nLEGACY_FLAG=\n";

    // SURFENV-002: the .env.local must be flagged.
    let env_files = vec![
        (PathBuf::from(".env.local"), env_local.to_string()),
        (PathBuf::from(".env.example"), env_example.to_string()),
    ];
    let gitignore_findings = check_gitignore_hygiene(&env_files, Some(gitignore));
    assert!(
        gitignore_findings
            .iter()
            .any(|f| f.file == Path::new(".env.local") && !f.suppressed),
        "synthetic .env.local must trip SURFENV-002"
    );
    // .env.example is intentionally committed — must not trip.
    assert!(
        !gitignore_findings
            .iter()
            .any(|f| f.file == Path::new(".env.example")),
        "synthetic .env.example must NOT trip SURFENV-002"
    );

    // SURFENV-003: the prod-shaped DATABASE_URL must trip.
    let prod_findings = scan_prod_values(".env.local", env_local);
    assert_eq!(prod_findings.len(), 1);

    // SURFENV-004: LEGACY_FLAG is in the example but missing from the
    // concrete; nothing should be missing from the example.
    let drift_findings = check_env_drift(".env.example", env_example, ".env.local", env_local);
    assert!(
        drift_findings.iter().any(|f| f.key == "LEGACY_FLAG"),
        "expected LEGACY_FLAG drift finding"
    );
}
