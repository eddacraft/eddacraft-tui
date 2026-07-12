use std::collections::BTreeSet;
use std::path::Path;

use thiserror::Error;

use crate::api::{PlanDetail, PlanSummary};
use crate::{Workspace, WorkspaceReadError};

const PLAN_INDEX: &str = "plans/index.aps.md";
const MODULE_LINK_MARKER: &str = "](./modules/";

#[derive(Debug, Error)]
pub enum PlanReadError {
    #[error("plan id is invalid")]
    InvalidId,
    #[error("plan data is not valid UTF-8")]
    InvalidUtf8,
    #[error(transparent)]
    Workspace(#[from] WorkspaceReadError),
}

pub fn load_plans(workspace: &Workspace) -> Result<Vec<PlanSummary>, PlanReadError> {
    let index = read_text(workspace, PLAN_INDEX)?;
    let paths = indexed_module_paths(&index);
    let mut plans = Vec::with_capacity(paths.len());
    for path in paths {
        let source = read_text(workspace, &path)?;
        let id = path
            .strip_prefix("plans/modules/")
            .and_then(|path| path.strip_suffix(".aps.md"))
            .ok_or(PlanReadError::InvalidId)?;
        plans.push(parse_summary(id, &source));
    }
    Ok(plans)
}

pub fn load_plan(workspace: &Workspace, id: &str) -> Result<Option<PlanDetail>, PlanReadError> {
    validate_plan_id(id)?;
    let summary = load_plans(workspace)?
        .into_iter()
        .find(|plan| plan.id == id);
    let Some(summary) = summary else {
        return Ok(None);
    };
    let path = format!("plans/modules/{id}.aps.md");
    let source = read_text(workspace, &path)?;
    Ok(Some(PlanDetail::read_only(
        summary,
        extract_section(&source, "Purpose"),
    )))
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

fn read_text(workspace: &Workspace, path: &str) -> Result<String, PlanReadError> {
    let bytes = workspace.read(Path::new(path))?;
    String::from_utf8(bytes).map_err(|_| PlanReadError::InvalidUtf8)
}

fn indexed_module_paths(index: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for line in index.lines() {
        let mut rest = line;
        while let Some(start) = rest.find(MODULE_LINK_MARKER) {
            rest = &rest[start + MODULE_LINK_MARKER.len()..];
            let Some(end) = rest.find(')') else {
                break;
            };
            let candidate = &rest[..end];
            if candidate.ends_with(".aps.md") && !candidate.contains('/') {
                paths.insert(format!("plans/modules/{candidate}"));
            }
            rest = &rest[end + 1..];
        }
    }
    paths
}

fn parse_summary(id: &str, source: &str) -> PlanSummary {
    let title = source
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .unwrap_or(id)
        .trim()
        .to_owned();
    let mut scope = "-".to_owned();
    let mut status = "Unknown".to_owned();
    let mut progress = "-".to_owned();
    for line in source.lines() {
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let cells = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.len() < 4
            || cells[0].eq_ignore_ascii_case("id")
            || cells[0].chars().all(|character| character == '-')
        {
            continue;
        }
        cells[0].clone_into(&mut scope);
        cells[2].clone_into(&mut status);
        cells[3].clone_into(&mut progress);
        break;
    }
    PlanSummary {
        id: id.to_owned(),
        scope,
        title,
        status,
        progress,
    }
}

fn extract_section(source: &str, heading: &str) -> String {
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
