//! Shared `@anvil-ignore` resolution for SURFENV structural rules.
//!
//! Per [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md)
//! every Track 3 surface module reuses the canonical
//! [`crate::antipattern::parse_suppression`] entry point. This module
//! wraps that parser into the two flavours SURFENV rules actually use:
//!
//! - [`resolve_line_suppression`] — directive on the line *immediately
//!   above* an offending entry. Used by SURFENV-001 (secret in value)
//!   and SURFENV-003 (production-shaped value), where each finding is
//!   anchored to a single source line.
//! - [`resolve_file_header_suppression`] — directive in the first few
//!   lines of an env file. Used by SURFENV-002 (`.gitignore` hygiene)
//!   and SURFENV-004 (`.env.example` drift), where the finding is
//!   per-file and there's no specific entry to anchor to.
//!
//! Both functions ignore directives whose rule ID doesn't match the
//! caller — leaking suppression across rules is exactly the bug
//! ADR-029 was meant to prevent.

use crate::antipattern::parse_suppression;

/// Maximum number of leading file lines scanned for a SURFENV
/// file-level suppression directive. Bounded so a multi-megabyte
/// `.env` (vendor-dump) doesn't pay for an exhaustive scan.
pub const HEADER_LINE_BUDGET: usize = 5;

/// Sentinel string [`crate::antipattern::parse_suppression`] returns
/// when a directive omits the `-- <reason>` clause. The shared SURFENV
/// helpers normalise it to `None` so callers can pattern-match on
/// `.is_none()` to detect a missing reason — without this filter,
/// every reason-less directive would surface as
/// `Some("No reason provided")`, an observable contract violation
/// caught in council review.
const PARSE_SUPPRESSION_NO_REASON_SENTINEL: &str = "No reason provided";

fn normalise_reason(reason: String) -> Option<String> {
    if reason == PARSE_SUPPRESSION_NO_REASON_SENTINEL {
        None
    } else {
        Some(reason)
    }
}

/// Resolve a directive on the line immediately above `line_number`.
///
/// `line_number` is 1-indexed, mirroring `EnvEntry::line`. Returns
/// `(suppressed, reason)` — `(false, None)` when no matching directive
/// is found.
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

/// Resolve a directive in the first [`HEADER_LINE_BUDGET`] lines of a
/// file. Used by per-file structural rules.
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
    use super::{HEADER_LINE_BUDGET, resolve_file_header_suppression, resolve_line_suppression};

    #[test]
    fn line_suppression_matches_previous_line() {
        let lines = vec![
            "# @anvil-ignore SURFENV-003 -- staging replay",
            "DATABASE_URL=postgres://prod-db/app",
        ];
        let (suppressed, reason) = resolve_line_suppression(&lines, 2, "SURFENV-003");
        assert!(suppressed);
        assert_eq!(reason.as_deref(), Some("staging replay"));
    }

    #[test]
    fn line_suppression_rejects_other_rule_id() {
        let lines = vec![
            "# @anvil-ignore SURFENV-001 -- different rule",
            "DATABASE_URL=postgres://prod-db/app",
        ];
        let (suppressed, _) = resolve_line_suppression(&lines, 2, "SURFENV-003");
        assert!(!suppressed);
    }

    #[test]
    fn line_suppression_at_first_line_returns_false() {
        let lines = vec!["DATABASE_URL=postgres://prod-db/app"];
        let (suppressed, reason) = resolve_line_suppression(&lines, 1, "SURFENV-003");
        assert!(!suppressed);
        assert!(reason.is_none());
    }

    #[test]
    fn header_suppression_finds_directive_in_budget() {
        let content = "# top comment\n\
# @anvil-ignore SURFENV-002 -- intentional commit\n\
FOO=bar\n";
        let (suppressed, reason) = resolve_file_header_suppression(content, "SURFENV-002");
        assert!(suppressed);
        assert_eq!(reason.as_deref(), Some("intentional commit"));
    }

    #[test]
    fn line_directive_without_reason_returns_none() {
        // Council finding: a directive written without `-- <reason>`
        // must surface as `None`, not as `Some("No reason provided")`.
        let lines = vec![
            "# @anvil-ignore SURFENV-003",
            "DATABASE_URL=postgres://prod-db/app",
        ];
        let (suppressed, reason) = resolve_line_suppression(&lines, 2, "SURFENV-003");
        assert!(suppressed);
        assert!(
            reason.is_none(),
            "expected None, got Some({reason:?})"
        );
    }

    #[test]
    fn header_directive_without_reason_returns_none() {
        let content = "# @anvil-ignore SURFENV-002\nFOO=bar\n";
        let (suppressed, reason) = resolve_file_header_suppression(content, "SURFENV-002");
        assert!(suppressed);
        assert!(reason.is_none());
    }

    #[test]
    fn header_suppression_skips_directive_outside_budget() {
        use std::fmt::Write as _;
        let mut prelude = String::new();
        for i in 0..HEADER_LINE_BUDGET + 2 {
            writeln!(prelude, "# line {i}").expect("writeln to String never fails");
        }
        let content = format!("{prelude}# @anvil-ignore SURFENV-002 -- buried\nFOO=bar\n");
        let (suppressed, _) = resolve_file_header_suppression(&content, "SURFENV-002");
        assert!(!suppressed);
    }
}
