//! Integration tests validating cross-type relationships and invariants
//! across the anvil-kernel-types crate.

use anvil_kernel_types::{
    EdgeType, EngineEvent, EngineId, ErrorCode, ErrorPayload, EventPayload, EventType, SymbolEdge,
    SymbolKind, SymbolNode, TrustLevel, Visibility,
};

// -- Graph nodes reference trust levels coherently --

#[test]
fn symbol_node_carries_trust_level() {
    let node = SymbolNode {
        id: 1,
        kind: SymbolKind::Module,
        name: "auth".into(),
        visibility: Visibility::Internal,
        file: "src/auth/mod.ts".into(),
        trust_level: TrustLevel::Privileged,
    };
    assert_eq!(node.trust_level, TrustLevel::Privileged);
}

#[test]
fn default_trust_level_for_new_nodes() {
    let node = SymbolNode {
        id: 0,
        kind: SymbolKind::Function,
        name: "unknown_fn".into(),
        visibility: Visibility::Public,
        file: "generated.ts".into(),
        trust_level: TrustLevel::default(),
    };
    assert_eq!(node.trust_level, TrustLevel::Unknown);
}

// -- Events carry correct engine identity --

#[test]
fn engine_event_binds_engine_id_to_payload() {
    let event = EngineEvent {
        event_type: EventType::Violation,
        seq: 42,
        timestamp: "2026-03-19T12:00:00Z".into(),
        engine: EngineId::Rust,
        payload: EventPayload::Violation {
            policy_id: "no-orphan-modules".into(),
            file: "src/orphan.ts".into(),
            symbol: "OrphanClass".into(),
            message: "Module has no parent boundary".into(),
        },
    };
    assert_eq!(event.engine, EngineId::Rust);
    assert_eq!(event.event_type, EventType::Violation);
}

#[test]
fn error_event_carries_error_payload() {
    let event = EngineEvent {
        event_type: EventType::Error,
        seq: 1,
        timestamp: "t".into(),
        engine: EngineId::Legacy,
        payload: EventPayload::Error(ErrorPayload {
            code: ErrorCode::ParseError,
            file: Some("bad.ts".into()),
            message: "Unexpected EOF".into(),
            recoverable: true,
        }),
    };

    match &event.payload {
        EventPayload::Error(err) => {
            assert_eq!(err.code, ErrorCode::ParseError);
            assert!(err.recoverable);
        }
        _ => panic!("expected Error payload"),
    }
}

// -- Graph edges connect nodes --

#[test]
fn edge_connects_two_distinct_nodes() {
    let nodes = [
        SymbolNode {
            id: 1,
            kind: SymbolKind::Module,
            name: "core".into(),
            visibility: Visibility::Public,
            file: "src/core.ts".into(),
            trust_level: TrustLevel::Internal,
        },
        SymbolNode {
            id: 2,
            kind: SymbolKind::Function,
            name: "validate".into(),
            visibility: Visibility::Internal,
            file: "src/core/validate.ts".into(),
            trust_level: TrustLevel::Internal,
        },
    ];

    let edge = SymbolEdge {
        from: nodes[0].id,
        to: nodes[1].id,
        edge_type: EdgeType::Contains,
    };

    assert_eq!(edge.from, 1);
    assert_eq!(edge.to, 2);
    assert_ne!(edge.from, edge.to);
}

// -- Full serialisation round-trip across types --

#[test]
fn full_event_with_nested_types_round_trips() {
    let event = EngineEvent {
        event_type: EventType::Snapshot,
        seq: 100,
        timestamp: "2026-03-19T00:00:00Z".into(),
        engine: EngineId::Rust,
        payload: EventPayload::Snapshot {
            node_count: 500,
            edge_count: 1200,
            files_watched: 80,
            changed_path: Some("/repo/src/changed.ts".into()),
        },
    };

    let json = serde_json::to_string_pretty(&event).unwrap();
    let back: EngineEvent = serde_json::from_str(&json).unwrap();

    assert_eq!(back.event_type, EventType::Snapshot);
    assert_eq!(back.seq, 100);
    assert_eq!(back.engine, EngineId::Rust);

    match &back.payload {
        EventPayload::Snapshot {
            node_count,
            edge_count,
            files_watched,
            changed_path,
        } => {
            assert_eq!(*node_count, 500);
            assert_eq!(*edge_count, 1200);
            assert_eq!(*files_watched, 80);
            // `changed_path` is `#[serde(skip)]`: the `Some(...)` set above is
            // intentionally dropped by serialisation, so it never survives a
            // round-trip. This pins that the internal dispatch hint cannot
            // leak onto — or be injected through — the serialised form.
            assert_eq!(changed_path.as_deref(), None);
        }
        _ => panic!("expected Snapshot payload"),
    }
}

#[test]
fn graph_types_round_trip_together() {
    // CLAWP-061: round-trip BOTH endpoints' nodes alongside the edge and
    // assert the edge references a present node on each end. The prior
    // test created only the `from` node (id 10) and checked `edge.from`,
    // so the `to` endpoint (id 20) was a dangling reference that
    // round-tripped undetected.
    let source = SymbolNode {
        id: 10,
        kind: SymbolKind::Export,
        name: "default".into(),
        visibility: Visibility::Public,
        file: "index.ts".into(),
        trust_level: TrustLevel::Boundary,
    };
    let target = SymbolNode {
        id: 20,
        kind: SymbolKind::Module,
        name: "dep".into(),
        visibility: Visibility::Internal,
        file: "src/dep.ts".into(),
        trust_level: TrustLevel::Internal,
    };
    let edge = SymbolEdge {
        from: source.id,
        to: target.id,
        edge_type: EdgeType::Imports,
    };

    let source_back: SymbolNode =
        serde_json::from_str(&serde_json::to_string(&source).unwrap()).unwrap();
    let target_back: SymbolNode =
        serde_json::from_str(&serde_json::to_string(&target).unwrap()).unwrap();
    let edge_back: SymbolEdge =
        serde_json::from_str(&serde_json::to_string(&edge).unwrap()).unwrap();

    // Pin each endpoint to its specific node (stronger than mere
    // set-membership, which a swapped from/to would also satisfy), and
    // require the two endpoints to be distinct — no dangling or
    // self-referential edge.
    assert_eq!(
        edge_back.from, source_back.id,
        "edge source must reference the source node"
    );
    assert_eq!(
        edge_back.to, target_back.id,
        "edge target must reference the target node (no dangling endpoint)"
    );
    assert_ne!(
        edge_back.from, edge_back.to,
        "edge must connect two distinct nodes"
    );
    assert_eq!(source_back.trust_level, TrustLevel::Boundary);
}

// -- All trust levels can be assigned to nodes --

#[test]
fn all_trust_levels_assignable_to_nodes() {
    let levels = [
        TrustLevel::Unknown,
        TrustLevel::Internal,
        TrustLevel::Boundary,
        TrustLevel::External,
        TrustLevel::Privileged,
    ];

    for (i, level) in levels.iter().enumerate() {
        let node = SymbolNode {
            id: i as u64,
            kind: SymbolKind::Function,
            name: format!("fn_{i}"),
            visibility: Visibility::Public,
            file: "test.ts".into(),
            trust_level: *level,
        };
        assert_eq!(node.trust_level, *level);
    }
}

// -- Both engine ids produce valid events --

#[test]
fn both_engines_produce_valid_events() {
    for engine in [EngineId::Rust, EngineId::Legacy] {
        let event = EngineEvent {
            event_type: EventType::Progress,
            seq: 0,
            timestamp: "t".into(),
            engine,
            payload: EventPayload::Progress {
                phase: "init".into(),
                current: 0,
                total: 0,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.engine, engine);
    }
}

// -- EventType binds to its matching payload variant --

#[test]
fn event_type_pairs_with_its_payload_variant() {
    // CLAWP-062: pin the EventType<->EventPayload pairing across all four
    // event kinds. The other tests each construct one specific pairing,
    // but nothing asserted that a given EventType binds to its matching
    // payload variant — a regression that crossed them (or dropped a
    // variant) would not be caught.
    let events = [
        EngineEvent {
            event_type: EventType::Violation,
            seq: 1,
            timestamp: "t".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Violation {
                policy_id: "p".into(),
                file: "f.ts".into(),
                symbol: "S".into(),
                message: "m".into(),
            },
        },
        EngineEvent {
            event_type: EventType::Error,
            seq: 2,
            timestamp: "t".into(),
            engine: EngineId::Legacy,
            payload: EventPayload::Error(ErrorPayload {
                code: ErrorCode::ParseError,
                file: None,
                message: "m".into(),
                recoverable: false,
            }),
        },
        EngineEvent {
            event_type: EventType::Snapshot,
            seq: 3,
            timestamp: "t".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Snapshot {
                node_count: 1,
                edge_count: 0,
                files_watched: 1,
                changed_path: None,
            },
        },
        EngineEvent {
            event_type: EventType::Progress,
            seq: 4,
            timestamp: "t".into(),
            engine: EngineId::Rust,
            payload: EventPayload::Progress {
                phase: "init".into(),
                current: 0,
                total: 1,
            },
        },
    ];

    for event in events {
        // Correct pairing round-trips cleanly for every kind.
        let json = serde_json::to_string(&event).unwrap();
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event_type, event.event_type);

        // The real guard: tamper the wire `event_type` so it disagrees
        // with the payload variant, and assert the deserialiser REJECTS
        // it. (A positive `matches!` after `from_str` would be
        // tautological — `EngineEvent`'s `try_from` already enforces the
        // pairing, so a mismatch can't survive deserialisation. Asserting
        // the rejection per kind is what actually exercises the invariant.)
        let mut wire = serde_json::to_value(&event).unwrap();
        let mismatched = if wire["event_type"] == serde_json::json!("Progress") {
            "Violation"
        } else {
            "Progress"
        };
        wire["event_type"] = serde_json::json!(mismatched);
        assert!(
            serde_json::from_str::<EngineEvent>(&wire.to_string()).is_err(),
            "event_type `{mismatched}` disagreeing with the payload must be rejected: {wire}"
        );
    }
}
