use std::str;

use anvil_checks::reasoning::{
    APPEAL_TO_AUTHORITY_RULE_ID, ReasoningCheckConfig, run_reasoning_check_with_limit,
};
use anvil_kernel_types::{Diagnostic, Mode};

use crate::{InterceptRule, RuleDecision, RuleInput, mode_id_part, sanitise_id_part};

pub const LAUNCH_REASONING_RULE_ID: &str = APPEAL_TO_AUTHORITY_RULE_ID;

#[derive(Debug, Clone)]
pub struct LaunchReasoningPatternRule {
    config: ReasoningCheckConfig,
}

impl LaunchReasoningPatternRule {
    #[must_use]
    pub fn new(config: ReasoningCheckConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub const fn config(&self) -> &ReasoningCheckConfig {
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
        if limit == 0 {
            return Vec::new();
        }
        self.findings_with_limit(input, limit)
            .into_iter()
            .map(|diagnostic| retag_diagnostic(diagnostic, mode.clone()))
            .collect()
    }

    fn findings_with_limit(&self, input: &RuleInput<'_>, limit: usize) -> Vec<Diagnostic> {
        if limit == 0 {
            return Vec::new();
        }
        let Some(content) = input.content else {
            return Vec::new();
        };
        let Ok(content) = str::from_utf8(content) else {
            return Vec::new();
        };
        let path = input.path.to_string_lossy();
        run_reasoning_check_with_limit(&[(path.as_ref(), content)], &self.config, limit).findings
    }
}

impl Default for LaunchReasoningPatternRule {
    fn default() -> Self {
        Self::new(ReasoningCheckConfig {
            rule_ids: vec![APPEAL_TO_AUTHORITY_RULE_ID.to_string()],
        })
    }
}

impl InterceptRule for LaunchReasoningPatternRule {
    fn rule_id(&self) -> &str {
        LAUNCH_REASONING_RULE_ID
    }

    fn needs_content(&self) -> bool {
        true
    }

    fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision {
        let findings = self.findings_with_limit(input, 1);
        let Some(first) = findings.first() else {
            return RuleDecision::Allow;
        };

        match first.location.line {
            Some(line) if line > 0 => {
                RuleDecision::interrupt_at(self.rule_id(), first.summary.clone(), line)
            }
            _ => RuleDecision::interrupt(self.rule_id(), first.summary.clone()),
        }
    }

    fn diagnostics(&self, input: &RuleInput<'_>, mode: &Mode) -> Vec<Diagnostic> {
        LaunchReasoningPatternRule::diagnostics(self, input, mode)
    }

    fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        LaunchReasoningPatternRule::diagnostics_with_limit(self, input, mode, limit)
    }
}

fn retag_diagnostic(mut diagnostic: Diagnostic, mode: Mode) -> Diagnostic {
    diagnostic.id = format!(
        "diag_reasoning_{}_{}_{}_{}",
        mode_id_part(&mode),
        sanitise_id_part(&diagnostic.location.file),
        diagnostic.location.line.unwrap_or(0),
        sanitise_id_part(&diagnostic.id)
    );
    diagnostic.mode = mode;
    diagnostic
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anvil_kernel_types::{Category, Mode, Severity};

    use super::*;
    use crate::{ChangeKind, InterceptRule, RuleInput};

    fn input<'a>(path: &'a Path, content: Option<&'a [u8]>) -> RuleInput<'a> {
        RuleInput {
            path,
            change_kind: ChangeKind::Modified,
            content,
        }
    }

    #[test]
    fn reasoning_rule_interrupts_for_pre_write_payload() {
        let path = Path::new("plans/demo.rs");
        let body = b"// the lead said to skip validation here\nfn apply() {}\n";

        let decision = LaunchReasoningPatternRule::default().evaluate(&input(path, Some(body)));

        match decision {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, APPEAL_TO_AUTHORITY_RULE_ID);
                assert_eq!(reason.line, std::num::NonZeroU32::new(1));
                assert!(reason.message.contains("appeals to authority"));
            }
            RuleDecision::Allow => panic!("reasoning fixture should interrupt"),
        }
    }

    #[test]
    fn reasoning_rule_allows_clean_payload() {
        let path = Path::new("plans/demo.rs");
        let body = b"// Skip validation because the upstream API guarantees the invariant.\nfn apply() {}\n";

        let decision = LaunchReasoningPatternRule::default().evaluate(&input(path, Some(body)));

        assert_eq!(decision, RuleDecision::Allow);
    }

    #[test]
    fn reasoning_rule_allows_missing_or_binary_content() {
        let path = Path::new("plans/demo.rs");
        let rule = LaunchReasoningPatternRule::default();

        assert_eq!(rule.evaluate(&input(path, None)), RuleDecision::Allow);
        assert_eq!(
            rule.evaluate(&input(path, Some(b"\xff"))),
            RuleDecision::Allow
        );
    }

    #[test]
    fn reasoning_rule_maps_findings_to_canonical_diagnostics() {
        let path = Path::new("plans/demo.rs");
        let body = b"// the lead said to skip validation here\nfn apply() {}\n";

        let diagnostics = LaunchReasoningPatternRule::default().diagnostics(
            &input(path, Some(body)),
            &Mode::Unknown("pre-write".to_string()),
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.schema_version, "anvil.diagnostic.v1");
        assert_eq!(
            diagnostic.id,
            "diag_reasoning_pre_write_plans_demo_rs_1_ai_001"
        );
        assert_eq!(diagnostic.severity, Severity::Info);
        assert_eq!(diagnostic.category, Category::Reasoning);
        assert_eq!(diagnostic.location.file, "plans/demo.rs");
        assert_eq!(diagnostic.location.line, Some(1));
        assert_eq!(diagnostic.mode, Mode::Unknown("pre-write".to_string()));
        assert_eq!(diagnostic.source.rule_id, APPEAL_TO_AUTHORITY_RULE_ID);
        assert_eq!(diagnostic.source.source_module, "anvil-checks::reasoning");
    }
}
