use std::fmt;

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

/// Wrapper that makes a `Warning` renderable via `miette`.
///
/// Phase B: loads the source file at construction time so miette can render
/// an inline code excerpt with a span underline. Falls back to Phase A
/// (message-only) when the file is missing, binary, or unreadable.
pub struct WarningReport<'a> {
    warning: &'a Warning,
    source: Option<miette::NamedSource<String>>,
    span: Option<miette::SourceSpan>,
}

impl<'a> WarningReport<'a> {
    pub fn new(warning: &'a Warning) -> Self {
        let (source, span) = load_source_and_span(warning);
        Self {
            warning,
            source,
            span,
        }
    }
}

/// Read the source file and compute the byte span for `warning.location`.
/// Returns `(None, None)` on any I/O or parse error so the caller degrades
/// gracefully to Phase A rendering.
fn load_source_and_span(
    warning: &Warning,
) -> (
    Option<miette::NamedSource<String>>,
    Option<miette::SourceSpan>,
) {
    // Require a known byte column before loading source. Without it we cannot
    // place a meaningful underline, and secret findings deliberately omit the
    // column so the raw file content is never exposed (the `message` field is
    // already redacted by the scanner).
    if warning.location.column.is_none() {
        return (None, None);
    }
    let path = std::path::Path::new(&warning.location.file);
    let Ok(content) = std::fs::read_to_string(path) else {
        return (None, None);
    };
    let span = compute_span(&content, &warning.location);
    let named = miette::NamedSource::new(&warning.location.file, content);
    (Some(named), span)
}

/// Convert a `Location` to a byte `SourceSpan` inside `content`.
///
/// Lines are 1-based; `column` is a 0-based byte offset within the line
/// (from `regex::Match::start()`). Returns `None` when `column` is absent or
/// the offset falls outside the line's byte range so diagnostics degrade
/// gracefully rather than showing a misleading or out-of-bounds excerpt.
fn compute_span(content: &str, loc: &Location) -> Option<miette::SourceSpan> {
    if loc.line == 0 {
        return None;
    }
    let col = loc.column?;
    // Walk lines to find the byte offset of `loc.line` (1-based).
    // `split('\n')` keeps any trailing `\r` on Windows files, which is
    // correct: the scanner's column offsets are relative to the raw line bytes.
    let mut line_start: usize = 0;
    for (i, line) in content.split('\n').enumerate() {
        if i + 1 == loc.line {
            // Reject column offsets that fall at or past the end of this
            // line — the byte position does not exist in the line content.
            if col >= line.len() {
                return None;
            }
            let offset = line_start + col;
            let length = if let (Some(el), Some(ec)) = (loc.end_line, loc.end_column) {
                // Explicit end span (not currently populated by the scanner,
                // but honoured when present for future callers).
                let end_start: usize = content
                    .split('\n')
                    .take(el.saturating_sub(1))
                    .map(|l| l.len() + 1)
                    .sum();
                let end_offset = end_start + ec;
                end_offset.saturating_sub(offset).max(1)
            } else {
                // col < line.len() is guaranteed above, so this is >= 1.
                line.len() - col
            };
            // Clamp to actual content length so a malformed explicit end span
            // cannot produce a SourceSpan that exceeds the source buffer.
            let length = length.min(content.len().saturating_sub(offset)).max(1);
            return Some(miette::SourceSpan::new(offset.into(), length));
        }
        line_start += line.len() + 1; // +1 for the '\n' byte
    }
    None
}

impl fmt::Display for WarningReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let w = self.warning;
        write!(f, "{}:{}: {}", w.location.file, w.location.line, w.title)?;
        // Preserve the scanner's `message` detail (e.g. "Found … at line …"),
        // which the `title` headline alone drops — but only when it adds
        // information beyond the title.
        if !w.message.is_empty() && w.message != w.title {
            write!(f, " — {}", w.message)?;
        }
        Ok(())
    }
}

impl fmt::Debug for WarningReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for WarningReport<'_> {}

impl miette::Diagnostic for WarningReport<'_> {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(self.warning.id.as_str()))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(match self.warning.severity {
            WarningSeverity::Error => miette::Severity::Error,
            WarningSeverity::Warning => miette::Severity::Warning,
            WarningSeverity::Info => miette::Severity::Advice,
        })
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        if self.warning.suggestion.is_empty() {
            None
        } else {
            Some(Box::new(self.warning.suggestion.as_str()))
        }
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.source.as_ref().map(|s| s as &dyn miette::SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let span = self.span?;
        Some(Box::new(std::iter::once(
            miette::LabeledSpan::new_with_span(None, span),
        )))
    }
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
        WarningReport, WarningSeverity, count_by_severity, create_warning_fingerprint,
        create_warning_result, validate_warning_result_consistency,
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

    #[test]
    fn warning_report_display_preserves_message_detail() {
        let mut w = sample_warning("AP-010", WarningSeverity::Warning, None);
        w.title = "Anti-pattern detected".to_string();
        w.message = "Found `eslint-disable` at line 2".to_string();
        let shown = WarningReport::new(&w).to_string();
        assert!(
            shown.starts_with("src/a.ts:2: Anti-pattern detected"),
            "{shown}"
        );
        assert!(
            shown.contains("Found `eslint-disable` at line 2"),
            "message detail must survive Display: {shown}"
        );
    }

    #[test]
    fn warning_report_display_omits_redundant_message() {
        let mut w = sample_warning("AP-011", WarningSeverity::Warning, None);
        w.title = "Same text".to_string();
        w.message = "Same text".to_string();
        assert_eq!(WarningReport::new(&w).to_string(), "src/a.ts:2: Same text");
    }

    #[test]
    fn warning_report_diagnostic_maps_code_and_help() {
        use miette::Diagnostic;
        let w = sample_warning("AP-012", WarningSeverity::Warning, None);
        let report = WarningReport::new(&w);
        assert_eq!(
            report.code().map(|c| c.to_string()).as_deref(),
            Some("AP-012")
        );
        assert_eq!(
            report.help().map(|h| h.to_string()).as_deref(),
            Some("Sample suggestion")
        );
    }

    #[test]
    fn warning_report_help_is_none_without_suggestion() {
        use miette::Diagnostic;
        let mut w = sample_warning("AP-013", WarningSeverity::Info, None);
        w.suggestion = String::new();
        assert!(WarningReport::new(&w).help().is_none());
    }

    #[test]
    fn warning_report_severity_maps_each_level() {
        use miette::{Diagnostic, Severity};
        for (sev, want) in [
            (WarningSeverity::Error, Severity::Error),
            (WarningSeverity::Warning, Severity::Warning),
            (WarningSeverity::Info, Severity::Advice),
        ] {
            let w = sample_warning("AP-014", sev, None);
            assert!(
                matches!(WarningReport::new(&w).severity(), Some(s) if s == want),
                "severity {sev:?} should map to {want:?}"
            );
        }
    }

    #[test]
    fn warning_report_source_code_none_for_missing_file() {
        use miette::Diagnostic;
        let w = sample_warning("AP-015", WarningSeverity::Warning, None);
        // sample_warning uses "src/a.ts" which doesn't exist on disk.
        assert!(
            WarningReport::new(&w).source_code().is_none(),
            "missing file must not panic, source_code() must be None"
        );
    }

    #[test]
    fn warning_report_labels_none_for_missing_file() {
        use miette::Diagnostic;
        let w = sample_warning("AP-016", WarningSeverity::Warning, None);
        assert!(WarningReport::new(&w).labels().is_none());
    }

    #[test]
    fn compute_span_finds_correct_byte_offset() {
        let content = "line one\nline two\nline three\n";
        // Line 2, column 5 → "two"
        let loc = Location {
            file: String::new(),
            line: 2,
            column: Some(5),
            end_line: None,
            end_column: None,
        };
        let span = super::compute_span(content, &loc).expect("span must be computed");
        assert_eq!(span.offset(), 14, "byte offset of 'two' in line 2");
        // length = "two".len() = 3 (rest of line from column 5)
        assert_eq!(span.len(), 3);
    }

    #[test]
    fn compute_span_explicit_end_column() {
        let content = "hello world\n";
        let loc = Location {
            file: String::new(),
            line: 1,
            column: Some(6),
            end_line: Some(1),
            end_column: Some(11),
        };
        let span = super::compute_span(content, &loc).expect("span must be computed");
        assert_eq!(span.offset(), 6);
        assert_eq!(span.len(), 5); // "world"
    }

    #[test]
    fn compute_span_returns_none_for_line_zero() {
        let content = "foo\n";
        let loc = Location {
            file: String::new(),
            line: 0,
            column: None,
            end_line: None,
            end_column: None,
        };
        assert!(super::compute_span(content, &loc).is_none());
    }

    #[test]
    fn compute_span_returns_none_for_missing_column() {
        let content = "foo\nbar\n";
        let loc = Location {
            file: String::new(),
            line: 1,
            column: None,
            end_line: None,
            end_column: None,
        };
        assert!(super::compute_span(content, &loc).is_none());
    }

    #[test]
    fn compute_span_returns_none_for_column_past_end_of_line() {
        let content = "hi\n";
        let loc = Location {
            file: String::new(),
            line: 1,
            column: Some(100),
            end_line: None,
            end_column: None,
        };
        assert!(super::compute_span(content, &loc).is_none());
    }

    #[test]
    fn warning_report_source_none_when_column_is_none() {
        use miette::Diagnostic;
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "first line").unwrap();

        let mut w = sample_warning("AP-018", WarningSeverity::Warning, None);
        w.location.file = tmp.path().to_string_lossy().into_owned();
        w.location.line = 1;
        w.location.column = None;

        let report = WarningReport::new(&w);
        assert!(
            report.source_code().is_none(),
            "source must not load when column is unknown (prevents secret exposure)"
        );
        assert!(report.labels().is_none());
    }

    #[test]
    fn warning_report_source_code_and_labels_with_real_file() {
        use miette::Diagnostic;
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "first line").unwrap();
        writeln!(tmp, "second line with SECRET_KEY = \"abc123\"").unwrap();
        writeln!(tmp, "third line").unwrap();

        let mut w = sample_warning("AP-017", WarningSeverity::Error, None);
        w.location.file = tmp.path().to_string_lossy().into_owned();
        w.location.line = 2;
        w.location.column = Some(12); // points at 'w' in "with"

        let report = WarningReport::new(&w);
        assert!(
            report.source_code().is_some(),
            "source must load from real file"
        );
        let labels: Vec<_> = report.labels().expect("labels must be Some").collect();
        assert_eq!(labels.len(), 1);
        // Span starts at byte 12 within line 2 (line 1 = "first line\n" = 11 bytes)
        assert_eq!(labels[0].offset(), 11 + 12);
    }
}
