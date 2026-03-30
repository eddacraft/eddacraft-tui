// Architecture validation — layer assignment, boundary checking, violation detection.

use std::path::Path;

use glob::Pattern;

use crate::definition::ArchitectureDefinition;
use crate::types::{
    Boundary, BoundarySeverity, BoundaryViolation, DetectionConfidence, Layer, LayerAssignment,
    Layers, create_default_boundaries,
};

/// Validation statistics.
#[derive(Debug, Clone)]
pub struct ValidationStats {
    /// Total files analysed.
    pub files_analysed: usize,
    /// Files assigned to a layer.
    pub files_assigned: usize,
    /// Files with no layer assignment (orphans).
    pub orphan_count: usize,
    /// Total boundary violations found.
    pub violation_count: usize,
}

/// Result of architecture validation.
#[derive(Debug)]
pub struct ValidationResult {
    /// Whether the architecture is valid (no error-severity violations).
    pub valid: bool,
    /// Layer assignments for all analysed files.
    pub assignments: Vec<LayerAssignment>,
    /// Detected boundary violations.
    pub violations: Vec<BoundaryViolation>,
    /// Validation statistics.
    pub stats: ValidationStats,
    /// Whether boundary checking was active during this run.
    /// When `false`, `violations` will always be empty and `valid` always `true`
    /// regardless of the actual dependency structure (RCLI-013a).
    pub boundary_checking_active: bool,
}

/// Errors during validation.
#[derive(Debug, thiserror::Error)]
pub enum ValidateError {
    #[error("I/O error scanning workspace: {0}")]
    Io(#[from] std::io::Error),
    #[error("glob pattern error: {0}")]
    Pattern(#[from] glob::PatternError),
}

/// Validate the architecture of a workspace against a definition.
///
/// Collects source files, assigns each to a layer, and checks for
/// cross-layer boundary violations.
pub fn validate(
    workspace_root: &Path,
    definition: &ArchitectureDefinition,
) -> Result<ValidationResult, ValidateError> {
    let files = collect_source_files(workspace_root, definition);
    let assignments = assign_layers(&files, &definition.layers);

    let assigned_count = assignments.iter().filter(|a| a.layer.is_some()).count();
    let orphan_count = assignments.len() - assigned_count;

    // Build boundary rules from layer definitions.
    let boundaries = create_default_boundaries(&definition.layers);
    let violations = check_boundaries(&assignments, &boundaries);
    let violation_count = violations.len();

    let has_errors = violations.iter().any(|v| {
        v.boundary
            .as_ref()
            .is_some_and(|b| b.severity == BoundarySeverity::Error)
    });

    Ok(ValidationResult {
        valid: !has_errors,
        assignments,
        violations,
        stats: ValidationStats {
            files_analysed: files.len(),
            files_assigned: assigned_count,
            orphan_count,
            violation_count,
        },
        boundary_checking_active: boundary_checking_active(),
    })
}

/// Assign each file to a layer based on glob pattern matching.
pub fn assign_layers(files: &[String], layers: &Layers) -> Vec<LayerAssignment> {
    files
        .iter()
        .map(|file| assign_layer(file, layers))
        .collect()
}

/// Assign a single file to a layer.
fn assign_layer(file: &str, layers: &Layers) -> LayerAssignment {
    for (layer_name, layer) in layers {
        if let Some(pattern) = matches_layer(file, layer) {
            return LayerAssignment {
                file: file.into(),
                layer: Some(layer_name.clone()),
                confidence: DetectionConfidence::High,
                matched_pattern: Some(pattern),
            };
        }
    }

    LayerAssignment {
        file: file.into(),
        layer: None,
        confidence: DetectionConfidence::Low,
        matched_pattern: None,
    }
}

/// Check whether a file matches any of the layer's glob patterns.
/// Returns the first matching pattern or `None`.
fn matches_layer(file: &str, layer: &Layer) -> Option<String> {
    for pat_str in &layer.patterns {
        if let Ok(pattern) = Pattern::new(pat_str)
            && pattern.matches(file)
        {
            return Some(pat_str.clone());
        }
    }
    None
}

/// Check for boundary violations among assigned files.
///
/// **STUB (RCLI-013a):** Full import-edge extraction requires AST parsing
/// which is deferred to the kernel integration phase. This always returns
/// an empty list. The `validate()` caller emits a warning so users are
/// not misled by a clean result.
fn check_boundaries(
    _assignments: &[LayerAssignment],
    _boundaries: &[Boundary],
) -> Vec<BoundaryViolation> {
    Vec::new()
}

/// Whether boundary checking is currently active.
///
/// Returns `false` until the kernel AST parser is integrated (RCLI-013a).
#[must_use]
pub const fn boundary_checking_active() -> bool {
    false
}

/// Collect source files from the workspace, respecting exclude patterns.
fn collect_source_files(workspace_root: &Path, definition: &ArchitectureDefinition) -> Vec<String> {
    let empty = Vec::new();
    let exclude_patterns: Vec<Pattern> = definition
        .options
        .as_ref()
        .map_or(&empty, |opts| &opts.exclude_patterns)
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect();

    let include_extensions = ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs"];

    let mut files = Vec::new();

    let walker = walkdir::WalkDir::new(workspace_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden dirs and common build output for performance.
            if e.file_type().is_dir() {
                return name != "node_modules"
                    && name != ".git"
                    && name != "dist"
                    && name != "build"
                    && name != "target";
            }
            true
        });

    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !include_extensions.contains(&ext) {
            continue;
        }

        let rel_path = path
            .strip_prefix(workspace_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let excluded = exclude_patterns.iter().any(|p| p.matches(&rel_path));
        if !excluded {
            files.push(rel_path);
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::types::Layer;

    fn sample_layers() -> Layers {
        let mut layers = HashMap::new();
        layers.insert(
            "core".into(),
            Layer {
                patterns: vec!["src/core/**".into()],
                depends_on: vec![],
                description: None,
            },
        );
        layers.insert(
            "app".into(),
            Layer {
                patterns: vec!["src/app/**".into()],
                depends_on: vec!["core".into()],
                description: None,
            },
        );
        layers
    }

    #[test]
    fn assign_layer_matches_glob() {
        let layers = sample_layers();
        let assignment = assign_layer("src/core/entity.ts", &layers);
        assert_eq!(assignment.layer.as_deref(), Some("core"));
        assert_eq!(assignment.confidence, DetectionConfidence::High);
    }

    #[test]
    fn assign_layer_returns_none_for_unmatched() {
        let layers = sample_layers();
        let assignment = assign_layer("random/file.ts", &layers);
        assert!(assignment.layer.is_none());
        assert_eq!(assignment.confidence, DetectionConfidence::Low);
    }

    #[test]
    fn assign_layers_processes_all_files() {
        let layers = sample_layers();
        let files = vec![
            "src/core/entity.ts".into(),
            "src/app/service.ts".into(),
            "unmatched.ts".into(),
        ];
        let assignments = assign_layers(&files, &layers);
        assert_eq!(assignments.len(), 3);

        let assigned: Vec<_> = assignments.iter().filter(|a| a.layer.is_some()).collect();
        assert_eq!(assigned.len(), 2);
    }

    #[test]
    fn validate_with_empty_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();
        let def = crate::yaml_parser::create_definition_from_template(
            &crate::definition::ArchitectureTemplate::Starter,
        );

        let result = validate(tmp.path(), &def).unwrap();
        assert!(result.valid);
        assert_eq!(result.stats.files_analysed, 0);
    }

    #[test]
    fn validate_assigns_files_to_layers() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src_dir = tmp.path().join("src").join("core");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("entity.ts"), "export class Foo {}").unwrap();

        let mut layers = HashMap::new();
        layers.insert(
            "core".into(),
            Layer {
                patterns: vec!["src/core/**".into()],
                depends_on: vec![],
                description: None,
            },
        );

        let def = crate::definition::ArchitectureDefinition {
            schema_version: "0.1.0".into(),
            template: crate::definition::ArchitectureTemplate::Custom,
            layers,
            bounded_contexts: None,
            rules: vec![],
            options: Some(crate::definition::ArchitectureOptions {
                detect_orphans: true,
                detect_circular: true,
                default_severity: crate::definition::RuleSeverity::Error,
                exclude_patterns: vec![],
            }),
        };

        let result = validate(tmp.path(), &def).unwrap();
        assert_eq!(result.stats.files_analysed, 1);
        assert_eq!(result.stats.files_assigned, 1);
        assert_eq!(result.stats.orphan_count, 0);
    }

    #[test]
    fn collect_source_files_excludes_node_modules() {
        let tmp = tempfile::TempDir::new().unwrap();
        let nm = tmp.path().join("node_modules").join("pkg");
        std::fs::create_dir_all(&nm).unwrap();
        std::fs::write(nm.join("index.ts"), "").unwrap();

        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.ts"), "").unwrap();

        let def = crate::definition::ArchitectureDefinition {
            schema_version: "0.1.0".into(),
            template: crate::definition::ArchitectureTemplate::Custom,
            layers: HashMap::new(),
            bounded_contexts: None,
            rules: vec![],
            options: Some(crate::definition::get_default_options()),
        };

        let files = collect_source_files(tmp.path(), &def);
        assert_eq!(files.len(), 1);
        assert!(files[0].contains("main.ts"));
    }

    #[test]
    fn collect_source_files_excludes_target_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let target = tmp.path().join("target").join("debug");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("main.rs"), "").unwrap();

        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "").unwrap();

        let def = crate::definition::ArchitectureDefinition {
            schema_version: "0.1.0".into(),
            template: crate::definition::ArchitectureTemplate::Custom,
            layers: HashMap::new(),
            bounded_contexts: None,
            rules: vec![],
            options: Some(crate::definition::get_default_options()),
        };

        let files = collect_source_files(tmp.path(), &def);
        assert_eq!(files.len(), 1);
        assert!(files[0].contains("lib.rs"));
    }
}
