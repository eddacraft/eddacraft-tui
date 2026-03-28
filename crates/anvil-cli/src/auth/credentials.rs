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

pub fn credentials_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join("anvil"))
        .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))
}

pub fn credentials_path() -> Result<PathBuf> {
    Ok(credentials_dir()?.join("credentials.json"))
}

/// Load credentials, checking XDG path first then legacy locations.
///
/// Search order:
/// 1. `$XDG_CONFIG_HOME/anvil/credentials.json`
/// 2. `~/.anvil/auth.json` (legacy JSON)
/// 3. `~/.anvil/license` (legacy plain-text token)
/// 4. `ANVIL_LICENSE` env var (plain-text token)
///
/// When credentials are found at a legacy path (2--3), they are
/// automatically migrated to the XDG location. Env var credentials
/// (4) are returned directly and never persisted to disk.
pub fn load() -> Result<Option<Credentials>> {
    load_with_fallback()
}

/// Try each credential source in priority order, returning the first hit.
///
/// This is the public entry point that `load()` delegates to. Separated so
/// callers that need the explicit name (e.g. for documentation or wiring)
/// can reference it directly.
pub fn load_with_fallback() -> Result<Option<Credentials>> {
    let xdg_path = credentials_path()?;
    let home = dirs::home_dir();
    let legacy_auth = home.as_ref().map(|h| h.join(".anvil/auth.json"));
    let legacy_license = home.as_ref().map(|h| h.join(".anvil/license"));
    let env_token = std::env::var("ANVIL_LICENSE").ok();

    resolve_credentials(
        &xdg_path,
        legacy_auth.as_deref(),
        legacy_license.as_deref(),
        env_token.as_deref(),
    )
}

/// Resolve credentials from concrete paths and an optional env token.
///
/// Extracted so tests can supply all inputs without touching the real
/// filesystem or process environment.
fn resolve_credentials(
    xdg_path: &std::path::Path,
    legacy_auth: Option<&std::path::Path>,
    legacy_license: Option<&std::path::Path>,
    env_token: Option<&str>,
) -> Result<Option<Credentials>> {
    // 1. XDG path (canonical)
    if xdg_path.exists() {
        let content = std::fs::read_to_string(xdg_path)
            .with_context(|| format!("reading {}", xdg_path.display()))?;
        let creds: Credentials = serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", xdg_path.display()))?;
        return Ok(Some(creds));
    }

    // 2. Legacy ~/.anvil/auth.json
    if let Some(path) = legacy_auth
        && path.exists()
    {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let creds: Credentials = serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        migrate_to_xdg(xdg_path, &creds)?;
        return Ok(Some(creds));
    }

    // 3. Legacy ~/.anvil/license (plain-text token)
    if let Some(path) = legacy_license
        && path.exists()
    {
        let token =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let token = token.trim();
        if !token.is_empty() {
            let creds = Credentials {
                license: token.to_string(),
                refresh_token: None,
                email: None,
                expires_at: None,
            };
            migrate_to_xdg(xdg_path, &creds)?;
            return Ok(Some(creds));
        }
    }

    // 4. ANVIL_LICENSE env var — returned directly, never persisted to disk
    if let Some(token) = env_token {
        let token = token.trim();
        if !token.is_empty() {
            let creds = Credentials {
                license: token.to_string(),
                refresh_token: None,
                email: None,
                expires_at: None,
            };
            return Ok(Some(creds));
        }
    }

    Ok(None)
}

/// Atomically write `content` to `path` via a random temp file + rename.
///
/// Uses `tempfile` for unpredictable filenames (prevents symlink attacks).
/// On Unix the temp file is created with mode 0o600.
fn atomic_write(path: &std::path::Path, content: &[u8]) -> Result<()> {
    use std::io::Write;

    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("no parent directory for {}", path.display()))?;

    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;

    let mut tmp = tempfile::NamedTempFile::new_in(dir)
        .with_context(|| format!("creating temp file in {}", dir.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .with_context(|| "setting temp file permissions")?;
    }

    tmp.write_all(content)
        .with_context(|| format!("writing temp file for {}", path.display()))?;
    tmp.flush()?;

    let tmp_path = tmp.into_temp_path();
    let tmp_display = tmp_path.display().to_string();
    tmp_path
        .persist(path)
        .with_context(|| format!("persisting {tmp_display} -> {}", path.display()))?;

    Ok(())
}

/// Copy credentials to the XDG location and print a migration notice.
fn migrate_to_xdg(xdg_path: &std::path::Path, creds: &Credentials) -> Result<()> {
    if let Some(dir) = xdg_path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }

    let content = serde_json::to_string_pretty(creds)?;
    atomic_write(xdg_path, content.as_bytes())?;

    eprintln!("Migrated credentials \u{2192} {}", xdg_path.display());
    Ok(())
}

pub fn save(creds: &Credentials) -> Result<()> {
    let dir = credentials_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    let path = dir.join("credentials.json");
    let content = serde_json::to_string_pretty(creds)?;
    atomic_write(&path, content.as_bytes())
}

pub fn clear() -> Result<()> {
    let path = credentials_path()?;
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_creds_json() -> String {
        serde_json::to_string_pretty(&Credentials {
            license: "test-token-abc".to_string(),
            refresh_token: Some("refresh-xyz".to_string()),
            email: Some("user@example.com".to_string()),
            expires_at: Some("2099-01-01T00:00:00Z".to_string()),
        })
        .unwrap()
    }

    // ── XDG path (priority 1) ────────────────────────────────────

    #[test]
    fn load_from_xdg_path() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::write(&xdg, sample_creds_json()).unwrap();

        let result = resolve_credentials(&xdg, None, None, None).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().license, "test-token-abc");
    }

    // ── Legacy auth.json (priority 2) ────────────────────────────

    #[test]
    fn load_from_legacy_auth_json() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let legacy = tmp.path().join(".anvil/auth.json");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, sample_creds_json()).unwrap();

        let result = resolve_credentials(&xdg, Some(&legacy), None, None).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().license, "test-token-abc");
    }

    #[test]
    fn legacy_auth_json_migrates_to_xdg() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let legacy = tmp.path().join(".anvil/auth.json");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, sample_creds_json()).unwrap();

        resolve_credentials(&xdg, Some(&legacy), None, None).unwrap();
        assert!(xdg.exists(), "credentials should be migrated to XDG path");

        let migrated: Credentials =
            serde_json::from_str(&std::fs::read_to_string(&xdg).unwrap()).unwrap();
        assert_eq!(migrated.license, "test-token-abc");
    }

    // ── Legacy license file (priority 3) ─────────────────────────

    #[test]
    fn load_from_legacy_license_file() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let license_file = tmp.path().join(".anvil/license");
        std::fs::create_dir_all(license_file.parent().unwrap()).unwrap();
        std::fs::write(&license_file, "plain-text-token\n").unwrap();

        let result = resolve_credentials(&xdg, None, Some(&license_file), None).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().license, "plain-text-token");
    }

    #[test]
    fn legacy_license_file_migrates_to_xdg() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let license_file = tmp.path().join(".anvil/license");
        std::fs::create_dir_all(license_file.parent().unwrap()).unwrap();
        std::fs::write(&license_file, "  token-with-whitespace  \n").unwrap();

        resolve_credentials(&xdg, None, Some(&license_file), None).unwrap();
        assert!(xdg.exists());

        let migrated: Credentials =
            serde_json::from_str(&std::fs::read_to_string(&xdg).unwrap()).unwrap();
        assert_eq!(migrated.license, "token-with-whitespace");
    }

    #[test]
    fn empty_license_file_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let license_file = tmp.path().join(".anvil/license");
        std::fs::create_dir_all(license_file.parent().unwrap()).unwrap();
        std::fs::write(&license_file, "  \n").unwrap();

        let result = resolve_credentials(&xdg, None, Some(&license_file), None).unwrap();
        assert!(result.is_none());
    }

    // ── ANVIL_LICENSE env var (priority 4) ────────────────────────

    #[test]
    fn load_from_env_token() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");

        let result = resolve_credentials(&xdg, None, None, Some("env-token-123")).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().license, "env-token-123");
    }

    #[test]
    fn empty_env_token_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");

        let result = resolve_credentials(&xdg, None, None, Some("   ")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn env_token_does_not_persist_to_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");

        let result = resolve_credentials(&xdg, None, None, Some("env-only-token")).unwrap();
        assert_eq!(result.unwrap().license, "env-only-token");
        assert!(
            !xdg.exists(),
            "env var credentials must not be written to disk"
        );
    }

    // ── Priority order ───────────────────────────────────────────

    #[test]
    fn xdg_takes_priority_over_legacy() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let legacy = tmp.path().join(".anvil/auth.json");

        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();

        let xdg_creds = Credentials {
            license: "xdg-token".to_string(),
            refresh_token: None,
            email: None,
            expires_at: None,
        };
        let legacy_creds = Credentials {
            license: "legacy-token".to_string(),
            refresh_token: None,
            email: None,
            expires_at: None,
        };

        std::fs::write(&xdg, serde_json::to_string_pretty(&xdg_creds).unwrap()).unwrap();
        std::fs::write(
            &legacy,
            serde_json::to_string_pretty(&legacy_creds).unwrap(),
        )
        .unwrap();

        let result = resolve_credentials(&xdg, Some(&legacy), None, None).unwrap();
        assert_eq!(result.unwrap().license, "xdg-token");
    }

    #[test]
    fn legacy_auth_json_takes_priority_over_license_file() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let legacy_auth = tmp.path().join(".anvil/auth.json");
        let legacy_license = tmp.path().join(".anvil/license");

        std::fs::create_dir_all(legacy_auth.parent().unwrap()).unwrap();

        let auth_creds = Credentials {
            license: "auth-json-token".to_string(),
            refresh_token: None,
            email: None,
            expires_at: None,
        };
        std::fs::write(
            &legacy_auth,
            serde_json::to_string_pretty(&auth_creds).unwrap(),
        )
        .unwrap();
        std::fs::write(&legacy_license, "license-file-token").unwrap();

        let result = resolve_credentials(
            &xdg,
            Some(&legacy_auth),
            Some(&legacy_license),
            Some("env-token"),
        )
        .unwrap();
        assert_eq!(result.unwrap().license, "auth-json-token");
    }

    #[test]
    fn license_file_takes_priority_over_env() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let legacy_license = tmp.path().join(".anvil/license");

        std::fs::create_dir_all(legacy_license.parent().unwrap()).unwrap();
        std::fs::write(&legacy_license, "file-token").unwrap();

        let result =
            resolve_credentials(&xdg, None, Some(&legacy_license), Some("env-token")).unwrap();
        assert_eq!(result.unwrap().license, "file-token");
    }

    // ── No credentials anywhere ──────────────────────────────────

    #[test]
    fn returns_none_when_no_credentials() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");

        let result = resolve_credentials(&xdg, None, None, None).unwrap();
        assert!(result.is_none());
    }

    // ── Migration sets correct permissions ────────────────────────

    #[cfg(unix)]
    #[test]
    fn migrated_file_has_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let legacy = tmp.path().join(".anvil/auth.json");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, sample_creds_json()).unwrap();

        resolve_credentials(&xdg, Some(&legacy), None, None).unwrap();

        let perms = std::fs::metadata(&xdg).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }

    // ── XDG path not overwritten by migration ─────────────────────

    #[test]
    fn xdg_not_overwritten_when_already_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg = tmp.path().join("config/anvil/credentials.json");
        let legacy = tmp.path().join(".anvil/auth.json");

        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();

        let xdg_creds = Credentials {
            license: "xdg-token".to_string(),
            refresh_token: None,
            email: None,
            expires_at: None,
        };
        std::fs::write(&xdg, serde_json::to_string_pretty(&xdg_creds).unwrap()).unwrap();
        std::fs::write(&legacy, sample_creds_json()).unwrap();

        resolve_credentials(&xdg, Some(&legacy), None, None).unwrap();

        // XDG file should still have the original token, not the legacy one
        let content: Credentials =
            serde_json::from_str(&std::fs::read_to_string(&xdg).unwrap()).unwrap();
        assert_eq!(content.license, "xdg-token");
    }

    // ── Token alias ──────────────────────────────────────────────

    #[test]
    fn deserialises_token_alias_as_license() {
        let json = r#"{"token": "aliased-value"}"#;
        let creds: Credentials = serde_json::from_str(json).unwrap();
        assert_eq!(creds.license, "aliased-value");
    }

    // ── credentials_dir uses dirs::config_dir ───────────────────

    #[test]
    fn credentials_dir_matches_platform_config_dir() {
        if let Some(expected) = dirs::config_dir() {
            let dir = credentials_dir().unwrap();
            assert_eq!(dir, expected.join("anvil"));
        }
    }

    // ── load_with_fallback is equivalent to load ────────────────

    #[test]
    fn load_with_fallback_delegates_correctly() {
        let r1 = load();
        let r2 = load_with_fallback();
        assert_eq!(r1.is_ok(), r2.is_ok());
    }
}
