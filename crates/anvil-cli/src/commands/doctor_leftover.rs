//! UCFG-016: TTY doctor offers to migrate or remove leftover config.
//!
//! Non-TTY, `--json`, CI, git hooks, and `ANVIL_NO_PROMPT` stay warn-only.
//! A single healthy canonical `.anvil.<ext>` is never prompted.

use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::commands::gate_config::LEGACY_GATE_CONFIG_REL;
use crate::commands::migrate::{ArchitectureMigrateArgs, GateConfigMigrateArgs};

/// One leftover problem doctor can offer to clean up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeftoverOffer {
    pub kind: LeftoverKind,
    pub summary: String,
    pub choices: Vec<LeftoverChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LeftoverKind {
    LoneAnvilrc,
    ShadowedConfigs,
    LegacyGateConfig,
    UnrecordedArchitecture,
    ShadowedArchitecture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LeftoverChoice {
    MigrateAnvilrc { remove_old: bool },
    RemoveShadowedConfigs { names: Vec<String> },
    FoldGateConfig { accept_weakening: bool },
    RecordArchitectureSource,
    RemoveArchitectureYaml,
    Skip,
}

impl LeftoverChoice {
    fn label(&self) -> String {
        match self {
            Self::MigrateAnvilrc { remove_old: false } => {
                "migrate to .anvil.yaml (keep .anvilrc)".to_string()
            }
            Self::MigrateAnvilrc { remove_old: true } => {
                "migrate to .anvil.yaml and remove .anvilrc".to_string()
            }
            Self::RemoveShadowedConfigs { names } => {
                format!("remove shadowed {}", names.join(", "))
            }
            Self::FoldGateConfig {
                accept_weakening: false,
            } => "fold .anvil/gate-config.json into the project config".to_string(),
            Self::FoldGateConfig {
                accept_weakening: true,
            } => "fold .anvil/gate-config.json and accept weaker enforcement".to_string(),
            Self::RecordArchitectureSource => {
                "record architecture.source for .anvil/architecture.yaml".to_string()
            }
            Self::RemoveArchitectureYaml => "remove shadowed .anvil/architecture.yaml".to_string(),
            Self::Skip => "skip".to_string(),
        }
    }
}

/// Interactive leftover cleanup is allowed only on a real TTY that is not
/// `--json` and not a CI / hook / explicit no-prompt environment.
pub(crate) fn should_offer_leftover_cleanup(
    json: bool,
    interactive_tty: bool,
    non_interactive_env: bool,
) -> bool {
    !json && interactive_tty && !non_interactive_env
}

pub(crate) fn leftover_offers_in(root: &Path) -> Vec<LeftoverOffer> {
    let mut offers = Vec::new();

    if let Ok(present) = present_config_names(root) {
        if present.len() == 1 && present[0] == ".anvilrc" {
            offers.push(LeftoverOffer {
                kind: LeftoverKind::LoneAnvilrc,
                summary: "legacy .anvilrc is the only project config".to_string(),
                choices: vec![
                    LeftoverChoice::MigrateAnvilrc { remove_old: false },
                    LeftoverChoice::MigrateAnvilrc { remove_old: true },
                ],
            });
        } else if present.len() > 1 {
            let winner = &present[0];
            let shadowed: Vec<String> = present[1..].to_vec();
            offers.push(LeftoverOffer {
                kind: LeftoverKind::ShadowedConfigs,
                summary: format!(
                    "multiple project config files found; {winner} wins (discover-first precedence)"
                ),
                choices: vec![LeftoverChoice::RemoveShadowedConfigs { names: shadowed }],
            });
        }
    }

    if root.join(LEGACY_GATE_CONFIG_REL).is_file() {
        offers.push(LeftoverOffer {
            kind: LeftoverKind::LegacyGateConfig,
            summary: "legacy .anvil/gate-config.json is present and ignored by gate runs"
                .to_string(),
            choices: vec![LeftoverChoice::FoldGateConfig {
                accept_weakening: false,
            }],
        });
    }

    match architecture_leftover(root) {
        ArchitectureLeftover::None => {}
        ArchitectureLeftover::Unrecorded => {
            offers.push(LeftoverOffer {
                kind: LeftoverKind::UnrecordedArchitecture,
                summary: "standalone .anvil/architecture.yaml is unrecorded in the project config"
                    .to_string(),
                choices: vec![LeftoverChoice::RecordArchitectureSource],
            });
        }
        ArchitectureLeftover::Shadowed => {
            offers.push(LeftoverOffer {
                kind: LeftoverKind::ShadowedArchitecture,
                summary:
                    "architecture section wins; standalone .anvil/architecture.yaml is shadowed"
                        .to_string(),
                choices: vec![LeftoverChoice::RemoveArchitectureYaml],
            });
        }
    }

    offers
}

pub(crate) fn parse_choice(input: &str, offer: &LeftoverOffer) -> LeftoverChoice {
    let trimmed = input.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("s")
        || trimmed.eq_ignore_ascii_case("skip")
    {
        return LeftoverChoice::Skip;
    }
    if let Ok(index) = trimmed.parse::<usize>()
        && index >= 1
        && let Some(choice) = offer.choices.get(index - 1)
    {
        return choice.clone();
    }
    LeftoverChoice::Skip
}

pub(crate) fn apply_leftover_choice(root: &Path, choice: &LeftoverChoice) -> Result<String> {
    match choice {
        LeftoverChoice::Skip => Ok(String::new()),
        LeftoverChoice::MigrateAnvilrc { remove_old } => {
            crate::commands::config::convert_and_write(
                root,
                "yaml",
                false,
                *remove_old,
                "doctor leftover",
            )
            .map(|outcome| outcome.render_human())
        }
        LeftoverChoice::RemoveShadowedConfigs { names } => remove_shadowed_configs(root, names),
        LeftoverChoice::FoldGateConfig { accept_weakening } => {
            // json_mode false: this interactive cleanup path is suppressed
            // under `--json` by `should_offer_leftover_cleanup`.
            crate::commands::migrate::run_gate_config_in(
                &GateConfigMigrateArgs {
                    apply: true,
                    accept_weakening: *accept_weakening,
                },
                root,
                false,
            )?;
            Ok("folded .anvil/gate-config.json into the project config".to_string())
        }
        LeftoverChoice::RecordArchitectureSource => {
            crate::commands::migrate::run_architecture_in(
                &ArchitectureMigrateArgs { apply: true },
                root,
                false,
            )?;
            Ok("recorded architecture.source in the project config".to_string())
        }
        LeftoverChoice::RemoveArchitectureYaml => {
            crate::install_root::ensure_project_write_allowed("doctor leftover")?;
            let path = anvil_architecture::yaml_parser::get_architecture_yaml_path(root);
            std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
            Ok("removed shadowed .anvil/architecture.yaml".to_string())
        }
    }
}

pub(crate) fn run_offers(
    root: &Path,
    offers: &[LeftoverOffer],
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<Vec<String>> {
    let mut applied = Vec::new();
    for offer in offers {
        writeln!(output, "anvil doctor: leftover config")?;
        writeln!(output, "{}", offer.summary)?;
        for (i, choice) in offer.choices.iter().enumerate() {
            writeln!(output, "  [{}] {}", i + 1, choice.label())?;
        }
        writeln!(output, "  [s] skip")?;
        write!(output, "Choice: ")?;
        output.flush()?;

        let mut line = String::new();
        let n = input.read_line(&mut line)?;
        let choice = if n == 0 {
            LeftoverChoice::Skip
        } else {
            parse_choice(&line, offer)
        };
        match apply_leftover_choice(root, &choice) {
            Ok(summary) if !summary.is_empty() => {
                writeln!(output, "{summary}")?;
                applied.push(summary);
            }
            Ok(_) => {
                writeln!(output, "skipped")?;
            }
            Err(error) => {
                writeln!(output, "anvil: leftover action failed: {error:#}")?;
            }
        }
    }
    Ok(applied)
}

pub(crate) fn run_offers_on_stdio(root: &Path, offers: &[LeftoverOffer]) -> Result<Vec<String>> {
    let _ = crossterm::terminal::disable_raw_mode();
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut output = std::io::stderr();
    run_offers(root, offers, &mut input, &mut output)
}

fn present_config_names(root: &Path) -> std::io::Result<Vec<String>> {
    let mut present = Vec::new();
    for format in &anvil_config::DISCOVER_PRECEDENCE {
        let name = format!(".anvil.{}", format.extension());
        match root.join(&name).try_exists() {
            Ok(true) => present.push(name),
            Ok(false) => {}
            Err(error) => return Err(error),
        }
    }
    match root.join(".anvilrc").try_exists() {
        Ok(true) => present.push(".anvilrc".to_string()),
        Ok(false) => {}
        Err(error) => return Err(error),
    }
    Ok(present)
}

fn remove_shadowed_configs(root: &Path, names: &[String]) -> Result<String> {
    crate::install_root::ensure_project_write_allowed("doctor leftover")?;
    let present = present_config_names(root).context("listing project config files")?;
    let winner = present.first().map(String::as_str);
    for name in names {
        if winner == Some(name.as_str()) {
            bail!("refusing to remove discover winner {name}");
        }
        if !present.iter().any(|existing| existing == name) {
            bail!("refusing to remove {name}: not a present project config");
        }
        let path = root.join(name);
        std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(format!("removed shadowed {}", names.join(", ")))
}

enum ArchitectureLeftover {
    None,
    Unrecorded,
    Shadowed,
}

fn architecture_leftover(root: &Path) -> ArchitectureLeftover {
    let legacy = anvil_architecture::yaml_parser::architecture_yaml_exists(root);
    if !legacy {
        return ArchitectureLeftover::None;
    }
    let section = crate::commands::config::load_project_config(root)
        .ok()
        .and_then(|project| {
            project
                .value
                .get("architecture")
                .filter(|value| !value.is_null())
                .map(|_| ())
        })
        .is_some();
    if !section {
        return ArchitectureLeftover::Unrecorded;
    }
    if architecture_delegates_to_legacy(root) {
        ArchitectureLeftover::None
    } else {
        ArchitectureLeftover::Shadowed
    }
}

fn architecture_delegates_to_legacy(root: &Path) -> bool {
    let legacy_path = anvil_architecture::yaml_parser::get_architecture_yaml_path(root);
    match crate::architecture_source::resolve_architecture(root) {
        Ok(Some((
            _,
            crate::architecture_source::ArchitectureOrigin::Section(
                anvil_config::SectionProvenance::Delegated { path, .. },
            ),
        ))) => legacy_path
            .canonicalize()
            .is_ok_and(|canonical_legacy| canonical_legacy == path),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn write(root: &Path, rel: &str, body: &str) {
        if let Some(parent) = root.join(rel).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(root.join(rel), body).unwrap();
    }

    #[test]
    fn should_offer_only_on_interactive_tty() {
        assert!(should_offer_leftover_cleanup(false, true, false));
        assert!(!should_offer_leftover_cleanup(true, true, false));
        assert!(!should_offer_leftover_cleanup(false, false, false));
        assert!(!should_offer_leftover_cleanup(false, true, true));
    }

    #[test]
    fn leftover_offers_empty_when_single_canonical() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "checks: []\n");
        assert!(leftover_offers_in(tmp.path()).is_empty());
    }

    #[test]
    fn leftover_offers_lone_anvilrc() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvilrc", r#"{"checks":[]}"#);
        let offers = leftover_offers_in(tmp.path());
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].kind, LeftoverKind::LoneAnvilrc);
        assert!(matches!(
            offers[0].choices.as_slice(),
            [
                LeftoverChoice::MigrateAnvilrc { remove_old: false },
                LeftoverChoice::MigrateAnvilrc { remove_old: true },
            ]
        ));
    }

    #[test]
    fn leftover_offers_shadowed_anvilrc_does_not_include_winner() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "checks: []\n");
        write(tmp.path(), ".anvilrc", "{}");
        let offers = leftover_offers_in(tmp.path());
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].kind, LeftoverKind::ShadowedConfigs);
        match &offers[0].choices[0] {
            LeftoverChoice::RemoveShadowedConfigs { names } => {
                assert_eq!(names, &[".anvilrc".to_string()]);
            }
            other => panic!("expected remove shadowed, got {other:?}"),
        }
    }

    #[test]
    fn leftover_offers_dual_canonical_removes_shadowed_not_winner() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "checks: []\n");
        write(tmp.path(), ".anvil.json", "{}");
        let offers = leftover_offers_in(tmp.path());
        assert_eq!(offers.len(), 1);
        match &offers[0].choices[0] {
            LeftoverChoice::RemoveShadowedConfigs { names } => {
                assert_eq!(names, &[".anvil.json".to_string()]);
                assert!(!names.iter().any(|name| name == ".anvil.yaml"));
            }
            other => panic!("expected remove shadowed, got {other:?}"),
        }
    }

    #[test]
    fn leftover_offers_gate_config_and_unrecorded_architecture() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "checks: [secret-detection]\n");
        write(
            tmp.path(),
            ".anvil/gate-config.json",
            r#"{"version":1,"checks":[{"name":"lint","description":"","enabled":true}],"thresholds":{}}"#,
        );
        write(
            tmp.path(),
            ".anvil/architecture.yaml",
            "schema_version: \"0.1.0\"\n",
        );
        let kinds: Vec<_> = leftover_offers_in(tmp.path())
            .into_iter()
            .map(|offer| offer.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                LeftoverKind::LegacyGateConfig,
                LeftoverKind::UnrecordedArchitecture,
            ]
        );
    }

    #[test]
    fn leftover_offers_shadowed_architecture_not_delegated() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "architecture:\n  layers: {}\n");
        write(
            tmp.path(),
            ".anvil/architecture.yaml",
            "schema_version: \"0.1.0\"\n",
        );
        let offers = leftover_offers_in(tmp.path());
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].kind, LeftoverKind::ShadowedArchitecture);
    }

    #[test]
    fn leftover_offers_empty_when_architecture_delegates_to_legacy() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            ".anvil.yaml",
            "architecture:\n  source: \".anvil/architecture.yaml\"\n",
        );
        write(
            tmp.path(),
            ".anvil/architecture.yaml",
            "schema_version: \"0.1.0\"\n",
        );
        assert!(leftover_offers_in(tmp.path()).is_empty());
    }

    #[test]
    fn parse_choice_numbers_skip_and_unknown() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvilrc", "{}");
        let offer = leftover_offers_in(tmp.path()).pop().unwrap();
        assert!(matches!(
            parse_choice("1", &offer),
            LeftoverChoice::MigrateAnvilrc { remove_old: false }
        ));
        assert!(matches!(
            parse_choice("2", &offer),
            LeftoverChoice::MigrateAnvilrc { remove_old: true }
        ));
        assert_eq!(parse_choice("s", &offer), LeftoverChoice::Skip);
        assert_eq!(parse_choice("", &offer), LeftoverChoice::Skip);
        assert_eq!(parse_choice("9", &offer), LeftoverChoice::Skip);
        assert_eq!(parse_choice("nope", &offer), LeftoverChoice::Skip);
    }

    #[test]
    fn apply_migrate_anvilrc_writes_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvilrc", r#"{"checks":["secret-detection"]}"#);
        apply_leftover_choice(
            tmp.path(),
            &LeftoverChoice::MigrateAnvilrc { remove_old: true },
        )
        .unwrap();
        assert!(tmp.path().join(".anvil.yaml").is_file());
        assert!(!tmp.path().join(".anvilrc").exists());
    }

    #[test]
    fn apply_remove_shadowed_keeps_winner() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "checks: []\n");
        write(tmp.path(), ".anvilrc", "{}");
        apply_leftover_choice(
            tmp.path(),
            &LeftoverChoice::RemoveShadowedConfigs {
                names: vec![".anvilrc".to_string()],
            },
        )
        .unwrap();
        assert!(tmp.path().join(".anvil.yaml").is_file());
        assert!(!tmp.path().join(".anvilrc").exists());
    }

    #[test]
    fn apply_remove_refuses_discover_winner() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "checks: []\n");
        write(tmp.path(), ".anvil.json", "{}");
        let err = apply_leftover_choice(
            tmp.path(),
            &LeftoverChoice::RemoveShadowedConfigs {
                names: vec![".anvil.yaml".to_string()],
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("discover winner"), "{err}");
        assert!(tmp.path().join(".anvil.yaml").is_file());
        assert!(tmp.path().join(".anvil.json").is_file());
    }

    #[test]
    fn apply_skip_is_noop() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvilrc", "{}");
        apply_leftover_choice(tmp.path(), &LeftoverChoice::Skip).unwrap();
        assert!(tmp.path().join(".anvilrc").is_file());
        assert!(!tmp.path().join(".anvil.yaml").exists());
    }

    #[test]
    fn apply_record_architecture_source() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "checks: [lint]\n");
        write(
            tmp.path(),
            ".anvil/architecture.yaml",
            "schema_version: \"0.1.0\"\nlayers:\n  core:\n    patterns: [\"src/**\"]\n",
        );
        apply_leftover_choice(tmp.path(), &LeftoverChoice::RecordArchitectureSource).unwrap();
        let value = anvil_config::parse_file(&tmp.path().join(".anvil.yaml")).unwrap();
        assert_eq!(value["architecture"]["source"], ".anvil/architecture.yaml");
        assert!(tmp.path().join(".anvil/architecture.yaml").is_file());
    }

    #[test]
    fn apply_remove_shadowed_architecture() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "architecture:\n  layers: {}\n");
        write(
            tmp.path(),
            ".anvil/architecture.yaml",
            "schema_version: \"0.1.0\"\n",
        );
        apply_leftover_choice(tmp.path(), &LeftoverChoice::RemoveArchitectureYaml).unwrap();
        assert!(!tmp.path().join(".anvil/architecture.yaml").exists());
        assert!(tmp.path().join(".anvil.yaml").is_file());
    }

    #[test]
    fn run_offers_skip_writes_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "checks: []\n");
        write(tmp.path(), ".anvilrc", "{}");
        let offers = leftover_offers_in(tmp.path());
        let mut input = Cursor::new("s\n");
        let mut output = Vec::new();
        let applied = run_offers(tmp.path(), &offers, &mut input, &mut output).unwrap();
        assert!(applied.is_empty());
        assert!(tmp.path().join(".anvilrc").is_file());
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("leftover config"), "{text}");
        assert!(text.contains("[s] skip"), "{text}");
    }

    #[test]
    fn run_offers_applies_numbered_choice() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvil.yaml", "checks: []\n");
        write(tmp.path(), ".anvilrc", "{}");
        let offers = leftover_offers_in(tmp.path());
        let mut input = Cursor::new("1\n");
        let mut output = Vec::new();
        let applied = run_offers(tmp.path(), &offers, &mut input, &mut output).unwrap();
        assert_eq!(applied.len(), 1);
        assert!(!tmp.path().join(".anvilrc").exists());
        assert!(tmp.path().join(".anvil.yaml").is_file());
    }

    #[test]
    fn run_offers_eof_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), ".anvilrc", "{}");
        let offers = leftover_offers_in(tmp.path());
        let mut input = Cursor::new("");
        let mut output = Vec::new();
        let applied = run_offers(tmp.path(), &offers, &mut input, &mut output).unwrap();
        assert!(applied.is_empty());
        assert!(tmp.path().join(".anvilrc").is_file());
    }
}
