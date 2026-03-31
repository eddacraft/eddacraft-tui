use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    #[cfg(windows)]
    {
        return dirs::config_dir()
            .map(|d| d.join("anvil"))
            .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"));
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

    #[cfg(target_os = "macos")]
    if !primary.exists() {
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

/// Load credentials from the canonical path, with env var fallback.
///
/// Search order:
/// 1. `credentials_path()` (XDG path, with macOS fallback)
/// 2. `ANVIL_LICENSE` env var (plain-text token, for CI environments)
///
/// Env var credentials are returned directly and never persisted to disk.
pub fn load() -> Result<Option<Credentials>> {
    let path = credentials_path()?;
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let creds: Credentials = serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        return Ok(Some(creds));
    }

    // ANVIL_LICENSE env var — returned directly, never persisted to disk.
    // Expiry is enforced server-side via the /auth/verify endpoint.
    if let Ok(token) = std::env::var("ANVIL_LICENSE") {
        let token = token.trim();
        if !token.is_empty() {
            return Ok(Some(Credentials {
                license: token.to_string(),
                refresh_token: None,
                email: None,
                expires_at: None,
            }));
        }
    }

    Ok(None)
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
