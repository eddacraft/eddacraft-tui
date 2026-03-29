use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Source file path (for plan conversion)
    source: Option<String>,

    /// Target format for plan conversion (aps, json, yaml)
    #[arg(long)]
    to: Option<String>,

    /// Output format for constraint export (llms.txt, mcp-resource, prompt-fragment)
    #[arg(long)]
    format: Option<String>,

    /// Output file path
    #[arg(long, short)]
    output: Option<String>,

    /// Source format override (auto-detected if omitted)
    #[arg(long)]
    from: Option<String>,

    /// Compact JSON output
    #[arg(long)]
    compact: bool,
}

#[derive(Debug, Serialize)]
struct ExportResult {
    output_path: String,
    format: String,
    size_bytes: usize,
}

fn normalize_target_format(format: &str) -> String {
    format.to_lowercase()
}

fn normalize_constraint_format(format: &str) -> String {
    match format.to_lowercase().as_str() {
        "llms.txt" | "llmstxt" | "llms" => "llms.txt".to_string(),
        "mcp-resource" | "mcp" => "mcp-resource".to_string(),
        "prompt-fragment" | "prompt" => "prompt-fragment".to_string(),
        other => other.to_string(),
    }
}

fn generate_default_output(source: &str, ext: &str) -> String {
    let path = std::path::PathBuf::from(source);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let dir = path.parent().unwrap_or(std::path::Path::new("."));
    dir.join(format!("{stem}.aps.{ext}"))
        .to_string_lossy()
        .to_string()
}

fn export_plan(
    source: &str,
    target_format: String,
    output: Option<&str>,
    compact: bool,
    global: &GlobalArgs,
) -> Result<()> {
    if !std::path::Path::new(source).exists() {
        bail!("Source file not found: {source}");
    }

    let content = std::fs::read_to_string(source).with_context(|| format!("reading {source}"))?;

    let output_path = output.map(String::from).unwrap_or_else(|| {
        let default_ext = match target_format.as_str() {
            "yaml" | "yml" => "yaml",
            _ => "json",
        };
        generate_default_output(source, default_ext)
    });

    let output_dir = std::path::Path::new(&output_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("creating {}", output_dir.display()))?;

    let result_content = match target_format.as_str() {
        "yaml" | "yml" => {
            let value: serde_yaml::Value = serde_yaml::from_str(&content)
                .or_else(|_| {
                    serde_json::from_str::<serde_json::Value>(&content)
                        .map(|v| serde_yaml::to_value(v).expect("JSON to YAML conversion"))
                })
                .with_context(|| format!("parsing {source}"))?;
            serde_yaml::to_string(&value).context("serialising to YAML")?
        }
        "json" | "aps" => {
            let value: serde_json::Value = serde_yaml::from_str(&content)
                .or_else(|_| serde_json::from_str(&content))
                .with_context(|| format!("parsing {source}"))?;
            if compact {
                serde_json::to_string(&value)?
            } else {
                serde_json::to_string_pretty(&value)?
            }
        }
        other => bail!("Unsupported target format: {other}"),
    };

    std::fs::write(&output_path, &result_content)
        .with_context(|| format!("writing {output_path}"))?;

    let result = ExportResult {
        output_path: output_path.clone(),
        format: target_format.to_string(),
        size_bytes: result_content.len(),
    };

    if global.json {
        crate::output::json::print(&result)?;
    } else {
        println!("Exported to {}", target_format.to_uppercase());
        println!("  Output: {output_path}");
        println!("  Size:   {} bytes", result_content.len());
    }

    Ok(())
}

fn export_constraints(format: &str, _output: Option<&str>, _global: &GlobalArgs) -> Result<()> {
    let normalized = normalize_constraint_format(format);
    match normalized.as_str() {
        "llms.txt" | "mcp-resource" | "prompt-fragment" => {}
        other => {
            bail!("Unsupported format: {other}. Supported: llms.txt, mcp-resource, prompt-fragment")
        }
    }
    bail!(
        "Constraint export as '{normalized}' is not yet implemented. \
         This feature is planned but generating placeholder output could be mistaken for real constraints."
    )
}

pub fn run(args: &ExportArgs, global: &GlobalArgs) -> Result<()> {
    if let Some(format) = &args.format {
        export_constraints(format, args.output.as_deref(), global)
    } else if let Some(to) = &args.to {
        let source = args
            .source
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Source file path is required for plan conversion"))?;
        let target = normalize_target_format(to);
        export_plan(source, target, args.output.as_deref(), args.compact, global)
    } else {
        bail!(
            "Either --format or --to must be specified\n\nExamples:\n  Constraint export: anvil export --format llms.txt\n  Plan conversion:   anvil export source.yaml --to json"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: ExportArgs,
    }

    #[test]
    fn args_parses_empty() {
        let w = Wrapper::try_parse_from(["test"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_with_to() {
        let w = Wrapper::try_parse_from(["test", "file.md", "--to", "json"]).unwrap();
        assert_eq!(w.inner.to.as_deref(), Some("json"));
    }

    #[test]
    fn args_parses_with_format() {
        let w = Wrapper::try_parse_from(["test", "--format", "llms.txt"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn normalize_target_handles_known() {
        assert_eq!(normalize_target_format("json"), "json");
        assert_eq!(normalize_target_format("yaml"), "yaml");
    }
}
