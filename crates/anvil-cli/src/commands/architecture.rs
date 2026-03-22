#![allow(dead_code)]
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
    /// Watch for architecture violations
    Watch {
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
            "No architecture.yaml found. Create .anvil/architecture.yaml to define your architecture"
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
        .map_or(0, Vec::len);

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

fn run_watch(file: Option<&str>) -> Result<()> {
    let path = resolve_arch_config(file)?;
    let workspace_root = std::env::current_dir().context("getting current directory")?;

    let watcher_config = anvil_kernel::watcher::WatcherConfig {
        root: workspace_root.clone(),
        ..Default::default()
    };

    let watch_config = anvil_kernel::watch::WatchConfig {
        root: workspace_root,
        architecture_config: Some(path),
        watcher: watcher_config,
    };

    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let handle = anvil_kernel::watch::run_watch(&watch_config, event_tx)
        .context("starting architecture watcher")?;

    for event in &event_rx {
        use anvil_kernel_types::EventPayload;
        match &event.payload {
            EventPayload::Violation {
                policy_id,
                file,
                message,
                ..
            } => {
                eprintln!("\u{26a0} [{policy_id}] {file}: {message}");
            }
            EventPayload::Snapshot {
                node_count,
                edge_count,
                files_watched,
            } => {
                eprintln!(
                    "\u{1f4f8} {node_count} nodes, {edge_count} edges, {files_watched} files"
                );
            }
            EventPayload::Error(err) => {
                eprintln!("\u{2717} {}", err.message);
            }
            EventPayload::Progress {
                phase,
                current,
                total,
            } => {
                eprintln!("\u{25b6} {phase}: {current}/{total}");
            }
        }
    }

    handle.stop().context("stopping watcher")?;
    Ok(())
}

pub fn run(args: &ArchitectureArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        ArchitectureCommand::Validate { file } => {
            let path = resolve_arch_config(file.as_deref())?;
            let result = validate_architecture(&path)?;

            if global.json {
                crate::output::json::print(&result)?;
            } else {
                println!();
                if result.valid {
                    println!("\u{2713} Architecture configuration is valid");
                } else {
                    println!("\u{2717} Architecture configuration has errors");
                }
                println!("  Template: {}", result.template);
                println!("  Layers:   {}", result.layers);
                println!("  Rules:    {}", result.rules);
                if !result.issues.is_empty() {
                    println!();
                    for issue in &result.issues {
                        println!("  \u{26a0} {issue}");
                    }
                }
            }

            if !result.valid {
                std::process::exit(1);
            }
        }
        ArchitectureCommand::Watch { file } => {
            run_watch(file.as_deref())?;
        }
        ArchitectureCommand::Show { file } => {
            let path = resolve_arch_config(file.as_deref())?;
            let def = parse_architecture(&path)?;

            if global.json {
                crate::output::json::print(&def)?;
            } else {
                println!();
                println!("Architecture Definition");
                println!("{}", "\u{2500}".repeat(40));
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
    fn args_parses_watch() {
        let w = Wrapper::try_parse_from(["test", "watch"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_show() {
        let w = Wrapper::try_parse_from(["test", "show"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }
}
