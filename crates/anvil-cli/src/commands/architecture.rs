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

const ARCH_CONFIG_FILENAME: &str = "architecture.yaml";

fn resolve_arch_config(file: Option<&str>) -> Result<PathBuf> {
    if let Some(f) = file {
        let p = PathBuf::from(f);
        if !p.exists() {
            bail!("Architecture config not found: {f}");
        }
        return Ok(p);
    }

    let workspace = std::env::current_dir().context("getting current directory")?;
    let path = workspace.join(".anvil").join(ARCH_CONFIG_FILENAME);
    if !path.exists() {
        bail!(
            "No architecture.yaml found.\n  Create .anvil/architecture.yaml manually or see: anvil architecture --help"
        );
    }
    Ok(path)
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
}

fn parse_architecture(path: &std::path::Path) -> Result<ArchDefinition> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

    let value: serde_yaml::Value =
        serde_yaml::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;

    let template = value
        .get("template")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let mut layers = Vec::new();
    if let Some(layers_map) = value.get("layers").and_then(|v| v.as_mapping()) {
        for (name, def) in layers_map {
            let name_str = name.as_str().unwrap_or("unknown").to_string();
            let patterns = def
                .get("patterns")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let depends_on = def
                .get("depends_on")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            layers.push(LayerInfo {
                name: name_str,
                patterns,
                depends_on,
            });
        }
    }

    let rules_count = value
        .get("rules")
        .and_then(|v| v.as_sequence())
        .map_or(0, std::vec::Vec::len);

    Ok(ArchDefinition {
        template,
        layers,
        rules_count,
    })
}

fn validate_architecture(path: &std::path::Path) -> Result<ValidationResult> {
    let def = parse_architecture(path)?;
    let mut issues = Vec::new();

    for layer in &def.layers {
        for dep in &layer.depends_on {
            if !def.layers.iter().any(|l| l.name == *dep) {
                issues.push(format!(
                    "Layer \"{}\" depends on unknown layer \"{}\"",
                    layer.name, dep
                ));
            }
        }
    }

    Ok(ValidationResult {
        valid: issues.is_empty(),
        template: def.template,
        layers: def.layers.len(),
        rules: def.rules_count,
        issues,
    })
}

pub fn run(args: &ArchitectureArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        ArchitectureCommand::Validate { file } => {
            let path = resolve_arch_config(file.as_deref())?;
            let result = validate_architecture(&path)?;

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
                        crate::output::plain::warn(issue);
                    }
                }
            }

            if !result.valid {
                return Err(crate::output::AlreadyReported.into());
            }
        }
        ArchitectureCommand::Show { file } => {
            let path = resolve_arch_config(file.as_deref())?;
            let def = parse_architecture(&path)?;

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

    #[test]
    fn resolve_arch_config_explicit_file_found() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(&file, "template: custom").unwrap();
        let result = resolve_arch_config(Some(file.to_str().unwrap())).unwrap();
        assert_eq!(result, file);
    }

    #[test]
    fn resolve_arch_config_explicit_file_missing() {
        let err = resolve_arch_config(Some("/nonexistent/arch.yaml")).unwrap_err();
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
            "template: layered\nlayers:\n  ui:\n    patterns:\n      - \"src/ui/**\"\n    depends_on:\n      - domain\n  domain:\n    patterns:\n      - \"src/domain/**\"\n",
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
            "template: custom\nrules:\n  - name: no-cross-import\n  - name: no-circular\n",
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
        assert_eq!(def.template, "unknown");
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
            "template: layered\nlayers:\n  ui:\n    patterns: [\"src/ui/**\"]\n    depends_on: [domain]\n  domain:\n    patterns: [\"src/domain/**\"]\n",
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
    fn validate_no_layers_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arch.yaml");
        std::fs::write(&file, "template: empty\n").unwrap();

        let result = validate_architecture(&file).unwrap();
        assert!(result.valid);
        assert_eq!(result.layers, 0);
    }
}
