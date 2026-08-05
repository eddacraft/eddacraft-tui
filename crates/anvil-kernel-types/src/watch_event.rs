//! Wire-level NDJSON envelope for `anvil --json watch` — `anvil.watch.event.v1`.
//!
//! Defined per `docs/specs/watch-output-contract.md` (WOUT-001). The CLI in
//! `crates/anvil-cli/src/commands/watch.rs` serialises one of these per line
//! on stdout in JSON mode; consumers parse the stream as NDJSON.
//!
//! This shape is distinct from [`crate::EngineEvent`], which is the in-process
//! type emitted by the kernel onto an `mpsc::channel` — that type uses
//! externally tagged serde and carries an internal `EngineId` field that is
//! intentionally not part of the public consumer contract.

use serde::{Deserialize, Serialize};

use crate::{Diagnostic, EngineEvent, ErrorCode, EventPayload};

/// Current outer schema version string for the watch NDJSON envelope.
///
/// Bumps to `anvil.watch.event.v2` only on breaking changes; additive
/// evolution (new optional payload fields, new `event_type` values) stays
/// on `v1` per the spec's versioning rules.
pub const WATCH_EVENT_SCHEMA_VERSION: &str = "anvil.watch.event.v1";

/// Wire-level discriminator for watch events. Lower-case strings on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchEventType {
    Progress,
    Snapshot,
    Violation,
    Error,
    ActionResult,
}

/// Observable result of a dispatched watch action.
///
/// `action` is the structurally unique required field for the untagged WOUT-v1
/// payload variant. `exit_code` is absent when the child did not exit normally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchActionResult {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daemon_verdict: Option<WatchDaemonVerdict>,
}

/// Structured save-time daemon evidence attached to an action result.
///
/// Wire vocabulary remains string-valued so a newer daemon can add assurance
/// states, reasons, coverage values, or check families without breaking a
/// WOUT-v1 consumer. Consumers must interpret state and coverage independently
/// from the child exit code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchDaemonVerdict {
    pub assurance_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assurance_reason: Option<String>,
    pub coverage: String,
    pub check_families: Vec<String>,
    pub finding_count: u64,
    pub diagnostics: Vec<Diagnostic>,
}

/// Wire-level event payload. `serde(untagged)` so each variant serialises
/// directly to its inner object — consumers see `payload.phase`, not
/// `payload.Progress.phase`.
///
/// The variant order matters for deserialisation: serde tries each in turn
/// and accepts the first match. The variants here are structurally
/// distinguishable by required field names (`phase` vs `node_count` vs
/// `policy_id` vs `code` vs `action`), so ambiguity is not possible within v1.
///
/// **WOUT v1 forward-compat rule (binding):** any new variant added to
/// this enum within v1 MUST introduce at least one required field name
/// not present in any other variant. Without that guard, serde's
/// first-match untagged deserialisation would silently misroute a new
/// payload into an existing variant. The outer `event_type` field stays
/// authoritative on the wire — consumers MUST dispatch on `event_type`
/// rather than relying on `WatchEventPayload`'s structural matching for
/// future variants. See `docs/specs/watch-output-contract.md` →
/// "Versioning and Non-Goals" for the spec-side rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WatchEventPayload {
    Progress {
        phase: String,
        current: u64,
        total: u64,
    },
    Snapshot {
        node_count: u64,
        edge_count: u64,
        files_watched: u64,
    },
    Violation {
        policy_id: String,
        file: String,
        symbol: String,
        message: String,
    },
    Error {
        code: ErrorCode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file: Option<String>,
        message: String,
        recoverable: bool,
    },
    ActionResult(WatchActionResult),
}

/// `anvil.watch.event.v1` NDJSON envelope.
///
/// Per the spec, `schema_version` is always [`WATCH_EVENT_SCHEMA_VERSION`]
/// within v1. The wire format keeps it as an owned `String` (not a
/// `&'static str`) so consumers using `serde_json::from_str` round-trip
/// without lifetime gymnastics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchEventEnvelope {
    pub schema_version: String,
    pub seq: u64,
    pub timestamp: String,
    pub event_type: WatchEventType,
    pub payload: WatchEventPayload,
}

impl WatchEventEnvelope {
    /// Build a v1 action-result envelope using the caller's sequence number.
    #[must_use]
    pub fn from_action_result(
        seq: u64,
        timestamp: impl Into<String>,
        result: WatchActionResult,
    ) -> Self {
        Self {
            schema_version: WATCH_EVENT_SCHEMA_VERSION.to_string(),
            seq,
            timestamp: timestamp.into(),
            event_type: WatchEventType::ActionResult,
            payload: WatchEventPayload::ActionResult(result),
        }
    }

    /// Build the v1 wire envelope from a kernel [`EngineEvent`]. The wire
    /// `seq` is taken from the engine event so consumers can detect
    /// dropped or reordered lines.
    #[must_use]
    pub fn from_engine_event(event: &EngineEvent) -> Self {
        let (event_type, payload) = match &event.payload {
            EventPayload::Progress {
                phase,
                current,
                total,
            } => (
                WatchEventType::Progress,
                WatchEventPayload::Progress {
                    phase: phase.clone(),
                    current: *current,
                    total: *total,
                },
            ),
            EventPayload::Snapshot {
                node_count,
                edge_count,
                files_watched,
                // `changed_path` is an internal CLI dispatch hint (RLB-007);
                // it is deliberately dropped here so the `anvil.watch.event.v1`
                // Snapshot wire shape stays exactly {node_count, edge_count,
                // files_watched}. Pinned by `snapshot_wire_shape_omits_changed_path`.
                changed_path: _,
            } => (
                WatchEventType::Snapshot,
                WatchEventPayload::Snapshot {
                    node_count: *node_count,
                    edge_count: *edge_count,
                    files_watched: *files_watched,
                },
            ),
            EventPayload::Violation {
                policy_id,
                file,
                symbol,
                message,
            } => (
                WatchEventType::Violation,
                WatchEventPayload::Violation {
                    policy_id: policy_id.clone(),
                    file: file.clone(),
                    symbol: symbol.clone(),
                    message: message.clone(),
                },
            ),
            EventPayload::Error(err) => (
                WatchEventType::Error,
                WatchEventPayload::Error {
                    code: err.code,
                    file: err.file.clone(),
                    message: err.message.clone(),
                    recoverable: err.recoverable,
                },
            ),
        };

        Self {
            schema_version: WATCH_EVENT_SCHEMA_VERSION.to_string(),
            seq: event.seq,
            timestamp: event.timestamp.clone(),
            event_type,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::KnownMode;
    use crate::{
        Category, Diagnostic, DiagnosticSource, EngineId, ErrorPayload, EventType, Location, Mode,
        Severity,
    };
    use serde_json::Value;

    fn make_event(payload: EventPayload, event_type: EventType, seq: u64) -> EngineEvent {
        // Build through the deriving constructor so the helper can never
        // mint an event whose event_type disagrees with its payload; the
        // explicit `event_type` is kept as a consistency assertion so a
        // call site that passes an `event_type` not matching its payload
        // fails loudly instead of silently bypassing the invariant via a
        // struct literal.
        let event = EngineEvent::new(
            seq,
            "2026-05-14T10:21:30Z".to_string(),
            EngineId::Rust,
            payload,
        );
        assert_eq!(
            event.event_type, event_type,
            "make_event called with an event_type that disagrees with the payload variant"
        );
        event
    }

    fn sample_diagnostic() -> Diagnostic {
        Diagnostic::new(
            "diag-watch-antipattern",
            Severity::Warning,
            "Unchecked escape hatch",
            Location {
                file: "src/lib.rs".into(),
                line: Some(12),
                column: Some(5),
                end_line: None,
                end_column: None,
            },
            Category::Antipattern,
            DiagnosticSource {
                rule_id: "antipattern-ignore".into(),
                source_module: "anvil-checks::antipattern".into(),
            },
            Mode::known(KnownMode::SaveTime),
        )
    }

    #[test]
    fn action_result_with_daemon_verdict_is_parseable_and_propagates_seq() {
        let result = WatchActionResult {
            action: "check".into(),
            exit_code: Some(0),
            duration_ms: 17,
            error_detail: None,
            daemon_verdict: Some(WatchDaemonVerdict {
                assurance_state: "clean".into(),
                assurance_reason: None,
                coverage: "certified".into(),
                check_families: vec!["antipattern".into()],
                finding_count: 1,
                diagnostics: vec![sample_diagnostic()],
            }),
        };

        let envelope = WatchEventEnvelope::from_action_result(42, "2026-08-05T10:21:30Z", result);
        let json = serde_json::to_string(&envelope).expect("serialise action result");
        let value: Value = serde_json::from_str(&json).expect("parse action result JSON");
        let round_trip: WatchEventEnvelope =
            serde_json::from_str(&json).expect("round-trip action result");

        assert_eq!(round_trip, envelope);
        assert_eq!(value["schema_version"], WATCH_EVENT_SCHEMA_VERSION);
        assert_eq!(value["seq"], 42);
        assert_eq!(value["event_type"], "action_result");
        assert_eq!(value["payload"]["action"], "check");
        assert_eq!(
            value["payload"]["daemon_verdict"]["check_families"],
            serde_json::json!(["antipattern"])
        );
        assert_eq!(value["payload"]["daemon_verdict"]["finding_count"], 1);
        assert_eq!(
            value["payload"]["daemon_verdict"]["diagnostics"][0]["schema_version"],
            "anvil.diagnostic.v1"
        );
        assert_eq!(
            value["payload"]["daemon_verdict"]["diagnostics"][0]["category"],
            "antipattern"
        );
    }

    #[test]
    fn stale_partial_daemon_verdict_does_not_masquerade_as_pass() {
        let envelope = WatchEventEnvelope::from_action_result(
            7,
            "2026-08-05T10:21:31Z",
            WatchActionResult {
                action: "check".into(),
                // The scoped daemon may return no findings while its evidence
                // remains stale/partial. Exit 0 must not erase those fields.
                exit_code: Some(0),
                duration_ms: 9,
                error_detail: None,
                daemon_verdict: Some(WatchDaemonVerdict {
                    assurance_state: "stale".into(),
                    assurance_reason: Some("cross-file-resolution-needed".into()),
                    coverage: "partial".into(),
                    check_families: vec!["antipattern".into()],
                    finding_count: 0,
                    diagnostics: Vec::new(),
                }),
            },
        );

        let value = serde_json::to_value(&envelope).expect("serialise degraded verdict");
        assert_eq!(value["payload"]["exit_code"], 0);
        assert_eq!(
            value["payload"]["daemon_verdict"]["assurance_state"],
            "stale"
        );
        assert_eq!(
            value["payload"]["daemon_verdict"]["assurance_reason"],
            "cross-file-resolution-needed"
        );
        assert_eq!(value["payload"]["daemon_verdict"]["coverage"], "partial");
        assert_eq!(
            value["payload"]["daemon_verdict"]["check_families"],
            serde_json::json!(["antipattern"])
        );
        assert_eq!(value["payload"]["daemon_verdict"]["finding_count"], 0);
    }

    #[test]
    fn action_result_without_daemon_verdict_omits_optional_fields() {
        let envelope = WatchEventEnvelope::from_action_result(
            u64::MAX,
            "2026-08-05T10:21:32Z",
            WatchActionResult {
                action: "test".into(),
                exit_code: None,
                duration_ms: 23,
                error_detail: Some("cancelled".into()),
                daemon_verdict: None,
            },
        );

        let value = serde_json::to_value(&envelope).expect("serialise generic action result");
        assert_eq!(value["seq"], u64::MAX);
        assert_eq!(value["payload"]["action"], "test");
        assert_eq!(value["payload"]["duration_ms"], 23);
        assert_eq!(value["payload"]["error_detail"], "cancelled");
        assert!(value["payload"].get("exit_code").is_none());
        assert!(value["payload"].get("daemon_verdict").is_none());
    }

    #[test]
    fn schema_version_constant_matches_spec() {
        assert_eq!(WATCH_EVENT_SCHEMA_VERSION, "anvil.watch.event.v1");
    }

    #[test]
    fn envelope_emits_schema_version_seq_timestamp_event_type_and_payload() {
        let event = make_event(
            EventPayload::Progress {
                phase: "initial-scan".into(),
                current: 12,
                total: 100,
            },
            EventType::Progress,
            0,
        );
        let envelope = WatchEventEnvelope::from_engine_event(&event);
        let value: Value = serde_json::to_value(&envelope).expect("serialise");

        assert_eq!(value["schema_version"], "anvil.watch.event.v1");
        assert_eq!(value["seq"], 0);
        assert_eq!(value["timestamp"], "2026-05-14T10:21:30Z");
        assert_eq!(value["event_type"], "progress");
        // Payload should be flat (untagged), not wrapped in a variant tag.
        assert_eq!(value["payload"]["phase"], "initial-scan");
        assert_eq!(value["payload"]["current"], 12);
        assert_eq!(value["payload"]["total"], 100);
        // Crucially, no internal `engine` field leaks onto the wire.
        assert!(value.get("engine").is_none());
    }

    #[test]
    fn snapshot_payload_round_trips() {
        let event = make_event(
            EventPayload::Snapshot {
                node_count: 312,
                edge_count: 845,
                files_watched: 64,
                changed_path: None,
            },
            EventType::Snapshot,
            3,
        );
        let envelope = WatchEventEnvelope::from_engine_event(&event);
        let json = serde_json::to_string(&envelope).expect("serialise");
        let back: WatchEventEnvelope = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, envelope);
    }

    /// RLB-007: the internal `EventPayload::Snapshot.changed_path` dispatch
    /// hint MUST NOT leak onto the `anvil.watch.event.v1` wire envelope. A
    /// snapshot carrying a `changed_path` still serialises to exactly
    /// `{node_count, edge_count, files_watched}` so existing NDJSON consumers
    /// see an unchanged Snapshot shape.
    #[test]
    fn snapshot_wire_shape_omits_changed_path() {
        let event = make_event(
            EventPayload::Snapshot {
                node_count: 1,
                edge_count: 2,
                files_watched: 3,
                changed_path: Some("/repo/src/changed.ts".to_string()),
            },
            EventType::Snapshot,
            7,
        );
        let envelope = WatchEventEnvelope::from_engine_event(&event);
        let json = serde_json::to_string(&envelope).expect("serialise");
        let value: Value = serde_json::from_str(&json).expect("parse own output");
        assert!(
            value["payload"].get("changed_path").is_none(),
            "changed_path must not appear on the wire envelope: {value}"
        );
        assert_eq!(value["payload"]["node_count"], 1);
        assert_eq!(value["payload"]["edge_count"], 2);
        assert_eq!(value["payload"]["files_watched"], 3);
        // Exactly the three documented fields, nothing more.
        assert_eq!(
            value["payload"].as_object().map(serde_json::Map::len),
            Some(3),
            "Snapshot wire payload must have exactly 3 fields: {value}"
        );
    }

    /// The spec at `docs/specs/watch-output-contract.md` documents that
    /// `symbol` may be empty for file-scope rules. The empty string MUST
    /// stay on the wire as an empty string — not be omitted, not be
    /// `null`. A future refactor that adds `skip_serializing_if =
    /// "String::is_empty"` would break consumers that assert
    /// `payload.symbol` is always a string.
    #[test]
    fn violation_payload_empty_symbol_serialises_as_empty_string() {
        let event = make_event(
            EventPayload::Violation {
                policy_id: "file-scope".into(),
                file: "src/lib.ts".into(),
                symbol: String::new(),
                message: "file-scope rule".into(),
            },
            EventType::Violation,
            1,
        );
        let envelope = WatchEventEnvelope::from_engine_event(&event);
        let value: Value = serde_json::to_value(&envelope).expect("serialise");
        assert_eq!(
            value["payload"]["symbol"], "",
            "empty symbol must round-trip as an empty string, not be omitted or null"
        );
    }

    #[test]
    fn violation_payload_serialises_required_fields() {
        let event = make_event(
            EventPayload::Violation {
                policy_id: "no-circular-deps".into(),
                file: "src/main.ts".into(),
                symbol: "App".into(),
                message: "Circular dependency detected".into(),
            },
            EventType::Violation,
            7,
        );
        let envelope = WatchEventEnvelope::from_engine_event(&event);
        let value: Value = serde_json::to_value(&envelope).expect("serialise");

        assert_eq!(value["event_type"], "violation");
        assert_eq!(value["payload"]["policy_id"], "no-circular-deps");
        assert_eq!(value["payload"]["file"], "src/main.ts");
        assert_eq!(value["payload"]["symbol"], "App");
        assert_eq!(value["payload"]["message"], "Circular dependency detected");
    }

    #[test]
    fn error_payload_omits_file_when_absent() {
        let event = make_event(
            EventPayload::Error(ErrorPayload {
                code: ErrorCode::Internal,
                file: None,
                message: "boom".into(),
                recoverable: false,
            }),
            EventType::Error,
            1,
        );
        let envelope = WatchEventEnvelope::from_engine_event(&event);
        let value: Value = serde_json::to_value(&envelope).expect("serialise");

        assert_eq!(value["event_type"], "error");
        assert_eq!(value["payload"]["code"], "Internal");
        assert_eq!(value["payload"]["message"], "boom");
        assert_eq!(value["payload"]["recoverable"], false);
        assert!(
            value["payload"].get("file").is_none(),
            "file should be omitted when None, got: {value}"
        );
    }

    #[test]
    fn error_payload_keeps_file_when_present() {
        let event = make_event(
            EventPayload::Error(ErrorPayload {
                code: ErrorCode::ParseError,
                file: Some("src/broken.ts".into()),
                message: "Unexpected token".into(),
                recoverable: true,
            }),
            EventType::Error,
            9,
        );
        let envelope = WatchEventEnvelope::from_engine_event(&event);
        let value: Value = serde_json::to_value(&envelope).expect("serialise");

        assert_eq!(value["payload"]["code"], "ParseError");
        assert_eq!(value["payload"]["file"], "src/broken.ts");
        assert_eq!(value["payload"]["recoverable"], true);
    }

    #[test]
    fn seq_propagates_from_engine_event() {
        for seq in [0, 1, 42, u64::MAX] {
            let event = make_event(
                EventPayload::Progress {
                    phase: "x".into(),
                    current: 0,
                    total: 1,
                },
                EventType::Progress,
                seq,
            );
            let envelope = WatchEventEnvelope::from_engine_event(&event);
            assert_eq!(envelope.seq, seq);
        }
    }

    /// WOUT-001 alignment guarantee: for every `EngineEvent` the
    /// kernel emits, `from_engine_event` produces an envelope whose
    /// outer `event_type` discriminator matches the payload's variant.
    /// Consumers MUST be able to trust this pairing — see the spec's
    /// "Versioning and Non-Goals" section.
    #[test]
    fn event_type_and_payload_variant_always_agree() {
        let cases: &[(EventType, EventPayload, WatchEventType)] = &[
            (
                EventType::Progress,
                EventPayload::Progress {
                    phase: "p".into(),
                    current: 0,
                    total: 1,
                },
                WatchEventType::Progress,
            ),
            (
                EventType::Snapshot,
                EventPayload::Snapshot {
                    node_count: 0,
                    edge_count: 0,
                    files_watched: 0,
                    changed_path: None,
                },
                WatchEventType::Snapshot,
            ),
            (
                EventType::Violation,
                EventPayload::Violation {
                    policy_id: "p".into(),
                    file: "f".into(),
                    symbol: String::new(),
                    message: "m".into(),
                },
                WatchEventType::Violation,
            ),
            (
                EventType::Error,
                EventPayload::Error(crate::ErrorPayload {
                    code: ErrorCode::Internal,
                    file: None,
                    message: "e".into(),
                    recoverable: false,
                }),
                WatchEventType::Error,
            ),
        ];
        for (event_type, payload, expected) in cases {
            let event = make_event(payload.clone(), *event_type, 0);
            let envelope = WatchEventEnvelope::from_engine_event(&event);
            assert_eq!(
                envelope.event_type, *expected,
                "from_engine_event must keep event_type and payload variant in lockstep"
            );
            // Also assert the discriminant of the payload matches —
            // belt-and-braces against a future refactor that produces
            // a mismatched envelope.
            let payload_variant = match envelope.payload {
                WatchEventPayload::Progress { .. } => WatchEventType::Progress,
                WatchEventPayload::Snapshot { .. } => WatchEventType::Snapshot,
                WatchEventPayload::Violation { .. } => WatchEventType::Violation,
                WatchEventPayload::Error { .. } => WatchEventType::Error,
                WatchEventPayload::ActionResult(_) => WatchEventType::ActionResult,
            };
            assert_eq!(
                payload_variant, envelope.event_type,
                "payload variant must agree with envelope.event_type"
            );
        }
    }

    /// WOUT-001 wire-string pinning. `ErrorCode` does NOT use
    /// `#[serde(rename_all = ...)]`, so each variant serialises to its
    /// Rust name in `PascalCase`. The `anvil.watch.event.v1` contract
    /// publishes those exact strings; this test fails loudly the moment
    /// anyone tries to "normalise" the casing.
    #[test]
    fn error_code_wire_strings_are_pascal_case_and_pinned() {
        for (variant, wire) in [
            (ErrorCode::ParseError, "\"ParseError\""),
            (ErrorCode::ConfigError, "\"ConfigError\""),
            (ErrorCode::Internal, "\"Internal\""),
        ] {
            let json = serde_json::to_string(&variant).expect("serialise");
            assert_eq!(
                json, wire,
                "ErrorCode::{variant:?} must serialise to {wire} — \
                 watch v1 consumers parse this string verbatim and any \
                 rename is a breaking change."
            );
        }
    }

    #[test]
    fn event_type_serialises_snake_case() {
        for (variant, expected) in [
            (WatchEventType::Progress, "progress"),
            (WatchEventType::Snapshot, "snapshot"),
            (WatchEventType::Violation, "violation"),
            (WatchEventType::Error, "error"),
            (WatchEventType::ActionResult, "action_result"),
        ] {
            let v = serde_json::to_value(variant).expect("serialise");
            assert_eq!(v, expected, "{variant:?} should serialise to {expected}");
            let back: WatchEventType = serde_json::from_value(v).expect("deserialise");
            assert_eq!(back, variant);
        }
    }
}
