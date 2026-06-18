//! GitHub Actions workflow governance surface (SURFGHA).
//!
//! T2 (Policy) coverage for `.github/workflows/*.yml` — a supply-chain
//! pattern catalogue + `#` suppression per
//! `plans/modules/surface-github-actions.aps.md`. Blast radius is critical:
//! supply-chain compromise is the canonical "one ungoverned file ruins
//! everything" case.
//!
//! First slice (SURFGHA-001 file detection, SURFGHA-002 supply-chain
//! catalogue: unpinned branch action refs, `pull_request_target`, self-hosted
//! runners). Suppressions reuse the Rust antipattern parser per
//! [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md).

pub mod check;
pub mod scanner;
pub mod suppression;

pub use check::{SurfghaCheckResult, run_surfgha_check};
pub use scanner::{GhaFinding, GhaRisk, SURFGHA_002_RULE_ID, is_workflow_file, scan_workflow};
pub use suppression::resolve_line_suppression;

/// Canonical registry of every SURFGHA structural rule ID. A future
/// cross-rule suppression audit drives its coverage from this slice.
pub const SURFGHA_RULES: &[&str] = &[SURFGHA_002_RULE_ID];
