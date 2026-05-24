//! Anvil policy engine facade — POLENG-001 skeleton (ADR-040).
//!
//! Downstream crates depend on this facade, never on `regorus` directly,
//! so the engine choice stays swappable without a fan-out refactor.
//! Builtins, determinism, post-processing, coverage/trace, CLI, and bench
//! harness land in POLENG-003..-008; the `PolicyInput` v1 schema (POLENG-002)
//! lives in [`input`].

use regorus::{Engine as RegorusEngine, Value as RegorusValue};
use thiserror::Error;

pub mod input;

pub use input::PolicyInput;

/// Configuration for an [`Engine`]. Empty in the skeleton; populated by
/// later POLENG tasks (determinism opt-ins, builtin allow-list, etc.).
#[derive(Debug, Clone, Default)]
pub struct EngineConfig {}

/// Result of a single evaluation.
///
/// `value` is `None` when the Rego query produced no result — Rego's
/// `undefined` outcome, which is semantically distinct from a query
/// that resolved to JSON `null`. Callers MUST treat these cases
/// separately (e.g. an unknown rule reference vs. a rule that returned
/// `null` explicitly).
///
/// Coverage and trace become first-class fields in POLENG-006; result
/// post-processing (severity, new-edge annotation) lands in POLENG-005.
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("regorus error: {0}")]
    Regorus(String),
    #[error("invalid policy input: {0}")]
    Input(String),
}

pub struct Engine {
    inner: RegorusEngine,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}

impl Engine {
    pub fn new(_config: EngineConfig) -> Result<Self, EngineError> {
        Ok(Self {
            inner: RegorusEngine::new(),
        })
    }

    /// Register a Rego policy module with the engine.
    ///
    /// Loading model is provisional — POLENG-002 fixes the public shape
    /// (path-based discovery vs. explicit register). Treat this signature
    /// as unstable until POLENG-002 lands.
    pub fn add_policy(
        &mut self,
        path: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<(), EngineError> {
        self.inner
            .add_policy(path.into(), source.into())
            .map_err(|e| EngineError::Regorus(e.to_string()))?;
        Ok(())
    }

    /// Evaluate a Rego query against the loaded policies and the given input.
    pub fn eval(&mut self, input: &PolicyInput, query: &str) -> Result<EvalResult, EngineError> {
        let input_json =
            serde_json::to_string(input).map_err(|e| EngineError::Input(e.to_string()))?;
        self.inner
            .set_input_json(&input_json)
            .map_err(|e| EngineError::Input(e.to_string()))?;

        let results = self
            .inner
            .eval_query(query.to_string(), false)
            .map_err(|e| EngineError::Regorus(e.to_string()))?;

        let value = match results.result.first().and_then(|qr| qr.expressions.first()) {
            // Preserve the Rego undefined vs. null distinction: Rego
            // `undefined` (no expression result, or `Value::Undefined`)
            // collapses to `None`; an explicit JSON `null` stays `Some(Null)`.
            None => None,
            Some(expr) if matches!(expr.value, RegorusValue::Undefined) => None,
            Some(expr) => Some(
                serde_json::to_value(&expr.value)
                    .map_err(|e| EngineError::Regorus(e.to_string()))?,
            ),
        };

        Ok(EvalResult { value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_new_constructs_with_default_config() {
        let engine = Engine::new(EngineConfig::default());
        assert!(engine.is_ok());
    }

    #[test]
    fn eval_round_trips_hello_world_policy() {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");

        engine
            .add_policy(
                "hello.rego",
                r#"package hello
import rego.v1

greeting := "hello world"
"#,
            )
            .expect("add_policy");

        let input = PolicyInput::default();
        let result = engine.eval(&input, "data.hello.greeting").expect("eval");

        assert_eq!(
            result.value,
            Some(serde_json::Value::String("hello world".into()))
        );
    }

    #[test]
    fn eval_distinguishes_undefined_from_null() {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");

        engine
            .add_policy(
                "shapes.rego",
                r"package shapes
import rego.v1

explicit_null := null
",
            )
            .expect("add_policy");

        let input = PolicyInput::default();

        // Undefined: querying a rule that does not exist → None.
        let undefined = engine
            .eval(&input, "data.shapes.no_such_rule")
            .expect("eval undefined");
        assert_eq!(undefined.value, None);

        // Explicit null: a rule that returns null → Some(Null).
        let null = engine
            .eval(&input, "data.shapes.explicit_null")
            .expect("eval null");
        assert_eq!(null.value, Some(serde_json::Value::Null));
    }
}
