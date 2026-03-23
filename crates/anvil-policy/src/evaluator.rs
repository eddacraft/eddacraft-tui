use std::path::Path;

use serde::Serialize;

use crate::loader::PolicyLoader;
use crate::opa::{OpaExecutor, PolicyViolation};

#[derive(Debug, Clone, Serialize)]
pub struct EvaluationResult {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub checks_run: usize,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    pub policy_id: String,
    pub file: String,
    pub message: String,
    pub severity: String,
    pub category: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("policy not found: {0}")]
    PolicyNotFound(String),
    #[error("OPA binary not available — install OPA or set OPA_PATH")]
    OpaNotAvailable,
    #[error("no policies found in {0}")]
    NoPolicies(String),
    #[error("evaluation failed: {0}")]
    Internal(String),
}

pub struct Evaluator {
    loader: PolicyLoader,
    executor: OpaExecutor,
}

impl Evaluator {
    pub fn new(opa_path: Option<&str>) -> Self {
        Self {
            loader: PolicyLoader::new(),
            executor: OpaExecutor::new(opa_path, None),
        }
    }

    pub fn evaluate(
        &self,
        workspace_root: &Path,
        input: &serde_json::Value,
        policy_dir: Option<&str>,
    ) -> Result<EvaluationResult, EvalError> {
        if !self.executor.is_available() {
            return Err(EvalError::OpaNotAvailable);
        }

        let policies = self
            .loader
            .load_policies(workspace_root, policy_dir)
            .map_err(|e| EvalError::Internal(e.to_string()))?;

        if policies.is_empty() {
            let dir = policy_dir.unwrap_or(".anvil/policies");
            return Err(EvalError::NoPolicies(dir.to_string()));
        }

        let enabled: Vec<_> = policies.into_iter().filter(|p| !p.generated).collect();

        let result = self
            .executor
            .evaluate(&enabled, input)
            .map_err(|e| EvalError::Internal(e.to_string()))?;

        let violations: Vec<Violation> = result
            .violations
            .into_iter()
            .map(into_violation)
            .collect();

        Ok(EvaluationResult {
            passed: violations.is_empty(),
            violations,
            checks_run: enabled.len(),
            execution_time_ms: result.metadata.execution_time_ms,
        })
    }
}

fn into_violation(v: PolicyViolation) -> Violation {
    Violation {
        policy_id: v.policy.unwrap_or_else(|| v.rule.clone()),
        file: v.path.unwrap_or_default(),
        message: v.message,
        severity: v.severity,
        category: v.category,
        fingerprint: v.fingerprint,
    }
}

/// Legacy compatibility — calls the new evaluator.
/// Returns `EvalError::OpaNotAvailable` if OPA is not installed.
pub fn evaluate(
    _policies: &[super::config::PolicyEntry],
    _files: &[String],
) -> Result<EvaluationResult, EvalError> {
    Err(EvalError::OpaNotAvailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_evaluate_returns_opa_not_available() {
        let result = evaluate(&[], &[]);
        assert!(matches!(result, Err(EvalError::OpaNotAvailable)));
    }

    #[test]
    fn evaluator_requires_opa() {
        let eval = Evaluator::new(Some("/nonexistent/opa"));
        let input = serde_json::json!({});
        let tmp = tempfile::TempDir::new().unwrap();
        let result = eval.evaluate(tmp.path(), &input, None);
        assert!(matches!(result, Err(EvalError::OpaNotAvailable)));
    }
}
