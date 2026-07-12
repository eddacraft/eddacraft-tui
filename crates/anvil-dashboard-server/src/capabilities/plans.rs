use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anvil_plan_read_model::{
    PlanReadLimitError, PlanReadLimits, PlanStatusSnapshot,
    build_bounded_plan_status_snapshot_from_sources, extract_markdown_section,
};
use thiserror::Error;

use crate::api::{PlanDetail, PlanSummary, PlanTimelineEntry};
use crate::{Workspace, WorkspaceReadError};

const PLAN_INDEX: &str = "plans/index.aps.md";
pub const MAX_PLAN_MODULES: usize = 256;
pub const MAX_PLAN_WORK_ITEMS: usize = 4096;
pub const MAX_PLAN_SOURCE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum PlanReadError {
    #[error("plan id is invalid")]
    InvalidId,
    #[error("plan data is not valid UTF-8")]
    InvalidUtf8,
    #[error(transparent)]
    Limit(#[from] PlanReadLimitError),
    #[error(transparent)]
    Workspace(#[from] WorkspaceReadError),
}

pub fn load_plans(workspace: &Workspace) -> Result<Vec<PlanSummary>, PlanReadError> {
    let (snapshot, _) = load_snapshot(workspace)?;
    Ok(snapshot.modules.iter().filter_map(plan_summary).collect())
}

pub fn load_plan(workspace: &Workspace, id: &str) -> Result<Option<PlanDetail>, PlanReadError> {
    validate_plan_id(id)?;
    let (snapshot, sources) = load_snapshot(workspace)?;
    let Some(module) = snapshot
        .modules
        .iter()
        .find(|module| module_id(&module.path).is_some_and(|candidate| candidate == id))
    else {
        return Ok(None);
    };
    let Some(summary) = plan_summary(module) else {
        return Ok(None);
    };
    let source = sources
        .borrow()
        .get(&module.path)
        .cloned()
        .unwrap_or_default();
    let timeline = snapshot
        .work_items
        .iter()
        .filter(|item| item.module == module.scope)
        .map(|item| PlanTimelineEntry {
            id: item.id.clone(),
            title: item.title.clone(),
            status: item.status.clone(),
            validation_contract: item.validation.clone(),
            readiness: item.status.to_ascii_lowercase().starts_with("ready"),
        })
        .collect();
    Ok(Some(PlanDetail::read_only(
        summary,
        extract_markdown_section(&source, "Purpose"),
        timeline,
    )))
}

fn load_snapshot(
    workspace: &Workspace,
) -> Result<(PlanStatusSnapshot, RefCell<BTreeMap<PathBuf, String>>), PlanReadError> {
    let index = read_text(workspace, Path::new(PLAN_INDEX))?;
    let sources = RefCell::new(BTreeMap::new());
    let snapshot = build_bounded_plan_status_snapshot_from_sources(
        workspace.root().to_path_buf(),
        &index,
        PlanReadLimits {
            max_modules: MAX_PLAN_MODULES,
            max_work_items: MAX_PLAN_WORK_ITEMS,
            max_source_bytes: MAX_PLAN_SOURCE_BYTES,
            max_module_reads: MAX_PLAN_MODULES,
        },
        |relative_path| {
            let source = read_text(workspace, &Path::new("plans").join(relative_path)).ok()?;
            sources
                .borrow_mut()
                .insert(relative_path.to_path_buf(), source.clone());
            Some(source)
        },
    )?;
    Ok((snapshot, sources))
}

fn plan_summary(module: &anvil_plan_read_model::ModuleSummary) -> Option<PlanSummary> {
    let id = module_id(&module.path)?.to_owned();
    let progress = match (module.done, module.total) {
        (Some(done), Some(total)) => format!("{done}/{total}"),
        _ => "-".to_owned(),
    };
    Some(PlanSummary {
        id,
        scope: module.scope.clone(),
        title: module.title.clone(),
        status: module.status.clone(),
        progress,
    })
}

fn module_id(path: &Path) -> Option<&str> {
    path.file_name()?.to_str()?.strip_suffix(".aps.md")
}

fn validate_plan_id(id: &str) -> Result<(), PlanReadError> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(PlanReadError::InvalidId);
    }
    Ok(())
}

fn read_text(workspace: &Workspace, path: &Path) -> Result<String, PlanReadError> {
    String::from_utf8(workspace.read(path)?).map_err(|_| PlanReadError::InvalidUtf8)
}
