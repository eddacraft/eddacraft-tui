//! SDT-004: the vendored gitleaks ruleset, tier 1.
//!
//! ADR-136 fixes the acquisition posture: Anvil vendors detection **knowledge
//! as data** and compiles it into its own scanner. No third-party engine,
//! binary, or runtime enters the product.
//!
//! # Why these rules do not go through `SecretPatternDef`
//!
//! ADR-136 §1 describes the conversion target as Anvil's existing
//! [`SecretPatternDef`](crate::secret::types::SecretPatternDef) form. That form
//! cannot carry the property tier 1 depends on. `SecretPatternDef` is
//! `{ name, pattern }`, and
//! [`compile_custom_patterns`](crate::secret::patterns::compile_custom_patterns)
//! hardcodes `high_confidence: false` because "the scanner cannot know whether
//! a hand-written regex is structurally unambiguous".
//!
//! That reasoning is correct *about operator-supplied regexes* and false about
//! these. A tier-1 rule matches a credential carrying a literal,
//! provider-specific prefix — the match **is** the credential — and it arrives
//! with a digest-verified upstream provenance rather than out of somebody's
//! `.anvilrc`. Routed through `SecretPatternDef`, every tier-1 rule would
//! inherit the fuzzy filter stack: the `example`/`test`/`dummy` keyword
//! allowlist that used to eat textbook credentials (issue #1800) and
//! `looks_like_code`, whose mixed-case arm eats camelCase bindings (CIB-363).
//! Tier 1 is entirely prefix-anchored, so that would hit all of it.
//!
//! Extending `SecretPatternDef` with a confidence field was rejected for a
//! second, independent reason: it is `Serialize`/`Deserialize` and
//! operator-facing, so the field would become a config switch letting any
//! `.anvilrc` custom pattern opt itself out of the false-positive filters, with
//! no provenance behind the claim. Vendored rules are repo-owned constants like
//! the built-ins, so they compile through the built-in path instead, and
//! `SecretPatternDef` is left untouched.
//!
//! Anvil's allowlist and suppression layer applies to these rules exactly as it
//! does to built-ins (ADR-136 §1): shape-anchored allowlist entries and any
//! operator `custom_allowlist` entry still suppress a vendored match, and the
//! suppression is recorded with `AllowlistProvenance` like any other.
//!
//! # The data
//!
//! `vendor/gitleaks/tier1.json` is **generated** by
//! `scripts/secret/refresh-gitleaks-ruleset.sh` from a digest-verified upstream
//! pin; `--check` fails CI on drift. See `vendor/gitleaks/PROVENANCE.md`.

use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use crate::secret::patterns::CompiledPattern;

/// The generated tier-1 data, embedded at compile time so the shipped binary
/// carries no runtime file dependency.
const TIER1_JSON: &str = include_str!("vendor/gitleaks/tier1.json");

#[derive(Debug, Deserialize)]
pub struct VendoredRuleset {
    /// Upstream release tag — the value that reaches finding provenance and the
    /// calibration report.
    pub version: String,
    /// Upstream commit the rules were converted from.
    pub commit: String,
    /// Upstream ruleset name (`gitleaks`).
    pub ruleset: String,
    /// Confidence tier these rules belong to.
    pub tier: u8,
    pub rules: Vec<VendoredRule>,
}

#[derive(Debug, Deserialize)]
pub struct VendoredRule {
    /// Upstream rule id, used verbatim as the Anvil rule name so a finding
    /// names something a reader can look up upstream.
    pub id: String,
    pub description: String,
    /// The upstream regex, **verbatim**. Never rewritten during conversion.
    pub pattern: String,
    /// Which capture group holds the credential, when upstream wraps it in
    /// delimiter or keyword scaffolding. `None` means the whole match is the
    /// credential.
    pub secret_group: Option<usize>,
    /// The literal provider prefix the rule is anchored on — the tier-1
    /// criterion, recorded so it is auditable rather than asserted.
    pub prefix: String,
}

/// The parsed ruleset. Malformed vendored data is a developer/refresh bug, not
/// a runtime condition: it is a generated, committed artefact, so we panic
/// rather than silently ship a smaller catalogue.
pub static VENDORED_RULESET: LazyLock<VendoredRuleset> = LazyLock::new(|| {
    serde_json::from_str(TIER1_JSON)
        .unwrap_or_else(|err| panic!("vendored tier-1 ruleset is not valid JSON: {err}"))
});

/// Ruleset version string carried into finding provenance and the SDT-002
/// calibration report — e.g. `gitleaks@v8.30.1 tier1`.
pub static VENDORED_RULESET_VERSION: LazyLock<String> = LazyLock::new(|| {
    let ruleset = &*VENDORED_RULESET;
    format!(
        "{}@{} tier{}",
        ruleset.ruleset, ruleset.version, ruleset.tier
    )
});

/// The vendored rules compiled once per process, in the same shape as the
/// built-ins and with the same fail-loud contract: a rule that will not compile
/// under the `regex` crate is a refresh bug that must be a CI failure, never a
/// silent reduction in detection coverage.
/// Compile one vendored rule into the built-in `CompiledPattern` shape.
///
/// Separate from the `LazyLock` so the fail-loud contract below is reachable by
/// test: a guard that only runs inside a static initialiser cannot be proved to
/// fire.
fn compile_vendored_rule(rule: &VendoredRule, version: &str) -> CompiledPattern {
    let regex = Regex::new(&rule.pattern).unwrap_or_else(|err| {
        panic!(
            "vendored secret rule `{}` failed to compile: {err}. Upstream regexes are taken \
             verbatim, so a rule using a construct the `regex` crate does not support must be \
             removed from tier1-rules.txt deliberately",
            rule.id
        )
    });
    // A `secret_group` that does not exist in the compiled regex would make
    // `match_ranges` return nothing for every line — the rule would sit in the
    // catalogue and detect nothing, which is exactly the silent coverage loss
    // this module exists to stop. Group 0 is the whole match and is spelled
    // `None`, so a declared group must be 1-based and within `captures_len()`.
    if let Some(group) = rule.secret_group {
        let capture_count = regex.captures_len();
        assert!(
            group >= 1 && group < capture_count,
            "vendored secret rule `{}` declares secret_group {group}, which is not a capture \
             group of its regex (it has {} group(s) besides the whole match). The generated data \
             and the regex disagree — regenerate with \
             scripts/secret/refresh-gitleaks-ruleset.sh rather than hand-editing",
            rule.id,
            capture_count.saturating_sub(1)
        );
    }
    CompiledPattern {
        name: rule.id.clone(),
        regex,
        // Tier 1 is prefix-anchored by construction — the match is the
        // credential — so these carry the same confidence as the built-in shape
        // patterns. See the module doc.
        high_confidence: true,
        secret_group: rule.secret_group,
        ruleset_version: Some(version.to_string()),
    }
}

/// The vendored rules compiled once per process, in the same shape as the
/// built-ins and with the same fail-loud contract: a rule that will not compile
/// under the `regex` crate, or whose declared capture group does not exist, is a
/// refresh bug that must be a CI failure, never a silent reduction in detection
/// coverage.
pub static VENDORED_COMPILED_PATTERNS: LazyLock<Vec<CompiledPattern>> = LazyLock::new(|| {
    let version = VENDORED_RULESET_VERSION.clone();
    VENDORED_RULESET
        .rules
        .iter()
        .map(|rule| compile_vendored_rule(rule, &version))
        .collect()
});

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        VENDORED_COMPILED_PATTERNS, VENDORED_RULESET, VENDORED_RULESET_VERSION, VendoredRule,
        compile_vendored_rule,
    };

    fn rule(pattern: &str, secret_group: Option<usize>) -> VendoredRule {
        VendoredRule {
            id: "probe-rule".to_string(),
            description: "probe".to_string(),
            pattern: pattern.to_string(),
            secret_group,
            prefix: "probe_".to_string(),
        }
    }

    #[test]
    fn a_declared_capture_group_that_does_not_exist_is_a_loud_failure() {
        // Without the guard this rule compiles fine and then matches nothing on
        // every line: present in the catalogue, detecting zero. Reverting the
        // assert in `compile_vendored_rule` fails this test.
        // `.err()` rather than `expect_err`: the Ok arm is `CompiledPattern`,
        // which is deliberately not `Debug` (it would print a compiled regex
        // set into test output).
        let err = std::panic::catch_unwind(|| {
            compile_vendored_rule(&rule(r"\bprobe_[a-f0-9]{8}", Some(1)), "probe@v0 tier1")
        })
        .err()
        .expect("a secret_group with no matching capture group must panic");
        let message = err
            .downcast_ref::<String>()
            .map(String::as_str)
            .unwrap_or_default();
        assert!(
            message.contains("secret_group 1"),
            "the panic must name the offending group, got {message:?}"
        );
    }

    #[test]
    fn group_zero_is_rejected_because_none_is_how_the_whole_match_is_spelled() {
        assert!(
            std::panic::catch_unwind(|| {
                compile_vendored_rule(&rule(r"\b(probe_[a-f0-9]{8})", Some(0)), "probe@v0 tier1")
            })
            .is_err(),
            "group 0 is the whole match; declaring it would be a data error, not a narrowing"
        );
    }

    #[test]
    fn a_valid_declared_group_compiles() {
        let compiled =
            compile_vendored_rule(&rule(r"\b(probe_[a-f0-9]{8})'?", Some(1)), "probe@v0 tier1");
        assert_eq!(compiled.secret_group, Some(1));
        assert!(compiled.high_confidence);
        assert_eq!(compiled.ruleset_version.as_deref(), Some("probe@v0 tier1"));
        let line = "TOKEN=probe_0badf00d'";
        let range = compiled
            .first_match_range(line)
            .expect("the probe rule fires");
        assert_eq!(&line[range], "probe_0badf00d");
    }

    #[test]
    fn every_vendored_rule_compiles_under_the_regex_crate() {
        assert_eq!(
            VENDORED_COMPILED_PATTERNS.len(),
            VENDORED_RULESET.rules.len(),
            "every vendored rule must compile; the LazyLock panics otherwise"
        );
        assert!(
            !VENDORED_COMPILED_PATTERNS.is_empty(),
            "tier 1 must not be empty — an empty vendored set is a silent loss of detection"
        );
    }

    #[test]
    fn vendored_rule_ids_are_unique() {
        let unique: BTreeSet<&str> = VENDORED_RULESET
            .rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect();
        assert_eq!(
            unique.len(),
            VENDORED_RULESET.rules.len(),
            "duplicate vendored rule id would report one credential twice"
        );
    }

    #[test]
    fn every_vendored_rule_is_prefix_anchored_and_high_confidence() {
        // The tier-1 criterion, asserted in the product rather than only in the
        // generator: a rule with no literal provider prefix is not tier 1, and
        // shipping it as high-confidence would exempt a fuzzy rule from the
        // false-positive filters.
        for rule in &VENDORED_RULESET.rules {
            assert!(
                rule.prefix.len() >= 4,
                "vendored rule {} has prefix {:?}, which is too short to anchor on",
                rule.id,
                rule.prefix
            );
        }
        assert!(
            VENDORED_COMPILED_PATTERNS
                .iter()
                .all(|pattern| pattern.high_confidence),
            "tier-1 rules compile high-confidence by construction"
        );
    }

    #[test]
    fn ruleset_version_names_the_upstream_pin() {
        let version = &*VENDORED_RULESET_VERSION;
        assert!(version.starts_with("gitleaks@v"), "got {version}");
        assert!(version.ends_with("tier1"), "got {version}");
    }
}
