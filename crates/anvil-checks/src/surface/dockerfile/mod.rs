//! Dockerfile governance surface (SURFDOCK).
//!
//! T2 (Policy) coverage for `Dockerfile`/`Containerfile`/`*.Dockerfile` — a
//! build-hygiene / supply-chain pattern catalogue + `#` suppression per
//! `plans/modules/surface-dockerfile.aps.md`.
//!
//! First slice (SURFDOCK-001 file detection, SURFDOCK-002 catalogue:
//! `ADD` remote-fetch, pipe-to-shell, `:latest` base images, `sudo` in
//! layers, `apt-get install` without `--no-install-recommends`). Suppressions
//! reuse the Rust antipattern parser per
//! [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md).

pub mod check;
pub mod scanner;
pub mod suppression;

pub use check::{SurfdockCheckResult, run_surfdock_check};
pub use scanner::{
    DockerFinding, DockerRisk, SURFDOCK_002_RULE_ID, is_dockerfile, scan_dockerfile,
};
pub use suppression::resolve_line_suppression;

/// Canonical registry of every SURFDOCK structural rule ID.
pub const SURFDOCK_RULES: &[&str] = &[SURFDOCK_002_RULE_ID];
