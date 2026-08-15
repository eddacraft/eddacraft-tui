//! Shared support for detection-rule suites (CIB-336).
//!
//! A rule's positive fixtures must be derived from its *threat model* — the
//! enumerated defect/attack shapes the rule exists to catch — never from the
//! branches of the regex that happens to implement it. A suite written from
//! the pattern mirrors the implementation and structurally cannot fail when
//! the pattern narrows: PY-008 passed 33/33 through the #3880 regression
//! because every positive assertion terminated on a character in the
//! delimiter class under test.
//!
//! `assert_rule_fires_on` makes the threat model reviewable *data*: each
//! shape carries a label naming the threat it represents, so a reviewer can
//! diff the labelled list against the rule's stated intent, and a shape
//! without a fixture is impossible by construction. See
//! `docs/guides/anvil-rule-authoring.md` ("Testing the rule").

use anvil_checks::antipattern::{ScanOptions, scan_file};

/// Assert that `rule` fires on every labelled threat shape.
///
/// `threat_shapes` is `(label, fixture)`: the label names the defect/attack
/// shape (from the rule's threat model), the fixture is the source line(s)
/// scanned at `path`. On failure, every missed shape is reported — one run
/// shows the full extent of a narrowing, not just its first casualty.
pub fn assert_rule_fires_on(path: &str, rule: &str, threat_shapes: &[(&str, &str)]) {
    // A helper that exists to prevent vacuously-green suites must not pass
    // vacuously itself (verification advisory on CIB-336).
    assert!(
        !threat_shapes.is_empty(),
        "{rule}: empty threat-shape list — enumerate the shapes the rule exists to catch"
    );
    // Scope the scan to the rule under test (review suggestion on #3906):
    // the helper already knows which rule it is exercising, so there is no
    // reason to run the full catalogue per fixture. Explicit selection also
    // bypasses `enabled`/`opt_in` gating in `select_antipatterns`, which is
    // what a rule-under-test wants — opt-in rules get threat-model coverage
    // without `include_opt_in` plumbing.
    let options = ScanOptions {
        patterns: Some(vec![rule.to_string()]),
        include_opt_in: false,
    };
    let missed: Vec<String> = threat_shapes
        .iter()
        .filter(|(_, fixture)| {
            !scan_file(path, fixture, Some(&options))
                .warnings
                .iter()
                .any(|w| w.id == rule)
        })
        .map(|(label, fixture)| format!("  {label}: {fixture:?}"))
        .collect();
    assert!(
        missed.is_empty(),
        "{rule} must fire on every enumerated threat shape; missed {} of {}:\n{}",
        missed.len(),
        threat_shapes.len(),
        missed.join("\n")
    );
}
