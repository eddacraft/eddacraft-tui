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
    /// SCAN-002: per-line length guard (in bytes). Lines longer than this
    /// threshold are skipped before any regex is applied so that a
    /// pathological minified/base64/concatenated line cannot trigger
    /// worst-case backtracking across the 18 built-in patterns or
    /// user-supplied custom regexes.
    ///
    /// Threshold default rationale: 4096 bytes is short enough to neutralise
    /// the realistic `ReDoS` blast radius (catastrophic backtracking on
    /// `a*a*` style regexes scales with line length) while remaining well
    /// above any line we expect to see in real source code — Prettier's
    /// `printWidth` default is 80, the longest commonly-formatted line in
    /// generated assets is sub-1 KB, and the 4 KB ceiling still leaves
    /// minified bundles excluded at the file-extension layer
    /// (`.min.js`, `.min.css`, `.map`).
    ///
    /// `serde(default)` keeps the field backward-compatible with on-disk
    /// configs written before SCAN-002.
    #[serde(default = "default_max_line_bytes")]
    pub max_line_bytes: usize,
}

/// SCAN-002 default — see `SecretCheckConfig::max_line_bytes`.
const fn default_max_line_bytes() -> usize {
    4096
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
            max_line_bytes: default_max_line_bytes(),
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
    /// SCAN-002: total number of lines skipped because they exceeded
    /// `SecretCheckConfig::max_line_bytes`. Surfaced so reviewers can tell
    /// "0 findings" from "we skipped a 4 MB minified bundle line that
    /// might have held a secret". `serde(default)` keeps the field
    /// backward-compatible.
    #[serde(default)]
    pub lines_skipped_oversize: usize,
}
