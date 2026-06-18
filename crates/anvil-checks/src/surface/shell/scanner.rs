//! SURFSH-001 (file detection) + SURFSH-002 (dangerous-command scan) for the
//! shell-script governance surface (T1 — Scanned).
//!
//! Per `plans/modules/surface-shell.aps.md`, this surface **reuses** the
//! existing `command_safety` rule engine rather than duplicating its
//! catalogue: each command in a checked-in shell script is parsed and
//! analysed against the shared `default_filesystem_rules()` (the `rm -rf /`
//! family etc.). One catalogue, two consumers (the runtime command-safety
//! check and this static surface).
//!
//! Suppressions reuse the canonical Rust antipattern parser per
//! [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md);
//! the `#` comment style is already supported.
//!
//! Phase-1 scope: detection is extension-based (`*.sh`/`*.bash`) — shebang-only
//! scripts (no extension) are a documented follow-up. Shell-specific rules not
//! in the shared catalogue (`chmod 777`, `curl … | sh`) are deferred to a
//! follow-up that extends `command_safety`'s rules, so both consumers benefit.
//!
//! Known limitations (warn-only surface): subshell/group wrappers
//! (`(rm -rf /)`, `{ …; }`) aren't decomposed by the shared parser, so a
//! command wrapped that way is missed; and a dangerous command on line 1
//! (no preceding line) can't carry an `# @anvil-ignore` directive. Heredoc
//! bodies are correctly skipped (treated as data, not commands).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::suppression::resolve_line_suppression;
use crate::command_safety::matcher::analyse_command;
use crate::command_safety::parser::parse_compound_command;
use crate::command_safety::rules::default_filesystem_rules;
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
    /// Why the shared catalogue flagged it.
    pub reason: String,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Scan one shell script's `content` for dangerous commands (SURFSH-002),
/// reusing the shared `command_safety` filesystem ruleset.
#[must_use]
pub fn scan_shell(display_path: &str, content: &str) -> Vec<ShellFinding> {
    scan_shell_with_rules(display_path, content, &default_filesystem_rules())
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

    for instr in logical_lines(&lines) {
        // A line may be a compound command (`a && b | c`); analyse each part.
        for parsed in parse_compound_command(&instr.text).commands {
            let analysis = analyse_command(&parsed.raw, &parsed, rules, None);
            if matches!(analysis.action, CommandAction::Block | CommandAction::Warn) {
                let (suppressed, reason) =
                    resolve_line_suppression(&lines, instr.line, SURFSH_002_RULE_ID);
                findings.push(ShellFinding {
                    file: display_path.to_string(),
                    line: instr.line,
                    command: truncate(parsed.raw.trim()),
                    severity: analysis.severity.into(),
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

fn logical_lines(lines: &[&str]) -> Vec<LogicalLine> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut start: Option<usize> = None;
    // Open heredoc: (closing marker, strip-leading-tabs for `<<-`). Body lines
    // are script *data*, not commands, so a heredoc that documents a dangerous
    // command must not be scanned (would be an unsuppressible false positive).
    let mut heredoc: Option<(String, bool)> = None;

    for (idx, raw) in lines.iter().enumerate() {
        let line_number = idx + 1;

        if let Some((marker, strip_tabs)) = &heredoc {
            let candidate = if *strip_tabs {
                raw.trim_start_matches('\t').trim_end()
            } else {
                raw.trim_end()
            };
            if candidate == marker {
                heredoc = None;
            }
            continue; // skip heredoc body + closing marker line
        }

        let trimmed = raw.trim();
        // Only break on a comment when not mid-continuation; a `#` opening a
        // fresh line is a comment (shell comments run to end of line).
        if start.is_none() && (trimmed.is_empty() || trimmed.starts_with('#')) {
            continue;
        }
        if start.is_none() {
            start = Some(line_number);
        }
        // A line continues only on an ODD number of trailing backslashes; an
        // even count (e.g. `\\`) is an escaped backslash, not a continuation.
        let is_cont = ends_with_continuation(trimmed);
        let body = if is_cont {
            trimmed.strip_suffix('\\').unwrap_or(trimmed).trim_end()
        } else {
            trimmed
        };
        if !buf.is_empty() {
            buf.push(' ');
        }
        buf.push_str(body);
        if !is_cont {
            if !buf.trim().is_empty() {
                out.push(LogicalLine {
                    text: buf.clone(),
                    line: start.expect("start set"),
                });
            }
            // A heredoc opened on this logical line suppresses its body lines.
            heredoc = heredoc_opener(&buf);
            buf.clear();
            start = None;
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

/// If `instruction` opens a heredoc (`<< MARKER`, `<<-MARKER`, `<< 'MARKER'`),
/// return the closing marker and whether `<<-` tab-stripping applies. A
/// here-string (`<<<`) has no body and is ignored.
fn heredoc_opener(instruction: &str) -> Option<(String, bool)> {
    let bytes = instruction.as_bytes();
    let mut i = 0;
    while let Some(pos) = instruction[i..].find("<<") {
        let at = i + pos;
        // Skip here-strings (`<<<`).
        if bytes.get(at + 2) == Some(&b'<') {
            i = at + 3;
            continue;
        }
        let mut rest = &instruction[at + 2..];
        let strip_tabs = rest.starts_with('-');
        if strip_tabs {
            rest = &rest[1..];
        }
        let rest = rest.trim_start();
        let quoted = rest.starts_with('"') || rest.starts_with('\'');
        // The marker is the next token, with surrounding quotes stripped.
        let marker: String = rest
            .split(|c: char| c.is_whitespace() || c == ';' || c == '|' || c == '&')
            .next()
            .unwrap_or("")
            .trim_matches(['"', '\''])
            .to_string();
        // A heredoc marker is identifier-like (or quoted); a `<<` followed by
        // an expression is an arithmetic left-shift (`$((x<<2))`), not a
        // heredoc — must not flip the scanner into body-skipping mode.
        let marker_like = quoted
            || marker
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
        if marker.is_empty() || !marker_like {
            i = at + 2;
            continue;
        }
        return Some((marker, strip_tabs));
    }
    None
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
}
