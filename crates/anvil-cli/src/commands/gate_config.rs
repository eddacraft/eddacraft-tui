//! `anvil gate-config` — gate composition over the unified project
//! config (UCFG-005, ADR-120 pt 4).
//!
//! Since UCFG-004 the main config is the single truth: the top-level
//! `checks` list selects checks, and the `gate` section carries
//! composition (thresholds, per-check config). This command reads and
//! writes THAT file. The legacy `.anvil/gate-config.json` is never
//! read for state and never written; when one is present the command
//! points at `anvil migrate gate-config` (and `anvil doctor` warns).
//!
//! `--disable` refuses checks whose rule class is hard-pinned per
//! ADR-039 (local check→class map, drift-guarded by test against
//! `anvil_config::HARD_PINNED_CLASSES`) — the UCFG-004 verifier
//! obligation: a config-writing surface must not silently drop
//! `secret-detection` / `command-safety` from the selection. The
//! explicit `anvil migrate gate-config --accept-weakening` fold is
//! the one sanctioned diff-and-confirm exception (ADR-120 pt 4).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, bail};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::GlobalArgs;
use crate::commands::check_catalog::{
    canonical_check_name, closest_registered_id, default_gate_config_checks,
};
use crate::commands::config::{load_project_config, serialize_config};
use crate::output::{self, OutputMode};

pub(crate) const LEGACY_GATE_CONFIG_REL: &str = ".anvil/gate-config.json";

/// Legacy default for the `overall_score` threshold, shown when the
/// gate section carries none (display continuity with the retired
/// JSON's default).
const DEFAULT_OVERALL_SCORE: u64 = 80;

/// Gate checks whose selection removal is refused because their rule
/// class is hard-pinned (ADR-039). Maps check name → pinned class.
const HARD_PINNED_CHECKS: &[(&str, &str)] = &[
    ("secret-detection", "secrets"),
    ("command-safety", "command-safety"),
];

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

// ── Legacy JSON types (read by `anvil migrate gate-config` only) ────

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

// ── Presented view over the unified config ──────────────────────────

/// One row of the presented gate configuration: catalog identity plus
/// selection state and any per-check config from the gate section.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GateConfigRow {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct GateConfigView {
    source: String,
    checks: Vec<GateConfigRow>,
    thresholds: BTreeMap<String, u64>,
}

/// The effective selection per the UCFG-004 reconciliation rule:
/// explicit top-level `checks` list; else `gate.checks` keys; else
/// the catalog defaults. Returns (names, explicit) where `explicit`
/// says an explicit selection exists in the file.
pub(crate) fn effective_selection(
    value: &serde_json::Value,
    section: Option<&anvil_config::GateSection>,
) -> (Vec<String>, bool) {
    let top_level: Vec<String> = value
        .get("checks")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|name| canonical_check_name(name).unwrap_or(name).to_string())
                .collect()
        })
        .unwrap_or_default();
    if !top_level.is_empty() {
        return (top_level, true);
    }
    if let Some(section) = section
        && !section.checks.is_empty()
    {
        return (section.check_names(), true);
    }
    (
        default_gate_config_checks()
            .into_iter()
            .filter(|(_, _, enabled)| *enabled)
            .map(|(name, _, _)| name.to_string())
            .collect(),
        false,
    )
}

fn build_view(workspace: &Path) -> Result<GateConfigView> {
    let project = load_project_config(workspace)?;
    let section = anvil_config::GateSection::from_config_value(&project.value)
        .map_err(|e| anyhow::anyhow!("invalid config: {e}"))?;
    let (selected, _) = effective_selection(&project.value, section.as_ref());

    let mut thresholds = section
        .as_ref()
        .map(|s| s.thresholds.clone())
        .unwrap_or_default();
    thresholds
        .entry("overall_score".to_string())
        .or_insert(DEFAULT_OVERALL_SCORE);

    let checks = default_gate_config_checks()
        .into_iter()
        .map(|(name, description, _)| {
            let config = section
                .as_ref()
                .and_then(|s| s.checks.get(name))
                .filter(|table| !table.is_empty())
                .cloned();
            GateConfigRow {
                name: name.to_string(),
                description: description.to_string(),
                enabled: selected.iter().any(|s| s == name),
                config,
            }
        })
        .collect();

    Ok(GateConfigView {
        source: project.label,
        checks,
        thresholds,
    })
}

// ── Entry point ─────────────────────────────────────────────────────

pub fn run(args: &GateConfigArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_global(global);
    let cwd = std::env::current_dir()?;

    if let Some(ref check_name) = args.enable {
        return run_toggle(&cwd, check_name, true, mode);
    }

    if let Some(ref check_name) = args.disable {
        return run_toggle(&cwd, check_name, false, mode);
    }

    // `--list` and no flags both show the current config.
    run_list(&cwd, mode, global.verbose)
}

// ── List ────────────────────────────────────────────────────────────

fn run_list(workspace: &Path, mode: OutputMode, verbose: bool) -> Result<()> {
    let view = build_view(workspace)?;

    match mode {
        OutputMode::Json => output::json::print(&view)?,
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            output::plain::blank();
            output::plain::section("Gate Configuration");
            output::plain::label("Source", &view.source);

            let threshold = view
                .thresholds
                .get("overall_score")
                .copied()
                .unwrap_or(DEFAULT_OVERALL_SCORE);
            output::plain::label("Score threshold", format!("{threshold}%"));
            output::plain::blank();

            output::plain::section("Checks");
            for check in &view.checks {
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

            if workspace.join(LEGACY_GATE_CONFIG_REL).exists() {
                output::plain::blank();
                output::plain::dim(&format!(
                    "Legacy {LEGACY_GATE_CONFIG_REL} present — its contents are ignored; \
                     run `anvil migrate gate-config` to fold it into the project config."
                ));
            }
        }
    }
    Ok(())
}

// ── Enable / Disable ────────────────────────────────────────────────

fn run_toggle(workspace: &Path, check_name: &str, enable: bool, mode: OutputMode) -> Result<()> {
    // DISTRIB-006 (ADR-060): this rewrites the project config the
    // production binary reads. Refuse under a gated ANVIL_HOME.
    crate::install_root::ensure_project_write_allowed("gate-config write")?;

    let canonical_name = validated_toggle_target(check_name)?;

    if !enable
        && let Some((_, class)) = HARD_PINNED_CHECKS
            .iter()
            .find(|(name, _)| *name == canonical_name)
    {
        bail!(
            "check \"{canonical_name}\" cannot be disabled via gate-config — its \
             rule class `{class}` is hard-pinned (ADR-039). Per-finding \
             suppression via `@anvil-ignore` remains available."
        );
    }

    let mut project = load_project_config(workspace)?;
    let section = anvil_config::GateSection::from_config_value(&project.value)
        .map_err(|e| anyhow::anyhow!("invalid config: {e}"))?;
    let (mut selected, _) = effective_selection(&project.value, section.as_ref());

    if enable {
        if !selected.iter().any(|s| s == &canonical_name) {
            selected.push(canonical_name.clone());
        }
    } else {
        selected.retain(|s| s != &canonical_name);
        // An empty `checks` list reads as absent (UCFG-004: absent or
        // empty falls back to gate.checks keys or catalog defaults),
        // so writing one would silently re-enable checks instead of
        // disabling the last one. Refuse with the honest picture.
        if selected.is_empty() {
            bail!(
                "disabling \"{canonical_name}\" would leave no selected checks — an \
                 empty `checks` list falls back to the default selection, which \
                 would NOT disable it. Keep at least one check selected, or remove \
                 anvil's gate from your workflow instead."
            );
        }
    }

    // Materialise the selection as the explicit top-level list — the
    // one truth the gate reads (and the UCFG-004 fold obligation: a
    // written selection is always an explicit list, so section-key
    // presence can never resurrect a disabled check). A config that
    // parses to a non-table top level must not be silently replaced.
    let Some(root_obj) = project.value.as_object_mut() else {
        bail!(
            "{} does not parse to a table at the top level — fix the config \
             before toggling checks",
            project.label
        );
    };
    root_obj.insert(
        "checks".to_string(),
        serde_json::Value::Array(
            selected
                .iter()
                .map(|s| serde_json::Value::String(s.clone()))
                .collect(),
        ),
    );

    let text = serialize_config(&project.value, project.writable_format)?;
    crate::util::atomic_write(&project.writable_path, text.as_bytes())?;

    let action = if enable { "Enabled" } else { "Disabled" };
    match mode {
        OutputMode::Json => {
            output::json::print(&serde_json::json!({
                "action": action.to_lowercase(),
                "check": canonical_name,
                "config": project.writable_path.display().to_string(),
            }))?;
        }
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            output::plain::success(&format!(
                "{action} check: {canonical_name} (in {})",
                project.writable_path.display()
            ));
        }
    }
    Ok(())
}

/// Resolve and validate a toggle target against the check catalog,
/// preserving the OPSUP-002 error UX: non-configurable checks are not
/// typos; unknown identifiers get a did-you-mean.
fn validated_toggle_target(check_name: &str) -> Result<String> {
    let configurable: Vec<&str> = default_gate_config_checks()
        .into_iter()
        .map(|(name, _, _)| name)
        .collect();
    if let Some(canonical) = canonical_check_name(check_name) {
        if configurable.contains(&canonical) {
            return Ok(canonical.to_string());
        }
        bail!(
            "Check \"{canonical}\" is not configurable via gate-config \
             (its activation is flag-driven). Configurable: {}",
            configurable.join(", ")
        );
    }
    let suggestion = closest_registered_id(check_name)
        .map(|s| format!(" (did you mean \"{s}\"?)"))
        .unwrap_or_default();
    bail!(
        "Unknown check: \"{check_name}\"{suggestion}. Available: {}",
        configurable.join(", ")
    );
}

// ── Legacy JSON access (fold input for `anvil migrate gate-config`) ─

pub(crate) fn load_legacy_gate_config(workspace: &Path) -> Result<Option<GateConfig>> {
    let path = workspace.join(LEGACY_GATE_CONFIG_REL);
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(anyhow::anyhow!("reading {}: {e}", path.display())),
    };
    let mut config: GateConfig = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid gate config at {}: {e}", path.display()))?;
    for check in &mut config.checks {
        if let Some(canonical) = canonical_check_name(&check.name) {
            check.name = canonical.to_string();
        }
    }
    Ok(Some(config))
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn write_yaml(dir: &Path, body: &str) {
        std::fs::write(dir.join(".anvil.yaml"), body).unwrap();
    }

    fn read_yaml(dir: &Path) -> serde_json::Value {
        anvil_config::parse_file(&dir.join(".anvil.yaml")).unwrap()
    }

    // ── View derivation ─────────────────────────────────────────

    #[test]
    fn view_defaults_when_no_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let view = build_view(dir.path()).unwrap();
        assert_eq!(view.checks.len(), 9);
        assert_eq!(view.thresholds.get("overall_score"), Some(&80));
        let coverage = view.checks.iter().find(|c| c.name == "coverage").unwrap();
        assert!(!coverage.enabled, "coverage defaults to disabled");
    }

    #[test]
    fn view_reflects_top_level_checks_selection() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "checks:\n  - secret-detection\n");
        let view = build_view(dir.path()).unwrap();
        let secret = view
            .checks
            .iter()
            .find(|c| c.name == "secret-detection")
            .unwrap();
        let lint = view.checks.iter().find(|c| c.name == "lint").unwrap();
        assert!(secret.enabled);
        assert!(!lint.enabled, "unlisted check reads as disabled");
    }

    #[test]
    fn view_surfaces_gate_section_thresholds_and_config() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "checks: [secret-detection]\ngate:\n  thresholds:\n    overall_score: 95\n  checks:\n    secret-detection:\n      max_findings: 0\n",
        );
        let view = build_view(dir.path()).unwrap();
        assert_eq!(view.thresholds.get("overall_score"), Some(&95));
        let secret = view
            .checks
            .iter()
            .find(|c| c.name == "secret-detection")
            .unwrap();
        assert_eq!(
            secret.config.as_ref().unwrap().get("max_findings"),
            Some(&serde_json::json!(0))
        );
    }

    // ── Toggle over the unified config ──────────────────────────

    #[test]
    fn disable_writes_explicit_top_level_list() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "checks:\n  - lint\n  - coverage\n");
        run_toggle(dir.path(), "coverage", false, OutputMode::Plain).unwrap();
        let value = read_yaml(dir.path());
        let checks: Vec<&str> = value["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(checks, vec!["lint"]);
    }

    #[test]
    fn enable_appends_to_selection_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "checks:\n  - lint\n");
        run_toggle(dir.path(), "coverage", true, OutputMode::Plain).unwrap();
        let value = read_yaml(dir.path());
        let checks: Vec<&str> = value["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(checks, vec!["lint", "coverage"]);
    }

    #[test]
    fn toggle_accepts_legacy_internal_name() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "checks:\n  - secret-detection\n  - lint\n");
        // "architecture" is the legacy internal name for import-boundaries.
        run_toggle(dir.path(), "architecture", true, OutputMode::Plain).unwrap();
        let value = read_yaml(dir.path());
        assert!(
            value["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_str() == Some("import-boundaries"))
        );
    }

    #[test]
    fn toggle_with_no_config_materialises_defaults_in_canonical_file() {
        let dir = tempfile::tempdir().unwrap();
        run_toggle(dir.path(), "coverage", true, OutputMode::Plain).unwrap();
        assert!(dir.path().join(".anvil.yaml").exists());
        let value = read_yaml(dir.path());
        let checks = value["checks"].as_array().unwrap();
        assert!(checks.iter().any(|v| v.as_str() == Some("coverage")));
        assert!(
            checks
                .iter()
                .any(|v| v.as_str() == Some("secret-detection")),
            "default selection is materialised, not dropped"
        );
    }

    #[test]
    fn toggle_preserves_unrelated_config_keys() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "schema_version: \"1.0.0\"\nchecks: [lint]\ngate:\n  thresholds:\n    overall_score: 90\n",
        );
        run_toggle(dir.path(), "coverage", true, OutputMode::Plain).unwrap();
        let value = read_yaml(dir.path());
        assert_eq!(value["schema_version"], "1.0.0");
        assert_eq!(value["gate"]["thresholds"]["overall_score"], 90);
    }

    // ── Hard-pin refusal (UCFG-004 obligation b) ────────────────

    #[test]
    fn disable_hard_pinned_check_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "checks:\n  - secret-detection\n");
        for check in ["secret-detection", "command-safety"] {
            let err = run_toggle(dir.path(), check, false, OutputMode::Plain).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("hard-pinned"), "{check}: {msg}");
            assert!(msg.contains("ADR-039"), "{check}: {msg}");
        }
        // The file is untouched by refused toggles.
        let value = read_yaml(dir.path());
        assert_eq!(value["checks"].as_array().unwrap().len(), 1);
    }

    /// Drift guard (verifier advisory): the local check→class map must
    /// stay within the ADR-039 source of truth in anvil-config.
    #[test]
    fn hard_pinned_check_classes_match_anvil_config() {
        for (_, class) in HARD_PINNED_CHECKS {
            assert!(
                anvil_config::HARD_PINNED_CLASSES.contains(class),
                "class {class} not in anvil_config::HARD_PINNED_CLASSES"
            );
        }
        assert_eq!(
            HARD_PINNED_CHECKS.len(),
            anvil_config::HARD_PINNED_CLASSES.len(),
            "a new hard-pinned class needs a gate-config mapping"
        );
    }

    /// UCFG-005 verifier blocking finding 2: a disable that would
    /// empty the selection is refused — an empty list reads as absent
    /// and would silently fall back to defaults / section keys.
    #[test]
    fn disable_emptying_selection_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "checks:\n  - lint\n");
        let err = run_toggle(dir.path(), "lint", false, OutputMode::Plain).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no selected checks"), "{msg}");
        // File untouched by the refusal.
        let value = read_yaml(dir.path());
        assert_eq!(value["checks"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn disable_emptying_section_driven_selection_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "gate:\n  checks:\n    lint: {}\n");
        let err = run_toggle(dir.path(), "lint", false, OutputMode::Plain).unwrap_err();
        assert!(err.to_string().contains("no selected checks"), "{err}");
    }

    #[test]
    fn enable_hard_pinned_check_is_allowed() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "checks:\n  - lint\n");
        run_toggle(dir.path(), "secret-detection", true, OutputMode::Plain).unwrap();
    }

    // ── Error UX preserved ──────────────────────────────────────

    #[test]
    fn toggle_unknown_check_suggests_closest_id() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_toggle(dir.path(), "lnt", false, OutputMode::Plain).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Unknown check"), "got: {msg}");
        assert!(msg.contains("did you mean \"lint\"?"), "got: {msg}");
    }

    #[test]
    fn toggle_known_but_non_configurable_check_is_not_a_typo() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_toggle(dir.path(), "sql-migrations", false, OutputMode::Plain).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not configurable via gate-config"), "{msg}");
        assert!(!msg.contains("did you mean"), "{msg}");
    }

    // ── Legacy JSON is never state ──────────────────────────────

    #[test]
    fn toggle_never_writes_legacy_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "checks: [lint]\n");
        run_toggle(dir.path(), "coverage", true, OutputMode::Plain).unwrap();
        assert!(
            !dir.path().join(LEGACY_GATE_CONFIG_REL).exists(),
            "the retired JSON must never be created"
        );
    }

    #[test]
    fn list_ignores_legacy_json_contents() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "checks: [lint]\n");
        std::fs::create_dir_all(dir.path().join(".anvil")).unwrap();
        std::fs::write(
            dir.path().join(LEGACY_GATE_CONFIG_REL),
            r#"{"version":1,"checks":[{"name":"coverage","description":"","enabled":true}],"thresholds":{}}"#,
        )
        .unwrap();
        let view = build_view(dir.path()).unwrap();
        let coverage = view.checks.iter().find(|c| c.name == "coverage").unwrap();
        assert!(
            !coverage.enabled,
            "legacy JSON must not influence the presented state"
        );
    }

    #[test]
    fn load_legacy_gate_config_canonicalises_names() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".anvil")).unwrap();
        std::fs::write(
            dir.path().join(LEGACY_GATE_CONFIG_REL),
            r#"{"version":1,"checks":[{"name":"secret","description":"","enabled":true}],"thresholds":{"overall_score":80}}"#,
        )
        .unwrap();
        let legacy = load_legacy_gate_config(dir.path()).unwrap().unwrap();
        assert_eq!(legacy.checks[0].name, "secret-detection");
        assert!(
            load_legacy_gate_config(&dir.path().join("nope"))
                .unwrap()
                .is_none()
        );
    }

    // ── Clap parsing ────────────────────────────────────────────

    #[test]
    fn clap_parses_gate_config_flags() {
        use clap::Parser;
        for argv in [
            ["anvil", "gate-config", "--list"].as_slice(),
            ["anvil", "gate-config", "--enable", "policy"].as_slice(),
            ["anvil", "gate-config", "--disable", "coverage"].as_slice(),
        ] {
            assert!(crate::Cli::try_parse_from(argv).is_ok(), "{argv:?}");
        }
    }
}
