//! Filesystem adapter for the pure APS plan read model.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

pub use anvil_plan_read_model::PlanStatusSnapshot;
pub(crate) use anvil_plan_read_model::is_done_status;
#[cfg(test)]
pub use anvil_plan_read_model::{ModuleSummary, PlanWarning, PlanWarningKind, WorkItemSummary};

/// Load the APS index and module sources from a repository, then delegate all
/// parsing and read-model construction to `anvil-plan-read-model`.
pub fn build_plan_status_snapshot(repo_root: &Path) -> Result<PlanStatusSnapshot> {
    let index_path = repo_root.join("plans/index.aps.md");
    let index = fs::read_to_string(&index_path)
        .with_context(|| format!("failed to read {}", index_path.display()))?;
    let plans_root = repo_root.join("plans");

    Ok(
        anvil_plan_read_model::build_plan_status_snapshot_from_sources(
            repo_root.to_path_buf(),
            &index,
            |relative_path| fs::read_to_string(plans_root.join(relative_path)).ok(),
        ),
    )
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
    fn falls_back_to_module_header_for_estimated_task_index_rows() {
        let repo = fixture_repo();
        write(
            repo.path().join("plans/index.aps.md"),
            r"# Test Plan

| Module | Scope | Est. Tasks | Dependencies |
| ------ | ----- | ---------- | ------------ |
| [aps-canonical-alignment](./modules/aps-canonical-alignment.aps.md) | APSCAN | 2 | architecture-safety |
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
