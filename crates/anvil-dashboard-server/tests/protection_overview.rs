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
          "runs": [
            {"id":"gate-1","result":"fail","label":"Failed","score":72,"warningCount":2,"durationSeconds":4.8,"startedAt":"2026-07-13T08:30:00Z","newWarningCount":1,"changedFileCount":1}
          ],
          "warningDetails": [
            {"id":"warning-secret","severity":"high","rule":"secret-detection","category":"Secrets","message":"Potential secret","filePath":"src/config.ts","line":18,"ageLabel":"2m ago","evidenceId":"evidence-secret","explanation":"A secret-shaped value was detected.","matchedPattern":"secret","evidenceExcerpt":[{"number":18,"text":"const token = redacted","highlighted":true}]}
          ],
          "affectedFiles": [
            {"path":"src/config.ts","highestSeverity":"high","warningCount":1,"firstSeen":"2m ago","lastSeen":"now","warningId":"warning-secret"}
          ],
          "checkRows": [
            ["lint", "passed", "100", "No lint errors"],
            ["secret-detection", "failed", "0", "Potential secret in src/config.ts:18\nmore detail"]
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
    assert_eq!(overview.warnings.len(), 1);
    assert_eq!(overview.warnings[0].category, "Secrets");
    assert_eq!(overview.warnings[0].message, "Potential secret");
    assert!(overview.next_attention.is_some());
    assert_eq!(overview.warnings_state, DataState::Partial);
    assert_eq!(overview.affected_files_state, DataState::Complete);
    assert_eq!(overview.recent_runs.len(), 1);
    assert_eq!(overview.recent_runs[0].started_at, "2026-07-13T08:30:00Z");
    assert_eq!(overview.warnings[0].rule, "secret-detection");
    assert_eq!(overview.warnings[0].evidence_excerpt[0].number, 18);
    assert_eq!(overview.affected_files.len(), 1);
    assert_eq!(overview.affected_files[0].warning_id, "warning-secret");
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
