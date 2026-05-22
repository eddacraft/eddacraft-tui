//! Scanner pattern catalogue — registry-backed.
//!
//! Prior to RSCAN-004 this file carried a hardcoded `PATTERN_DEFS` array of 13
//! rules (AP-001..AP-013). That array predated the `.anvil` format and drifted
//! from the TS scanner's catalogue. ADR-026 established the compiled registry
//! (`patterns/compiled/registry.json`) as the single source of truth, and this
//! module now exposes it to the scanner via a `LazyLock<Vec<AntiPattern>>`.
//!
//! Retired HTML + CSS rules are no longer part of the registry, so no
//! catalogue entries should target HTML/CSS extensions or categories.
//! AP-008 and AP-009 are now dynamic-execution rules, so the test below
//! guards the semantic retirement instead of treating those IDs as reserved.

use std::sync::LazyLock;

use crate::antipattern::registry_loader::{LoadRegistryOptions, load_registry_patterns};
use crate::antipattern::types::AntiPattern;

static PATTERN_CATALOGUE: LazyLock<Vec<AntiPattern>> =
    LazyLock::new(|| load_registry_patterns(&LoadRegistryOptions::default()));

#[must_use]
pub fn all_patterns() -> Vec<AntiPattern> {
    PATTERN_CATALOGUE.clone()
}

#[must_use]
pub fn get_pattern(id: &str) -> Option<AntiPattern> {
    PATTERN_CATALOGUE
        .iter()
        .find(|pattern| pattern.id == id)
        .cloned()
}

#[must_use]
pub fn get_enabled_patterns() -> Vec<AntiPattern> {
    PATTERN_CATALOGUE
        .iter()
        .filter(|pattern| pattern.enabled)
        .cloned()
        .collect()
}

#[must_use]
pub fn get_default_patterns() -> Vec<AntiPattern> {
    PATTERN_CATALOGUE
        .iter()
        .filter(|pattern| pattern.enabled && !pattern.opt_in)
        .cloned()
        .collect()
}

#[must_use]
pub fn get_pattern_ids() -> Vec<String> {
    PATTERN_CATALOGUE
        .iter()
        .map(|pattern| pattern.id.clone())
        .collect()
}

#[must_use]
pub fn is_valid_pattern_id(id: &str) -> bool {
    PATTERN_CATALOGUE.iter().any(|pattern| pattern.id == id)
}

/// Count of patterns currently loaded from the registry.
#[must_use]
pub fn patterns_count() -> usize {
    PATTERN_CATALOGUE.len()
}

#[cfg(test)]
mod tests {
    use crate::antipattern::patterns::{
        all_patterns, get_default_patterns, get_enabled_patterns, get_pattern, get_pattern_ids,
        is_valid_pattern_id, patterns_count,
    };

    #[test]
    fn catalogue_is_non_empty_when_registry_is_available() {
        let patterns = all_patterns();
        assert!(
            !patterns.is_empty(),
            "registry-backed catalogue must load at least one pattern"
        );
        assert_eq!(get_pattern_ids().len(), patterns.len());
        assert_eq!(patterns_count(), patterns.len());
    }

    #[test]
    fn filters_default_and_opt_in_patterns() {
        let default_patterns = get_default_patterns();
        let enabled_patterns = get_enabled_patterns();

        assert!(
            default_patterns.len() <= enabled_patterns.len(),
            "defaults must be a subset of enabled"
        );
        assert!(default_patterns.iter().all(|pattern| !pattern.opt_in));
        assert!(enabled_patterns.iter().all(|pattern| pattern.enabled));
    }

    #[test]
    fn returns_core_anti_patterns_from_registry() {
        assert!(get_pattern("AP-001").is_some(), "AP-001 missing");
        assert!(get_pattern("AP-003").is_some(), "AP-003 missing");
        assert!(get_pattern("AP-006").is_some(), "AP-006 missing");
    }

    #[test]
    fn retired_html_css_patterns_are_absent() {
        use crate::antipattern::types::AntiPatternCategory;

        for id in ["AP-010", "AP-011", "AP-012", "AP-013"] {
            assert!(
                get_pattern(id).is_none(),
                "retired HTML/CSS pattern {id} must not appear in catalogue"
            );
        }

        for pattern in all_patterns() {
            assert!(
                !matches!(
                    pattern.category,
                    AntiPatternCategory::Html | AntiPatternCategory::Css
                ),
                "retired HTML/CSS pattern category must not appear in catalogue: {}",
                pattern.id
            );

            // Match the scanner's case-insensitive extension comparison
            // (`scanner.rs::matches_file_extension` uses `eq_ignore_ascii_case`)
            // so the retirement guard catches `.HTML` / `.CSS` and any other
            // casing — not just the lowercase form.
            let targets_html_or_css = pattern
                .file_extensions
                .as_deref()
                .unwrap_or_default()
                .iter()
                .any(|ext| {
                    [".html", ".htm", ".css", ".scss", ".less"]
                        .iter()
                        .any(|retired| ext.eq_ignore_ascii_case(retired))
                });

            assert!(
                !targets_html_or_css,
                "retired HTML/CSS file targets must not appear in catalogue: {}",
                pattern.id
            );
        }
    }

    #[test]
    fn validates_pattern_ids() {
        assert!(is_valid_pattern_id("AP-001"));
        assert!(!is_valid_pattern_id("AP-999"));
    }

    #[test]
    fn registry_backed_patterns_carry_family_provenance() {
        let ap003 = get_pattern("AP-003").expect("AP-003 exists");
        assert!(
            ap003.family.is_some(),
            "AP-003 must carry family provenance from registry"
        );
        assert!(ap003.definition_ref.is_some());
        assert!(ap003.spectrum_position.is_some());
    }
}
