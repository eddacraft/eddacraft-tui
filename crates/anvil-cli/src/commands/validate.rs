use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use anyhow::{Result, bail};
use clap::Args;
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::GlobalArgs;
use crate::output::{self, OutputMode};

/// Supported plan format names.
const FORMAT_APS: &str = "aps-markdown";

/// Task ID pattern: 1-10 uppercase alphanumeric scope, hyphen, 3-digit number.
const TASK_ID_PATTERN: &str = r"^[A-Z0-9]{1,10}-\d{3}$";

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Plan file path to validate.
    plan: String,

    /// Explicitly specify input format (bypasses auto-detection).
    #[arg(long)]
    format: Option<String>,

    /// Skip hash integrity validation.
    #[arg(long)]
    no_validate_hash: bool,
}

// ── Validation types ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
struct ValidationIssue {
    severity: IssueSeverity,
    message: String,
    rule: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum DocumentType {
    Index,
    Leaf,
}

impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Index => f.write_str("index"),
            Self::Leaf => f.write_str("leaf"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct DetectedFormat {
    format: String,
    confidence: u8,
    document_type: DocumentType,
}

// ── JSON output schema ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ValidateOutput {
    valid: bool,
    format: DetectedFormat,
    file: String,
    #[serde(rename = "executionTimeMs")]
    execution_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hash_verified: Option<bool>,
    tasks_found: usize,
    issues: Vec<ValidationIssue>,
    errors: usize,
    warnings: usize,
}

// ── Entry point ─────────────────────────────────────────────────────

pub fn run(args: &ValidateArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_global(global);
    let start = Instant::now();
    let validate_hash = !args.no_validate_hash;

    let plan_path = Path::new(&args.plan);
    if !plan_path.exists() {
        bail!("Plan file not found: {}", args.plan);
    }

    let content = std::fs::read_to_string(plan_path)?;
    let file_display = args.plan.clone();

    // Detect format.
    let detected = detect_format(&content, args.format.as_deref());

    // Parse and validate.
    let mut issues = Vec::new();
    let title = extract_title(&content);

    let task_count = match detected.document_type {
        DocumentType::Index => {
            validate_index_structure(&content, &file_display, &mut issues);
            0 // Index files don't contain tasks directly.
        }
        DocumentType::Leaf => {
            validate_leaf_structure(&content, &file_display, &mut issues);
            validate_tasks(&content, &file_display, &mut issues)
        }
    };

    // Hash verification (APS markdown only, when requested).
    let hash_verified = if validate_hash && detected.format == FORMAT_APS {
        verify_content_hash(&content)
    } else {
        None
    };

    let elapsed = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    // Treat hash mismatch as a validation error.
    if hash_verified == Some(false) {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Content hash verification failed — file may have been tampered with"
                .to_string(),
            rule: "hash-integrity".to_string(),
            path: Some(file_display.clone()),
            line: None,
        });
    }

    let error_count = issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Warning)
        .count();
    let valid = error_count == 0;

    match mode {
        OutputMode::Json => {
            let output = ValidateOutput {
                valid,
                format: detected.clone(),
                file: file_display,
                execution_time_ms: elapsed,
                title: title.clone(),
                hash_verified,
                tasks_found: task_count,
                issues: issues.clone(),
                errors: error_count,
                warnings: warning_count,
            };
            output::json::print(&output)?;
        }
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            print_human(
                &file_display,
                &detected,
                title.as_deref(),
                &issues,
                valid,
                hash_verified,
                task_count,
                error_count,
                warning_count,
                global.verbose,
            );
        }
    }

    if valid {
        Ok(())
    } else {
        Err(output::AlreadyReported.into())
    }
}

// ── Format detection ────────────────────────────────────────────────

fn detect_format(content: &str, format_override: Option<&str>) -> DetectedFormat {
    if let Some(fmt) = format_override {
        let doc_type = if is_index_file(content) {
            DocumentType::Index
        } else {
            DocumentType::Leaf
        };
        return DetectedFormat {
            format: fmt.to_string(),
            confidence: 100,
            document_type: doc_type,
        };
    }

    let mut confidence: u8 = 0;

    // Check for APS indicators.
    if content.contains("## Tasks") || content.contains("## Modules") {
        confidence += 15;
    }
    // SCOPE-NNN pattern in headings.
    let task_heading_re = Regex::new(r"### [A-Z0-9]{1,10}-\d{3}:").unwrap();
    if task_heading_re.is_match(content) {
        confidence += 30;
    }
    // Intent field.
    if content.contains("**Intent:**") {
        confidence += 20;
    }
    // .aps.md link references.
    if content.contains(".aps.md") {
        confidence += 15;
    }
    // Status/Owner/Priority metadata fields.
    if content.contains("**Status:**") {
        confidence += 5;
    }
    if content.contains("**Owner:**") {
        confidence += 5;
    }
    if content.contains("**Priority:**") {
        confidence += 5;
    }

    let doc_type = if is_index_file(content) {
        DocumentType::Index
    } else {
        DocumentType::Leaf
    };

    // Cap at 100.
    DetectedFormat {
        format: FORMAT_APS.to_string(),
        confidence: confidence.min(100),
        document_type: doc_type,
    }
}

fn is_index_file(content: &str) -> bool {
    content.contains("## Modules")
}

// ── Title extraction ────────────────────────────────────────────────

fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            // Skip HTML comments that sometimes precede the title.
            if !title.starts_with("<!--") {
                return Some(title.to_string());
            }
        }
    }
    None
}

// ── Structure validation ────────────────────────────────────────────

fn validate_index_structure(content: &str, file: &str, issues: &mut Vec<ValidationIssue>) {
    // Must have ## Modules section.
    if !content.contains("## Modules") {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Index file missing required '## Modules' section".to_string(),
            rule: "required-sections".to_string(),
            path: Some(file.to_string()),
            line: None,
        });
    }

    // Check for module link entries.
    let link_re = Regex::new(r"\[.*?\]\(.*?\.aps\.md\)").unwrap();
    let has_module_links = content.lines().any(|line| link_re.is_match(line));
    if content.contains("## Modules") && !has_module_links {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Warning,
            message: "Modules section has no .aps.md links".to_string(),
            rule: "module-links".to_string(),
            path: Some(file.to_string()),
            line: None,
        });
    }

    // Validate module link paths exist.
    let plan_dir = Path::new(file).parent();
    let path_re = Regex::new(r"\]\((.*?\.aps\.md)\)").unwrap();
    for (line_num, line) in content.lines().enumerate() {
        for cap in path_re.captures_iter(line) {
            let link_path = &cap[1];
            // Reject absolute paths (cross-platform).
            if Path::new(link_path).is_absolute() {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    message: format!("Unsafe module path: {link_path}"),
                    rule: "path-safety".to_string(),
                    path: Some(file.to_string()),
                    line: Some(line_num + 1),
                });
                continue;
            }
            // Reject any parent-directory components (catches .., ./foo/../../etc).
            if Path::new(link_path)
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    message: format!(
                        "Unsafe module path: {link_path} (parent directory traversal)"
                    ),
                    rule: "path-safety".to_string(),
                    path: Some(file.to_string()),
                    line: Some(line_num + 1),
                });
                continue;
            }
            if let Some(dir) = plan_dir {
                let resolved = dir.join(link_path);
                if !resolved.exists() {
                    issues.push(ValidationIssue {
                        severity: IssueSeverity::Error,
                        message: format!("Broken module link: {link_path} (file not found)"),
                        rule: "broken-links".to_string(),
                        path: Some(file.to_string()),
                        line: Some(line_num + 1),
                    });
                }
            }
        }
    }
}

fn validate_leaf_structure(content: &str, file: &str, issues: &mut Vec<ValidationIssue>) {
    // Leaf specs should have a ## Tasks section.
    if !content.contains("## Tasks") {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Leaf spec missing required '## Tasks' section".to_string(),
            rule: "required-sections".to_string(),
            path: Some(file.to_string()),
            line: None,
        });
    }

    // Should have an H1 title.
    let has_title = content
        .lines()
        .any(|l| l.trim().starts_with("# ") && !l.trim().starts_with("# <!--"));
    if !has_title {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Warning,
            message: "Document missing H1 title".to_string(),
            rule: "document-title".to_string(),
            path: Some(file.to_string()),
            line: None,
        });
    }
}

// ── Task validation ─────────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn validate_tasks(content: &str, file: &str, issues: &mut Vec<ValidationIssue>) -> usize {
    let task_heading_re = Regex::new(r"^### (.+)$").unwrap();
    let task_id_re = Regex::new(TASK_ID_PATTERN).unwrap();

    let mut task_count = 0;
    let mut seen_ids: BTreeMap<String, usize> = BTreeMap::new();
    let mut dependency_targets: BTreeSet<String> = BTreeSet::new();
    let mut current_task_id: Option<String> = None;
    let mut current_task_line: usize = 0;
    let mut current_task_has_intent = false;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Detect task heading: ### SCOPE-NNN: title
        if let Some(caps) = task_heading_re.captures(trimmed) {
            // Flush previous task.
            if let Some(ref id) = current_task_id
                && !current_task_has_intent
            {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    message: format!("Task {id} missing required **Intent:** field"),
                    rule: "task-intent-required".to_string(),
                    path: Some(file.to_string()),
                    line: Some(current_task_line),
                });
            }

            let heading = caps[1].trim();

            // Extract task ID (before first colon).
            let task_id = heading.split(':').next().unwrap_or("").trim();

            if task_id_re.is_match(task_id) {
                task_count += 1;

                // Duplicate check.
                if let Some(first_line) = seen_ids.get(task_id) {
                    issues.push(ValidationIssue {
                        severity: IssueSeverity::Error,
                        message: format!(
                            "Duplicate task ID: {task_id} (first seen at line {first_line})"
                        ),
                        rule: "duplicate-task-id".to_string(),
                        path: Some(file.to_string()),
                        line: Some(line_num + 1),
                    });
                } else {
                    seen_ids.insert(task_id.to_string(), line_num + 1);
                }

                current_task_id = Some(task_id.to_string());
                current_task_line = line_num + 1;
                current_task_has_intent = false;
            } else if !task_id.is_empty() {
                // H3 heading that doesn't match task ID format — might be
                // a phase heading (e.g. "### Phase 1 — Check & Validate")
                // which is fine. Only warn if it looks like a malformed task ID.
                if task_id.contains('-')
                    && task_id
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_uppercase())
                {
                    issues.push(ValidationIssue {
                        severity: IssueSeverity::Warning,
                        message: format!("Heading looks like a task but ID \"{task_id}\" doesn't match SCOPE-NNN format"),
                        rule: "task-id-format".to_string(),
                        path: Some(file.to_string()),
                        line: Some(line_num + 1),
                    });
                }
                current_task_id = None;
            }
            continue;
        }

        // Track fields within current task.
        if current_task_id.is_some() {
            if trimmed.starts_with("- **Intent:**") || trimmed.starts_with("**Intent:**") {
                current_task_has_intent = true;
            }

            // Collect dependency targets for cross-reference.
            if trimmed.starts_with("- **Dependencies:**")
                || trimmed.starts_with("**Dependencies:**")
            {
                let deps_str = trimmed.split_once(":**").map_or("", |x| x.1).trim();
                for dep in deps_str.split(',') {
                    let dep = dep.trim();
                    if !dep.is_empty() && dep != "(none)" && dep != "—" && dep != "-" {
                        dependency_targets.insert(dep.to_string());
                    }
                }
            }
        }
    }

    // Flush last task.
    if let Some(ref id) = current_task_id
        && !current_task_has_intent
    {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: format!("Task {id} missing required **Intent:** field"),
            rule: "task-intent-required".to_string(),
            path: Some(file.to_string()),
            line: Some(current_task_line),
        });
    }

    // Check for broken dependency references.
    for dep in &dependency_targets {
        if task_id_re.is_match(dep) && !seen_ids.contains_key(dep.as_str()) {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                message: format!("Dependency {dep} references a task not found in this file"),
                rule: "broken-dependency".to_string(),
                path: Some(file.to_string()),
                line: None,
            });
        }
    }

    task_count
}

// ── Hash verification ───────────────────────────────────────────────

/// Verify the content hash embedded in the APS markdown.
///
/// Looks for a `<!-- hash: <hex> -->` HTML comment in the file. If found,
/// hashes the content excluding the hash line itself and compares.
fn verify_content_hash(content: &str) -> Option<bool> {
    let hash_re = Regex::new(r"<!--\s*hash:\s*([0-9a-fA-F]+)\s*-->").unwrap();

    // Use the last match to skip hash comments inside code blocks.
    let caps = hash_re.captures_iter(content).last()?;
    let expected = caps[1].to_lowercase();

    // Remove exactly the matched hash comment using byte offsets.
    // Do NOT trim — preserving exact whitespace ensures the hash
    // covers the original bytes.
    let m = caps.get(0).unwrap();
    let content_without_hash = format!("{}{}", &content[..m.start()], &content[m.end()..]);

    let mut hasher = Sha256::new();
    hasher.update(content_without_hash.as_bytes());
    let actual = hex::encode(hasher.finalize());

    Some(actual == expected)
}

// ── Plain output ────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn print_human(
    file: &str,
    detected: &DetectedFormat,
    title: Option<&str>,
    issues: &[ValidationIssue],
    valid: bool,
    hash_verified: Option<bool>,
    task_count: usize,
    error_count: usize,
    warning_count: usize,
    verbose: bool,
) {
    output::plain::blank();

    if valid {
        output::plain::success("Plan is valid");
    } else {
        output::plain::error("Plan validation failed");
    }

    output::plain::blank();
    output::plain::section("Plan Details");
    output::plain::label("File", file);
    output::plain::label(
        "Format",
        format!("{} ({}% confidence)", detected.format, detected.confidence),
    );
    output::plain::label("Type", detected.document_type);

    if let Some(t) = title {
        output::plain::label("Title", t);
    }

    if detected.document_type == DocumentType::Leaf {
        output::plain::label("Tasks", task_count);
    }

    match hash_verified {
        Some(true) => output::plain::success("Hash verified"),
        Some(false) => {
            output::plain::error("Hash verification failed — content may have been modified");
        }
        None => {}
    }

    // Show issues.
    if !issues.is_empty() {
        output::plain::blank();

        let errors: Vec<&ValidationIssue> = issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        let warnings: Vec<&ValidationIssue> = issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Warning)
            .collect();

        if !errors.is_empty() {
            output::plain::section("Errors");
            if verbose {
                for issue in &errors {
                    print_issue(issue);
                }
            } else {
                output::plain::error(&format!("Found {} validation error(s)", errors.len()));
                for issue in errors.iter().take(3) {
                    output::plain::dim(&format!("  {}", issue.message));
                }
                if errors.len() > 3 {
                    output::plain::dim(&format!("  ... and {} more", errors.len() - 3));
                }
            }
        }

        if !warnings.is_empty() {
            output::plain::section("Warnings");
            if verbose {
                for issue in &warnings {
                    print_issue(issue);
                }
            } else {
                output::plain::warn(&format!("Found {} warning(s)", warnings.len()));
                for issue in warnings.iter().take(3) {
                    output::plain::dim(&format!("  {}", issue.message));
                }
                if warnings.len() > 3 {
                    output::plain::dim(&format!("  ... and {} more", warnings.len() - 3));
                }
            }
        }
    }

    output::plain::blank();
    output::plain::section("Summary");
    output::plain::label("Errors", error_count);
    output::plain::label("Warnings", warning_count);
}

fn print_issue(issue: &ValidationIssue) {
    let icon = match issue.severity {
        IssueSeverity::Error => "\u{2717}",
        IssueSeverity::Warning => "\u{26a0}",
    };
    let location = match (&issue.path, issue.line) {
        (Some(p), Some(l)) => format!("{p}:{l}"),
        (Some(p), None) => p.clone(),
        _ => String::new(),
    };
    output::plain::item(icon, &format!("[{}] {}", issue.rule, issue.message));
    if !location.is_empty() {
        output::plain::dim(&format!("  {location}"));
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Format detection ────────────────────────────────────────

    #[test]
    fn detect_aps_leaf_with_tasks() {
        let content =
            "# My Plan\n\n## Tasks\n\n### TEST-001: first task\n\n- **Intent:** Do something\n";
        let detected = detect_format(content, None);
        assert_eq!(detected.format, FORMAT_APS);
        assert_eq!(detected.document_type, DocumentType::Leaf);
        assert!(detected.confidence >= 45); // Tasks(15) + heading(30)
    }

    #[test]
    fn detect_aps_index_with_modules() {
        let content =
            "# Plan Index\n\n## Modules\n\n### auth\n- **Path:** [./auth.aps.md](./auth.aps.md)\n";
        let detected = detect_format(content, None);
        assert_eq!(detected.document_type, DocumentType::Index);
        assert!(detected.confidence >= 30); // Modules(15) + .aps.md(15)
    }

    #[test]
    fn detect_format_override() {
        let detected = detect_format("some content", Some("speckit"));
        assert_eq!(detected.format, "speckit");
        assert_eq!(detected.confidence, 100);
    }

    // ── Title extraction ────────────────────────────────────────

    #[test]
    fn extract_title_from_h1() {
        assert_eq!(
            extract_title("# My Plan\n\nContent"),
            Some("My Plan".to_string())
        );
    }

    #[test]
    fn extract_title_skips_html_comment() {
        let content = "<!--\ncomment\n-->\n\n# Real Title\n";
        assert_eq!(extract_title(content), Some("Real Title".to_string()));
    }

    #[test]
    fn extract_title_returns_none_when_missing() {
        assert_eq!(extract_title("No headings here"), None);
    }

    // ── Index structure ─────────────────────────────────────────

    #[test]
    fn index_missing_modules_section() {
        let mut issues = Vec::new();
        validate_index_structure("# Plan\n\n## Overview\n", "plan.aps.md", &mut issues);
        assert!(issues.iter().any(|i| i.rule == "required-sections"));
    }

    #[test]
    fn index_warns_on_empty_modules() {
        let mut issues = Vec::new();
        validate_index_structure(
            "# Plan\n\n## Modules\n\nNothing here.\n",
            "plan.aps.md",
            &mut issues,
        );
        assert!(issues.iter().any(|i| i.rule == "module-links"));
    }

    // ── Leaf structure ──────────────────────────────────────────

    #[test]
    fn leaf_missing_tasks_section() {
        let mut issues = Vec::new();
        validate_leaf_structure("# Plan\n\n## Overview\n", "plan.aps.md", &mut issues);
        assert!(issues.iter().any(|i| i.rule == "required-sections"));
    }

    #[test]
    fn leaf_warns_on_missing_title() {
        let mut issues = Vec::new();
        validate_leaf_structure(
            "## Tasks\n\n### TEST-001: task\n",
            "plan.aps.md",
            &mut issues,
        );
        assert!(issues.iter().any(|i| i.rule == "document-title"));
    }

    // ── Task validation ─────────────────────────────────────────

    #[test]
    fn validates_well_formed_tasks() {
        let content = "# Plan\n\n## Tasks\n\n### TEST-001: first task\n\n- **Intent:** Do something\n\n### TEST-002: second task\n\n- **Intent:** Do another thing\n";
        let mut issues = Vec::new();
        let count = validate_tasks(content, "plan.aps.md", &mut issues);
        assert_eq!(count, 2);
        assert!(issues.is_empty());
    }

    #[test]
    fn detects_missing_intent() {
        let content = "## Tasks\n\n### TEST-001: task without intent\n\n- **Status:** Proposed\n";
        let mut issues = Vec::new();
        validate_tasks(content, "plan.aps.md", &mut issues);
        assert!(issues.iter().any(|i| i.rule == "task-intent-required"));
    }

    #[test]
    fn detects_duplicate_task_ids() {
        let content = "## Tasks\n\n### TEST-001: first\n\n- **Intent:** A\n\n### TEST-001: duplicate\n\n- **Intent:** B\n";
        let mut issues = Vec::new();
        validate_tasks(content, "plan.aps.md", &mut issues);
        assert!(issues.iter().any(|i| i.rule == "duplicate-task-id"));
    }

    #[test]
    fn warns_on_malformed_task_id() {
        let content = "## Tasks\n\n### TEST-01: short number\n\n- **Intent:** A\n";
        let mut issues = Vec::new();
        validate_tasks(content, "plan.aps.md", &mut issues);
        assert!(issues.iter().any(|i| i.rule == "task-id-format"));
    }

    #[test]
    fn warns_on_broken_dependency() {
        let content =
            "## Tasks\n\n### TEST-001: task\n\n- **Intent:** A\n- **Dependencies:** TEST-999\n";
        let mut issues = Vec::new();
        validate_tasks(content, "plan.aps.md", &mut issues);
        assert!(issues.iter().any(|i| i.rule == "broken-dependency"));
    }

    #[test]
    fn ignores_h4_phase_headings() {
        // H4 headings (####) are not matched by the H3 regex — they are skipped entirely.
        let content = "## Tasks\n\n#### Phase 1 — Setup\n\n### TEST-001: task\n\n- **Intent:** A\n";
        let mut issues = Vec::new();
        let count = validate_tasks(content, "plan.aps.md", &mut issues);
        assert_eq!(count, 1);
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn h3_non_task_heading_does_not_produce_error() {
        // H3 heading without a task-ID-like pattern should not trigger task-id-format warning.
        let content = "## Tasks\n\n### Overview\n\nSome overview text.\n\n### TEST-001: task\n\n- **Intent:** A\n";
        let mut issues = Vec::new();
        let count = validate_tasks(content, "plan.aps.md", &mut issues);
        assert_eq!(count, 1);
        // "Overview" has no hyphen + uppercase start, so no task-id-format warning.
        assert!(!issues.iter().any(|i| i.rule == "task-id-format"));
    }

    // ── Hash verification ───────────────────────────────────────

    #[test]
    fn verify_hash_returns_none_when_no_hash() {
        assert_eq!(verify_content_hash("# Plan\n\nNo hash here"), None);
    }

    #[test]
    fn verify_hash_detects_valid_hash() {
        // Compute the expected hash of the content without the hash comment.
        // The hash is appended with a newline, so the body after removal is
        // "{body}\n" — we must hash exactly that.
        let body = "# Plan\n\nContent here";
        let body_with_newline = format!("{body}\n");
        let mut hasher = Sha256::new();
        hasher.update(body_with_newline.as_bytes());
        let hash = hex::encode(hasher.finalize());

        let content = format!("{body}\n<!-- hash: {hash} -->");
        assert_eq!(verify_content_hash(&content), Some(true));
    }

    #[test]
    fn verify_hash_detects_tampered_content() {
        let content = "# Plan\n\nContent here\n<!-- hash: 0000000000000000000000000000000000000000000000000000000000000000 -->";
        assert_eq!(verify_content_hash(content), Some(false));
    }

    // ── Path safety ─────────────────────────────────────────────

    #[test]
    #[cfg(unix)]
    fn rejects_absolute_module_paths() {
        let content = "## Modules\n\n- [mod](/etc/passwd.aps.md)\n";
        let mut issues = Vec::new();
        validate_index_structure(content, "plan.aps.md", &mut issues);
        assert!(issues.iter().any(|i| i.rule == "path-safety"));
    }

    #[test]
    fn rejects_parent_directory_escapes() {
        let content = "## Modules\n\n- [mod](../../secret.aps.md)\n";
        let mut issues = Vec::new();
        validate_index_structure(content, "plan.aps.md", &mut issues);
        assert!(issues.iter().any(|i| i.rule == "path-safety"));
    }

    // ── Clap argument parsing ───────────────────────────────────

    #[test]
    fn clap_parses_validate_with_plan_arg() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "validate", "plan.aps.md"]);
        assert!(result.is_ok());
    }

    #[test]
    fn clap_rejects_validate_without_plan() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "validate"]);
        assert!(result.is_err());
    }

    #[test]
    fn clap_parses_validate_with_format_override() {
        use clap::Parser;
        let result =
            crate::Cli::try_parse_from(["anvil", "validate", "spec.md", "--format", "speckit"]);
        assert!(result.is_ok());
    }
}
