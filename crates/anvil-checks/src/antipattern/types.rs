use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WarningCategory {
    AntiPattern,
    Boundary,
    Architecture,
}

/// Artifact kinds that the scanner understands. Must stay in sync with the
/// `.anvil` frontmatter `targets` enum and the TS `ArtifactKind` type so both
/// engines filter rules identically.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    Source,
    PrDescription,
    CommitMessage,
    AgentOutput,
}

impl ArtifactKind {
    /// Wire-format string (matches the `targets` values in compiled
    /// registry patterns).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::PrDescription => "pr-description",
            Self::CommitMessage => "commit-message",
            Self::AgentOutput => "agent-output",
        }
    }

    #[must_use]
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "source" => Some(Self::Source),
            "pr-description" => Some(Self::PrDescription),
            "commit-message" => Some(Self::CommitMessage),
            "agent-output" => Some(Self::AgentOutput),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WarningSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SuppressionScope {
    Statement,
    Import,
    File,
    Line,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: Option<usize>,
    #[serde(rename = "endLine")]
    pub end_line: Option<usize>,
    #[serde(rename = "endColumn")]
    pub end_column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Suppression {
    pub reason: String,
    pub author: Option<String>,
    pub timestamp: Option<String>,
    pub scope: SuppressionScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Warning {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    pub category: WarningCategory,
    pub severity: WarningSeverity,
    pub confidence: Confidence,
    pub title: String,
    pub message: String,
    pub explanation: String,
    pub suggestion: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nudge: Option<String>,
    pub location: Location,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppressed: Option<Suppression>,
    /// Family id this rule belongs to (e.g. "guardrail-suppression").
    /// Populated for registry-sourced patterns; omitted for legacy hardcoded
    /// patterns still in `PATTERN_DEFS` until RSCAN-004 retires them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spectrum_position: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AntiPatternCategory {
    EscapeHatch,
    ErrorHandling,
    CodeQuality,
    TypeSafety,
    TypeEvasion,
    Accountability,
    DeferredDebt,
    Html,
    Css,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AntiPattern {
    pub id: String,
    pub name: String,
    pub category: AntiPatternCategory,
    pub severity: WarningSeverity,
    pub confidence: Confidence,
    pub regex: String,
    pub title: String,
    pub explanation: String,
    pub suggestion: String,
    pub nudge: Option<String>,
    #[serde(rename = "fileExtensions")]
    pub file_extensions: Option<Vec<String>>,
    #[serde(rename = "allFileTypes")]
    pub all_file_types: bool,
    pub allowlist: Vec<String>,
    pub threshold: Option<usize>,
    pub enabled: bool,
    #[serde(rename = "optIn")]
    pub opt_in: bool,
    /// Family provenance (optional). Populated for patterns sourced from the
    /// compiled `.anvil` registry; `None` for legacy hardcoded patterns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spectrum_position: Option<u32>,
    /// Artifact types this pattern targets. `None` = legacy (source-only)
    /// behaviour; populated from registry `targets` when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WarningSummary {
    pub total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub suppressed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WarningResult {
    pub warnings: Vec<Warning>,
    pub summary: WarningSummary,
    pub patterns_checked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AntipatternCheckConfig {
    pub patterns: Vec<String>,
    #[serde(rename = "includeOptIn")]
    pub include_opt_in: bool,
    pub extensions: Vec<String>,
    #[serde(rename = "severityThreshold")]
    pub severity_threshold: WarningSeverity,
}

impl Default for AntipatternCheckConfig {
    fn default() -> Self {
        Self {
            patterns: Vec::new(),
            include_opt_in: false,
            extensions: vec![
                ".ts".to_string(),
                ".tsx".to_string(),
                ".js".to_string(),
                ".jsx".to_string(),
                ".mjs".to_string(),
                ".cjs".to_string(),
                // RSTLAN-006: `.rs` joins the default scan set so the
                // already-`.rs`-scoped deferred-debt rules (DD-001 TODO/FIXME,
                // DD-002 HACK/XXX, DD-003 temporary/workaround — all un-ticketed)
                // fire on Rust across `anvil check`/`gate`/drift and the
                // save-time daemon, the same as TS. JS/TS-specific rules stay
                // extension-restricted.
                ".rs".to_string(),
                ".html".to_string(),
                ".htm".to_string(),
                ".css".to_string(),
                ".scss".to_string(),
                ".less".to_string(),
            ],
            severity_threshold: WarningSeverity::Error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AntipatternCheckResult {
    pub passed: bool,
    pub score: u8,
    pub message: String,
    pub warnings: WarningResult,
    pub files_scanned: usize,
    pub patterns_checked: Vec<String>,
}

#[must_use]
pub fn create_warning_fingerprint(warning: &Warning) -> String {
    let pattern = warning.pattern.clone().unwrap_or_default();
    format!(
        "{}:{}:{}:{}",
        warning.id, warning.location.file, warning.location.line, pattern
    )
}

#[must_use]
pub fn count_by_severity(warnings: &[Warning]) -> WarningSummary {
    let mut summary = WarningSummary {
        total: warnings.len(),
        ..WarningSummary::default()
    };

    for warning in warnings {
        if warning.suppressed.is_some() {
            summary.suppressed += 1;
        } else {
            match warning.severity {
                WarningSeverity::Error => summary.errors += 1,
                WarningSeverity::Warning => summary.warnings += 1,
                WarningSeverity::Info => summary.info += 1,
            }
        }
    }

    summary
}

#[must_use]
pub fn create_warning_result(
    warnings: Vec<Warning>,
    patterns_checked: Vec<String>,
) -> WarningResult {
    let summary = count_by_severity(&warnings);
    WarningResult {
        warnings,
        summary,
        patterns_checked,
    }
}

#[must_use]
pub fn validate_warning_result_consistency(result: &WarningResult) -> bool {
    let computed = count_by_severity(&result.warnings);
    result.summary == computed
}

#[cfg(test)]
mod tests {
    use crate::antipattern::types::{
        Confidence, Location, Suppression, SuppressionScope, Warning, WarningCategory,
        WarningSeverity, count_by_severity, create_warning_fingerprint, create_warning_result,
        validate_warning_result_consistency,
    };

    fn sample_warning(
        id: &str,
        severity: WarningSeverity,
        suppressed: Option<Suppression>,
    ) -> Warning {
        Warning {
            id: id.to_string(),
            fingerprint: None,
            category: WarningCategory::AntiPattern,
            severity,
            confidence: Confidence::High,
            title: "Sample".to_string(),
            message: "Sample message".to_string(),
            explanation: "Sample explanation".to_string(),
            suggestion: "Sample suggestion".to_string(),
            nudge: None,
            family: None,
            definition_ref: None,
            spectrum_position: None,
            location: Location {
                file: "src/a.ts".to_string(),
                line: 2,
                column: Some(3),
                end_line: None,
                end_column: None,
            },
            pattern: Some(id.to_string()),
            suppressed,
        }
    }

    #[test]
    fn fingerprint_uses_expected_format() {
        let warning = sample_warning("AP-001", WarningSeverity::Warning, None);
        assert_eq!(
            create_warning_fingerprint(&warning),
            "AP-001:src/a.ts:2:AP-001"
        );
    }

    #[test]
    fn counts_severity_with_suppressed_bucket() {
        let suppressed = Suppression {
            reason: "legacy path".to_string(),
            author: None,
            timestamp: None,
            scope: SuppressionScope::Line,
        };
        let warnings = vec![
            sample_warning("AP-001", WarningSeverity::Warning, None),
            sample_warning("AP-002", WarningSeverity::Info, None),
            sample_warning("AP-003", WarningSeverity::Error, Some(suppressed)),
        ];

        let summary = count_by_severity(&warnings);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.errors, 0);
        assert_eq!(summary.warnings, 1);
        assert_eq!(summary.info, 1);
        assert_eq!(summary.suppressed, 1);
    }

    #[test]
    fn warning_result_consistency_is_valid_when_generated() {
        let warnings = vec![sample_warning("AP-006", WarningSeverity::Warning, None)];
        let result = create_warning_result(warnings, vec!["AP-006".to_string()]);
        assert!(validate_warning_result_consistency(&result));
    }
}
