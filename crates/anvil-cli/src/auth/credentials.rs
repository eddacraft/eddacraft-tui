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

pub fn credentials_dir() -> PathBuf {
    let config_home = std::env::var("XDG_CONFIG_HOME").map_or_else(
        |_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        },
        PathBuf::from,
    );
    config_home.join("anvil")
}

pub fn credentials_path() -> PathBuf {
    credentials_dir().join("credentials.json")
}

pub fn load() -> Result<Option<Credentials>> {
    let path = credentials_path();
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
    let dir = credentials_dir();
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
    let path = credentials_path();
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

pub fn is_expired(creds: &Credentials) -> bool {
    match creds.expires_at.as_deref() {
        None => false,
        Some(expires_str) => {
            if expires_str.is_empty() {
                return false;
            }

            let Ok(expires_ts) = expires_str.parse::<u64>() else {
                return false;
            };

            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            now_secs >= expires_ts
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialise_roundtrip() {
        let creds = Credentials {
            token: "access-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            email: Some("user@example.com".to_string()),
            expires_at: Some("1234567890".to_string()),
        };

        let tmp = tempfile::NamedTempFile::new().unwrap();
        let json = serde_json::to_string_pretty(&creds).unwrap();
        std::fs::write(tmp.path(), &json).unwrap();

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        let loaded: Credentials = serde_json::from_str(&content).unwrap();

        assert_eq!(loaded.token, creds.token);
        assert_eq!(loaded.refresh_token, creds.refresh_token);
        assert_eq!(loaded.email, creds.email);
        assert_eq!(loaded.expires_at, creds.expires_at);
    }

    #[test]
    fn is_expired_future_token() {
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        let creds = Credentials {
            token: "t".into(),
            refresh_token: None,
            email: None,
            expires_at: Some(future.to_string()),
        };
        assert!(!is_expired(&creds));
    }

    #[test]
    fn is_expired_past_token() {
        let past = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(3600);
        let creds = Credentials {
            token: "t".into(),
            refresh_token: None,
            email: None,
            expires_at: Some(past.to_string()),
        };
        assert!(is_expired(&creds));
    }

    #[test]
    fn is_expired_none_empty_invalid() {
        let none = Credentials {
            token: "t".into(),
            refresh_token: None,
            email: None,
            expires_at: None,
        };
        assert!(!is_expired(&none));

        let empty = Credentials {
            token: "t".into(),
            refresh_token: None,
            email: None,
            expires_at: Some(String::new()),
        };
        assert!(!is_expired(&empty));

        let invalid = Credentials {
            token: "t".into(),
            refresh_token: None,
            email: None,
            expires_at: Some("not-a-timestamp".into()),
        };
        assert!(!is_expired(&invalid));
    }
}
