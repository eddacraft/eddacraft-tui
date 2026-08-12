use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct ArchitectureArgs {
    #[command(subcommand)]
    command: ArchitectureCommand,
}

#[derive(Debug, clap::Subcommand)]
enum ArchitectureCommand {
    /// Validate architecture definition
    Validate {
        /// Architecture config file
        #[arg(long, short)]
        file: Option<String>,
    },
    /// Show current architecture definition
    Show {
        /// Architecture config file
        #[arg(long, short)]
        file: Option<String>,
    },
}

/// Load the definition the command operates on: an explicit `--file`
/// path, or the project's resolved architecture (main-config section —
/// inline or source-delegated — with the standalone
/// `.anvil/architecture.yaml` as the legacy fallback).
fn load_definition(file: Option<&str>) -> Result<anvil_architecture::ArchitectureDefinition> {
    if let Some(f) = file {
        let p = PathBuf::from(f);
        if !p.exists() {
            bail!("Architecture config not found: {f}");
        }
        return anvil_architecture::parse_architecture_definition_file(&p)
            .with_context(|| format!("parsing {}", p.display()));
    }

    let workspace = std::env::current_dir().context("getting current directory")?;
    match crate::architecture_source::resolve_architecture(&workspace)? {
        Some((definition, _origin)) => Ok(definition),
        None => bail!(
            "No architecture config found.\n  Add an `architecture` section to your \
             project config, or create .anvil/architecture.yaml — see: anvil \
             architecture --help"
        ),
    }
}

#[derive(Debug, Serialize)]
struct LayerInfo {
    name: String,
    patterns: Vec<String>,
    depends_on: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ArchDefinition {
    template: String,
    layers: Vec<LayerInfo>,
    rules_count: usize,
}

#[derive(Debug, Serialize)]
struct ValidationResult {
    valid: bool,
    template: String,
    layers: usize,
    rules: usize,
    issues: Vec<String>,
    warnings: Vec<String>,
    diagnostics: Vec<anvil_architecture::ArchitectureDefinitionDiagnostic>,
}

fn validate_architecture(
    definition: &anvil_architecture::ArchitectureDefinition,
) -> ValidationResult {
    let def = ArchDefinition::from_definition(definition);
    let diagnostics = anvil_architecture::diagnose_definition(definition);
    let issues = diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect::<Vec<_>>();
    let warnings = diagnostics
        .iter()
        .filter(|d| !d.is_error())
        .map(|d| d.message.clone())
        .collect::<Vec<_>>();

    ValidationResult {
        valid: issues.is_empty(),
        template: def.template,
        layers: def.layers.len(),
        rules: def.rules_count,
        issues,
        warnings,
        diagnostics,
    }
}

impl ArchDefinition {
    fn from_definition(definition: &anvil_architecture::ArchitectureDefinition) -> Self {
        let layers = definition
            .layers
            .iter()
            .map(|(name, layer)| LayerInfo {
                name: name.clone(),
                patterns: layer.patterns.clone(),
                depends_on: layer.depends_on.clone(),
            })
            .collect();

        Self {
            template: definition.template.to_string(),
            layers,
            rules_count: definition.rules.len(),
        }
    }
}

pub fn run(args: &ArchitectureArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        ArchitectureCommand::Validate { file } => {
            let definition = load_definition(file.as_deref())?;
            let result = validate_architecture(&definition);

            if global.json {
                crate::output::json::print(&result)?;
            } else {
                crate::output::plain::blank();
                if result.valid {
                    crate::output::plain::success("Architecture configuration is valid");
                } else {
                    crate::output::plain::error("Architecture configuration has errors");
                }
                println!("  Template: {}", result.template);
                println!("  Layers:   {}", result.layers);
                println!("  Rules:    {}", result.rules);
                if !result.issues.is_empty() {
                    println!();
                    for issue in &result.issues {
                        crate::output::plain::error(issue);
                    }
                }
                if !result.warnings.is_empty() {
                    println!();
                    for warning in &result.warnings {
                        crate::output::plain::warn(warning);
                    }
                }
            }

            if !result.valid {
                return Err(crate::output::AlreadyReported.into());
            }
        }
        ArchitectureCommand::Show { file } => {
            let definition = load_definition(file.as_deref())?;
            let def = ArchDefinition::from_definition(&definition);

            if global.json {
                crate::output::json::print(&def)?;
            } else {
                crate::output::plain::blank();
                crate::output::plain::section("Architecture Definition");
                println!("  Template: {}", def.template);
                println!();
                println!("  Layers");
                for layer in &def.layers {
                    println!("    {}", layer.name);
                    println!("      Patterns:   {}", layer.patterns.join(", "));
                    let deps = if layer.depends_on.is_empty() {
                        "(none)".to_string()
                    } else {
                        layer.depends_on.join(", ")
                    };
                    println!("      Depends on: {deps}");
                }
                if def.rules_count > 0 {
                    println!();
                    println!("  Rules: {}", def.rules_count);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: ArchitectureArgs,
    }

    #[test]
    fn args_parses_validate() {
        let w = Wrapper::try_parse_from(["test", "validate"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_show() {
        let w = Wrapper::try_parse_from(["test", "show"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_validate_with_file() {
        let w = Wrapper::try_parse_from(["test", "validate", "--file", "arch.yaml"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_show_with_file() {
        let w = Wrapper::try_parse_from(["test", "show", "-f", "arch.yaml"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    // Path-based wrappers keeping the pre-UCFG-008 test corpus
    // exercising the same file-parse semantics `--file` uses.
    fn parse_architecture(path: &std::path::Path) -> Result<ArchDefinition> {
        let definition = anvil_architecture::parse_architecture_definition_file(path)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(ArchDefinition::from_definition(&definition))
    }

    fn validate_architecture(path: &std::path::Path) -> Result<ValidationResult> {
        let definition = anvil_architecture::parse_architecture_definition_file(path)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(super::validate_architecture(&definition))
    }

    #[test]
    fn load_definition_explicit_file_found() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(&file, "template: custom").unwrap();
        let definition = load_definition(Some(file.to_str().unwrap())).unwrap();
        assert_eq!(definition.template.to_string(), "custom");
    }

    #[test]
    fn load_definition_explicit_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let missing_file = dir.path().join("arch.yaml");
        let err = load_definition(Some(missing_file.to_str().unwrap())).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn parse_minimal_architecture() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(&file, "template: layered\n").unwrap();

        let def = parse_architecture(&file).unwrap();
        assert_eq!(def.template, "layered");
        assert!(def.layers.is_empty());
        assert_eq!(def.rules_count, 0);
    }

    #[test]
    fn parse_architecture_with_layers() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(
            &file,
            "template: layered\nlayers:\n  ui:\n    patterns:\n      - \"src/ui/**\"\n    depends_on:\n      - domain\n  domain:\n    patterns:\n      - \"src/domain/**\"\n    depends_on: []\n",
        )
        .unwrap();

        let def = parse_architecture(&file).unwrap();
        assert_eq!(def.template, "layered");
        assert_eq!(def.layers.len(), 2);

        let ui = def.layers.iter().find(|l| l.name == "ui").unwrap();
        assert_eq!(ui.patterns, vec!["src/ui/**"]);
        assert_eq!(ui.depends_on, vec!["domain"]);

        let domain = def.layers.iter().find(|l| l.name == "domain").unwrap();
        assert!(domain.depends_on.is_empty());
    }

    #[test]
    fn parse_architecture_with_rules() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(
            &file,
            "template: custom\nlayers:\n  ui:\n    patterns: [\"src/ui/**\"]\n    depends_on: []\n  domain:\n    patterns: [\"src/domain/**\"]\n    depends_on: []\nrules:\n  - name: no-cross-import\n    from: ui\n    to: domain\n  - name: no-circular\n    from: domain\n    to: ui\n",
        )
        .unwrap();

        let def = parse_architecture(&file).unwrap();
        assert_eq!(def.rules_count, 2);
    }

    #[test]
    fn parse_architecture_defaults_unknown_template() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(&file, "layers: {}").unwrap();

        let def = parse_architecture(&file).unwrap();
        assert_eq!(def.template, "custom");
    }

    #[test]
    fn parse_architecture_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(&file, ": [invalid yaml {{").unwrap();

        assert!(parse_architecture(&file).is_err());
    }

    #[test]
    fn validate_valid_architecture() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(
            &file,
            "template: layered\nlayers:\n  ui:\n    patterns: [\"src/ui/**\"]\n    depends_on: [domain]\n  domain:\n    patterns: [\"src/domain/**\"]\n    depends_on: []\n",
        )
        .unwrap();

        let result = validate_architecture(&file).unwrap();
        assert!(result.valid);
        assert!(result.issues.is_empty());
        assert_eq!(result.layers, 2);
    }

    #[test]
    fn validate_detects_unknown_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(
            &file,
            "template: layered\nlayers:\n  ui:\n    patterns: [\"src/ui/**\"]\n    depends_on: [nonexistent]\n",
        )
        .unwrap();

        let result = validate_architecture(&file).unwrap();
        assert!(!result.valid);
        assert_eq!(result.issues.len(), 1);
        assert!(result.issues[0].contains("nonexistent"));
    }

    #[test]
    fn architecture_validate_maps_unknown_dependency_to_layer_key() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(
            &file,
            "template: layered\nlayers:\n  ui:\n    patterns: [\"src/ui/**\"]\n    depends_on: [domain]\n",
        )
        .unwrap();

        let result = validate_architecture(&file).unwrap();

        assert!(!result.valid);
        assert!(
            result.diagnostics.iter().any(|d| {
                d.code == "unknown-layer-dependency"
                    && d.section == "layers.ui.depends_on"
                    && d.key == "domain"
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn architecture_validate_blocks_overlapping_layer_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(
            &file,
            "template: layered\nlayers:\n  app:\n    patterns: [\"src/**\"]\n    depends_on: []\n  ui:\n    patterns: [\"src/ui/**\"]\n    depends_on: []\n",
        )
        .unwrap();

        let result = validate_architecture(&file).unwrap();

        assert!(!result.valid);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "overlapping-layer-patterns" && d.is_error())
        );
    }

    #[test]
    fn architecture_validate_warns_for_empty_layer_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(
            &file,
            "template: layered\nlayers:\n  empty:\n    patterns: []\n    depends_on: []\n",
        )
        .unwrap();

        let result = validate_architecture(&file).unwrap();

        assert!(result.valid);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "empty-layer" && !d.is_error())
        );
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn architecture_validate_rejects_wrong_typed_layers() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(&file, "template: custom\nlayers: definitely-not-a-map\n").unwrap();

        let err = validate_architecture(&file).unwrap_err();

        assert!(err.to_string().contains("parsing"));
    }

    #[test]
    fn architecture_validate_rejects_over_cap_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        let max_yaml_size = usize::try_from(anvil_architecture::ARCHITECTURE_YAML_MAX_SIZE)
            .expect("architecture YAML size cap fits usize");
        let content = " ".repeat(max_yaml_size + 1);
        std::fs::write(&file, content).unwrap();

        let err = validate_architecture(&file).unwrap_err();

        let error_chain = format!("{err:#}");
        assert!(error_chain.contains("exceeds"), "{error_chain}");
    }

    #[test]
    fn architecture_validate_maps_unknown_rule_layer_to_rule_id() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(
            &file,
            "template: layered\nlayers:\n  ui:\n    patterns: [\"src/ui/**\"]\n    depends_on: []\nrules:\n  - name: no-ui-to-domain\n    from: ui\n    to: domain\n",
        )
        .unwrap();

        let result = validate_architecture(&file).unwrap();

        assert!(!result.valid);
        assert!(
            result.diagnostics.iter().any(|d| {
                d.code == "unknown-rule-layer"
                    && d.section == "rules.no-ui-to-domain"
                    && d.key == "domain"
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn validate_no_layers_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(&file, "template: custom\n").unwrap();

        let result = validate_architecture(&file).unwrap();
        assert!(result.valid);
        assert_eq!(result.layers, 0);
    }
}
