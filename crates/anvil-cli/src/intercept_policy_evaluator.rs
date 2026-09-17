//! ADR-149: CLI-backed [`PolicyEvaluator`] injected into the intercept daemon.
//!
//! The daemon crate defines the trait and never links `regorus`. This impl
//! reuses the MCP pre-write evaluator (including the OPAE-011 compiled-engine
//! cache) so save-time and pre-write see the same packs.

use std::path::Path;

use anvil_intercept::save_time::PolicyEvaluator;
use anvil_kernel_types::Diagnostic;
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
    fn evaluate(&self, workspace_root: &Path, changed_paths: &[String]) -> Vec<Diagnostic> {
        if changed_paths.is_empty() {
            return Vec::new();
        }
        let posture = crate::mcp::enforcement::load_for_workspace(workspace_root);
        let changes: Vec<(&str, ChangeKind)> = changed_paths
            .iter()
            .map(|path| (path.as_str(), ChangeKind::Modified))
            .collect();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::mcp::policy_prewrite::evaluate_many(workspace_root, &changes, posture)
        })) {
            Ok(outcome) => outcome.diagnostics,
            Err(_) => Vec::new(),
        }
    }
}
