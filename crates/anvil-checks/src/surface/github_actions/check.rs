//! Aggregator for the SURFGHA rule pack.
//!
//! `run_surfgha_check` is the single entry point. Discovery is the caller's
//! job: pass the `(path, content)` pairs for which
//! [`is_workflow_file`](super::scanner::is_workflow_file) is true. Mirrors
//! [`crate::surface::sql::run_surfsql_check`].

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::scanner::{GhaFinding, scan_workflow};

/// Aggregated result of running every SURFGHA rule against a file set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SurfghaCheckResult {
    /// SURFGHA-002 — supply-chain risks.
    pub risks: Vec<GhaFinding>,
}

impl SurfghaCheckResult {
    /// Total finding count, including suppressed findings.
    #[must_use]
    pub fn total_findings(&self) -> usize {
        self.risks.len()
    }

    /// Count an operator actually has to action (unsuppressed only).
    #[must_use]
    pub fn unsuppressed_findings(&self) -> usize {
        self.risks.iter().filter(|f| !f.suppressed).count()
    }
}

/// Run every SURFGHA rule against a set of workflow files.
#[must_use]
pub fn run_surfgha_check(workflow_files: &[(PathBuf, String)]) -> SurfghaCheckResult {
    let mut result = SurfghaCheckResult::default();
    for (path, content) in workflow_files {
        let display = path.to_string_lossy();
        result.risks.extend(scan_workflow(&display, content));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{SurfghaCheckResult, run_surfgha_check};
    use crate::surface::github_actions::scanner::GhaRisk;
    use std::path::PathBuf;

    #[test]
    fn empty_input_yields_default_result() {
        let result = run_surfgha_check(&[]);
        assert_eq!(result.total_findings(), 0);
        assert_eq!(result.unsuppressed_findings(), 0);
    }

    #[test]
    fn aggregates_risks_across_files() {
        let a = "on:\n  pull_request_target:\n".to_string();
        let b = "jobs:\n  b:\n    runs-on: self-hosted\n    steps:\n      - uses: x/y@main\n"
            .to_string();
        let files = vec![
            (PathBuf::from(".github/workflows/a.yml"), a),
            (PathBuf::from(".github/workflows/b.yml"), b),
        ];
        let result = run_surfgha_check(&files);
        let risks: Vec<GhaRisk> = result.risks.iter().map(|f| f.risk).collect();
        assert!(risks.contains(&GhaRisk::PullRequestTarget));
        assert!(risks.contains(&GhaRisk::SelfHostedRunner));
        assert!(risks.contains(&GhaRisk::UnpinnedActionRef));
        assert_eq!(result.unsuppressed_findings(), 3);
    }

    #[test]
    fn result_round_trips_via_json() {
        let files = vec![(
            PathBuf::from(".github/workflows/a.yml"),
            "on: [pull_request_target]\n".to_string(),
        )];
        let result = run_surfgha_check(&files);
        let json = serde_json::to_string(&result).expect("serialize");
        let round: SurfghaCheckResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round.total_findings(), result.total_findings());
    }
}
