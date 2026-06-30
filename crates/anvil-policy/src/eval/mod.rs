//! Eval-harness integration (EVAL module).
//!
//! Anvil-owned contracts for running trust/safety regression suites through an
//! external eval harness without coupling the core domain to framework
//! internals. Everything binds to the **frozen** `anvil policy eval --json` v1
//! output contract ([`docs/specs/policy-eval-output-v1.md`]), not to
//! `anvil-policy-engine` types.
//!
//! - [`port`] — EVAL-001: the `EvalHarnessPort` contract and its value types.
//! - [`adapter`] — EVAL-002: the concrete adapter that normalises v1 JSON.
//! - [`store`] — EVAL-004: canonical persistence of eval results.
//! - [`guidance`] — EVAL-005: linking failures to remediation guidance.

pub mod adapter;
pub mod guidance;
pub mod port;
pub mod store;

pub use adapter::{PolicyEvalAdapter, PolicyEvalRunner, SubprocessRunner, normalise};
pub use guidance::{GuidedFinding, PolicyGuidance, guidance_for};
pub use port::{
    EvalFinding, EvalHarnessError, EvalHarnessPort, EvalRegressionReport, EvalRunSummary,
    EvalSeverity, EvalSuite,
};
pub use store::{EvalRecord, EvalResultStore, StoreError};
