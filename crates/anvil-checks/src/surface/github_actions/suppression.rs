//! `@anvil-ignore` resolution for SURFGHA rules.
//!
//! Per [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md)
//! every Track 3 surface reuses the canonical
//! [`crate::antipattern::parse_suppression`] entry point. Workflow YAML uses
//! the `#` comment style, which is already in that parser's grammar:
//!
//! ```yaml
//! # @anvil-ignore SURFGHA-002 -- vetted internal action, pinned next sprint
//! - uses: myorg/internal@main
//! ```
//!
//! Mirrors the surface-agnostic SURFSQL/SURFENV helpers; a future refactor
//! may hoist these into a shared `surface` helper.

use crate::antipattern::parse_suppression;

/// Sentinel [`crate::antipattern::parse_suppression`] returns when a
/// directive omits the `-- <reason>` clause; normalised to `None` here.
const PARSE_SUPPRESSION_NO_REASON_SENTINEL: &str = "No reason provided";

fn normalise_reason(reason: String) -> Option<String> {
    if reason == PARSE_SUPPRESSION_NO_REASON_SENTINEL {
        None
    } else {
        Some(reason)
    }
}

/// Resolve a directive on the line immediately above `line_number`
/// (1-indexed). Returns `(suppressed, reason)`.
#[must_use]
pub fn resolve_line_suppression(
    lines: &[&str],
    line_number: usize,
    rule_id: &str,
) -> (bool, Option<String>) {
    if line_number <= 1 {
        return (false, None);
    }
    let previous = lines.get(line_number - 2).copied().unwrap_or("");
    let Some((id, reason)) = parse_suppression(previous) else {
        return (false, None);
    };
    if id != rule_id {
        return (false, None);
    }
    (true, normalise_reason(reason))
}

#[cfg(test)]
mod tests {
    use super::resolve_line_suppression;

    #[test]
    fn yaml_comment_directive_suppresses_following_line() {
        let lines = vec![
            "# @anvil-ignore SURFGHA-002 -- pinned next sprint",
            "- uses: foo/bar@main",
        ];
        let (suppressed, reason) = resolve_line_suppression(&lines, 2, "SURFGHA-002");
        assert!(suppressed);
        assert_eq!(reason.as_deref(), Some("pinned next sprint"));
    }

    #[test]
    fn directive_for_other_rule_does_not_suppress() {
        let lines = vec![
            "# @anvil-ignore SURFGHA-003 -- different rule",
            "- uses: foo/bar@main",
        ];
        let (suppressed, _) = resolve_line_suppression(&lines, 2, "SURFGHA-002");
        assert!(!suppressed);
    }
}
