//! `@anvil-ignore` resolution for SURFDOCK rules.
//!
//! Per [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md)
//! every Track 3 surface reuses the canonical
//! [`crate::antipattern::parse_suppression`] entry point. Dockerfiles use the
//! `#` comment style, already in that parser's grammar:
//!
//! ```dockerfile
//! # @anvil-ignore SURFDOCK-002 -- vendored installer, checksum verified below
//! RUN curl -fsSL https://get.example.com | sh
//! ```
//!
//! Mirrors the surface-agnostic SURFSQL/SURFENV/SURFGHA helpers.

use crate::antipattern::parse_suppression;

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
    fn comment_directive_suppresses_following_instruction() {
        let lines = vec![
            "# @anvil-ignore SURFDOCK-002 -- checksum verified",
            "ADD https://example.com/x /tmp/",
        ];
        let (suppressed, reason) = resolve_line_suppression(&lines, 2, "SURFDOCK-002");
        assert!(suppressed);
        assert_eq!(reason.as_deref(), Some("checksum verified"));
    }

    #[test]
    fn directive_for_other_rule_does_not_suppress() {
        let lines = vec![
            "# @anvil-ignore SURFDOCK-003 -- different rule",
            "ADD https://example.com/x /tmp/",
        ];
        let (suppressed, _) = resolve_line_suppression(&lines, 2, "SURFDOCK-002");
        assert!(!suppressed);
    }
}
