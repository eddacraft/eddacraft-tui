//! AI-001 — appeal-to-authority reasoning rule.
//!
//! Flags source comments that justify code with an appeal to authority,
//! social proof, or deflection rather than technical reasoning. Examples
//! include "the lead said to do it this way", "as discussed with the
//! architect", "we've always done it this way", and "trust me, just use
//! it". These signals correlate with code that has skipped review and
//! tend to reappear in AI-generated changes that mirror chat-room prose.
//!
//! ## Scope
//!
//! - Only **comment regions** match. String content with the same prose
//!   does not match — the rule treats narrative, not data, as the signal.
//! - Comment families recognised: `//`, `/* … */`, `#`, `<!-- … -->`.
//! - One emission per matching line (multiple matches collapse) so the
//!   output stays readable when several phrases co-occur.
//!
//! ## False positives vs false negatives
//!
//! Reasoning rules ship as `Severity::Info` and the heuristics are kept
//! deliberately broad: false positives are acceptable, false negatives
//! are the failure mode. AI-002 onward will tighten precision once we
//! have telemetry on the AI-001 trigger rate.
//!
//! ## Suppression
//!
//! Honours `@anvil-ignore AI-001` on the line above per ADR-029. The
//! parser is shared with the anti-pattern scanner via
//! [`crate::antipattern::parse_suppression`].

use std::sync::LazyLock;

use regex::Regex;

use anvil_kernel_types::diagnostics::KnownMode;
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};

use crate::antipattern::parse_suppression;

/// Stable rule identifier used in diagnostics, suppression directives, and
/// the registration table.
pub const RULE_ID: &str = "AI-001";

/// Source-module string emitted on every diagnostic so consumers can route
/// or filter on producer.
pub const SOURCE_MODULE: &str = "anvil-checks::reasoning";

/// Heuristic phrase patterns. Each pattern is matched against the comment
/// content (stripped of its leading marker) using
/// `regex::RegexSet::is_match`-style semantics — a single hit on any
/// pattern fires the rule for the line.
///
/// Patterns are intentionally narrow at launch:
///
/// - role-as-source: `<senior|lead|architect|principal|cto|vp> (said|told|wants|insists|approved)`
/// - "as discussed with …": shifts the justification onto another party
/// - boss/manager/stakeholder/product wants/asked/requires
/// - "trust me" / "just use it" / "just do it"
/// - "we've always done it this way" / "we've done this for years"
/// - "don't worry about <test|edge case|safety|security|the typing>"
///
/// Whole patterns are wrapped in `(?i)` for case-insensitive matching;
/// `\b` boundaries keep matches anchored to whole words.
const APPEAL_TO_AUTHORITY_PATTERNS: &[&str] = &[
    // role-as-source
    r"(?i)\b(senior|lead|architect|principal|cto|vp|tech\s+lead|staff\s+engineer)\b[^.\n]{0,80}\b(said|told|wants?|insists?|approved|asked|requested|signed\s*off)\b",
    // "as discussed with …"
    r"(?i)\bas\s+(discussed|agreed|decided)\s+(with|by|in)\b",
    // "boss/manager/stakeholder/product wants/asked/requires"
    r"(?i)\b(boss|manager|stakeholder|product(\s+owner)?|business)\b[^.\n]{0,40}\b(asked|wants?|requires?|needs?|insists?)\b",
    // "trust me" / "just <verb> it"
    r"(?i)\btrust\s+me\b",
    r"(?i)\bjust\s+(do|use|ship|push|merge|commit)\s+it\b",
    // "we've always done it this way"
    r"(?i)\bwe('?ve|\s+have)?\s+(always|been)\b[^.\n]{0,40}\b(done|doing)\b[^.\n]{0,40}\bthis\s+way\b",
    r"(?i)\bwe('?ve|\s+have)?\s+done\s+(it|this)\s+(this\s+way|like\s+this)\s+for\s+(years|ages|ever)",
    // "don't worry about …"
    r"(?i)\bdon'?t\s+worry\s+about\b[^.\n]{0,80}\b(test(s|ing)?|edge\s+case[s]?|safety|security|typing|types|null[s]?)\b",
];

static COMPILED_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    APPEAL_TO_AUTHORITY_PATTERNS
        .iter()
        .map(|src| Regex::new(src).expect("static AI-001 regex must compile"))
        .collect()
});

/// Regex used to extract comment regions from a single line.
///
/// Captures (in order):
/// 1. `//` line comment trailing text
/// 2. `/* … */` block-comment body (single-line block comments only — true
///    multi-line block comments are tracked stateully by the line scanner)
/// 3. `#` line comment trailing text (excludes `#!` shebang, `#include`,
///    `#define`, `#if`, etc — handled by leading-only `#` plus a space or
///    end-of-line)
/// 4. `<!-- … -->` HTML / Markdown comment body
static COMMENT_LINE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?P<slash>//[^\n]*)|(?P<block>/\*[^\n]*?\*/)|(?P<hash>(?:^|\s)#(?:\s[^\n]*|$))|(?P<html><!--[^\n]*?-->)",
    )
    .expect("static comment-region regex must compile")
});

/// Per-line scan state — block comments span lines, so the entry point
/// threads this state across the source.
#[derive(Debug, Default, Clone, Copy)]
struct ScanState {
    /// True when the current line begins inside an unterminated `/* … */`
    /// block comment. Reset to false when `*/` is seen on the same line.
    in_block_comment: bool,
}

/// Run the AI-001 rule against a single source artefact.
///
/// `file` is the path that will be surfaced on each [`Diagnostic.location`]
/// and is also the value the user references in their `@anvil-ignore AI-001`
/// suppression directive. `content` is the raw file contents.
#[must_use]
pub fn scan_file(file: &str, content: &str) -> Vec<Diagnostic> {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut state = ScanState::default();
    let mut findings = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let line_number = index + 1;
        let comment_text = collect_comment_text(line, &mut state);
        if comment_text.is_empty() {
            continue;
        }
        if !any_pattern_matches(&comment_text) {
            continue;
        }

        // Suppression check — directive lives on the previous line, parsed
        // through the canonical ADR-029 entry point.
        if line_number > 1
            && let Some((id, _reason)) = parse_suppression(lines[line_number - 2])
            && id == RULE_ID
        {
            continue;
        }

        findings.push(make_diagnostic(file, line_number));
    }

    findings
}

fn make_diagnostic(file: &str, line_number: usize) -> Diagnostic {
    // The wire schema uses u32 for line/column; a 1-based line that
    // overflows u32 means the file is too large to be source code, so
    // saturate rather than panic.
    let line_u32 = u32::try_from(line_number).unwrap_or(u32::MAX);

    Diagnostic::new(
        RULE_ID,
        Severity::Info,
        "Comment appeals to authority instead of giving a technical reason",
        Location {
            file: file.to_string(),
            line: Some(line_u32),
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Reasoning,
        DiagnosticSource {
            rule_id: RULE_ID.to_string(),
            source_module: SOURCE_MODULE.to_string(),
        },
        Mode::known(KnownMode::SaveTime),
    )
    .with_remediation_hint(
        "Replace the appeal with the technical reason — what constraint does this code satisfy, and what breaks if you change it?",
    )
}

/// Pull the comment-region text out of `line`, advancing `state` for any
/// block-comment opener / closer that crosses the line boundary. Returns
/// an empty string when the line carries no comment content.
fn collect_comment_text(line: &str, state: &mut ScanState) -> String {
    let mut buf = String::new();

    if state.in_block_comment {
        // We are inside a `/* … */` that started on a previous line. Take
        // everything up to the closer (or the entire line if the closer
        // isn't here) as comment text.
        if let Some(end) = line.find("*/") {
            buf.push_str(&line[..end]);
            state.in_block_comment = false;
            // The remainder of the line may itself contain a fresh
            // single-line comment — fall through to scan it.
            let after = &line[end + 2..];
            buf.push(' ');
            buf.push_str(&collect_single_line_comments(after, state));
        } else {
            buf.push_str(line);
        }
        return buf;
    }

    // Line begins outside a block comment — scan for any single-line
    // comment markers, plus a `/*` that opens a new block.
    buf.push_str(&collect_single_line_comments(line, state));
    buf
}

fn collect_single_line_comments(line: &str, state: &mut ScanState) -> String {
    let mut buf = String::new();

    for capture in COMMENT_LINE_REGEX.captures_iter(line) {
        if let Some(matched) = capture.name("slash") {
            buf.push(' ');
            buf.push_str(&matched.as_str()[2..]);
        } else if let Some(matched) = capture.name("block") {
            // Single-line `/* … */` — strip the markers.
            let inner = matched.as_str();
            let stripped = inner
                .strip_prefix("/*")
                .and_then(|s| s.strip_suffix("*/"))
                .unwrap_or(inner);
            buf.push(' ');
            buf.push_str(stripped);
        } else if let Some(matched) = capture.name("hash") {
            // Trim the leading whitespace + `#` marker; keep the prose.
            let raw = matched.as_str().trim_start();
            let stripped = raw.strip_prefix('#').unwrap_or(raw);
            buf.push(' ');
            buf.push_str(stripped.trim_start());
        } else if let Some(matched) = capture.name("html") {
            let inner = matched.as_str();
            let stripped = inner
                .strip_prefix("<!--")
                .and_then(|s| s.strip_suffix("-->"))
                .unwrap_or(inner);
            buf.push(' ');
            buf.push_str(stripped);
        }
    }

    // Detect an unterminated `/*` that opens a multi-line block comment.
    // We only care about openers that appear *outside* a string. The
    // narrow heuristic below skips strings by checking whether the opener
    // sits inside a balanced pair of double / single quotes on the same
    // line — good enough for AI-001's launch precision target.
    if let Some(open_pos) = find_unbalanced_block_open(line) {
        // Take everything after the opener as comment text — the closer
        // (if any) lives on a future line.
        let tail = &line[open_pos + 2..];
        buf.push(' ');
        buf.push_str(tail);
        state.in_block_comment = true;
    }

    buf
}

/// Find the byte index of a `/*` opener that is not paired with a `*/`
/// closer on the same line and is not enclosed in matching quotes.
/// Returns `None` when no such opener exists.
fn find_unbalanced_block_open(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut idx = 0;
    let mut in_double = false;
    let mut in_single = false;

    while idx + 1 < bytes.len() {
        let byte = bytes[idx];
        let next = bytes[idx + 1];
        match byte {
            b'\\' if in_double || in_single => {
                idx += 2;
                continue;
            }
            b'"' if !in_single => in_double = !in_double,
            b'\'' if !in_double => in_single = !in_single,
            b'/' if next == b'*' && !in_double && !in_single => {
                // Found an opener. Is there a matching closer further on?
                if line[idx + 2..].contains("*/") {
                    // Balanced single-line block comment — already handled.
                    idx += 2;
                    continue;
                }
                return Some(idx);
            }
            _ => {}
        }
        idx += 1;
    }

    None
}

fn any_pattern_matches(text: &str) -> bool {
    COMPILED_PATTERNS.iter().any(|regex| regex.is_match(text))
}

#[cfg(test)]
mod tests {
    use super::{RULE_ID, SOURCE_MODULE, scan_file};
    use anvil_kernel_types::{Category, Severity};

    fn finding_lines(file: &str, content: &str) -> Vec<u32> {
        scan_file(file, content)
            .into_iter()
            .filter_map(|d| d.location.line)
            .collect()
    }

    // ---- Positive cases: each pattern class fires --------------------------

    #[test]
    fn flags_role_as_source_in_slash_comment() {
        let content = "// the lead said to ignore the null check here\nlet x = value;";
        assert_eq!(finding_lines("src/a.ts", content), vec![1]);
    }

    #[test]
    fn flags_as_discussed_with_in_block_comment() {
        let content = "/* keeping this method as discussed with the architect */\nfn a() {}";
        assert_eq!(finding_lines("src/a.rs", content), vec![1]);
    }

    #[test]
    fn flags_boss_wants_in_hash_comment() {
        let content = "# the product owner wants this disabled in prod\nset -e\n";
        assert_eq!(finding_lines("scripts/release.sh", content), vec![1]);
    }

    #[test]
    fn flags_trust_me_in_html_comment() {
        let content = "<!-- trust me, this script tag has to load synchronously -->\n<body></body>";
        assert_eq!(finding_lines("docs/index.html", content), vec![1]);
    }

    #[test]
    fn flags_just_do_it_in_slash_comment() {
        let content = "// just ship it, we'll fix it after launch\nfn release() {}";
        assert_eq!(finding_lines("src/a.rs", content), vec![1]);
    }

    #[test]
    fn flags_weve_always_done_it_this_way() {
        let content = "// we've always done it this way and changing it now is risky\n";
        assert_eq!(finding_lines("src/a.ts", content), vec![1]);
    }

    #[test]
    fn flags_dont_worry_about_safety() {
        let content = "// don't worry about edge cases here, the caller validates\n";
        assert_eq!(finding_lines("src/a.ts", content), vec![1]);
    }

    // ---- Negative cases: string content with the same prose does NOT match --

    #[test]
    fn does_not_flag_string_literal_with_same_phrasing() {
        // The prose lives inside a normal string — not a comment. Must not match.
        let content = "let m = \"the lead said to ignore the null check\";\n";
        assert!(finding_lines("src/a.rs", content).is_empty());
    }

    #[test]
    fn does_not_flag_javascript_template_literal_with_same_phrasing() {
        let content = "const m = `as discussed with the architect`;\n";
        // Backticks aren't tracked by the quote-balance heuristic, but the
        // surrounding `const` line has no `//` / `/*` / `#` / `<!--` marker,
        // so no comment region is collected and AI-001 must not fire.
        assert!(finding_lines("src/a.ts", content).is_empty());
    }

    #[test]
    fn does_not_flag_code_without_comment_marker() {
        let content = "fn trust_me() { just_do_it(); }\n";
        assert!(finding_lines("src/a.rs", content).is_empty());
    }

    #[test]
    fn does_not_flag_unrelated_comment() {
        let content = "// returns the cached entry when present\nfn get() {}\n";
        assert!(finding_lines("src/a.rs", content).is_empty());
    }

    // ---- Suppression directive --------------------------------------------

    #[test]
    fn suppression_directive_on_previous_line_silences_finding() {
        let content = "// @anvil-ignore AI-001 -- migrating from a legacy reviewer convention\n// the lead said to leave this branch in for now\nfn a() {}\n";
        assert!(finding_lines("src/a.rs", content).is_empty());
    }

    #[test]
    fn suppression_directive_for_other_rule_does_not_silence() {
        let content = "// @anvil-ignore AP-003\n// the lead said to leave this branch in for now\n";
        assert_eq!(finding_lines("src/a.rs", content), vec![2]);
    }

    // ---- Comment-region detection across families ------------------------

    #[test]
    fn slash_block_and_hash_comments_all_scan() {
        let content = "// the lead said skip auth here\n# the manager wants this disabled\n/* as discussed with the principal engineer */\n";
        let lines = finding_lines("src/a.sh", content);
        assert_eq!(lines, vec![1, 2, 3]);
    }

    #[test]
    fn html_comment_spanning_a_single_line_matches() {
        let content = "<!-- the architect approved this path; trust me -->\n";
        assert_eq!(finding_lines("docs/a.md", content), vec![1]);
    }

    #[test]
    fn multi_line_block_comment_matches_in_body() {
        let content = "/*\n * we've always done it this way\n */\nfn a() {}\n";
        assert_eq!(finding_lines("src/a.rs", content), vec![2]);
    }

    // ---- Diagnostic shape -------------------------------------------------

    #[test]
    fn diagnostic_carries_canonical_metadata() {
        let content = "// the architect said this is fine\n";
        let findings = scan_file("src/a.rs", content);
        assert_eq!(findings.len(), 1);
        let diag = &findings[0];
        assert_eq!(diag.id, RULE_ID);
        assert_eq!(diag.category, Category::Reasoning);
        assert_eq!(diag.severity, Severity::Info);
        assert_eq!(diag.source.rule_id, RULE_ID);
        assert_eq!(diag.source.source_module, SOURCE_MODULE);
        assert_eq!(diag.schema_version, "anvil.diagnostic.v1");
        assert_eq!(diag.location.file, "src/a.rs");
        assert!(diag.remediation_hint.is_some());
    }

    #[test]
    fn shebang_and_directives_do_not_match_hash_comment_class() {
        // `#!/usr/bin/env bash` is a shebang, not a hash comment. The
        // hash-comment regex requires a space (or EOL) after `#`, so this
        // does not enter the comment buffer and AI-001 cannot fire.
        let content = "#!/usr/bin/env bash\n#include <stdio.h>\n";
        assert!(finding_lines("src/a.sh", content).is_empty());
    }
}
