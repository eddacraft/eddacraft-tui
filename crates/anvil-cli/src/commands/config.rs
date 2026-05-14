use std::path::Path;

use anvil_config::{ConfigFormat, RuleModes, discover, parse_file, parse_str};
use anyhow::{Context, bail};
use clap::{Args, Subcommand};
use serde_json::{Map, Value};

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show the effective Anvil config.
    Show,
    /// Set a rule mode in the project config.
    Set { rule: String, mode: String },
    /// Convert the project config to another format and print it.
    Convert {
        #[arg(long)]
        to: String,
    },
}

pub fn run(args: &ConfigArgs, _global: &GlobalArgs) -> anyhow::Result<()> {
    let root = Path::new(".");
    match &args.command {
        ConfigCommand::Show => print!("{}", show_config(root)?),
        ConfigCommand::Set { rule, mode } => {
            set_rule_mode(root, rule, mode)?;
            println!("set {rule}={mode}");
        }
        ConfigCommand::Convert { to } => print!("{}", convert_config(root, to)?),
    }
    Ok(())
}

fn show_config(root: &Path) -> anyhow::Result<String> {
    let (label, value) = load_project_config(root)?;
    let modes = RuleModes::from_value(&value)?;
    Ok(format!(
        "config: {label}\nrule modes: {}\n",
        modes.summary()
    ))
}

fn set_rule_mode(root: &Path, rule: &str, mode: &str) -> anyhow::Result<()> {
    if !matches!(
        rule,
        "public-api-expansion"
            | "new-dependency-introduction"
            | "cross-layer-violation"
            | "privilege-expansion"
    ) {
        bail!(
            "unknown rule `{rule}`; expected one of public-api-expansion, new-dependency-introduction, cross-layer-violation, privilege-expansion"
        );
    }

    let (_, mut value) = load_project_config(root)?;
    ensure_rule_mode(&mut value, rule, mode);
    RuleModes::from_value(&value).with_context(|| format!("invalid rule mode `{mode}`"))?;

    let (path, format) = writable_config_path(root)?;
    let text = serialize_config(&value, format)?;
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn convert_config(root: &Path, format: &str) -> anyhow::Result<String> {
    let (_, value) = load_project_config(root)?;
    let format = parse_output_format(format)?;
    serialize_config(&value, format)
}

fn load_project_config(root: &Path) -> anyhow::Result<(String, Value)> {
    if let Some(discovered) = discover(root, ".anvil")? {
        let value = parse_file(&discovered.path)?;
        let label = discovered
            .path
            .strip_prefix(root)
            .unwrap_or(&discovered.path)
            .to_string_lossy()
            .into_owned();
        return Ok((label, value));
    }

    let rc_path = root.join(".anvilrc");
    match std::fs::read_to_string(&rc_path) {
        Ok(contents) => {
            let value = serde_json::from_str(&contents)
                .or_else(|_| parse_str(&contents, ConfigFormat::Yaml, &rc_path))?;
            Ok((String::from(".anvilrc"), value))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok((String::from("defaults"), serde_json::json!({})))
        }
        Err(error) => Err(error).context("reading .anvilrc"),
    }
}

fn writable_config_path(root: &Path) -> anyhow::Result<(std::path::PathBuf, ConfigFormat)> {
    if let Some(discovered) = discover(root, ".anvil")? {
        return Ok((discovered.path, discovered.format));
    }
    Ok((root.join(".anvil.yaml"), ConfigFormat::Yaml))
}

fn ensure_rule_mode(value: &mut Value, rule: &str, mode: &str) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    let root = value.as_object_mut().expect("object normalised above");
    let enforcement = root
        .entry("enforcement")
        .or_insert_with(|| Value::Object(Map::new()));
    if !enforcement.is_object() {
        *enforcement = Value::Object(Map::new());
    }
    let enforcement = enforcement
        .as_object_mut()
        .expect("object normalised above");
    let rules = enforcement
        .entry("rules")
        .or_insert_with(|| Value::Object(Map::new()));
    if !rules.is_object() {
        *rules = Value::Object(Map::new());
    }
    let rules = rules.as_object_mut().expect("object normalised above");
    rules.insert(
        rule.to_string(),
        serde_json::json!({
            "mode": mode,
        }),
    );
}

fn parse_output_format(raw: &str) -> anyhow::Result<ConfigFormat> {
    match raw.to_ascii_lowercase().as_str() {
        "yaml" => Ok(ConfigFormat::Yaml),
        "yml" => Ok(ConfigFormat::Yml),
        "json" => Ok(ConfigFormat::Json),
        "toml" => Ok(ConfigFormat::Toml),
        other => bail!("unsupported config format `{other}`; expected yaml, yml, json, or toml"),
    }
}

fn serialize_config(value: &Value, format: ConfigFormat) -> anyhow::Result<String> {
    match format {
        ConfigFormat::Yaml | ConfigFormat::Yml => {
            serde_yaml::to_string(value).context("serialising config as yaml")
        }
        ConfigFormat::Json => serde_json::to_string_pretty(value)
            .map(|mut text| {
                text.push('\n');
                text
            })
            .context("serialising config as json"),
        ConfigFormat::Toml => toml::to_string_pretty(value).context("serialising config as toml"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn show_reports_default_rule_modes_when_config_is_missing() {
        let tmp = TempDir::new().unwrap();

        let output = show_config(tmp.path()).unwrap();

        assert!(output.contains("config: defaults"));
        assert!(output.contains("public-api-expansion=warn"));
    }

    #[test]
    fn set_creates_yaml_config_and_preserves_rule_mode() {
        let tmp = TempDir::new().unwrap();

        set_rule_mode(tmp.path(), "public-api-expansion", "enforce").unwrap();

        let output = show_config(tmp.path()).unwrap();
        assert!(output.contains("config: .anvil.yaml"));
        assert!(output.contains("public-api-expansion=enforce"));
    }

    #[test]
    fn set_preserves_existing_json_config_format() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.json"), "{}").unwrap();

        set_rule_mode(tmp.path(), "public-api-expansion", "enforce").unwrap();

        let contents = std::fs::read_to_string(tmp.path().join(".anvil.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(
            parsed["enforcement"]["rules"]["public-api-expansion"]["mode"],
            "enforce"
        );
    }

    #[test]
    fn set_preserves_existing_toml_config_format() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.toml"), "").unwrap();

        set_rule_mode(tmp.path(), "public-api-expansion", "enforce").unwrap();

        let contents = std::fs::read_to_string(tmp.path().join(".anvil.toml")).unwrap();
        assert!(contents.contains("[enforcement.rules.public-api-expansion]"));
        assert!(contents.contains("mode = \"enforce\""));
    }

    #[test]
    fn convert_prints_requested_format_without_writing_start_flags() {
        let tmp = TempDir::new().unwrap();
        set_rule_mode(tmp.path(), "new-dependency-introduction", "off").unwrap();

        let output = convert_config(tmp.path(), "toml").unwrap();

        assert!(output.contains("[enforcement.rules.new-dependency-introduction]"));
        assert!(output.contains("mode = \"off\""));
    }
}
