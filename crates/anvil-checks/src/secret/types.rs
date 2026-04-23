use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPatternDef {
    pub name: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretCheckConfig {
    pub enable_entropy: bool,
    pub entropy_threshold: f64,
    pub min_entropy_length: usize,
    pub scan_git_history: bool,
    pub git_history_depth: usize,
    pub skip_extensions: Vec<String>,
    pub custom_patterns: Vec<SecretPatternDef>,
    pub custom_allowlist: Vec<String>,
}

impl Default for SecretCheckConfig {
    fn default() -> Self {
        Self {
            enable_entropy: true,
            entropy_threshold: 4.5,
            min_entropy_length: 16,
            scan_git_history: false,
            git_history_depth: 10,
            skip_extensions: vec![
                ".lock".to_string(),
                ".min.js".to_string(),
                ".min.css".to_string(),
                ".map".to_string(),
                ".svg".to_string(),
                ".png".to_string(),
                ".jpg".to_string(),
                ".jpeg".to_string(),
                ".gif".to_string(),
                ".ico".to_string(),
                ".woff".to_string(),
                ".woff2".to_string(),
                ".ttf".to_string(),
                ".eot".to_string(),
            ],
            custom_patterns: Vec::new(),
            custom_allowlist: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFinding {
    pub file: String,
    pub line: usize,
    pub finding_type: FindingType,
    pub pattern_name: String,
    pub redacted_match: String,
    pub redacted_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingType {
    Pattern,
    Entropy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyFinding {
    pub file: String,
    pub line: usize,
    pub entropy: f64,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretCheckResult {
    pub passed: bool,
    pub score: u8,
    pub message: String,
    pub findings: Vec<SecretFinding>,
    /// Configuration errors surfaced during the run — currently populated when
    /// a `custom_patterns` regex fails to compile. Wire-compatible with
    /// pre-EAMIG-003 consumers via `serde(default)`.
    #[serde(default)]
    pub pattern_errors: Vec<String>,
}
