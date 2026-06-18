//! SQL migrations governance surface (SURFSQL).
//!
//! T2 (Policy) coverage for `.sql` migration files: a destructive-pattern
//! catalogue + `--` suppression today, with schema-hygiene rules and a drift
//! baseline layering on per `plans/modules/surface-sql-migrations.aps.md`.
//! Ranked #1 in Track 3 by demand × blast radius — schema/data destruction
//! cannot be undone.
//!
//! This is the first slice (SURFSQL-001 file detection + migration-directory
//! heuristics, SURFSQL-002 destructive-pattern catalogue, SURFSQL-004 `--`
//! suppression). Phase 1 is Postgres-flavoured. Suppressions reuse the Rust
//! antipattern parser per
//! [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md).

pub mod check;
pub mod scanner;
pub mod suppression;

pub use check::{SurfsqlCheckResult, run_surfsql_check};
pub use scanner::{
    DestructiveKind, HygieneKind, SURFSQL_002_RULE_ID, SURFSQL_003_RULE_ID, SqlFinding,
    SqlHygieneFinding, is_sql_migration_file, scan_sql_file, scan_sql_hygiene,
};
pub use suppression::{resolve_file_header_suppression, resolve_line_suppression};

/// Canonical registry of every SURFSQL structural rule ID.
///
/// A cross-rule suppression audit drives its coverage from this slice, so
/// registering a new rule trips the audit's exhaustiveness check until a
/// matching suppression case exists. Keep in sync with the
/// `SURFSQL_00n_RULE_ID` constants as rules land.
pub const SURFSQL_RULES: &[&str] = &[SURFSQL_002_RULE_ID, SURFSQL_003_RULE_ID];
