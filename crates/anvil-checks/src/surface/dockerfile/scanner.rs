//! SURFDOCK-001 (file detection) + SURFDOCK-002 (build-hygiene / supply-chain
//! pattern catalogue) for the Dockerfile governance surface.
//!
//! Track 3 surfaces are **pattern-catalogue** work, not parser work
//! (`plans/specs/2026-04-08-language-and-coverage-design.md` §8.3 row 3): we
//! assemble logical instructions (joining `\`-continued lines) and match
//! keywords, not a full Dockerfile parse. Findings anchor to the line the
//! instruction starts on.
//!
//! Suppressions reuse the canonical Rust antipattern parser per
//! [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md);
//! the `#` comment style is already supported.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::suppression::resolve_line_suppression;

/// SURFDOCK-002 — build-hygiene / supply-chain risks in Dockerfiles.
pub const SURFDOCK_002_RULE_ID: &str = "SURFDOCK-002";

/// True when `path` is a Dockerfile: named `Dockerfile`/`Containerfile`, a
/// `*.Dockerfile`/`*.Containerfile` (e.g. `web.Dockerfile`), or a suffixed
/// `Dockerfile.<variant>`/`Containerfile.<variant>` (e.g. `Dockerfile.prod`,
/// `Dockerfile.dev` — a documented Docker `-f` convention).
#[must_use]
pub fn is_dockerfile(path: &Path) -> bool {
    // Suffixed build variant (`Dockerfile.prod`, `Dockerfile.dev`), excluding
    // documentation extensions so `Dockerfile.md` (a doc) is not scanned.
    const DOC_EXTS: &[&str] = &["md", "markdown", "txt", "rst", "adoc", "html"];
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    if lower == "dockerfile"
        || lower == "containerfile"
        || lower.ends_with(".dockerfile")
        || lower.ends_with(".containerfile")
    {
        return true;
    }
    for prefix in ["dockerfile.", "containerfile."] {
        if lower.starts_with(prefix) {
            // Exclude when the FINAL extension is a doc type, so multi-suffix
            // names like `Dockerfile.prod.md` are treated as docs, not builds.
            let final_ext = lower.rsplit('.').next().unwrap_or("");
            return !final_ext.is_empty() && !DOC_EXTS.contains(&final_ext);
        }
    }
    false
}

/// The kind of Dockerfile risk a [`DockerFinding`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerRisk {
    /// `ADD https://…` fetches a remote URL into the image without checksum
    /// verification (prefer `RUN curl … && verify`, or `COPY`).
    AddRemoteFetch,
    /// `RUN curl/wget … | sh` — pipe-to-shell installs run unverified code.
    PipeToShell,
    /// `FROM …:latest` — a mutable base tag is not reproducible.
    LatestBaseImage,
    /// `sudo` inside a container layer (the build already runs privileged).
    SudoInRun,
    /// `apt-get install` without `--no-install-recommends` (layer bloat +
    /// larger attack surface).
    AptMissingNoRecommends,
}

impl DockerRisk {
    /// Human-readable summary used in the finding message.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::AddRemoteFetch => "ADD of a remote URL skips checksum verification",
            Self::PipeToShell => "piping a download straight to a shell runs unverified code",
            Self::LatestBaseImage => {
                "FROM a :latest tag is not reproducible (pin a version/digest)"
            }
            Self::SudoInRun => "sudo in a container layer is unnecessary and risky",
            Self::AptMissingNoRecommends => {
                "apt-get install without --no-install-recommends bloats the image"
            }
        }
    }
}

/// A single SURFDOCK-002 finding, anchored to the instruction's start line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerFinding {
    pub file: String,
    /// 1-indexed line where the instruction starts.
    pub line: usize,
    pub risk: DockerRisk,
    /// The offending instruction, collapsed and truncated for display.
    pub instruction: String,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Scan one Dockerfile's `content` for build-hygiene risks (SURFDOCK-002).
#[must_use]
pub fn scan_dockerfile(display_path: &str, content: &str) -> Vec<DockerFinding> {
    let lines: Vec<&str> = content.lines().collect();
    let mut findings = Vec::new();

    for instr in logical_instructions(&lines) {
        for risk in classify(&instr.normalised) {
            let (suppressed, reason) =
                resolve_line_suppression(&lines, instr.line, SURFDOCK_002_RULE_ID);
            findings.push(DockerFinding {
                file: display_path.to_string(),
                line: instr.line,
                risk,
                instruction: truncate(&instr.normalised),
                suppressed,
                suppression_reason: reason,
            });
        }
    }
    findings
}

const INSTRUCTION_CAP: usize = 120;

fn truncate(s: &str) -> String {
    if s.len() <= INSTRUCTION_CAP {
        return s.to_string();
    }
    let mut end = INSTRUCTION_CAP;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// A logical Dockerfile instruction: its normalised (lowercased,
/// whitespace-collapsed) text and the 1-indexed line it starts on.
struct Instruction {
    normalised: String,
    line: usize,
}

/// Assemble logical instructions from raw lines: skip full-line `#` comments
/// (Dockerfile comments must start the line), join `\`-continued lines into one
/// instruction, and fold `BuildKit` heredoc bodies (`RUN <<EOF … EOF`) into the
/// opening instruction so rules see the script they run. The start line is the
/// first non-comment line.
fn logical_instructions(lines: &[&str]) -> Vec<Instruction> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut start: Option<usize> = None;
    let mut continuing = false;
    // When inside a heredoc, the closing delimiter we are scanning for.
    let mut heredoc: Option<String> = None;

    for (idx, raw) in lines.iter().enumerate() {
        let line_number = idx + 1;

        // Inside a heredoc body: absorb every line into the current instruction
        // until the closing delimiter (a line equal to the delimiter token),
        // which ends the instruction. Body lines are kept verbatim so rules
        // (apt-get / pipe-to-shell / sudo) see commands written there.
        if let Some(delim) = heredoc.as_deref() {
            let body = raw.trim();
            if body == delim {
                heredoc = None;
                flush(&mut out, &mut buf, &mut start);
            } else if !body.is_empty() {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(body);
            }
            continue;
        }

        let trimmed = raw.trim();
        // A full-line comment only breaks a non-continued instruction; inside
        // a continuation it is skipped (Dockerfile allows comment lines there).
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.is_empty() {
            if !continuing && start.is_some() {
                flush(&mut out, &mut buf, &mut start);
            }
            continue;
        }
        if start.is_none() {
            start = Some(line_number);
        }
        let body = trimmed.strip_suffix('\\').map_or(trimmed, str::trim_end);

        // A heredoc opener (`RUN <<EOF`, `<<-EOF`, `<<"EOF"`, `… <<EOT bash`)
        // suspends flushing until the closing delimiter. Keep only the
        // instruction prefix before the `<<` redirect so the folded body reads
        // as a clean `RUN <commands>` (the `<<EOT bash` noise would otherwise
        // break command-position rules like sudo).
        if let Some(delim) = heredoc_delimiter(body) {
            let prefix = body[..body.find("<<").unwrap_or(body.len())].trim_end();
            if !prefix.is_empty() {
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(prefix);
            }
            heredoc = Some(delim);
            continuing = false;
            continue;
        }

        if !buf.is_empty() {
            buf.push(' ');
        }
        buf.push_str(body);
        continuing = trimmed.ends_with('\\');
        if !continuing {
            flush(&mut out, &mut buf, &mut start);
        }
    }
    flush(&mut out, &mut buf, &mut start);
    out
}

/// Extract a heredoc delimiter from an instruction line if it opens one.
///
/// Recognises the `BuildKit` forms `<<WORD`, `<<-WORD` and a quoted `<<"WORD"` /
/// `<<'WORD'`. The delimiter must be an identifier (letter/underscore start)
/// so an arithmetic/shift `<<` (e.g. `$((1<<2))`) is not mistaken for a
/// heredoc. Anything after the delimiter on the opener (e.g. `<<EOT bash`) is
/// ignored — only the closing-delimiter word matters.
fn heredoc_delimiter(line: &str) -> Option<String> {
    let after = &line[line.find("<<")? + 2..];
    let after = after.strip_prefix('-').unwrap_or(after).trim_start();
    let token: String = after.chars().take_while(|c| !c.is_whitespace()).collect();
    let token = token.trim_matches(|c| c == '"' || c == '\'');
    let mut chars = token.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return None,
    }
    if chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Some(token.to_string())
    } else {
        None
    }
}

fn flush(out: &mut Vec<Instruction>, buf: &mut String, start: &mut Option<usize>) {
    let normalised: String = buf
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if let (false, Some(line)) = (normalised.is_empty(), *start) {
        out.push(Instruction { normalised, line });
    }
    buf.clear();
    *start = None;
}

/// Classify a normalised (lowercased) instruction into zero or more risks.
fn classify(norm: &str) -> Vec<DockerRisk> {
    let mut risks = Vec::new();
    if norm.starts_with("add ") && (norm.contains("http://") || norm.contains("https://")) {
        risks.push(DockerRisk::AddRemoteFetch);
    }
    if norm.starts_with("run ") {
        if (norm.contains("curl") || norm.contains("wget")) && pipes_to_shell(norm) {
            risks.push(DockerRisk::PipeToShell);
        }
        if runs_sudo_command(norm) {
            risks.push(DockerRisk::SudoInRun);
        }
        if norm.contains("apt-get install") && !norm.contains("--no-install-recommends") {
            risks.push(DockerRisk::AptMissingNoRecommends);
        }
    }
    if norm.starts_with("from ") && from_uses_latest(norm) {
        risks.push(DockerRisk::LatestBaseImage);
    }
    risks
}

/// True when a `RUN` layer actually *invokes* `sudo` as a command, rather
/// than merely mentioning the token (e.g. `apt install … sudo …`, where `sudo`
/// is a package name). We strip the `run ` keyword and check each
/// `&&`/`||`/`;`/`|`-separated command segment for a leading `sudo`, so
/// `RUN sudo apt-get update` is flagged but `RUN apt install -y sudo` is not.
fn runs_sudo_command(norm: &str) -> bool {
    let body = norm.strip_prefix("run ").unwrap_or(norm);
    body.split(['&', '|', ';'])
        .map(str::trim_start)
        .any(|seg| seg == "sudo" || seg.starts_with("sudo "))
}

/// True when a `from …` instruction pins an explicit `:latest` tag on its
/// image reference. The image is the first token after `from` that is not an
/// option flag (`--platform=…`, `--chown`, …), so `FROM --platform=… x:latest`
/// is handled. Implicit-latest (a bare image with no tag) is deferred —
/// distinguishing it from a build-stage reference needs stage-name tracking.
fn from_uses_latest(norm: &str) -> bool {
    norm.split_whitespace()
        .skip(1) // the `from` keyword
        .find(|tok| !tok.starts_with("--"))
        .is_some_and(|image| image.ends_with(":latest"))
}

/// True when a normalised `run` instruction pipes into a shell command —
/// `| sh`, `|bash`, `| /bin/sh`, `| ash` (Alpine), `| dash`, `| zsh`, with or
/// without a leading absolute path.
fn pipes_to_shell(norm: &str) -> bool {
    // Neutralise `||` (shell logical-OR) so it isn't mistaken for a pipe:
    // `curl … || sh` is a fallback, not a pipe-to-shell.
    let piped = norm.replace("||", "  ");
    piped.split('|').skip(1).any(|segment| {
        let after_pipe = segment.trim_start();
        // Drop a leading absolute path: `/bin/sh` -> `sh`.
        let cmd = after_pipe.rsplit('/').next().unwrap_or(after_pipe);
        let word = cmd.split_whitespace().next().unwrap_or("");
        matches!(word, "sh" | "bash" | "ash" | "dash" | "zsh")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn risks(content: &str) -> Vec<DockerRisk> {
        scan_dockerfile("Dockerfile", content)
            .into_iter()
            .map(|f| f.risk)
            .collect()
    }

    #[test]
    fn detects_dockerfile_names() {
        assert!(is_dockerfile(Path::new("Dockerfile")));
        assert!(is_dockerfile(Path::new("app/Dockerfile")));
        assert!(is_dockerfile(Path::new("Containerfile")));
        assert!(is_dockerfile(Path::new("web.Dockerfile")));
        assert!(is_dockerfile(Path::new("svc.dockerfile")));
        // Suffixed build variants are a real `-f` convention.
        assert!(is_dockerfile(Path::new("Dockerfile.prod")));
        assert!(is_dockerfile(Path::new("app/Dockerfile.dev")));
        // ...but doc files named like one are not build inputs — including
        // multi-suffix docs whose FINAL extension is a doc type.
        assert!(!is_dockerfile(Path::new("Dockerfile.md")));
        assert!(!is_dockerfile(Path::new("Dockerfile.txt")));
        assert!(!is_dockerfile(Path::new("Dockerfile.prod.md")));
        assert!(!is_dockerfile(Path::new("Containerfile.dev.txt")));
        assert!(!is_dockerfile(Path::new("compose.yml")));
        assert!(!is_dockerfile(Path::new("src/main.rs")));
    }

    #[test]
    fn flags_add_remote_fetch() {
        assert_eq!(
            risks("ADD https://example.com/x.tar.gz /tmp/\n"),
            vec![DockerRisk::AddRemoteFetch]
        );
        // COPY and local ADD are fine.
        assert!(risks("COPY . /app\n").is_empty());
        assert!(risks("ADD ./local.tar.gz /tmp/\n").is_empty());
    }

    #[test]
    fn flags_pipe_to_shell() {
        assert_eq!(
            risks("RUN curl -fsSL https://get.example.com | sh\n"),
            vec![DockerRisk::PipeToShell]
        );
        assert_eq!(
            risks("RUN wget -qO- https://x | bash\n"),
            vec![DockerRisk::PipeToShell]
        );
        // A pipe without a downloader, or a download without a pipe, is fine.
        assert!(risks("RUN cat file | sh\n").is_empty());
        assert!(risks("RUN curl -fsSL https://x -o /tmp/x\n").is_empty());
    }

    #[test]
    fn flags_latest_base_image_explicit_only() {
        assert_eq!(
            risks("FROM node:latest\n"),
            vec![DockerRisk::LatestBaseImage]
        );
        assert_eq!(
            risks("FROM node:latest AS build\n"),
            vec![DockerRisk::LatestBaseImage]
        );
        // Pinned tags/digests are fine.
        assert!(risks("FROM node:20.11-alpine\n").is_empty());
        assert!(risks("FROM node@sha256:abc\n").is_empty());
        // Implicit-latest (no tag) is deferred — not flagged, and a build-stage
        // reference must never flag.
        assert!(risks("FROM build\n").is_empty());
        // The image after a --platform flag is still inspected (council FN).
        assert_eq!(
            risks("FROM --platform=linux/amd64 node:latest\n"),
            vec![DockerRisk::LatestBaseImage]
        );
        assert!(risks("FROM --platform=$BUILDPLATFORM node:20-alpine\n").is_empty());
    }

    #[test]
    fn pipe_to_shell_covers_path_and_alpine_shells() {
        assert_eq!(
            risks("RUN curl -fsSL https://x | /bin/sh\n"),
            vec![DockerRisk::PipeToShell]
        );
        assert_eq!(
            risks("RUN wget -qO- https://x | ash\n"),
            vec![DockerRisk::PipeToShell]
        );
        // A pipe to a non-shell is fine.
        assert!(risks("RUN curl -fsSL https://x | tar xz\n").is_empty());
        // `||` (logical OR) is a fallback, not a pipe-to-shell (council FP).
        assert!(risks("RUN curl -fsSL https://x -o /tmp/x || sh /tmp/fallback\n").is_empty());
    }

    #[test]
    fn flags_sudo_and_apt_recommends() {
        assert_eq!(
            risks("RUN sudo apt-get update\n"),
            vec![DockerRisk::SudoInRun]
        );
        assert_eq!(
            risks("RUN apt-get install -y nginx\n"),
            vec![DockerRisk::AptMissingNoRecommends]
        );
        assert!(risks("RUN apt-get install -y --no-install-recommends nginx\n").is_empty());
    }

    #[test]
    fn sudo_as_a_package_name_is_not_flagged() {
        // SURFDOCK-006 external validation (nvm): `sudo` installed as an apt
        // package must NOT trip SudoInRun — the rule targets sudo *invocation*,
        // not the token appearing in a package list.
        assert!(
            !risks("RUN apt install -y vim sudo htop\n").contains(&DockerRisk::SudoInRun),
            "sudo as a package argument is not a sudo invocation"
        );
        // But sudo after a command separator IS an invocation.
        assert!(
            risks("RUN apt-get update && sudo make install\n").contains(&DockerRisk::SudoInRun),
            "sudo in command position is still flagged"
        );
    }

    #[test]
    fn handles_line_continuations() {
        // A multi-line RUN must be assembled before matching pipe-to-shell.
        let content = "RUN curl -fsSL https://get.example.com \\\n    | sh\n";
        assert_eq!(risks(content), vec![DockerRisk::PipeToShell]);
    }

    #[test]
    fn ignores_comment_lines_and_suppresses() {
        // A risk keyword in a comment line does not fire.
        assert!(risks("# ADD https://example.com/x\nCOPY . /app\n").is_empty());
        // Suppression on the line above the instruction.
        let content = "# @anvil-ignore SURFDOCK-002 -- vendored, checksum verified below\nADD https://example.com/x.tar.gz /tmp/\n";
        let findings = scan_dockerfile("Dockerfile", content);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].suppressed);
        assert_eq!(
            findings[0].suppression_reason.as_deref(),
            Some("vendored, checksum verified below")
        );
    }

    #[test]
    fn clean_dockerfile_has_no_findings() {
        let content =
            "FROM node:20.11-alpine AS build\nWORKDIR /app\nCOPY . .\nRUN npm ci\nUSER node\n";
        assert!(scan_dockerfile("Dockerfile", content).is_empty());
    }

    // ── SURFDOCK-007: BuildKit heredoc RUN bodies ───────────────────

    #[test]
    fn heredoc_run_body_is_scanned_for_apt_recommends() {
        // The awesome-compose false negative: apt-get inside `RUN <<EOF`.
        let content = "RUN <<EOF\napt-get update\napt-get install -y git\nEOF\n";
        assert_eq!(risks(content), vec![DockerRisk::AptMissingNoRecommends]);
    }

    #[test]
    fn heredoc_run_body_is_scanned_for_pipe_to_shell() {
        // A `<<EOT bash` opener with a pipe-to-shell install in the body.
        let content = "RUN <<EOT bash\n    set -ex\n    curl -sSf https://example.com/install.sh | bash\nEOT\n";
        assert_eq!(risks(content), vec![DockerRisk::PipeToShell]);
    }

    #[test]
    fn heredoc_dash_and_quoted_delimiters_close_correctly() {
        // `<<-EOF` (indent-stripping) and quoted `<<"EOF"` both close on EOF.
        assert_eq!(
            risks("RUN <<-EOF\n\tsudo apt-get update\nEOF\n"),
            vec![DockerRisk::SudoInRun]
        );
        assert_eq!(
            risks("RUN <<\"EOF\"\napt-get install -y nginx\nEOF\n"),
            vec![DockerRisk::AptMissingNoRecommends]
        );
    }

    #[test]
    fn instruction_after_heredoc_resumes_normally() {
        // Lines after the closing delimiter are ordinary instructions again.
        let content = "RUN <<EOF\necho hi\nEOF\nRUN apt-get install -y curl\n";
        assert_eq!(risks(content), vec![DockerRisk::AptMissingNoRecommends]);
    }

    #[test]
    fn guarded_heredoc_body_is_clean() {
        // `--no-install-recommends` inside the heredoc clears the apt rule.
        let content =
            "RUN <<EOF\napt-get update\napt-get install -y --no-install-recommends git\nEOF\n";
        assert!(risks(content).is_empty());
    }

    #[test]
    fn shift_operator_is_not_mistaken_for_a_heredoc() {
        // A bash arithmetic shift must not open a heredoc and swallow lines.
        let content = "RUN echo $((1<<2))\nADD https://example.com/x /tmp/\n";
        // The ADD on the next line must still be seen (not absorbed).
        assert_eq!(risks(content), vec![DockerRisk::AddRemoteFetch]);
    }
}
