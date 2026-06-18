//! Shell-script governance surface (SURFSH).
//!
//! T1 (Scanned) coverage for `*.sh`/`*.bash` — a static scan of checked-in
//! shell scripts that **reuses** the shared `command_safety` filesystem rule
//! engine (the `rm -rf /` family etc.) rather than duplicating it, per
//! `plans/modules/surface-shell.aps.md`. One catalogue, two consumers.
//!
//! First slice (SURFSH-001 file detection, SURFSH-002 dangerous-command scan
//! over the shared catalogue). Suppressions reuse the Rust antipattern parser
//! per [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md).

pub mod check;
pub mod scanner;
pub mod suppression;

pub use check::{SurfshCheckResult, run_surfsh_check};
pub use scanner::{
    SURFSH_002_RULE_ID, ShellFinding, ShellSeverity, is_shell_file, scan_shell,
    scan_shell_with_rules,
};
pub use suppression::resolve_line_suppression;

/// Canonical registry of every SURFSH structural rule ID.
pub const SURFSH_RULES: &[&str] = &[SURFSH_002_RULE_ID];
