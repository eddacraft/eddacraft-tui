use std::fs;
use std::io::{IsTerminal, Write};
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
pub(crate) struct AnvilConfig {
    pub(crate) schema_version: String,
    pub(crate) planning_dir: String,
    pub(crate) format: String,
    pub(crate) checks: Vec<String>,
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
    // Use the shared `config_exists_in` helper so `init` and the onboarding
    // flow agree on whether a config exists — they previously diverged on
    // zero-byte `.anvilrc` files, leaving onboarding calling `init` and
    // `init` immediately bailing on the empty file it could not see.
    if anvil_tui::surfaces::onboarding::config_exists_in(root) && !args.force {
        anyhow::bail!(".anvilrc already exists. Use --force to overwrite.");
    }

    // A zero-byte `.anvilrc` is treated as "missing" by `config_exists_in`,
    // but `write_new` (O_CREAT | O_EXCL) would still fail because the inode
    // exists. Remove the empty stub so the upcoming create proceeds cleanly
    // — the only information it could possibly hold is "nothing".
    if !args.force {
        let config_path = root.join(".anvilrc");
        if let Ok(meta) = fs::metadata(&config_path)
            && meta.is_file()
            && meta.len() == 0
        {
            fs::remove_file(&config_path)
                .with_context(|| format!("failed to remove empty {}", config_path.display()))?;
        }
    }

    if global.json {
        let config = AnvilConfig::default();
        generate_config_with_force(&config, root, args.force)?;
        let json = serde_json::to_string_pretty(&config)?;
        println!("{json}");
    } else if global.no_tui || !std::io::stdout().is_terminal() {
        run_plain(root, args.force)?;
    } else {
        run_tui(root, args.force)?;
    }

    Ok(())
}

fn run_tui(root: &Path, force: bool) -> anyhow::Result<()> {
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

    // Use the selected directory as the init root (not just the planning dir).
    let init_root = if state.config.directory == "." {
        root.to_path_buf()
    } else {
        root.join(&state.config.directory)
    };
    fs::create_dir_all(&init_root)
        .with_context(|| format!("failed to create directory {}", init_root.display()))?;

    let config = AnvilConfig {
        schema_version: SCHEMA_VERSION.to_string(),
        planning_dir: "plans".to_string(),
        format: format_label(state.config.format),
        checks: checks.clone(),
    };

    generate_config_with_force(&config, &init_root, force)?;
    print_success(&config.planning_dir, &checks);
    Ok(())
}

fn run_plain(root: &Path, force: bool) -> anyhow::Result<()> {
    let config = AnvilConfig::default();
    generate_config_with_force(&config, root, force)?;
    print_success(&config.planning_dir, &config.checks);
    Ok(())
}

pub(crate) fn generate_config(config: &AnvilConfig, root: &Path) -> anyhow::Result<bool> {
    generate_config_with_force(config, root, false)
}

pub(crate) fn generate_config_with_force(
    config: &AnvilConfig,
    root: &Path,
    force: bool,
) -> anyhow::Result<bool> {
    let content = match config.format.as_str() {
        "toml" => toml_serialise(config),
        "yaml" => yaml_serialise(config),
        _ => serde_json::to_string_pretty(config).context("failed to serialise config")?,
    };
    // Ensure the root exists before any file writes — `write_new` opens with
    // O_CREAT | O_EXCL and will fail with NotFound rather than a useful
    // "directory missing" error if a caller passes a freshly-picked path.
    fs::create_dir_all(root)
        .with_context(|| format!("failed to create directory {}", root.display()))?;
    let path = root.join(".anvilrc");
    if force {
        crate::util::atomic_write(&path, content.as_bytes()).context("failed to write .anvilrc")?;
    } else {
        crate::util::write_new(&path, content.as_bytes()).context("failed to write .anvilrc")?;
    }

    fs::create_dir_all(root.join(".anvil/cache")).context("failed to create .anvil/cache/")?;

    let gitignore_updated = append_gitignore_entry(root)?;

    let planning_dir = root.join(&config.planning_dir);
    if !planning_dir.exists() {
        fs::create_dir_all(&planning_dir)
            .with_context(|| format!("failed to create {}/", config.planning_dir))?;
    }

    Ok(gitignore_updated)
}

fn append_gitignore_entry(root: &Path) -> anyhow::Result<bool> {
    let gitignore = root.join(".gitignore");
    let entry = ".anvil/cache/";

    // Refuse to modify a symlinked .gitignore — a hostile symlink could
    // redirect the append into a file outside the project root. We use
    // `symlink_metadata` so we see the link itself rather than following it.
    match fs::symlink_metadata(&gitignore) {
        Ok(meta) if meta.file_type().is_symlink() => {
            anyhow::bail!(
                ".gitignore is a symbolic link; refusing to modify for safety. Resolve manually and re-run."
            );
        }
        Ok(_) | Err(_) => {}
    }

    // Read the file once so we can both scan for the existing entry and
    // decide whether a leading newline is needed. A missing file is not an
    // error; any other read failure is surfaced rather than silently
    // swallowed with `unwrap_or_default`.
    let existing: Option<String> = match fs::read_to_string(&gitignore) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).context("failed to read .gitignore"),
    };

    if let Some(contents) = existing.as_deref() {
        // Simple trimmed-line equality check. We do not strip trailing
        // inline comments (e.g. `".anvil/cache/ # keep"`), so a
        // hand-authored entry with a trailing comment will trigger a
        // duplicate append. Anvil never writes that form itself, and the
        // duplicate is harmless to git's ignore semantics.
        for line in contents.lines() {
            if line.trim() == entry {
                return Ok(false);
            }
        }
    }

    let needs_newline = existing
        .as_deref()
        .is_some_and(|c| !c.is_empty() && !c.ends_with('\n'));

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore)
        .context("failed to open .gitignore for appending")?;

    if needs_newline {
        writeln!(file).context("failed to write newline to .gitignore")?;
    }

    writeln!(file, "{entry}").context("failed to append to .gitignore")?;
    Ok(true)
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

/// Simple YAML serialisation (no external crate needed for this shape).
fn yaml_serialise(config: &AnvilConfig) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "schemaVersion: \"{}\"", config.schema_version);
    let _ = writeln!(out, "planningDir: \"{}\"", config.planning_dir);
    let _ = writeln!(out, "format: \"{}\"", config.format);
    out.push_str("checks:\n");
    for check in &config.checks {
        let _ = writeln!(out, "  - \"{check}\"");
    }
    out
}

/// Simple TOML serialisation.
fn toml_serialise(config: &AnvilConfig) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "schema_version = \"{}\"", config.schema_version);
    let _ = writeln!(out, "planning_dir = \"{}\"", config.planning_dir);
    let _ = writeln!(out, "format = \"{}\"", config.format);
    let checks: Vec<String> = config.checks.iter().map(|c| format!("\"{c}\"")).collect();
    let _ = writeln!(out, "checks = [{}]", checks.join(", "));
    out
}

pub(crate) fn format_label(fmt: anvil_tui::surfaces::init::ConfigFormat) -> String {
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

        let result = run_plain(dir.path(), false);

        assert!(result.is_ok());
        assert!(dir.path().join(".anvilrc").exists());
        assert!(dir.path().join(".anvil").is_dir());
        assert!(dir.path().join(".anvil/cache").is_dir());
        assert!(dir.path().join("plans").is_dir());

        let content = fs::read_to_string(dir.path().join(".anvilrc")).unwrap();
        // Default format is YAML, so check as text.
        assert!(content.contains("schemaVersion: \"1.0.0\""));
        assert!(content.contains("planningDir: \"plans\""));
        assert!(content.contains("format: \"yaml\""));
        assert!(content.contains("secret-detection"));
        assert!(content.contains("import-boundaries"));

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
        assert!(content.contains("schemaVersion"));
        assert!(content.contains("1.0.0"));
        assert!(!content.contains("old"));
    }

    #[test]
    fn gitignore_not_duplicated() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), ".anvil/cache/\n").unwrap();

        let updated = append_gitignore_entry(dir.path()).unwrap();
        assert!(
            !updated,
            "should report no update when entry already present"
        );

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        let count = content.matches(".anvil/cache/").count();
        assert_eq!(count, 1, "entry should not be duplicated");
    }

    #[test]
    fn gitignore_reports_update_when_appending() {
        let dir = tempfile::tempdir().unwrap();
        let updated = append_gitignore_entry(dir.path()).unwrap();
        assert!(updated, "should report update when gitignore is created");

        let updated_again = append_gitignore_entry(dir.path()).unwrap();
        assert!(
            !updated_again,
            "should report no update on second call with entry present",
        );
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
