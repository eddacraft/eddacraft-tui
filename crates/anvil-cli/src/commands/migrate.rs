//! MLP2-040 — `anvil migrate` reads a legacy `.anvilrc` and writes the
//! equivalent `.anvil.<ext>` (yaml / yml / json / toml).
//!
//! Existing `.anvilrc` projects keep working through `gate.rs`'s fallback
//! path; this command is the one-time bridge for operators who want to
//! land on the multi-format surface from MLP-011 without hand-editing the
//! file.

use std::path::{Path, PathBuf};

use anvil_config::ConfigFormat;
use anyhow::{Context, Result, bail};
use clap::Args;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct MigrateArgs {
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

pub fn run(args: &MigrateArgs, _global: &GlobalArgs) -> Result<()> {
    let root = PathBuf::from(".");
    run_in(args, &root)
}

pub(crate) fn run_in(args: &MigrateArgs, root: &Path) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn args(format: &str, force: bool, remove_old: bool) -> MigrateArgs {
        MigrateArgs {
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
        let err = run_in(&args("yaml", false, false), tmp.path()).unwrap_err();
        assert!(err.to_string().contains("no legacy"));
    }

    #[test]
    fn migrates_json_anvilrc_to_yaml() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), r#"{"checks":["secret-detection"]}"#);
        run_in(&args("yaml", false, false), tmp.path()).unwrap();

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
        run_in(&args("toml", false, false), tmp.path()).unwrap();

        let new = std::fs::read_to_string(tmp.path().join(".anvil.toml")).unwrap();
        assert!(new.contains("checks"), "got:\n{new}");
        assert!(new.contains("\"a\""), "got:\n{new}");
        assert!(new.contains("\"b\""), "got:\n{new}");
    }

    #[test]
    fn migrates_toml_anvilrc_to_json() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), "checks = [\"secret-detection\"]\n");
        run_in(&args("json", false, false), tmp.path()).unwrap();

        let new = std::fs::read_to_string(tmp.path().join(".anvil.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&new).unwrap();
        let checks = parsed.get("checks").unwrap().as_array().unwrap();
        assert_eq!(checks[0].as_str().unwrap(), "secret-detection");
    }

    #[test]
    fn removes_legacy_anvilrc_when_flag_set() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), r#"{"checks":[]}"#);
        run_in(&args("yaml", false, true), tmp.path()).unwrap();

        assert!(!tmp.path().join(".anvilrc").exists());
        assert!(tmp.path().join(".anvil.yaml").exists());
    }

    #[test]
    fn refuses_to_overwrite_without_force() {
        let tmp = TempDir::new().unwrap();
        write_anvilrc(tmp.path(), r#"{"checks":[]}"#);
        std::fs::write(tmp.path().join(".anvil.yaml"), "pre-existing\n").unwrap();

        let err = run_in(&args("yaml", false, false), tmp.path()).unwrap_err();
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

        run_in(&args("yaml", true, false), tmp.path()).unwrap();
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
        let err = run_in(&args("ini", false, false), tmp.path()).unwrap_err();
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

        run_in(&args("yaml", false, false), tmp.path()).unwrap();

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
}
