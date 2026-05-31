use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::GlobalArgs;
use crate::commands::check_catalog::{
    canonical_check_name, default_gate_config_checks, definition_by_canonical,
};
use crate::output::{self, OutputMode};

const ANVIL_DIR: &str = ".anvil";
const CONFIG_FILENAME: &str = "gate-config.json";

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

fn default_config() -> GateConfig {
    GateConfig {
        version: 1,
        checks: default_gate_config_checks()
            .into_iter()
            .map(|(name, description, enabled)| GateCheck {
                name: name.to_string(),
                description: description.to_string(),
                enabled,
                config: None,
            })
            .collect(),
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
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
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
    let canonical_name = canonical_check_name(check_name).unwrap_or(check_name);

    let check = config.checks.iter_mut().find(|c| c.name == canonical_name);

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
                "check": canonical_name,
            }))?;
        }
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            output::plain::success(&format!("{action} check: {canonical_name}"));
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
    let mut config: GateConfig = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid gate config at {}: {e}", path.display()))?;
    normalize_check_names(&mut config);
    Ok(config)
}

fn normalize_check_names(config: &mut GateConfig) {
    for check in &mut config.checks {
        if let Some(canonical) = canonical_check_name(&check.name) {
            check.name = canonical.to_string();
            if let Some(def) = definition_by_canonical(canonical) {
                check.description = def.description.to_string();
            }
        }
    }
}

fn save_config(workspace: &Path, config: &GateConfig) -> Result<()> {
    // DISTRIB-006 (ADR-060): the gate config is durable per-project state the
    // production binary reads. Refuse to persist it under a gated ANVIL_HOME
    // without `--touch-project-state`.
    crate::install_root::ensure_project_write_allowed("gate-config write")?;

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
    fn default_config_has_nine_checks() {
        // Eight gate checks pre-AIGUARD-003 plus `command-safety` wired
        // in by AIGUARD-003.
        let config = default_config();
        assert_eq!(config.checks.len(), 9);
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

    #[test]
    fn default_config_uses_canonical_secret_and_boundary_names() {
        let config = default_config();
        assert!(config.checks.iter().any(|c| c.name == "secret-detection"));
        assert!(config.checks.iter().any(|c| c.name == "import-boundaries"));
        assert!(config.checks.iter().any(|c| c.name == "antipattern-scan"));
    }

    #[test]
    fn normalize_check_names_upgrades_legacy_internal_names() {
        let mut config = GateConfig {
            version: 1,
            checks: vec![
                GateCheck {
                    name: "secret".to_string(),
                    description: "old description".to_string(),
                    enabled: true,
                    config: None,
                },
                GateCheck {
                    name: "architecture".to_string(),
                    description: "old description".to_string(),
                    enabled: true,
                    config: None,
                },
            ],
            thresholds: BTreeMap::new(),
            global_config: None,
        };
        normalize_check_names(&mut config);
        assert_eq!(config.checks[0].name, "secret-detection");
        assert_eq!(
            config.checks[0].description,
            "Detect leaked secrets and credentials"
        );
        assert_eq!(config.checks[1].name, "import-boundaries");
        assert_eq!(
            config.checks[1].description,
            "Enforce module import boundaries"
        );
    }

    #[test]
    fn toggle_accepts_legacy_internal_name() {
        let dir = tempfile::tempdir().unwrap();
        save_config(dir.path(), &default_config()).unwrap();

        // Disable via legacy internal name.
        run_toggle(dir.path(), "secret", false, OutputMode::Plain).unwrap();

        let reloaded = load_config(dir.path()).unwrap();
        let check = reloaded
            .checks
            .iter()
            .find(|c| c.name == "secret-detection")
            .unwrap();
        assert!(!check.enabled);
    }

    #[test]
    fn toggle_accepts_stable_check_id() {
        let dir = tempfile::tempdir().unwrap();
        save_config(dir.path(), &default_config()).unwrap();

        run_toggle(dir.path(), "ANV-CORE-001", false, OutputMode::Plain).unwrap();

        let reloaded = load_config(dir.path()).unwrap();
        let check = reloaded
            .checks
            .iter()
            .find(|c| c.name == "secret-detection")
            .unwrap();
        assert!(!check.enabled);
    }

    #[test]
    fn normalize_check_names_refreshes_canonical_descriptions() {
        let mut config = GateConfig {
            version: 1,
            checks: vec![GateCheck {
                name: "secret-detection".to_string(),
                description: String::new(),
                enabled: true,
                config: None,
            }],
            thresholds: BTreeMap::new(),
            global_config: None,
        };
        normalize_check_names(&mut config);
        assert_eq!(config.checks[0].name, "secret-detection");
        assert_eq!(
            config.checks[0].description,
            "Detect leaked secrets and credentials"
        );
    }

    // ── Config round-trip ───────────────────────────────────────

    #[test]
    fn config_json_round_trip() {
        let config = default_config();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: GateConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.checks.len(), 9);
        assert_eq!(parsed.version, 1);
    }

    // ── Save / Load ─────────────────────────────────────────────

    #[test]
    fn save_and_load_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = default_config();
        save_config(dir.path(), &config).unwrap();

        let loaded = load_config(dir.path()).unwrap();
        assert_eq!(loaded.checks.len(), 9);
        assert_eq!(loaded.thresholds.get("overall_score"), Some(&80));
    }

    #[test]
    fn load_returns_default_when_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.checks.len(), 9);
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

    // Regression guard for #1016: every check in the default gate-config must
    // map to a dispatchable gate check via the catalog, otherwise toggling a
    // check via `anvil gate-config --enable` would have no effect on the
    // actual `anvil gate` run.
    #[test]
    fn default_config_checks_match_gate_available() {
        use crate::commands::check_catalog::gate_internal_name;
        let config = default_config();
        for check in &config.checks {
            assert!(
                gate_internal_name(&check.name).is_some(),
                "gate_config default contains unregistered '{}'",
                check.name
            );
        }
    }
}
