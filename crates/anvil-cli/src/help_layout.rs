use std::fmt;

use clap::Command;

const CLI_SURFACE_RUNBOOK: &str = include_str!("../../../docs/runbooks/cli-surface.md");
const CLI_SURFACE_RUNBOOK_PATH: &str = "docs/runbooks/cli-surface.md";

/// Return every non-hidden command path exposed by clap, including nested
/// subcommands but excluding hidden compatibility aliases.
pub fn visible_command_paths(root: &Command) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    let mut prefix = vec![root.get_name().to_owned()];
    collect_visible_command_paths(root, &mut prefix, &mut paths);
    paths.sort();
    paths
}

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

pub fn lint_clic_010_layout(root: &Command) -> Vec<HelpLayoutFinding> {
    let mut findings = Vec::new();
    let mut prefix = vec![root.get_name().to_owned()];
    lint_visible_subcommands(root, &mut prefix, &mut findings);
    findings
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpLayoutFinding {
    path: String,
    kind: HelpLayoutFindingKind,
}

impl fmt::Display for HelpLayoutFinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HelpLayoutFindingKind {
    Summary,
    WhenToUse,
    LearnMore,
}

impl fmt::Display for HelpLayoutFindingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Summary => write!(f, "missing one-line summary"),
            Self::WhenToUse => write!(f, "missing when-to-use guidance"),
            Self::LearnMore => write!(f, "missing learn-more docs pointer"),
        }
    }
}

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

fn finding(path: &[String], kind: HelpLayoutFindingKind) -> HelpLayoutFinding {
    HelpLayoutFinding {
        path: path.join(" "),
        kind,
    }
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
        "{CLI_SURFACE_RUNBOOK_PATH}#anvil-{}",
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
