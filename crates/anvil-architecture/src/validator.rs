// Architecture validation — layer assignment, boundary checking, violation detection.

use std::path::Path;

use glob::Pattern;

use crate::definition::{ArchitectureDefinition, ArchitectureRule, RuleSeverity};
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
/// cross-layer boundary violations. Without import edges, boundary checking
/// produces no violations. Use [`validate_with_edges`] for full analysis.
pub fn validate(
    workspace_root: &Path,
    definition: &ArchitectureDefinition,
) -> Result<ValidationResult, ValidateError> {
    validate_with_edges(workspace_root, definition, &[])
}

/// Validate architecture with import edges for full boundary analysis.
///
/// Import edges should be extracted via tree-sitter (kernel parser) or
/// equivalent. Each edge maps a source file to an imported file, both
/// relative to the workspace root.
pub fn validate_with_edges(
    workspace_root: &Path,
    definition: &ArchitectureDefinition,
    edges: &[ImportEdge],
) -> Result<ValidationResult, ValidateError> {
    let files = collect_source_files(workspace_root, definition);
    Ok(validate_files(definition, &files, edges))
}

/// Validate architecture using a pre-collected file list and import edges.
///
/// This is a pure function — no I/O. Use it to avoid redundant file-tree
/// walks when the caller has already collected source files (e.g. for
/// import edge extraction).
pub fn validate_with_files_and_edges(
    definition: &ArchitectureDefinition,
    files: &[String],
    edges: &[ImportEdge],
) -> ValidationResult {
    validate_files(definition, files, edges)
}

fn validate_files(
    definition: &ArchitectureDefinition,
    files: &[String],
    edges: &[ImportEdge],
) -> ValidationResult {
    let assignments = assign_layers(files, &definition.layers);

    let assigned_count = assignments.iter().filter(|a| a.layer.is_some()).count();
    let orphan_count = assignments.len() - assigned_count;

    // Build boundary rules from layer definitions, then merge explicit rules.
    let boundaries = merge_rules_into_boundaries(
        create_default_boundaries(&definition.layers),
        &definition.rules,
    );
    let violations = check_boundaries(&assignments, &boundaries, edges);
    let violation_count = violations.len();

    let has_errors = violations.iter().any(|v| {
        v.boundary
            .as_ref()
            .is_some_and(|b| b.severity == BoundarySeverity::Error)
    });

    ValidationResult {
        valid: !has_errors,
        assignments,
        violations,
        stats: ValidationStats {
            files_analysed: files.len(),
            files_assigned: assigned_count,
            orphan_count,
            violation_count,
        },
        boundary_checking_active: !edges.is_empty(),
    }
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

/// Merge explicit user-authored rules from `architecture.yaml` into the
/// boundary set. "Allow wins" semantics: allow rules always remove matching
/// boundaries (including earlier explicit ones), deny rules add or override.
///
/// - `allowed: true` rules remove any boundary with matching `from`/`to`
///   (regardless of severity)
/// - `allowed: false` rules add new deny boundaries (or override severity)
/// - `RuleSeverity::Ignore` on deny rules skips them; allow rules are
///   unaffected by severity
fn merge_rules_into_boundaries(
    mut boundaries: Vec<Boundary>,
    rules: &[ArchitectureRule],
) -> Vec<Boundary> {
    for rule in rules {
        if rule.allowed {
            // Allow rules always remove matching boundaries regardless of severity.
            boundaries.retain(|b| !(b.from == rule.from && b.to == rule.to));
            continue;
        }

        let severity = match rule.severity {
            RuleSeverity::Error => BoundarySeverity::Error,
            RuleSeverity::Warn => BoundarySeverity::Warning,
            RuleSeverity::Info => BoundarySeverity::Info,
            RuleSeverity::Ignore => continue, // ignore-severity deny rules have no effect
        };

        {
            // Check if a boundary already exists for this edge.
            if let Some(existing) = boundaries
                .iter_mut()
                .find(|b| b.from == rule.from && b.to == rule.to)
            {
                // Override severity and message from the explicit rule.
                existing.severity = severity;
                if let Some(ref msg) = rule.message {
                    existing.message.clone_from(msg);
                }
                existing.name.clone_from(&rule.name);
            } else {
                // Add a new deny boundary.
                boundaries.push(Boundary {
                    name: rule.name.clone(),
                    from: rule.from.clone(),
                    to: rule.to.clone(),
                    severity,
                    message: rule.message.clone().unwrap_or_else(|| {
                        format!(
                            "{} must not depend on {} (rule: {})",
                            rule.from, rule.to, rule.name
                        )
                    }),
                    confidence: Some(DetectionConfidence::High),
                });
            }
        }
    }
    boundaries
}

/// Known source file extensions for extensionless import resolution.
///
/// RSTLAN-007: `rs` is included so a Rust import edge whose target was left
/// extensionless (e.g. a `mod foo;` resolved only to `src/foo`) still resolves
/// to `src/foo.rs` for layer lookup rather than being silently treated as
/// unassigned. Fully-resolved Rust edges (the common case — `resolve_rust_import`
/// already appends `.rs`) match exactly and never reach this fallback.
const IMPORT_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs"];

/// ESM `.js` extension mappings — when a `.js` import specifier doesn't match
/// an actual `.js` file, try these TypeScript equivalents (common in ESM
/// projects where `import './foo.js'` resolves to `foo.ts` at build time).
const ESM_EXTENSION_MAP: &[(&str, &[&str])] = &[
    (".js", &[".ts", ".tsx"]),
    (".jsx", &[".tsx"]),
    (".mjs", &[".mts"]),
    (".cjs", &[".cts"]),
];

/// An import edge for boundary checking — source file importing a target file.
#[derive(Debug, Clone)]
pub struct ImportEdge {
    /// File containing the import (relative to workspace root).
    pub from_file: String,
    /// Resolved file being imported (relative to workspace root).
    pub to_file: String,
    /// Line number of the import statement.
    pub line: u32,
}

/// Check for boundary violations among assigned files using import edges.
///
/// For each import edge, looks up the layer of the source and target files.
/// If the import crosses a boundary that is not allowed, emits a violation.
/// Target files are matched by exact path or by resolving extensionless imports
/// using known source file extensions (e.g. `../app/service` matching `src/app/service.ts`).
pub fn check_boundaries(
    assignments: &[LayerAssignment],
    boundaries: &[Boundary],
    edges: &[ImportEdge],
) -> Vec<BoundaryViolation> {
    use std::collections::HashMap;

    let file_to_layer: HashMap<&str, &str> = assignments
        .iter()
        .filter_map(|a| a.layer.as_deref().map(|l| (a.file.as_str(), l)))
        .collect();

    let mut violations = Vec::new();

    for edge in edges {
        let from_layer = file_to_layer.get(edge.from_file.as_str()).copied();

        // Resolve target: try exact match, then extensionless resolution,
        // then ESM extension swaps (e.g. `./foo.js` → `foo.ts`).
        let to_layer = file_to_layer
            .get(edge.to_file.as_str())
            .copied()
            .or_else(|| {
                // Extensionless: try appending known extensions.
                IMPORT_EXTENSIONS.iter().find_map(|ext| {
                    let candidate = format!("{}.{ext}", edge.to_file);
                    file_to_layer.get(candidate.as_str()).copied()
                })
            })
            .or_else(|| {
                // ESM extension swap: `import './foo.js'` → `foo.ts`.
                ESM_EXTENSION_MAP.iter().find_map(|(from_ext, to_exts)| {
                    let stem = edge.to_file.strip_suffix(from_ext)?;
                    to_exts.iter().find_map(|to_ext| {
                        let candidate = format!("{stem}{to_ext}");
                        file_to_layer.get(candidate.as_str()).copied()
                    })
                })
            });

        let (Some(from_l), Some(to_l)) = (from_layer, to_layer) else {
            continue; // Skip edges involving unassigned files.
        };

        if from_l == to_l {
            continue; // Same-layer imports are always allowed.
        }

        // Check if any boundary forbids this cross-layer import.
        if let Some(boundary) = boundaries.iter().find(|b| b.from == from_l && b.to == to_l) {
            violations.push(BoundaryViolation {
                edge: crate::types::DependencyEdge {
                    from: edge.from_file.clone(),
                    to: edge.to_file.clone(),
                    from_layer: Some(from_l.to_string()),
                    to_layer: Some(to_l.to_string()),
                    line: edge.line,
                    import_type: crate::types::ImportType::Import,
                },
                boundary: Some(boundary.clone()),
                is_new: true,
                baseline_id: None,
            });
        }
    }

    violations
}

/// Collect source files from the workspace, respecting exclude patterns.
///
/// Public so callers can share a single file list across both import
/// edge extraction and architecture validation (avoids redundant walks).
pub fn collect_source_files(
    workspace_root: &Path,
    definition: &ArchitectureDefinition,
) -> Vec<String> {
    let empty = Vec::new();
    let exclude_patterns: Vec<Pattern> = definition
        .options
        .as_ref()
        .map_or(&empty, |opts| &opts.exclude_patterns)
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect();

    let include_extensions = ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "py"];

    let mut files = Vec::new();

    // SCAN-001: validator discovery uses `ignore::WalkBuilder` so it
    // shares the noise-pruning walker (skips target/, node_modules/, etc; not .gitignore) shape with the rest of the
    // scan-fanout sites. The downstream boundary check is single-pass
    // string-pattern matching (not regex on file content), so we don't
    // add a rayon stage here — the walker swap is the win that matters
    // for the entx-class repo benchmark.
    let walker = ignore::WalkBuilder::new(workspace_root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip well-known non-source directories for performance.
            if e.file_type().is_some_and(|ft| ft.is_dir()) {
                return name != "node_modules"
                    && name != ".git"
                    && name != "dist"
                    && name != "build"
                    && name != "target";
            }
            true
        })
        .build();

    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
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
    use std::collections::BTreeMap;

    use super::*;
    use crate::types::Layer;

    fn sample_layers() -> Layers {
        let mut layers = BTreeMap::new();
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

    // EATEST-021: When two layers' glob patterns both match the same file,
    // the assignment must be deterministic — the alphabetically-first layer
    // name wins, because `Layers` is a `BTreeMap` and iteration order is by
    // key. Pins behaviour that callers (baseline diffs, violation IDs) rely
    // on across runs.
    #[test]
    fn assign_layer_resolves_multi_match_alphabetically_and_deterministically() {
        fn build_overlapping_layers(insertion_order: &[&str]) -> Layers {
            let mut layers = BTreeMap::new();
            for name in insertion_order {
                layers.insert(
                    (*name).into(),
                    Layer {
                        patterns: vec!["src/shared/**".into()],
                        depends_on: vec![],
                        description: None,
                    },
                );
            }
            layers
        }

        // Insertion order must not affect the outcome — alphabetical order
        // is sourced from the `BTreeMap`'s keys, not how it was constructed.
        let order_a = build_overlapping_layers(&["zlayer", "mlayer", "alayer"]);
        let order_b = build_overlapping_layers(&["alayer", "zlayer", "mlayer"]);
        let order_c = build_overlapping_layers(&["mlayer", "alayer", "zlayer"]);

        for layers in [&order_a, &order_b, &order_c] {
            let assignment = assign_layer("src/shared/util.ts", layers);
            assert_eq!(
                assignment.layer.as_deref(),
                Some("alayer"),
                "alphabetically-first layer must win on glob collisions"
            );
        }
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

        let mut layers = BTreeMap::new();
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
    fn merge_rules_allow_removes_boundary() {
        let boundaries = vec![Boundary {
            name: "no-app-to-infra".into(),
            from: "app".into(),
            to: "infra".into(),
            severity: BoundarySeverity::Error,
            message: "forbidden".into(),
            confidence: None,
        }];
        let rules = vec![crate::definition::ArchitectureRule {
            name: "allow-app-to-infra".into(),
            from: "app".into(),
            to: "infra".into(),
            severity: crate::definition::RuleSeverity::Error,
            allowed: true,
            message: None,
        }];

        let result = merge_rules_into_boundaries(boundaries, &rules);
        assert!(result.is_empty(), "allow rule should remove the boundary");
    }

    #[test]
    fn merge_rules_deny_adds_boundary() {
        let boundaries = Vec::new();
        let rules = vec![crate::definition::ArchitectureRule {
            name: "no-core-to-ui".into(),
            from: "core".into(),
            to: "ui".into(),
            severity: crate::definition::RuleSeverity::Warn,
            allowed: false,
            message: Some("core must not touch ui".into()),
        }];

        let result = merge_rules_into_boundaries(boundaries, &rules);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "no-core-to-ui");
        assert_eq!(result[0].severity, BoundarySeverity::Warning);
        assert_eq!(result[0].message, "core must not touch ui");
    }

    #[test]
    fn merge_rules_deny_overrides_existing() {
        let boundaries = vec![Boundary {
            name: "auto-generated".into(),
            from: "app".into(),
            to: "core".into(),
            severity: BoundarySeverity::Error,
            message: "auto message".into(),
            confidence: None,
        }];
        let rules = vec![crate::definition::ArchitectureRule {
            name: "custom-rule".into(),
            from: "app".into(),
            to: "core".into(),
            severity: crate::definition::RuleSeverity::Warn,
            allowed: false,
            message: Some("downgraded to warning".into()),
        }];

        let result = merge_rules_into_boundaries(boundaries, &rules);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "custom-rule");
        assert_eq!(result[0].severity, BoundarySeverity::Warning);
        assert_eq!(result[0].message, "downgraded to warning");
    }

    #[test]
    fn merge_rules_ignore_severity_skipped() {
        let boundaries = vec![Boundary {
            name: "no-x-to-y".into(),
            from: "x".into(),
            to: "y".into(),
            severity: BoundarySeverity::Error,
            message: "forbidden".into(),
            confidence: None,
        }];
        let rules = vec![crate::definition::ArchitectureRule {
            name: "ignore-rule".into(),
            from: "x".into(),
            to: "y".into(),
            severity: crate::definition::RuleSeverity::Ignore,
            allowed: false,
            message: None,
        }];

        let result = merge_rules_into_boundaries(boundaries, &rules);
        assert_eq!(result.len(), 1, "ignore-severity rule should be skipped");
        assert_eq!(result[0].name, "no-x-to-y");
    }

    #[test]
    fn merge_rules_allow_ignores_severity() {
        let boundaries = vec![Boundary {
            name: "no-x-to-y".into(),
            from: "x".into(),
            to: "y".into(),
            severity: BoundarySeverity::Error,
            message: "forbidden".into(),
            confidence: None,
        }];
        let rules = vec![crate::definition::ArchitectureRule {
            name: "allow-with-ignore".into(),
            from: "x".into(),
            to: "y".into(),
            severity: crate::definition::RuleSeverity::Ignore,
            allowed: true,
            message: None,
        }];

        let result = merge_rules_into_boundaries(boundaries, &rules);
        assert!(
            result.is_empty(),
            "allow rule should remove boundary regardless of severity"
        );
    }

    #[test]
    fn check_boundaries_detects_violation() {
        let assignments = vec![
            LayerAssignment {
                file: "src/core/entity.ts".into(),
                layer: Some("core".into()),
                confidence: DetectionConfidence::High,
                matched_pattern: None,
            },
            LayerAssignment {
                file: "src/app/service.ts".into(),
                layer: Some("app".into()),
                confidence: DetectionConfidence::High,
                matched_pattern: None,
            },
        ];
        let boundaries = vec![Boundary {
            name: "no-core-to-app".into(),
            from: "core".into(),
            to: "app".into(),
            severity: BoundarySeverity::Error,
            message: "core must not depend on app".into(),
            confidence: None,
        }];
        let edges = vec![ImportEdge {
            from_file: "src/core/entity.ts".into(),
            to_file: "src/app/service.ts".into(),
            line: 1,
        }];

        let violations = check_boundaries(&assignments, &boundaries, &edges);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].edge.from, "src/core/entity.ts");
        assert!(violations[0].boundary.is_some());
    }

    #[test]
    fn check_boundaries_allows_valid_import() {
        let assignments = vec![
            LayerAssignment {
                file: "src/app/service.ts".into(),
                layer: Some("app".into()),
                confidence: DetectionConfidence::High,
                matched_pattern: None,
            },
            LayerAssignment {
                file: "src/core/entity.ts".into(),
                layer: Some("core".into()),
                confidence: DetectionConfidence::High,
                matched_pattern: None,
            },
        ];
        // Only forbid core→app, not app→core.
        let boundaries = vec![Boundary {
            name: "no-core-to-app".into(),
            from: "core".into(),
            to: "app".into(),
            severity: BoundarySeverity::Error,
            message: "core must not depend on app".into(),
            confidence: None,
        }];
        let edges = vec![ImportEdge {
            from_file: "src/app/service.ts".into(),
            to_file: "src/core/entity.ts".into(),
            line: 1,
        }];

        let violations = check_boundaries(&assignments, &boundaries, &edges);
        assert!(violations.is_empty(), "app→core should be allowed");
    }

    #[test]
    fn check_boundaries_ignores_same_layer() {
        let assignments = vec![
            LayerAssignment {
                file: "src/core/a.ts".into(),
                layer: Some("core".into()),
                confidence: DetectionConfidence::High,
                matched_pattern: None,
            },
            LayerAssignment {
                file: "src/core/b.ts".into(),
                layer: Some("core".into()),
                confidence: DetectionConfidence::High,
                matched_pattern: None,
            },
        ];
        let boundaries = vec![Boundary {
            name: "no-core-to-core".into(),
            from: "core".into(),
            to: "core".into(),
            severity: BoundarySeverity::Error,
            message: "shouldn't trigger".into(),
            confidence: None,
        }];
        let edges = vec![ImportEdge {
            from_file: "src/core/a.ts".into(),
            to_file: "src/core/b.ts".into(),
            line: 1,
        }];

        let violations = check_boundaries(&assignments, &boundaries, &edges);
        assert!(
            violations.is_empty(),
            "same-layer imports should be allowed"
        );
    }

    #[test]
    fn validate_respects_explicit_rules() {
        let tmp = tempfile::TempDir::new().unwrap();

        let mut layers = BTreeMap::new();
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

        // Without rules, there's a default boundary forbidding core→app.
        let def_no_rules = crate::definition::ArchitectureDefinition {
            schema_version: "0.1.0".into(),
            template: crate::definition::ArchitectureTemplate::Custom,
            layers: layers.clone(),
            bounded_contexts: None,
            rules: vec![],
            options: Some(crate::definition::get_default_options()),
        };
        let _result = validate(tmp.path(), &def_no_rules).unwrap();
        // Boundary checking is inactive here because `validate` passes no edges,
        // but we can still verify the boundaries were built correctly via the function.
        let boundaries_no_rules =
            merge_rules_into_boundaries(create_default_boundaries(&layers), &[]);
        let has_core_to_app = boundaries_no_rules
            .iter()
            .any(|b| b.from == "core" && b.to == "app");
        assert!(has_core_to_app, "should have core→app boundary by default");

        // With an allow rule, that boundary is removed.
        let rules = vec![crate::definition::ArchitectureRule {
            name: "allow-core-to-app".into(),
            from: "core".into(),
            to: "app".into(),
            severity: crate::definition::RuleSeverity::Error,
            allowed: true,
            message: None,
        }];
        let boundaries_with_rules =
            merge_rules_into_boundaries(create_default_boundaries(&layers), &rules);
        let has_core_to_app = boundaries_with_rules
            .iter()
            .any(|b| b.from == "core" && b.to == "app");
        assert!(
            !has_core_to_app,
            "allow rule should remove core→app boundary"
        );
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
            layers: BTreeMap::new(),
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
            layers: BTreeMap::new(),
            bounded_contexts: None,
            rules: vec![],
            options: Some(crate::definition::get_default_options()),
        };

        let files = collect_source_files(tmp.path(), &def);
        assert_eq!(files.len(), 1);
        assert!(files[0].contains("lib.rs"));
    }

    fn sample_definition(layers: Layers) -> ArchitectureDefinition {
        ArchitectureDefinition {
            schema_version: "1".into(),
            template: crate::definition::ArchitectureTemplate::Layered,
            layers,
            bounded_contexts: None,
            rules: vec![],
            options: None,
        }
    }

    #[test]
    fn validate_with_files_and_edges_produces_same_result() {
        let definition = sample_definition(sample_layers());
        let files = vec!["src/core/entity.ts".into(), "src/app/service.ts".into()];
        let edges = vec![ImportEdge {
            from_file: "src/app/service.ts".into(),
            to_file: "src/core/entity.ts".into(),
            line: 1,
        }];

        let result = validate_with_files_and_edges(&definition, &files, &edges);

        assert!(result.valid);
        assert_eq!(result.stats.files_analysed, 2);
        assert_eq!(result.stats.files_assigned, 2);
        assert!(result.boundary_checking_active);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn validate_with_files_and_edges_detects_violation() {
        let definition = sample_definition(sample_layers());
        // core importing from app violates the dependency direction.
        let files = vec!["src/core/entity.ts".into(), "src/app/service.ts".into()];
        let edges = vec![ImportEdge {
            from_file: "src/core/entity.ts".into(),
            to_file: "src/app/service.ts".into(),
            line: 5,
        }];

        let result = validate_with_files_and_edges(&definition, &files, &edges);

        assert!(!result.valid);
        assert!(!result.violations.is_empty());
        assert_eq!(result.violations[0].edge.from, "src/core/entity.ts");
    }

    // --- RSTLAN-007: the validate surface reports Rust crates/modules ---------

    #[test]
    fn assign_layer_matches_rust_file() {
        // Layer assignment is path-glob based, so `.rs` files land in layers the
        // same way `.ts` files do — there is no language gate.
        let layers = sample_layers();
        let assignment = assign_layer("src/core/entity.rs", &layers);
        assert_eq!(assignment.layer.as_deref(), Some("core"));
        assert_eq!(assignment.confidence, DetectionConfidence::High);
    }

    #[test]
    fn validate_surface_detects_rust_cross_layer_violation() {
        // The same public validate surface the CLI/MCP/dashboard sit on must
        // flag a forbidden Rust cross-layer import, with the `.rs` paths
        // appearing verbatim in the violation (no "Rust ignored" silent path).
        let definition = sample_definition(sample_layers());
        let files = vec!["src/core/entity.rs".into(), "src/app/service.rs".into()];
        let edges = vec![ImportEdge {
            // core -> app violates the dependency direction (crate::app::service).
            from_file: "src/core/entity.rs".into(),
            to_file: "src/app/service.rs".into(),
            line: 7,
        }];

        let result = validate_with_files_and_edges(&definition, &files, &edges);

        assert!(
            !result.valid,
            "a forbidden Rust cross-layer import must fail"
        );
        assert!(result.boundary_checking_active);
        let v = result
            .violations
            .first()
            .expect("expected a Rust boundary violation");
        assert_eq!(v.edge.from, "src/core/entity.rs");
        assert_eq!(v.edge.to, "src/app/service.rs");
    }

    // --- PYLAN-008: the validate surface reports Python packages/modules -------

    #[test]
    fn assign_layer_matches_python_file() {
        // Layer assignment is path-glob based, so `.py` files land in layers the
        // same way `.ts`/`.rs` files do — no language gate.
        let layers = sample_layers();
        let assignment = assign_layer("src/core/entity.py", &layers);
        assert_eq!(assignment.layer.as_deref(), Some("core"));
        assert_eq!(assignment.confidence, DetectionConfidence::High);
    }

    #[test]
    fn validate_surface_detects_python_cross_layer_violation() {
        // The public validate surface must flag a forbidden Python cross-layer
        // import (the resolver in `python_resolve` maps `..app.service` to the
        // `.py` file; the file paths appear verbatim in the violation).
        let definition = sample_definition(sample_layers());
        let files = vec!["src/core/entity.py".into(), "src/app/service.py".into()];
        let edges = vec![ImportEdge {
            // core -> app violates the dependency direction.
            from_file: "src/core/entity.py".into(),
            to_file: "src/app/service.py".into(),
            line: 4,
        }];

        let result = validate_with_files_and_edges(&definition, &files, &edges);

        assert!(
            !result.valid,
            "a forbidden Python cross-layer import must fail"
        );
        assert!(result.boundary_checking_active);
        let v = result
            .violations
            .first()
            .expect("expected a Python boundary violation");
        assert_eq!(v.edge.from, "src/core/entity.py");
        assert_eq!(v.edge.to, "src/app/service.py");
    }

    #[test]
    fn check_boundaries_resolves_extensionless_rust_target() {
        // RSTLAN-007: a Rust edge whose target was left extensionless still
        // resolves to the `.rs` file for layer lookup (IMPORT_EXTENSIONS now
        // carries `rs`), so it is not silently treated as unassigned.
        let assignments = assign_layers(
            &["src/core/entity.rs".into(), "src/app/service.rs".into()],
            &sample_layers(),
        );
        let boundaries = vec![Boundary {
            name: "core-cannot-use-app".into(),
            from: "core".into(),
            to: "app".into(),
            severity: BoundarySeverity::Error,
            message: "core must not depend on app".into(),
            confidence: None,
        }];
        let edges = vec![ImportEdge {
            from_file: "src/core/entity.rs".into(),
            to_file: "src/app/service".into(), // extensionless
            line: 3,
        }];

        let violations = check_boundaries(&assignments, &boundaries, &edges);
        assert_eq!(
            violations.len(),
            1,
            "extensionless Rust target must resolve to src/app/service.rs"
        );
    }
}
