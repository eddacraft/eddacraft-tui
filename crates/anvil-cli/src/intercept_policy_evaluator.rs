//! ADR-149: CLI-backed [`PolicyEvaluator`] injected into the intercept daemon.
//!
//! The daemon crate defines the trait and never links `regorus`. This impl
//! reuses the MCP pre-write evaluator (including the OPAE-011 compiled-engine
//! cache) so save-time and pre-write see the same packs.

use std::path::Path;

use anvil_intercept::save_time::{PolicyEvalOutcome, PolicyEvaluator};
use anvil_kernel_types::diagnostics::KnownMode;
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};
use anvil_policy_engine::context::assertion::ChangeKind;

/// Stateless evaluator. Pack compile state lives in the process-local
/// pre-write cache.
#[derive(Debug, Default)]
pub struct CliPolicyEvaluator;

impl CliPolicyEvaluator {
    /// Construct the evaluator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl PolicyEvaluator for CliPolicyEvaluator {
    fn evaluate(&self, workspace_root: &Path, changed_paths: &[String]) -> PolicyEvalOutcome {
        if changed_paths.is_empty() {
            return PolicyEvalOutcome::inert();
        }
        let posture = crate::mcp::enforcement::load_for_workspace(workspace_root);
        let changes: Vec<(&str, ChangeKind)> = changed_paths
            .iter()
            .map(|path| (path.as_str(), ChangeKind::Modified))
            .collect();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::mcp::policy_prewrite::evaluate_many(workspace_root, &changes, posture)
        })) {
            Ok(outcome) => {
                if outcome.evaluated {
                    PolicyEvalOutcome::evaluated(outcome.diagnostics)
                } else {
                    PolicyEvalOutcome::inert()
                }
            }
            // Fail open with a visible warning: an empty list would still let
            // validate_paths claim CheckFamily::Policy and hide the failure.
            Err(_) => PolicyEvalOutcome::evaluated(vec![panic_warning_diagnostic(changed_paths)]),
        }
    }
}

fn panic_warning_diagnostic(changed_paths: &[String]) -> Diagnostic {
    let file = changed_paths
        .first()
        .cloned()
        .unwrap_or_else(|| ".".to_string());
    Diagnostic::new(
        "diag_policy_save_time_panicked",
        Severity::Warning,
        "Save-time policy evaluation panicked; failing open (the write is not blocked).",
        Location {
            file,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Policy,
        DiagnosticSource {
            rule_id: "policy-eval-panicked".to_string(),
            source_module: "anvil-cli::intercept_policy_evaluator".to_string(),
        },
        Mode::known(KnownMode::SaveTime),
    )
    .with_remediation_hint(
        "This is a policy-engine degradation, not a policy finding; the save is not blocked. \
         Check the installed packs for drift, or set \
         ANVIL_INTERCEPT_DISABLE_POLICY_EVALUATOR=1 (withhold the save-time hook) or \
         ANVIL_POLICY_ENFORCEMENT=off (bypass policy evaluation) while you recover.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_warning_anchors_first_changed_path() {
        let d = panic_warning_diagnostic(&["src/a.rs".into(), "src/b.rs".into()]);
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.category, Category::Policy);
        assert_eq!(d.location.file, "src/a.rs");
        assert_eq!(d.mode, Mode::known(KnownMode::SaveTime));
    }
}
