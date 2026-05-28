//! `.env` file governance surface (SURFENV).
//!
//! T1 (Scanned) coverage for `.env`, `.env.*`, `.envrc` files: parse the
//! file into key/value pairs, then hand each value off to the existing
//! secret scanner. Subsequent SURFENV tasks layer structural rules on top
//! (committed-`.env` detection, `.env.example` drift, prod-shaped values
//! in non-prod files — see `plans/modules/surface-env-files.aps.md`).
//!
//! This is the first slice (SURFENV-001 — file detection + scan-path
//! integration). Suppressions reuse the Rust antipattern parser per
//! [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md).

pub mod check;
pub mod drift;
pub mod gitignore;
pub mod parser;
pub mod prod_value;
pub mod scanner;
pub mod suppression;

pub use check::{SurfenvCheckResult, run_surfenv_check};
pub use drift::{DriftFinding, DriftKind, SURFENV_004_RULE_ID, check_env_drift};
pub use gitignore::{
    GitignoreFinding, GitignoreFindingKind, SURFENV_002_RULE_ID, check_gitignore_hygiene,
    check_gitignore_hygiene_for_paths,
};
pub use parser::{EnvEntry, EnvParseError, parse_env};
pub use prod_value::{ProdIndicator, ProdValueFinding, SURFENV_003_RULE_ID, scan_prod_values};
pub use scanner::{EnvFinding, SURFENV_001_RULE_ID, is_env_file, scan_env_file};
pub use suppression::{resolve_file_header_suppression, resolve_line_suppression};

/// Canonical registry of every SURFENV structural rule ID.
///
/// The cross-rule suppression audit (`tests/surfenv_suppression_audit.rs`)
/// drives its coverage from this slice, so registering a new rule trips the
/// audit's exhaustiveness check until a matching suppression case is added.
/// Keep this in sync with the `SURFENV_001_RULE_ID`..`SURFENV_004_RULE_ID`
/// constants re-exported above.
pub const SURFENV_RULES: &[&str] = &[
    SURFENV_001_RULE_ID,
    SURFENV_002_RULE_ID,
    SURFENV_003_RULE_ID,
    SURFENV_004_RULE_ID,
];
