// Architecture definition schema — templates, layers, bounded contexts, rules.

use std::collections::BTreeMap;

use glob::Pattern;
use serde::{Deserialize, Serialize};

use crate::types::Layer;

/// Schema version for architecture definitions.
pub const ARCHITECTURE_DEFINITION_VERSION: &str = "0.1.0";

/// Available architecture templates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchitectureTemplate {
    Starter,
    Layered,
    Hexagonal,
    Clean,
    Ddd,
    Monorepo,
    Serverless,
    NxWorkspace,
    Custom,
}

impl std::fmt::Display for ArchitectureTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starter => write!(f, "starter"),
            Self::Layered => write!(f, "layered"),
            Self::Hexagonal => write!(f, "hexagonal"),
            Self::Clean => write!(f, "clean"),
            Self::Ddd => write!(f, "ddd"),
            Self::Monorepo => write!(f, "monorepo"),
            Self::Serverless => write!(f, "serverless"),
            Self::NxWorkspace => write!(f, "nx-workspace"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

/// Layer definition within a definition schema.
pub type LayerDefinition = Layer;

/// A bounded context with optional layer overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundedContext {
    /// Layer overrides for this context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<BTreeMap<String, LayerDefinition>>,
    /// Contexts this one is allowed to depend on.
    #[serde(default)]
    pub allowed_dependencies: Vec<String>,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Rule severity level.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSeverity {
    #[default]
    Error,
    Warn,
    Info,
    Ignore,
}

/// An explicit architecture rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureRule {
    /// Rule name.
    pub name: String,
    /// Source pattern/layer.
    pub from: String,
    /// Target pattern/layer.
    pub to: String,
    /// Severity level.
    #[serde(default)]
    pub severity: RuleSeverity,
    /// Whether the dependency is allowed (true) or forbidden (false).
    #[serde(default)]
    pub allowed: bool,
    /// Human-readable message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Architecture validation options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureOptions {
    /// Detect orphan files not assigned to any layer.
    #[serde(default = "default_true")]
    pub detect_orphans: bool,
    /// Detect circular dependencies between layers.
    #[serde(default = "default_true")]
    pub detect_circular: bool,
    /// Default severity for violations.
    #[serde(default)]
    pub default_severity: RuleSeverity,
    /// File patterns to exclude from analysis.
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "**/*.test.ts".into(),
        "**/*.spec.ts".into(),
        "**/__tests__/**".into(),
        "**/__fixtures__/**".into(),
        "**/node_modules/**".into(),
    ]
}

impl Default for ArchitectureOptions {
    fn default() -> Self {
        get_default_options()
    }
}

/// Complete architecture definition (parsed from `architecture.yaml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDefinition {
    /// Schema version.
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    /// Template used as the starting point.
    #[serde(default = "default_template")]
    pub template: ArchitectureTemplate,
    /// Layer definitions.
    #[serde(default)]
    pub layers: BTreeMap<String, LayerDefinition>,
    /// Bounded contexts (optional, for DDD-style projects).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounded_contexts: Option<BTreeMap<String, BoundedContext>>,
    /// Explicit architecture rules.
    #[serde(default)]
    pub rules: Vec<ArchitectureRule>,
    /// Validation options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<ArchitectureOptions>,
}

fn default_schema_version() -> String {
    ARCHITECTURE_DEFINITION_VERSION.into()
}

fn default_template() -> ArchitectureTemplate {
    ArchitectureTemplate::Custom
}

/// Validation error for architecture definitions.
#[derive(Debug, thiserror::Error)]
pub enum DefinitionValidationError {
    #[error("unknown layer '{layer}' referenced in depends_on of layer '{owner}'")]
    UnknownLayerDependency { owner: String, layer: String },
    #[error("rule '{rule}' references unknown layer '{layer}'")]
    UnknownRuleLayer { rule: String, layer: String },
    #[error("schema version '{version}' is not supported (expected {expected})")]
    UnsupportedVersion { version: String, expected: String },
    #[error("layer '{layer}' contains an empty pattern")]
    EmptyLayerPattern { layer: String },
    #[error(
        "layer '{left_layer}' pattern '{left_pattern}' overlaps with layer '{right_layer}' pattern '{right_pattern}'"
    )]
    OverlappingLayerPatterns {
        left_layer: String,
        left_pattern: String,
        right_layer: String,
        right_pattern: String,
    },
    #[error(
        "layer '{left_layer}' pattern '{left_pattern}' and layer '{right_layer}' pattern '{right_pattern}' are too complex to check for overlap safely"
    )]
    PatternOverlapComplexityExceeded {
        left_layer: String,
        left_pattern: String,
        right_layer: String,
        right_pattern: String,
    },
}

/// Upper bound on generated glob witnesses during overlap checks.
const MAX_PATTERN_WITNESSES: usize = 64;

/// Severity for architecture definition diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureDefinitionDiagnosticSeverity {
    Error,
    Warning,
}

/// Structured diagnostic for `.anvil/architecture.yaml` definition checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchitectureDefinitionDiagnostic {
    pub severity: ArchitectureDefinitionDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub section: String,
    pub key: String,
}

impl ArchitectureDefinitionDiagnostic {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        section: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            severity: ArchitectureDefinitionDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            section: section.into(),
            key: key.into(),
        }
    }

    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        section: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            severity: ArchitectureDefinitionDiagnosticSeverity::Warning,
            code: code.into(),
            message: message.into(),
            section: section.into(),
            key: key.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == ArchitectureDefinitionDiagnosticSeverity::Error
    }
}

/// Extract the leading major number from a `MAJOR[.MINOR[.PATCH]]` version
/// string. Returns `None` when the leading segment is missing or not a u64.
fn parse_major_version(version: &str) -> Option<u64> {
    version.split('.').next()?.parse::<u64>().ok()
}

/// Check whether a provided schema version is compatible with the supported
/// one: the leading major number must match. Different majors (or
/// unparseable input) are rejected.
///
/// Project convention — not strict semver: while the current schema is in
/// the major-0 unstable range, any minor/patch bump is still treated as
/// non-breaking so user configs survive schema evolution. Real breaking
/// changes are signalled by bumping the major number.
pub(crate) fn is_schema_version_compatible(provided: &str) -> bool {
    match (
        parse_major_version(provided),
        parse_major_version(ARCHITECTURE_DEFINITION_VERSION),
    ) {
        (Some(p), Some(s)) => p == s,
        _ => false,
    }
}

/// Validate an architecture definition for internal consistency.
pub fn validate_definition(
    definition: &ArchitectureDefinition,
) -> Result<(), Vec<DefinitionValidationError>> {
    let mut errors = Vec::new();

    // Schema version check: tolerate minor/patch bumps within the same
    // major so user configs survive non-breaking schema evolution.
    if !is_schema_version_compatible(&definition.schema_version) {
        errors.push(DefinitionValidationError::UnsupportedVersion {
            version: definition.schema_version.clone(),
            expected: ARCHITECTURE_DEFINITION_VERSION.into(),
        });
    }

    // Check layer dependency references
    for (name, layer) in &definition.layers {
        for pattern in &layer.patterns {
            if pattern.trim().is_empty() {
                errors.push(DefinitionValidationError::EmptyLayerPattern {
                    layer: name.clone(),
                });
            }
        }

        for dep in &layer.depends_on {
            if !definition.layers.contains_key(dep) {
                errors.push(DefinitionValidationError::UnknownLayerDependency {
                    owner: name.clone(),
                    layer: dep.clone(),
                });
            }
        }
    }

    let layer_entries: Vec<_> = definition.layers.iter().collect();
    for (left_index, (left_name, left_layer)) in layer_entries.iter().enumerate() {
        for (right_name, right_layer) in layer_entries.iter().skip(left_index + 1) {
            for left_pattern in &left_layer.patterns {
                for right_pattern in &right_layer.patterns {
                    match layer_patterns_overlap(left_pattern, right_pattern) {
                        Some(true) => errors.push(DefinitionValidationError::OverlappingLayerPatterns {
                            left_layer: (*left_name).clone(),
                            left_pattern: left_pattern.clone(),
                            right_layer: (*right_name).clone(),
                            right_pattern: right_pattern.clone(),
                        }),
                        Some(false) => {}
                        None => errors.push(
                            DefinitionValidationError::PatternOverlapComplexityExceeded {
                                left_layer: (*left_name).clone(),
                                left_pattern: left_pattern.clone(),
                                right_layer: (*right_name).clone(),
                                right_pattern: right_pattern.clone(),
                            },
                        ),
                    }
                }
            }
        }
    }

    // Check rule references
    for rule in &definition.rules {
        if !definition.layers.contains_key(&rule.from) {
            errors.push(DefinitionValidationError::UnknownRuleLayer {
                rule: rule.name.clone(),
                layer: rule.from.clone(),
            });
        }
        if !definition.layers.contains_key(&rule.to) {
            errors.push(DefinitionValidationError::UnknownRuleLayer {
                rule: rule.name.clone(),
                layer: rule.to.clone(),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Produce structured diagnostics for an architecture definition.
pub fn diagnose_definition(
    definition: &ArchitectureDefinition,
) -> Vec<ArchitectureDefinitionDiagnostic> {
    let mut diagnostics = Vec::new();

    if !is_schema_version_compatible(&definition.schema_version) {
        diagnostics.push(ArchitectureDefinitionDiagnostic::error(
            "unsupported-schema-version",
            format!(
                "Schema version '{}' is not supported (expected {})",
                definition.schema_version, ARCHITECTURE_DEFINITION_VERSION
            ),
            "schema_version",
            &definition.schema_version,
        ));
    }

    for (name, layer) in &definition.layers {
        if layer.patterns.is_empty() {
            diagnostics.push(ArchitectureDefinitionDiagnostic::warning(
                "empty-layer",
                format!("Layer \"{name}\" does not match any patterns"),
                format!("layers.{name}.patterns"),
                "patterns",
            ));
        }

        for pattern in &layer.patterns {
            if pattern.trim().is_empty() {
                diagnostics.push(ArchitectureDefinitionDiagnostic::error(
                    "empty-layer-pattern",
                    format!("Layer \"{name}\" contains an empty pattern"),
                    format!("layers.{name}.patterns"),
                    "patterns",
                ));
            }
        }

        for dep in &layer.depends_on {
            if !definition.layers.contains_key(dep) {
                diagnostics.push(ArchitectureDefinitionDiagnostic::error(
                    "unknown-layer-dependency",
                    format!("Layer \"{name}\" depends on unknown layer \"{dep}\""),
                    format!("layers.{name}.depends_on"),
                    dep,
                ));
            }
        }
    }

    let layer_entries: Vec<_> = definition.layers.iter().collect();
    for (left_index, (left_name, left_layer)) in layer_entries.iter().enumerate() {
        for (right_name, right_layer) in layer_entries.iter().skip(left_index + 1) {
            for left_pattern in &left_layer.patterns {
                for right_pattern in &right_layer.patterns {
                    match layer_patterns_overlap(left_pattern, right_pattern) {
                        Some(true) => diagnostics.push(ArchitectureDefinitionDiagnostic::error(
                            "overlapping-layer-patterns",
                            format!(
                                "Layer \"{left_name}\" pattern \"{left_pattern}\" overlaps with layer \"{right_name}\" pattern \"{right_pattern}\""
                            ),
                            format!("layers.{right_name}.patterns"),
                            right_pattern,
                        )),
                        Some(false) => {}
                        None => diagnostics.push(ArchitectureDefinitionDiagnostic::error(
                            "pattern-overlap-complexity-exceeded",
                            format!(
                                "Layer \"{left_name}\" pattern \"{left_pattern}\" and layer \"{right_name}\" pattern \"{right_pattern}\" are too complex to check for overlap safely"
                            ),
                            format!("layers.{right_name}.patterns"),
                            right_pattern,
                        )),
                    }
                }
            }
        }
    }

    for rule in &definition.rules {
        if !definition.layers.contains_key(&rule.from) {
            diagnostics.push(ArchitectureDefinitionDiagnostic::error(
                "unknown-rule-layer",
                format!(
                    "Rule \"{}\" references unknown layer \"{}\"",
                    rule.name, rule.from
                ),
                format!("rules.{}", rule.name),
                &rule.from,
            ));
        }
        if !definition.layers.contains_key(&rule.to) {
            diagnostics.push(ArchitectureDefinitionDiagnostic::error(
                "unknown-rule-layer",
                format!(
                    "Rule \"{}\" references unknown layer \"{}\"",
                    rule.name, rule.to
                ),
                format!("rules.{}", rule.name),
                &rule.to,
            ));
        }
    }

    diagnostics
}

/// `None` means the overlap check was aborted because witness generation exceeded
/// [`MAX_PATTERN_WITNESSES`].
fn layer_patterns_overlap(left: &str, right: &str) -> Option<bool> {
    let Ok(left_pattern) = Pattern::new(left) else {
        return Some(false);
    };
    let Ok(right_pattern) = Pattern::new(right) else {
        return Some(false);
    };

    let left_witnesses = pattern_witnesses(left)?;
    let right_witnesses = pattern_witnesses(right)?;

    Some(
        left_witnesses
            .iter()
            .any(|witness| right_pattern.matches(witness) && left_pattern.matches(witness))
            || right_witnesses
                .iter()
                .any(|witness| left_pattern.matches(witness) && right_pattern.matches(witness)),
    )
}

fn pattern_witnesses(pattern: &str) -> Option<Vec<String>> {
    let normalised = pattern.trim().replace('\\', "/");
    let mut witnesses = vec![String::new()];

    for segment in normalised.split('/') {
        let candidates = segment_candidates(segment);
        let mut next = Vec::new();
        for prefix in &witnesses {
            for candidate in &candidates {
                if candidate.is_empty() {
                    next.push(prefix.clone());
                } else if prefix.is_empty() {
                    next.push(candidate.clone());
                } else {
                    next.push(format!("{prefix}/{candidate}"));
                }
            }
        }
        if next.len() > MAX_PATTERN_WITNESSES {
            return None;
        }
        witnesses = next;
    }

    witnesses.sort();
    witnesses.dedup();
    Some(witnesses)
}

fn segment_candidates(segment: &str) -> Vec<String> {
    if segment == "**" {
        return vec!["x".into(), "domain".into(), "ui".into(), "x/y".into()];
    }

    if !segment.contains('*') && !segment.contains('?') && !segment.contains('[') {
        return vec![segment.to_string()];
    }

    let sample = segment
        .replace("**", "x")
        .replace('*', "sample")
        .replace('?', "x");
    vec![
        sample,
        segment
            .replace("**", "domain")
            .replace('*', "domain")
            .replace('?', "x"),
        segment
            .replace("**", "ui")
            .replace('*', "ui")
            .replace('?', "x"),
    ]
}

/// Get default architecture options.
pub fn get_default_options() -> ArchitectureOptions {
    ArchitectureOptions {
        detect_orphans: true,
        detect_circular: true,
        default_severity: RuleSeverity::Error,
        exclude_patterns: default_exclude_patterns(),
    }
}

/// Get the list of available template names.
pub fn get_available_templates() -> Vec<ArchitectureTemplate> {
    vec![
        ArchitectureTemplate::Starter,
        ArchitectureTemplate::Layered,
        ArchitectureTemplate::Hexagonal,
        ArchitectureTemplate::Clean,
        ArchitectureTemplate::Ddd,
        ArchitectureTemplate::Monorepo,
        ArchitectureTemplate::Serverless,
        ArchitectureTemplate::NxWorkspace,
        ArchitectureTemplate::Custom,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_serialises_kebab_case() {
        let json = serde_json::to_string(&ArchitectureTemplate::NxWorkspace).unwrap();
        assert_eq!(json, "\"nx-workspace\"");
    }

    #[test]
    fn template_deserialises_kebab_case() {
        let t: ArchitectureTemplate = serde_json::from_str("\"nx-workspace\"").unwrap();
        assert_eq!(t, ArchitectureTemplate::NxWorkspace);
    }

    #[test]
    fn template_display() {
        assert_eq!(ArchitectureTemplate::Hexagonal.to_string(), "hexagonal");
        assert_eq!(
            ArchitectureTemplate::NxWorkspace.to_string(),
            "nx-workspace"
        );
    }

    #[test]
    fn available_templates_has_nine_entries() {
        assert_eq!(get_available_templates().len(), 9);
    }

    #[test]
    fn default_options_exclude_test_files() {
        let opts = get_default_options();
        assert!(opts.exclude_patterns.iter().any(|p| p == "**/*.test.ts"));
    }

    #[test]
    fn validate_definition_accepts_valid() {
        let mut layers = BTreeMap::new();
        layers.insert(
            "core".into(),
            LayerDefinition {
                patterns: vec!["src/core/**".into()],
                depends_on: vec![],
                description: None,
            },
        );
        layers.insert(
            "app".into(),
            LayerDefinition {
                patterns: vec!["src/app/**".into()],
                depends_on: vec!["core".into()],
                description: None,
            },
        );

        let def = ArchitectureDefinition {
            schema_version: ARCHITECTURE_DEFINITION_VERSION.into(),
            template: ArchitectureTemplate::Custom,
            layers,
            bounded_contexts: None,
            rules: vec![],
            options: None,
        };

        assert!(validate_definition(&def).is_ok());
    }

    #[test]
    fn validate_definition_rejects_unknown_dep() {
        let mut layers = BTreeMap::new();
        layers.insert(
            "app".into(),
            LayerDefinition {
                patterns: vec!["src/app/**".into()],
                depends_on: vec!["nonexistent".into()],
                description: None,
            },
        );

        let def = ArchitectureDefinition {
            schema_version: ARCHITECTURE_DEFINITION_VERSION.into(),
            template: ArchitectureTemplate::Custom,
            layers,
            bounded_contexts: None,
            rules: vec![],
            options: None,
        };

        let errors = validate_definition(&def).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            DefinitionValidationError::UnknownLayerDependency { .. }
        ));
    }

    #[test]
    fn validate_definition_rejects_overlapping_layer_patterns() {
        let mut layers = BTreeMap::new();
        layers.insert(
            "app".into(),
            LayerDefinition {
                patterns: vec!["src/**".into()],
                depends_on: vec![],
                description: None,
            },
        );
        layers.insert(
            "ui".into(),
            LayerDefinition {
                patterns: vec!["src/ui/**".into()],
                depends_on: vec![],
                description: None,
            },
        );

        let def = ArchitectureDefinition {
            schema_version: ARCHITECTURE_DEFINITION_VERSION.into(),
            template: ArchitectureTemplate::Custom,
            layers,
            bounded_contexts: None,
            rules: vec![],
            options: None,
        };

        let errors = validate_definition(&def).unwrap_err();
        assert!(errors.iter().any(|e| {
            matches!(
                e,
                DefinitionValidationError::OverlappingLayerPatterns { .. }
            )
        }));
    }

    #[test]
    fn diagnose_definition_rejects_overly_complex_overlap_patterns() {
        let mut layers = BTreeMap::new();
        layers.insert(
            "left".into(),
            LayerDefinition {
                patterns: vec!["*/*/*/*/*/*/*/*".into()],
                depends_on: vec![],
                description: None,
            },
        );
        layers.insert(
            "right".into(),
            LayerDefinition {
                patterns: vec!["domain/*".into()],
                depends_on: vec![],
                description: None,
            },
        );

        let def = ArchitectureDefinition {
            schema_version: ARCHITECTURE_DEFINITION_VERSION.into(),
            template: ArchitectureTemplate::Custom,
            layers,
            bounded_contexts: None,
            rules: vec![],
            options: None,
        };

        let diagnostics = diagnose_definition(&def);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "pattern-overlap-complexity-exceeded" && d.is_error()),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn diagnose_definition_detects_mid_pattern_glob_overlap() {
        let mut layers = BTreeMap::new();
        layers.insert(
            "rust_modules".into(),
            LayerDefinition {
                patterns: vec!["src/*/*.rs".into()],
                depends_on: vec![],
                description: None,
            },
        );
        layers.insert(
            "domain".into(),
            LayerDefinition {
                patterns: vec!["src/domain/*".into()],
                depends_on: vec![],
                description: None,
            },
        );

        let def = ArchitectureDefinition {
            schema_version: ARCHITECTURE_DEFINITION_VERSION.into(),
            template: ArchitectureTemplate::Custom,
            layers,
            bounded_contexts: None,
            rules: vec![],
            options: None,
        };

        let diagnostics = diagnose_definition(&def);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "overlapping-layer-patterns" && d.is_error()),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn validate_definition_rejects_bad_version() {
        let def = ArchitectureDefinition {
            schema_version: "99.0.0".into(),
            template: ArchitectureTemplate::Custom,
            layers: BTreeMap::new(),
            bounded_contexts: None,
            rules: vec![],
            options: None,
        };

        let errors = validate_definition(&def).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DefinitionValidationError::UnsupportedVersion { .. }))
        );
    }

    #[test]
    fn definition_round_trips_yaml() {
        let yaml = r#"
schema_version: "0.1.0"
template: layered
layers:
  core:
    patterns: ["src/core/**"]
    depends_on: []
rules: []
"#;
        let def: ArchitectureDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.template, ArchitectureTemplate::Layered);
        assert!(def.layers.contains_key("core"));

        let serialised = serde_yaml::to_string(&def).unwrap();
        let reparsed: ArchitectureDefinition = serde_yaml::from_str(&serialised).unwrap();
        assert_eq!(reparsed.template, ArchitectureTemplate::Layered);
    }

    #[test]
    fn rule_severity_defaults_to_error() {
        assert_eq!(RuleSeverity::default(), RuleSeverity::Error);
    }

    /// EATEST-023: Pins the semver tolerance rule for `schema_version`:
    /// minor/patch bumps within the same major must validate successfully,
    /// but any major bump must be rejected. Guards against tightening the
    /// check back to exact-match (which would break every existing config
    /// on the next schema patch).
    #[test]
    fn validate_definition_accepts_future_minor_or_patch_bump() {
        fn make_def(version: &str) -> ArchitectureDefinition {
            ArchitectureDefinition {
                schema_version: version.into(),
                template: ArchitectureTemplate::Custom,
                layers: BTreeMap::new(),
                bounded_contexts: None,
                rules: vec![],
                options: None,
            }
        }

        // Future patch bump: same major.minor as current `0.1.0`.
        assert!(
            validate_definition(&make_def("0.1.1")).is_ok(),
            "future patch bump 0.1.1 must validate against expected 0.1.0"
        );

        // Future minor bump (and a larger one for good measure).
        assert!(validate_definition(&make_def("0.2.0")).is_ok());
        assert!(validate_definition(&make_def("0.99.99")).is_ok());

        // Bare major is deliberately accepted — the leading segment parses
        // as the current major, so `parse_major_version` returns a match.
        assert!(validate_definition(&make_def("0")).is_ok());

        // Far-future major bump must be rejected. Use a major value high
        // enough that it cannot collide with the supported major after
        // routine schema bumps (mirrors `validate_definition_rejects_bad_version`).
        let errors = validate_definition(&make_def("99.0.0")).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, DefinitionValidationError::UnsupportedVersion { .. })),
            "major bump 99.0.0 must be rejected"
        );

        // Unparseable leading major segment must be rejected, not silently
        // accepted. Inputs where the segment before the first `.` does not
        // parse as a u64.
        for bogus in ["", "abc", "x.1.0", "-1.0.0"] {
            let errs = validate_definition(&make_def(bogus)).unwrap_err();
            assert!(
                errs.iter()
                    .any(|e| matches!(e, DefinitionValidationError::UnsupportedVersion { .. })),
                "unparseable schema_version {bogus:?} must be rejected"
            );
        }
    }
}
