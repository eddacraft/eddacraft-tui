//! Aggregator for the SURFDOCK rule pack.
//!
//! `run_surfdock_check` is the single entry point. Discovery is the caller's
//! job: pass `(path, content)` pairs for which
//! [`is_dockerfile`](super::scanner::is_dockerfile) is true. Mirrors
//! [`crate::surface::sql::run_surfsql_check`].

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::scanner::{DockerFinding, scan_dockerfile};

/// Aggregated result of running every SURFDOCK rule against a file set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SurfdockCheckResult {
    /// SURFDOCK-002 — build-hygiene / supply-chain risks.
    pub risks: Vec<DockerFinding>,
}

impl SurfdockCheckResult {
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

/// Run every SURFDOCK rule against a set of Dockerfiles.
#[must_use]
pub fn run_surfdock_check(dockerfiles: &[(PathBuf, String)]) -> SurfdockCheckResult {
    let mut result = SurfdockCheckResult::default();
    for (path, content) in dockerfiles {
        let display = path.to_string_lossy();
        result.risks.extend(scan_dockerfile(&display, content));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{SurfdockCheckResult, run_surfdock_check};
    use crate::surface::dockerfile::scanner::DockerRisk;
    use std::path::PathBuf;

    #[test]
    fn empty_input_yields_default_result() {
        let result = run_surfdock_check(&[]);
        assert_eq!(result.total_findings(), 0);
        assert_eq!(result.unsuppressed_findings(), 0);
    }

    #[test]
    fn aggregates_risks_across_files() {
        let a = "FROM node:latest\n".to_string();
        let b = "RUN apt-get install -y nginx\n".to_string();
        let files = vec![
            (PathBuf::from("a.Dockerfile"), a),
            (PathBuf::from("b/Dockerfile"), b),
        ];
        let result = run_surfdock_check(&files);
        let risks: Vec<DockerRisk> = result.risks.iter().map(|f| f.risk).collect();
        assert!(risks.contains(&DockerRisk::LatestBaseImage));
        assert!(risks.contains(&DockerRisk::AptMissingNoRecommends));
        assert_eq!(result.unsuppressed_findings(), 2);
    }

    #[test]
    fn result_round_trips_via_json() {
        let files = vec![(
            PathBuf::from("Dockerfile"),
            "FROM node:latest\n".to_string(),
        )];
        let result = run_surfdock_check(&files);
        let json = serde_json::to_string(&result).expect("serialize");
        let round: SurfdockCheckResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round.total_findings(), result.total_findings());
    }
}
