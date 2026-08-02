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
    // Prefer a fresh read via the stable module id path
    // (`plans/modules/<id>.aps.md`). Fall back to the snapshot map when the
    // re-read misses (key normalisation), never silently empty Purpose.
    // Use slash-only relative strings so Windows WorkspaceAnchor accepts them
    // even if a caller built PathBufs via Path::join (see Workspace::read).
    let by_id_rel = format!("plans/modules/{id}.aps.md");
    let module_rel = format!(
        "plans/{}",
        module
            .path
            .to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches("./")
    );
    let by_id_key = PathBuf::from(format!("modules/{id}.aps.md"));
    let normalised = PathBuf::from(module.path.to_string_lossy().replace('\\', "/"));
    let source = read_text(workspace, Path::new(&by_id_rel))
        .ok()
        .or_else(|| {
            let map = sources.borrow();
            map.get(&module.path)
                .or_else(|| map.get(&normalised))
                .or_else(|| map.get(&by_id_key))
                .cloned()
                .or_else(|| {
                    map.iter().find_map(|(path, contents)| {
                        path_keys_match(path, &module.path).then(|| contents.clone())
                    })
                })
        })
        .or_else(|| read_text(workspace, Path::new(&module_rel)).ok())
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
            // Slash-only wire path — never Path::join (Windows `\` is refused).
            let rel = format!(
                "plans/{}",
                relative_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .trim_start_matches("./")
            );
            let source = read_text(workspace, Path::new(&rel)).ok()?;
            // Store with forward-slash keys so Windows `\` PathBuf lookups still hit.
            let key = PathBuf::from(relative_path.to_string_lossy().replace('\\', "/"));
            sources.borrow_mut().insert(key, source.clone());
            Some(source)
        },
    )?;
    Ok((snapshot, sources))
}

fn path_keys_match(left: &Path, right: &Path) -> bool {
    if left.as_os_str() == right.as_os_str() || left.file_name() == right.file_name() {
        return true;
    }
    // Compare component-wise so `modules/foo` and `modules\foo` match.
    left.components().eq(right.components())
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
    // Defensive: even if a caller passes a Path::join result, keep the wire
    // slash-only. Workspace::read also normalises; this keeps map keys tidy.
    let wire = path.to_string_lossy().replace('\\', "/");
    String::from_utf8(workspace.read(Path::new(&wire))?).map_err(|_| PlanReadError::InvalidUtf8)
}
