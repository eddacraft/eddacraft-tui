//! Anvil policy engine facade — over `regorus` (ADR-040).
//!
//! Downstream crates depend on this facade, never on `regorus` directly,
//! so the engine choice stays swappable without a fan-out refactor.
//! The `PolicyInput` v1 schema (POLENG-002) lives in [`input`]; the
//! determinism contract and the [`Builtin`] trait (POLENG-004) live in
//! [`determinism`]. Concrete builtins, post-processing, coverage/trace,
//! CLI, and the bench harness land in POLENG-003 and POLENG-005..-008.

use regorus::{Engine as RegorusEngine, Value as RegorusValue};
use thiserror::Error;

pub mod determinism;
pub mod input;

pub use determinism::{Builtin, BuiltinError, DeterminismClass};
pub use input::PolicyInput;

/// Configuration for an [`Engine`].
#[derive(Debug, Clone, Default)]
pub struct EngineConfig {
    /// Allow registering [`DeterminismClass::Impure`] builtins. Off by
    /// default: an impure builtin forfeits the determinism guarantee
    /// (POLENG-004), so opting in is an explicit, auditable choice.
    pub allow_impure_builtins: bool,
}

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
    #[error(
        "refused to register impure builtin `{0}`; set EngineConfig::allow_impure_builtins to opt in"
    )]
    ImpureBuiltinRejected(String),
}

pub struct Engine {
    inner: RegorusEngine,
    config: EngineConfig,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        Ok(Self {
            inner: RegorusEngine::new(),
            config,
        })
    }

    /// Register a first-party [`Builtin`] under its Rego call path.
    ///
    /// Enforces the determinism contract (POLENG-004): a
    /// [`DeterminismClass::Impure`] builtin is rejected with
    /// [`EngineError::ImpureBuiltinRejected`] unless the engine was built with
    /// [`EngineConfig::allow_impure_builtins`]. Builtins speak
    /// [`serde_json::Value`]; the engine bridges to and from `regorus::Value`
    /// at the call boundary so implementors never depend on `regorus`.
    pub fn register_builtin(
        &mut self,
        builtin: std::sync::Arc<dyn Builtin>,
    ) -> Result<(), EngineError> {
        let name = builtin.name().to_string();
        let arity = builtin.arity();
        if builtin.determinism() == DeterminismClass::Impure && !self.config.allow_impure_builtins {
            return Err(EngineError::ImpureBuiltinRejected(name));
        }
        // `builtin` is moved into the extension closure, which `regorus` keeps
        // for the engine's lifetime; the closure bridges `regorus::Value` to
        // and from `serde_json::Value` so the builtin never touches `regorus`.
        self.inner
            .add_extension(
                name,
                arity,
                Box::new(
                    move |args: Vec<RegorusValue>| -> anyhow::Result<RegorusValue> {
                        let json_args: Vec<serde_json::Value> = args
                            .iter()
                            .map(serde_json::to_value)
                            .collect::<Result<_, _>>()?;
                        let out = builtin
                            .call(&json_args)
                            .map_err(|e| anyhow::Error::msg(e.to_string()))?;
                        Ok(RegorusValue::from(out))
                    },
                ),
            )
            .map_err(|e| EngineError::Regorus(e.to_string()))?;
        Ok(())
    }

    /// Register a Rego policy module with the engine.
    ///
    /// The loading model (explicit register vs. path-based discovery) is still
    /// provisional and may gain variants when the CLI surface lands
    /// (POLENG-007); the existing signature stays source-compatible.
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
