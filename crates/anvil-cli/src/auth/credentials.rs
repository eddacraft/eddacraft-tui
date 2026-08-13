use std::fs::File;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Sidecar next to `credentials.json`. The credential file is replaced
/// atomically on save, so the rotation lock cannot live on that inode.
const REFRESH_LOCK_FILE: &str = "credentials.lock";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    #[serde(alias = "token")]
    pub license: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Explicit marker recorded when the credential was minted via the
    /// early-access edict flow. `None` covers credentials saved before this
    /// field existed; [`is_edict`] treats unmarked credentials as ordinary
    /// sessions (CIB-221 — the `anvil_beta_` prefix alone is not enough).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_edict: Option<bool>,
}

/// Returns the canonical write directory for credentials.
///
/// On Windows, returns `%APPDATA%/anvil` via `dirs::config_dir()` since
/// there is no XDG convention.
///
/// On Unix (Linux + macOS), returns `$XDG_CONFIG_HOME/anvil` or
/// `~/.config/anvil`. On macOS this is intentional — we migrate
/// credentials FROM `~/Library/Application Support/anvil/` TO
/// `~/.config/anvil/` by always writing to the XDG path.
pub fn credentials_dir() -> Result<PathBuf> {
    // DISTRIB-006 (ADR-060): under a non-default ANVIL_HOME, user-owned state
    // (credentials) re-roots to `<ANVIL_HOME>/user/` so a pre-release candidate
    // never reads or writes the production login. Unset = platform default below.
    if let Some(user_dir) = crate::install_root::install_root().user_dir() {
        return Ok(user_dir);
    }

    // No `return`: once the other arm is cfg-stripped this block is the
    // function's tail expression (CIB-193).
    #[cfg(windows)]
    {
        dirs::config_dir()
            .map(|d| d.join("anvil"))
            .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))
    }

    #[cfg(not(windows))]
    {
        let config_home = std::env::var("XDG_CONFIG_HOME").map_or_else(
            |_| {
                dirs::home_dir()
                    .map(|h| h.join(".config"))
                    .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))
            },
            |v| Ok(PathBuf::from(v)),
        )?;
        Ok(config_home.join("anvil"))
    }
}

/// Returns the path used for reading credentials.
///
/// Checks the canonical XDG path first. On macOS, if that doesn't exist,
/// falls back to `~/Library/Application Support/anvil/credentials.json`
/// so that beta users who stored credentials via the native macOS
/// convention are not unexpectedly logged out.
///
/// Writes always use `credentials_dir()` (the XDG path), so the next
/// `save()` after a fallback read will migrate credentials to the
/// canonical location automatically.
pub fn credentials_path() -> Result<PathBuf> {
    let primary = credentials_dir()?.join("credentials.json");

    // DISTRIB-006: under a non-default ANVIL_HOME the candidate is isolated to
    // `<ANVIL_HOME>/user/` — do not fall back to the production macOS credential
    // location, or the candidate would read the prod login it is meant to avoid.
    #[cfg(target_os = "macos")]
    if !primary.exists() && !crate::install_root::install_root().is_overridden() {
        if let Some(home) = dirs::home_dir() {
            let macos_fallback = home
                .join("Library")
                .join("Application Support")
                .join("anvil")
                .join("credentials.json");
            if macos_fallback.exists() {
                return Ok(macos_fallback);
            }
        }
    }

    Ok(primary)
}

/// Where a loaded credential came from. Surfaced so commands can explain *why*
/// an identity is known without an explicit login this session — the recurring
/// confusion behind GH #2587, where `anvil auth whoami` succeeded from a
/// previously-stored credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialSource {
    /// Read from the on-disk credentials file at this path (a prior login).
    StoredFile(PathBuf),
    /// Supplied via the `ANVIL_LICENSE` environment variable (CI/automation).
    Environment,
}

impl CredentialSource {
    /// A short human phrase naming the source, e.g.
    /// `stored credentials (~/.config/anvil/credentials.json)`.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::StoredFile(path) => format!("stored credentials ({})", path.display()),
            Self::Environment => "ANVIL_LICENSE environment variable".to_string(),
        }
    }

    /// Stable machine token for `--json` output (`file` | `env`).
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::StoredFile(_) => "file",
            Self::Environment => "env",
        }
    }
}

/// Load credentials from the canonical path, with env var fallback.
///
/// Search order:
/// 1. `credentials_path()` (XDG path, with macOS fallback)
/// 2. `ANVIL_LICENSE` env var (plain-text token, for CI environments)
///
/// Env var credentials are returned directly and never persisted to disk.
pub fn load() -> Result<Option<Credentials>> {
    Ok(load_with_source()?.map(|(creds, _)| creds))
}

/// Like [`load`], but also reports the [`CredentialSource`]. Use this when the
/// caller needs to tell the user *where* an identity came from (GH #2587).
pub fn load_with_source() -> Result<Option<(Credentials, CredentialSource)>> {
    let path = credentials_path()?;
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let creds: Credentials = serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        return Ok(Some((creds, CredentialSource::StoredFile(path))));
    }

    // ANVIL_LICENSE env var — returned directly, never persisted to disk.
    // Expiry is enforced server-side via the /auth/verify endpoint.
    if let Ok(token) = std::env::var("ANVIL_LICENSE") {
        let token = token.trim();
        if !token.is_empty() {
            return Ok(Some((
                Credentials {
                    license: token.to_string(),
                    refresh_token: None,
                    email: None,
                    expires_at: None,
                    is_edict: None,
                },
                CredentialSource::Environment,
            )));
        }
    }

    Ok(None)
}

/// Inter-process exclusive lock for the credential refresh transaction.
///
/// Held across load → `/session/refresh` → save so two CLI processes cannot
/// exchange the same rotating refresh token. Atomic replace of
/// `credentials.json` protects file integrity but does not serialise that
/// transaction; reuse of a rotated token revokes the whole session family.
pub struct CredentialRefreshLock {
    _file: File,
}

impl CredentialRefreshLock {
    /// Block until this process owns the refresh lock.
    pub fn acquire() -> Result<Self> {
        let file = open_refresh_lock()?;
        fs2::FileExt::lock_exclusive(&file).context("locking credentials refresh transaction")?;
        Ok(Self { _file: file })
    }
}

fn open_refresh_lock() -> Result<File> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt as _;

    let dir = credentials_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(REFRESH_LOCK_FILE);
    let mut options = std::fs::OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }
    options
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))
}

/// Save credentials atomically to the canonical XDG path.
///
/// Uses `crate::util::atomic_write` which creates a random temp file
/// (preventing symlink attacks and concurrent collisions) and handles
/// Windows overwrite semantics.
pub fn save(creds: &Credentials) -> Result<()> {
    let dir = credentials_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    let path = dir.join("credentials.json");
    let content = serde_json::to_string_pretty(creds)?;
    crate::util::atomic_write(&path, content.as_bytes())
}

/// Remove credential files, including macOS fallback location.
///
/// On macOS, clears both the canonical XDG path and the
/// `~/Library/Application Support/anvil/` fallback so that logout is
/// effective regardless of where credentials were originally stored.
pub fn clear() -> Result<()> {
    let path = credentials_dir()?.join("credentials.json");
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }

    // On macOS, also clear the fallback location so that `load()` does
    // not re-discover stale credentials after logout.
    #[cfg(target_os = "macos")]
    if let Some(home) = dirs::home_dir() {
        let macos_fallback = home
            .join("Library")
            .join("Application Support")
            .join("anvil")
            .join("credentials.json");
        if macos_fallback.exists() {
            std::fs::remove_file(&macos_fallback)
                .with_context(|| format!("removing {}", macos_fallback.display()))?;
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub fn is_expired(creds: &Credentials) -> bool {
    match &creds.expires_at {
        None => false,
        Some(expires) => match DateTime::parse_from_rfc3339(expires) {
            Ok(dt) => dt.with_timezone(&Utc) <= Utc::now(),
            Err(_) => true,
        },
    }
}

pub fn is_edict(creds: &Credentials) -> bool {
    // Prefer the explicit marker recorded at edict-login time; that is the
    // source of truth from `/auth/verify`'s `isEdict` field.
    //
    // CIB-221: when the marker is missing, do **not** treat `anvil_beta_*`
    // as an edict. That prefix is also the general access-token format, so
    // the old heuristic misclassified authenticated pro/beta sessions and
    // forced a live edict re-verify (and a false "run auth login --edict"
    // nag when the network check failed). Unmarked credentials are ordinary
    // sessions; only `is_edict: Some(true)` requires the edict path.
    creds.is_edict.unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_creds(license: &str) -> Credentials {
        Credentials {
            license: license.to_string(),
            refresh_token: None,
            email: None,
            expires_at: None,
            is_edict: None,
        }
    }

    #[test]
    fn not_expired_when_no_expiry() {
        let creds = make_creds("tok");
        assert!(!is_expired(&creds));
    }

    #[test]
    fn explicit_is_edict_flag_takes_precedence_over_prefix() {
        // `anvil_beta_*` is the general access-token format; an explicit
        // `is_edict: Some(false)` must win so a regular service token isn't
        // misclassified as an edict by accident.
        let creds = Credentials {
            is_edict: Some(false),
            ..make_creds("anvil_beta_service_token")
        };
        assert!(!is_edict(&creds));

        // Conversely, an explicit `Some(true)` lets future credential
        // formats register as edicts even if the license doesn't start with
        // the `anvil_beta_` prefix.
        let creds = Credentials {
            is_edict: Some(true),
            ..make_creds("future_edict_format")
        };
        assert!(is_edict(&creds));
    }

    #[test]
    fn unmarked_credentials_are_not_edicts() {
        // CIB-221: `is_edict: None` is an ordinary session, even when the
        // licence uses the shared `anvil_beta_*` access-token prefix.
        assert!(!is_edict(&make_creds("anvil_beta_abc")));
        assert!(!is_edict(&make_creds("jwt.header.payload")));
        assert!(is_edict(&Credentials {
            is_edict: Some(true),
            ..make_creds("anvil_beta_edict")
        }));
    }

    #[test]
    fn expired_when_in_the_past() {
        let creds = Credentials {
            expires_at: Some("2020-01-01T00:00:00Z".to_string()),
            ..make_creds("tok")
        };
        assert!(is_expired(&creds));
    }

    #[test]
    fn not_expired_when_in_the_future() {
        let creds = Credentials {
            expires_at: Some("2099-12-31T23:59:59Z".to_string()),
            ..make_creds("tok")
        };
        assert!(!is_expired(&creds));
    }

    #[test]
    fn expired_when_unparseable_date() {
        let creds = Credentials {
            expires_at: Some("not-a-date".to_string()),
            ..make_creds("tok")
        };
        assert!(is_expired(&creds));
    }

    #[test]
    fn serde_roundtrip_full() {
        let creds = Credentials {
            license: "lic-123".to_string(),
            refresh_token: Some("refresh-456".to_string()),
            email: Some("user@example.com".to_string()),
            expires_at: Some("2099-12-31T23:59:59Z".to_string()),
            is_edict: Some(true),
        };
        let json = serde_json::to_string(&creds).unwrap();
        let parsed: Credentials = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.license, "lic-123");
        assert_eq!(parsed.refresh_token.as_deref(), Some("refresh-456"));
        assert_eq!(parsed.email.as_deref(), Some("user@example.com"));
    }

    #[test]
    fn serde_camel_case_field_names() {
        let creds = Credentials {
            license: "tok".to_string(),
            refresh_token: Some("rt".to_string()),
            email: None,
            expires_at: Some("2099-01-01T00:00:00Z".to_string()),
            is_edict: Some(true),
        };
        let json = serde_json::to_string(&creds).unwrap();
        assert!(json.contains("refreshToken"), "should use camelCase");
        assert!(json.contains("expiresAt"), "should use camelCase");
        assert!(!json.contains("refresh_token"), "should not use snake_case");
    }

    #[test]
    fn serde_skips_none_fields() {
        let creds = make_creds("tok");
        let json = serde_json::to_string(&creds).unwrap();
        assert!(!json.contains("refreshToken"));
        assert!(!json.contains("email"));
        assert!(!json.contains("expiresAt"));
        assert!(!json.contains("isEdict"));
    }

    #[test]
    fn serde_accepts_token_alias() {
        let json = r#"{"token": "my-token"}"#;
        let creds: Credentials = serde_json::from_str(json).unwrap();
        assert_eq!(creds.license, "my-token");
    }

    #[test]
    #[cfg(unix)]
    fn save_load_clear_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        temp_env::with_vars(
            [
                ("XDG_CONFIG_HOME", Some(dir.path().to_str().unwrap())),
                ("ANVIL_LICENSE", None),
            ],
            || {
                let loaded = load().unwrap();
                assert!(loaded.is_none());

                let creds = Credentials {
                    license: "test-lic".to_string(),
                    refresh_token: Some("test-refresh".to_string()),
                    email: Some("test@example.com".to_string()),
                    expires_at: Some("2099-01-01T00:00:00Z".to_string()),
                    is_edict: None,
                };
                save(&creds).unwrap();

                let loaded = load().unwrap().expect("should find saved credentials");
                assert_eq!(loaded.license, "test-lic");
                assert_eq!(loaded.email.as_deref(), Some("test@example.com"));

                clear().unwrap();
                let loaded = load().unwrap();
                assert!(loaded.is_none());
            },
        );
    }

    #[test]
    #[cfg(unix)]
    fn load_from_env_var() {
        let dir = tempfile::tempdir().unwrap();
        temp_env::with_vars(
            [
                ("XDG_CONFIG_HOME", Some(dir.path().to_str().unwrap())),
                ("ANVIL_LICENSE", Some("env-token")),
            ],
            || {
                let loaded = load().unwrap().expect("should load from env var");
                assert_eq!(loaded.license, "env-token");
                assert!(loaded.refresh_token.is_none());
            },
        );
    }

    #[test]
    #[cfg(unix)]
    fn load_empty_env_var_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        temp_env::with_vars(
            [
                ("XDG_CONFIG_HOME", Some(dir.path().to_str().unwrap())),
                ("ANVIL_LICENSE", Some("  ")),
            ],
            || {
                let loaded = load().unwrap();
                assert!(loaded.is_none());
            },
        );
    }

    #[test]
    #[cfg(unix)]
    fn file_credentials_take_priority_over_env() {
        let dir = tempfile::tempdir().unwrap();
        temp_env::with_vars(
            [
                ("XDG_CONFIG_HOME", Some(dir.path().to_str().unwrap())),
                ("ANVIL_LICENSE", Some("env-token")),
            ],
            || {
                let creds = Credentials {
                    license: "file-token".to_string(),
                    refresh_token: None,
                    email: None,
                    expires_at: None,
                    is_edict: None,
                };
                save(&creds).unwrap();

                let loaded = load().unwrap().expect("should find credentials");
                assert_eq!(loaded.license, "file-token");
            },
        );
    }

    #[test]
    #[cfg(unix)]
    fn load_with_source_reports_file_vs_env_vs_none() {
        // GH #2587: callers need to know *where* a credential came from so they
        // can explain why an identity is known without an explicit login.
        let dir = tempfile::tempdir().unwrap();
        temp_env::with_vars(
            [
                ("XDG_CONFIG_HOME", Some(dir.path().to_str().unwrap())),
                ("ANVIL_LICENSE", None),
            ],
            || {
                // None when nothing is present.
                assert!(load_with_source().unwrap().is_none());

                // A stored file reports `StoredFile(path)`.
                save(&make_creds("file-token")).unwrap();
                let (creds, source) = load_with_source().unwrap().expect("stored creds");
                assert_eq!(creds.license, "file-token");
                assert!(matches!(source, CredentialSource::StoredFile(_)));
                assert_eq!(source.kind(), "file");
            },
        );

        // With no file, the env var reports `Environment`.
        let empty = tempfile::tempdir().unwrap();
        temp_env::with_vars(
            [
                ("XDG_CONFIG_HOME", Some(empty.path().to_str().unwrap())),
                ("ANVIL_LICENSE", Some("env-token")),
            ],
            || {
                let (creds, source) = load_with_source().unwrap().expect("env creds");
                assert_eq!(creds.license, "env-token");
                assert_eq!(source, CredentialSource::Environment);
                assert_eq!(source.kind(), "env");
            },
        );
    }

    #[test]
    fn credential_source_describe_is_human_readable() {
        assert_eq!(
            CredentialSource::Environment.describe(),
            "ANVIL_LICENSE environment variable"
        );
        let file =
            CredentialSource::StoredFile(PathBuf::from("/home/u/.config/anvil/credentials.json"));
        assert!(file.describe().starts_with("stored credentials ("));
        assert!(file.describe().contains("credentials.json"));
    }

    #[test]
    #[cfg(unix)]
    fn credentials_dir_respects_xdg() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let xdg_config_home = tmp_dir.path().to_str().unwrap();

        temp_env::with_var("XDG_CONFIG_HOME", Some(xdg_config_home), || {
            let dir = credentials_dir().unwrap();
            let expected = tmp_dir.path().join("anvil");
            assert_eq!(dir, expected);
        });
    }

    #[test]
    #[cfg(unix)]
    fn refresh_lock_file_is_created_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().unwrap();
        temp_env::with_vars(
            [
                ("XDG_CONFIG_HOME", Some(dir.path().to_str().unwrap())),
                ("ANVIL_LICENSE", None),
            ],
            || {
                let _lock = CredentialRefreshLock::acquire().unwrap();
                let path = credentials_dir().unwrap().join(REFRESH_LOCK_FILE);
                assert!(path.is_file(), "refresh lock sidecar must exist");
                let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o600, "refresh lock must be owner-only, got {mode:o}");
            },
        );
    }
}
