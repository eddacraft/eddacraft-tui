use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::GlobalArgs;
use crate::output::{self, OutputMode};

const ANVIL_DIR: &str = ".anvil";
const CONFIG_FILENAME: &str = "gate-config.json";

// TODO(RCLI2): --interactive mode deferred (needs crossterm prompts).

#[derive(Debug, Args)]
pub struct GateConfigArgs {
    /// List current gate configuration.
    #[arg(long, short, conflicts_with_all = ["enable", "disable"])]
    list: bool,

    /// Enable a specific check.
    #[arg(long, short, conflicts_with = "disable")]
    enable: Option<String>,

    /// Disable a specific check.
    #[arg(long, short, conflicts_with = "enable")]
    disable: Option<String>,
}

// ── Config types (parity with Node.js GateConfig) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    pub version: u32,
    pub checks: Vec<GateCheck>,
    pub thresholds: BTreeMap<String, u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_config: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCheck {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, serde_json::Value>>,
}

/// Default gate checks matching the Node.js defaults.
fn default_config() -> GateConfig {
    GateConfig {
        version: 1,
        checks: vec![
            GateCheck {
                name: "lint".to_string(),
                description: "Code quality and style checks".to_string(),
                enabled: true,
                config: None,
            },
            GateCheck {
                name: "test".to_string(),
                description: "Test suite execution".to_string(),
                enabled: true,
                config: None,
            },
            GateCheck {
                name: "coverage".to_string(),
                description: "Code coverage thresholds".to_string(),
                enabled: false,
                config: None,
            },
            GateCheck {
                name: "dependency".to_string(),
                description: "Dependency vulnerability scanning".to_string(),
                enabled: true,
                config: None,
            },
            GateCheck {
                name: "secret".to_string(),
                description: "Secret and credential detection".to_string(),
                enabled: true,
                config: None,
            },
            GateCheck {
                name: "architecture".to_string(),
                description: "Architecture boundary validation".to_string(),
                enabled: true,
                config: None,
            },
            GateCheck {
                name: "policy".to_string(),
                description: "Policy compliance evaluation".to_string(),
                enabled: true,
                config: None,
            },
        ],
        thresholds: {
            let mut m = BTreeMap::new();
            m.insert("overall_score".to_string(), 80);
            m
        },
        global_config: None,
    }
}

// ── Entry point ─────────────────────────────────────────────────────

pub fn run(args: &GateConfigArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_global(global);
    let cwd = std::env::current_dir()?;

    if args.list {
        return run_list(&cwd, mode, global.verbose);
    }

    if let Some(ref check_name) = args.enable {
        return run_toggle(&cwd, check_name, true, mode);
    }

    if let Some(ref check_name) = args.disable {
        return run_toggle(&cwd, check_name, false, mode);
    }

    // No flags — show current config (same as --list).
    run_list(&cwd, mode, global.verbose)
}

// ── List ────────────────────────────────────────────────────────────

fn run_list(workspace: &Path, mode: OutputMode, verbose: bool) -> Result<()> {
    let config = load_config(workspace)?;

    match mode {
        OutputMode::Json => output::json::print(&config)?,
        OutputMode::Plain | OutputMode::Tui => {
            output::plain::blank();
            output::plain::section("Gate Configuration");

            let threshold = config.thresholds.get("overall_score").copied().unwrap_or(0);
            output::plain::label("Score threshold", format!("{threshold}%"));
            output::plain::blank();

            output::plain::section("Checks");
            for check in &config.checks {
                let icon = if check.enabled {
                    "\u{2713}"
                } else {
                    "\u{2717}"
                };
                output::plain::item(icon, &format!("{}: {}", check.name, check.description));
                if verbose
                    && let Some(ref cfg) = check.config
                    && !cfg.is_empty()
                {
                    let json = serde_json::to_string(cfg).unwrap_or_default();
                    output::plain::dim(&format!("  Config: {json}"));
                }
            }
        }
    }
    Ok(())
}

// ── Enable / Disable ────────────────────────────────────────────────

fn run_toggle(workspace: &Path, check_name: &str, enable: bool, mode: OutputMode) -> Result<()> {
    let mut config = load_config(workspace)?;

    let check = config.checks.iter_mut().find(|c| c.name == check_name);

    let Some(check) = check else {
        let available: Vec<&str> = config.checks.iter().map(|c| c.name.as_str()).collect();
        bail!(
            "Unknown check: \"{check_name}\". Available: {}",
            available.join(", ")
        );
    };

    check.enabled = enable;
    save_config(workspace, &config)?;

    let action = if enable { "Enabled" } else { "Disabled" };

    match mode {
        OutputMode::Json => {
            output::json::print(&serde_json::json!({
                "action": action.to_lowercase(),
                "check": check_name,
            }))?;
        }
        OutputMode::Plain | OutputMode::Tui => {
            output::plain::success(&format!("{action} check: {check_name}"));
        }
    }
    Ok(())
}

// ── Config I/O ──────────────────────────────────────────────────────

fn resolve_config_path(workspace: &Path) -> PathBuf {
    workspace.join(ANVIL_DIR).join(CONFIG_FILENAME)
}

fn load_config(workspace: &Path) -> Result<GateConfig> {
    let path = resolve_config_path(workspace);

    if !path.exists() {
        return Ok(default_config());
    }

    let content = std::fs::read_to_string(&path)?;
    let config: GateConfig = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid gate config at {}: {e}", path.display()))?;
    Ok(config)
}

fn save_config(workspace: &Path, config: &GateConfig) -> Result<()> {
    let path = resolve_config_path(workspace);

    // Ensure parent dir exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(config)?;
    crate::util::atomic_write(&path, json.as_bytes())?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Default config ──────────────────────────────────────────

    #[test]
    fn default_config_has_seven_checks() {
        let config = default_config();
        assert_eq!(config.checks.len(), 7);
        assert_eq!(config.version, 1);
    }

    #[test]
    fn default_config_has_overall_threshold() {
        let config = default_config();
        assert_eq!(config.thresholds.get("overall_score"), Some(&80));
    }

    #[test]
    fn default_coverage_is_disabled() {
        let config = default_config();
        let coverage = config.checks.iter().find(|c| c.name == "coverage").unwrap();
        assert!(!coverage.enabled);
    }

    // ── Config round-trip ───────────────────────────────────────

    #[test]
    fn config_json_round_trip() {
        let config = default_config();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: GateConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.checks.len(), 7);
        assert_eq!(parsed.version, 1);
    }

    // ── Save / Load ─────────────────────────────────────────────

    #[test]
    fn save_and_load_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = default_config();
        save_config(dir.path(), &config).unwrap();

        let loaded = load_config(dir.path()).unwrap();
        assert_eq!(loaded.checks.len(), 7);
        assert_eq!(loaded.thresholds.get("overall_score"), Some(&80));
    }

    #[test]
    fn load_returns_default_when_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.checks.len(), 7);
    }

    #[test]
    fn config_path_uses_anvil_dir() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join(ANVIL_DIR).join(CONFIG_FILENAME);
        assert_eq!(resolve_config_path(dir.path()), expected);
    }

    // ── Toggle ──────────────────────────────────────────────────

    #[test]
    fn enable_check_persists() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = default_config();
        config.checks.iter_mut().for_each(|c| c.enabled = false);
        save_config(dir.path(), &config).unwrap();

        // Toggle lint on.
        let mut loaded = load_config(dir.path()).unwrap();
        let lint = loaded.checks.iter_mut().find(|c| c.name == "lint").unwrap();
        lint.enabled = true;
        save_config(dir.path(), &loaded).unwrap();

        let reloaded = load_config(dir.path()).unwrap();
        let lint = reloaded.checks.iter().find(|c| c.name == "lint").unwrap();
        assert!(lint.enabled);
    }

    #[test]
    fn disable_check_persists() {
        let dir = tempfile::tempdir().unwrap();
        save_config(dir.path(), &default_config()).unwrap();

        let mut loaded = load_config(dir.path()).unwrap();
        let lint = loaded.checks.iter_mut().find(|c| c.name == "lint").unwrap();
        lint.enabled = false;
        save_config(dir.path(), &loaded).unwrap();

        let reloaded = load_config(dir.path()).unwrap();
        let lint = reloaded.checks.iter().find(|c| c.name == "lint").unwrap();
        assert!(!lint.enabled);
    }

    // ── Clap parsing ────────────────────────────────────────────

    #[test]
    fn clap_parses_gate_config_list() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "gate-config", "--list"]);
        assert!(result.is_ok());
    }

    #[test]
    fn clap_parses_gate_config_enable() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "gate-config", "--enable", "policy"]);
        assert!(result.is_ok());
    }

    #[test]
    fn clap_parses_gate_config_disable() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "gate-config", "--disable", "coverage"]);
        assert!(result.is_ok());
    }
}
