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
//! When anvil legitimately introduces a new `.env*` file, add it to
//! [`ANVIL_COMMITTED_ENV_FILES`] (if tracked in git) or
//! [`ANVIL_OPTIONAL_ENV_FILES`] (if local-only and gitignored) — never
//! both — and, if a finding is intentional, annotate the file with the
//! appropriate `# @anvil-ignore SURFENV-NNN -- ...` directive (the same
//! workflow operators downstream will use).

use std::path::{Path, PathBuf};

use anvil_checks::secret::SecretCheckConfig;
use anvil_checks::surface::env::{
    check_env_drift, check_gitignore_hygiene, is_env_file, scan_env_file, scan_prod_values,
};

/// Committed env templates — intentionally tracked. These files MUST
/// exist in any non-shallow checkout; the baseline trip-wire fails if
/// one disappears, so the SURFENV-001 / -003 / -004 scans cannot be
/// silently skipped by a regression that deletes a template.
const ANVIL_COMMITTED_ENV_FILES: &[&str] = &[
    "apps/anvil-api/.env.example",
    "apps/website/.env.local.example",
];

/// Local-only env paths that are gitignored. The SURFENV-002 gitignore
/// hygiene test still includes these with empty content when absent,
/// so the gitignore pattern coverage is asserted regardless of whether
/// the file happens to exist in this checkout. CLAWP-012 (council
/// pass-2 finding) closed the original gap where an absent optional
/// path silently dropped out of the hygiene input via `filter_map`.
/// Operations review previously flagged a hole at
/// `.github/actions-runner/.env`; closed by including it below.
const ANVIL_OPTIONAL_ENV_FILES: &[&str] = &[".github/actions-runner/.env"];

/// Single source of truth for "every env file anvil cares about" —
/// committed templates plus gitignored optional paths, in that order.
/// All baseline tests iterate this chain rather than a separate union
/// list, so adding a new path to one of the subsets above is enough; no
/// follow-up edit can drift them apart.
fn anvil_env_files() -> impl Iterator<Item = &'static str> {
    ANVIL_COMMITTED_ENV_FILES
        .iter()
        .chain(ANVIL_OPTIONAL_ENV_FILES.iter())
        .copied()
}

fn is_optional_anvil_env(relative: &str) -> bool {
    ANVIL_OPTIONAL_ENV_FILES.contains(&relative)
}

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
    for relative in anvil_env_files() {
        let path = PathBuf::from(relative);
        assert!(
            is_env_file(&path),
            "anvil env file {relative} no longer recognised by is_env_file"
        );
    }
}

#[test]
fn surfenv_001_secret_scan_is_clean_on_anvil() {
    for relative in anvil_env_files() {
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

    // CLAWP-012: include optional gitignored paths with empty content
    // when absent, so the gitignore pattern is exercised even on
    // checkouts where `.github/actions-runner/.env` does not exist
    // (the common case). The hygiene check itself is path-only —
    // content is read only for in-file suppression directives, which
    // an absent file cannot carry anyway. Committed templates that go
    // missing are caught by the dedicated trip-wire in
    // `anvil_committed_env_templates_are_present`.
    let env_files: Vec<(PathBuf, String)> = anvil_env_files()
        .filter_map(|relative| match read_repo_file(relative) {
            Some(content) => Some((PathBuf::from(relative), content)),
            None if is_optional_anvil_env(relative) => {
                Some((PathBuf::from(relative), String::new()))
            }
            None => None,
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
fn anvil_committed_env_templates_are_present() {
    // Trip-wire (CLAWP-012 companion): every committed template named
    // in [`ANVIL_COMMITTED_ENV_FILES`] must be readable from this
    // checkout. Without this, a regression that deletes a template
    // would let the SURFENV-001 / -003 / -004 baseline scans silently
    // skip it via `read_repo_file` returning `None`. Optional /
    // gitignored paths are excluded from this trip-wire by design —
    // they are expected to be absent on most contributor machines.
    for relative in ANVIL_COMMITTED_ENV_FILES {
        assert!(
            read_repo_file(relative).is_some(),
            "committed env template {relative} is missing from this checkout — baseline scans would silently skip it"
        );
    }
}

#[test]
fn surfenv_003_prod_value_scan_is_clean_on_anvil() {
    // SURFENV-003 only fires on non-prod env files; anvil currently
    // commits only `.env.example` templates so this should always be
    // empty. The test still runs the scanner to assert "no findings"
    // remains the contract.
    for relative in anvil_env_files() {
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
    for relative in anvil_env_files().filter(|relative| relative.ends_with(".example")) {
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
        "expected at least one committed `.example` template in anvil_env_files()"
    );
}

#[test]
fn external_validation_smoke_against_synthetic_repo() {
    // Stand-in for "external candidate validation" until a real
    // external repo lands as a SURFENV-006 follow-up. The synthetic
    // case exercises every rule end-to-end on adversarial input that
    // exercises the rule interactions — the kind of file an external
    // operator is most likely to point at.
    //
    // CLAWP-013: SURFENV-001 (secret-in-value) must be exercised by a
    // *positive* case in the same smoke. The `AKIA…` AWS Access Key
    // ID pattern is deterministic — entropy is disabled below, and
    // `AKIAQRSTUVWXYZ012345` matches the literal regex
    // `AKIA[0-9A-Z]{16}` in `crates/anvil-checks/src/secret/patterns.rs`.
    // The well-known AWS docs example `AKIAIOSFODNN7EXAMPLE` is NOT
    // usable here because the default allowlist case-insensitively
    // strips substrings containing `example` / `test` / `dummy` /
    // `sample` / `placeholder` / `lorem ipsum`. The committed-template
    // baseline (`surfenv_001_secret_scan_is_clean_on_anvil`) only
    // asserts the *negative* case; without a positive trip here the
    // SURFENV-001 detection path could regress silently.
    let gitignore = "node_modules/\ndist/\n# .env intentionally NOT ignored — bug we're catching\n";
    let env_local = "DATABASE_URL=postgres://prod-db.acme.io/app\n\
                     API_KEY=local-dev-key\n\
                     AWS_ACCESS_KEY_ID=AKIAQRSTUVWXYZ012345\n";
    let env_example = "DATABASE_URL=\nAPI_KEY=\nAWS_ACCESS_KEY_ID=\nLEGACY_FLAG=\n";

    // SURFENV-001: the AWS Access Key ID must be flagged. Run with
    // entropy disabled so the assertion turns purely on the
    // deterministic prefix pattern, not on Shannon entropy heuristics.
    let secret_findings = scan_env_file(".env.local", env_local, &config_no_entropy());
    let unsuppressed_secrets: Vec<_> = secret_findings.iter().filter(|f| !f.suppressed).collect();
    assert!(
        !unsuppressed_secrets.is_empty(),
        "synthetic .env.local with an AWS Access Key ID must trip at least one \
         unsuppressed SURFENV-001 finding (entropy disabled); got: {secret_findings:#?}"
    );
    assert!(
        unsuppressed_secrets
            .iter()
            .any(|f| f.key == "AWS_ACCESS_KEY_ID"),
        "expected the unsuppressed SURFENV-001 finding to name AWS_ACCESS_KEY_ID; \
         got keys: {:?}",
        unsuppressed_secrets
            .iter()
            .map(|f| &f.key)
            .collect::<Vec<_>>()
    );

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
