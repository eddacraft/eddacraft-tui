//! INTR-003: antipattern scanner as an `InterceptRule` (single-file, no disk I/O).

use anvil_checks::antipattern::{
    AntipatternCheckConfig, ScanOptions, Warning, WarningSeverity, scan_file,
};
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};

use crate::{ChangeKind, InterceptRule, RuleDecision, RuleInput, mode_id_part, sanitise_id_part};

pub const ANTIPATTERN_RULE_ID: &str = "antipattern-scan";

/// Hot-path rule that interrupts when changed file content matches an
/// anti-pattern at or above the configured severity threshold.
#[derive(Debug, Clone)]
pub struct AntipatternScanRule {
    config: AntipatternCheckConfig,
}

impl AntipatternScanRule {
    #[must_use]
    pub const fn new(config: AntipatternCheckConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub const fn config(&self) -> &AntipatternCheckConfig {
        &self.config
    }

    #[must_use]
    pub fn diagnostics(&self, input: &RuleInput<'_>, mode: &Mode) -> Vec<Diagnostic> {
        self.diagnostics_with_limit(input, mode, usize::MAX)
    }

    #[must_use]
    pub fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        self.findings_with_limit(input, limit)
            .into_iter()
            .map(|warning| warning_to_diagnostic(&warning, mode.clone()))
            .collect()
    }

    fn findings_with_limit(&self, input: &RuleInput<'_>, limit: usize) -> Vec<Warning> {
        if limit == 0 {
            return Vec::new();
        }
        if input.change_kind == ChangeKind::Removed {
            return Vec::new();
        }
        if !self.is_scannable_path(input) {
            return Vec::new();
        }
        let Some(content) = input.content else {
            return Vec::new();
        };
        let content = String::from_utf8_lossy(content);
        let path = input.path.to_string_lossy();
        let options = ScanOptions {
            patterns: if self.config.patterns.is_empty() {
                None
            } else {
                Some(self.config.patterns.clone())
            },
            include_opt_in: self.config.include_opt_in,
        };

        let threshold = severity_level(self.config.severity_threshold);
        scan_file(path.as_ref(), content.as_ref(), Some(&options))
            .warnings
            .into_iter()
            .filter(|warning| {
                warning.suppressed.is_none() && severity_level(warning.severity) >= threshold
            })
            .take(limit)
            .collect()
    }

    fn is_scannable_path(&self, input: &RuleInput<'_>) -> bool {
        let path = input.path.to_string_lossy();
        self.config
            .extensions
            .iter()
            .any(|extension| path.ends_with(extension))
    }
}

impl Default for AntipatternScanRule {
    fn default() -> Self {
        Self::new(AntipatternCheckConfig::default())
    }
}

impl InterceptRule for AntipatternScanRule {
    fn rule_id(&self) -> &str {
        ANTIPATTERN_RULE_ID
    }

    fn needs_content(&self) -> bool {
        true
    }

    fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision {
        let findings = self.findings_with_limit(input, 1);
        let Some(first) = findings.first() else {
            return RuleDecision::Allow;
        };

        let message = build_message(first);
        match u32::try_from(first.location.line) {
            Ok(line) if line > 0 => RuleDecision::interrupt_at(ANTIPATTERN_RULE_ID, message, line),
            _ => RuleDecision::interrupt(ANTIPATTERN_RULE_ID, message),
        }
    }

    fn diagnostics(&self, input: &RuleInput<'_>, mode: &Mode) -> Vec<Diagnostic> {
        AntipatternScanRule::diagnostics(self, input, mode)
    }

    fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        AntipatternScanRule::diagnostics_with_limit(self, input, mode, limit)
    }
}

/// Severity ordering used for the interrupt threshold. Mirrors the
/// private `severity_level` in `anvil-checks::antipattern::check` so the
/// wrapper's interrupt semantics match the check's pass/fail semantics.
const fn severity_level(severity: WarningSeverity) -> usize {
    match severity {
        WarningSeverity::Error => 3,
        WarningSeverity::Warning => 2,
        WarningSeverity::Info => 1,
    }
}

const fn map_severity(severity: WarningSeverity) -> Severity {
    match severity {
        WarningSeverity::Error => Severity::Error,
        WarningSeverity::Warning => Severity::Warning,
        WarningSeverity::Info => Severity::Info,
    }
}

fn build_message(warning: &Warning) -> String {
    let pattern_id = warning.pattern.as_deref().unwrap_or(&warning.id);
    format!("Anti-pattern detected ({}: {})", pattern_id, warning.title)
}

fn warning_to_diagnostic(warning: &Warning, mode: Mode) -> Diagnostic {
    let pattern_id = warning
        .pattern
        .clone()
        .unwrap_or_else(|| warning.id.clone());
    let mut diagnostic = Diagnostic::new(
        format!(
            "diag_antipattern_{}_{}_{}_{}",
            mode_id_part(&mode),
            sanitise_id_part(&warning.location.file),
            warning.location.line,
            sanitise_id_part(&pattern_id)
        ),
        map_severity(warning.severity),
        build_message(warning),
        Location {
            file: warning.location.file.clone(),
            line: u32::try_from(warning.location.line).ok().filter(|l| *l > 0),
            // The scanner's column is a 0-based byte offset
            // (`regex::Match::start()`); the canonical envelope expects
            // 1-based columns.
            column: warning
                .location
                .column
                .and_then(|c| u32::try_from(c.saturating_add(1)).ok()),
            end_line: None,
            end_column: None,
        },
        Category::Antipattern,
        DiagnosticSource {
            rule_id: ANTIPATTERN_RULE_ID.to_string(),
            source_module: "anvil-checks::antipattern".to_string(),
        },
        mode,
    );
    if !warning.suggestion.is_empty() {
        diagnostic = diagnostic.with_remediation_hint(warning.suggestion.clone());
    }
    diagnostic
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anvil_checks::antipattern::WarningSeverity;
    use anvil_kernel_types::{Category, Mode, Severity};

    use super::*;
    use crate::{ChangeKind, InterceptRule, RuleInput};

    /// AP-008 (dynamic-execution `eval`) is an error-severity,
    /// enabled-by-default registry pattern, so it interrupts under the
    /// default `Error` threshold. The path must avoid the pattern's
    /// allowlist (`**/*.test.ts`, `**/__tests__/**`).
    const EVAL_FIXTURE: &[u8] = b"const result = eval(userInput);\n";

    fn input<'a>(path: &'a Path, content: Option<&'a [u8]>) -> RuleInput<'a> {
        RuleInput {
            path,
            change_kind: ChangeKind::Modified,
            content,
        }
    }

    #[test]
    fn antipattern_rule_interrupts_on_error_severity_finding() {
        let path = Path::new("src/dynamic.ts");

        let decision = AntipatternScanRule::default().evaluate(&input(path, Some(EVAL_FIXTURE)));

        match decision {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, ANTIPATTERN_RULE_ID);
                assert_eq!(reason.line, std::num::NonZeroU32::new(1));
                assert!(
                    reason.message.contains("AP-008"),
                    "message must carry the antipattern id: {}",
                    reason.message
                );
            }
            RuleDecision::Allow => panic!("eval fixture should interrupt"),
        }
    }

    #[test]
    fn antipattern_rule_allows_clean_content() {
        let path = Path::new("src/clean.ts");
        let body = b"const value = 1;\n";

        let decision = AntipatternScanRule::default().evaluate(&input(path, Some(body)));

        assert_eq!(decision, RuleDecision::Allow);
    }

    #[test]
    fn antipattern_rule_allows_findings_below_severity_threshold() {
        // `: any` is a warning-severity finding; the default threshold is
        // Error, so the write is allowed — mirroring
        // `run_antipattern_check`'s default pass-with-issues semantics.
        let path = Path::new("src/loose.ts");
        let body = b"const value: any = source;\n";

        let rule = AntipatternScanRule::default();
        assert_eq!(rule.evaluate(&input(path, Some(body))), RuleDecision::Allow);

        // Lowering the threshold makes the same finding interrupt.
        let strict = AntipatternScanRule::new(AntipatternCheckConfig {
            severity_threshold: WarningSeverity::Warning,
            ..AntipatternCheckConfig::default()
        });
        assert!(matches!(
            strict.evaluate(&input(path, Some(body))),
            RuleDecision::Interrupt(_)
        ));
    }

    /// Inline-suppressed findings do not interrupt — the ADR-029
    /// suppression contract flows through `scan_file` unchanged.
    #[test]
    fn antipattern_rule_allows_suppressed_findings() {
        let path = Path::new("src/dynamic.ts");
        let body = b"// @anvil-ignore AP-008 -- council-approved fixture\nconst result = eval(userInput);\n";

        let decision = AntipatternScanRule::default().evaluate(&input(path, Some(body)));

        assert_eq!(decision, RuleDecision::Allow);
    }

    #[test]
    fn antipattern_rule_allows_removed_changes() {
        let path = Path::new("src/dynamic.ts");
        let removed = RuleInput {
            path,
            change_kind: ChangeKind::Removed,
            content: Some(EVAL_FIXTURE),
        };

        assert_eq!(
            AntipatternScanRule::default().evaluate(&removed),
            RuleDecision::Allow
        );
    }

    #[test]
    fn antipattern_rule_allows_missing_or_binary_content() {
        let path = Path::new("src/dynamic.ts");
        let rule = AntipatternScanRule::default();

        assert_eq!(rule.evaluate(&input(path, None)), RuleDecision::Allow);
        assert_eq!(
            rule.evaluate(&input(path, Some(b"\xff\xfe\x00"))),
            RuleDecision::Allow
        );
    }

    #[test]
    fn antipattern_rule_skips_non_scannable_extensions() {
        let path = Path::new("notes/dynamic.md");

        let decision = AntipatternScanRule::default().evaluate(&input(path, Some(EVAL_FIXTURE)));

        assert_eq!(decision, RuleDecision::Allow);
    }

    #[test]
    fn antipattern_rule_maps_findings_to_canonical_diagnostics() {
        let path = Path::new("src/dynamic.ts");

        let diagnostics = AntipatternScanRule::default().diagnostics(
            &input(path, Some(EVAL_FIXTURE)),
            &Mode::Unknown("pre-write".to_string()),
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.schema_version, "anvil.diagnostic.v1");
        assert_eq!(
            diagnostic.id,
            "diag_antipattern_pre_write_src_dynamic_ts_1_ap_008"
        );
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.category, Category::Antipattern);
        assert_eq!(diagnostic.location.file, "src/dynamic.ts");
        assert_eq!(diagnostic.location.line, Some(1));
        assert_eq!(diagnostic.mode, Mode::Unknown("pre-write".to_string()));
        assert_eq!(diagnostic.source.rule_id, ANTIPATTERN_RULE_ID);
        assert_eq!(diagnostic.source.source_module, "anvil-checks::antipattern");
        assert!(
            diagnostic.remediation_hint.is_some(),
            "registry suggestion should flow through as the remediation hint"
        );
    }

    #[test]
    fn antipattern_rule_diagnostics_respect_limit() {
        let path = Path::new("src/dynamic.ts");
        let rule = AntipatternScanRule::default();
        let mode = Mode::Unknown("pre-write".to_string());

        assert!(
            rule.diagnostics_with_limit(&input(path, Some(EVAL_FIXTURE)), &mode, 0)
                .is_empty()
        );
    }

    #[test]
    fn antipattern_rule_is_registry_composable() {
        let rule: Box<dyn InterceptRule> = Box::new(AntipatternScanRule::default());
        assert_eq!(rule.rule_id(), ANTIPATTERN_RULE_ID);
        assert!(rule.needs_content());
    }
}
