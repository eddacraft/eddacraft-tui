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
    #[error("parse error: {0}")]
    Parse(String),
}

pub fn load_config(path: &str) -> Result<PolicyConfig, ConfigError> {
    let content =
        std::fs::read_to_string(path).map_err(|_| ConfigError::NotFound(path.to_string()))?;
    serde_yaml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
}
