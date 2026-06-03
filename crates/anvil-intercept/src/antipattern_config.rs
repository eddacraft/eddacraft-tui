//! DSV-011: operator config surface for the save-time antipattern check set.
//!
//! Without this, the daemon ran `validate_paths`' antipattern family on a
//! hardcoded [`AntipatternCheckConfig::default`] — an operator could not
//! select patterns, opt-in extras, the scanned extensions, or the severity
//! threshold the save-time verdict applies. This module loads that config from
//! an operator-owned `antipattern.yaml` beside the confinement config.
//!
//! Trust model mirrors [`crate::confinement`] and reuses its one audited
//! owner-only reader ([`crate::confinement::read_trusted`]): the file is read
//! owner-only via an `O_NOFOLLOW` open (no symlinked leaf, no group/world
//! writability), resolved through the daemon's own
//! [`crate::confinement::anvil_config_dir`] (`ANVIL_HOME`/XDG/HOME — never an
//! `anvil-cli` path), and **never** read from a repo `.anvil.yaml`.
//!
//! Failure posture is *fail-safe + loud*, the safe direction for a warnings
//! surface: a missing file folds into the full default check set; an
//! **untrusted or malformed** file is logged at `error` and also folds into the
//! full default set ([`load_or_fail_safe`]) — a broken config never silently
//! *disables* save-time checks (the dangerous direction), and it never silently
//! degrades either (it is loud). The fallible [`load`] / [`load_from`] remain
//! available for callers that want to propagate the error instead.

use std::path::{Path, PathBuf};

use anvil_checks::antipattern::types::{AntipatternCheckConfig, WarningSeverity};
use serde::Deserialize;
use thiserror::Error;

use crate::confinement::{self, ConfinementError};

/// Basename of the operator antipattern config under the resolved config dir.
const CONFIG_FILE_NAME: &str = "antipattern.yaml";

/// Errors loading the operator antipattern config. Every variant is a *loud*
/// failure; the production [`load_or_fail_safe`] logs it and falls back to the
/// full default check set rather than trusting an untrusted file or silently
/// disabling checks.
#[derive(Debug, Error)]
pub enum AntipatternConfigError {
    /// No `ANVIL_HOME`/XDG/HOME candidate to resolve the config dir from.
    #[error(
        "cannot resolve an antipattern config directory (no ANVIL_HOME, XDG_CONFIG_HOME, or HOME)"
    )]
    NoConfigDir,
    /// IO error reading the config file.
    #[error("antipattern config IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The config file is not owner-only (wrong owner or group/world-writable).
    #[error(
        "antipattern config {path} is not owner-only (mode {mode:#o}, owner uid {owner_uid}, current uid {current_uid})"
    )]
    NotOwnerOnly {
        path: PathBuf,
        mode: u32,
        owner_uid: u32,
        current_uid: u32,
    },
    /// The config path is a symlink — refused (it could redirect the read to a
    /// file another principal controls).
    #[error("antipattern config {0} is a symlink — refusing (it could redirect the read)")]
    SymlinkedConfig(PathBuf),
    /// The config file exists but does not parse (includes an unknown/misspelt
    /// key — the on-disk form is `deny_unknown_fields`).
    #[error("antipattern config {path} is malformed: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
}

impl AntipatternConfigError {
    /// Map the shared confinement reader's error into this module's error, so
    /// the confinement-flavoured Display strings never surface to an operator
    /// debugging their antipattern config. `read_trusted`/`anvil_config_dir`
    /// only ever yield `NoConfigDir`/`Io`/`NotOwnerOnly`/`SymlinkedConfig`; the
    /// catch-all defensively wraps anything else as `Io` rather than panicking.
    fn from_reader(fallback_path: &Path, err: ConfinementError) -> Self {
        match err {
            ConfinementError::NoConfigDir => Self::NoConfigDir,
            ConfinementError::Io { path, source } => Self::Io { path, source },
            ConfinementError::NotOwnerOnly {
                path,
                mode,
                owner_uid,
                current_uid,
            } => Self::NotOwnerOnly {
                path,
                mode,
                owner_uid,
                current_uid,
            },
            ConfinementError::SymlinkedConfig(path) => Self::SymlinkedConfig(path),
            other => Self::Io {
                path: fallback_path.to_path_buf(),
                source: std::io::Error::other(other.to_string()),
            },
        }
    }
}

/// On-disk form of the operator antipattern config. `deny_unknown_fields` makes
/// a misspelt key (e.g. `severityThreshhold:`) a *loud parse error*, not a
/// silently-ignored field. Every field is optional; an omitted field takes the
/// [`AntipatternCheckConfig::default`] value, so a partial file overrides only
/// what it names.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct AntipatternConfigFile {
    /// The patterns to run; empty / omitted ⇒ the daemon's built-in set.
    #[serde(default)]
    patterns: Option<Vec<String>>,
    /// Whether to include opt-in patterns.
    #[serde(rename = "includeOptIn", default)]
    include_opt_in: Option<bool>,
    /// File extensions the scan considers; omitted ⇒ the default extension set.
    #[serde(default)]
    extensions: Option<Vec<String>>,
    /// The minimum severity a finding must reach to be reported.
    #[serde(rename = "severityThreshold", default)]
    severity_threshold: Option<WarningSeverity>,
}

impl AntipatternConfigFile {
    /// Overlay the named fields onto the default config (omitted ⇒ default).
    fn into_config(self) -> AntipatternCheckConfig {
        let default = AntipatternCheckConfig::default();
        AntipatternCheckConfig {
            patterns: self.patterns.unwrap_or(default.patterns),
            include_opt_in: self.include_opt_in.unwrap_or(default.include_opt_in),
            extensions: self.extensions.unwrap_or(default.extensions),
            severity_threshold: self
                .severity_threshold
                .unwrap_or(default.severity_threshold),
        }
    }
}

/// Load + resolve the antipattern config from an explicit path.
///
/// A missing file folds into [`AntipatternCheckConfig::default`] (`Ok`). A
/// present file must be owner-only and parse; otherwise this returns a *loud*
/// `Err` (the production caller fails safe on it).
pub fn load_from(path: &Path) -> Result<AntipatternCheckConfig, AntipatternConfigError> {
    let raw = match confinement::read_trusted(path) {
        Ok(Some(raw)) => raw,
        Ok(None) => return Ok(AntipatternCheckConfig::default()),
        Err(err) => return Err(AntipatternConfigError::from_reader(path, err)),
    };
    let file: AntipatternConfigFile =
        serde_yaml::from_str(&raw).map_err(|source| AntipatternConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(file.into_config())
}

/// Load the antipattern config from the resolved config dir.
pub fn load() -> Result<AntipatternCheckConfig, AntipatternConfigError> {
    let dir = confinement::anvil_config_dir()
        .map_err(|err| AntipatternConfigError::from_reader(Path::new(CONFIG_FILE_NAME), err))?;
    load_from(&dir.join(CONFIG_FILE_NAME))
}

/// Production loader: load the antipattern config, **failing safe + loud** on an
/// untrusted or malformed config. The fail-safe posture is the full default
/// check set — a broken config never silently disables save-time checks (the
/// dangerous direction for a warnings surface), and the error is logged at
/// `error` so the degraded state is observable, never silent.
#[must_use]
pub fn load_or_fail_safe() -> AntipatternCheckConfig {
    resolve_or_fail_safe(load())
}

/// Pure policy mapper for [`load_or_fail_safe`] — unit-testable without mutating
/// process env. `Ok` passes through; `NoConfigDir` (no resolvable location) and
/// every untrusted/malformed error fall back to the full default check set,
/// the former with a `warn`, the latter with a louder `error`.
fn resolve_or_fail_safe(
    result: Result<AntipatternCheckConfig, AntipatternConfigError>,
) -> AntipatternCheckConfig {
    match result {
        Ok(config) => config,
        Err(AntipatternConfigError::NoConfigDir) => {
            tracing::warn!(
                "no antipattern config directory could be resolved \
                 (no ANVIL_HOME/XDG_CONFIG_HOME/HOME) — using the default check set"
            );
            AntipatternCheckConfig::default()
        }
        Err(err) => {
            tracing::error!(
                error = %err,
                "antipattern config load failed — falling back to the full default \
                 check set (fail-safe: a broken config never silently disables \
                 save-time checks)"
            );
            AntipatternCheckConfig::default()
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    /// A missing config file is not an error — it folds into the default set.
    #[test]
    fn missing_file_is_default() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("antipattern.yaml");
        let loaded = load_from(&path).expect("missing file is Ok(default)");
        assert_eq!(loaded, AntipatternCheckConfig::default());
    }

    /// A partial config overrides only the fields it names; the rest stay at
    /// the default.
    #[test]
    fn partial_config_overlays_default() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("antipattern.yaml");
        fs::write(&path, "includeOptIn: true\nseverityThreshold: warning\n").expect("write");
        set_owner_only(&path);

        let loaded = load_from(&path).expect("valid config");
        let default = AntipatternCheckConfig::default();
        assert!(loaded.include_opt_in, "includeOptIn override applied");
        assert_eq!(loaded.severity_threshold, WarningSeverity::Warning);
        // Untouched fields keep the default.
        assert_eq!(loaded.patterns, default.patterns);
        assert_eq!(loaded.extensions, default.extensions);
    }

    /// An unknown/misspelt key is a loud parse error (`deny_unknown_fields`),
    /// not a silently-ignored field.
    #[test]
    fn unknown_key_is_a_parse_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("antipattern.yaml");
        fs::write(&path, "severityThreshhold: warning\n").expect("write"); // typo
        set_owner_only(&path);

        let err = load_from(&path).expect_err("typo must fail loudly");
        assert!(
            matches!(err, AntipatternConfigError::Parse { .. }),
            "got {err:?}",
        );
    }

    /// A group/world-writable config is refused — fail-safe load returns the
    /// default rather than trusting a file another principal could rewrite.
    #[test]
    fn group_writable_file_is_refused() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("antipattern.yaml");
        fs::write(&path, "includeOptIn: true\n").expect("write");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).expect("chmod");

        let err = load_from(&path).expect_err("group-writable must be refused");
        assert!(
            matches!(err, AntipatternConfigError::NotOwnerOnly { .. }),
            "got {err:?}",
        );
        // And the production posture folds it to the default, loudly (not a panic).
        assert_eq!(
            resolve_or_fail_safe(Err(err)),
            AntipatternCheckConfig::default(),
        );
    }

    /// A symlinked config leaf is refused (it could redirect the read).
    #[test]
    fn symlinked_config_is_refused() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let real = tmp.path().join("real.yaml");
        fs::write(&real, "includeOptIn: true\n").expect("write");
        set_owner_only(&real);
        let link = tmp.path().join("antipattern.yaml");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");

        let err = load_from(&link).expect_err("symlinked config must be refused");
        assert!(
            matches!(err, AntipatternConfigError::SymlinkedConfig(_)),
            "got {err:?}",
        );
    }

    /// `NoConfigDir` is the absent-location case: fail-safe folds it to the
    /// default with a `warn`, not the louder untrusted-config `error`.
    #[test]
    fn no_config_dir_falls_back_to_default() {
        assert_eq!(
            resolve_or_fail_safe(Err(AntipatternConfigError::NoConfigDir)),
            AntipatternCheckConfig::default(),
        );
    }

    /// Set a file owner-only (0o600) so the owner-only reader accepts it. Tests
    /// that create files via `fs::write` would otherwise inherit the umask.
    fn set_owner_only(path: &Path) {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("chmod 600");
    }
}
