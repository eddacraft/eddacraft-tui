use std::path::Path;

use anvil_config::{ConfigFormat, RuleModes, discover, parse_file, parse_str};
use anyhow::{Context, bail};
use clap::{Args, Subcommand};
use serde_json::{Map, Value};

use crate::GlobalArgs;

pub(crate) struct ProjectConfig {
    pub(crate) label: String,
    pub(crate) value: Value,
    pub(crate) writable_path: std::path::PathBuf,
    pub(crate) writable_format: ConfigFormat,
}

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show the effective anvil config.
    Show,
    /// Set a rule mode in the project config.
    Set {
        /// Rule to set: `public-api-expansion`, `new-dependency-introduction`,
        /// `cross-layer-violation`, or `privilege-expansion`.
        #[arg(value_name = "RULE")]
        rule: String,
        /// Mode to apply: `off`, `warn`, or `enforce`.
        #[arg(value_name = "MODE")]
        mode: String,
    },
    /// Convert the project config to another canonical format.
    ///
    /// Rewrites the discovered project config as `.anvil.<ext>`. This does
    /// not change rule modes.
    Convert {
        /// Destination format: yaml, yml, json, or toml. Never `.anvilrc`.
        #[arg(long)]
        to: String,
        /// Print the converted config instead of writing `.anvil.<ext>`.
        #[arg(long)]
        stdout: bool,
        /// Overwrite an existing destination file.
        #[arg(long)]
        force: bool,
        /// Delete the source file when the destination is a different path.
        #[arg(long)]
        remove_old: bool,
    },
}

pub fn run(args: &ConfigArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let root = Path::new(".");
    match &args.command {
        ConfigCommand::Show => {
            let view = collect_config_show(root)?;
            // Issue #3915: `--json` is advertised on this surface, so the
            // whole of stdout has to be the document — the prose form and
            // the JSON form are alternatives, never concatenated.
            if global.json {
                crate::output::json::print(&view.to_json())?;
            } else {
                print!("{}", view.render_human());
            }
        }
        ConfigCommand::Set { rule, mode } => {
            let written = set_rule_mode(root, rule, mode)?;
            // Issue #3938 (per the #3915 contract): an accepted `--json`
            // makes the whole of stdout the document, never the human line.
            if global.json {
                crate::output::json::print(&serde_json::json!({
                    "rule": rule,
                    "mode": mode,
                    "config": written.display().to_string(),
                }))?;
            } else {
                println!("set {rule}={mode}");
            }
        }
        ConfigCommand::Convert {
            to,
            stdout,
            force,
            remove_old,
        } => {
            if *stdout {
                let converted = convert_config(root, to)?;
                if global.json {
                    // `--stdout` prints the config in the *target* format,
                    // so raw text would break the one-JSON-document contract
                    // for every non-JSON target (issue #3938). The converted
                    // text travels inside the document instead.
                    crate::output::json::print(&serde_json::json!({
                        "format": parse_output_format(to)?.extension(),
                        "converted": converted,
                    }))?;
                } else {
                    print!("{converted}");
                }
            } else {
                let outcome = convert_and_write(root, to, *force, *remove_old, "config convert")?;
                if global.json {
                    crate::output::json::print(&outcome.to_json())?;
                } else {
                    println!("{}", outcome.render_human());
                }
            }
        }
    }
    Ok(())
}

/// What `config show` reports, collected once and rendered as either prose
/// or JSON — the two forms cannot drift because they read the same fields.
struct ConfigShow {
    label: String,
    modes: RuleModes,
    note: Option<String>,
}

impl ConfigShow {
    fn render_human(&self) -> String {
        let note = self
            .note
            .as_ref()
            .map(|note| format!("note: {note}\n"))
            .unwrap_or_default();
        format!(
            "config: {label}\nrule modes: {}\n{note}",
            self.modes.clone().summary(),
            label = self.label
        )
    }

    /// The `--json` document. `note` is always present (null when nothing is
    /// deprecated) so consumers get one stable shape rather than a key that
    /// appears only on legacy configs.
    fn to_json(&self) -> Value {
        serde_json::json!({
            "config": self.label,
            "rule_modes": {
                "public-api-expansion": self.modes.public_api_expansion.as_str(),
                "new-dependency-introduction": self.modes.new_dependency_introduction.as_str(),
                "cross-layer-violation": self.modes.cross_layer_violation.as_str(),
                "privilege-expansion": self.modes.privilege_expansion.as_str(),
            },
            "note": self.note,
        })
    }
}

fn collect_config_show(root: &Path) -> anyhow::Result<ConfigShow> {
    let config = load_project_config(root)?;
    let modes = RuleModes::from_value(&config.value)?;
    // UCFG-002: surface legacy camelCase keys so operators actually see
    // the deprecation (the anvil-config helper had no render path).
    let mut probe = config.value.clone();
    let renamed = anvil_config::normalize_legacy_keys(&mut probe);
    Ok(ConfigShow {
        label: config.label,
        modes,
        note: anvil_config::legacy_keys_deprecation_note(&renamed),
    })
}

fn set_rule_mode(root: &Path, rule: &str, mode: &str) -> anyhow::Result<std::path::PathBuf> {
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

    // DISTRIB-006 (ADR-060): writing a rule mode rewrites the project config
    // (`.anvilrc` / `.anvil.<ext>`) that the production binary reads. Refuse under
    // a gated ANVIL_HOME without `--touch-project-state`.
    crate::install_root::ensure_project_write_allowed("config set")?;

    let mut config = load_project_config(root)?;
    ensure_rule_mode(&mut config.value, rule, mode);
    // ADR-120 pt 3 "rewritten on the next owned write": an owned write
    // of a legacy-cased file emits canonical snake_case keys.
    let renamed = anvil_config::normalize_legacy_keys(&mut config.value);
    if let Some(note) = anvil_config::legacy_keys_deprecation_note(&renamed) {
        eprintln!("anvil: {note} (rewritten in this write)");
    }
    RuleModes::from_value(&config.value).with_context(|| format!("invalid rule mode `{mode}`"))?;

    let text = serialize_config(&config.value, config.writable_format)?;
    std::fs::write(&config.writable_path, text)
        .with_context(|| format!("writing {}", config.writable_path.display()))?;
    Ok(config.writable_path)
}

fn convert_config(root: &Path, format: &str) -> anyhow::Result<String> {
    let config = load_project_config(root)?;
    let format = parse_output_format(format)?;
    let mut value = config.value;
    align_format_metadata(&mut value, format);
    serialize_config(&value, format)
}

/// Write the discovered project config as `.anvil.<ext>` (UCFG-015).
///
/// Source is the discover winner, or leftover `.anvilrc`. Destination is
/// never `.anvilrc`. `--remove-old` deletes the source only when dest is
/// a different path.
pub(crate) fn convert_and_write(
    root: &Path,
    to: &str,
    force: bool,
    remove_old: bool,
    write_gate: &str,
) -> anyhow::Result<ConvertOutcome> {
    crate::install_root::ensure_project_write_allowed(write_gate)?;

    let dest_format = parse_output_format(to)?;
    let source = source_project_config(root)?;
    let dest = root.join(format!(".anvil.{}", dest_format.extension()));

    let same_path = source.writable_path == dest;
    if dest.exists() && !same_path && !force {
        bail!(
            "refusing to overwrite existing {}; pass --force",
            dest.display()
        );
    }

    let mut value = source.value;
    let renamed = anvil_config::normalize_legacy_keys(&mut value);
    if let Some(note) = anvil_config::legacy_keys_deprecation_note(&renamed) {
        eprintln!("anvil: {note} (rewritten during conversion)");
    }
    align_format_metadata(&mut value, dest_format);

    let text = serialize_config(&value, dest_format)?;
    crate::util::atomic_write(&dest, text.as_bytes())
        .with_context(|| format!("writing {}", dest.display()))?;

    if remove_old && !same_path {
        std::fs::remove_file(&source.writable_path)
            .with_context(|| format!("removing {}", source.writable_path.display()))?;
        return Ok(ConvertOutcome {
            source: source.writable_path,
            destination: dest,
            source_removed: true,
            rewrote_in_place: false,
        });
    }

    Ok(ConvertOutcome {
        source: source.writable_path,
        destination: dest,
        source_removed: false,
        rewrote_in_place: same_path,
    })
}

/// What `config convert` (write mode) did, collected once and rendered as
/// either prose or the `--json` document — the `ConfigShow` pattern, so the
/// two forms cannot drift.
#[derive(Debug)]
pub(crate) struct ConvertOutcome {
    source: std::path::PathBuf,
    destination: std::path::PathBuf,
    source_removed: bool,
    rewrote_in_place: bool,
}

impl ConvertOutcome {
    pub(crate) fn render_human(&self) -> String {
        if self.source_removed {
            format!(
                "anvil: converted {} → {} (source file removed)",
                self.source.display(),
                self.destination.display()
            )
        } else if self.rewrote_in_place {
            format!("anvil: rewrote {}", self.destination.display())
        } else {
            format!(
                "anvil: converted {} → {} (source kept; pass --remove-old to delete)",
                self.source.display(),
                self.destination.display()
            )
        }
    }

    /// `source_removed` covers the same-path rewrite too (nothing was
    /// removed), so consumers get one stable three-field shape.
    fn to_json(&self) -> Value {
        serde_json::json!({
            "source": self.source.display().to_string(),
            "destination": self.destination.display().to_string(),
            "source_removed": self.source_removed,
        })
    }
}

fn source_project_config(root: &Path) -> anyhow::Result<ProjectConfig> {
    let config = load_project_config(root)?;
    if config.label == "defaults" {
        bail!("no project config to convert (run `anvil init`)");
    }
    Ok(config)
}

/// Parse a discovered `.anvil.*` file. Syntax/schema parse failures become
/// [`crate::output::InvalidProjectConfig`] so dispatch can emit exit 4;
/// I/O and other runtime faults stay ordinary errors (exit 1).
pub(crate) fn parse_discovered_project_config(path: &Path) -> anyhow::Result<Value> {
    parse_file(path).map_err(map_project_parse_error)
}

fn map_project_parse_error(err: anvil_config::ParseError) -> anyhow::Error {
    use anvil_config::ParseError;
    match &err {
        ParseError::Yaml { .. }
        | ParseError::Json { .. }
        | ParseError::Toml { .. }
        | ParseError::NonFiniteTomlFloat { .. }
        | ParseError::AliasNotPermitted { .. }
        | ParseError::DepthExceeded { .. }
        | ParseError::UnrecognisedExtension { .. } => {
            crate::output::InvalidProjectConfig::new(err).into()
        }
        ParseError::Io { .. }
        | ParseError::NotARegularFile { .. }
        | ParseError::Symlink { .. }
        | ParseError::FileTooLarge { .. } => err.into(),
    }
}

pub(crate) fn load_project_config(root: &Path) -> anyhow::Result<ProjectConfig> {
    if let Some(discovered) = discover(root, ".anvil")? {
        let value = parse_discovered_project_config(&discovered.path)?;
        let label = discovered
            .path
            .strip_prefix(root)
            .unwrap_or(&discovered.path)
            .to_string_lossy()
            .into_owned();
        return Ok(ProjectConfig {
            label,
            value,
            writable_path: discovered.path,
            writable_format: discovered.format,
        });
    }

    let rc_path = root.join(".anvilrc");
    match std::fs::read_to_string(&rc_path) {
        Ok(contents) => {
            let (value, writable_format) = detect_anvilrc(&contents, &rc_path)?;
            Ok(ProjectConfig {
                label: String::from(".anvilrc"),
                value,
                writable_path: rc_path,
                writable_format,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ProjectConfig {
            label: String::from("defaults"),
            value: serde_json::json!({}),
            writable_path: root.join(".anvil.yaml"),
            writable_format: ConfigFormat::Yaml,
        }),
        Err(error) => Err(error).context("reading .anvilrc"),
    }
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

/// Detect embedded format of a legacy `.anvilrc` (JSON, then TOML, then YAML).
fn detect_anvilrc(contents: &str, path: &Path) -> anyhow::Result<(Value, ConfigFormat)> {
    for format in [ConfigFormat::Json, ConfigFormat::Toml, ConfigFormat::Yaml] {
        if let Ok(value) = parse_str(contents, format, path)
            && value.is_object()
        {
            return Ok((value, format));
        }
    }
    Err(crate::output::InvalidProjectConfig::new(format!(
        "failed to parse {} as JSON, YAML, or TOML",
        path.display()
    ))
    .into())
}

pub(crate) fn parse_output_format(raw: &str) -> anyhow::Result<ConfigFormat> {
    match raw.to_ascii_lowercase().as_str() {
        "anvilrc" | ".anvilrc" | "rc" => {
            bail!("refusing to write `.anvilrc`; expected yaml, yml, json, or toml")
        }
        "yaml" => Ok(ConfigFormat::Yaml),
        "yml" => Ok(ConfigFormat::Yml),
        "json" => Ok(ConfigFormat::Json),
        "toml" => Ok(ConfigFormat::Toml),
        other => bail!("unsupported config format `{other}`; expected yaml, yml, json, or toml"),
    }
}

/// Rewrite top-level `format` metadata to the destination extension.
///
/// Discovery treats the filename as authoritative. The embedded field is
/// init/start metadata and must not retain a source spelling after an
/// owned conversion (issue #3914). Absent keys stay absent so legacy
/// files without the field do not gain one.
fn align_format_metadata(value: &mut Value, dest_format: ConfigFormat) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    if obj.contains_key("format") {
        obj.insert(
            "format".to_string(),
            Value::String(dest_format.extension().to_string()),
        );
    }
}

pub(crate) fn serialize_config(value: &Value, format: ConfigFormat) -> anyhow::Result<String> {
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

        let output = collect_config_show(tmp.path()).unwrap().render_human();

        assert!(output.contains("config: defaults"));
        assert!(output.contains("public-api-expansion=warn"));
    }

    #[test]
    fn set_creates_yaml_config_and_preserves_rule_mode() {
        let tmp = TempDir::new().unwrap();

        set_rule_mode(tmp.path(), "public-api-expansion", "enforce").unwrap();

        let output = collect_config_show(tmp.path()).unwrap().render_human();
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
    fn set_updates_existing_anvilrc_in_place() {
        // legacy-fallback coverage (.anvilrc deliberately) — `config set`
        // must edit the legacy file in place, not seed a canonical twin.
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), "{}").unwrap();

        set_rule_mode(tmp.path(), "public-api-expansion", "enforce").unwrap();

        assert!(!tmp.path().join(".anvil.yaml").exists());
        let contents = std::fs::read_to_string(tmp.path().join(".anvilrc")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(
            parsed["enforcement"]["rules"]["public-api-expansion"]["mode"],
            "enforce"
        );
    }

    #[test]
    fn convert_prints_requested_format_without_writing_start_flags() {
        let tmp = TempDir::new().unwrap();
        set_rule_mode(tmp.path(), "new-dependency-introduction", "off").unwrap();

        let output = convert_config(tmp.path(), "toml").unwrap();

        assert!(output.contains("[enforcement.rules.new-dependency-introduction]"));
        assert!(output.contains("mode = \"off\""));
        assert!(
            !tmp.path().join(".anvil.toml").exists(),
            "--stdout path must not write"
        );
    }

    #[test]
    fn convert_writes_canonical_dest_and_never_anvilrc() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), "checks:\n  - lint\n").unwrap();

        let msg = convert_and_write(tmp.path(), "json", false, false, "config convert")
            .unwrap()
            .render_human();
        assert!(msg.contains(".anvil.json"), "{msg}");
        assert!(tmp.path().join(".anvil.json").exists());
        assert!(tmp.path().join(".anvil.yaml").exists());
        assert!(!tmp.path().join(".anvilrc").exists());
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join(".anvil.json")).unwrap())
                .unwrap();
        assert_eq!(parsed["checks"][0], "lint");
    }

    #[test]
    fn convert_refuses_anvilrc_dest() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), "checks: []\n").unwrap();
        let err =
            convert_and_write(tmp.path(), "anvilrc", false, false, "config convert").unwrap_err();
        assert!(err.to_string().contains(".anvilrc"), "{err}");
        assert!(!tmp.path().join(".anvilrc").exists());
    }

    #[test]
    fn convert_remove_old_deletes_source_when_dest_differs() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), "checks:\n  - lint\n").unwrap();
        convert_and_write(tmp.path(), "toml", false, true, "config convert").unwrap();
        assert!(tmp.path().join(".anvil.toml").exists());
        assert!(!tmp.path().join(".anvil.yaml").exists());
    }

    #[test]
    fn convert_remove_old_keeps_file_on_same_format_rewrite() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), "schemaVersion: \"1.0.0\"\n").unwrap();
        convert_and_write(tmp.path(), "yaml", false, true, "config convert").unwrap();
        assert!(tmp.path().join(".anvil.yaml").exists());
        let body = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        assert!(body.contains("schema_version"), "{body}");
        assert!(!body.contains("schemaVersion"), "{body}");
    }

    #[test]
    fn convert_write_errors_when_no_config() {
        let tmp = TempDir::new().unwrap();
        let err =
            convert_and_write(tmp.path(), "yaml", false, false, "config convert").unwrap_err();
        assert!(err.to_string().contains("no project config"), "{err}");
        assert!(!tmp.path().join(".anvil.yaml").exists());
    }

    // ── UCFG-002 pins ───────────────────────────────────────────

    /// ADR-120 pt 1: no command creates a `.anvilrc` — a rule-mode set
    /// with no config materialises the canonical file.
    #[test]
    fn set_with_no_config_creates_canonical_file() {
        let tmp = TempDir::new().unwrap();
        set_rule_mode(tmp.path(), "public-api-expansion", "off").unwrap();
        assert!(tmp.path().join(".anvil.yaml").exists());
        assert!(!tmp.path().join(".anvilrc").exists());
    }

    /// ADR-120 pt 3 "rewritten on the next owned write": setting a rule
    /// mode on a legacy `camelCase` file emits canonical `snake_case` keys
    /// (in place — the filename does not change; migration is explicit).
    #[test]
    fn set_rewrites_legacy_camel_keys_in_place() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"schemaVersion":"1.0.0","planningDir":"plans","checks":[]}"#,
        )
        .unwrap();
        set_rule_mode(tmp.path(), "public-api-expansion", "warn").unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".anvilrc")).unwrap();
        assert!(content.contains("schema_version"), "{content}");
        assert!(!content.contains("schemaVersion"), "{content}");
        assert!(!tmp.path().join(".anvil.yaml").exists(), "no silent rename");
    }

    /// UCFG-002 dual-config leg: with both names present, `config set`
    /// edits the discover winner and leaves the legacy file untouched.
    #[test]
    fn set_edits_the_discover_winner_in_dual_state() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), r#"{"checks":["lint"]}"#).unwrap();
        std::fs::write(tmp.path().join(".anvil.yaml"), "checks:\n  - lint\n").unwrap();
        let legacy_before = std::fs::read(tmp.path().join(".anvilrc")).unwrap();
        set_rule_mode(tmp.path(), "public-api-expansion", "enforce").unwrap();
        let canonical = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        assert!(canonical.contains("public-api-expansion"), "{canonical}");
        assert_eq!(
            std::fs::read(tmp.path().join(".anvilrc")).unwrap(),
            legacy_before,
            "legacy file must be byte-identical"
        );
    }

    /// The deprecation note renders on `config show` for legacy-cased
    /// files and stays silent for canonical ones.
    #[test]
    fn show_renders_the_legacy_key_note() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "schemaVersion: \"1.0.0\"\nchecks: []\n",
        )
        .unwrap();
        let shown = collect_config_show(tmp.path()).unwrap().render_human();
        assert!(shown.contains("deprecated camelCase"), "{shown}");
        assert!(shown.contains("migrate schema"), "{shown}");

        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "schema_version: \"1.0.0\"\nchecks: []\n",
        )
        .unwrap();
        let shown = collect_config_show(tmp.path()).unwrap().render_human();
        assert!(!shown.contains("deprecated"), "{shown}");
    }

    const CONVERT_FORMATS: &[&str] = &["yaml", "yml", "json", "toml"];

    fn write_format_fixture(root: &Path, format: ConfigFormat, format_meta: &str) {
        let value = serde_json::json!({
            "schema_version": "1.0.0",
            "planning_dir": "plans",
            "format": format_meta,
            "checks": ["lint"],
        });
        std::fs::write(
            root.join(format!(".anvil.{}", format.extension())),
            serialize_config(&value, format).unwrap(),
        )
        .unwrap();
    }

    fn parsed_format_field(root: &Path, format: ConfigFormat) -> Option<String> {
        let path = root.join(format!(".anvil.{}", format.extension()));
        anvil_config::parse_file(&path).unwrap()["format"]
            .as_str()
            .map(str::to_owned)
    }

    /// Issue #3914: pairwise conversion must not keep the source `format`
    /// spelling in the destination body.
    #[test]
    fn convert_rewrites_embedded_format_metadata_pairwise() {
        for src in CONVERT_FORMATS {
            for dest in CONVERT_FORMATS {
                let tmp = TempDir::new().unwrap();
                let src_fmt = parse_output_format(src).unwrap();
                let dest_fmt = parse_output_format(dest).unwrap();
                write_format_fixture(tmp.path(), src_fmt, src_fmt.extension());

                convert_and_write(tmp.path(), dest, false, true, "config convert").unwrap();

                assert_eq!(
                    parsed_format_field(tmp.path(), dest_fmt).as_deref(),
                    Some(dest_fmt.extension()),
                    "{src} → {dest} retained stale format metadata"
                );
            }
        }
    }

    #[test]
    fn convert_stdout_rewrites_embedded_format_metadata() {
        let tmp = TempDir::new().unwrap();
        write_format_fixture(tmp.path(), ConfigFormat::Yml, "yml");

        let output = convert_config(tmp.path(), "json").unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["format"], "json", "{output}");
        assert!(
            !tmp.path().join(".anvil.json").exists(),
            "--stdout path must not write"
        );
    }

    #[test]
    fn convert_rewrites_stale_same_path_format_metadata() {
        let tmp = TempDir::new().unwrap();
        write_format_fixture(tmp.path(), ConfigFormat::Yaml, "yml");

        convert_and_write(tmp.path(), "yaml", false, true, "config convert").unwrap();

        assert_eq!(
            parsed_format_field(tmp.path(), ConfigFormat::Yaml).as_deref(),
            Some("yaml")
        );
    }

    #[test]
    fn convert_does_not_invent_format_metadata_when_absent() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.yml"), "checks:\n  - lint\n").unwrap();

        convert_and_write(tmp.path(), "json", false, true, "config convert").unwrap();

        let parsed = anvil_config::parse_file(&tmp.path().join(".anvil.json")).unwrap();
        assert!(parsed.get("format").is_none(), "{parsed}");
        assert_eq!(parsed["checks"][0], "lint");
    }

    #[test]
    fn convert_legacy_anvilrc_rewrites_format_metadata() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "{\n  \"format\": \"json\",\n  \"checks\": []\n}\n",
        )
        .unwrap();

        convert_and_write(tmp.path(), "toml", false, false, "config convert").unwrap();

        let body = std::fs::read_to_string(tmp.path().join(".anvil.toml")).unwrap();
        assert!(body.contains("format = \"toml\""), "{body}");
        assert!(!body.contains("format = \"json\""), "{body}");
    }

    #[test]
    fn convert_round_trip_does_not_oscillate_format_metadata() {
        let tmp = TempDir::new().unwrap();
        write_format_fixture(tmp.path(), ConfigFormat::Yaml, "yaml");

        for dest in ["json", "toml", "yml", "yaml"] {
            convert_and_write(tmp.path(), dest, false, true, "config convert").unwrap();
        }

        let parsed = anvil_config::parse_file(&tmp.path().join(".anvil.yaml")).unwrap();
        assert_eq!(parsed["format"], "yaml", "{parsed}");
        let keys: std::collections::BTreeSet<_> =
            parsed.as_object().unwrap().keys().cloned().collect();
        let expected: std::collections::BTreeSet<_> =
            ["checks", "format", "planning_dir", "schema_version"]
                .into_iter()
                .map(str::to_string)
                .collect();
        assert_eq!(keys, expected, "{parsed}");
    }

    #[test]
    fn convert_init_shaped_file_matches_fresh_init_typed_view() {
        use crate::commands::init::{AnvilConfig, generate_config};
        use crate::config_view::InitConfigView;

        let src = TempDir::new().unwrap();
        generate_config(
            &AnvilConfig {
                format: "yaml".to_string(),
                ..AnvilConfig::default()
            },
            src.path(),
        )
        .unwrap();
        convert_and_write(src.path(), "json", false, true, "config convert").unwrap();
        let converted = anvil_config::parse_file(&src.path().join(".anvil.json")).unwrap();
        let converted_view = InitConfigView::from_value(&converted).unwrap();

        let fresh = TempDir::new().unwrap();
        generate_config(
            &AnvilConfig {
                format: "json".to_string(),
                ..AnvilConfig::default()
            },
            fresh.path(),
        )
        .unwrap();
        let fresh_value = anvil_config::parse_file(&fresh.path().join(".anvil.json")).unwrap();
        let fresh_view = InitConfigView::from_value(&fresh_value).unwrap();

        assert_eq!(converted_view, fresh_view);
    }
}
