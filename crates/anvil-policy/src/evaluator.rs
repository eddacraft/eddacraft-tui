use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EvaluationResult {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub checks_run: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    pub policy_id: String,
    pub file: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("policy not found: {0}")]
    PolicyNotFound(String),
    #[error("evaluation failed: {0}")]
    Internal(String),
}

pub fn evaluate(
    _policies: &[super::config::PolicyEntry],
    _files: &[String],
) -> Result<EvaluationResult, EvalError> {
    Ok(EvaluationResult {
        passed: true,
        violations: Vec::new(),
        checks_run: 0,
    })
}
