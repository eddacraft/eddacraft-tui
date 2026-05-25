//! Determinism contract (POLENG-004).
//!
//! Anvil's *own* data sources must be reproducible: a builtin's output is a
//! pure function of its arguments and the policy [`crate::PolicyInput`]. This
//! module makes that a *declared, type-enforced* property rather than a lint:
//! [`Builtin`] requires a [`DeterminismClass`] via [`Builtin::determinism`], so
//! a builtin cannot be written — let alone registered — without declaring its
//! class, and [`crate::Engine::register_builtin`] rejects an
//! [`DeterminismClass::Impure`] builtin unless the engine was explicitly
//! configured to allow one. The `repeatable_eval_is_byte_identical_over_100_runs`
//! integration test (`tests/determinism.rs`) exercises the runtime guarantee (a
//! representative policy evaluated 100× yields identical bytes).
//!
//! ## Scope and the stdlib fence (POLENG-009)
//!
//! The [`Builtin`] contract above governs only the first-party `anvil.*`
//! builtins. The Rego *stdlib* that policy text can call directly is fenced
//! separately, two ways:
//!
//! 1. **Feature removal.** The crate builds `regorus` without its `full-opa`
//!    bundle, dropping the `time` / `uuid` / `http` / `net` / `opa-runtime`
//!    builtin groups (and their deps) entirely — a policy referencing
//!    `time.now_ns()` or `uuid.rfc4122()` fails to resolve. See this crate's
//!    `Cargo.toml`.
//! 2. **Runtime shadow.** `rand.intn` rides on the `std` feature and can't be
//!    feature-dropped, so [`crate::Engine::new`] registers an extension that
//!    shadows it with an error — unless [`crate::EngineConfig::allow_impure_builtins`]
//!    is set (an explicit opt-in to non-determinism).
//!
//! So the byte-identical guarantee holds for any policy on a default engine:
//! impure stdlib builtins either don't resolve or error at call time, rather
//! than silently producing non-reproducible output. (`http.send` and
//! `opa.runtime` env access were already stubbed in regorus 0.10.0; the fence
//! removes them outright.)

/// Determinism classification every registered [`Builtin`] must declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeterminismClass {
    /// Output is a pure function of the builtin's arguments and the policy
    /// input document. No clock, environment, network, filesystem, or RNG.
    /// The only class that may be registered on a default engine.
    Pure,
    /// Observes ambient state (clock, environment, network, filesystem). The
    /// engine refuses to register an impure builtin unless
    /// [`crate::EngineConfig::allow_impure_builtins`] is explicitly set, and
    /// doing so forfeits the determinism guarantee.
    Impure,
}

/// Error raised by a [`Builtin`] while evaluating.
///
/// Builtins surface failures through this facade type rather than
/// `anyhow`/`regorus` errors, so implementors depend only on this crate.
#[derive(Debug, thiserror::Error)]
pub enum BuiltinError {
    /// The builtin was called with the wrong number of arguments. (regorus
    /// enforces arity at the call site too; this guards hand-built tests and
    /// future dynamic registration paths.)
    #[error("builtin `{name}` expected {expected} argument(s), got {got}")]
    Arity {
        name: String,
        expected: u8,
        got: usize,
    },
    /// An argument had the wrong shape or an otherwise invalid value.
    #[error("builtin `{name}`: {message}")]
    Invalid { name: String, message: String },
}

/// A first-party builtin exposed to Rego policies.
///
/// A builtin is a pure function of the current [`crate::PolicyInput`] and its
/// JSON-valued arguments — which is exactly what [`DeterminismClass::Pure`]
/// means. The engine threads the input document into every call, so a
/// zero-argument data source such as `anvil.repo_state()` can read repo state
/// without the policy passing it in.
///
/// Builtins speak [`serde_json::Value`], never `regorus::Value`, so
/// implementors depend only on this facade (ADR-040 D-1); the engine bridges
/// the two value representations at registration time.
///
/// Every builtin MUST declare a [`DeterminismClass`]. This is the load-bearing
/// half of the determinism contract: there is no way to express a builtin
/// without stating whether it is pure.
pub trait Builtin: Send + Sync {
    /// Rego call path the builtin is registered under (e.g. `anvil.plan`).
    /// Fixed per builtin, hence `'static`.
    fn name(&self) -> &'static str;

    /// Number of arguments the builtin accepts.
    fn arity(&self) -> u8;

    /// Determinism classification — see [`DeterminismClass`].
    fn determinism(&self) -> DeterminismClass;

    /// Evaluate the builtin against the current input document and the
    /// JSON-valued arguments supplied by the policy.
    fn call(
        &self,
        input: &crate::PolicyInput,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, BuiltinError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, EngineConfig, EngineError, PolicyInput};
    use std::sync::Arc;

    /// Minimal pure builtin: `anvil.echo(x)` returns its argument unchanged.
    struct Echo;
    impl Builtin for Echo {
        fn name(&self) -> &'static str {
            "anvil.echo"
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
            args: &[serde_json::Value],
        ) -> Result<serde_json::Value, BuiltinError> {
            Ok(args[0].clone())
        }
    }

    /// An impure builtin used only to prove the engine refuses it by default.
    struct WallClock;
    impl Builtin for WallClock {
        fn name(&self) -> &'static str {
            "anvil.now"
        }
        fn arity(&self) -> u8 {
            0
        }
        fn determinism(&self) -> DeterminismClass {
            DeterminismClass::Impure
        }
        fn call(
            &self,
            _input: &PolicyInput,
            _args: &[serde_json::Value],
        ) -> Result<serde_json::Value, BuiltinError> {
            Ok(serde_json::json!("now"))
        }
    }

    #[test]
    fn pure_builtin_registers_and_is_callable() {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        engine.register_builtin(Arc::new(Echo)).expect("register");
        engine
            .add_policy(
                "echo.rego",
                "package t\nimport rego.v1\nout := anvil.echo(\"hi\")\n",
            )
            .expect("add_policy");
        let result = engine
            .eval(&PolicyInput::default(), "data.t.out")
            .expect("eval");
        assert_eq!(result.value, Some(serde_json::json!("hi")));
    }

    #[test]
    fn impure_builtin_rejected_by_default() {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        let err = engine
            .register_builtin(Arc::new(WallClock))
            .expect_err("impure builtin must be rejected");
        assert!(matches!(err, EngineError::ImpureBuiltinRejected(name) if name == "anvil.now"));
    }

    #[test]
    fn impure_builtin_allowed_when_opted_in() {
        let mut engine = Engine::new(EngineConfig {
            allow_impure_builtins: true,
            ..Default::default()
        })
        .expect("engine");
        engine
            .register_builtin(Arc::new(WallClock))
            .expect("opted-in impure builtin should register");
    }

    /// Evaluate a one-rule policy and return whether the whole flow succeeded.
    fn try_eval(allow_impure: bool, rule_body: &str) -> Result<Option<serde_json::Value>, ()> {
        let mut engine = Engine::new(EngineConfig {
            allow_impure_builtins: allow_impure,
            ..Default::default()
        })
        .map_err(|_| ())?;
        engine
            .add_policy(
                "t.rego",
                format!("package t\nimport rego.v1\nx := {rule_body}\n"),
            )
            .map_err(|_| ())?;
        engine
            .eval(&PolicyInput::default(), "data.t.x")
            .map(|r| r.value)
            .map_err(|_| ())
    }

    #[test]
    fn impure_stdlib_builtins_are_removed_by_features() {
        // POLENG-009: time/uuid (and http/net/opa-runtime) are dropped from the
        // regorus feature set, so a policy referencing them cannot evaluate.
        assert!(
            try_eval(false, "time.now_ns()").is_err(),
            "time.now_ns must not resolve"
        );
        assert!(
            try_eval(false, "uuid.rfc4122(\"x\")").is_err(),
            "uuid.rfc4122 must not resolve"
        );
        // Even opting into impurity does not bring back a feature-removed builtin.
        assert!(
            try_eval(true, "time.now_ns()").is_err(),
            "feature-removed builtins stay removed regardless of allow_impure_builtins"
        );
    }

    #[test]
    fn rand_intn_is_fenced_by_default_and_allowed_when_opted_in() {
        // `rand.intn` rides on the `std` feature and can't be feature-dropped,
        // so it is shadowed by an erroring extension on a default engine…
        assert!(
            try_eval(false, "rand.intn(\"seed\", 10)").is_err(),
            "rand.intn must be fenced by default"
        );
        // …and reachable only when the caller opts into non-determinism.
        let allowed = try_eval(true, "rand.intn(\"seed\", 10)")
            .expect("rand.intn should evaluate when impurity is allowed");
        assert!(
            allowed.is_some_and(|v| v.is_number()),
            "rand.intn should yield a number when opted in"
        );
    }

    #[test]
    fn rand_intn_shadow_fires_through_an_indirect_call() {
        // The shadow is resolved in the interpreter's call path, so it fires
        // even when `rand.intn` is reached indirectly (here, inside an array
        // literal) — not only as a bare top-level call.
        assert!(
            try_eval(false, "[rand.intn(\"s\", 10)]").is_err(),
            "the rand.intn shadow must fire regardless of call position"
        );
    }

    #[test]
    fn pure_stdlib_builtins_still_work() {
        // The pure subset (sprintf/count/regex/etc.) is retained.
        let out = try_eval(false, "count([1, 2, 3])").expect("pure builtin works");
        assert_eq!(out, Some(serde_json::json!(3)));
    }
}
