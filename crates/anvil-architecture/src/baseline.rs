// Baseline management — load, save, create `.anvil/architecture.json`.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::Utc;

use crate::ANVIL_DIR;
use crate::types::{
    ArchitectureBaseline, BaselineSnapshot, BaselineViolation, Boundary, EntryPoint, Layers,
    create_default_boundaries, create_default_layers,
};
use crate::util::atomic_write;

/// Baseline file name.
pub const BASELINE_FILENAME: &str = "architecture.json";

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
    load_baseline_capped(workspace_root, MAX_BASELINE_BYTES)
}

/// Maximum size of `.anvil/architecture.json` that [`load_baseline`] reads into
/// memory (CIB-084). A baseline larger than this is almost certainly corrupt or
/// hostile; the read is refused rather than committing unbounded memory.
pub const MAX_BASELINE_BYTES: u64 = 16 * 1024 * 1024;

/// [`load_baseline`] with an explicit read cap, so the size guard is testable
/// without writing a multi-megabyte fixture.
fn load_baseline_capped(
    workspace_root: &Path,
    cap: u64,
) -> Result<Option<ArchitectureBaseline>, BaselineError> {
    let path = get_baseline_path(workspace_root);

    if !path.exists() {
        return Ok(None);
    }

    let path_str = path.display().to_string();
    let content =
        crate::util::read_to_string_capped(&path, cap).map_err(|e| BaselineError::Io {
            path: path_str,
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
    let mut by_id: BTreeMap<&str, &BaselineViolation> = BTreeMap::new();

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
    let mut result: Vec<BaselineViolation> = current
        .iter()
        .filter(|v| !baseline_ids.contains(v.id.as_str()))
        .cloned()
        .collect();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

/// Find violations that were FIXED (in baseline but not in current).
pub fn find_fixed_violations(
    current: &[BaselineViolation],
    baseline: &[BaselineViolation],
) -> Vec<BaselineViolation> {
    let current_ids: std::collections::HashSet<&str> =
        current.iter().map(|v| v.id.as_str()).collect();
    let mut result: Vec<BaselineViolation> = baseline
        .iter()
        .filter(|v| !current_ids.contains(v.id.as_str()))
        .cloned()
        .collect();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
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
    fn load_baseline_rejects_a_file_over_the_read_cap() {
        // CIB-084: an over-cap architecture.json is refused (as an IO error)
        // before it is read into memory or parsed.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(ANVIL_DIR)).unwrap();
        std::fs::write(get_baseline_path(tmp.path()), "{}").unwrap();
        let err = load_baseline_capped(tmp.path(), 1).unwrap_err();
        assert!(matches!(err, BaselineError::Io { .. }), "{err:?}");
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
    fn merge_violations_deduplicates_and_sorts_by_id() {
        let existing = vec![make_violation("v2"), make_violation("v1")];
        let new = vec![make_violation("v3"), make_violation("v2")];

        let merged = merge_violations(&existing, &new);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].id, "v1");
        assert_eq!(merged[1].id, "v2");
        assert_eq!(merged[2].id, "v3");
    }

    // EATEST-022: When `merge_violations` sees two entries sharing the same
    // ID but different fields (e.g. different `from_file`), the new-list
    // entry must win. Pins the precedence semantics callers depend on when
    // refreshing a baseline.
    #[test]
    fn merge_violations_new_wins_on_id_collision() {
        let existing = vec![BaselineViolation {
            id: "shared".into(),
            from_layer: "a".into(),
            to_layer: "b".into(),
            from_file: "old_path.ts".into(),
            to_file: "b.ts".into(),
            import_line: 1,
            rule: None,
        }];
        let new = vec![BaselineViolation {
            id: "shared".into(),
            from_layer: "a".into(),
            to_layer: "b".into(),
            from_file: "new_path.ts".into(),
            to_file: "b.ts".into(),
            import_line: 9,
            rule: Some("refreshed".into()),
        }];

        let merged = merge_violations(&existing, &new);
        assert_eq!(merged.len(), 1, "ID collision must produce a single entry");
        assert_eq!(
            merged[0].from_file, "new_path.ts",
            "new_violations entry must win on ID collision"
        );
        assert_eq!(merged[0].import_line, 9);
        assert_eq!(merged[0].rule.as_deref(), Some("refreshed"));
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
