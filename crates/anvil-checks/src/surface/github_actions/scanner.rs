//! SURFGHA-001 (file detection) + SURFGHA-002 (supply-chain pattern
//! catalogue) for the GitHub Actions workflow governance surface.
//!
//! Track 3 surfaces are **pattern-catalogue** work, not parser work
//! (`plans/specs/2026-04-08-language-and-coverage-design.md` §8.3 row 2): we
//! do `#`-comment-aware line scanning, not a full YAML parse, so findings
//! anchor to real source lines. Blast radius is supply-chain compromise —
//! the canonical "one ungoverned file ruins everything" case.
//!
//! Suppressions reuse the canonical Rust antipattern parser per
//! [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md);
//! the `#` comment style is already part of that parser's grammar.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::suppression::resolve_line_suppression;

/// SURFGHA-002 — supply-chain risks in workflow YAML.
pub const SURFGHA_002_RULE_ID: &str = "SURFGHA-002";

/// True when `path` is a GitHub Actions workflow file: a `*.yml`/`*.yaml`
/// file **directly** inside a `.github/workflows/` directory. GitHub does not
/// recurse into subdirectories, so `.github/workflows/sub/ci.yml` is not a
/// workflow; the segment must also be a real path component, not a substring
/// of a longer name (`x.github/workflows/…`).
#[must_use]
pub fn is_workflow_file(path: &Path) -> bool {
    const SEG: &str = ".github/workflows/";
    let is_yaml = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("yml") || e.eq_ignore_ascii_case("yaml"));
    if !is_yaml {
        return false;
    }
    let normalised = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let Some(pos) = normalised.find(SEG) else {
        return false;
    };
    // The segment must start the path or be preceded by a `/`.
    if pos != 0 && normalised.as_bytes()[pos - 1] != b'/' {
        return false;
    }
    // The file must sit directly in the directory (no further `/`).
    let after = &normalised[pos + SEG.len()..];
    !after.is_empty() && !after.contains('/')
}

/// The kind of supply-chain risk a [`GhaFinding`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GhaRisk {
    /// `uses: owner/repo@<branch>` — a mutable branch ref (not a SHA or a
    /// version tag) can be re-pointed at malicious code after review.
    UnpinnedActionRef,
    /// `pull_request_target` trigger — runs with repo write/secrets in the
    /// context of an untrusted fork PR.
    PullRequestTarget,
    /// `runs-on: self-hosted` — a self-hosted runner exposed to untrusted
    /// (e.g. fork PR) code can be compromised.
    SelfHostedRunner,
}

impl GhaRisk {
    /// Human-readable summary used in the finding message.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::UnpinnedActionRef => {
                "action pinned to a mutable branch ref (pin to a commit SHA or release tag)"
            }
            Self::PullRequestTarget => {
                "pull_request_target runs with write/secrets in an untrusted fork context"
            }
            Self::SelfHostedRunner => "self-hosted runner may be exposed to untrusted code",
        }
    }
}

/// A single SURFGHA-002 finding, anchored to its source line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhaFinding {
    pub file: String,
    /// 1-indexed source line.
    pub line: usize,
    pub risk: GhaRisk,
    /// The offending line, trimmed and truncated for display.
    pub snippet: String,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Scan one workflow file's `content` for supply-chain risks (SURFGHA-002).
///
/// Line-based and `#`-comment-aware: the part of each line after an unquoted
/// `#` is ignored, so a risk keyword inside a comment never fires.
#[must_use]
pub fn scan_workflow(display_path: &str, content: &str) -> Vec<GhaFinding> {
    let lines: Vec<&str> = content.lines().collect();
    let mut findings = Vec::new();
    // Indent of an open multi-line `runs-on:` block, so a self-hosted runner
    // declared as `runs-on:\n  group: self-hosted` is still detected — not
    // just the inline `runs-on: self-hosted` form.
    let mut runs_on_block: Option<usize> = None;

    for (idx, raw) in lines.iter().enumerate() {
        let line_number = idx + 1;
        let code = strip_comment(raw);
        let indent = code.len() - code.trim_start().len();
        let trimmed = code.trim();

        let mut risks: Vec<GhaRisk> = Vec::new();
        if uses_branch_ref(trimmed).is_some() {
            risks.push(GhaRisk::UnpinnedActionRef);
        }
        if is_pull_request_target_line(trimmed) {
            risks.push(GhaRisk::PullRequestTarget);
        }
        if self_hosted_on_line(trimmed, indent, runs_on_block) {
            risks.push(GhaRisk::SelfHostedRunner);
        }

        for risk in risks {
            let (suppressed, reason) =
                resolve_line_suppression(&lines, line_number, SURFGHA_002_RULE_ID);
            findings.push(GhaFinding {
                file: display_path.to_string(),
                line: line_number,
                risk,
                snippet: truncate(trimmed),
                suppressed,
                suppression_reason: reason,
            });
        }

        runs_on_block = next_runs_on_block(trimmed, indent, runs_on_block);
    }
    findings
}

const SNIPPET_CAP: usize = 120;

fn truncate(s: &str) -> String {
    if s.len() <= SNIPPET_CAP {
        return s.to_string();
    }
    let mut end = SNIPPET_CAP;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// Drop the `#`-comment tail of a YAML line. A `#` inside a single- or
/// double-quoted scalar is data, not a comment, so quote state is tracked.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            // A `#` only starts a comment at line start or after whitespace.
            b'#' if !in_single
                && !in_double
                && (i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') =>
            {
                return &line[..i];
            }
            _ => {}
        }
    }
    line
}

/// True when `trimmed` declares the `pull_request_target` trigger in a
/// **structural** position — the key (`pull_request_target:`), a list item
/// (`- pull_request_target`), or an `on:` scalar/flow-sequence/flow-mapping
/// value. This deliberately does NOT match the token inside an arbitrary
/// string value (a `name:`/`run:`/`description:` that merely mentions it).
fn is_pull_request_target_line(trimmed: &str) -> bool {
    const TOK: &str = "pull_request_target";
    // Key form: `pull_request_target:` (with or without an inline value).
    if trimmed == TOK
        || trimmed
            .strip_prefix(TOK)
            .is_some_and(|r| r.starts_with(':'))
    {
        return true;
    }
    // List-item form: `- pull_request_target` / `- pull_request_target:`.
    if let Some(rest) = trimmed.strip_prefix('-') {
        let rest = rest.trim_start();
        if rest == TOK || rest.strip_prefix(TOK).is_some_and(|r| r.starts_with(':')) {
            return true;
        }
    }
    // `on:` value forms: scalar, flow sequence, or flow mapping.
    if let Some(rest) = trimmed.strip_prefix("on:") {
        if rest.trim() == TOK {
            return true;
        }
        // Flow sequence: `on: [push, pull_request_target]`.
        if rest.contains('[')
            && rest
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .any(|seg| seg == TOK)
        {
            return true;
        }
        // Flow mapping: `on: { pull_request_target: {} }` — key form only.
        if rest.contains('{') && flow_mapping_has_key(rest, TOK) {
            return true;
        }
    }
    false
}

/// True when `hay` contains `key` as a **top-level** YAML flow-mapping key
/// (brace depth 1). Nested keys such as
/// `on: { workflow_call: { inputs: { pull_request_target: {} } } }` do not
/// match, and substrings like `not_pull_request_target:` are rejected.
fn flow_mapping_has_key(hay: &str, key: &str) -> bool {
    flow_map_top_level_value(hay, key).is_some()
}

/// Locate `key` as a top-level key inside the first flow mapping in `hay` and
/// return the remainder of the string starting at that key's value (after `:`).
///
/// Tracks brace depth and quote state so nested maps and quoted scalars do not
/// produce false positives (e.g. `with: { uses: … }` is not a top-level `uses`).
fn flow_map_top_level_value<'a>(hay: &'a str, key: &str) -> Option<&'a str> {
    let bytes = hay.as_bytes();
    let open = hay.find('{')?;
    let mut i = open;
    let mut depth: i32 = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut expect_key = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_single {
            if b == b'\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if b == b'"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' => {
                in_single = true;
                i += 1;
            }
            b'"' => {
                in_double = true;
                i += 1;
            }
            b'{' => {
                depth += 1;
                if depth == 1 {
                    expect_key = true;
                }
                i += 1;
            }
            b'}' => {
                depth -= 1;
                if depth <= 0 {
                    return None;
                }
                i += 1;
            }
            b',' if depth == 1 => {
                expect_key = true;
                i += 1;
            }
            _ if depth == 1 && expect_key && !b.is_ascii_whitespace() => {
                // Optional quoted key: "key" / 'key'
                let (parsed, next) = parse_flow_key(hay, i);
                i = next;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b':' {
                    i += 1; // consume ':'
                    if parsed.as_deref() == Some(key) {
                        return Some(&hay[i..]);
                    }
                    expect_key = false;
                }
                // If no colon, keep scanning (malformed / not a key).
            }
            _ => {
                i += 1;
            }
        }
    }
    None
}

/// Parse a flow-mapping key at `start`: unquoted identifier, or a single- or
/// double-quoted scalar. Returns `(Some(key), index_after_key)` or
/// `(None, start+1)` when the next token is not a key shape.
fn parse_flow_key(hay: &str, start: usize) -> (Option<String>, usize) {
    let bytes = hay.as_bytes();
    if start >= bytes.len() {
        return (None, start);
    }
    let b = bytes[start];
    if b == b'"' || b == b'\'' {
        let quote = b;
        let mut i = start + 1;
        while i < bytes.len() {
            if bytes[i] == quote {
                let inner = &hay[start + 1..i];
                return (Some(inner.to_string()), i + 1);
            }
            i += 1;
        }
        return (None, start + 1);
    }
    if !(b.is_ascii_alphanumeric() || b == b'_') {
        return (None, start + 1);
    }
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' {
            i += 1;
        } else {
            break;
        }
    }
    (Some(hay[start..i].to_string()), i)
}

/// True when this line declares a self-hosted runner — either inline
/// (`runs-on: self-hosted` / `[self-hosted, …]`) or as an indented
/// continuation of an open multi-line `runs-on:` block.
fn self_hosted_on_line(trimmed: &str, indent: usize, runs_on_block: Option<usize>) -> bool {
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("runs-on:") && lower.contains("self-hosted") {
        return true;
    }
    matches!(runs_on_block, Some(bi) if indent > bi && lower.contains("self-hosted"))
}

/// Track the open multi-line `runs-on:` block across lines: a bare
/// `runs-on:` (no inline value) opens one at its indent; any later non-blank
/// line at or above that indent closes it.
fn next_runs_on_block(trimmed: &str, indent: usize, current: Option<usize>) -> Option<usize> {
    if trimmed == "runs-on:" {
        return Some(indent);
    }
    match current {
        Some(bi) if !trimmed.is_empty() && indent <= bi => None,
        other => other,
    }
}

/// If `line` is a `uses:` step pinned to a mutable **branch** ref, return the
/// ref. Returns `None` for SHA pins, version tags (`v1`, `v1.2.3`), local
/// (`./…`) or docker (`docker://…`) actions, and non-`uses` lines.
///
/// Recognises block form (`uses:` / `- uses:`) and YAML flow-mapping steps
/// such as `- { uses: actions/checkout@main }`.
fn uses_branch_ref(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("uses:")
        .or_else(|| line.strip_prefix("- uses:"))
        .or_else(|| line.strip_prefix("-uses:"))
        .or_else(|| uses_value_from_flow_mapping(line))?;
    let value = take_flow_scalar(rest).trim_matches(['"', '\'']);
    if value.starts_with("./") || value.starts_with("docker://") {
        return None;
    }
    let reference = value.rsplit_once('@')?.1;
    if reference.is_empty() || is_sha(reference) || is_version_tag(reference) {
        return None;
    }
    Some(reference)
}

/// Extract the value after a top-level `uses:` key inside a YAML flow mapping,
/// e.g. `- { uses: owner/repo@ref }` or `{ name: x, uses: owner/repo@ref }`.
/// Nested maps such as `with: { uses: … }` are ignored.
fn uses_value_from_flow_mapping(line: &str) -> Option<&str> {
    let s = line.trim_start();
    let s = match s.strip_prefix('-') {
        Some(rest) => rest.trim_start(),
        None => s,
    };
    if !s.starts_with('{') {
        return None;
    }
    flow_map_top_level_value(s, "uses")
}

/// Take a single YAML flow scalar from the start of `s`, stopping at an
/// unquoted `,` or `}` so multi-key flow maps do not bleed into the value.
fn take_flow_scalar(s: &str) -> &str {
    let s = s.trim_start();
    if s.is_empty() {
        return s;
    }
    let bytes = s.as_bytes();
    let quote = bytes[0];
    if quote == b'"' || quote == b'\'' {
        for i in 1..bytes.len() {
            if bytes[i] == quote {
                return &s[..=i];
            }
        }
        return s;
    }
    for (i, &b) in bytes.iter().enumerate() {
        if b == b',' || b == b'}' {
            return s[..i].trim_end();
        }
    }
    s.trim_end()
}

/// A 40- or 64-hex-char commit SHA (full or sha256-ish) is an immutable pin.
fn is_sha(reference: &str) -> bool {
    let len = reference.len();
    (len == 40 || len == 64) && reference.chars().all(|c| c.is_ascii_hexdigit())
}

/// A release/version tag — `vN[.N…]` or a bare `N[.N…]` (e.g. `@v4`, `@3`,
/// `@2.0.3`). These are accepted pins per the module (only mutable **branch**
/// refs are flagged); many popular actions tag without a `v` prefix.
fn is_version_tag(reference: &str) -> bool {
    let core = reference.strip_prefix('v').unwrap_or(reference);
    !core.is_empty()
        && core
            .split('.')
            .all(|seg| !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn risks(content: &str) -> Vec<GhaRisk> {
        scan_workflow("w.yml", content)
            .into_iter()
            .map(|f| f.risk)
            .collect()
    }

    #[test]
    fn detects_workflow_files_by_path() {
        assert!(is_workflow_file(Path::new(".github/workflows/ci.yml")));
        assert!(is_workflow_file(Path::new(".github/workflows/ci.yaml")));
        assert!(is_workflow_file(Path::new(
            "repo/.github/workflows/release.yml"
        )));
        assert!(is_workflow_file(Path::new(
            "repo\\.github\\workflows\\ci.yml"
        )));
        // Not a workflow: wrong dir or wrong extension.
        assert!(!is_workflow_file(Path::new("src/ci.yml")));
        assert!(!is_workflow_file(Path::new(".github/actions/x/action.yml")));
        assert!(!is_workflow_file(Path::new(".github/workflows/notes.md")));
        // GitHub does not recurse: a nested subdir file is not a workflow.
        assert!(!is_workflow_file(Path::new(".github/workflows/sub/ci.yml")));
        // The segment must be a real path component, not a substring.
        assert!(!is_workflow_file(Path::new("x.github/workflows/ci.yml")));
    }

    #[test]
    fn flags_unpinned_branch_refs_only() {
        assert_eq!(
            risks("      - uses: actions/checkout@main\n"),
            vec![GhaRisk::UnpinnedActionRef]
        );
        assert_eq!(
            risks("      - uses: foo/bar@my-feature-branch\n"),
            vec![GhaRisk::UnpinnedActionRef]
        );
        // Pinned forms must NOT fire — including bare-numeric tags (common:
        // docker/build-push-action@3, hashicorp/setup-terraform@2.0.3).
        assert!(risks("      - uses: actions/checkout@v4\n").is_empty());
        assert!(risks("      - uses: actions/checkout@v4.1.0\n").is_empty());
        assert!(risks("      - uses: docker/build-push-action@3\n").is_empty());
        assert!(risks("      - uses: hashicorp/setup-terraform@2.0.3\n").is_empty());
        assert!(
            risks("      - uses: actions/checkout@8f4b7f84864484a7bf31766abe9204da3cbe65b3\n")
                .is_empty()
        );
        // Local + docker actions are out of scope.
        assert!(risks("      - uses: ./.github/actions/setup\n").is_empty());
        assert!(risks("      - uses: docker://alpine:3.19\n").is_empty());
    }

    #[test]
    fn flags_pull_request_target_trigger() {
        assert_eq!(
            risks("on:\n  pull_request_target:\n"),
            vec![GhaRisk::PullRequestTarget]
        );
        assert_eq!(
            risks("on: [push, pull_request_target]\n"),
            vec![GhaRisk::PullRequestTarget]
        );
        assert_eq!(
            risks("on: pull_request_target\n"),
            vec![GhaRisk::PullRequestTarget]
        );
        // Not a substring match: `pull_request` alone is fine.
        assert!(risks("on:\n  pull_request:\n").is_empty());
    }

    #[test]
    fn flags_flow_mapping_pull_request_target_and_uses() {
        // YAML flow mappings are valid GitHub Actions syntax and must not
        // bypass SURFGHA-002 (clawpatch fnd_sig-feat-service-e706bccd06).
        assert_eq!(
            risks("on: { pull_request_target: {} }\n"),
            vec![GhaRisk::PullRequestTarget]
        );
        assert_eq!(
            risks("on: { push: null, pull_request_target: {} }\n"),
            vec![GhaRisk::PullRequestTarget]
        );
        assert_eq!(
            risks("      - { uses: actions/checkout@main }\n"),
            vec![GhaRisk::UnpinnedActionRef]
        );
        assert_eq!(
            risks("      - { name: Checkout, uses: actions/checkout@main }\n"),
            vec![GhaRisk::UnpinnedActionRef]
        );
        // Nested keys must not fire (top-level only).
        assert!(
            risks("on: { workflow_call: { inputs: { pull_request_target: {} } } }\n").is_empty()
        );
        assert!(
            risks("      - { with: { uses: actions/checkout@main }, run: echo hi }\n").is_empty()
        );
        // Pinned flow-mapping uses remain clean.
        assert!(risks("      - { uses: actions/checkout@v4 }\n").is_empty());
        // Substring key must not fire.
        assert!(risks("on: { not_pull_request_target: {} }\n").is_empty());
    }

    #[test]
    fn pull_request_target_in_value_strings_is_not_flagged() {
        // The token in a name/run/description value is prose, not a trigger.
        assert!(risks("    name: \"runs on pull_request_target events\"\n").is_empty());
        assert!(risks("    - run: echo pull_request_target is risky\n").is_empty());
        assert!(risks("not-pull_request_target: true\n").is_empty());
    }

    #[test]
    fn flags_self_hosted_runner() {
        assert_eq!(
            risks("    runs-on: self-hosted\n"),
            vec![GhaRisk::SelfHostedRunner]
        );
        assert_eq!(
            risks("    runs-on: [self-hosted, linux]\n"),
            vec![GhaRisk::SelfHostedRunner]
        );
        assert!(risks("    runs-on: ubuntu-latest\n").is_empty());
    }

    #[test]
    fn flags_self_hosted_in_multiline_runs_on_block() {
        // Org runner-group form: `runs-on:` then an indented `group:`.
        let content = "jobs:\n  deploy:\n    runs-on:\n      group: self-hosted-prod\n      labels: [linux]\n    steps: []\n";
        assert_eq!(risks(content), vec![GhaRisk::SelfHostedRunner]);
        // The block closes at dedent: a later inline ubuntu runner is clean.
        let content2 = "jobs:\n  a:\n    runs-on:\n      group: self-hosted\n  b:\n    runs-on: ubuntu-latest\n";
        assert_eq!(risks(content2), vec![GhaRisk::SelfHostedRunner]);
    }

    #[test]
    fn ignores_risks_inside_comments() {
        assert!(risks("      # uses: actions/checkout@main\n").is_empty());
        assert!(risks("    name: test  # pull_request_target notes\n").is_empty());
        // A `#` inside a quoted scalar is not a comment.
        assert_eq!(
            risks("      - uses: \"actions/checkout@main\"  # pinned soon\n"),
            vec![GhaRisk::UnpinnedActionRef]
        );
    }

    #[test]
    fn anchors_and_suppresses() {
        // Directive on the line immediately above the offending `uses:`.
        let content = "jobs:\n  build:\n    steps:\n      # @anvil-ignore SURFGHA-002 -- vetted internal action\n      - uses: myorg/internal@main\n";
        let findings = scan_workflow("w.yml", content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 5);
        assert!(findings[0].suppressed);
        assert_eq!(
            findings[0].suppression_reason.as_deref(),
            Some("vetted internal action")
        );
    }

    #[test]
    fn clean_workflow_has_no_findings() {
        let content = "name: ci\non:\n  push:\n  pull_request:\npermissions:\n  contents: read\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n";
        assert!(scan_workflow("ci.yml", content).is_empty());
    }
}
