use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
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
    #[serde(skip)]
    pub(crate) body: String,
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

#[allow(clippy::too_many_lines)]
pub fn build_plan_status_snapshot(repo_root: &Path) -> Result<PlanStatusSnapshot> {
    let index_path = repo_root.join("plans/index.aps.md");
    let index = fs::read_to_string(&index_path)
        .with_context(|| format!("failed to read {}", index_path.display()))?;

    let mut modules = parse_index_modules(&index);
    let mut work_items = Vec::new();
    let mut warnings = Vec::new();

    for module in &modules {
        let module_path = repo_root.join("plans").join(&module.path);
        let Ok(contents) = fs::read_to_string(&module_path) else {
            warnings.push(PlanWarning {
                kind: PlanWarningKind::MissingModulePath,
                module: Some(module.scope.clone()),
                work_item: None,
                message: format!("module path is missing: {}", module.path.display()),
            });
            continue;
        };

        let parsed_items = parse_work_items(&module.scope, &contents);
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
        repo_root: repo_root.to_path_buf(),
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

pub(crate) fn is_done_status(status: &str) -> bool {
    matches!(
        status_token(status).as_str(),
        "done" | "complete" | "completed" | "merged" | "released" | "released/shipped" | "archived"
    )
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
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    fn write(path: impl AsRef<std::path::Path>, contents: &str) {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn fixture_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        write(
            dir.path().join("plans/index.aps.md"),
            r"# Test Plan

## Engineering Platform

| Module | Scope | Status | Progress | Notes |
| ------ | ----- | ------ | -------- | ----- |
| [aps-canonical-alignment](./modules/aps-canonical-alignment.aps.md) | APSCAN | In Progress | 1/2 | Active APS migration work. |
",
        );
        write(
            dir.path()
                .join("plans/modules/aps-canonical-alignment.aps.md"),
            r"# APS Canonical Alignment

| ID | Owner | Status | Progress |
| -- | ----- | ------ | -------- |
| APSCAN | — | In Progress | 1/2 |

## Work Items

### APSCAN-001: Done item

- **Status:** Done
- **Validation:** `pnpm test:aps-active-lint`

### APSCAN-002: Ready item

- **Status:** Ready
- **Validation:** `pnpm docs:check`
- **Dependencies:** APSCAN-001
",
        );
        dir
    }

    #[test]
    fn loads_index_modules() {
        let repo = fixture_repo();

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert_eq!(snapshot.modules.len(), 1);
        assert_eq!(snapshot.modules[0].scope, "APSCAN");
        assert_eq!(snapshot.modules[0].done, Some(1));
        assert_eq!(snapshot.modules[0].total, Some(2));
        assert_eq!(snapshot.work_items.len(), 2);
        assert!(snapshot.enrichments.is_empty());
    }

    #[test]
    fn loads_est_tasks_index_tables() {
        let repo = fixture_repo();
        write(
            repo.path().join("plans/index.aps.md"),
            r"# Test Plan

| Module | Scope | Est. Tasks | Dependencies |
| ------ | ----- | ---------- | ------------ |
| [aps-canonical-alignment](./modules/aps-canonical-alignment.aps.md) | APSCAN | 1/2 | Migration work — **In Progress**. |
",
        );

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert_eq!(snapshot.modules.len(), 1);
        assert_eq!(snapshot.modules[0].status, "In Progress");
        assert_eq!(snapshot.modules[0].done, Some(1));
        assert_eq!(snapshot.modules[0].total, Some(2));
    }

    #[test]
    fn skips_archived_index_modules() {
        let repo = fixture_repo();
        write(
            repo.path().join("plans/index.aps.md"),
            r"# Test Plan

| Module | Scope | Status | Progress | Notes |
| ------ | ----- | ------ | -------- | ----- |
| [old](./archive/modules/old.aps.md) | OLD | Complete | 1/1 | Historical. |
| [aps-canonical-alignment](./modules/aps-canonical-alignment.aps.md) | APSCAN | In Progress | 1/2 | Active. |
",
        );

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert_eq!(snapshot.modules.len(), 1);
        assert_eq!(snapshot.modules[0].scope, "APSCAN");
    }

    #[test]
    fn detects_index_module_count_mismatch() {
        let repo = fixture_repo();
        write(
            repo.path().join("plans/index.aps.md"),
            r"# Test Plan

| Module | Scope | Status | Progress | Notes |
| ------ | ----- | ------ | -------- | ----- |
| [aps-canonical-alignment](./modules/aps-canonical-alignment.aps.md) | APSCAN | In Progress | 0/2 | Active APS migration work. |
",
        );

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert!(snapshot.warnings.iter().any(|warning| {
            warning.module.as_deref() == Some("APSCAN")
                && warning.kind == PlanWarningKind::ProgressMismatch
        }));
    }

    #[test]
    fn detects_missing_module_path() {
        let repo = fixture_repo();
        write(
            repo.path().join("plans/index.aps.md"),
            r"# Test Plan

| Module | Scope | Status | Progress | Notes |
| ------ | ----- | ------ | -------- | ----- |
| [missing](./modules/missing.aps.md) | MISS | In Progress | 0/1 | Missing. |
",
        );

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert!(snapshot.warnings.iter().any(|warning| {
            warning.module.as_deref() == Some("MISS")
                && warning.kind == PlanWarningKind::MissingModulePath
        }));
    }

    #[test]
    fn detects_open_item_without_validation() {
        let repo = fixture_repo();
        write(
            repo.path()
                .join("plans/modules/aps-canonical-alignment.aps.md"),
            r"# APS Canonical Alignment

| ID | Owner | Status | Progress |
| -- | ----- | ------ | -------- |
| APSCAN | — | In Progress | 0/1 |

## Work Items

### APSCAN-002: Ready item

- **Status:** Ready
",
        );

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert!(snapshot.warnings.iter().any(|warning| {
            warning.work_item.as_deref() == Some("APSCAN-002")
                && warning.kind == PlanWarningKind::MissingValidation
        }));
    }

    #[test]
    fn completed_modules_do_not_warn_without_ready_items() {
        let repo = fixture_repo();
        write(
            repo.path()
                .join("plans/modules/aps-canonical-alignment.aps.md"),
            r"# APS Canonical Alignment

| ID | Owner | Status | Progress |
| -- | ----- | ------ | -------- |
| APSCAN | - | Complete | 1/1 |

## Work Items

### APSCAN-001: Done item

- **Status:** Done
- **Validation:** `cargo test`
",
        );

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert!(
            !snapshot
                .warnings
                .iter()
                .any(|warning| warning.kind == PlanWarningKind::NoReadyNextItem)
        );
    }

    #[test]
    fn leaves_enrichments_empty() {
        let repo = fixture_repo();

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert!(snapshot.enrichments.is_empty());
    }

    #[test]
    fn terminal_status_with_prose_is_done() {
        let repo = fixture_repo();
        write(
            repo.path()
                .join("plans/modules/aps-canonical-alignment.aps.md"),
            r#"# APS Canonical Alignment

| ID | Owner | Status | Progress |
| -- | ----- | ------ | -------- |
| APSCAN | — | In Progress | 2/2 |

## Work Items

### APSCAN-001: Merged item

- **Status:** Merged via PR #1900

### APSCAN-002: Released item

- **Status:** Released/Shipped via v0.7.1-beta
"#,
        );

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert!(!snapshot.warnings.iter().any(|warning| {
            warning.kind == PlanWarningKind::MissingValidation
                || warning.kind == PlanWarningKind::NoReadyNextItem
        }));
    }

    #[test]
    fn completed_module_without_ready_item_is_not_stale() {
        let repo = fixture_repo();
        write(
            repo.path().join("plans/index.aps.md"),
            r#"# Test Plan

| Module | Scope | Status | Progress | Notes |
| ------ | ----- | ------ | -------- | ----- |
| [aps-canonical-alignment](./modules/aps-canonical-alignment.aps.md) | APSCAN | Complete | 2/2 | Done. |
"#,
        );
        write(
            repo.path()
                .join("plans/modules/aps-canonical-alignment.aps.md"),
            r#"# APS Canonical Alignment

| ID | Owner | Status | Progress |
| -- | ----- | ------ | -------- |
| APSCAN | — | Complete | 2/2 |

## Work Items

### APSCAN-001: Done item

- **Status:** Done

### APSCAN-002: Done item

- **Status:** Complete
"#,
        );

        let snapshot = build_plan_status_snapshot(repo.path()).unwrap();

        assert!(
            !snapshot
                .warnings
                .iter()
                .any(|warning| warning.kind == PlanWarningKind::NoReadyNextItem)
        );
    }
}
