//! CLIC-010 help-layout support (CIB-040).
//!
//! Centralises the help-text layout policy beside the CLI surface runbook:
//! runtime `--help` augmentation (`augment_clic_010_help`) appends a uniform
//! WHEN TO USE / COMMON FLAGS / LEARN MORE footer to every visible command,
//! while the test-only inventory and lint helpers assert that every visible
//! command path has the metadata the footer needs.

use clap::Command;

const CLI_SURFACE_RUNBOOK: &str = include_str!("../../../docs/runbooks/cli-surface.md");
const CLI_SURFACE_RUNBOOK_PATH: &str = "docs/runbooks/cli-surface.md";

/// Append the uniform CLIC-010 footer to every visible command's long help,
/// preserving any command-specific footer the command already declared.
pub fn augment_clic_010_help(mut command: Command) -> Command {
    let mut path = vec![command.get_name().to_owned()];
    augment_visible_subcommands(&mut command, &mut path);
    command
}

fn augment_visible_subcommands(command: &mut Command, prefix: &mut Vec<String>) {
    for subcommand in command
        .get_subcommands_mut()
        .filter(|sub| !sub.is_hide_set())
    {
        prefix.push(subcommand.get_name().to_owned());
        if let Some(addition) = clic_010_footer(prefix) {
            append_after_long_help(subcommand, &addition);
        }
        augment_visible_subcommands(subcommand, prefix);
        prefix.pop();
    }
}

fn clic_010_footer(path: &[String]) -> Option<String> {
    let when_to_use = when_to_use(path)?;
    let learn_more = learn_more_pointer(path)?;
    Some(format!(
        "WHEN TO USE:\n  {when_to_use}\n\nCOMMON FLAGS:\n  --json       Output results as JSON instead of human-readable text.\n  --no-tui     Disable TUI rendering; use plain text output.\n  -v, --verbose  Enable verbose logging.\n\nLEARN MORE:\n  {learn_more}",
    ))
}

fn append_after_long_help(command: &mut Command, addition: &str) {
    let mut combined = String::new();
    if let Some(existing) = command.get_after_long_help() {
        combined.push_str(&existing.to_string());
    } else if let Some(existing) = command.get_after_help() {
        combined.push_str(&existing.to_string());
    }
    if !combined.trim().is_empty() {
        combined.push_str("\n\n");
    }
    combined.push_str(addition);

    let updated = std::mem::take(command).after_long_help(combined);
    *command = updated;
}

fn when_to_use(path: &[String]) -> Option<String> {
    let top_level = path.get(1)?;
    let base = runbook_section(top_level).and_then(extract_when_to_use)?;
    if path.len() <= 2 {
        Some(base)
    } else {
        Some(format!(
            "{base} Use `{}` for this focused operation.",
            path.join(" ")
        ))
    }
}

fn learn_more_pointer(path: &[String]) -> Option<String> {
    let top_level = path.get(1)?;
    runbook_section(top_level)?;
    Some(format!(
        "{CLI_SURFACE_RUNBOOK_PATH}#{}",
        markdown_slug(&format!("anvil {top_level}"))
    ))
}

fn runbook_section(top_level: &str) -> Option<&'static str> {
    let heading = format!("## anvil {top_level}");
    let start = CLI_SURFACE_RUNBOOK.find(&heading)?;
    let body = &CLI_SURFACE_RUNBOOK[start..];
    let end = body[heading.len()..]
        .find("\n## anvil ")
        .map_or(body.len(), |offset| heading.len() + offset);
    Some(&body[..end])
}

fn extract_when_to_use(section: &str) -> Option<String> {
    // Collapse soft line wraps so markdown reflow (e.g. "**When\nto use:**")
    // does not hide the field. Paragraph breaks survive as a sentinel token.
    let collapsed = section
        .replace("\n\n", " \u{1e} ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let marker = "**When to use:**";
    let start = collapsed.find(marker)? + marker.len();
    let rest = collapsed[start..].trim_start();
    // The field ends at the paragraph sentinel or the next bold field marker.
    let end = [rest.find('\u{1e}'), rest.find("**")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(rest.len());
    let prose = rest[..end].trim();
    if prose.is_empty() {
        None
    } else {
        Some(prose.to_owned())
    }
}

fn markdown_slug(heading: &str) -> String {
    heading
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c.is_whitespace() || c == '-' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ── CLIC-010 lint (test-only) ───────────────────────────────────────
//
// The CI-facing lint that asserts every visible command path has the
// metadata the runtime footer needs. Compiled only under `cfg(test)`
// because the runtime augmentation, not the lint, ships in the binary.

#[cfg(test)]
use std::fmt;

/// Return every non-hidden command path exposed by clap, including nested
/// subcommands but excluding hidden compatibility aliases.
#[cfg(test)]
pub fn visible_command_paths(root: &Command) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    let mut prefix = vec![root.get_name().to_owned()];
    collect_visible_command_paths(root, &mut prefix, &mut paths);
    paths.sort();
    paths
}

#[cfg(test)]
pub fn contains_path(paths: &[Vec<String>], expected: &[&str]) -> bool {
    paths.iter().any(|path| {
        path.len() == expected.len()
            && path
                .iter()
                .map(String::as_str)
                .zip(expected.iter().copied())
                .all(|(actual, expected)| actual == expected)
    })
}

#[cfg(test)]
pub fn lint_clic_010_layout(root: &Command) -> Vec<HelpLayoutFinding> {
    let mut findings = Vec::new();
    let mut prefix = vec![root.get_name().to_owned()];
    lint_visible_subcommands(root, &mut prefix, &mut findings);
    findings
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpLayoutFinding {
    path: String,
    kind: HelpLayoutFindingKind,
}

#[cfg(test)]
impl fmt::Display for HelpLayoutFinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.kind)
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum HelpLayoutFindingKind {
    Summary,
    WhenToUse,
    LearnMore,
}

#[cfg(test)]
impl fmt::Display for HelpLayoutFindingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Summary => write!(f, "missing one-line summary"),
            Self::WhenToUse => write!(f, "missing when-to-use guidance"),
            Self::LearnMore => write!(f, "missing learn-more docs pointer"),
        }
    }
}

#[cfg(test)]
fn collect_visible_command_paths(
    command: &Command,
    prefix: &mut Vec<String>,
    paths: &mut Vec<Vec<String>>,
) {
    for subcommand in command.get_subcommands().filter(|sub| !sub.is_hide_set()) {
        prefix.push(subcommand.get_name().to_owned());
        paths.push(prefix.clone());
        collect_visible_command_paths(subcommand, prefix, paths);
        prefix.pop();
    }
}

#[cfg(test)]
fn lint_visible_subcommands(
    command: &Command,
    prefix: &mut Vec<String>,
    findings: &mut Vec<HelpLayoutFinding>,
) {
    for subcommand in command.get_subcommands().filter(|sub| !sub.is_hide_set()) {
        prefix.push(subcommand.get_name().to_owned());
        lint_command(prefix, subcommand, findings);
        lint_visible_subcommands(subcommand, prefix, findings);
        prefix.pop();
    }
}

#[cfg(test)]
fn lint_command(path: &[String], command: &Command, findings: &mut Vec<HelpLayoutFinding>) {
    if command.get_about().is_none() && command.get_long_about().is_none() {
        findings.push(finding(path, HelpLayoutFindingKind::Summary));
    }
    if when_to_use(path).is_none() {
        findings.push(finding(path, HelpLayoutFindingKind::WhenToUse));
    }
    if learn_more_pointer(path).is_none() {
        findings.push(finding(path, HelpLayoutFindingKind::LearnMore));
    }
}

#[cfg(test)]
fn finding(path: &[String], kind: HelpLayoutFindingKind) -> HelpLayoutFinding {
    HelpLayoutFinding {
        path: path.join(" "),
        kind,
    }
}
