//! Determinism contract (POLENG-004).
//!
//! Anvil policy evaluation must be reproducible: the same policy over the same
//! [`crate::PolicyInput`] always produces byte-identical output. That holds
//! only if every data source a policy can reach is itself deterministic. This
//! module makes that a *declared* property of each builtin and lets the engine
//! refuse anything that isn't.
//!
//! The guarantee is enforced by the type system, not a lint: [`Builtin`]
//! requires a [`DeterminismClass`] via [`Builtin::determinism`], so a builtin
//! cannot be written — let alone registered — without declaring its class.
//! [`crate::Engine::register_builtin`] then rejects an [`DeterminismClass::Impure`]
//! builtin unless the engine was explicitly configured to allow one. The
//! end-to-end `repeatable_eval` integration test exercises the runtime
//! guarantee (a representative policy evaluated 100× yields identical bytes).

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
}
