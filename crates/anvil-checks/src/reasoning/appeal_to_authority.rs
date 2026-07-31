//! AI-001: appeal-to-authority reasoning rule.

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
/// - role-as-source: `<senior|lead|architect|principal engineer|cto|vp> (said|told|wants|insists|approved)`
/// - "as discussed with …": shifts the justification onto another party
/// - boss/manager/stakeholder/product wants/asked/requires
/// - "trust me" / "just use it" / "just do it"
/// - "we've always done it this way" / "we've done this for years"
/// - "don't worry about <test|edge case|safety|security|the typing>"
///
/// Whole patterns are wrapped in `(?i)` for case-insensitive matching;
/// `\b` boundaries keep matches anchored to whole words.
///
/// `principal` is only matched when followed by `engineer`/`architect` —
/// the bare adjective ("the principal reason …") is a common technical
/// phrase, not an authority appeal. `as discussed in …` is excluded so
/// references like `as decided in ADR-029` don't fire.
const APPEAL_TO_AUTHORITY_PATTERNS: &[&str] = &[
    // role-as-source — `principal` requires an engineer/architect suffix
    // to avoid the "the principal reason" adjective form.
    r"(?i)\b(senior|lead|architect|principal\s+(?:engineer|architect)|cto|vp|tech\s+lead|staff\s+engineer)\b[^.\n]{0,80}\b(said|told|wants?|insists?|approved|asked|requested|signed\s*off)\b",
    // "as discussed with …" / "as agreed with …" — only `with` qualifies
    // to keep ADR/RFC references like "as decided in ADR-029" clean.
    r"(?i)\bas\s+(discussed|agreed|decided)\s+with\b",
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
    scan_file_with_limit(file, content, usize::MAX)
}

/// Run the AI-001 rule and return at most `limit` diagnostics.
#[must_use]
pub fn scan_file_with_limit(file: &str, content: &str, limit: usize) -> Vec<Diagnostic> {
    if limit == 0 {
        return Vec::new();
    }
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
        if findings.len() == limit {
            return findings;
        }
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

/// Walk `line` left-to-right, skipping string literals and emitting any
/// comment-region text into the returned buffer. Recognises `//`, `/* */`,
/// `#`, and `<!-- -->` openers but only when they appear outside `"…"`,
/// `'…'`, or `` `…` `` strings (with `\` escapes honoured). When an
/// unterminated `/*` is hit, sets `state.in_block_comment` so the caller
/// continues consuming the next line as comment text.
///
/// The string-aware scan is what guarantees the documented invariant:
/// `// …`, `/* … */`, and `<!-- … -->` content inside a string literal
/// (e.g. a URL like `"https://…"`) is data, not narrative, and must not
/// match. It also stops `// description /*` from accidentally opening a
/// multi-line block comment, which would corrupt scan state on later
/// lines.
fn collect_single_line_comments(line: &str, state: &mut ScanState) -> String {
    let mut buf = String::new();
    let bytes = line.as_bytes();
    let mut idx = 0;
    let mut in_double = false;
    let mut in_single = false;
    let mut in_backtick = false;

    while idx < bytes.len() {
        let byte = bytes[idx];
        let next = bytes.get(idx + 1).copied().unwrap_or(0);

        // Inside any string literal, only watch for the matching closer
        // and honour `\` escapes. Comment markers inside strings are data.
        if in_double || in_single || in_backtick {
            if byte == b'\\' && idx + 1 < bytes.len() {
                idx += 2;
                continue;
            }
            if (in_double && byte == b'"')
                || (in_single && byte == b'\'')
                || (in_backtick && byte == b'`')
            {
                in_double = false;
                in_single = false;
                in_backtick = false;
            }
            idx += 1;
            continue;
        }

        // Outside strings: detect string openers and comment markers.
        match (byte, next) {
            (b'"', _) => {
                in_double = true;
                idx += 1;
            }
            (b'\'', _) => {
                in_single = true;
                idx += 1;
            }
            (b'`', _) => {
                in_backtick = true;
                idx += 1;
            }
            // `//` line comment — consumes the rest of the line.
            (b'/', b'/') => {
                buf.push(' ');
                buf.push_str(&line[idx + 2..]);
                return buf;
            }
            // `/* … */` block. If terminated on this line, strip markers
            // and continue scanning. Otherwise, take the rest as comment
            // text and flag the multi-line block opener.
            (b'/', b'*') => {
                let body_start = idx + 2;
                if let Some(rel_close) = line[body_start..].find("*/") {
                    let body_end = body_start + rel_close;
                    buf.push(' ');
                    buf.push_str(&line[body_start..body_end]);
                    idx = body_end + 2;
                    continue;
                }
                buf.push(' ');
                buf.push_str(&line[body_start..]);
                state.in_block_comment = true;
                return buf;
            }
            // `#` line comment — only when preceded by whitespace or BOL,
            // and followed by a space or EOL. This filters out `#!`
            // shebangs, `#include`, `#define`, `#if`, etc.
            (b'#', _) => {
                let leading_ok = idx == 0 || bytes[idx - 1].is_ascii_whitespace();
                let next_is_space = next == b' ' || next == b'\t';
                let next_is_eol = idx + 1 == bytes.len();
                if leading_ok && (next_is_space || next_is_eol) {
                    buf.push(' ');
                    buf.push_str(line[idx + 1..].trim_start());
                    return buf;
                }
                idx += 1;
            }
            // `<!-- … -->` HTML / Markdown comment.
            (b'<', _) if line[idx..].starts_with("<!--") => {
                let body_start = idx + 4;
                if let Some(rel_close) = line[body_start..].find("-->") {
                    let body_end = body_start + rel_close;
                    buf.push(' ');
                    buf.push_str(&line[body_start..body_end]);
                    idx = body_end + 3;
                    continue;
                }
                // Unterminated single-line HTML comment — take the rest
                // as comment text. Multi-line `<!-- … -->` tracking is
                // not currently threaded across lines (documented in the
                // module-level Scope section).
                buf.push(' ');
                buf.push_str(&line[body_start..]);
                return buf;
            }
            _ => {
                idx += 1;
            }
        }
    }

    buf
}

fn any_pattern_matches(text: &str) -> bool {
    COMPILED_PATTERNS.iter().any(|regex| regex.is_match(text))
}

#[cfg(test)]
mod tests {
    use super::{RULE_ID, SOURCE_MODULE, scan_file, scan_file_with_limit};
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
    fn scan_file_with_limit_stops_after_requested_findings() {
        let content = "// the lead said skip auth here\n# the manager wants this disabled\n";
        let findings = scan_file_with_limit("src/a.sh", content, 1);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].location.line, Some(1));
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

    // ---- String-aware comment scanning -----------------------------------

    #[test]
    fn does_not_flag_double_slash_inside_url_in_string() {
        // The `//` lives inside a string literal — must not be parsed as a
        // line comment, so the trigger phrase inside the URL stays data.
        let content = "let url = \"https://example.com/the-lead-said\";\n";
        assert!(finding_lines("src/a.ts", content).is_empty());
    }

    #[test]
    fn does_not_flag_block_marker_inside_string() {
        // `/* … */` inside a string literal must stay data, not be parsed
        // as a single-line block comment.
        let content = "let glob = \"/* the lead said use this glob */\";\nfn ok() {}\n";
        assert!(finding_lines("src/a.rs", content).is_empty());
    }

    #[test]
    fn block_open_after_line_comment_does_not_corrupt_state() {
        // `// description /* not a real opener` must NOT flip the scanner
        // into multi-line block-comment mode. Otherwise the next line of
        // real code gets eaten as comment text.
        let content = "// description /* not a real opener\nlet x = unrelated_call_that_must_not_be_swallowed();\n";
        let findings = scan_file("src/a.rs", content);
        // No appeal-to-authority phrase on either line — must produce no
        // findings. Critically, the second line must NOT be scanned as
        // comment content.
        assert!(findings.is_empty());
    }

    // ---- Pattern tightening: false-positive guards -----------------------

    #[test]
    fn does_not_flag_adr_reference_with_in() {
        // `as decided in <artefact>` is a legitimate technical reference,
        // not an appeal to authority. Only `with` qualifies.
        let content = "// as decided in ADR-029, this path is reserved for AI-001\nfn a() {}\n";
        assert!(finding_lines("src/a.rs", content).is_empty());
    }

    #[test]
    fn does_not_flag_principal_as_adjective() {
        // `principal` is a common adjective in technical prose and must
        // not fire on its own — only `principal engineer` / `principal
        // architect` qualify.
        let content = "// the principal reason this interface exists is to decouple X\nfn a() {}\n";
        assert!(finding_lines("src/a.rs", content).is_empty());
    }

    #[test]
    fn flags_principal_engineer_compound_form() {
        // The compound form is a real authority appeal and must still
        // fire after the tightening.
        let content = "// the principal engineer said skip the type narrowing\nfn a() {}\n";
        assert_eq!(finding_lines("src/a.rs", content), vec![1]);
    }
}
