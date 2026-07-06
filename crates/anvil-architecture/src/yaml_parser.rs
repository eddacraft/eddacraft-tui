// YAML parsing for architecture definitions.
//
// Reads and writes `.anvil/architecture.yaml`, applies template defaults,
// and merges user overrides with template layer structures.

use std::collections::BTreeMap;
use std::path::Path;

use crate::ANVIL_DIR;
use crate::definition::{
    ARCHITECTURE_DEFINITION_VERSION, ArchitectureDefinition, ArchitectureTemplate,
    get_default_options,
};
use crate::types::Layer;
use crate::util::{atomic_write, read_to_string_capped};

/// File name for the architecture definition.
pub const ARCHITECTURE_YAML_FILENAME: &str = "architecture.yaml";

/// Maximum architecture YAML file size (1 MiB) — guards against
/// billion-laughs YAML expansion attacks.
pub const ARCHITECTURE_YAML_MAX_SIZE: u64 = 1024 * 1024;
pub(crate) const MAX_YAML_SIZE: u64 = ARCHITECTURE_YAML_MAX_SIZE;

type LayersRecord = BTreeMap<String, Layer>;

/// Errors that can occur during YAML parsing.
#[derive(Debug, thiserror::Error)]
pub enum YamlParseError {
    #[error("architecture YAML not found: {0}")]
    NotFound(String),
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid architecture YAML: {0}")]
    InvalidYaml(String),
    #[error("I/O error writing {path}: {source}")]
    WriteIo {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Get the full path to the architecture YAML file.
pub fn get_architecture_yaml_path(workspace_root: &Path) -> std::path::PathBuf {
    workspace_root
        .join(ANVIL_DIR)
        .join(ARCHITECTURE_YAML_FILENAME)
}

/// Check whether `architecture.yaml` exists in the workspace.
pub fn architecture_yaml_exists(workspace_root: &Path) -> bool {
    get_architecture_yaml_path(workspace_root).exists()
}

/// Parse the architecture definition from `.anvil/architecture.yaml`.
pub fn parse_architecture_definition(
    workspace_root: &Path,
) -> Result<ArchitectureDefinition, YamlParseError> {
    let yaml_path = get_architecture_yaml_path(workspace_root);
    parse_architecture_definition_file(&yaml_path)
}

/// Parse an architecture definition from an explicit YAML path.
pub fn parse_architecture_definition_file(
    yaml_path: &Path,
) -> Result<ArchitectureDefinition, YamlParseError> {
    let yaml_str = yaml_path.display().to_string();

    if !yaml_path.exists() {
        return Err(YamlParseError::NotFound(yaml_str));
    }

    let content =
        read_to_string_capped(yaml_path, MAX_YAML_SIZE).map_err(|e| YamlParseError::Io {
            path: yaml_str.clone(),
            source: e,
        })?;

    let definition: ArchitectureDefinition =
        serde_yaml::from_str(&content).map_err(|e| YamlParseError::InvalidYaml(e.to_string()))?;

    Ok(apply_defaults(definition))
}

/// Write an architecture definition to `.anvil/architecture.yaml`.
pub fn write_architecture_yaml(
    workspace_root: &Path,
    definition: &ArchitectureDefinition,
) -> Result<(), YamlParseError> {
    let yaml_path = get_architecture_yaml_path(workspace_root);
    let yaml_str = yaml_path.display().to_string();

    // Ensure the directory exists.
    if let Some(parent) = yaml_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| YamlParseError::WriteIo {
            path: yaml_str.clone(),
            source: e,
        })?;
    }

    let content = serde_yaml::to_string(definition)
        .map_err(|e| YamlParseError::InvalidYaml(e.to_string()))?;

    atomic_write(&yaml_path, content.as_bytes()).map_err(|e| YamlParseError::WriteIo {
        path: yaml_str,
        source: e,
    })?;

    Ok(())
}

/// Apply defaults (fill in missing options).
fn apply_defaults(mut definition: ArchitectureDefinition) -> ArchitectureDefinition {
    if definition.options.is_none() {
        definition.options = Some(get_default_options());
    }
    definition
}

// =============================================================================
// Template defaults
// =============================================================================

fn layered_template() -> LayersRecord {
    let mut m = LayersRecord::new();
    m.insert(
        "presentation".into(),
        Layer {
            patterns: vec![
                "src/controllers/**".into(),
                "src/routes/**".into(),
                "src/api/**".into(),
            ],
            depends_on: vec!["business".into(), "shared".into()],
            description: None,
        },
    );
    m.insert(
        "business".into(),
        Layer {
            patterns: vec!["src/services/**".into(), "src/use-cases/**".into()],
            depends_on: vec!["data".into(), "shared".into()],
            description: None,
        },
    );
    m.insert(
        "data".into(),
        Layer {
            patterns: vec![
                "src/repositories/**".into(),
                "src/db/**".into(),
                "src/data/**".into(),
            ],
            depends_on: vec!["shared".into()],
            description: None,
        },
    );
    m.insert(
        "shared".into(),
        Layer {
            patterns: vec![
                "src/utils/**".into(),
                "src/lib/**".into(),
                "src/common/**".into(),
            ],
            depends_on: vec![],
            description: None,
        },
    );
    m
}

fn hexagonal_template() -> LayersRecord {
    let mut m = LayersRecord::new();
    m.insert(
        "core".into(),
        Layer {
            patterns: vec!["src/domain/**".into(), "src/core/**".into()],
            depends_on: vec![],
            description: Some("Domain logic - no external dependencies".into()),
        },
    );
    m.insert(
        "ports".into(),
        Layer {
            patterns: vec!["src/ports/**".into(), "src/interfaces/**".into()],
            depends_on: vec!["core".into()],
            description: Some("Port interfaces".into()),
        },
    );
    m.insert(
        "adapters".into(),
        Layer {
            patterns: vec!["src/adapters/**".into(), "src/infrastructure/**".into()],
            depends_on: vec!["ports".into(), "core".into()],
            description: Some("Adapter implementations".into()),
        },
    );
    m.insert(
        "application".into(),
        Layer {
            patterns: vec!["src/application/**".into(), "src/services/**".into()],
            depends_on: vec!["core".into(), "ports".into()],
            description: Some("Application services".into()),
        },
    );
    m
}

fn clean_template() -> LayersRecord {
    let mut m = LayersRecord::new();
    m.insert(
        "entities".into(),
        Layer {
            patterns: vec!["src/entities/**".into(), "src/domain/entities/**".into()],
            depends_on: vec![],
            description: Some("Enterprise business rules".into()),
        },
    );
    m.insert(
        "use_cases".into(),
        Layer {
            patterns: vec!["src/use-cases/**".into(), "src/application/**".into()],
            depends_on: vec!["entities".into()],
            description: Some("Application business rules".into()),
        },
    );
    m.insert(
        "interface_adapters".into(),
        Layer {
            patterns: vec![
                "src/adapters/**".into(),
                "src/controllers/**".into(),
                "src/presenters/**".into(),
            ],
            depends_on: vec!["use_cases".into(), "entities".into()],
            description: Some("Interface adapters".into()),
        },
    );
    m.insert(
        "frameworks".into(),
        Layer {
            patterns: vec![
                "src/frameworks/**".into(),
                "src/infrastructure/**".into(),
                "src/db/**".into(),
            ],
            depends_on: vec![
                "interface_adapters".into(),
                "use_cases".into(),
                "entities".into(),
            ],
            description: Some("Frameworks and drivers".into()),
        },
    );
    m
}

fn ddd_template() -> LayersRecord {
    let mut m = LayersRecord::new();
    m.insert(
        "domain".into(),
        Layer {
            patterns: vec!["src/domain/**".into()],
            depends_on: vec![],
            description: Some("Domain model and logic".into()),
        },
    );
    m.insert(
        "application".into(),
        Layer {
            patterns: vec!["src/application/**".into()],
            depends_on: vec!["domain".into()],
            description: Some("Application services".into()),
        },
    );
    m.insert(
        "infrastructure".into(),
        Layer {
            patterns: vec!["src/infrastructure/**".into()],
            depends_on: vec!["domain".into(), "application".into()],
            description: Some("Infrastructure implementations".into()),
        },
    );
    m.insert(
        "interfaces".into(),
        Layer {
            patterns: vec!["src/interfaces/**".into(), "src/api/**".into()],
            depends_on: vec!["application".into(), "domain".into()],
            description: Some("User interfaces and API".into()),
        },
    );
    m
}

fn starter_template() -> LayersRecord {
    let mut m = LayersRecord::new();
    m.insert(
        "components".into(),
        Layer {
            patterns: vec!["src/components/**".into(), "src/ui/**".into()],
            depends_on: vec!["lib".into()],
            description: Some("UI components and visual elements".into()),
        },
    );
    m.insert(
        "lib".into(),
        Layer {
            patterns: vec![
                "src/lib/**".into(),
                "src/utils/**".into(),
                "src/helpers/**".into(),
            ],
            depends_on: vec![],
            description: Some("Shared utilities and helper functions".into()),
        },
    );
    m.insert(
        "services".into(),
        Layer {
            patterns: vec!["src/services/**".into(), "src/api/**".into()],
            depends_on: vec!["lib".into()],
            description: Some("API calls and external service integrations".into()),
        },
    );
    m
}

fn monorepo_template() -> LayersRecord {
    let mut m = LayersRecord::new();
    m.insert(
        "packages".into(),
        Layer {
            patterns: vec![
                "apps/**".into(),
                "packages/**".into(),
                "libs/**".into(),
                "utils/**".into(),
            ],
            depends_on: vec!["shared".into()],
            description: Some("Application and library packages".into()),
        },
    );
    m.insert(
        "shared".into(),
        Layer {
            patterns: vec!["shared/**".into()],
            depends_on: vec![],
            description: Some("Shared utilities and configurations".into()),
        },
    );
    m
}

fn serverless_template() -> LayersRecord {
    let mut m = LayersRecord::new();
    m.insert(
        "functions".into(),
        Layer {
            patterns: vec![
                "src/functions/**".into(),
                "src/handlers/**".into(),
                "src/lambdas/**".into(),
            ],
            depends_on: vec!["services".into(), "shared".into()],
            description: Some("Serverless function handlers".into()),
        },
    );
    m.insert(
        "services".into(),
        Layer {
            patterns: vec!["src/services/**".into(), "src/business/**".into()],
            depends_on: vec!["shared".into()],
            description: Some("Business logic shared across functions".into()),
        },
    );
    m.insert(
        "shared".into(),
        Layer {
            patterns: vec![
                "src/shared/**".into(),
                "src/utils/**".into(),
                "src/lib/**".into(),
            ],
            depends_on: vec![],
            description: Some("Shared utilities and configurations".into()),
        },
    );
    m
}

fn nx_workspace_template() -> LayersRecord {
    let mut m = LayersRecord::new();
    m.insert(
        "apps".into(),
        Layer {
            patterns: vec!["apps/**".into()],
            depends_on: vec!["feature-libs".into(), "shared-libs".into()],
            description: Some("Deployable applications".into()),
        },
    );
    m.insert(
        "feature-libs".into(),
        Layer {
            patterns: vec!["libs/feature-*/**".into(), "libs/*/feature-*/**".into()],
            depends_on: vec![
                "data-access-libs".into(),
                "ui-libs".into(),
                "shared-libs".into(),
            ],
            description: Some("Feature libraries".into()),
        },
    );
    m.insert(
        "data-access-libs".into(),
        Layer {
            patterns: vec![
                "libs/data-access-*/**".into(),
                "libs/*/data-access-*/**".into(),
            ],
            depends_on: vec!["shared-libs".into()],
            description: Some("Data access libraries".into()),
        },
    );
    m.insert(
        "ui-libs".into(),
        Layer {
            patterns: vec!["libs/ui-*/**".into(), "libs/*/ui-*/**".into()],
            depends_on: vec!["shared-libs".into()],
            description: Some("UI component libraries".into()),
        },
    );
    m.insert(
        "shared-libs".into(),
        Layer {
            patterns: vec![
                "libs/shared/**".into(),
                "libs/util-*/**".into(),
                "libs/*/util-*/**".into(),
            ],
            depends_on: vec![],
            description: Some("Shared utilities and configurations".into()),
        },
    );
    m
}

/// Get template default layers for a given template.
pub fn get_template_defaults(template: &ArchitectureTemplate) -> LayersRecord {
    match template {
        ArchitectureTemplate::Starter => starter_template(),
        ArchitectureTemplate::Layered => layered_template(),
        ArchitectureTemplate::Hexagonal => hexagonal_template(),
        ArchitectureTemplate::Clean => clean_template(),
        ArchitectureTemplate::Ddd => ddd_template(),
        ArchitectureTemplate::Monorepo => monorepo_template(),
        ArchitectureTemplate::Serverless => serverless_template(),
        ArchitectureTemplate::NxWorkspace => nx_workspace_template(),
        ArchitectureTemplate::Custom => LayersRecord::new(),
    }
}

/// Merge user definition with template defaults.
///
/// If the definition has no user-defined layers, fills them in from the
/// template. Always ensures options are populated.
pub fn merge_with_template(definition: ArchitectureDefinition) -> ArchitectureDefinition {
    let template_layers = get_template_defaults(&definition.template);
    let has_user_layers = !definition.layers.is_empty();

    ArchitectureDefinition {
        layers: if has_user_layers {
            definition.layers
        } else {
            template_layers
        },
        options: Some(definition.options.unwrap_or_else(get_default_options)),
        ..definition
    }
}

/// Create a fully-populated definition from a template name.
pub fn create_definition_from_template(template: &ArchitectureTemplate) -> ArchitectureDefinition {
    ArchitectureDefinition {
        schema_version: ARCHITECTURE_DEFINITION_VERSION.into(),
        template: template.clone(),
        layers: get_template_defaults(template),
        bounded_contexts: None,
        rules: vec![],
        options: Some(get_default_options()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architecture_yaml_exists_returns_false_for_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(!architecture_yaml_exists(tmp.path()));
    }

    #[test]
    fn parse_definition_errors_when_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = parse_architecture_definition(tmp.path());
        assert!(matches!(result, Err(YamlParseError::NotFound(_))));
    }

    #[test]
    fn parse_and_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let def = create_definition_from_template(&ArchitectureTemplate::Layered);

        write_architecture_yaml(tmp.path(), &def).unwrap();
        assert!(architecture_yaml_exists(tmp.path()));

        let parsed = parse_architecture_definition(tmp.path()).unwrap();
        assert_eq!(parsed.template, ArchitectureTemplate::Layered);
        assert!(!parsed.layers.is_empty());
    }

    #[test]
    fn template_defaults_layered_has_four_layers() {
        let layers = get_template_defaults(&ArchitectureTemplate::Layered);
        assert_eq!(layers.len(), 4);
        assert!(layers.contains_key("presentation"));
        assert!(layers.contains_key("business"));
        assert!(layers.contains_key("data"));
        assert!(layers.contains_key("shared"));
    }

    #[test]
    fn template_defaults_hexagonal_has_four_layers() {
        let layers = get_template_defaults(&ArchitectureTemplate::Hexagonal);
        assert_eq!(layers.len(), 4);
        assert!(layers.contains_key("core"));
        assert!(layers.contains_key("ports"));
    }

    #[test]
    fn template_defaults_clean_has_four_layers() {
        let layers = get_template_defaults(&ArchitectureTemplate::Clean);
        assert_eq!(layers.len(), 4);
        assert!(layers.contains_key("entities"));
        assert!(layers.contains_key("frameworks"));
    }

    #[test]
    fn template_defaults_ddd_has_four_layers() {
        let layers = get_template_defaults(&ArchitectureTemplate::Ddd);
        assert_eq!(layers.len(), 4);
        assert!(layers.contains_key("domain"));
    }

    #[test]
    fn template_defaults_starter_has_three_layers() {
        let layers = get_template_defaults(&ArchitectureTemplate::Starter);
        assert_eq!(layers.len(), 3);
    }

    #[test]
    fn template_defaults_nx_workspace_has_five_layers() {
        let layers = get_template_defaults(&ArchitectureTemplate::NxWorkspace);
        assert_eq!(layers.len(), 5);
        assert!(layers.contains_key("apps"));
        assert!(layers.contains_key("shared-libs"));
    }

    #[test]
    fn template_defaults_custom_is_empty() {
        let layers = get_template_defaults(&ArchitectureTemplate::Custom);
        assert!(layers.is_empty());
    }

    #[test]
    fn merge_with_template_fills_empty_layers() {
        let def = ArchitectureDefinition {
            schema_version: ARCHITECTURE_DEFINITION_VERSION.into(),
            template: ArchitectureTemplate::Hexagonal,
            layers: BTreeMap::new(),
            bounded_contexts: None,
            rules: vec![],
            options: None,
        };

        let merged = merge_with_template(def);
        assert!(!merged.layers.is_empty());
        assert!(merged.layers.contains_key("core"));
        assert!(merged.options.is_some());
    }

    #[test]
    fn merge_with_template_preserves_user_layers() {
        let mut user_layers = BTreeMap::new();
        user_layers.insert(
            "my_layer".into(),
            Layer {
                patterns: vec!["src/**".into()],
                depends_on: vec![],
                description: None,
            },
        );

        let def = ArchitectureDefinition {
            schema_version: ARCHITECTURE_DEFINITION_VERSION.into(),
            template: ArchitectureTemplate::Hexagonal,
            layers: user_layers,
            bounded_contexts: None,
            rules: vec![],
            options: None,
        };

        let merged = merge_with_template(def);
        assert!(merged.layers.contains_key("my_layer"));
        assert!(!merged.layers.contains_key("core"));
    }

    #[test]
    fn create_definition_from_template_populates_all_fields() {
        let def = create_definition_from_template(&ArchitectureTemplate::Serverless);
        assert_eq!(def.schema_version, ARCHITECTURE_DEFINITION_VERSION);
        assert_eq!(def.template, ArchitectureTemplate::Serverless);
        assert!(!def.layers.is_empty());
        assert!(def.options.is_some());
    }

    #[test]
    fn invalid_yaml_returns_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(ANVIL_DIR);
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join(ARCHITECTURE_YAML_FILENAME),
            ":\n  :\n  bad: [",
        )
        .unwrap();

        let result = parse_architecture_definition(tmp.path());
        assert!(matches!(result, Err(YamlParseError::InvalidYaml(_))));
    }

    // EATEST-020: Verify the 1 MiB ceiling has no off-by-one. References the
    // module-level `MAX_YAML_SIZE` directly so changes to the production
    // constant flow through to the boundary tests.
    #[allow(clippy::cast_possible_truncation)]
    const TEST_MAX_YAML_SIZE: usize = MAX_YAML_SIZE as usize;

    /// Build YAML content of exactly `size` bytes whose final line is a valid
    /// `template:` mapping (parseable as `ArchitectureDefinition`).
    fn build_padded_yaml(size: usize) -> String {
        let footer = "template: custom\n";
        assert!(size > footer.len() + 3, "size too small to pad");
        let comment_bytes = size - footer.len();
        // "# " + (comment_bytes - 3) filler chars + "\n" = comment_bytes bytes.
        let mut content = String::with_capacity(size);
        content.push_str("# ");
        content.extend(std::iter::repeat_n('a', comment_bytes - 3));
        content.push('\n');
        content.push_str(footer);
        assert_eq!(content.len(), size, "padded yaml must hit exact byte count");
        content
    }

    #[test]
    fn parse_accepts_yaml_at_exact_size_limit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(ANVIL_DIR);
        std::fs::create_dir_all(&anvil_dir).unwrap();
        let content = build_padded_yaml(TEST_MAX_YAML_SIZE);
        std::fs::write(anvil_dir.join(ARCHITECTURE_YAML_FILENAME), &content).unwrap();

        let result = parse_architecture_definition(tmp.path());
        assert!(
            result.is_ok(),
            "yaml of exactly MAX_YAML_SIZE bytes must parse: {:?}",
            result.err()
        );
        let def = result.unwrap();
        assert_eq!(def.template, ArchitectureTemplate::Custom);
    }

    #[test]
    fn parse_rejects_yaml_one_byte_over_size_limit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(ANVIL_DIR);
        std::fs::create_dir_all(&anvil_dir).unwrap();
        let content = build_padded_yaml(TEST_MAX_YAML_SIZE + 1);
        std::fs::write(anvil_dir.join(ARCHITECTURE_YAML_FILENAME), &content).unwrap();

        let result = parse_architecture_definition(tmp.path());
        match result {
            Err(YamlParseError::Io { source, .. }) => {
                assert!(
                    source.to_string().contains("read cap"),
                    "expected size-limit error, got: {source}"
                );
            }
            other => panic!("expected IO size-limit error, got: {other:?}"),
        }
    }
}
