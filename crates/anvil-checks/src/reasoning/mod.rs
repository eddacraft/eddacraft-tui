//! Reasoning-pattern checks (AI-001+).
//!
//! Reasoning rules live alongside `secret/`, `antipattern/`, and
//! `command_safety/` per the locked 2026-04-26 decision (see
//! `plans/modules/realtime-ai-validation.aps.md`, Open Question 3). The
//! family flags comments whose prose justifies dubious code with appeals
//! to authority, social proof, or deflection rather than technical
//! reasoning. The first rule, AI-001 (appeal-to-authority), lives in
//! [`appeal_to_authority`]; AI-002+ will extend the catalogue here.

pub mod appeal_to_authority;
pub mod types;

pub use appeal_to_authority::{
    RULE_ID as APPEAL_TO_AUTHORITY_RULE_ID, scan_file as scan_appeal_to_authority,
};
pub use types::{ReasoningCheckConfig, ReasoningCheckResult};

use anvil_kernel_types::Diagnostic;

/// Run every shipping reasoning rule against a file's contents.
///
/// Mirrors `run_secret_check` / `run_antipattern_check` so the surface API
/// stays uniform across rule families. The result aggregates findings from
/// every rule whose ID is empty in `config.rule_ids` (default — run all)
/// or explicitly listed.
#[must_use]
pub fn run_reasoning_check(
    files: &[(&str, &str)],
    config: &ReasoningCheckConfig,
) -> ReasoningCheckResult {
    let want_appeal = config.rule_ids.is_empty()
        || config
            .rule_ids
            .iter()
            .any(|id| id == APPEAL_TO_AUTHORITY_RULE_ID);

    let mut findings: Vec<Diagnostic> = Vec::new();
    for (file, content) in files {
        if want_appeal {
            findings.extend(scan_appeal_to_authority(file, content));
        }
    }

    if findings.is_empty() {
        return ReasoningCheckResult::clean();
    }

    let count = findings.len();
    // Reasoning rules ship as info-only signals; the score is informational
    // and tracks the same shape as the other rule families. -10 per finding,
    // floored at 0.
    let score_usize = 100_usize.saturating_sub(count.saturating_mul(10));
    let score = u8::try_from(score_usize).unwrap_or(0);

    ReasoningCheckResult {
        passed: false,
        score,
        message: format!("Found {count} reasoning-pattern signal(s)"),
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::{ReasoningCheckConfig, run_reasoning_check};

    #[test]
    fn run_reasoning_check_collects_findings_across_files() {
        let files = [
            (
                "src/a.rs",
                "// the lead said to skip this branch\nfn a() {}\n",
            ),
            ("src/b.rs", "// returns the cached entry\nfn b() {}\n"),
            (
                "scripts/c.sh",
                "# the manager wants this disabled in prod\nexit 0\n",
            ),
        ];
        let result = run_reasoning_check(&files, &ReasoningCheckConfig::default());
        assert!(!result.passed);
        assert_eq!(result.findings.len(), 2);
        assert_eq!(result.score, 80);
    }

    #[test]
    fn run_reasoning_check_returns_clean_when_no_match() {
        let files = [("src/a.rs", "// returns the cached entry\nfn a() {}\n")];
        let result = run_reasoning_check(&files, &ReasoningCheckConfig::default());
        assert!(result.passed);
        assert_eq!(result.score, 100);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn run_reasoning_check_respects_explicit_rule_filter() {
        // Asking for an unknown rule yields zero findings even when AI-001
        // would otherwise fire.
        let files = [(
            "src/a.rs",
            "// the lead said to skip this branch\nfn a() {}\n",
        )];
        let config = ReasoningCheckConfig {
            rule_ids: vec!["AI-999".to_string()],
        };
        let result = run_reasoning_check(&files, &config);
        assert!(result.passed);
        assert!(result.findings.is_empty());
    }
}
