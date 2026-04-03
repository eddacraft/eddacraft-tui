// Architecture definition schema — templates, layers, bounded contexts, rules.

use std::collections::BTreeMap;

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
}

/// Validate an architecture definition for internal consistency.
pub fn validate_definition(
    definition: &ArchitectureDefinition,
) -> Result<(), Vec<DefinitionValidationError>> {
    let mut errors = Vec::new();

    // Check schema version
    if definition.schema_version != ARCHITECTURE_DEFINITION_VERSION {
        errors.push(DefinitionValidationError::UnsupportedVersion {
            version: definition.schema_version.clone(),
            expected: ARCHITECTURE_DEFINITION_VERSION.into(),
        });
    }

    // Check layer dependency references
    for (name, layer) in &definition.layers {
        for dep in &layer.depends_on {
            if !definition.layers.contains_key(dep) {
                errors.push(DefinitionValidationError::UnknownLayerDependency {
                    owner: name.clone(),
                    layer: dep.clone(),
                });
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
}
