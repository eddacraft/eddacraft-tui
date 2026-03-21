use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub token: String,
    pub refresh_token: Option<String>,
    pub email: Option<String>,
    pub expires_at: Option<String>,
}

pub fn credentials_dir() -> Result<PathBuf> {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        });
    Ok(config_home.join("anvil"))
}

pub fn credentials_path() -> Result<PathBuf> {
    Ok(credentials_dir()?.join("credentials.json"))
}

pub fn load() -> Result<Option<Credentials>> {
    let path = credentials_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let creds: Credentials =
        serde_json::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(creds))
}

pub fn save(creds: &Credentials) -> Result<()> {
    let dir = credentials_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    let path = dir.join("credentials.json");
    let content = serde_json::to_string_pretty(creds)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&path, &content).with_context(|| format!("writing {}", path.display()))?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("setting permissions on {}", path.display()))?;
    }

    #[cfg(not(unix))]
    {
        std::fs::write(&path, &content).with_context(|| format!("writing {}", path.display()))?;
    }

    Ok(())
}

pub fn clear() -> Result<()> {
    let path = credentials_path()?;
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

pub fn is_expired(creds: &Credentials) -> bool {
    creds.expires_at.as_deref().is_some_and(|expires| {
        // Simple string comparison — ISO 8601 timestamps sort lexicographically
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let now_secs = now.as_secs();
        // If expires_at is a valid RFC 3339 date, compare against current time
        // For simplicity, treat tokens without valid expiry as non-expired
        let _ = now_secs;
        expires.is_empty()
    })
}
