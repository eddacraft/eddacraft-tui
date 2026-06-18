//! `@anvil-ignore` resolution for SURFSQL rules.
//!
//! Per [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md)
//! every Track 3 surface reuses the canonical
//! [`crate::antipattern::parse_suppression`] entry point — the SQL `--`
//! comment style is already in that parser's grammar, so a directive reads:
//!
//! ```sql
//! -- @anvil-ignore SURFSQL-002 -- one-off cleanup, data already archived
//! DROP TABLE legacy_events;
//! ```
//!
//! These wrappers mirror the SURFENV helpers (`super::super::env::suppression`);
//! the logic is surface-agnostic and a future refactor may hoist it into a
//! shared `surface` helper.

use crate::antipattern::parse_suppression;

/// Maximum number of leading file lines scanned for a file-level SURFSQL
/// suppression directive. Bounded so a huge generated migration doesn't pay
/// for an exhaustive scan.
pub const HEADER_LINE_BUDGET: usize = 5;

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

/// Resolve a directive in the first [`HEADER_LINE_BUDGET`] lines of a file.
/// Used by future per-file SURFSQL rules.
#[must_use]
pub fn resolve_file_header_suppression(content: &str, rule_id: &str) -> (bool, Option<String>) {
    for line in content.lines().take(HEADER_LINE_BUDGET) {
        if let Some((id, reason)) = parse_suppression(line)
            && id == rule_id
        {
            return (true, normalise_reason(reason));
        }
    }
    (false, None)
}

#[cfg(test)]
mod tests {
    use super::{resolve_file_header_suppression, resolve_line_suppression};

    #[test]
    fn sql_comment_directive_suppresses_following_statement() {
        let lines = vec![
            "-- @anvil-ignore SURFSQL-002 -- data already archived",
            "DROP TABLE legacy_events;",
        ];
        let (suppressed, reason) = resolve_line_suppression(&lines, 2, "SURFSQL-002");
        assert!(suppressed);
        assert_eq!(reason.as_deref(), Some("data already archived"));
    }

    #[test]
    fn directive_for_other_rule_does_not_suppress() {
        let lines = vec![
            "-- @anvil-ignore SURFSQL-003 -- different rule",
            "DROP TABLE legacy_events;",
        ];
        let (suppressed, _) = resolve_line_suppression(&lines, 2, "SURFSQL-002");
        assert!(!suppressed);
    }

    #[test]
    fn directive_without_reason_normalises_to_none() {
        let lines = vec!["-- @anvil-ignore SURFSQL-002", "TRUNCATE events;"];
        let (suppressed, reason) = resolve_line_suppression(&lines, 2, "SURFSQL-002");
        assert!(suppressed);
        assert!(reason.is_none());
    }

    #[test]
    fn header_directive_resolves_within_budget() {
        let content =
            "-- migration 042\n-- @anvil-ignore SURFSQL-002 -- bulk seed\nDELETE FROM seeds;\n";
        let (suppressed, reason) = resolve_file_header_suppression(content, "SURFSQL-002");
        assert!(suppressed);
        assert_eq!(reason.as_deref(), Some("bulk seed"));
    }
}
