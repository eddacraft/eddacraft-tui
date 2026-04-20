//! Scanner-parity integration test (RSCAN-007 / ADR-026).
//!
//! Loads shared fixtures from `tests/scanner-parity/fixtures.json` and
//! asserts that the Rust scanner's output matches each fixture's declared
//! `expected_matches`. The TS scanner runs the same fixtures through
//! `packages/anvil/core/src/antipattern/scanner-parity.test.ts`. If both
//! suites pass the same fixture data, the engines are in parity on the
//! covered rules.
//!
//! Known divergences (not yet covered by fixtures) are documented in
//! `tests/scanner-parity/README.md`.

use std::path::PathBuf;

use anvil_checks::antipattern::{Artifact, ArtifactKind, scan_artifact};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FixtureFile {
    fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    name: String,
    artifact_kind: String,
    reference: String,
    content: String,
    expected_matches: Vec<ExpectedMatch>,
}

#[derive(Debug, Deserialize, Clone)]
struct ExpectedMatch {
    id: String,
    line: usize,
}

fn workspace_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is the crate dir; its grandparent is the workspace root.
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn load_fixtures() -> FixtureFile {
    let path = workspace_root()
        .join("tests")
        .join("scanner-parity")
        .join("fixtures.json");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_slice(&bytes).expect("fixtures.json is valid JSON")
}

fn parse_kind(s: &str) -> ArtifactKind {
    ArtifactKind::from_wire(s).unwrap_or_else(|| panic!("unknown artifact_kind: {s}"))
}

#[test]
fn rust_scanner_matches_every_parity_fixture() {
    let fixtures = load_fixtures();
    let mut failures = Vec::new();

    for fixture in &fixtures.fixtures {
        let artifact = Artifact {
            kind: parse_kind(&fixture.artifact_kind),
            reference: fixture.reference.clone(),
            content: fixture.content.clone(),
        };
        let result = scan_artifact(&artifact, None);

        let actual: Vec<ExpectedMatch> = result
            .warnings
            .iter()
            .map(|w| ExpectedMatch {
                id: w.id.clone(),
                line: w.location.line,
            })
            .collect();

        if !matches_equivalent(&actual, &fixture.expected_matches) {
            failures.push(format!(
                "[{}] expected {:?}, got {:?}",
                fixture.name,
                summarise(&fixture.expected_matches),
                summarise(&actual),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "scanner-parity fixtures diverged:\n  {}",
        failures.join("\n  ")
    );
}

fn matches_equivalent(actual: &[ExpectedMatch], expected: &[ExpectedMatch]) -> bool {
    let mut a = summarise(actual);
    let mut e = summarise(expected);
    a.sort();
    e.sort();
    a == e
}

fn summarise(matches: &[ExpectedMatch]) -> Vec<String> {
    matches
        .iter()
        .map(|m| format!("{}:{}", m.id, m.line))
        .collect()
}
