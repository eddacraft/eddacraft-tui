//! L4 policy framework (MLP-006).
//!
//! Parses `anvil/policy.yml` (`.json` / `.toml`) and evaluates pre-push /
//! commit-policy decisions. Library crate; CLI and hooks bind the engine.

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
