//! MLP2-008: RTAI-007 mid-edit telemetry envelope ↔ `gate_evaluated`
//! Kindling row join-back contract.
//!
//! These tests pin the explicit field map between the two mid-edit
//! surfaces and prove the join is exact: a telemetry envelope and the
//! Kindling row produced for the same `scan_buffer` call carry a
//! byte-identical `gate_eval_id` (the W3C `traceparent` parent-id),
//! derived from the single shared extractor
//! [`anvil_intercept::kindling_observation::gate_eval_id_from_traceparent`].
//! A downstream subscriber joins them with
//! `envelope.mirror.gate_eval_id == row.gate_eval_id`.

use anvil_intercept::kindling_observation::{
    Enforcement, ObservationContext, from_midedit_response, gate_eval_id_from_traceparent,
};
use anvil_intercept::midedit::ScanBufferResponse;
use anvil_intercept::telemetry::{
    ControlDecision, MirrorPath, TelemetryCorrelation, TelemetryEmitter,
};
use anvil_kernel_types::diagnostics::{Category, DiagnosticSource, KnownMode, Location, Severity};
use anvil_kernel_types::{Diagnostic, Mode};

/// Valid W3C traceparent; its parent-id (3rd segment) is the join key.
const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
const PARENT_ID: &str = "b7ad6b7169203331";
const SESSION_ID: &str = "11111111-1111-4111-8111-111111111111";
const FILE: &str = "src/lib.rs";

fn warning_diag() -> Diagnostic {
    Diagnostic::new(
        "diag-mlp2-008",
        Severity::Warning,
        "advisory mid-edit finding",
        Location {
            file: FILE.to_string(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Other,
        DiagnosticSource {
            rule_id: "anvil.test.mlp2-008".to_string(),
            source_module: "anvil-checks::test".to_string(),
        },
        Mode::known(KnownMode::MidEdit),
    )
}

fn response_with(diagnostics: Vec<Diagnostic>) -> ScanBufferResponse {
    ScanBufferResponse {
        version: 1,
        diagnostics,
        truncated: false,
        rules_sha: None,
        spoof_block: None,
    }
}

/// The envelope side: build the RTAI-007 mid-edit telemetry envelope
/// for one decision, carrying the `traceparent` on the correlation.
fn midedit_envelope(traceparent: Option<&str>) -> anvil_intercept::telemetry::NotificationEnvelope {
    let mut emitter = TelemetryEmitter::new();
    let correlation = TelemetryCorrelation {
        session_id: Some(SESSION_ID.to_string()),
        traceparent: traceparent.map(str::to_string),
        ..TelemetryCorrelation::default()
    };
    emitter.midedit_envelope_for_decision(correlation, &[warning_diag()])
}

#[test]
fn shared_extractor_yields_parent_id_or_none() {
    // The single source of the join key: parent-id on a valid
    // traceparent, None when absent or unparseable (so neither surface
    // ever invents a non-matching key from a bad input).
    assert_eq!(
        gate_eval_id_from_traceparent(Some(TRACEPARENT)).as_deref(),
        Some(PARENT_ID),
    );
    assert_eq!(gate_eval_id_from_traceparent(None), None);
    assert_eq!(
        gate_eval_id_from_traceparent(Some("not-a-traceparent")),
        None
    );
}

#[test]
fn envelope_and_row_join_on_gate_eval_id() {
    // Row side: the daemon stamps the row's gate_eval_id from the same
    // extractor (see ipc::derive_gate_eval_id), so model that here.
    let gate_eval_id = gate_eval_id_from_traceparent(Some(TRACEPARENT)).expect("valid traceparent");
    let ctx = ObservationContext {
        session_id: SESSION_ID,
        timestamp: "2026-06-14T12:00:00Z",
        gate_eval_id: &gate_eval_id,
        file_path: FILE,
        duration_ms: 7,
    };
    let row = from_midedit_response(&ctx, &response_with(vec![warning_diag()]))
        .expect("finding-bearing scan emits a row");

    // Envelope side, same traceparent.
    let envelope = midedit_envelope(Some(TRACEPARENT));
    let mirror = envelope
        .mirror
        .as_ref()
        .expect("mid-edit envelope has a mirror");

    // The join key matches byte-for-byte across the two surfaces.
    assert_eq!(mirror.gate_eval_id.as_deref(), Some(PARENT_ID));
    assert_eq!(row.gate_eval_id, PARENT_ID);
    assert_eq!(
        mirror.gate_eval_id.as_deref(),
        Some(row.gate_eval_id.as_str()),
        "subscriber must be able to join mirror.gate_eval_id == row.gate_eval_id",
    );

    // Field map holds across the joined pair.
    assert_eq!(mirror.path, Some(MirrorPath::MidEdit));
    assert_eq!(mirror.decision, ControlDecision::Warn);
    assert_eq!(row.enforcement, Enforcement::Warning);
    assert_eq!(envelope.correlation.session_id.as_deref(), Some(SESSION_ID));
    assert_eq!(row.session_id, SESSION_ID);
    assert_eq!(row.inputs.changed_files, vec![FILE.to_string()]);
}

#[test]
fn envelope_omits_join_key_without_traceparent() {
    // Forward-compat: with no traceparent the field is None and is
    // omitted from the wire entirely (no spurious random id that could
    // never match a row), so pre-MLP2-008 consumers are byte-unaffected.
    let envelope = midedit_envelope(None);
    let mirror = envelope
        .mirror
        .as_ref()
        .expect("mid-edit envelope has a mirror");
    assert_eq!(mirror.gate_eval_id, None);

    let json = serde_json::to_value(&envelope).expect("envelope serialises");
    assert!(
        json["mirror"].get("gate_eval_id").is_none(),
        "absent join key must not appear on the wire, got: {}",
        json["mirror"],
    );
}
