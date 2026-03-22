#![allow(dead_code)]
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
    let lower = format.to_lowercase();
    match lower.as_str() {
        "yml" => "yaml".to_string(),
        _ => lower,
    }
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
    let mut stem_path = path.clone();
    if stem_path
        .extension()
        .is_some_and(|extname| extname.eq_ignore_ascii_case("md"))
    {
        stem_path = stem_path.with_extension("");
    }
    let stem = stem_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    dir.join(format!("{stem}.aps.{ext}"))
        .to_string_lossy()
        .to_string()
}

fn export_plan(
    source: &str,
    target_format: &str,
    output: Option<&str>,
    from: Option<&str>,
    compact: bool,
    global: &GlobalArgs,
) -> Result<()> {
    if !std::path::Path::new(source).exists() {
        bail!("Source file not found: {source}");
    }
    if from.is_some() {
        bail!("--from override is not yet supported in the Rust CLI export command");
    }
    if std::path::Path::new(source)
        .extension()
        .is_some_and(|extname| extname.eq_ignore_ascii_case("md"))
    {
        bail!(
            "Markdown/APS plan export is not yet implemented in the Rust CLI; use YAML/JSON sources only"
        );
    }

    let content = std::fs::read_to_string(source).with_context(|| format!("reading {source}"))?;
    let target_ext = if target_format == "yaml" {
        "yaml"
    } else {
        "json"
    };

    let output_path =
        output.map_or_else(|| generate_default_output(source, target_ext), String::from);

    let output_dir = std::path::Path::new(&output_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("creating {}", output_dir.display()))?;

    let result_content = match target_format {
        "yaml" => {
            let value: serde_yaml::Value = if let Ok(v) = serde_yaml::from_str(&content) {
                v
            } else {
                let json: serde_json::Value = serde_json::from_str(&content)
                    .with_context(|| format!("parsing {source} as JSON or YAML"))?;
                serde_yaml::to_value(json).context("converting JSON to YAML value")?
            };
            serde_yaml::to_string(&value).context("serialising to YAML")?
        }
        "json" | "aps" => {
            let value: serde_json::Value = serde_yaml::from_str(&content)
                .or_else(|_| serde_json::from_str(&content))
                .with_context(|| format!("parsing {source} as YAML or JSON"))?;
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

fn export_constraints(format: &str, output: Option<&str>, global: &GlobalArgs) -> Result<()> {
    let normalized = normalize_constraint_format(format);
    let workspace_root = if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        && output.status.success()
    {
        std::path::PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
    } else {
        std::env::current_dir().context("getting current directory")?
    };

    let default_filename = match normalized.as_str() {
        "llms.txt" => ".llms.txt",
        "mcp-resource" => "anvil-constraints.mcp.json",
        "prompt-fragment" => "anvil-constraints-prompt.txt",
        other => {
            bail!("Unsupported format: {other}. Supported: llms.txt, mcp-resource, prompt-fragment")
        }
    };

    let _ = (output, global, workspace_root, default_filename);
    bail!("Constraint export for '{normalized}' is not yet implemented in the Rust CLI")
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
        export_plan(
            source,
            &target,
            args.output.as_deref(),
            args.from.as_deref(),
            args.compact,
            global,
        )
    } else {
        bail!(
            "Either --format or --to must be specified\n\nExamples:\n  Constraint export: anvil export --format llms.txt\n  Data conversion:   anvil export source.yaml --to json"
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
    fn normalize_target_canonicalises_yml() {
        assert_eq!(normalize_target_format("yml"), "yaml");
        assert_eq!(normalize_target_format("YML"), "yaml");
        assert_eq!(normalize_target_format("yaml"), "yaml");
        assert_eq!(normalize_target_format("json"), "json");
    }

    #[test]
    fn generate_output_handles_dotted_stems() {
        let result = generate_default_output("plans/my.plan.md", "json");
        assert_eq!(result, "plans/my.aps.json");
    }

    #[test]
    fn generate_output_handles_bare_filename() {
        let result = generate_default_output("plan.md", "yaml");
        assert_eq!(result, "./plan.aps.yaml");
    }

    #[test]
    fn args_parses_with_from() {
        let w = Wrapper::try_parse_from(["test", "file.yaml", "--to", "json", "--from", "yaml"])
            .unwrap();
        assert_eq!(w.inner.from.as_deref(), Some("yaml"));
    }
}
