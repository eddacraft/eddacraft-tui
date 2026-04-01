// Core architecture types — entry points, layers, boundaries, violations, baselines.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// =============================================================================
// Entry point types
// =============================================================================

/// Entry point types detected in the codebase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryPointType {
    Package,
    Application,
    Http,
    Api,
    Cli,
    Worker,
    Test,
    Unknown,
}

/// Confidence level for detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionConfidence {
    High,
    Medium,
    Low,
}

/// A detected entry point in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    /// File path relative to workspace root.
    pub path: String,
    /// Type of entry point.
    #[serde(rename = "type")]
    pub entry_type: EntryPointType,
    /// Detection confidence.
    pub confidence: DetectionConfidence,
    /// Named exports if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exports: Option<Vec<String>>,
}

// =============================================================================
// Layer types
// =============================================================================

/// Layer definition with dependency rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    /// Glob patterns matching files in this layer.
    pub patterns: Vec<String>,
    /// Layers this layer is allowed to depend on.
    pub depends_on: Vec<String>,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Map of layer name to layer definition.
pub type Layers = HashMap<String, Layer>;

// =============================================================================
// Boundary types
// =============================================================================

/// Boundary violation severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundarySeverity {
    Error,
    Warning,
    Info,
}

/// Explicit boundary rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Boundary {
    /// Unique boundary name.
    pub name: String,
    /// Source layer.
    pub from: String,
    /// Target layer.
    pub to: String,
    /// Violation severity.
    pub severity: BoundarySeverity,
    /// Human-readable message when violated.
    pub message: String,
    /// Inference confidence (for auto-detected boundaries).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<DetectionConfidence>,
}

// =============================================================================
// Violation types (for baseline snapshot)
// =============================================================================

/// A recorded violation in the baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineViolation {
    /// Unique violation ID (hash of from+to+line).
    pub id: String,
    /// Source layer.
    pub from_layer: String,
    /// Target layer.
    pub to_layer: String,
    /// File containing the import.
    pub from_file: String,
    /// File being imported.
    pub to_file: String,
    /// Line number of the import.
    pub import_line: u32,
    /// Rule name that was violated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// Snapshot of the architecture state at baseline time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineSnapshot {
    /// Total modules analysed.
    pub module_count: u32,
    /// When the baseline was created.
    pub timestamp: String,
    /// Existing violations at baseline time.
    pub violations: Vec<BaselineViolation>,
}

// =============================================================================
// Architecture baseline (full)
// =============================================================================

/// Complete architecture baseline stored in `.anvil/architecture.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureBaseline {
    /// Schema version.
    pub schema_version: String,
    /// When the baseline was created.
    pub created_at: String,
    /// When the baseline was last updated.
    pub updated_at: String,
    /// Detected entry points.
    pub entry_points: Vec<EntryPoint>,
    /// Layer definitions with dependency rules.
    pub layers: Layers,
    /// Explicit boundary rules.
    pub boundaries: Vec<Boundary>,
    /// Snapshot of violations at baseline time.
    pub baseline_snapshot: BaselineSnapshot,
}

// =============================================================================
// Layer detection result
// =============================================================================

/// Result of layer detection for a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerAssignment {
    /// File path.
    pub file: String,
    /// Assigned layer (None if unassigned).
    pub layer: Option<String>,
    /// Assignment confidence.
    pub confidence: DetectionConfidence,
    /// Pattern that matched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_pattern: Option<String>,
}

// =============================================================================
// Dependency edge
// =============================================================================

/// Import type for a dependency edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportType {
    Import,
    Require,
    Dynamic,
}

/// A dependency edge between two files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Source file.
    pub from: String,
    /// Target file.
    pub to: String,
    /// Source layer.
    pub from_layer: Option<String>,
    /// Target layer.
    pub to_layer: Option<String>,
    /// Import line number.
    pub line: u32,
    /// Import type.
    #[serde(rename = "type")]
    pub import_type: ImportType,
}

// =============================================================================
// Boundary violation
// =============================================================================

/// A detected boundary violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryViolation {
    /// The violating edge.
    pub edge: DependencyEdge,
    /// Explicit boundary violated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary: Option<Boundary>,
    /// Whether this is a NEW violation.
    pub is_new: bool,
    /// ID in baseline if existing violation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_id: Option<String>,
}

// =============================================================================
// Utility functions
// =============================================================================

/// Create a deterministic violation ID from edge details.
///
/// Uses SHA-256 truncated to 64 bits (16 hex chars) for compact, low-collision
/// IDs. Each field is length-prefixed so `("a:b", "c")` and `("a", "b:c")`
/// produce distinct values. Not cryptographically collision-resistant at this
/// truncation — sufficient for baseline deduplication at typical project scale.
pub fn create_violation_id(from_file: &str, to_file: &str, line: u32) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    #[allow(clippy::cast_possible_truncation)]
    {
        hasher.update((from_file.len() as u64).to_le_bytes());
        hasher.update(from_file.as_bytes());
        hasher.update((to_file.len() as u64).to_le_bytes());
        hasher.update(to_file.as_bytes());
    }
    hasher.update(line.to_le_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..8])
}

/// Check if a violation exists in the baseline.
///
/// Matches first by the current (line-aware) ID, then falls back to the legacy
/// `line: 0` ID so that baselines created before line numbers were tracked
/// continue to suppress their violations without a forced regeneration.
pub fn is_existing_violation(violation: &BoundaryViolation, baseline: &BaselineSnapshot) -> bool {
    let id = create_violation_id(
        &violation.edge.from,
        &violation.edge.to,
        violation.edge.line,
    );
    if baseline.violations.iter().any(|v| v.id == id) {
        return true;
    }
    // Backward compat: baselines generated with line=0 should still match.
    if violation.edge.line != 0 {
        let legacy_id = create_violation_id(&violation.edge.from, &violation.edge.to, 0);
        return baseline.violations.iter().any(|v| v.id == legacy_id);
    }
    false
}

/// Create default layer structure for common patterns.
///
/// Patterns use `**/` prefix to match files in any directory structure,
/// supporting both single-app (`src/`) and monorepo (`packages/*/src/`) layouts.
pub fn create_default_layers() -> Layers {
    let mut layers = Layers::new();

    layers.insert(
        "presentation".into(),
        Layer {
            patterns: vec![
                "**/controllers/**".into(),
                "**/routes/**".into(),
                "**/api/**".into(),
                "**/handlers/**".into(),
                "**/endpoints/**".into(),
                "**/pages/**".into(),
            ],
            depends_on: vec!["application".into(), "shared".into()],
            description: Some("HTTP handlers, controllers, API routes".into()),
        },
    );

    layers.insert(
        "application".into(),
        Layer {
            patterns: vec![
                "**/services/**".into(),
                "**/use-cases/**".into(),
                "**/usecases/**".into(),
                "**/application/**".into(),
                "**/interactors/**".into(),
            ],
            depends_on: vec!["domain".into(), "infrastructure".into(), "shared".into()],
            description: Some("Business logic, use cases, services".into()),
        },
    );

    layers.insert(
        "domain".into(),
        Layer {
            patterns: vec![
                "**/domain/**".into(),
                "**/entities/**".into(),
                "**/models/**".into(),
                "**/core/**".into(),
                "**/business/**".into(),
            ],
            depends_on: vec!["shared".into()],
            description: Some("Domain entities, value objects, domain logic".into()),
        },
    );

    layers.insert(
        "infrastructure".into(),
        Layer {
            patterns: vec![
                "**/repositories/**".into(),
                "**/data/**".into(),
                "**/infrastructure/**".into(),
                "**/db/**".into(),
                "**/database/**".into(),
                "**/adapters/**".into(),
                "**/external/**".into(),
                "**/clients/**".into(),
            ],
            depends_on: vec!["domain".into(), "shared".into()],
            description: Some("Data access, external services, infrastructure".into()),
        },
    );

    layers.insert(
        "shared".into(),
        Layer {
            patterns: vec![
                "**/utils/**".into(),
                "**/lib/**".into(),
                "**/common/**".into(),
                "**/shared/**".into(),
                "**/helpers/**".into(),
                "**/types/**".into(),
                "**/constants/**".into(),
                "**/config/**".into(),
            ],
            depends_on: vec![],
            description: Some("Shared utilities, helpers, common code".into()),
        },
    );

    layers
}

/// Create default boundaries from layer structure.
///
/// For each layer pair where the dependency is not explicitly allowed,
/// creates a boundary rule that forbids the cross-layer import.
pub fn create_default_boundaries(layers: &Layers) -> Vec<Boundary> {
    let mut boundaries = Vec::new();
    let layer_names: Vec<&String> = layers.keys().collect();

    for from_layer in &layer_names {
        let allowed_deps = &layers[*from_layer].depends_on;

        for to_layer in &layer_names {
            if from_layer == to_layer {
                continue;
            }

            if !allowed_deps.iter().any(|d| d == *to_layer) {
                boundaries.push(Boundary {
                    name: format!("no-{from_layer}-to-{to_layer}"),
                    from: (*from_layer).clone(),
                    to: (*to_layer).clone(),
                    severity: BoundarySeverity::Error,
                    message: format!("{from_layer} layer must not directly depend on {to_layer}"),
                    confidence: Some(DetectionConfidence::High),
                });
            }
        }
    }

    boundaries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_violation_id_is_deterministic() {
        let a = create_violation_id("src/foo/bar.ts", "src/baz/qux.ts", 42);
        let b = create_violation_id("src/foo/bar.ts", "src/baz/qux.ts", 42);
        assert_eq!(a, b);
        assert_eq!(a.len(), 16); // 8 bytes hex-encoded
    }

    #[test]
    fn create_violation_id_no_collision_on_colon_fields() {
        let a = create_violation_id("a:b", "c", 1);
        let b = create_violation_id("a", "b:c", 1);
        assert_ne!(
            a, b,
            "length-prefixed hashing must prevent field boundary collisions"
        );
    }

    #[test]
    fn is_existing_violation_matches() {
        let violation = BoundaryViolation {
            edge: DependencyEdge {
                from: "a.ts".into(),
                to: "b.ts".into(),
                from_layer: None,
                to_layer: None,
                line: 10,
                import_type: ImportType::Import,
            },
            boundary: None,
            is_new: true,
            baseline_id: None,
        };

        let snapshot = BaselineSnapshot {
            module_count: 1,
            timestamp: "2026-01-01T00:00:00Z".into(),
            violations: vec![BaselineViolation {
                id: create_violation_id("a.ts", "b.ts", 10),
                from_layer: "x".into(),
                to_layer: "y".into(),
                from_file: "a.ts".into(),
                to_file: "b.ts".into(),
                import_line: 10,
                rule: None,
            }],
        };

        assert!(is_existing_violation(&violation, &snapshot));
    }

    #[test]
    fn is_existing_violation_does_not_match() {
        let violation = BoundaryViolation {
            edge: DependencyEdge {
                from: "a.ts".into(),
                to: "c.ts".into(),
                from_layer: None,
                to_layer: None,
                line: 10,
                import_type: ImportType::Import,
            },
            boundary: None,
            is_new: true,
            baseline_id: None,
        };

        let snapshot = BaselineSnapshot {
            module_count: 1,
            timestamp: "2026-01-01T00:00:00Z".into(),
            violations: vec![],
        };

        assert!(!is_existing_violation(&violation, &snapshot));
    }

    #[test]
    fn default_layers_has_five_layers() {
        let layers = create_default_layers();
        assert_eq!(layers.len(), 5);
        assert!(layers.contains_key("presentation"));
        assert!(layers.contains_key("application"));
        assert!(layers.contains_key("domain"));
        assert!(layers.contains_key("infrastructure"));
        assert!(layers.contains_key("shared"));
    }

    #[test]
    fn shared_layer_has_no_dependencies() {
        let layers = create_default_layers();
        assert!(layers["shared"].depends_on.is_empty());
    }

    #[test]
    fn default_boundaries_forbid_disallowed_deps() {
        let layers = create_default_layers();
        let boundaries = create_default_boundaries(&layers);

        // domain -> presentation should be forbidden
        let has_domain_to_presentation = boundaries
            .iter()
            .any(|b| b.from == "domain" && b.to == "presentation");
        assert!(has_domain_to_presentation);

        // presentation -> shared should be allowed (no boundary)
        let has_presentation_to_shared = boundaries
            .iter()
            .any(|b| b.from == "presentation" && b.to == "shared");
        assert!(!has_presentation_to_shared);
    }

    #[test]
    fn entry_point_type_serialises_snake_case() {
        let json = serde_json::to_string(&EntryPointType::Http).unwrap();
        assert_eq!(json, "\"http\"");
    }

    #[test]
    fn detection_confidence_round_trips() {
        let json = serde_json::to_string(&DetectionConfidence::Medium).unwrap();
        let parsed: DetectionConfidence = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, DetectionConfidence::Medium);
    }
}
