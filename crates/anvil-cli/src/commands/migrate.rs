//! `anvil migrate` — config migration, split into two subcommands:
//!
//! - `format` (MLP2-040): migrate a legacy `.anvilrc` to the multi-format
//!   `.anvil.<ext>` surface from MLP-011 (yaml / yml / json / toml). This
//!   is a *filename/encoding* migration; existing `.anvilrc` projects keep
//!   working through `gate.rs`'s fallback path.
//! - `schema` (DISTRIB-005): reconcile the *contents* of an existing
//!   `.anvil.<ext>` config across anvil minor versions, applying any
//!   registered schema migration for the version delta (dry-run by
//!   default, `--apply` to write).
//!
//! Bare `anvil migrate` routes to `format` with a deprecation notice so
//! the pre-subcommand surface keeps working.

use std::path::Path;

use anvil_config::ConfigFormat;
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct MigrateArgs {
    #[command(subcommand)]
    pub command: Option<MigrateCommand>,
}

#[derive(Debug, Subcommand)]
pub enum MigrateCommand {
    /// Migrate a legacy `.anvilrc` to the multi-format `.anvil.<ext>`
    /// surface (yaml / yml / json / toml).
    Format(FormatArgs),
    /// Reconcile an existing config's schema across anvil versions,
    /// applying any registered migration for the version delta.
    Schema(SchemaArgs),
    /// Fold a legacy `.anvil/gate-config.json` into the project
    /// config's `gate` section and top-level `checks` list, then
    /// remove the legacy file.
    GateConfig(GateConfigMigrateArgs),
    /// Point the project config's `architecture` section at an
    /// existing standalone `.anvil/architecture.yaml` via an explicit
    /// `source` line.
    Architecture(ArchitectureMigrateArgs),
}

#[derive(Debug, Args)]
pub struct FormatArgs {
    /// Target format for the new config file. Defaults to `yaml`.
    #[arg(long, default_value = "yaml")]
    pub format: String,

    /// Overwrite an existing `.anvil.<ext>` file.
    #[arg(long)]
    pub force: bool,

    /// Remove the legacy `.anvilrc` after writing the new file.
    #[arg(long)]
    pub remove_old: bool,
}

impl Default for FormatArgs {
    fn default() -> Self {
        Self {
            format: "yaml".to_string(),
            force: false,
            remove_old: false,
        }
    }
}

#[derive(Debug, Args)]
pub struct GateConfigMigrateArgs {
    /// Write the fold. The default is a dry-run preview that changes
    /// nothing on disk.
    #[arg(long)]
    pub apply: bool,

    /// Accept a fold that weakens enforcement (deselects checks that
    /// are currently effective). Required with --apply when the legacy
    /// file disables checks the project currently runs.
    #[arg(long)]
    pub accept_weakening: bool,
}

#[derive(Debug, Args)]
pub struct ArchitectureMigrateArgs {
    /// Write the source line. The default is a dry-run preview that
    /// changes nothing on disk.
    #[arg(long)]
    pub apply: bool,
}

#[derive(Debug, Args)]
pub struct SchemaArgs {
    /// Write the migrated config. The default is a dry-run preview that
    /// changes nothing on disk.
    #[arg(long)]
    pub apply: bool,
}

pub fn run(args: &MigrateArgs, _global: &GlobalArgs) -> Result<()> {
    run_in(args, Path::new("."))
}

/// Dispatch `anvil migrate` against `root`. A bare invocation (no
/// subcommand) routes to `format` with a deprecation notice. Split out
/// from [`run`] so tests can drive the full dispatch — including the
/// deprecation path — against a tempdir instead of the process CWD.
pub(crate) fn run_in(args: &MigrateArgs, root: &Path) -> Result<()> {
    match &args.command {
        Some(MigrateCommand::Format(format_args)) => run_format_in(format_args, root),
        Some(MigrateCommand::Schema(schema_args)) => {
            run_schema_in(schema_args, root, &anvil_config::production_migrations())
        }
        Some(MigrateCommand::GateConfig(gate_args)) => run_gate_config_in(gate_args, root),
        Some(MigrateCommand::Architecture(arch_args)) => run_architecture_in(arch_args, root),
        None => {
            eprintln!(
                "anvil: `anvil migrate` now has subcommands; running `format` for \
                 back-compat. Prefer `anvil migrate format` (legacy `.anvilrc` \
                 conversion) or `anvil migrate schema` (cross-version config \
                 reconciliation)."
            );
            run_format_in(&FormatArgs::default(), root)
        }
    }
}

// ---------------------------------------------------------------------------
// `anvil migrate format` — MLP2-040 filename/encoding migration.
// ---------------------------------------------------------------------------

pub(crate) fn run_format_in(args: &FormatArgs, root: &Path) -> Result<()> {
    // DISTRIB-006 (ADR-060): rewriting the project config (`.anvil.<ext>` /
    // `.anvilrc`) is a durable per-project mutation. Refuse under a gated
    // ANVIL_HOME without `--touch-project-state`.
    crate::install_root::ensure_project_write_allowed("migrate format")?;

    let format = parse_target_format(&args.format)?;

    let old = root.join(".anvilrc");
    if !old.exists() {
        bail!(
            "no legacy `.anvilrc` to migrate at {} (nothing to do)",
            old.display()
        );
    }

    let new = root.join(format!(".anvil.{}", format.extension()));
    if new.exists() && !args.force {
        bail!(
            "refusing to overwrite existing {}; pass --force",
            new.display()
        );
    }

    let contents =
        std::fs::read_to_string(&old).with_context(|| format!("reading {}", old.display()))?;
    let value = detect_and_parse(&contents, &old)?;

    let serialised = serialise_to_format(&value, format)
        .with_context(|| format!("serialising config as {}", format.extension()))?;
    crate::util::atomic_write(&new, serialised.as_bytes())
        .with_context(|| format!("writing {}", new.display()))?;

    if args.remove_old {
        std::fs::remove_file(&old).with_context(|| format!("removing {}", old.display()))?;
        println!(
            "anvil: migrated {} → {} (legacy file removed)",
            old.display(),
            new.display()
        );
    } else {
        println!(
            "anvil: migrated {} → {} (legacy file kept; pass --remove-old to delete)",
            old.display(),
            new.display()
        );
    }
    Ok(())
}

fn parse_target_format(raw: &str) -> Result<ConfigFormat> {
    match raw.to_ascii_lowercase().as_str() {
        "yaml" => Ok(ConfigFormat::Yaml),
        "yml" => Ok(ConfigFormat::Yml),
        "json" => Ok(ConfigFormat::Json),
        "toml" => Ok(ConfigFormat::Toml),
        other => bail!("unsupported format `{other}`; expected yaml, yml, json, or toml"),
    }
}

/// Detect the legacy `.anvilrc` format by trying JSON, then TOML, then
/// YAML. The first parser that produces an object wins. Mirrors the
/// pre-MLP2-040 reader in `commands/gate.rs` so a project that worked
/// under the old reader migrates cleanly.
fn detect_and_parse(contents: &str, path: &Path) -> Result<serde_json::Value> {
    for format in [ConfigFormat::Json, ConfigFormat::Toml, ConfigFormat::Yaml] {
        if let Ok(value) = anvil_config::parse_str(contents, format, path)
            && value.is_object()
        {
            return Ok(value);
        }
    }
    Err(anyhow::anyhow!(
        "failed to parse {} as JSON, YAML, or TOML",
        path.display()
    ))
}

fn serialise_to_format(value: &serde_json::Value, format: ConfigFormat) -> Result<String> {
    match format {
        ConfigFormat::Yaml | ConfigFormat::Yml => {
            serde_yaml::to_string(value).context("yaml serialisation failed")
        }
        ConfigFormat::Json => {
            let mut s = serde_json::to_string_pretty(value).context("json serialisation failed")?;
            s.push('\n');
            Ok(s)
        }
        ConfigFormat::Toml => toml::to_string_pretty(value).context("toml serialisation failed"),
    }
}

// ---------------------------------------------------------------------------
// `anvil migrate schema` — DISTRIB-005 cross-version config reconciliation.
// ---------------------------------------------------------------------------

/// Core of `anvil migrate schema`, parameterised on the working root and
/// the migration registry so tests can inject a tempdir + fixture
/// migrations (the production registry carries the ADR-120 casing rename,
/// introduced in `0.10.0-beta`).
pub(crate) fn run_schema_in(
    args: &SchemaArgs,
    root: &Path,
    migrations: &[anvil_config::SchemaMigration],
) -> Result<()> {
    let installed = env!("CARGO_PKG_VERSION");

    // Origin version: the anvil version that created this project's
    // identity, read from `anvil/project-id` (MLP2-003). This — not
    // baseline metadata — is where the creating version is recorded.
    let origin = crate::activation::identity::read_project_id(root)
        .context("reading anvil/project-id")?
        .and_then(|id| id.created_by_version);
    let Some(origin) = origin else {
        println!(
            "anvil: cannot determine the anvil version that created this project \
             (no `created_by_version` in anvil/project-id). Run `anvil start` to \
             establish project identity, or review your config against the current \
             schema manually."
        );
        return Ok(());
    };

    // Which registered migrations apply for origin -> installed?
    let steps =
        anvil_config::plan_for_versions(&origin, installed, migrations).with_context(|| {
            format!("comparing project version `{origin}` against installed version `{installed}`")
        })?;

    if steps.is_empty() {
        println!(
            "anvil: config schema is current — no migration needed for {origin} → {installed}."
        );
        return Ok(());
    }

    // A migration applies, so a config must exist to receive it.
    let Some(discovered) =
        anvil_config::discover(root, ".anvil").context("discovering .anvil.<ext> config")?
    else {
        bail!(
            "{} schema migration(s) are registered for {origin} → {installed}, but no \
             .anvil.<ext> config was found to migrate. Run `anvil init` or \
             `anvil migrate format` first.",
            steps.len()
        );
    };

    let mut value = anvil_config::parse_file(&discovered.path)
        .with_context(|| format!("parsing {}", discovered.path.display()))?;

    println!(
        "anvil: {} schema migration(s) apply to {} ({origin} → {installed}):",
        steps.len(),
        discovered.path.display()
    );
    for step in &steps {
        println!("  • {}", step.description);
    }

    if !args.apply {
        println!(
            "anvil: dry-run — no changes written. Re-run `anvil migrate schema --apply` to \
             write them."
        );
        return Ok(());
    }

    // DISTRIB-006 (ADR-060): `--apply` rewrites the discovered config in place —
    // durable per-project state. Refuse under a gated ANVIL_HOME (the dry-run
    // above is read-only and already returned).
    crate::install_root::ensure_project_write_allowed("migrate schema")?;

    anvil_config::apply_steps(&steps, &mut value);
    let serialised = serialise_to_format(&value, discovered.format).with_context(|| {
        format!(
            "serialising migrated config as {}",
            discovered.format.extension()
        )
    })?;
    crate::util::atomic_write(&discovered.path, serialised.as_bytes())
        .with_context(|| format!("writing {}", discovered.path.display()))?;

    println!(
        "anvil: migrated {} ({origin} → {installed}, {} step(s) applied).",
        discovered.path.display(),
        steps.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// `anvil migrate gate-config` — UCFG-005 fold of the retired JSON.
// ---------------------------------------------------------------------------

/// Fold `.anvil/gate-config.json` into the unified config: enabled
/// checks become the explicit top-level `checks` list (only when the
/// main config has no list of its own), and composition (version,
/// thresholds, `global_config`, per-check config) folds into the `gate`
/// section — **only fields absent from the main config**, every folded
/// key reported. A fold that would deselect currently-effective checks
/// weakens enforcement and requires `--accept-weakening` (ADR-120
/// pt 4 diff-and-confirm). On `--apply` the legacy file is removed.
#[allow(clippy::too_many_lines)] // Linear plan-then-apply; each phase is a distinct fold concern.
pub(crate) fn run_gate_config_in(args: &GateConfigMigrateArgs, root: &Path) -> Result<()> {
    use crate::commands::gate_config as gc;

    let Some(legacy) = gc::load_legacy_gate_config(root)? else {
        bail!(
            "no legacy {} to fold at {} (nothing to do)",
            gc::LEGACY_GATE_CONFIG_REL,
            root.display()
        );
    };

    let mut project = crate::commands::config::load_project_config(root)?;
    let section = anvil_config::GateSection::from_config_value(&project.value)
        .map_err(|e| anyhow::anyhow!("invalid config: {e}"))?;
    let (effective_now, has_explicit) = gc::effective_selection(&project.value, section.as_ref());
    let has_top_level_list = project
        .value
        .get("checks")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|arr| !arr.is_empty());

    let mut folded: Vec<String> = Vec::new();
    let mut weakened: Vec<String> = Vec::new();

    // Selection fold. Obligation (a) from the gate-section item: when
    // the fold adds gate.checks keys, an explicit top-level list must
    // exist so key presence can never resurrect a deselected check.
    let legacy_enabled: Vec<String> = legacy
        .checks
        .iter()
        .filter(|c| c.enabled)
        .map(|c| c.name.clone())
        .collect();
    let (fold_selection, selection_to_write) = if has_top_level_list {
        // Explicit list present — selection untouched, list keeps winning.
        (false, Vec::new())
    } else if has_explicit {
        // Section-driven selection: materialise the CURRENT effective
        // selection (main config wins) so folded section keys stay
        // composition-only.
        folded.push(format!(
            "checks (materialised from gate.checks keys: {})",
            effective_now.join(", ")
        ));
        (true, effective_now.clone())
    } else {
        // No selection anywhere: the legacy enabled list becomes it.
        // An empty enabled list cannot be expressed as a top-level
        // `checks: []` (empty reads as absent and falls back to the
        // defaults), so refuse rather than write a lie.
        if legacy_enabled.is_empty() {
            bail!(
                "the legacy gate-config.json disables every check — an empty \
                 `checks` list cannot express that (it reads as the default \
                 selection). Re-enable at least one check with `anvil \
                 gate-config --enable <check>` after folding composition, or \
                 remove anvil's gate from your workflow instead."
            );
        }
        weakened = effective_now
            .iter()
            .filter(|name| !legacy_enabled.contains(name))
            .cloned()
            .collect();
        folded.push(format!(
            "checks (selection list: {})",
            legacy_enabled.join(", ")
        ));
        (true, legacy_enabled.clone())
    };

    // Composition fold into the gate section, absent fields only.
    let existing = section.unwrap_or_default();
    let mut gate_patch: Vec<(String, serde_json::Value)> = Vec::new();
    if existing.version.is_none() {
        gate_patch.push(("version".into(), serde_json::json!(legacy.version)));
        folded.push("gate.version".into());
    }
    let mut thresholds_patch = serde_json::Map::new();
    for (name, score) in &legacy.thresholds {
        if !existing.thresholds.contains_key(name) {
            thresholds_patch.insert(name.clone(), serde_json::json!(score));
            folded.push(format!("gate.thresholds.{name}"));
        }
    }
    if let Some(global) = &legacy.global_config
        && existing.global_config.is_none()
        && !global.is_empty()
    {
        gate_patch.push((
            "global_config".into(),
            serde_json::to_value(global).expect("BTreeMap serialises"),
        ));
        folded.push("gate.global_config".into());
    }
    let mut checks_patch = serde_json::Map::new();
    for check in &legacy.checks {
        if let Some(config) = &check.config
            && !config.is_empty()
            && !existing.checks.contains_key(&check.name)
        {
            checks_patch.insert(
                check.name.clone(),
                serde_json::to_value(config).expect("BTreeMap serialises"),
            );
            folded.push(format!("gate.checks.{}", check.name));
        }
    }

    if folded.is_empty() {
        println!(
            "Nothing to fold: every gate-config.json field is already present in {}.",
            project.label
        );
        if args.apply {
            std::fs::remove_file(root.join(gc::LEGACY_GATE_CONFIG_REL))?;
            println!("Removed {}.", gc::LEGACY_GATE_CONFIG_REL);
        }
        return Ok(());
    }

    println!(
        "Fold plan ({} -> {}):",
        gc::LEGACY_GATE_CONFIG_REL,
        project.label
    );
    for key in &folded {
        println!("  + {key}");
    }
    // NOTE: thresholds join the weakening set the day a gate run
    // consumes gate.thresholds (reserved today — ADR-120 pt 4 names
    // "lowered thresholds" explicitly).
    if !weakened.is_empty() {
        println!(
            "  ! WEAKENS enforcement — currently-effective checks would be \
             deselected: {}",
            weakened.join(", ")
        );
    }

    if !args.apply {
        println!("Dry-run only; pass --apply to write.");
        return Ok(());
    }

    // DISTRIB-006 (ADR-060): durable per-project mutation.
    crate::install_root::ensure_project_write_allowed("migrate gate-config")?;

    if !weakened.is_empty() && !args.accept_weakening {
        bail!(
            "fold would weaken enforcement (deselecting: {}) — re-run with \
             --accept-weakening to confirm, or edit the selection afterwards \
             with `anvil gate-config --enable <check>`",
            weakened.join(", ")
        );
    }

    // Apply the patch to the parsed value. A config that parses to a
    // non-table top level must not be silently replaced (data loss).
    let Some(root_obj) = project.value.as_object_mut() else {
        bail!(
            "{} does not parse to a table at the top level — fix the config \
             before migrating",
            project.label
        );
    };
    if fold_selection {
        root_obj.insert(
            "checks".into(),
            serde_json::Value::Array(
                selection_to_write
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    let gate = root_obj
        .entry("gate")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !gate.is_object() {
        *gate = serde_json::Value::Object(serde_json::Map::new());
    }
    let gate_obj = gate.as_object_mut().expect("normalised above");
    for (key, value) in gate_patch {
        gate_obj.insert(key, value);
    }
    if !thresholds_patch.is_empty() {
        let thresholds = gate_obj
            .entry("thresholds")
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let Some(table) = thresholds.as_object_mut() {
            for (key, value) in thresholds_patch {
                table.entry(key).or_insert(value);
            }
        }
    }
    if !checks_patch.is_empty() {
        let checks = gate_obj
            .entry("checks")
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let Some(table) = checks.as_object_mut() {
            for (key, value) in checks_patch {
                table.entry(key).or_insert(value);
            }
        }
    }

    let text = crate::commands::config::serialize_config(&project.value, project.writable_format)?;
    crate::util::atomic_write(&project.writable_path, text.as_bytes())?;
    std::fs::remove_file(root.join(gc::LEGACY_GATE_CONFIG_REL))?;
    println!(
        "Folded into {} and removed {}.",
        project.writable_path.display(),
        gc::LEGACY_GATE_CONFIG_REL
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// `anvil migrate architecture` — explicit source line (UCFG-007).
// ---------------------------------------------------------------------------

/// Write `architecture: { source: ".anvil/architecture.yaml" }` into
/// the project config when a standalone legacy file exists and the
/// config has no `architecture` section yet. No other keys are added
/// or removed; the file is re-serialised, so comments and quoting
/// style are not preserved (the plan output says so). The standalone
/// file stays where it is (it is the delegation target); consumers
/// resolve it through the hardened delegation pipeline afterwards.
pub(crate) fn run_architecture_in(args: &ArchitectureMigrateArgs, root: &Path) -> Result<()> {
    if !anvil_architecture::yaml_parser::architecture_yaml_exists(root) {
        bail!(
            "no standalone .anvil/architecture.yaml at {} (nothing to do)",
            root.display()
        );
    }

    let mut project = crate::commands::config::load_project_config(root)?;
    if project
        .value
        .get("architecture")
        .is_some_and(|v| !v.is_null())
    {
        bail!(
            "{} already has an `architecture` section — nothing to migrate. \
             If the section delegates to the standalone file via \
             `architecture.source`, keep the file; otherwise remove it once \
             the section is authoritative",
            project.label
        );
    }

    let rel = ".anvil/architecture.yaml";
    println!(
        "Plan: add `architecture.source = \"{rel}\"` to {}.",
        project.label
    );
    println!("Note: the config is re-serialised — comments are not preserved.");
    if !args.apply {
        println!("Dry-run only; pass --apply to write.");
        return Ok(());
    }

    // DISTRIB-006 (ADR-060): durable per-project mutation.
    crate::install_root::ensure_project_write_allowed("migrate architecture")?;

    // A config file that parses to a non-table top level must not be
    // silently replaced — that would discard whatever the file held.
    let Some(root_obj) = project.value.as_object_mut() else {
        bail!(
            "{} does not parse to a table at the top level — fix the config \
             before migrating",
            project.label
        );
    };
    root_obj.insert("architecture".into(), serde_json::json!({ "source": rel }));

    let text = crate::commands::config::serialize_config(&project.value, project.writable_format)?;
    crate::util::atomic_write(&project.writable_path, text.as_bytes())?;
    println!(
        "Wrote architecture source line to {}.",
        project.writable_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -----------------------------------------------------------------
    // `format` subcommand (MLP2-040) — behaviour preserved under the
    // subcommand split; tests now drive `run_format_in` + `FormatArgs`.
    // -----------------------------------------------------------------

    fn format_args(format: &str, force: bool, remove_old: bool) -> FormatArgs {
        FormatArgs {
            format: format.to_string(),
            force,
            remove_old,
        }
    }

    fn write_anvilrc(dir: &Path, contents: &str) {
        std::fs::write(dir.join(".anvilrc"), contents).unwrap();
    }

    #[test]
    fn errors_when_no_anvilrc_present() {
        let tmp = TempDir::new().unwrap();
        let err = run_format_in(&format_args("yaml", false, false), tmp.path()).unwrap_err();
        assert!(err.to_string().contains("no legacy"));
    }

    #[test]
    fn migrates_json_anvilrc_to_yaml() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), r#"{"checks":["secret-detection"]}"#);
        run_format_in(&format_args("yaml", false, false), tmp.path()).unwrap();

        let new = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        // serde_yaml prefixes scalar lists onto the same line; substring
        // assertions stay format-tolerant.
        assert!(new.contains("checks:"), "got:\n{new}");
        assert!(new.contains("secret-detection"), "got:\n{new}");

        // Legacy file kept by default.
        assert!(tmp.path().join(".anvilrc").exists());
    }

    #[test]
    fn migrates_yaml_anvilrc_to_toml() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), "checks: [\"a\", \"b\"]\n");
        run_format_in(&format_args("toml", false, false), tmp.path()).unwrap();

        let new = std::fs::read_to_string(tmp.path().join(".anvil.toml")).unwrap();
        assert!(new.contains("checks"), "got:\n{new}");
        assert!(new.contains("\"a\""), "got:\n{new}");
        assert!(new.contains("\"b\""), "got:\n{new}");
    }

    #[test]
    fn migrates_toml_anvilrc_to_json() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), "checks = [\"secret-detection\"]\n");
        run_format_in(&format_args("json", false, false), tmp.path()).unwrap();

        let new = std::fs::read_to_string(tmp.path().join(".anvil.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&new).unwrap();
        let checks = parsed.get("checks").unwrap().as_array().unwrap();
        assert_eq!(checks[0].as_str().unwrap(), "secret-detection");
    }

    #[test]
    fn removes_legacy_anvilrc_when_flag_set() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), r#"{"checks":[]}"#);
        run_format_in(&format_args("yaml", false, true), tmp.path()).unwrap();

        assert!(!tmp.path().join(".anvilrc").exists());
        assert!(tmp.path().join(".anvil.yaml").exists());
    }

    #[test]
    fn refuses_to_overwrite_without_force() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), r#"{"checks":[]}"#);
        std::fs::write(tmp.path().join(".anvil.yaml"), "pre-existing\n").unwrap();

        let err = run_format_in(&format_args("yaml", false, false), tmp.path()).unwrap_err();
        assert!(err.to_string().contains("refusing to overwrite"));
        // Pre-existing file must be intact.
        let unchanged = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        assert_eq!(unchanged, "pre-existing\n");
    }

    #[test]
    fn force_overwrites_existing_target() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), r#"{"checks":["x"]}"#);
        std::fs::write(tmp.path().join(".anvil.yaml"), "garbage\n").unwrap();

        run_format_in(&format_args("yaml", true, false), tmp.path()).unwrap();
        let new = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        assert!(new.contains("checks:"));
        // serde_yaml may emit `- x` (unquoted) or `- "x"`; the contract
        // is that the value survives, not the quoting style.
        assert!(new.contains("- x") || new.contains("\"x\""), "got:\n{new}");
        // Pre-existing garbage must NOT remain.
        assert!(!new.contains("garbage"), "got:\n{new}");
    }

    #[test]
    fn rejects_unknown_format() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), r#"{"checks":[]}"#);
        let err = run_format_in(&format_args("ini", false, false), tmp.path()).unwrap_err();
        assert!(err.to_string().contains("unsupported format"));
    }

    #[test]
    fn round_trips_through_anvil_config_discover() {
        // The whole point of the migration is that `anvil-config::discover`
        // + `parse_file` then reads the new file back as the same Value.
        let tmp = TempDir::new().unwrap();
        write_anvilrc(
            tmp.path(),
            r#"{"checks":["secret-detection","import-boundaries"]}"#,
        );

        run_format_in(&format_args("yaml", false, false), tmp.path()).unwrap();

        let discovered = anvil_config::discover(tmp.path(), ".anvil")
            .unwrap()
            .expect("discover must find .anvil.yaml after migrate");
        let value = anvil_config::parse_file(&discovered.path).unwrap();
        let checks = value
            .get("checks")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        let ids: Vec<&str> = checks
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        assert!(ids.contains(&"secret-detection"));
        assert!(ids.contains(&"import-boundaries"));
    }

    #[test]
    fn bare_migrate_routes_to_format() {
        // `anvil migrate` with no subcommand dispatches through `run_in`'s
        // `None` arm to `format` (and prints the deprecation notice).
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), r#"{"checks":["secret-detection"]}"#);
        run_in(&MigrateArgs { command: None }, tmp.path()).unwrap();
        assert!(tmp.path().join(".anvil.yaml").exists());
    }

    #[test]
    fn schema_subcommand_dispatches_through_run_in() {
        // The `Schema` arm of `run_in` reaches `run_schema_in` with the
        // (empty) production registry — exercised here against a project
        // with identity + config so it resolves to the no-op path.
        let tmp = TempDir::new().unwrap();
        write_project_id(tmp.path(), "0.1.0");
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "checks: [secret-detection]\n",
        )
        .unwrap();
        run_in(
            &MigrateArgs {
                command: Some(MigrateCommand::Schema(SchemaArgs { apply: true })),
            },
            tmp.path(),
        )
        .unwrap();
        // Empty production registry → config untouched.
        let after = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        assert!(!after.contains("migrated_marker"), "got:\n{after}");
    }

    // -----------------------------------------------------------------
    // `schema` subcommand (DISTRIB-005).
    // -----------------------------------------------------------------

    use anvil_config::SchemaMigration;

    fn schema_args(apply: bool) -> SchemaArgs {
        SchemaArgs { apply }
    }

    /// Write `anvil/project-id` recording `created_by_version`.
    fn write_project_id(root: &Path, created_by_version: &str) {
        use crate::activation::identity::{ProjectIdentity, project_id_path};
        std::fs::create_dir_all(root.join("anvil")).unwrap();
        let id = ProjectIdentity::new_fresh(created_by_version);
        std::fs::write(project_id_path(root), id.render()).unwrap();
    }

    /// One fixture migration that adds a `migrated_marker` key; introduced
    /// well below any realistic `CARGO_PKG_VERSION` so it always applies
    /// to a project created by `0.1.0`.
    fn marker_migration() -> Vec<SchemaMigration> {
        vec![
            SchemaMigration::new("0.2.0: add `migrated_marker`", "0.2.0", |v| {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("migrated_marker".to_string(), serde_json::json!(true));
                }
            })
            .unwrap(),
        ]
    }

    #[test]
    fn schema_no_project_id_is_graceful() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "checks: [secret-detection]\n",
        )
        .unwrap();
        // No anvil/project-id written.
        run_schema_in(&schema_args(true), tmp.path(), &marker_migration()).unwrap();
        // Config untouched (we never reached a write).
        let after = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        assert!(!after.contains("migrated_marker"), "got:\n{after}");
    }

    #[test]
    fn schema_no_registered_migration_is_noop() {
        let tmp = TempDir::new().unwrap();
        write_project_id(tmp.path(), "0.1.0");
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "checks: [secret-detection]\n",
        )
        .unwrap();
        // Production registry: its only entry (introduced 0.10.0-beta) is
        // not selected for an installed 0.9.4-beta → "no migration needed".
        run_schema_in(
            &schema_args(true),
            tmp.path(),
            &anvil_config::production_migrations(),
        )
        .unwrap();
        let after = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        assert!(!after.contains("migrated_marker"), "got:\n{after}");
    }

    #[test]
    fn schema_dry_run_previews_without_writing() {
        let tmp = TempDir::new().unwrap();
        write_project_id(tmp.path(), "0.1.0");
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "checks: [secret-detection]\n",
        )
        .unwrap();

        run_schema_in(&schema_args(false), tmp.path(), &marker_migration()).unwrap();

        // Dry-run: the marker must NOT be on disk.
        let after = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        assert!(
            !after.contains("migrated_marker"),
            "dry-run wrote changes:\n{after}"
        );
    }

    #[test]
    fn schema_apply_writes_migrated_config() {
        let tmp = TempDir::new().unwrap();
        write_project_id(tmp.path(), "0.1.0");
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "checks: [secret-detection]\n",
        )
        .unwrap();

        run_schema_in(&schema_args(true), tmp.path(), &marker_migration()).unwrap();

        // Apply: the transform is on disk and the file still parses, with
        // the original key preserved.
        let discovered = anvil_config::discover(tmp.path(), ".anvil")
            .unwrap()
            .unwrap();
        let value = anvil_config::parse_file(&discovered.path).unwrap();
        assert_eq!(value.get("migrated_marker"), Some(&serde_json::json!(true)));
        assert!(value.get("checks").is_some());
    }

    #[test]
    fn schema_missing_config_with_applicable_migration_errors() {
        let tmp = TempDir::new().unwrap();
        write_project_id(tmp.path(), "0.1.0");
        // Migration applies, but there is no .anvil.<ext> to migrate.
        let err = run_schema_in(&schema_args(true), tmp.path(), &marker_migration()).unwrap_err();
        assert!(
            err.to_string().contains("no") && err.to_string().contains(".anvil.<ext>"),
            "got: {err}"
        );
    }

    #[test]
    fn schema_malformed_project_version_errors() {
        let tmp = TempDir::new().unwrap();
        write_project_id(tmp.path(), "garbage-not-semver");
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "checks: [secret-detection]\n",
        )
        .unwrap();
        let err = run_schema_in(&schema_args(false), tmp.path(), &marker_migration()).unwrap_err();
        assert!(
            err.to_string().contains("comparing project version"),
            "got: {err}"
        );
    }

    // ── `anvil migrate gate-config` fold (UCFG-005) ─────────────

    fn write_legacy(dir: &Path, body: &str) {
        std::fs::create_dir_all(dir.join(".anvil")).unwrap();
        std::fs::write(dir.join(".anvil/gate-config.json"), body).unwrap();
    }

    fn gate_args(apply: bool, accept: bool) -> GateConfigMigrateArgs {
        GateConfigMigrateArgs {
            apply,
            accept_weakening: accept,
        }
    }

    #[test]
    fn gate_fold_dry_run_writes_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        write_legacy(
            tmp.path(),
            r#"{"version":1,"checks":[{"name":"lint","description":"","enabled":true}],"thresholds":{"overall_score":90}}"#,
        );
        run_gate_config_in(&gate_args(false, false), tmp.path()).unwrap();
        assert!(tmp.path().join(".anvil/gate-config.json").exists());
        assert!(!tmp.path().join(".anvil.yaml").exists());
    }

    #[test]
    fn gate_fold_weakening_requires_confirmation() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Legacy disables secret-detection (in the default effective
        // selection) — folding the selection would weaken enforcement.
        write_legacy(
            tmp.path(),
            r#"{"version":1,"checks":[{"name":"lint","description":"","enabled":true},{"name":"secret-detection","description":"","enabled":false}],"thresholds":{}}"#,
        );
        let err = run_gate_config_in(&gate_args(true, false), tmp.path()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("weaken"), "{msg}");
        assert!(msg.contains("secret-detection"), "{msg}");
        assert!(msg.contains("--accept-weakening"), "{msg}");
        // Nothing changed on refusal.
        assert!(tmp.path().join(".anvil/gate-config.json").exists());

        run_gate_config_in(&gate_args(true, true), tmp.path()).unwrap();
        let value = anvil_config::parse_file(&tmp.path().join(".anvil.yaml")).unwrap();
        let checks: Vec<&str> = value["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(checks, vec!["lint"]);
        assert!(!tmp.path().join(".anvil/gate-config.json").exists());
    }

    #[test]
    fn gate_fold_respects_existing_selection_and_folds_absent_only() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "checks: [secret-detection]\ngate:\n  thresholds:\n    overall_score: 95\n",
        )
        .unwrap();
        write_legacy(
            tmp.path(),
            r#"{"version":1,"checks":[{"name":"lint","description":"","enabled":true,"config":{"max_warnings":0}}],"thresholds":{"overall_score":50,"minimum":10}}"#,
        );
        run_gate_config_in(&gate_args(true, false), tmp.path()).unwrap();
        let value = anvil_config::parse_file(&tmp.path().join(".anvil.yaml")).unwrap();
        // Explicit selection untouched (present, not absent).
        let checks: Vec<&str> = value["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(checks, vec!["secret-detection"]);
        // Existing threshold wins; absent one folds.
        assert_eq!(value["gate"]["thresholds"]["overall_score"], 95);
        assert_eq!(value["gate"]["thresholds"]["minimum"], 10);
        // Per-check config folds under gate.checks.
        assert_eq!(value["gate"]["checks"]["lint"]["max_warnings"], 0);
        assert!(!tmp.path().join(".anvil/gate-config.json").exists());
    }

    /// UCFG-005 verifier blocking finding 1: on a section-driven
    /// selection (gate.checks keys, no top-level list), the fold must
    /// materialise the current effective selection as an explicit
    /// top-level list before adding section keys — otherwise a
    /// legacy-DISABLED check's folded config would select it.
    #[test]
    fn gate_fold_materialises_section_driven_selection() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "gate:\n  checks:\n    lint: {}\n",
        )
        .unwrap();
        write_legacy(
            tmp.path(),
            r#"{"version":1,"checks":[{"name":"coverage","description":"","enabled":false,"config":{"minimum":50}}],"thresholds":{}}"#,
        );
        run_gate_config_in(&gate_args(true, false), tmp.path()).unwrap();
        let value = anvil_config::parse_file(&tmp.path().join(".anvil.yaml")).unwrap();
        // Explicit list materialised from the section keys.
        let checks: Vec<&str> = value["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(checks, vec!["lint"]);
        // Folded composition present but NOT selected.
        assert_eq!(value["gate"]["checks"]["coverage"]["minimum"], 50);
        assert!(
            !checks.contains(&"coverage"),
            "legacy-disabled check must not be resurrected by key presence"
        );
    }

    /// Copilot review: a legacy file with zero enabled checks cannot
    /// fold into `checks: []` — empty reads as absent and resurrects
    /// the default selection. Refused with guidance.
    #[test]
    fn gate_fold_all_disabled_legacy_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        write_legacy(
            tmp.path(),
            r#"{"version":1,"checks":[{"name":"lint","description":"","enabled":false}],"thresholds":{}}"#,
        );
        let err = run_gate_config_in(&gate_args(false, false), tmp.path()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("disables every check"), "{msg}");
        assert!(tmp.path().join(".anvil/gate-config.json").exists());
    }

    #[test]
    fn gate_fold_without_legacy_file_is_a_clear_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = run_gate_config_in(&gate_args(false, false), tmp.path()).unwrap_err();
        assert!(err.to_string().contains("nothing to do"), "{err}");
    }

    // ── `anvil migrate architecture` (UCFG-007) ─────────────────

    fn arch_migrate_args(apply: bool) -> ArchitectureMigrateArgs {
        ArchitectureMigrateArgs { apply }
    }

    #[test]
    fn architecture_migrate_adds_source_key_preserving_other_keys() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(
            tmp.path().join(".anvil/architecture.yaml"),
            "schema_version: \"0.1.0\"\nlayers:\n  core:\n    patterns: [\"src/**\"]\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "schema_version: \"1.0.0\"\nchecks: [lint]\n",
        )
        .unwrap();

        // Dry-run writes nothing.
        run_architecture_in(&arch_migrate_args(false), tmp.path()).unwrap();
        let value = anvil_config::parse_file(&tmp.path().join(".anvil.yaml")).unwrap();
        assert!(value.get("architecture").is_none());

        run_architecture_in(&arch_migrate_args(true), tmp.path()).unwrap();
        let value = anvil_config::parse_file(&tmp.path().join(".anvil.yaml")).unwrap();
        assert_eq!(value["architecture"]["source"], ".anvil/architecture.yaml");
        // Nothing else touched; standalone file kept (delegation target).
        assert_eq!(value["schema_version"], "1.0.0");
        assert_eq!(value["checks"][0], "lint");
        assert!(tmp.path().join(".anvil/architecture.yaml").exists());
    }

    #[test]
    fn architecture_migrate_refuses_when_section_exists_or_no_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = run_architecture_in(&arch_migrate_args(false), tmp.path()).unwrap_err();
        assert!(err.to_string().contains("nothing to do"), "{err}");

        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(
            tmp.path().join(".anvil/architecture.yaml"),
            "schema_version: \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "architecture:\n  layers: {}\n",
        )
        .unwrap();
        let err = run_architecture_in(&arch_migrate_args(true), tmp.path()).unwrap_err();
        assert!(
            err.to_string()
                .contains("already has an `architecture` section"),
            "{err}"
        );
    }
}
