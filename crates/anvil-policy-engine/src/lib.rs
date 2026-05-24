//! Anvil policy engine facade — over `regorus` (ADR-040).
//!
//! Downstream crates depend on this facade, never on `regorus` directly,
//! so the engine choice stays swappable without a fan-out refactor.
//! The `PolicyInput` v1 schema (POLENG-002) lives in [`input`]; the
//! determinism contract and the [`Builtin`] trait (POLENG-004) live in
//! [`determinism`]. Concrete builtins, post-processing, coverage/trace,
//! CLI, and the bench harness land in POLENG-003 and POLENG-005..-008.

use std::sync::{Arc, RwLock};

use regorus::{Engine as RegorusEngine, Value as RegorusValue};
use thiserror::Error;

pub mod builtins;
pub mod coverage;
pub mod determinism;
pub mod input;
pub mod result;
pub mod trace;

pub use coverage::{Coverage, FileCoverage};
pub use determinism::{Builtin, BuiltinError, DeterminismClass};
pub use input::PolicyInput;
pub use result::{EvalReport, Finding, PostProcessOptions, ResultError, Severity};
pub use trace::Trace;

/// Configuration for an [`Engine`].
///
/// Construct with struct-update syntax over [`Default`] so call sites stay
/// source-compatible as fields are added:
/// `EngineConfig { collect_coverage: true, ..Default::default() }`.
#[derive(Debug, Clone, Default)]
pub struct EngineConfig {
    /// Allow registering [`DeterminismClass::Impure`] builtins. Off by
    /// default: an impure builtin forfeits the determinism guarantee
    /// (POLENG-004), so opting in is an explicit, auditable choice.
    pub allow_impure_builtins: bool,
    /// Collect line coverage and expose it on [`EvalResult::coverage`]. Off by
    /// default (it adds per-eval bookkeeping); the CLI's `--explain` enables it.
    pub collect_coverage: bool,
    /// Collect the evaluation trace and expose it on [`EvalResult::trace`]. Off
    /// by default; the CLI's `--why` enables it.
    pub collect_trace: bool,
}

/// Result of a single evaluation.
///
/// `value` is `None` when the Rego query produced no result — Rego's
/// `undefined` outcome, which is semantically distinct from a query
/// that resolved to JSON `null`. Callers MUST treat these cases
/// separately (e.g. an unknown rule reference vs. a rule that returned
/// `null` explicitly).
///
/// `value` is the raw query result and is always meaningful, so it is a public
/// field. `coverage` and `trace` are opt-in observability — populated only when
/// the corresponding [`EngineConfig`] flag is set — so they are exposed via the
/// [`EvalResult::coverage`] / [`EvalResult::trace`] accessors that return
/// `Option<&_>`, keeping the "may be absent" contract in the type.
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub value: Option<serde_json::Value>,
    coverage: Option<Coverage>,
    trace: Option<Trace>,
}

impl EvalResult {
    /// Line coverage for this evaluation, when [`EngineConfig::collect_coverage`]
    /// was set.
    pub fn coverage(&self) -> Option<&Coverage> {
        self.coverage.as_ref()
    }

    /// Evaluation trace, when [`EngineConfig::collect_trace`] was set.
    pub fn trace(&self) -> Option<&Trace> {
        self.trace.as_ref()
    }
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
    #[error("result post-processing failed: {0}")]
    PostProcess(#[from] ResultError),
}

pub struct Engine {
    inner: RegorusEngine,
    config: EngineConfig,
    /// The input document for the current evaluation, shared with builtin
    /// closures so a data-source builtin (e.g. `anvil.repo_state()`) can read
    /// it. Refreshed at the start of every [`Engine::eval`].
    context: Arc<RwLock<PolicyInput>>,
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
            context: Arc::new(RwLock::new(PolicyInput::default())),
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
    pub fn register_builtin(&mut self, builtin: Arc<dyn Builtin>) -> Result<(), EngineError> {
        let name = builtin.name().to_string();
        let arity = builtin.arity();
        if builtin.determinism() == DeterminismClass::Impure && !self.config.allow_impure_builtins {
            return Err(EngineError::ImpureBuiltinRejected(name));
        }
        // `builtin` is moved into the extension closure, which `regorus` keeps
        // for the engine's lifetime; the closure reads the shared input
        // context and bridges `regorus::Value` to/from `serde_json::Value` so
        // the builtin never touches `regorus`.
        let context = Arc::clone(&self.context);
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
                        let input = context
                            .read()
                            .map_err(|_| anyhow::Error::msg("policy input lock poisoned"))?;
                        let out = builtin
                            .call(&input, &json_args)
                            .map_err(|e| anyhow::Error::msg(e.to_string()))?;
                        // `RegorusValue::from(serde_json::Value)` silently maps
                        // a non-representable value to `Undefined`; convert
                        // explicitly so a builtin bug surfaces as an error
                        // instead of a phantom `undefined` in the policy.
                        serde_json::from_value::<RegorusValue>(out).map_err(|e| {
                            anyhow::Error::msg(format!(
                                "builtin `{}` produced a value regorus cannot represent: {e}",
                                builtin.name()
                            ))
                        })
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

        // Refresh the shared context builtins read from. Cloning keeps the
        // borrow self-contained; `input` documents are small policy fixtures.
        *self
            .context
            .write()
            .map_err(|_| EngineError::Input("policy input lock poisoned".into()))? = input.clone();

        if self.config.collect_coverage {
            self.inner.set_enable_coverage(true);
            // Reset so coverage reflects only this evaluation.
            self.inner.clear_coverage_data();
        }

        let results = self
            .inner
            .eval_query(query.to_string(), self.config.collect_trace)
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

        let coverage = if self.config.collect_coverage {
            let report = self
                .inner
                .get_coverage_report()
                .map_err(|e| EngineError::Regorus(e.to_string()))?;
            Some(Coverage::from_regorus(&report))
        } else {
            None
        };
        let trace = self
            .config
            .collect_trace
            .then(|| Trace::from_results(&results));

        Ok(EvalResult {
            value,
            coverage,
            trace,
        })
    }

    /// Evaluate a findings query and apply ADR-002 / ADR-003 post-processing
    /// (POLENG-005): annotate each finding with `is_new_edge` / `baselined` and
    /// compute the process exit code. `query` should resolve to an array of
    /// finding objects (or be absent for "no findings").
    ///
    /// This is a convenience that discards the [`EvalResult`] — so any
    /// collected coverage/trace is lost. If you need findings *and*
    /// coverage/trace, call [`Engine::eval`] and pass `EvalResult::value` to
    /// [`result::post_process`] yourself (this is what the CLI does).
    pub fn evaluate_findings(
        &mut self,
        input: &PolicyInput,
        query: &str,
        opts: PostProcessOptions,
    ) -> Result<EvalReport, EngineError> {
        let raw = self
            .eval(input, query)?
            .value
            .unwrap_or(serde_json::Value::Null);
        Ok(result::post_process(&raw, input, opts)?)
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
