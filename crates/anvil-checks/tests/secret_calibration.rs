//! SDT-002: the secret-detection calibration corpus runner.
//!
//! Measures the built-in catalogue against a committed corpus and prints a
//! decomposed report — detection rate, false-positive rate, per-rule misses,
//! and the rules with no coverage at all — so a rules change can be shown to
//! help rather than harm. The beta "~50% detection" anecdote becomes a
//! reproducible number with named causes.
//!
//! Run it locally and see exactly what CI prints:
//!
//! ```bash
//! pnpm secret:calibrate
//! # or, without pnpm:
//! cargo test -p eddacraft-anvil-checks --test secret_calibration -- --nocapture
//! ```
//!
//! The corpus lives in `tests/corpus/secret/`; every canary there is
//! synthetic and provider-invalid by construction. **Read
//! `tests/corpus/secret/PROVENANCE.md` before editing any case file.**
//!
//! The committed expectations live in `manifest.json`. Any drift — a
//! regression *or* an improvement — fails this test with both numbers
//! printed, so no rules change lands unmeasured.
//!
//! **Scope boundary (SDT-006).** This runner measures the pattern/entropy
//! *engine*: it calls `scan_content_with_stats` directly, so file selection —
//! `skip_extensions`, the `MAX_FILE_SIZE` guard, unreadable files and the
//! SCAN-001 panic arm — is outside its reach by construction, and its cases
//! are `.corpus` files no path-based walker selects. Those paths are covered
//! by `tests/secret_file_coverage.rs`, which drives `run_secret_check` over
//! real files on disk. They are deliberately not folded in here: selection is
//! a pass/fail contract, not a detection *rate* a manifest baseline can track
//! drift on.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anvil_checks::secret::{
    AllowlistProvenance, SECRET_PATTERNS, SecretCheckConfig, SecretFinding, Suppression,
    VENDORED_COMPILED_PATTERNS, VENDORED_RULESET_VERSION, scan_content_with_stats,
};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Manifest {
    catalogue_size: usize,
    /// SDT-004: the vendored ruleset version this baseline was measured
    /// against. Recorded in the manifest as well as printed in the report, so a
    /// refresh that moves the pin without re-measuring fails here rather than
    /// silently invalidating the committed numbers.
    #[serde(default)]
    vendored_ruleset: Option<String>,
    expected: Expected,
    cases: Vec<Case>,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
struct Expected {
    in_catalogue_detected: usize,
    in_catalogue_total: usize,
    gap_probe_detected: usize,
    gap_probe_total: usize,
    planted_detected: usize,
    planted_total: usize,
    benign_flagged: usize,
    benign_total: usize,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    category: String,
    file: String,
    scan_path: String,
    expect: String,
    #[serde(default)]
    rule: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    known_residual: bool,
    #[serde(default)]
    config: Option<CaseConfig>,
    #[serde(default)]
    control: Option<Control>,
}

#[derive(Deserialize)]
struct CaseConfig {
    #[serde(default)]
    enable_entropy: Option<bool>,
    #[serde(default)]
    entropy_threshold: Option<f64>,
}

/// A non-vacuity control: the same bytes, moved out of the context that is
/// supposed to suppress them (or with the suppressing token substituted out).
/// A benign case whose control stays silent proves nothing — the scanner was
/// never going to flag it — so the report calls that out rather than banking
/// it as a false positive avoided.
#[derive(Deserialize)]
struct Control {
    expected_rule: String,
    #[serde(default)]
    scan_path: Option<String>,
    #[serde(default)]
    substitutions: Option<Vec<Vec<String>>>,
}

// ---------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------

struct Outcome {
    /// Rule names that fired, deduplicated and sorted for a stable report.
    fired: BTreeSet<String>,
    /// `rule => redacted match` for whatever did fire. Redacted, so a report
    /// posted in CI logs never carries the value itself.
    fired_detail: BTreeMap<String, String>,
    /// SDT-004: `rule => ruleset version` for the findings that came from a
    /// vendored rule. Read off `SecretFinding::ruleset_version`, so this is a
    /// live check that the provenance actually reaches the finding rather than
    /// a name-matching guess.
    vendored_fired: BTreeMap<String, String>,
    suppressed: Vec<Suppression>,
    lines_skipped_oversize: usize,
    /// `Some(true)` when a declared control fired (the case is non-vacuous),
    /// `Some(false)` when it stayed silent, `None` when no control is declared.
    control_fired: Option<bool>,
}

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/secret")
}

fn case_config(case: &Case) -> SecretCheckConfig {
    let mut config = SecretCheckConfig::default();
    if let Some(overrides) = &case.config {
        if let Some(enable_entropy) = overrides.enable_entropy {
            config.enable_entropy = enable_entropy;
        }
        if let Some(threshold) = overrides.entropy_threshold {
            config.entropy_threshold = threshold;
        }
    }
    config
}

fn read_case(root: &Path, case: &Case) -> String {
    let path = root.join(&case.file);
    std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "corpus case {} unreadable at {}: {err}",
            case.id,
            path.display()
        )
    })
}

fn control_added_expected_finding(
    baseline: &[SecretFinding],
    control: &[SecretFinding],
    expected_rule: &str,
) -> bool {
    let baseline_count = baseline
        .iter()
        .filter(|finding| finding.pattern_name == expected_rule)
        .count();
    control
        .iter()
        .filter(|finding| finding.pattern_name == expected_rule)
        .count()
        > baseline_count
}

fn measure(root: &Path, case: &Case) -> Outcome {
    let content = read_case(root, case);
    let config = case_config(case);
    let (findings, stats) = scan_content_with_stats(&content, &case.scan_path, &config);

    let control_fired = case.control.as_ref().map(|control| {
        let mut control_content = content.clone();
        for pair in control.substitutions.iter().flatten() {
            assert_eq!(
                pair.len(),
                2,
                "case {}: control substitutions must be [from, to] pairs",
                case.id
            );
            assert!(
                control_content.contains(&pair[0]),
                "case {}: control substitution {:?} does not occur in the case file — \
                 the control is silently a no-op",
                case.id,
                pair[0]
            );
            control_content = control_content.replace(&pair[0], &pair[1]);
        }
        let control_path = control.scan_path.as_deref().unwrap_or(&case.scan_path);
        let (control_findings, _) =
            scan_content_with_stats(&control_content, control_path, &config);
        control_added_expected_finding(&findings, &control_findings, &control.expected_rule)
    });

    Outcome {
        fired_detail: findings
            .iter()
            .map(|f| (f.pattern_name.clone(), f.redacted_match.clone()))
            .collect(),
        vendored_fired: findings
            .iter()
            .filter_map(|f| {
                f.ruleset_version
                    .as_ref()
                    .map(|version| (f.pattern_name.clone(), version.clone()))
            })
            .collect(),
        fired: findings.into_iter().map(|f| f.pattern_name).collect(),
        suppressed: stats.suppressions,
        lines_skipped_oversize: stats.lines_skipped_oversize,
        control_fired,
    }
}

/// Did the case produce the outcome its category cares about?
fn observed(case: &Case, outcome: &Outcome) -> &'static str {
    match case.category.as_str() {
        "true_positive" => {
            let rule = case.rule.as_deref().unwrap_or_default();
            if outcome.fired.contains(rule) {
                "detected"
            } else {
                "missed"
            }
        }
        "gap_probe" | "oversize_probe" => {
            if outcome.fired.is_empty() {
                "missed"
            } else {
                "detected"
            }
        }
        "benign" => {
            if outcome.fired.is_empty() {
                "clean"
            } else {
                "flagged"
            }
        }
        other => panic!("unknown corpus category: {other}"),
    }
}

fn provenance_label(provenance: &AllowlistProvenance) -> String {
    match provenance {
        AllowlistProvenance::BuiltinShape => "BuiltinShape".to_string(),
        AllowlistProvenance::BuiltinKeyword => "BuiltinKeyword".to_string(),
        AllowlistProvenance::BuiltinBenignFixture => "BuiltinBenignFixture".to_string(),
        AllowlistProvenance::Custom { pattern } => format!("Custom({pattern})"),
        AllowlistProvenance::InlineIgnore { rule_id, .. } => {
            format!("InlineIgnore({rule_id})")
        }
    }
}

/// Why a planted credential was not reported: an allowlist suppression is a
/// different defect from "no rule in the catalogue knows this shape", and the
/// module exists to tell them apart.
fn miss_cause(outcome: &Outcome) -> String {
    if outcome.lines_skipped_oversize > 0 {
        return format!(
            "{} line(s) skipped oversize (SCAN-002)",
            outcome.lines_skipped_oversize
        );
    }
    if outcome.suppressed.is_empty() {
        return "no rule matched".to_string();
    }
    let tiers: BTreeSet<String> = outcome
        .suppressed
        .iter()
        .map(|s| format!("{} via {}", s.rule_name, provenance_label(&s.provenance)))
        .collect();
    format!(
        "suppressed: {}",
        tiers.into_iter().collect::<Vec<_>>().join(", ")
    )
}

fn rate(hits: usize, total: usize) -> String {
    if total == 0 {
        return "n/a".to_string();
    }
    #[allow(clippy::cast_precision_loss)] // corpus sizes are tiny
    let percent = (hits as f64 / total as f64) * 100.0;
    format!("{hits}/{total} = {percent:.1}%")
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

struct Tally {
    in_catalogue_detected: usize,
    in_catalogue_total: usize,
    gap_probe_detected: usize,
    gap_probe_total: usize,
    benign_flagged: usize,
    benign_total: usize,
    known_residual_flagged: usize,
    uncontrolled_benign: Vec<String>,
    silent_controls: Vec<String>,
    drift: Vec<String>,
}

#[allow(clippy::too_many_lines)] // one linear report; splitting it hides the shape
fn build_report(manifest: &Manifest, root: &Path) -> (String, Tally, Expected) {
    let mut report = String::new();
    let mut tally = Tally {
        in_catalogue_detected: 0,
        in_catalogue_total: 0,
        gap_probe_detected: 0,
        gap_probe_total: 0,
        benign_flagged: 0,
        benign_total: 0,
        known_residual_flagged: 0,
        uncontrolled_benign: Vec::new(),
        silent_controls: Vec::new(),
        drift: Vec::new(),
    };
    let mut per_rule_misses: Vec<String> = Vec::new();
    let mut covered_rules: BTreeSet<String> = BTreeSet::new();
    let mut gap_detected_by: BTreeMap<String, String> = BTreeMap::new();
    let mut vendored_attribution: Vec<String> = Vec::new();
    let mut vendored_benign_flags: Vec<String> = Vec::new();
    let mut oversize_lines = 0usize;

    let _ = writeln!(
        report,
        "=== SDT-002 secret-detection calibration report ==="
    );
    let _ = writeln!(
        report,
        "catalogue: {} built-in patterns + {} vendored ({})   corpus: {} cases",
        SECRET_PATTERNS.len(),
        VENDORED_COMPILED_PATTERNS.len(),
        *VENDORED_RULESET_VERSION,
        manifest.cases.len()
    );
    let _ = writeln!(
        report,
        "config: SecretCheckConfig::default() unless a case overrides it"
    );
    let _ = writeln!(report);
    let _ = writeln!(
        report,
        "-- planted credentials: rules the catalogue claims --"
    );

    for case in manifest
        .cases
        .iter()
        .filter(|c| c.category == "true_positive")
    {
        let outcome = measure(root, case);
        let seen = observed(case, &outcome);
        let rule = case.rule.clone().unwrap_or_default();
        covered_rules.insert(rule.clone());
        tally.in_catalogue_total += 1;
        if seen == "detected" {
            tally.in_catalogue_detected += 1;
            let _ = writeln!(report, "  [detected] {:<24} {rule}", case.id);
        } else {
            per_rule_misses.push(format!("{rule} ({}) — {}", case.id, miss_cause(&outcome)));
            let _ = writeln!(
                report,
                "  [MISSED  ] {:<24} {rule} — {}",
                case.id,
                miss_cause(&outcome)
            );
        }
        if seen != case.expect {
            tally.drift.push(format!(
                "{}: expected {}, measured {}",
                case.id, case.expect, seen
            ));
        }
    }

    let _ = writeln!(report);
    let _ = writeln!(
        report,
        "-- planted credentials: providers outside the catalogue (SDT-004 target) --"
    );
    for case in manifest.cases.iter().filter(|c| c.category == "gap_probe") {
        let outcome = measure(root, case);
        let seen = observed(case, &outcome);
        let provider = case.provider.clone().unwrap_or_default();
        tally.gap_probe_total += 1;
        if seen == "detected" {
            tally.gap_probe_detected += 1;
            let by = outcome.fired.iter().cloned().collect::<Vec<_>>().join(", ");
            gap_detected_by.insert(provider.clone(), by.clone());
            for (rule, version) in &outcome.vendored_fired {
                vendored_attribution
                    .push(format!("{:<24} {provider} <- {rule} [{version}]", case.id));
            }
            let _ = writeln!(report, "  [detected] {:<24} {provider} <- {by}", case.id);
        } else {
            let _ = writeln!(
                report,
                "  [MISSED  ] {:<24} {provider} — {}",
                case.id,
                miss_cause(&outcome)
            );
        }
        if seen != case.expect {
            tally.drift.push(format!(
                "{}: expected {}, measured {}",
                case.id, case.expect, seen
            ));
        }
    }

    let _ = writeln!(report);
    let _ = writeln!(report, "-- unscanned surface --");
    for case in manifest
        .cases
        .iter()
        .filter(|c| c.category == "oversize_probe")
    {
        let outcome = measure(root, case);
        let seen = observed(case, &outcome);
        oversize_lines += outcome.lines_skipped_oversize;
        let _ = writeln!(
            report,
            "  [{}] {:<24} {} (lines_skipped_oversize={})",
            if seen == "detected" {
                "detected"
            } else {
                "MISSED  "
            },
            case.id,
            case.note.clone().unwrap_or_default(),
            outcome.lines_skipped_oversize
        );
        if seen != case.expect {
            tally.drift.push(format!(
                "{}: expected {}, measured {}",
                case.id, case.expect, seen
            ));
        }
    }

    let _ = writeln!(report);
    let _ = writeln!(report, "-- known-benign vectors (false-positive half) --");
    for case in manifest.cases.iter().filter(|c| c.category == "benign") {
        let outcome = measure(root, case);
        let seen = observed(case, &outcome);
        tally.benign_total += 1;
        let control = match outcome.control_fired {
            Some(true) => "control fires",
            Some(false) => "CONTROL SILENT — case is vacuous",
            None => "no control declared",
        };
        if outcome.control_fired == Some(false) {
            tally.silent_controls.push(case.id.clone());
        }
        if outcome.control_fired.is_none() {
            tally.uncontrolled_benign.push(case.id.clone());
        }
        if seen == "flagged" {
            tally.benign_flagged += 1;
            if case.known_residual {
                tally.known_residual_flagged += 1;
            }
            // SDT-004: the false-positive cost of the vendored tier, isolated.
            // "FP rate unchanged" is only meaningful if we can also say no
            // vendored rule is the thing doing the flagging.
            for rule in outcome.vendored_fired.keys() {
                vendored_benign_flags.push(format!("{} <- {rule}", case.id));
            }
            let by = outcome
                .fired_detail
                .iter()
                .map(|(rule, redacted)| format!("{rule} ({redacted})"))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                report,
                "  [{}] {:<28} {by}",
                if case.known_residual {
                    "residual"
                } else {
                    "FALSE POS"
                },
                case.id
            );
        } else {
            let suppressed_by: BTreeSet<String> = outcome
                .suppressed
                .iter()
                .map(|s| provenance_label(&s.provenance))
                .collect();
            let how = if suppressed_by.is_empty() {
                "never matched".to_string()
            } else {
                format!(
                    "suppressed via {}",
                    suppressed_by.into_iter().collect::<Vec<_>>().join(", ")
                )
            };
            let _ = writeln!(report, "  [clean   ] {:<28} {how}; {control}", case.id);
        }
        if seen != case.expect {
            tally.drift.push(format!(
                "{}: expected {}, measured {}",
                case.id, case.expect, seen
            ));
        }
    }

    let uncovered: Vec<&str> = SECRET_PATTERNS
        .iter()
        .map(|pattern| pattern.name)
        .filter(|name| !covered_rules.contains(*name))
        .collect();

    let planted_detected = tally.in_catalogue_detected + tally.gap_probe_detected;
    let planted_total = tally.in_catalogue_total + tally.gap_probe_total;

    let _ = writeln!(report);
    let _ = writeln!(report, "-- headline numbers --");
    let _ = writeln!(
        report,
        "  detection rate, catalogue rules ...... {}",
        rate(tally.in_catalogue_detected, tally.in_catalogue_total)
    );
    let _ = writeln!(
        report,
        "  detection rate, other providers ...... {}",
        rate(tally.gap_probe_detected, tally.gap_probe_total)
    );
    let _ = writeln!(
        report,
        "  detection rate, all planted secrets .. {}",
        rate(planted_detected, planted_total)
    );
    let _ = writeln!(
        report,
        "  false-positive rate .................. {} (known residual: {})",
        rate(tally.benign_flagged, tally.benign_total),
        tally.known_residual_flagged
    );
    let _ = writeln!(
        report,
        "  unscanned lines across the corpus .... {oversize_lines}"
    );
    let _ = writeln!(report);
    let _ = writeln!(report, "  per-rule misses (catalogue rules):");
    if per_rule_misses.is_empty() {
        let _ = writeln!(report, "    none");
    } else {
        for miss in &per_rule_misses {
            let _ = writeln!(report, "    {miss}");
        }
    }
    let _ = writeln!(report, "  catalogue rules with no true-positive case:");
    if uncovered.is_empty() {
        let _ = writeln!(report, "    none");
    } else {
        for name in &uncovered {
            let _ = writeln!(report, "    {name}");
        }
    }
    let _ = writeln!(report);
    let _ = writeln!(
        report,
        "  vendored tier-1 attribution ({} rule(s) from {}):",
        VENDORED_COMPILED_PATTERNS.len(),
        *VENDORED_RULESET_VERSION
    );
    if vendored_attribution.is_empty() {
        let _ = writeln!(
            report,
            "    none — no gap probe was closed by a vendored rule"
        );
    } else {
        for line in &vendored_attribution {
            let _ = writeln!(report, "    {line}");
        }
    }
    let _ = writeln!(
        report,
        "  benign cases flagged by a vendored rule: {}",
        if vendored_benign_flags.is_empty() {
            "none".to_string()
        } else {
            vendored_benign_flags.join(", ")
        }
    );
    let _ = writeln!(
        report,
        "  benign cases with no non-vacuity control: {}",
        if tally.uncontrolled_benign.is_empty() {
            "none".to_string()
        } else {
            tally.uncontrolled_benign.join(", ")
        }
    );
    let _ = writeln!(report);
    let _ = writeln!(
        report,
        "NOTE: the all-planted rate depends on corpus composition ({} catalogue \
         canaries + {} out-of-catalogue providers). It is comparable across \
         revisions of this corpus, not an absolute statement about the field.",
        tally.in_catalogue_total, tally.gap_probe_total
    );

    let measured = Expected {
        in_catalogue_detected: tally.in_catalogue_detected,
        in_catalogue_total: tally.in_catalogue_total,
        gap_probe_detected: tally.gap_probe_detected,
        gap_probe_total: tally.gap_probe_total,
        planted_detected,
        planted_total,
        benign_flagged: tally.benign_flagged,
        benign_total: tally.benign_total,
    };

    if !uncovered.is_empty() {
        tally.drift.push(format!(
            "catalogue rules with no true-positive case: {}",
            uncovered.join(", ")
        ));
    }

    (report, tally, measured)
}

// ---------------------------------------------------------------------------
// The gate
// ---------------------------------------------------------------------------

#[test]
fn secret_calibration_corpus_matches_the_committed_baseline() {
    let root = corpus_root();
    let manifest_path = root.join("manifest.json");
    let raw = std::fs::read_to_string(&manifest_path).unwrap_or_else(|err| {
        panic!(
            "corpus manifest unreadable at {}: {err}",
            manifest_path.display()
        )
    });
    let manifest: Manifest = serde_json::from_str(&raw).expect("corpus manifest is valid JSON");

    let (report, tally, measured) = build_report(&manifest, &root);
    // Printed before any assertion so the report is on stdout whether the run
    // is green or red — CI reads it either way.
    println!("{report}");

    assert_eq!(
        manifest.catalogue_size,
        SECRET_PATTERNS.len(),
        "the corpus manifest records {} built-in patterns but the catalogue has {}; \
         a rule was added or removed without a corpus case",
        manifest.catalogue_size,
        SECRET_PATTERNS.len()
    );

    assert_eq!(
        manifest.vendored_ruleset.as_deref(),
        Some(VENDORED_RULESET_VERSION.as_str()),
        "the corpus baseline was measured against vendored ruleset {:?} but the tree now \
         carries {:?}. A ruleset refresh moves detection and false-positive rate, so re-run \
         `pnpm secret:calibrate`, update `vendored_ruleset` and `expected` in manifest.json, \
         and record the before/after in plans/archive/modules/secret-detection-truth.aps.md (SDT-004).",
        manifest.vendored_ruleset,
        VENDORED_RULESET_VERSION.as_str(),
    );

    assert!(
        tally.uncontrolled_benign.is_empty(),
        "benign case(s) {:?} declare no non-vacuity control and must not count \
         towards the measured false-positive rate",
        tally.uncontrolled_benign
    );

    assert!(
        tally.silent_controls.is_empty(),
        "benign case(s) {:?} are vacuous — their non-vacuity control did not fire, \
         so the scanner was never going to flag them and they measure nothing",
        tally.silent_controls
    );

    assert!(
        tally.drift.is_empty() && measured == manifest.expected,
        "CALIBRATION DRIFT — the measured numbers no longer match \
         tests/corpus/secret/manifest.json.\n\
         \n\
         per-case drift:\n  {}\n\
         \n\
         committed: {:?}\n\
         measured:  {:?}\n\
         \n\
         This is not a failure to paper over: a rules change moved detection or \
         false-positive rate. Update `expected` and the affected `expect` fields \
         in manifest.json in the SAME change, and record the before/after in \
         plans/archive/modules/secret-detection-truth.aps.md (SDT-002).",
        if tally.drift.is_empty() {
            "(aggregates only)".to_string()
        } else {
            tally.drift.join("\n  ")
        },
        manifest.expected,
        measured,
    );
}

/// The corpus keeps every credential-shaped literal inside `cases/*.corpus`,
/// an extension no repository-wide walker selects (`anvil gate` scans
/// `.ts/.js/.rs/.json/.yaml/.yml/.toml/.env` and `.env*` names; `anvil audit`
/// and the discovery harness use comparable allowlists). The files that *are*
/// scanned by anvil's own gate — this runner and the corpus manifest — must
/// therefore stay clean, or the corpus turns anvil's dogfood run red against
/// itself. This test is the trip-wire for that boundary.
#[test]
fn corpus_metadata_stays_clean_under_the_scanner() {
    let root = corpus_root();
    let config = SecretCheckConfig::default();
    let scanned: [(&str, PathBuf); 3] = [
        ("manifest.json", root.join("manifest.json")),
        ("PROVENANCE.md", root.join("PROVENANCE.md")),
        (
            "secret_calibration.rs",
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/secret_calibration.rs"),
        ),
    ];

    for (label, path) in scanned {
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{label} unreadable at {}: {err}", path.display()));
        let (findings, _) = scan_content_with_stats(&content, label, &config);
        let reported: Vec<String> = findings
            .iter()
            .map(|f| {
                format!(
                    "{}:{} {} ({})",
                    label, f.line, f.pattern_name, f.redacted_match
                )
            })
            .collect();
        assert!(
            findings.is_empty(),
            "{label} is scanned by anvil's own gate and must carry no credential-shaped \
             values; canaries belong in cases/*.corpus. Reported: {reported:?}"
        );
    }
}

#[test]
fn control_requires_expected_rule_delta() {
    let finding = |pattern_name: &str| SecretFinding {
        pattern_name: pattern_name.to_owned(),
        ..SecretFinding::default()
    };
    let baseline = vec![finding("High Entropy String")];
    let unchanged_control = vec![finding("High Entropy String")];
    let intended_control = vec![finding("High Entropy String"), finding("API Key")];

    assert!(!control_added_expected_finding(
        &baseline,
        &unchanged_control,
        "API Key",
    ));
    assert!(control_added_expected_finding(
        &baseline,
        &intended_control,
        "API Key",
    ));
}
