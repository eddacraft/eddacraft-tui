//! Contextual policy assertions (CPOL).
//!
//! A layer over [`crate::PolicyInput`]-adjacent facts that evaluates authored,
//! declarative *assertions* with richer deterministic workflow context while
//! preserving Anvil policy-pack semantics (ADR-002 warnings-over-blocks,
//! ADR-040 D-2 determinism):
//!
//! - [`assertion`] — the assertion schema (CPOL-001): scope, a closed set of
//!   conditions, and an outcome severity band.
//! - [`adapters`] — deterministic context payloads and assertion evaluation
//!   (CPOL-002): no clock, no filesystem, no environment reads at build time.
//! - [`guidance`] — remediation-first violation output (CPOL-003), aligned with
//!   [`crate::pack`] validation conventions.

pub mod adapters;
pub mod assertion;
pub mod guidance;

pub use adapters::{
    AssertionContext, AssertionEvaluation, ChangedPath, Violation, evaluate, glob_match, in_scope,
};
pub use assertion::{
    Assertion, AssertionCondition, AssertionError, AssertionScope, ChangeKind,
    ChangedPathCountSpec, Comparison, ConfigKey, ConfigMatch, PathGlob, WorkflowPhase,
};
pub use guidance::{
    AssertionGuidance, GuidanceCode, assess, blocks_under, decision_under, guidance_for,
};
