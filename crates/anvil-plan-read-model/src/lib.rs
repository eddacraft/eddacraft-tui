//! Pure APS index and module read model.
//!
//! This crate parses caller-supplied content only. Filesystem access, path
//! containment, size limits, and symlink policy belong to the calling adapter.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct PlanStatusSnapshot {
    pub repo_root: PathBuf,
    pub modules: Vec<ModuleSummary>,
    pub work_items: Vec<WorkItemSummary>,
    pub warnings: Vec<PlanWarning>,
    pub enrichments: Vec<PlanEnrichment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleSummary {
    pub scope: String,
    pub title: String,
    pub path: PathBuf,
    pub status: String,
    pub done: Option<usize>,
    pub total: Option<usize>,
    pub section: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkItemSummary {
    pub id: String,
    pub title: String,
    pub module: String,
    pub status: String,
    pub validation: Option<String>,
    pub dependencies: Vec<String>,
    pub files: Vec<String>,
    /// Parser-only prose retained for warning derivation and omitted from the
    /// serialised read model. Public so presentation-adapter fixtures can build
    /// the same value without duplicating the type.
    #[doc(hidden)]
    #[serde(skip)]
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanWarning {
    pub kind: PlanWarningKind,
    pub module: Option<String>,
    pub work_item: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PlanWarningKind {
    ProgressMismatch,
    MissingModulePath,
    MissingValidation,
    InProgressAllDone,
    MergedProseOpenStatus,
    BlockedDependencyComplete,
    NoReadyNextItem,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PlanEnrichment;

#[derive(Debug, Clone, Copy)]
pub struct PlanReadLimits {
    pub max_modules: usize,
    pub max_work_items: usize,
    pub max_source_bytes: usize,
    pub max_module_reads: usize,
}

impl PlanReadLimits {
    pub const UNBOUNDED: Self = Self {
        max_modules: usize::MAX,
        max_work_items: usize::MAX,
        max_source_bytes: usize::MAX,
        max_module_reads: usize::MAX,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanReadLimitError {
    ModuleCount,
    WorkItemCount,
    SourceBytes,
    ModuleReads,
}

impl fmt::Display for PlanReadLimitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let boundary = match self {
            Self::ModuleCount => "module count",
            Self::WorkItemCount => "work-item count",
            Self::SourceBytes => "aggregate source bytes",
            Self::ModuleReads => "module read count",
        };
        write!(formatter, "plan {boundary} exceeds the configured limit")
    }
}

impl std::error::Error for PlanReadLimitError {}

/// Build a plan snapshot from caller-supplied, already-bounded APS content.
///
/// `read_module` receives each module path exactly as declared beneath
/// `plans/` in the index (for example `modules/dashboard.aps.md`). The caller
/// owns filesystem access, containment checks, size limits, and symlink policy.
/// Returning `None` records the same `MissingModulePath` warning as the legacy
/// CLI adapter did for an unreadable or missing module.
#[allow(clippy::too_many_lines)]
pub fn build_plan_status_snapshot_from_sources<F>(
    repo_root: PathBuf,
    index: &str,
    read_module: F,
) -> PlanStatusSnapshot
where
    F: FnMut(&Path) -> Option<String>,
{
    build_bounded_plan_status_snapshot_from_sources(
        repo_root,
        index,
        PlanReadLimits::UNBOUNDED,
        read_module,
    )
    .expect("unbounded plan read cannot exceed a configured limit")
}

/// Build a plan snapshot while bounding unique modules, work items, aggregate
/// caller-supplied source bytes, and module read attempts.
#[allow(clippy::too_many_lines)]
pub fn build_bounded_plan_status_snapshot_from_sources<F>(
    repo_root: PathBuf,
    index: &str,
    limits: PlanReadLimits,
    mut read_module: F,
) -> Result<PlanStatusSnapshot, PlanReadLimitError>
where
    F: FnMut(&Path) -> Option<String>,
{
    let mut modules = parse_index_modules(index);
    let mut unique_paths = BTreeSet::new();
    modules.retain(|module| unique_paths.insert(module.path.clone()));
    if modules.len() > limits.max_modules {
        return Err(PlanReadLimitError::ModuleCount);
    }
    let mut source_bytes = index.len();
    if source_bytes > limits.max_source_bytes {
        return Err(PlanReadLimitError::SourceBytes);
    }
    let mut module_reads = 0usize;
    let mut work_items = Vec::new();
    let mut warnings = Vec::new();

    for module in &mut modules {
        module_reads = module_reads
            .checked_add(1)
            .ok_or(PlanReadLimitError::ModuleReads)?;
        if module_reads > limits.max_module_reads {
            return Err(PlanReadLimitError::ModuleReads);
        }
        let Some(contents) = read_module(&module.path) else {
            warnings.push(PlanWarning {
                kind: PlanWarningKind::MissingModulePath,
                module: Some(module.scope.clone()),
                work_item: None,
                message: format!("module path is missing: {}", module.path.display()),
            });
            continue;
        };
        source_bytes = source_bytes
            .checked_add(contents.len())
            .ok_or(PlanReadLimitError::SourceBytes)?;
        if source_bytes > limits.max_source_bytes {
            return Err(PlanReadLimitError::SourceBytes);
        }

        if let Some(header) = parse_module_header(&module.scope, &contents) {
            if module.status.is_empty() || module.status == "Unknown" {
                module.status = header.status;
            }
            if module.done.is_none() {
                module.done = header.done;
                module.total = header.total;
            }
        }

        let parsed_items = parse_work_items(&module.scope, &contents);
        if work_items
            .len()
            .checked_add(parsed_items.len())
            .is_none_or(|count| count > limits.max_work_items)
        {
            return Err(PlanReadLimitError::WorkItemCount);
        }
        let done = parsed_items
            .iter()
            .filter(|item| is_done(&item.status))
            .count();
        let total = parsed_items.len();

        if total > 0
            && (module.done.is_some_and(|module_done| module_done != done)
                || module
                    .total
                    .is_some_and(|module_total| module_total != total))
        {
            warnings.push(PlanWarning {
                kind: PlanWarningKind::ProgressMismatch,
                module: Some(module.scope.clone()),
                work_item: None,
                message: format!(
                    "index progress is {}/{}, parsed module progress is {}/{}",
                    module.done.unwrap_or_default(),
                    module.total.unwrap_or_default(),
                    done,
                    total
                ),
            });
        }

        if total > 0 && done == total && module.status.eq_ignore_ascii_case("in progress") {
            warnings.push(PlanWarning {
                kind: PlanWarningKind::InProgressAllDone,
                module: Some(module.scope.clone()),
                work_item: None,
                message: "module is in progress but every parsed work item is done".to_string(),
            });
        }

        if module.status.eq_ignore_ascii_case("in progress")
            && done < total
            && !parsed_items.iter().any(|item| is_ready(&item.status))
        {
            warnings.push(PlanWarning {
                kind: PlanWarningKind::NoReadyNextItem,
                module: Some(module.scope.clone()),
                work_item: None,
                message: "module has active work but no ready next item".to_string(),
            });
        }

        let completed_ids: Vec<_> = parsed_items
            .iter()
            .filter(|item| is_done(&item.status))
            .map(|item| item.id.clone())
            .collect();

        for item in &parsed_items {
            if !is_done(&item.status) && item.validation.is_none() {
                warnings.push(PlanWarning {
                    kind: PlanWarningKind::MissingValidation,
                    module: Some(module.scope.clone()),
                    work_item: Some(item.id.clone()),
                    message: "open work item has no validation command".to_string(),
                });
            }

            if !is_done(&item.status) && item.body.to_ascii_lowercase().contains("merged") {
                warnings.push(PlanWarning {
                    kind: PlanWarningKind::MergedProseOpenStatus,
                    module: Some(module.scope.clone()),
                    work_item: Some(item.id.clone()),
                    message: "work item mentions merged state but is still open".to_string(),
                });
            }

            if status_token(&item.status) == "blocked"
                && !item.dependencies.is_empty()
                && item
                    .dependencies
                    .iter()
                    .all(|dependency| dependency_references_completed(dependency, &completed_ids))
            {
                warnings.push(PlanWarning {
                    kind: PlanWarningKind::BlockedDependencyComplete,
                    module: Some(module.scope.clone()),
                    work_item: Some(item.id.clone()),
                    message: "blocked item depends on a completed item".to_string(),
                });
            }
        }

        work_items.extend(parsed_items);
    }

    // Keep the future enrichment seam explicit but empty in the APS-only v1.
    let enrichments = Vec::new();

    // Stable row order helps tests and later non-interactive renderers.
    modules.sort_by(|left, right| left.scope.cmp(&right.scope));

    Ok(PlanStatusSnapshot {
        repo_root,
        modules,
        work_items,
        warnings,
        enrichments,
    })
}

fn parse_index_modules(index: &str) -> Vec<ModuleSummary> {
    let mut section = None;
    let mut headers: Vec<String> = Vec::new();
    let mut modules = Vec::new();

    for line in index.lines() {
        if let Some(heading) = line
            .strip_prefix("### ")
            .or_else(|| line.strip_prefix("## "))
        {
            section = Some(heading.trim().to_string());
            continue;
        }

        if !line.trim_start().starts_with('|') {
            continue;
        }

        let cells = table_cells(line);
        if cells.iter().all(|cell| cell.chars().all(|c| c == '-')) {
            continue;
        }
        if cells
            .first()
            .is_some_and(|cell| cell.eq_ignore_ascii_case("module"))
        {
            headers = cells.iter().map(|cell| normalise_header(cell)).collect();
            continue;
        }
        if cells.len() < 2 || !line.contains("](.") {
            continue;
        }

        let module_index = header_index(&headers, "module").unwrap_or(0);
        let Some(module_cell) = cells.get(module_index) else {
            continue;
        };

        let Some((title, path)) = parse_markdown_link(module_cell) else {
            continue;
        };
        let relative_path = path.trim_start_matches("./");
        if !relative_path.starts_with("modules/") {
            continue;
        }

        let progress_index =
            header_index(&headers, "progress").or_else(|| header_index(&headers, "esttasks"));
        let status_index = header_index(&headers, "status");
        let notes_index =
            header_index(&headers, "notes").or_else(|| header_index(&headers, "dependencies"));
        let notes = notes_index.and_then(|index| cells.get(index).cloned());
        let status = status_index
            .and_then(|index| cells.get(index).cloned())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                notes
                    .as_deref()
                    .and_then(infer_status)
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| "Unknown".to_string());
        let (done, total) = progress_index
            .and_then(|index| cells.get(index))
            .map_or((None, None), |value| parse_progress(value));

        modules.push(ModuleSummary {
            scope: header_index(&headers, "scope")
                .and_then(|index| cells.get(index).cloned())
                .unwrap_or_default(),
            title,
            path: PathBuf::from(relative_path),
            status,
            done,
            total,
            section: section.clone(),
            notes,
        });
    }

    modules
}

fn normalise_header(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

fn header_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|header| header == name)
}

fn infer_status(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("in progress") {
        Some("In Progress")
    } else if lower.contains("ready") {
        Some("Ready")
    } else if lower.contains("blocked") {
        Some("Blocked")
    } else if lower.contains("complete") || lower.contains("archived") {
        Some("Complete")
    } else if lower.contains("draft") {
        Some("Draft")
    } else {
        None
    }
}

#[derive(Debug)]
struct ModuleHeader {
    status: String,
    done: Option<usize>,
    total: Option<usize>,
}

fn parse_module_header(scope: &str, contents: &str) -> Option<ModuleHeader> {
    let mut headers: Vec<String> = Vec::new();

    for line in contents.lines() {
        if !line.trim_start().starts_with('|') {
            if !headers.is_empty() {
                break;
            }
            continue;
        }

        let cells = table_cells(line);
        if cells.iter().all(|cell| cell.chars().all(|c| c == '-')) {
            continue;
        }
        if cells
            .first()
            .is_some_and(|cell| cell.eq_ignore_ascii_case("id"))
        {
            headers = cells.iter().map(|cell| normalise_header(cell)).collect();
            continue;
        }

        let Some(id_index) = header_index(&headers, "id") else {
            continue;
        };
        if cells.get(id_index).is_none_or(|cell| cell != scope) {
            continue;
        }

        let (done, total) = header_index(&headers, "progress")
            .and_then(|index| cells.get(index))
            .map_or((None, None), |value| parse_progress(value));

        return Some(ModuleHeader {
            status: header_index(&headers, "status")
                .and_then(|index| cells.get(index))
                .cloned()
                .unwrap_or_default(),
            done,
            total,
        });
    }

    None
}

fn parse_work_items(module: &str, contents: &str) -> Vec<WorkItemSummary> {
    let mut items = Vec::new();
    let mut current: Option<WorkItemSummary> = None;

    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("### ")
            && let Some((id, title)) = rest.split_once(':')
        {
            if let Some(item) = current.take() {
                items.push(item);
            }
            current = Some(WorkItemSummary {
                id: id.trim().to_string(),
                title: title.trim().to_string(),
                module: module.to_string(),
                status: "open".to_string(),
                validation: None,
                dependencies: Vec::new(),
                files: Vec::new(),
                body: String::new(),
            });
            continue;
        }

        let Some(item) = current.as_mut() else {
            continue;
        };

        item.body.push_str(line);
        item.body.push('\n');

        if let Some(value) = parse_field(line, "Status") {
            item.status = value;
        } else if let Some(value) = parse_field(line, "Validation") {
            item.validation = Some(value);
        } else if let Some(value) = parse_field(line, "Dependencies") {
            item.dependencies = split_inline_list(&value);
        } else if let Some(value) = parse_field(line, "Files") {
            item.files = split_inline_list(&value);
        }
    }

    if let Some(item) = current {
        items.push(item);
    }

    items
}

fn parse_field(line: &str, name: &str) -> Option<String> {
    let trimmed = line.trim().trim_start_matches('-').trim();
    let bold_prefix = format!("**{name}:**");
    let plain_prefix = format!("{name}:");

    trimmed
        .strip_prefix(&bold_prefix)
        .or_else(|| trimmed.strip_prefix(&plain_prefix))
        .map(|value| value.trim().trim_matches('`').to_string())
        .filter(|value| !value.is_empty())
}

fn table_cells(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn parse_markdown_link(value: &str) -> Option<(String, String)> {
    let label_start = value.find('[')? + 1;
    let label_end = value[label_start..].find(']')? + label_start;
    let path_start = value[label_end..].find('(')? + label_end + 1;
    let path_end = value[path_start..].find(')')? + path_start;
    Some((
        value[label_start..label_end].to_string(),
        value[path_start..path_end].to_string(),
    ))
}

fn parse_progress(value: &str) -> (Option<usize>, Option<usize>) {
    let Some((left, right)) = value.split_once('/') else {
        return (
            None,
            value
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok(),
        );
    };

    let done = left
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok();
    let total = right
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok();
    (done, total)
}

fn split_inline_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn is_done_status(status: &str) -> bool {
    matches!(
        status_token(status).as_str(),
        "done" | "complete" | "completed" | "merged" | "released" | "released/shipped" | "archived"
    )
}

/// Extract a second-level Markdown section from caller-supplied APS content.
///
/// Presentation adapters use this helper so section parsing remains owned by
/// the pure read model rather than being reimplemented at each filesystem
/// boundary.
pub fn extract_markdown_section(source: &str, heading: &str) -> String {
    let marker = format!("## {heading}");
    let Some((_, section)) = source.split_once(&marker) else {
        return String::new();
    };
    let section = section.split("\n## ").next().unwrap_or(section);
    section
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_done(status: &str) -> bool {
    is_done_status(status)
}

fn is_ready(status: &str) -> bool {
    matches!(status_token(status).as_str(), "ready" | "open")
}

fn status_token(status: &str) -> String {
    let lower = status.trim().to_ascii_lowercase();
    if lower.starts_with("released/shipped") {
        return "released/shipped".to_string();
    }

    lower
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '(' | '-' | '—' | ':'))
        .next()
        .unwrap_or_default()
        .to_string()
}

fn dependency_references_completed(dependency: &str, completed_ids: &[String]) -> bool {
    dependency
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
        .any(|token| completed_ids.iter().any(|id| token == id))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use super::*;

    const INDEX: &str = r"# Test Plan

| Module | Scope | Status | Progress | Notes |
| ------ | ----- | ------ | -------- | ----- |
| [dashboard](./modules/dashboard.aps.md) | DASH | In Progress | 0/1 | Active. |
";

    const MODULE: &str = r"# Dashboard

| ID | Owner | Status | Progress |
| -- | ----- | ------ | -------- |
| DASH | - | In Progress | 0/1 |

### DASH-001: Build the dashboard

- **Status:** Ready
- **Validation:** `cargo test`
";

    #[test]
    fn builds_snapshot_from_bounded_in_memory_sources() {
        let modules = BTreeMap::from([(
            PathBuf::from("modules/dashboard.aps.md"),
            MODULE.to_string(),
        )]);

        let snapshot = build_plan_status_snapshot_from_sources(
            PathBuf::from("/workspace"),
            INDEX,
            |path: &Path| modules.get(path).cloned(),
        );

        assert_eq!(snapshot.modules[0].scope, "DASH");
        assert_eq!(snapshot.work_items[0].id, "DASH-001");
    }

    #[test]
    fn missing_bounded_source_becomes_a_warning() {
        let snapshot =
            build_plan_status_snapshot_from_sources(PathBuf::from("/workspace"), INDEX, |_path| {
                None
            });

        assert!(snapshot.warnings.iter().any(|warning| {
            warning.kind == PlanWarningKind::MissingModulePath
                && warning.module.as_deref() == Some("DASH")
        }));
    }

    #[test]
    fn bounded_snapshot_enforces_work_item_and_aggregate_byte_limits() {
        let work_item_error = build_bounded_plan_status_snapshot_from_sources(
            PathBuf::from("/workspace"),
            INDEX,
            PlanReadLimits {
                max_modules: 1,
                max_work_items: 0,
                max_source_bytes: usize::MAX,
                max_module_reads: 1,
            },
            |_path| Some(MODULE.to_owned()),
        )
        .expect_err("work-item count must be bounded");
        assert_eq!(work_item_error, PlanReadLimitError::WorkItemCount);

        let byte_error = build_bounded_plan_status_snapshot_from_sources(
            PathBuf::from("/workspace"),
            INDEX,
            PlanReadLimits {
                max_modules: 1,
                max_work_items: 1,
                max_source_bytes: INDEX.len() + MODULE.len() - 1,
                max_module_reads: 1,
            },
            |_path| Some(MODULE.to_owned()),
        )
        .expect_err("aggregate source bytes must be bounded");
        assert_eq!(byte_error, PlanReadLimitError::SourceBytes);
    }

    #[test]
    fn bounded_snapshot_deduplicates_paths_before_enforcing_read_budget() {
        let duplicate_index = format!(
            "{INDEX}| [dashboard duplicate](./modules/dashboard.aps.md) | DASH2 | Ready | 0/1 | Duplicate |\n"
        );
        let mut reads = 0;
        let snapshot = build_bounded_plan_status_snapshot_from_sources(
            PathBuf::from("/workspace"),
            &duplicate_index,
            PlanReadLimits {
                max_modules: 1,
                max_work_items: 1,
                max_source_bytes: usize::MAX,
                max_module_reads: 1,
            },
            |_path| {
                reads += 1;
                Some(MODULE.to_owned())
            },
        )
        .expect("duplicate paths share one bounded read");

        assert_eq!(reads, 1);
        assert_eq!(snapshot.modules.len(), 1);
        assert_eq!(snapshot.work_items.len(), 1);
    }

    #[test]
    fn bounded_snapshot_enforces_module_read_budget() {
        let second_module = INDEX.replace(
            "| [dashboard](./modules/dashboard.aps.md) | DASH |",
            "| [dashboard](./modules/dashboard.aps.md) | DASH |\n| [other](./modules/other.aps.md) | OTHER |",
        );
        let error = build_bounded_plan_status_snapshot_from_sources(
            PathBuf::from("/workspace"),
            &second_module,
            PlanReadLimits {
                max_modules: 2,
                max_work_items: usize::MAX,
                max_source_bytes: usize::MAX,
                max_module_reads: 1,
            },
            |_path| Some(MODULE.to_owned()),
        )
        .expect_err("module read count must be bounded");

        assert_eq!(error, PlanReadLimitError::ModuleReads);
    }
}
