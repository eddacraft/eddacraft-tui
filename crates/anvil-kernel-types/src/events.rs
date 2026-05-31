use serde::{Deserialize, Serialize};

use crate::EngineId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Progress,
    Snapshot,
    Violation,
    Error,
}

/// A kernel engine event.
///
/// `event_type` is redundant with `payload`'s variant — both encode the
/// same kind — but the field stays on the wire because the watch output
/// contract (`anvil.watch.event.v1`) treats it as the authoritative
/// dispatch tag. To keep the redundant pair from disagreeing, build
/// events with [`EngineEvent::new`] (which derives `event_type` from the
/// payload) and rely on the validating `Deserialize` impl, which rejects
/// any wire value whose `event_type` does not match its `payload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "EngineEventRepr")]
pub struct EngineEvent {
    pub event_type: EventType,
    pub seq: u64,
    pub timestamp: String,
    pub engine: EngineId,
    pub payload: EventPayload,
}

impl EngineEvent {
    /// Construct an event with `event_type` derived from `payload`, so the
    /// two can never disagree.
    #[must_use]
    pub fn new(seq: u64, timestamp: String, engine: EngineId, payload: EventPayload) -> Self {
        Self {
            event_type: payload.event_type(),
            seq,
            timestamp,
            engine,
            payload,
        }
    }
}

/// Wire shadow of [`EngineEvent`] used only to validate that the redundant
/// `event_type` tag agrees with the payload variant on deserialise.
#[derive(Deserialize)]
struct EngineEventRepr {
    event_type: EventType,
    seq: u64,
    timestamp: String,
    engine: EngineId,
    payload: EventPayload,
}

impl TryFrom<EngineEventRepr> for EngineEvent {
    type Error = String;

    fn try_from(repr: EngineEventRepr) -> Result<Self, Self::Error> {
        let derived = repr.payload.event_type();
        if repr.event_type != derived {
            return Err(format!(
                "EngineEvent event_type {:?} does not match payload variant {derived:?}",
                repr.event_type
            ));
        }
        Ok(Self {
            event_type: repr.event_type,
            seq: repr.seq,
            timestamp: repr.timestamp,
            engine: repr.engine,
            payload: repr.payload,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
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
    Error(ErrorPayload),
}

impl EventPayload {
    /// The [`EventType`] tag implied by this payload's variant. The
    /// single source of truth for the otherwise-redundant
    /// `EngineEvent::event_type` field.
    #[must_use]
    pub fn event_type(&self) -> EventType {
        match self {
            EventPayload::Progress { .. } => EventType::Progress,
            EventPayload::Snapshot { .. } => EventType::Snapshot,
            EventPayload::Violation { .. } => EventType::Violation,
            EventPayload::Error(_) => EventType::Error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub file: Option<String>,
    pub message: String,
    pub recoverable: bool,
}

/// Closed set of engine error codes. The variant names are pinned to the
/// wire — `ErrorCode::ParseError` MUST serialise to `"ParseError"` etc.
/// The watch output contract (`anvil.watch.event.v1`) and downstream
/// consumers parse this as a string-typed `code` field; renaming or
/// adding `#[serde(rename_all = ...)]` here is a breaking change for
/// every published wire surface that includes an error payload.
/// Pinning test:
/// [`crate::watch_event::tests::error_code_wire_strings_are_pascal_case_and_pinned`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    ParseError,
    ConfigError,
    Internal,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress_event() -> EngineEvent {
        EngineEvent {
            event_type: EventType::Progress,
            seq: 1,
            timestamp: "2026-03-19T00:00:00Z".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Progress {
                phase: "parsing".into(),
                current: 10,
                total: 100,
            },
        }
    }

    fn snapshot_event() -> EngineEvent {
        EngineEvent {
            event_type: EventType::Snapshot,
            seq: 2,
            timestamp: "2026-03-19T00:00:01Z".into(),
            engine: EngineId::Legacy,
            payload: EventPayload::Snapshot {
                node_count: 50,
                edge_count: 120,
                files_watched: 30,
            },
        }
    }

    fn violation_event() -> EngineEvent {
        EngineEvent {
            event_type: EventType::Violation,
            seq: 3,
            timestamp: "2026-03-19T00:00:02Z".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Violation {
                policy_id: "no-circular-deps".into(),
                file: "src/main.ts".into(),
                symbol: "App".into(),
                message: "Circular dependency detected".into(),
            },
        }
    }

    fn error_event() -> EngineEvent {
        EngineEvent {
            event_type: EventType::Error,
            seq: 4,
            timestamp: "2026-03-19T00:00:03Z".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Error(ErrorPayload {
                code: ErrorCode::ParseError,
                file: Some("broken.ts".into()),
                message: "Unexpected token".into(),
                recoverable: true,
            }),
        }
    }

    // --- EventType ---

    #[test]
    fn event_type_all_variants_distinct() {
        let variants = [
            EventType::Progress,
            EventType::Snapshot,
            EventType::Violation,
            EventType::Error,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn event_type_copy_semantics() {
        let a = EventType::Violation;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn event_type_serde_round_trip() {
        for variant in [
            EventType::Progress,
            EventType::Snapshot,
            EventType::Violation,
            EventType::Error,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: EventType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    // --- ErrorCode ---

    #[test]
    fn error_code_all_variants_distinct() {
        let variants = [
            ErrorCode::ParseError,
            ErrorCode::ConfigError,
            ErrorCode::Internal,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn error_code_copy_semantics() {
        let a = ErrorCode::Internal;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn error_code_serde_round_trip() {
        for code in [
            ErrorCode::ParseError,
            ErrorCode::ConfigError,
            ErrorCode::Internal,
        ] {
            let json = serde_json::to_string(&code).unwrap();
            let back: ErrorCode = serde_json::from_str(&json).unwrap();
            assert_eq!(code, back);
        }
    }

    #[test]
    fn error_code_invalid_variant_fails() {
        let result = serde_json::from_str::<ErrorCode>("\"Timeout\"");
        assert!(result.is_err());
    }

    // --- ErrorPayload ---

    #[test]
    fn error_payload_with_file() {
        let payload = ErrorPayload {
            code: ErrorCode::ParseError,
            file: Some("src/lib.ts".into()),
            message: "Syntax error".into(),
            recoverable: false,
        };
        assert_eq!(payload.file.as_deref(), Some("src/lib.ts"));
        assert!(!payload.recoverable);
    }

    #[test]
    fn error_payload_without_file() {
        let payload = ErrorPayload {
            code: ErrorCode::Internal,
            file: None,
            message: "Unknown failure".into(),
            recoverable: false,
        };
        assert!(payload.file.is_none());
    }

    #[test]
    fn error_payload_serde_round_trip() {
        let payload = ErrorPayload {
            code: ErrorCode::ConfigError,
            file: Some("anvil.config.ts".into()),
            message: "Invalid config".into(),
            recoverable: true,
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: ErrorPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.code, payload.code);
        assert_eq!(back.file, payload.file);
        assert_eq!(back.message, payload.message);
        assert_eq!(back.recoverable, payload.recoverable);
    }

    #[test]
    fn error_payload_none_file_serialises_as_null() {
        let payload = ErrorPayload {
            code: ErrorCode::Internal,
            file: None,
            message: "boom".into(),
            recoverable: false,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("null"));
    }

    // --- EventPayload ---

    #[test]
    fn event_payload_progress_variant() {
        let payload = EventPayload::Progress {
            phase: "analysis".into(),
            current: 5,
            total: 10,
        };
        match &payload {
            EventPayload::Progress {
                phase,
                current,
                total,
            } => {
                assert_eq!(phase, "analysis");
                assert_eq!(*current, 5);
                assert_eq!(*total, 10);
            }
            _ => panic!("expected Progress variant"),
        }
    }

    #[test]
    fn event_payload_snapshot_variant() {
        let payload = EventPayload::Snapshot {
            node_count: 100,
            edge_count: 200,
            files_watched: 50,
        };
        match &payload {
            EventPayload::Snapshot {
                node_count,
                edge_count,
                files_watched,
            } => {
                assert_eq!(*node_count, 100);
                assert_eq!(*edge_count, 200);
                assert_eq!(*files_watched, 50);
            }
            _ => panic!("expected Snapshot variant"),
        }
    }

    #[test]
    fn event_payload_violation_variant() {
        let payload = EventPayload::Violation {
            policy_id: "p1".into(),
            file: "f.ts".into(),
            symbol: "s".into(),
            message: "bad".into(),
        };
        match &payload {
            EventPayload::Violation {
                policy_id, message, ..
            } => {
                assert_eq!(policy_id, "p1");
                assert_eq!(message, "bad");
            }
            _ => panic!("expected Violation variant"),
        }
    }

    #[test]
    fn event_payload_serde_round_trip_all_variants() {
        let payloads = vec![
            EventPayload::Progress {
                phase: "init".into(),
                current: 0,
                total: 1,
            },
            EventPayload::Snapshot {
                node_count: 0,
                edge_count: 0,
                files_watched: 0,
            },
            EventPayload::Violation {
                policy_id: "id".into(),
                file: "f".into(),
                symbol: "s".into(),
                message: "m".into(),
            },
            EventPayload::Error(ErrorPayload {
                code: ErrorCode::Internal,
                file: None,
                message: "err".into(),
                recoverable: false,
            }),
        ];
        for payload in &payloads {
            let json = serde_json::to_string(payload).unwrap();
            let back: EventPayload = serde_json::from_str(&json).unwrap();
            // Verify round-trip succeeds (structural equality via debug)
            assert_eq!(format!("{payload:?}"), format!("{back:?}"));
        }
    }

    // --- EngineEvent ---

    #[test]
    fn engine_event_progress_round_trip() {
        let event = progress_event();
        let json = serde_json::to_string(&event).unwrap();
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event_type, EventType::Progress);
        assert_eq!(back.seq, 1);
        assert_eq!(back.engine, EngineId::Rust);
    }

    #[test]
    fn engine_event_snapshot_round_trip() {
        let event = snapshot_event();
        let json = serde_json::to_string(&event).unwrap();
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event_type, EventType::Snapshot);
        assert_eq!(back.engine, EngineId::Legacy);
    }

    #[test]
    fn engine_event_violation_round_trip() {
        let event = violation_event();
        let json = serde_json::to_string(&event).unwrap();
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event_type, EventType::Violation);
        assert_eq!(back.seq, 3);
    }

    #[test]
    fn engine_event_error_round_trip() {
        let event = error_event();
        let json = serde_json::to_string(&event).unwrap();
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event_type, EventType::Error);
        assert_eq!(back.seq, 4);
    }

    #[test]
    fn engine_event_clone_is_independent() {
        let event = progress_event();
        let mut cloned = event.clone();
        cloned.seq = 999;
        assert_eq!(event.seq, 1);
        assert_eq!(cloned.seq, 999);
    }

    #[test]
    fn engine_event_debug_format() {
        let dbg = format!("{:?}", progress_event());
        assert!(dbg.contains("Progress"));
        assert!(dbg.contains("parsing"));
    }

    #[test]
    fn engine_event_zero_seq() {
        let mut event = progress_event();
        event.seq = 0;
        let json = serde_json::to_string(&event).unwrap();
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.seq, 0);
    }

    #[test]
    fn engine_event_max_seq() {
        let mut event = progress_event();
        event.seq = u64::MAX;
        let json = serde_json::to_string(&event).unwrap();
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.seq, u64::MAX);
    }

    // --- Deserialisation error cases ---

    #[test]
    fn invalid_event_type_fails() {
        let result = serde_json::from_str::<EventType>("\"Warning\"");
        assert!(result.is_err());
    }

    #[test]
    fn engine_event_missing_payload_fails() {
        let json = r#"{"event_type":"Progress","seq":1,"timestamp":"t","engine":"Rust"}"#;
        let result = serde_json::from_str::<EngineEvent>(json);
        assert!(result.is_err());
    }

    #[test]
    fn engine_event_new_derives_event_type_from_payload() {
        let event = EngineEvent::new(
            7,
            "t".into(),
            EngineId::Rust,
            EventPayload::Violation {
                policy_id: "p".into(),
                file: "f".into(),
                symbol: "s".into(),
                message: "m".into(),
            },
        );
        assert_eq!(event.event_type, EventType::Violation);
    }

    #[test]
    fn engine_event_mismatched_event_type_fails_on_deserialise() {
        // event_type says Progress, payload is a Violation — the redundant
        // tags disagree, so the validating Deserialize must reject it
        // rather than admit an internally-inconsistent event.
        let json = r#"{
            "event_type":"Progress",
            "seq":1,
            "timestamp":"t",
            "engine":"Rust",
            "payload":{"Violation":{"policy_id":"p","file":"f","symbol":"s","message":"m"}}
        }"#;
        let result = serde_json::from_str::<EngineEvent>(json);
        assert!(
            result.is_err(),
            "mismatched event_type/payload must be rejected, got {result:?}"
        );
    }

    #[test]
    fn engine_event_matching_event_type_round_trips_via_new() {
        // A constructed event always agrees with its payload, so it
        // survives a JSON round-trip through the validating deserialiser.
        let event = EngineEvent::new(
            9,
            "t".into(),
            EngineId::Legacy,
            EventPayload::Snapshot {
                node_count: 1,
                edge_count: 2,
                files_watched: 3,
            },
        );
        let json = serde_json::to_string(&event).unwrap();
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event_type, EventType::Snapshot);
    }
}
