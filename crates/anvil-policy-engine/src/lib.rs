//! Anvil policy engine facade — over `regorus` (ADR-040).
//!
//! Downstream crates depend on this facade, never on `regorus` directly,
//! so the engine choice stays swappable without a fan-out refactor.
//! The `PolicyInput` v1 schema (POLENG-002) lives in [`input`]; the
//! determinism contract and the [`Builtin`] trait (POLENG-004) live in
//! [`determinism`]. Concrete builtins, post-processing, coverage/trace,
//! CLI, and the bench harness land in POLENG-003 and POLENG-005..-008.

use std::num::NonZeroU32;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use regorus::utils::limits::ExecutionTimerConfig;
use regorus::{Engine as RegorusEngine, Value as RegorusValue};
use thiserror::Error;

/// Default wall-clock ceiling on a single evaluation (POLENG-009). Generous
/// enough that real policies never hit it, tight enough that a pathological one
/// can't hang the process. `EngineConfig::eval_timeout = None` disables it.
const DEFAULT_EVAL_TIMEOUT: Duration = Duration::from_secs(10);

pub mod builtins;
pub mod context;
pub mod coverage;
pub mod determinism;
pub mod input;
pub mod io_risk;
pub mod pack;
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
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Allow registering [`DeterminismClass::Impure`] builtins, and skip the
    /// determinism fence on the impure Rego stdlib (POLENG-009: the `rand.intn`
    /// shadow). Off by default: opting in forfeits the determinism guarantee,
    /// so it is an explicit, auditable choice.
    pub allow_impure_builtins: bool,
    /// Collect line coverage and expose it via [`EvalResult::coverage`]. Off by
    /// default (it adds per-eval bookkeeping); the CLI's `--explain` enables it.
    pub collect_coverage: bool,
    /// Collect the evaluation trace and expose it via [`EvalResult::trace`]. Off
    /// by default; the CLI's `--why` enables it.
    pub collect_trace: bool,
    /// Wall-clock ceiling on a single [`Engine::eval`] (POLENG-009). `None`
    /// disables the limit; the [`Default`] is 10 seconds, so the engine is
    /// bounded by default.
    pub eval_timeout: Option<Duration>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            allow_impure_builtins: false,
            collect_coverage: false,
            collect_trace: false,
            eval_timeout: Some(DEFAULT_EVAL_TIMEOUT),
        }
    }
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

/// Best-effort message from a `catch_unwind` payload. `panic!` carries a
/// `&'static str` (literal) or a `String` (formatted); anything else is opaque.
fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "opaque panic payload".to_string()
    }
}

pub struct Engine {
    inner: RegorusEngine,
    config: EngineConfig,
    /// The input document for the current evaluation, shared with builtin
    /// closures so a data-source builtin (e.g. `anvil.repo_state()`) can read
    /// it. Refreshed at the start of every [`Engine::eval`].
    context: Arc<RwLock<PolicyInput>>,
    /// Set once a regorus call panics under [`Engine::guard`] (CIB-018).
    /// regorus 0.10 is a single-vendor crate with internal `unwrap`/`expect`;
    /// a panic mid-evaluation may leave its state inconsistent, so once one is
    /// caught the engine refuses further calls rather than risk acting on it.
    poisoned: bool,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        let mut inner = RegorusEngine::new();

        // POLENG-009 resource bound: cap wall-clock eval time so a pathological
        // policy can't hang the process. `None` disables the limit.
        if let Some(limit) = config.eval_timeout {
            let check_interval = NonZeroU32::new(1000).expect("non-zero check interval");
            inner.set_execution_timer_config(ExecutionTimerConfig {
                limit,
                check_interval,
            });
        }

        // POLENG-009 determinism fence. The impure stdlib builtin *groups*
        // (time/uuid/http/net/opa-runtime) are removed via the crate's regorus
        // feature set, so a policy referencing them fails to resolve. `rand.intn`
        // rides on the `std` feature and can't be feature-dropped, so shadow it
        // with an extension that errors. Extensions resolve before builtins, so
        // a policy calling `rand.intn(_, n)` fails fast — unless the caller
        // opted into non-determinism via `allow_impure_builtins`.
        if !config.allow_impure_builtins {
            inner
                .add_extension(
                    "rand.intn".to_string(),
                    2,
                    Box::new(|_args: Vec<RegorusValue>| -> anyhow::Result<RegorusValue> {
                        anyhow::bail!(
                            "impure builtin `rand.intn` is disabled for deterministic \
                             evaluation (POLENG-009); set \
                             EngineConfig::allow_impure_builtins to allow it"
                        )
                    }),
                )
                .map_err(|e| EngineError::Regorus(e.to_string()))?;
        }

        Ok(Self {
            inner,
            config,
            context: Arc::new(RwLock::new(PolicyInput::default())),
            poisoned: false,
        })
    }

    /// Run a regorus call under `catch_unwind` (CIB-018).
    ///
    /// regorus 0.10 has internal `unwrap`/`expect` paths; an adversarial or
    /// malformed policy can panic deep inside `add_policy`/`eval_query`. Without
    /// this guard that panic unwinds out of the `anvil` process and aborts it
    /// with no `--json` error envelope, breaking any pipeline parsing the
    /// output. Convert the panic into [`EngineError::Regorus`] and poison the
    /// engine, since regorus state may be inconsistent after a partial unwind.
    ///
    /// `AssertUnwindSafe` is sound here precisely *because* of the poison flag:
    /// after a caught panic the engine never exposes `inner` to a caller again
    /// (every subsequent call hits the poison check first), so a broken
    /// invariant inside `inner` cannot be observed.
    ///
    /// Note this is only effective under `panic = "unwind"`; the binary builds
    /// with it for exactly this reason (ADR-051). Under `panic = "abort"` the
    /// process aborts before this `catch_unwind` runs.
    fn guard<T>(
        &mut self,
        what: &str,
        f: impl FnOnce(&mut RegorusEngine) -> Result<T, EngineError>,
    ) -> Result<T, EngineError> {
        if self.poisoned {
            return Err(EngineError::Regorus(format!(
                "engine is unusable after a previously caught panic; cannot run {what}"
            )));
        }
        let inner = &mut self.inner;
        // The closure keeps its own error mapping (so `Input` vs `Regorus`
        // classification is preserved); `guard` only adds the panic catch.
        match panic::catch_unwind(AssertUnwindSafe(|| f(inner))) {
            Ok(result) => result,
            Err(payload) => {
                self.poisoned = true;
                let message = panic_payload_message(payload.as_ref());
                // CIB-017: a caught regorus panic is an abnormal condition the
                // caller turns into an error — surface it as a `warn!` so it's
                // observable in logs, not just in the returned `Err`.
                tracing::warn!(operation = what, panic = %message, "caught panic in regorus call; engine poisoned");
                Err(EngineError::Regorus(format!(
                    "regorus panicked during {what}: {message}"
                )))
            }
        }
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
        let extension = Box::new(
            move |args: Vec<RegorusValue>| -> anyhow::Result<RegorusValue> {
                let json_args: Vec<serde_json::Value> = args
                    .iter()
                    .map(serde_json::to_value)
                    .collect::<Result<_, _>>()?;
                let input = context.read().map_err(|_| {
                    // Defensive (CIB-017): a read can't poison the lock, so this
                    // only fires if a write-hold path poisoned it — see the note
                    // at the `context.write()` site in `eval`.
                    tracing::warn!("policy input lock poisoned");
                    anyhow::Error::msg("policy input lock poisoned")
                })?;
                let out = builtin
                    .call(&input, &json_args)
                    .map_err(|e| anyhow::Error::msg(e.to_string()))?;
                // `RegorusValue::from(serde_json::Value)` silently maps a
                // non-representable value to `Undefined`; convert explicitly so
                // a builtin bug surfaces as an error instead of a phantom
                // `undefined` in the policy.
                serde_json::from_value::<RegorusValue>(out).map_err(|e| {
                    anyhow::Error::msg(format!(
                        "builtin `{}` produced a value regorus cannot represent: {e}",
                        builtin.name()
                    ))
                })
            },
        );
        // Guarded: registration is a `pub` method callable after `eval`, and
        // `add_extension` is a regorus call like any other (CIB-018).
        self.guard("register_builtin", move |inner| {
            inner
                .add_extension(name, arity, extension)
                .map_err(|e| EngineError::Regorus(e.to_string()))
        })
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
        let (path, source) = (path.into(), source.into());
        self.guard("add_policy", move |inner| {
            inner
                .add_policy(path, source)
                .map(|_| ()) // regorus returns the package path; we don't need it
                .map_err(|e| EngineError::Regorus(e.to_string()))
        })
    }

    /// Evaluate a Rego query against the loaded policies and the given input.
    pub fn eval(&mut self, input: &PolicyInput, query: &str) -> Result<EvalResult, EngineError> {
        let input_json =
            serde_json::to_string(input).map_err(|e| EngineError::Input(e.to_string()))?;
        self.guard("set_input_json", |inner| {
            inner
                .set_input_json(&input_json)
                .map_err(|e| EngineError::Input(e.to_string()))
        })?;

        // Refresh the shared context builtins read from. Cloning keeps the
        // borrow self-contained; `input` documents are small policy fixtures.
        // Reached only after `set_input_json` succeeded; on its failure/panic we
        // returned above (poisoned), so the context is never left half-updated.
        *self.context.write().map_err(|_| {
            // CIB-017: defensive. The `guard` (CIB-018) catches panics in
            // regorus calls before they could poison this lock, and the only
            // write-hold is the infallible `input.clone()` below, so in
            // practice this is unreachable — but if a future non-guarded path
            // ever poisons the lock, a `warn!` makes it observable rather than
            // a bare error.
            tracing::warn!("policy input lock poisoned");
            EngineError::Input("policy input lock poisoned".into())
        })? = input.clone();

        if self.config.collect_coverage {
            self.guard("set_coverage", |inner| {
                inner.set_enable_coverage(true);
                // Reset so coverage reflects only this evaluation.
                inner.clear_coverage_data();
                Ok(())
            })?;
        }

        let query_owned = query.to_string();
        let collect_trace = self.config.collect_trace;
        let results = self.guard("eval_query", move |inner| {
            inner
                .eval_query(query_owned, collect_trace)
                .map_err(|e| EngineError::Regorus(e.to_string()))
        })?;

        let value = extract_primary_eval_value(&results)?;

        let coverage = if self.config.collect_coverage {
            let report = self.guard("get_coverage_report", |inner| {
                inner
                    .get_coverage_report()
                    .map_err(|e| EngineError::Regorus(e.to_string()))
            })?;
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

/// The facade exposes a single primary JSON value. Reject semicolon-separated
/// multi-expression queries that would silently drop trailing expressions.
/// Multi-row comprehension results (one expression, many bindings) keep the first
/// row for `value`; use [`EvalResult::trace`] for the full binding set.
fn extract_primary_eval_value(
    results: &regorus::QueryResults,
) -> Result<Option<serde_json::Value>, EngineError> {
    let Some(query_result) = results.result.first() else {
        return Ok(None);
    };
    if query_result.expressions.len() > 1 {
        return Err(EngineError::Regorus(
            "query produced multiple expressions; use a single data-path query or one binding"
                .into(),
        ));
    }
    match query_result.expressions.first() {
        // Preserve the Rego undefined vs. null distinction: Rego `undefined`
        // collapses to `None`; an explicit JSON `null` stays `Some(Null)`.
        None => Ok(None),
        Some(expr) if matches!(expr.value, RegorusValue::Undefined) => Ok(None),
        Some(expr) => Ok(Some(
            serde_json::to_value(&expr.value).map_err(|e| EngineError::Regorus(e.to_string()))?,
        )),
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

    /// CIB-018: a panic during evaluation — here a builtin that panics, a
    /// deterministic stand-in for regorus's own internal `unwrap`/`expect` on
    /// an adversarial policy — must be caught at the facade and surfaced as an
    /// `Err`, never abort the process (which would leave `--json` callers and
    /// pipeline parsers with no error envelope). After a caught panic the
    /// engine is poisoned and refuses further evaluation rather than operate on
    /// possibly-inconsistent regorus state.
    #[test]
    fn eval_rejects_multi_expression_queries() {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        engine
            .add_policy("t.rego", "package t\nimport rego.v1\n")
            .expect("add_policy");
        let err = engine
            .eval(&PolicyInput::default(), "1 + 2; 3 + 4")
            .expect_err("multi-expression query must be rejected");
        assert!(
            matches!(err, EngineError::Regorus(ref msg) if msg.contains("multiple expressions")),
            "got {err:?}"
        );
    }

    #[test]
    fn eval_catches_panic_and_poisons_engine() {
        /// A builtin whose `call` panics instead of returning.
        struct Boom;
        impl Builtin for Boom {
            fn name(&self) -> &'static str {
                "anvil.boom"
            }
            fn arity(&self) -> u8 {
                1
            }
            fn determinism(&self) -> DeterminismClass {
                DeterminismClass::Pure
            }
            fn call(
                &self,
                _input: &PolicyInput,
                _args: &[serde_json::Value],
            ) -> Result<serde_json::Value, BuiltinError> {
                panic!("boom from builtin");
            }
        }

        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        engine.register_builtin(Arc::new(Boom)).expect("register");
        engine
            .add_policy(
                "boom.rego",
                "package boom\nimport rego.v1\n\nsafe := 1\ntriggered := anvil.boom(1)\n",
            )
            .expect("add_policy");

        let input = PolicyInput::default();

        // A query that does not reach the panicking builtin evaluates normally:
        // registering/loading did not falsely poison a healthy engine.
        let healthy = engine.eval(&input, "data.boom.safe").expect("healthy eval");
        assert_eq!(healthy.value, Some(serde_json::json!(1)));

        // The panic is caught and surfaced as `Regorus` with the message
        // preserved — the test process surviving is itself proof of no abort.
        let first = engine.eval(&input, "data.boom.triggered");
        let Err(EngineError::Regorus(msg)) = first else {
            panic!("expected caught panic as EngineError::Regorus, got {first:?}");
        };
        assert!(
            msg.contains("boom from builtin"),
            "panic payload not surfaced: {msg}"
        );

        // Poisoned: even the previously-healthy query now fails fast, rather
        // than touching regorus state a mid-evaluation panic may have left
        // inconsistent.
        let second = engine.eval(&input, "data.boom.safe");
        assert!(
            matches!(second, Err(EngineError::Regorus(_))),
            "poisoned engine must refuse further eval, got {second:?}"
        );
    }
}
