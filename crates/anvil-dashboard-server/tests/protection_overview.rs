use std::fs;

use anvil_dashboard_server::{DataState, Workspace, load_persisted_protection_overview};
use tempfile::tempdir;

#[test]
fn maps_the_latest_gate_artefact_without_claiming_daemon_state() {
    let root = tempdir().expect("workspace");
    fs::create_dir(root.path().join(".anvil")).expect(".anvil");
    fs::write(
        root.path().join(".anvil/gates.json"),
        br#"{
          "status": "fail",
          "statusLabel": "FAILED - score 72/100",
          "score": 72,
          "warnings": "2",
          "durationSeconds": "4.8",
          "checksRun": "2",
          "checkRows": [
            ["lint", "passed", "100", "No lint errors"],
            ["secret-detection", "failed", "0", "Potential secret in src/config.ts:18\nmore detail"]
          ],
          "warningList": [
            {"severity":"error","message":"secret-detection: Potential secret in src/config.ts:18"},
            {"severity":"warn","message":"architecture: configuration required"}
          ]
        }"#,
    )
    .expect("gate fixture");
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    let overview = load_persisted_protection_overview(&workspace);

    assert_eq!(overview.data_state, DataState::Partial);
    assert!(
        overview.claim.is_none(),
        "a gate file is not daemon evidence"
    );
    let latest = overview.latest_run.expect("latest gate run");
    assert_eq!(latest.result, "fail");
    assert_eq!(latest.score, Some(72.0));
    assert!(overview.warnings.is_empty());
    assert!(overview.next_attention.is_some());
    assert_eq!(overview.warnings_state, DataState::Unavailable);
    assert_eq!(overview.affected_files_state, DataState::Unavailable);
    assert!(overview.recent_runs.is_empty());
    assert!(overview.affected_files.is_empty());
    assert!(overview.gaps.iter().any(|gap| {
        gap.component == "retained-warning-history"
            && gap.reason.contains("canonical gate snapshot")
    }));
}

#[test]
fn missing_gate_artefact_is_an_honest_empty_state() {
    let root = tempdir().expect("workspace");
    let workspace = Workspace::new(root.path()).expect("workspace boundary");

    let overview = load_persisted_protection_overview(&workspace);

    assert_eq!(overview.data_state, DataState::Unavailable);
    assert!(overview.latest_run.is_none());
    assert!(overview.warnings.is_empty());
    assert_eq!(overview.warnings_state, DataState::Unavailable);
    assert_eq!(overview.affected_files_state, DataState::Unavailable);
    assert!(overview.source_message.contains("No local gate artefact"));
}
