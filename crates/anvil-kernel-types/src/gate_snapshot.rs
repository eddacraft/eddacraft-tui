//! Canonical persisted gate-summary snapshot.
//!
//! The CLI writes this display-oriented contract to `.anvil/gates.json` and
//! read-only presentation adapters consume the same type. It deliberately
//! contains only facts produced by one gate run; retained diagnostics, run
//! history, affected files, and timestamps belong to other authoritative
//! evidence sources.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateSnapshot {
    pub status: String,
    pub status_label: String,
    pub score: f64,
    pub checks_run: String,
    pub warnings: String,
    pub duration_seconds: String,
    pub check_rows: Vec<Vec<String>>,
    pub warning_list: Vec<GateSnapshotWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateSnapshotWarning {
    pub severity: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_snapshot_round_trips_the_producer_shape() {
        let snapshot: GateSnapshot = serde_json::from_value(serde_json::json!({
            "status": "warn",
            "statusLabel": "PASSED — score 100/100",
            "score": 100,
            "checksRun": "1",
            "warnings": "1",
            "durationSeconds": "0.5",
            "checkRows": [["architecture", "config", "0", "configuration required"]],
            "warningList": [{"severity": "warn", "message": "architecture: configuration required"}]
        }))
        .expect("canonical gate snapshot");

        assert_eq!(snapshot.warning_list[0].severity, "warn");
    }
}
