//! Aggregator for the SURFSH rule pack.
//!
//! `run_surfsh_check` builds the shared `command_safety` filesystem ruleset
//! once, then scans each shell file against it. Discovery is the caller's
//! job: pass `(path, content)` pairs for which
//! [`is_shell_file`](super::scanner::is_shell_file) is true. Mirrors
//! [`crate::surface::sql::run_surfsql_check`].

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::scanner::{ShellFinding, scan_shell_with_rules};
use crate::command_safety::rules::default_filesystem_rules;

/// Aggregated result of running SURFSH against a file set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SurfshCheckResult {
    /// SURFSH-002 — dangerous commands in shell scripts.
    pub commands: Vec<ShellFinding>,
}

impl SurfshCheckResult {
    /// Total finding count, including suppressed findings.
    #[must_use]
    pub fn total_findings(&self) -> usize {
        self.commands.len()
    }

    /// Count an operator actually has to action (unsuppressed only).
    #[must_use]
    pub fn unsuppressed_findings(&self) -> usize {
        self.commands.iter().filter(|f| !f.suppressed).count()
    }
}

/// Run SURFSH against a set of shell scripts.
#[must_use]
pub fn run_surfsh_check(shell_files: &[(PathBuf, String)]) -> SurfshCheckResult {
    // Build the shared filesystem ruleset once, reuse across files.
    let rules = default_filesystem_rules();
    let mut result = SurfshCheckResult::default();
    for (path, content) in shell_files {
        let display = path.to_string_lossy();
        result
            .commands
            .extend(scan_shell_with_rules(&display, content, &rules));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{SurfshCheckResult, run_surfsh_check};
    use std::path::PathBuf;

    #[test]
    fn empty_input_yields_default_result() {
        let result = run_surfsh_check(&[]);
        assert_eq!(result.total_findings(), 0);
        assert_eq!(result.unsuppressed_findings(), 0);
    }

    #[test]
    fn aggregates_findings_across_files() {
        let files = vec![
            (PathBuf::from("a.sh"), "rm -rf /\n".to_string()),
            (PathBuf::from("b.bash"), "echo safe\n".to_string()),
        ];
        let result = run_surfsh_check(&files);
        assert_eq!(result.unsuppressed_findings(), 1);
    }

    #[test]
    fn result_round_trips_via_json() {
        let files = vec![(PathBuf::from("a.sh"), "rm -rf /\n".to_string())];
        let result = run_surfsh_check(&files);
        let json = serde_json::to_string(&result).expect("serialize");
        let round: SurfshCheckResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round.total_findings(), result.total_findings());
    }
}
