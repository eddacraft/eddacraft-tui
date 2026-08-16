//! Daily MCP self-heal (MCPLH-008).
//!
//! `anvil`, `anvil start`, and `anvil doctor` rewrite drifted Anvil-owned
//! MCP entries and poke live children when the CLI, daemon, or configs
//! change. `anvil mcp refresh` remains the explicit emergency cascade.
//!
//! Pin (`anvil mcp pin` or `ANVIL_MCP_PIN`) freezes daily heal and
//! in-process re-exec. Emergency refresh still runs.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::commands::mcp_generation::{
    bump_generation, generation_path, read_generation, write_generation_sidecar,
};

/// Session override. `0`/`false`/`no`/`off` forces auto even if a pin file
/// exists; `1`/`true`/`yes`/`on` freezes current; any other non-empty value
/// is treated as a pinned version label.
pub(crate) const PIN_ENV: &str = "ANVIL_MCP_PIN";
const PIN_FILE_NAME: &str = "mcp-heal.pin";
const CLI_VERSION_FILE_NAME: &str = "mcp-refresh.cli-version";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Why a daily path wants to poke live MCP children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PokeReason {
    /// Daily ensure/start/doctor: bump only when something actually changed.
    Changed {
        configs_rewritten: bool,
        daemon_recycled: bool,
    },
    /// Emergency `anvil mcp refresh` always pokes.
    Emergency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HealPolicy {
    Auto,
    Pinned {
        source: PinSource,
        version: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PinSource {
    Env,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PokeOutcome {
    pub bumped: bool,
    pub generation: u64,
    pub skipped_pin: bool,
}

impl HealPolicy {
    #[must_use]
    pub(crate) fn is_pinned(&self) -> bool {
        matches!(self, Self::Pinned { .. })
    }

    #[must_use]
    pub(crate) fn summary(&self) -> String {
        match self {
            Self::Auto => "auto".to_owned(),
            Self::Pinned {
                source: PinSource::Env,
                version: Some(version),
            } => format!("pinned to {version} ({PIN_ENV})"),
            Self::Pinned {
                source: PinSource::Env,
                version: None,
            } => format!("pinned ({PIN_ENV})"),
            Self::Pinned {
                source: PinSource::File,
                version: Some(version),
            } => format!("pinned to {version} (anvil mcp pin)"),
            Self::Pinned {
                source: PinSource::File,
                version: None,
            } => "pinned (anvil mcp pin)".to_owned(),
        }
    }
}

/// Install-scoped pin file, next to the refresh generation counter.
pub(crate) fn pin_path() -> Result<PathBuf> {
    Ok(generation_path()?.with_file_name(PIN_FILE_NAME))
}

fn cli_version_path() -> Result<PathBuf> {
    Ok(generation_path()?.with_file_name(CLI_VERSION_FILE_NAME))
}

/// Current heal policy: env wins, then the pin file.
#[must_use]
pub(crate) fn heal_policy() -> HealPolicy {
    heal_policy_from(env::var_os(PIN_ENV).as_deref(), pin_path().ok().as_deref())
}

#[must_use]
pub(crate) fn heal_policy_from(env_value: Option<&OsStr>, pin_file: Option<&Path>) -> HealPolicy {
    if let Some(raw) = env_value {
        let text = raw.to_string_lossy();
        let text = text.trim();
        if text.is_empty() {
            // fall through to file
        } else if is_falsey(text) {
            return HealPolicy::Auto;
        } else if is_truthy(text) {
            return HealPolicy::Pinned {
                source: PinSource::Env,
                version: None,
            };
        } else {
            return HealPolicy::Pinned {
                source: PinSource::Env,
                version: Some(text.to_owned()),
            };
        }
    }
    match pin_file.map(read_pin_file) {
        Some(PinFileRead::Freeze) => HealPolicy::Pinned {
            source: PinSource::File,
            version: None,
        },
        Some(PinFileRead::Version(version)) => HealPolicy::Pinned {
            source: PinSource::File,
            version: Some(version),
        },
        Some(PinFileRead::Absent) | None => HealPolicy::Auto,
    }
}

fn is_truthy(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn is_falsey(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PinFileRead {
    Absent,
    Freeze,
    Version(String),
}

fn read_pin_file(path: &Path) -> PinFileRead {
    let Ok(raw) = fs::read_to_string(path) else {
        return PinFileRead::Absent;
    };
    let text = raw.lines().next().unwrap_or("").trim();
    if text.is_empty() || is_truthy(text) {
        PinFileRead::Freeze
    } else if is_falsey(text) {
        PinFileRead::Absent
    } else {
        PinFileRead::Version(text.to_owned())
    }
}

/// Write a durable pin. `None` freezes whatever is current.
pub(crate) fn write_pin(version: Option<&str>) -> Result<PathBuf> {
    let path = pin_path()?;
    let body = match version.map(str::trim).filter(|value| !value.is_empty()) {
        Some(version) => format!("{version}\n"),
        None => "1\n".to_owned(),
    };
    write_generation_sidecar(&path, body.as_bytes())
        .with_context(|| format!("writing MCP heal pin {}", path.display()))?;
    Ok(path)
}

/// Remove a durable pin. Missing file is success.
pub(crate) fn clear_pin() -> Result<bool> {
    let path = pin_path()?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            Err(error).with_context(|| format!("removing MCP heal pin {}", path.display()))
        }
    }
}

/// Poke live MCP children when daily heal changed something, or always
/// for the emergency refresh verb.
pub(crate) fn poke_if_needed(reason: PokeReason) -> Result<PokeOutcome> {
    let generation = generation_path()?;
    let cli_version = cli_version_path()?;
    poke_if_needed_at(
        &generation,
        &cli_version,
        CLI_VERSION,
        &heal_policy(),
        reason,
    )
}

/// Surface a poke failure on stderr so daily paths are not silent.
pub(crate) fn warn_poke_failure(error: &anyhow::Error) {
    eprintln!("anvil: failed to poke live MCP children: {error}");
}

pub(crate) fn poke_if_needed_at(
    generation_file: &Path,
    cli_version_file: &Path,
    cli_version: &str,
    policy: &HealPolicy,
    reason: PokeReason,
) -> Result<PokeOutcome> {
    if matches!(reason, PokeReason::Changed { .. }) && policy.is_pinned() {
        return Ok(PokeOutcome {
            bumped: false,
            generation: read_generation(generation_file)?,
            skipped_pin: true,
        });
    }

    let should_bump = match reason {
        PokeReason::Emergency => true,
        PokeReason::Changed {
            configs_rewritten,
            daemon_recycled,
        } => {
            configs_rewritten
                || daemon_recycled
                || last_poked_cli_version(cli_version_file).as_deref() != Some(cli_version)
        }
    };

    if !should_bump {
        return Ok(PokeOutcome {
            bumped: false,
            generation: read_generation(generation_file)?,
            skipped_pin: false,
        });
    }

    let generation = bump_generation(generation_file)?;
    write_last_poked_cli_version(cli_version_file, cli_version)?;
    Ok(PokeOutcome {
        bumped: true,
        generation,
        skipped_pin: false,
    })
}

fn last_poked_cli_version(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let text = raw.lines().next().unwrap_or("").trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

fn write_last_poked_cli_version(path: &Path, version: &str) -> Result<()> {
    write_generation_sidecar(path, format!("{version}\n").as_bytes())
        .with_context(|| format!("writing last-poked CLI version {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;

    use super::{
        HealPolicy, PinFileRead, PinSource, PokeReason, heal_policy_from, poke_if_needed_at,
        read_pin_file,
    };
    use crate::commands::mcp_generation::read_generation;

    fn write(path: &Path, body: &str) {
        fs::write(path, body).expect("write fixture");
    }

    #[test]
    fn unset_env_and_missing_file_is_auto() {
        assert_eq!(heal_policy_from(None, None), HealPolicy::Auto);
    }

    #[test]
    fn falsey_env_overrides_pin_file() {
        let dir = tempfile::tempdir().unwrap();
        let pin = dir.path().join("mcp-heal.pin");
        write(&pin, "1\n");
        assert_eq!(
            heal_policy_from(Some(OsString::from("0").as_os_str()), Some(&pin)),
            HealPolicy::Auto
        );
    }

    #[test]
    fn truthy_env_pins_without_version() {
        assert_eq!(
            heal_policy_from(Some(OsString::from("true").as_os_str()), None),
            HealPolicy::Pinned {
                source: PinSource::Env,
                version: None,
            }
        );
    }

    #[test]
    fn version_env_pins_that_label() {
        assert_eq!(
            heal_policy_from(Some(OsString::from("0.9.2-beta").as_os_str()), None),
            HealPolicy::Pinned {
                source: PinSource::Env,
                version: Some("0.9.2-beta".into()),
            }
        );
    }

    #[test]
    fn pin_file_freezes_current() {
        let dir = tempfile::tempdir().unwrap();
        let pin = dir.path().join("mcp-heal.pin");
        write(&pin, "1\n");
        assert_eq!(
            heal_policy_from(None, Some(&pin)),
            HealPolicy::Pinned {
                source: PinSource::File,
                version: None,
            }
        );
    }

    #[test]
    fn pin_file_can_name_a_version() {
        let dir = tempfile::tempdir().unwrap();
        let pin = dir.path().join("mcp-heal.pin");
        write(&pin, "0.9.2-beta\n");
        assert_eq!(
            read_pin_file(&pin),
            PinFileRead::Version("0.9.2-beta".into())
        );
        assert_eq!(
            heal_policy_from(None, Some(&pin)),
            HealPolicy::Pinned {
                source: PinSource::File,
                version: Some("0.9.2-beta".into()),
            }
        );
    }

    #[test]
    fn daily_poke_bumps_when_cli_version_changes() {
        let dir = tempfile::tempdir().unwrap();
        let generation = dir.path().join("mcp-refresh.generation");
        let cli = dir.path().join("mcp-refresh.cli-version");
        write(&generation, "2\n");
        write(&cli, "0.9.3-beta\n");

        let outcome = poke_if_needed_at(
            &generation,
            &cli,
            "0.9.4-beta",
            &HealPolicy::Auto,
            PokeReason::Changed {
                configs_rewritten: false,
                daemon_recycled: false,
            },
        )
        .expect("poke");

        assert!(outcome.bumped);
        assert_eq!(outcome.generation, 3);
        assert_eq!(fs::read_to_string(&cli).unwrap().trim(), "0.9.4-beta");
    }

    #[test]
    fn daily_poke_stays_quiet_when_nothing_changed() {
        let dir = tempfile::tempdir().unwrap();
        let generation = dir.path().join("mcp-refresh.generation");
        let cli = dir.path().join("mcp-refresh.cli-version");
        write(&generation, "4\n");
        write(&cli, "0.9.4-beta\n");

        let outcome = poke_if_needed_at(
            &generation,
            &cli,
            "0.9.4-beta",
            &HealPolicy::Auto,
            PokeReason::Changed {
                configs_rewritten: false,
                daemon_recycled: false,
            },
        )
        .expect("poke");

        assert!(!outcome.bumped);
        assert_eq!(outcome.generation, 4);
        assert_eq!(read_generation(&generation).unwrap(), 4);
    }

    #[test]
    fn daily_poke_bumps_when_configs_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        let generation = dir.path().join("mcp-refresh.generation");
        let cli = dir.path().join("mcp-refresh.cli-version");
        write(&generation, "1\n");
        write(&cli, "0.9.4-beta\n");

        let outcome = poke_if_needed_at(
            &generation,
            &cli,
            "0.9.4-beta",
            &HealPolicy::Auto,
            PokeReason::Changed {
                configs_rewritten: true,
                daemon_recycled: false,
            },
        )
        .expect("poke");

        assert!(outcome.bumped);
        assert_eq!(outcome.generation, 2);
    }

    #[test]
    fn pin_blocks_daily_poke_even_when_cli_changed() {
        let dir = tempfile::tempdir().unwrap();
        let generation = dir.path().join("mcp-refresh.generation");
        let cli = dir.path().join("mcp-refresh.cli-version");
        write(&generation, "5\n");

        let outcome = poke_if_needed_at(
            &generation,
            &cli,
            "0.9.5-beta",
            &HealPolicy::Pinned {
                source: PinSource::File,
                version: None,
            },
            PokeReason::Changed {
                configs_rewritten: true,
                daemon_recycled: true,
            },
        )
        .expect("poke");

        assert!(outcome.skipped_pin);
        assert!(!outcome.bumped);
        assert_eq!(outcome.generation, 5);
    }

    #[test]
    fn emergency_refresh_pokes_even_when_pinned() {
        let dir = tempfile::tempdir().unwrap();
        let generation = dir.path().join("mcp-refresh.generation");
        let cli = dir.path().join("mcp-refresh.cli-version");
        write(&generation, "5\n");

        let outcome = poke_if_needed_at(
            &generation,
            &cli,
            "0.9.5-beta",
            &HealPolicy::Pinned {
                source: PinSource::Env,
                version: Some("0.9.2-beta".into()),
            },
            PokeReason::Emergency,
        )
        .expect("poke");

        assert!(outcome.bumped);
        assert!(!outcome.skipped_pin);
        assert_eq!(outcome.generation, 6);
    }
}
