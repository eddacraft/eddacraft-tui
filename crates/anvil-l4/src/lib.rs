//! L4 policy framework (MLP-006).
//!
//! Owns the `anvil/policy.yml` (`.json` / `.toml`) parser and the
//! per-branch pattern matching that drives `validate_at_l4`'s rule
//! resolution. ADR-037 §D-5 defines the schema; this crate provides
//! the parse + resolve primitive that the server-side validation
//! path consumes.
//!
//! ## Scope (MLP-006 v1 library)
//!
//! - [`Policy`] / [`BranchRule`] — the on-disk schema, deserialised
//!   from any of yaml / json / toml via [`anvil_config::parse_str`].
//! - [`Policy::resolve`] — given a branch name, return the first
//!   matching [`BranchRule`] in declaration order. Patterns use
//!   globset semantics (`main`, `dependabot/*`, `*`).
//! - [`Requirement`] / [`OnNoWitness`] / [`OnBlock`] / [`OnWarn`] —
//!   the closed-set enums from ADR-037 §D-5.
//! - [`Policy::commit_is_before_cutoff`] — `cutoff_commit` legacy
//!   acceptance helper. The caller passes ordered ancestor SHAs;
//!   this crate doesn't shell out to git.
//!
//! ## Out of scope (deferred to consumers)
//!
//! - `validate_at_l4` server-side execution path — owned by the
//!   future `validate-at-l4` command (CLI lane). The library here
//!   resolves the policy and tells the caller what to do; the
//!   caller invokes the rule engine.
//! - Writing the L4 witness to `refs/notes/anvil-l4` — owned by the
//!   git plumbing in the CLI / GitHub Action surface (MLP-010).
//!   ADR-037 §D-7 forbids the in-tree ledger from being touched at
//!   L4, so the notes ref is the only write target; that I/O lives
//!   outside this crate.
//! - DAG-aware merge verification — depends on `anvil-witness`'s
//!   merge-commit shape (MLP-002 follow-up #1); the L4 caller
//!   composes anvil-witness's verifier with the policy resolved
//!   here.
//! - `required_anvil_version` floor evaluation — exposed as an
//!   owned `Option<String>` field; the caller threads it through
//!   `anvil_rules::RequiredAnvilVersion::parse(...).satisfied_by(...)`
//!   (MLP-012). Keeping the dep direction one-way (anvil-l4 →
//!   anvil-config only) avoids coupling.
//! - Pre-push hook (MLP-004) — calls into this crate's resolver but
//!   lives in `crates/anvil-hook/` / `commands/hook.rs`.
//!
//! ## Schema example
//!
//! ```yaml
//! required_anvil_version: "0.6.0"
//! baseline:
//!   cutoff_commit: a3b2ea4e...
//! branches:
//!   - pattern: main
//!     require: l4_or_l3
//!     on_no_witness: validate_at_l4
//!     on_block: reject
//!   - pattern: dependabot/*
//!     require: l4_only
//!     on_no_witness: validate_at_l4
//!   - pattern: '*'
//!     require: l4_or_l3
//!     on_no_witness: validate_at_l4
//! ```

mod decide;
mod exceptions;
mod policy;
mod recognised_rules;
mod resolve;
mod validate;

pub use decide::{BlockKind, CommitDecision, VersionFloorOutcome, evaluate_version_floor};
pub use exceptions::{
    AppliedException, ExceptionDisposition, ExceptionOutcome, apply_exception_dispositions,
};
pub use policy::{
    BaselineSection, BranchRule, OnBlock, OnNoWitness, OnWarn, Policy, PolicyParseError,
    PolicyPinError, Requirement, pin_cutoff_commit,
};
pub use recognised_rules::{
    RecognisedRulesRegistry, RegistryError, RuleSetMetadata, RulesShaOutcome, evaluate_rules_sha,
};
pub use resolve::ResolveError;
pub use validate::{
    EngineUnavailableReason, NoOpValidationEngine, Severity, ValidationDiagnostic,
    ValidationEngine, ValidationRequest, ValidationVerdict, request_for, validate_at_l4,
    validate_range,
};
