use std::io::Write;
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
        use std::fs::OpenOptions;
        use std::os::unix::fs::OpenOptionsExt;
        use std::os::unix::fs::PermissionsExt;

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .with_context(|| format!("opening {}", path.display()))?;
        file.write_all(content.as_bytes())
            .with_context(|| format!("writing {}", path.display()))?;

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
