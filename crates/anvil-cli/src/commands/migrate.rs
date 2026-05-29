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
    /// Reconcile an existing config's schema across Anvil versions,
    /// applying any registered migration for the version delta.
    Schema(SchemaArgs),
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
/// migrations (the production registry is empty today).
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
        // Empty (production) registry → "no migration needed".
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
}
