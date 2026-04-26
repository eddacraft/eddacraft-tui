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

pub mod parser;
pub mod scanner;

pub use parser::{EnvEntry, EnvParseError, parse_env};
pub use scanner::{EnvFinding, is_env_file, scan_env_file};
