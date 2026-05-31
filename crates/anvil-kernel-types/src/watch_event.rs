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

use crate::{EngineEvent, ErrorCode, EventPayload};

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
}

/// Wire-level event payload. `serde(untagged)` so each variant serialises
/// directly to its inner object — consumers see `payload.phase`, not
/// `payload.Progress.phase`.
///
/// The variant order matters for deserialisation: serde tries each in turn
/// and accepts the first match. The variants here are structurally
/// distinguishable by required field names (`phase` vs `node_count` vs
/// `policy_id` vs `code`), so ambiguity is not possible within v1.
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
    use crate::{EngineId, ErrorPayload, EventType};
    use serde_json::Value;

    fn make_event(payload: EventPayload, event_type: EventType, seq: u64) -> EngineEvent {
        // Build through the deriving constructor so the helper can never
        // mint an event whose event_type disagrees with its payload; the
        // explicit `event_type` is kept as a consistency assertion so a
        // future swapped-arg call site fails loudly instead of silently
        // bypassing the invariant via a struct literal.
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
            },
            EventType::Snapshot,
            3,
        );
        let envelope = WatchEventEnvelope::from_engine_event(&event);
        let json = serde_json::to_string(&envelope).expect("serialise");
        let back: WatchEventEnvelope = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, envelope);
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
        ] {
            let v = serde_json::to_value(variant).expect("serialise");
            assert_eq!(v, expected, "{variant:?} should serialise to {expected}");
            let back: WatchEventType = serde_json::from_value(v).expect("deserialise");
            assert_eq!(back, variant);
        }
    }
}
