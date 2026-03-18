use std::fs;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anvil_tui::surfaces::init::{AvailableCheck, InitState};
use anyhow::Context;
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Overwrite existing configuration without prompting.
    #[arg(long)]
    force: bool,
}

/// Default checks enabled in a fresh `.anvilrc`.
const DEFAULT_CHECKS: &[&str] = &["secret-detection", "import-boundaries"];

/// Schema version for generated `.anvilrc` files.
const SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnvilConfig {
    schema_version: String,
    planning_dir: String,
    format: String,
    checks: Vec<String>,
}

impl Default for AnvilConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            planning_dir: "plans".to_string(),
            format: "yaml".to_string(),
            checks: DEFAULT_CHECKS.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

pub fn run(args: &InitArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let root = PathBuf::from(".");
    run_in(args, global, &root)
}

fn run_in(args: &InitArgs, global: &GlobalArgs, root: &Path) -> anyhow::Result<()> {
    let config_path = root.join(".anvilrc");

    if config_path.exists() && !args.force {
        anyhow::bail!(".anvilrc already exists. Use --force to overwrite.");
    }

    if global.no_tui || !std::io::stdout().is_terminal() {
        run_plain(root)?;
    } else {
        run_tui(root)?;
    }

    Ok(())
}

fn run_tui(root: &Path) -> anyhow::Result<()> {
    let available = default_available_checks();
    let state = InitState::new(available);
    let state = crate::tui::run_surface(state)?;

    if !state.confirmed {
        println!("Init cancelled.");
        return Ok(());
    }

    let checks: Vec<String> = if state.config.checks.is_empty() {
        DEFAULT_CHECKS.iter().map(|s| (*s).to_string()).collect()
    } else {
        state.config.checks
    };

    let config = AnvilConfig {
        schema_version: SCHEMA_VERSION.to_string(),
        planning_dir: state.config.directory.clone(),
        format: format_label(state.config.format),
        checks: checks.clone(),
    };

    generate_config(&config, root)?;
    print_success(&config.planning_dir, &checks);
    Ok(())
}

fn run_plain(root: &Path) -> anyhow::Result<()> {
    let config = AnvilConfig::default();
    generate_config(&config, root)?;
    print_success(&config.planning_dir, &config.checks);
    Ok(())
}

fn generate_config(config: &AnvilConfig, root: &Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(config).context("failed to serialise config")?;
    fs::write(root.join(".anvilrc"), json).context("failed to write .anvilrc")?;

    fs::create_dir_all(root.join(".anvil/cache")).context("failed to create .anvil/cache/")?;

    append_gitignore_entry(root)?;

    let planning_dir = root.join(&config.planning_dir);
    if !planning_dir.exists() {
        fs::create_dir_all(&planning_dir)
            .with_context(|| format!("failed to create {}/", config.planning_dir))?;
    }

    Ok(())
}

fn append_gitignore_entry(root: &Path) -> anyhow::Result<()> {
    let gitignore = root.join(".gitignore");
    let entry = ".anvil/cache/";

    if gitignore.exists() {
        let file = fs::File::open(&gitignore).context("failed to read .gitignore")?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.context("failed to read .gitignore line")?;
            if line.trim() == entry {
                return Ok(());
            }
        }
    }

    let needs_newline = if gitignore.exists() {
        let contents = fs::read_to_string(&gitignore).unwrap_or_default();
        !contents.is_empty() && !contents.ends_with('\n')
    } else {
        false
    };

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore)
        .context("failed to open .gitignore for appending")?;

    if needs_newline {
        writeln!(file).context("failed to write newline to .gitignore")?;
    }

    writeln!(file, "{entry}").context("failed to append to .gitignore")?;
    Ok(())
}

fn print_success(planning_dir: &str, checks: &[String]) {
    println!();
    println!("Anvil initialised successfully.");
    println!("  Config:    .anvilrc");
    println!("  Plans:     {planning_dir}/");
    println!("  Checks:    {}", checks.join(", "));
    println!("Run `anvil doctor` to verify your setup.");
    println!();
}

fn format_label(fmt: anvil_tui::surfaces::init::ConfigFormat) -> String {
    match fmt {
        anvil_tui::surfaces::init::ConfigFormat::Yaml => "yaml".to_string(),
        anvil_tui::surfaces::init::ConfigFormat::Json => "json".to_string(),
        anvil_tui::surfaces::init::ConfigFormat::Toml => "toml".to_string(),
    }
}

fn default_available_checks() -> Vec<AvailableCheck> {
    vec![
        AvailableCheck {
            name: "secret-detection".to_string(),
            description: "Detect leaked secrets and credentials".to_string(),
            enabled: true,
        },
        AvailableCheck {
            name: "import-boundaries".to_string(),
            description: "Enforce module import boundaries".to_string(),
            enabled: true,
        },
        AvailableCheck {
            name: "antipattern-scan".to_string(),
            description: "Detect common code antipatterns".to_string(),
            enabled: false,
        },
        AvailableCheck {
            name: "architecture".to_string(),
            description: "Validate architecture definitions".to_string(),
            enabled: false,
        },
        AvailableCheck {
            name: "policy".to_string(),
            description: "Evaluate OPA policy rules".to_string(),
            enabled: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_tui_global() -> GlobalArgs {
        GlobalArgs {
            json: false,
            no_tui: true,
            verbose: false,
        }
    }

    #[test]
    fn creates_anvilrc_and_anvil_dir() {
        let dir = tempfile::tempdir().unwrap();

        let result = run_plain(dir.path());

        assert!(result.is_ok());
        assert!(dir.path().join(".anvilrc").exists());
        assert!(dir.path().join(".anvil").is_dir());
        assert!(dir.path().join(".anvil/cache").is_dir());
        assert!(dir.path().join("plans").is_dir());

        let content = fs::read_to_string(dir.path().join(".anvilrc")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["schemaVersion"], "1.0.0");
        assert_eq!(parsed["planningDir"], "plans");
        assert_eq!(parsed["format"], "yaml");
        assert!(parsed["checks"].as_array().unwrap().len() >= 2);

        let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains(".anvil/cache/"));
    }

    #[test]
    fn existing_anvilrc_blocks_without_force() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".anvilrc"), "{}").unwrap();

        let args = InitArgs { force: false };
        let global = no_tui_global();
        let result = run_in(&args, &global, dir.path());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already exists"), "got: {err}");
    }

    #[test]
    fn force_overwrites_existing_anvilrc() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".anvilrc"), r#"{"old": true}"#).unwrap();

        let args = InitArgs { force: true };
        let global = no_tui_global();
        let result = run_in(&args, &global, dir.path());
        assert!(result.is_ok());

        let content = fs::read_to_string(dir.path().join(".anvilrc")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["schemaVersion"], "1.0.0");
        assert!(!content.contains("old"));
    }

    #[test]
    fn gitignore_not_duplicated() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), ".anvil/cache/\n").unwrap();

        append_gitignore_entry(dir.path()).unwrap();

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        let count = content.matches(".anvil/cache/").count();
        assert_eq!(count, 1, "entry should not be duplicated");
    }

    #[test]
    fn default_config_has_expected_shape() {
        let config = AnvilConfig::default();
        assert_eq!(config.schema_version, "1.0.0");
        assert_eq!(config.planning_dir, "plans");
        assert_eq!(config.format, "yaml");
        assert_eq!(config.checks.len(), 2);
        assert!(config.checks.contains(&"secret-detection".to_string()));
        assert!(config.checks.contains(&"import-boundaries".to_string()));
    }

    #[test]
    fn default_available_checks_count() {
        let checks = default_available_checks();
        assert_eq!(checks.len(), 5);
        assert!(checks[0].enabled);
        assert!(checks[1].enabled);
        assert!(!checks[2].enabled);
    }
}
