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
    let node = SymbolNode {
        id: 10,
        kind: SymbolKind::Export,
        name: "default".into(),
        visibility: Visibility::Public,
        file: "index.ts".into(),
        trust_level: TrustLevel::Boundary,
    };

    let edge = SymbolEdge {
        from: 10,
        to: 20,
        edge_type: EdgeType::Imports,
    };

    let node_json = serde_json::to_string(&node).unwrap();
    let edge_json = serde_json::to_string(&edge).unwrap();

    let node_back: SymbolNode = serde_json::from_str(&node_json).unwrap();
    let edge_back: SymbolEdge = serde_json::from_str(&edge_json).unwrap();

    // The edge should reference the node's id
    assert_eq!(edge_back.from, node_back.id);
    assert_eq!(node_back.trust_level, TrustLevel::Boundary);
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
