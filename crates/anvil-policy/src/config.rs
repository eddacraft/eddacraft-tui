use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub policies: Vec<PolicyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub enabled: bool,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config not found: {0}")]
    NotFound(String),
    #[error("I/O error reading config {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error: {0}")]
    Parse(String),
}

pub fn load_config(path: &str) -> Result<PolicyConfig, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ConfigError::NotFound(path.to_string())
        } else {
            ConfigError::Io {
                path: path.to_string(),
                source: e,
            }
        }
    })?;
    serde_yaml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_config_parses_valid_yaml() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let yaml = r#"
policies:
  - id: "policy-1"
    name: "Example policy"
    category: "testing"
    enabled: true
    description: "A test policy"
    severity: "medium"
"#;
        std::fs::write(tmp.path(), yaml).unwrap();

        let config = load_config(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(config.policies.len(), 1);
        assert_eq!(config.policies[0].id, "policy-1");
        assert!(config.policies[0].enabled);
    }

    #[test]
    fn load_config_returns_parse_error_for_invalid_yaml() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "policies: [unclosed").unwrap();

        match load_config(tmp.path().to_str().unwrap()) {
            Err(ConfigError::Parse(_)) => {}
            other => panic!("expected Parse error, got: {other:?}"),
        }
    }

    #[test]
    fn load_config_returns_not_found_for_missing_file() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let missing = tmpdir.path().join("does-not-exist.yaml");
        match load_config(missing.to_str().unwrap()) {
            Err(ConfigError::NotFound(_)) => {}
            other => panic!("expected NotFound, got: {other:?}"),
        }
    }
}
