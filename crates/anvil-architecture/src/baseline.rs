// Baseline management — load, save, create `.anvil/architecture.json`.

use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;

use crate::types::{
    ArchitectureBaseline, BaselineSnapshot, BaselineViolation, Boundary, EntryPoint, Layers,
    create_default_boundaries, create_default_layers,
};
use crate::util::atomic_write;

/// Baseline file name.
pub const BASELINE_FILENAME: &str = "architecture.json";

/// Anvil configuration directory.
pub const ANVIL_DIR: &str = ".anvil";

/// Errors from baseline operations.
#[derive(Debug, thiserror::Error)]
pub enum BaselineError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid baseline JSON: {0}")]
    InvalidJson(String),
}

/// Get the full path to the baseline file.
pub fn get_baseline_path(workspace_root: &Path) -> std::path::PathBuf {
    workspace_root.join(ANVIL_DIR).join(BASELINE_FILENAME)
}

/// Check whether a baseline file exists.
pub fn baseline_exists(workspace_root: &Path) -> bool {
    get_baseline_path(workspace_root).exists()
}

/// Load the architecture baseline from `.anvil/architecture.json`.
///
/// Returns `Ok(None)` if the file does not exist.
pub fn load_baseline(workspace_root: &Path) -> Result<Option<ArchitectureBaseline>, BaselineError> {
    let path = get_baseline_path(workspace_root);

    if !path.exists() {
        return Ok(None);
    }

    let path_str = path.display().to_string();
    let content = std::fs::read_to_string(&path).map_err(|e| BaselineError::Io {
        path: path_str.clone(),
        source: e,
    })?;

    let baseline: ArchitectureBaseline =
        serde_json::from_str(&content).map_err(|e| BaselineError::InvalidJson(e.to_string()))?;

    Ok(Some(baseline))
}

/// Save the architecture baseline to `.anvil/architecture.json`.
pub fn save_baseline(
    workspace_root: &Path,
    baseline: &ArchitectureBaseline,
) -> Result<(), BaselineError> {
    let path = get_baseline_path(workspace_root);
    let path_str = path.display().to_string();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| BaselineError::Io {
            path: path_str.clone(),
            source: e,
        })?;
    }

    let content = serde_json::to_string_pretty(baseline)
        .map_err(|e| BaselineError::InvalidJson(e.to_string()))?;

    atomic_write(&path, format!("{content}\n").as_bytes()).map_err(|e| BaselineError::Io {
        path: path_str,
        source: e,
    })?;

    Ok(())
}

/// Options for creating a baseline.
#[derive(Debug, Default)]
pub struct CreateBaselineOptions {
    pub entry_points: Vec<EntryPoint>,
    pub layers: Option<Layers>,
    pub boundaries: Option<Vec<Boundary>>,
    pub violations: Vec<BaselineViolation>,
    pub module_count: u32,
}

/// Create a new baseline with defaults.
pub fn create_baseline(options: CreateBaselineOptions) -> ArchitectureBaseline {
    let now = Utc::now().to_rfc3339();
    let layers = options.layers.unwrap_or_else(create_default_layers);
    let boundaries = options
        .boundaries
        .unwrap_or_else(|| create_default_boundaries(&layers));

    ArchitectureBaseline {
        schema_version: "0.1.0".into(),
        created_at: now.clone(),
        updated_at: now.clone(),
        entry_points: options.entry_points,
        layers,
        boundaries,
        baseline_snapshot: BaselineSnapshot {
            module_count: options.module_count,
            timestamp: now,
            violations: options.violations,
        },
    }
}

/// Merge new violations into an existing set (deduplicates by ID).
pub fn merge_violations(
    existing: &[BaselineViolation],
    new_violations: &[BaselineViolation],
) -> Vec<BaselineViolation> {
    let mut by_id: HashMap<&str, &BaselineViolation> = HashMap::new();

    for v in existing {
        by_id.insert(&v.id, v);
    }
    for v in new_violations {
        by_id.insert(&v.id, v);
    }

    by_id.into_values().cloned().collect()
}

/// Find violations that are NEW (not present in baseline).
pub fn find_new_violations(
    current: &[BaselineViolation],
    baseline: &[BaselineViolation],
) -> Vec<BaselineViolation> {
    let baseline_ids: std::collections::HashSet<&str> =
        baseline.iter().map(|v| v.id.as_str()).collect();
    current
        .iter()
        .filter(|v| !baseline_ids.contains(v.id.as_str()))
        .cloned()
        .collect()
}

/// Find violations that were FIXED (in baseline but not in current).
pub fn find_fixed_violations(
    current: &[BaselineViolation],
    baseline: &[BaselineViolation],
) -> Vec<BaselineViolation> {
    let current_ids: std::collections::HashSet<&str> =
        current.iter().map(|v| v.id.as_str()).collect();
    baseline
        .iter()
        .filter(|v| !current_ids.contains(v.id.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_exists_returns_false_for_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(!baseline_exists(tmp.path()));
    }

    #[test]
    fn load_baseline_returns_none_when_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = load_baseline(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn create_and_load_baseline_round_trips() {
        let tmp = tempfile::TempDir::new().unwrap();

        let baseline = create_baseline(CreateBaselineOptions {
            module_count: 42,
            ..Default::default()
        });
        assert_eq!(baseline.schema_version, "0.1.0");
        assert_eq!(baseline.baseline_snapshot.module_count, 42);

        save_baseline(tmp.path(), &baseline).unwrap();
        assert!(baseline_exists(tmp.path()));

        let loaded = load_baseline(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.schema_version, "0.1.0");
        assert_eq!(loaded.baseline_snapshot.module_count, 42);
    }

    #[test]
    fn load_baseline_rejects_invalid_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(ANVIL_DIR);
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(anvil_dir.join(BASELINE_FILENAME), "not-json").unwrap();

        let result = load_baseline(tmp.path());
        assert!(matches!(result, Err(BaselineError::InvalidJson(_))));
    }

    #[test]
    fn create_baseline_uses_defaults() {
        let baseline = create_baseline(CreateBaselineOptions::default());
        assert!(!baseline.layers.is_empty());
        assert!(!baseline.boundaries.is_empty());
    }

    fn make_violation(id: &str) -> BaselineViolation {
        BaselineViolation {
            id: id.into(),
            from_layer: "a".into(),
            to_layer: "b".into(),
            from_file: "a.ts".into(),
            to_file: "b.ts".into(),
            import_line: 1,
            rule: None,
        }
    }

    #[test]
    fn merge_violations_deduplicates() {
        let existing = vec![make_violation("v1"), make_violation("v2")];
        let new = vec![make_violation("v2"), make_violation("v3")];

        let merged = merge_violations(&existing, &new);
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn find_new_violations_filters_baseline() {
        let current = vec![make_violation("v1"), make_violation("v2")];
        let baseline = vec![make_violation("v1")];

        let new = find_new_violations(&current, &baseline);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].id, "v2");
    }

    #[test]
    fn find_fixed_violations_filters_current() {
        let current = vec![make_violation("v1")];
        let baseline = vec![make_violation("v1"), make_violation("v2")];

        let fixed = find_fixed_violations(&current, &baseline);
        assert_eq!(fixed.len(), 1);
        assert_eq!(fixed[0].id, "v2");
    }
}
