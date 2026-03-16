use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use anvil_kernel_types::{EngineEvent, EngineId, ErrorCode, ErrorPayload, EventPayload, EventType};

use crate::graph::SymbolGraph;
use crate::policy::engine::Violation;

pub struct EventEmitter {
    tx: mpsc::Sender<EngineEvent>,
    seq: AtomicU64,
    engine: EngineId,
}

impl EventEmitter {
    pub fn new(tx: mpsc::Sender<EngineEvent>, engine: EngineId) -> Self {
        Self {
            tx,
            seq: AtomicU64::new(0),
            engine,
        }
    }

    pub fn progress(&self, phase: &str, current: u64, total: u64) {
        self.emit(
            EventType::Progress,
            EventPayload::Progress {
                phase: phase.to_string(),
                current,
                total,
            },
        );
    }

    pub fn snapshot(&self, graph: &SymbolGraph, files_watched: u64) {
        let stats = graph.stats();
        self.emit(
            EventType::Snapshot,
            EventPayload::Snapshot {
                node_count: stats.node_count as u64,
                edge_count: stats.edge_count as u64,
                files_watched,
            },
        );
    }

    pub fn violation(&self, v: &Violation) {
        self.emit(
            EventType::Violation,
            EventPayload::Violation {
                policy_id: v.policy_id.clone(),
                file: v.file.clone(),
                symbol: v.symbol.clone(),
                message: v.message.clone(),
            },
        );
    }

    pub fn error(&self, code: ErrorCode, file: Option<&str>, message: &str, recoverable: bool) {
        self.emit(
            EventType::Error,
            EventPayload::Error(ErrorPayload {
                code,
                file: file.map(String::from),
                message: message.to_string(),
                recoverable,
            }),
        );
    }

    fn emit(&self, event_type: EventType, payload: EventPayload) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let event = EngineEvent {
            event_type,
            seq,
            timestamp: now_iso8601(),
            engine: self.engine,
            payload,
        };
        // Best-effort send — receiver may have been dropped
        let _ = self.tx.send(event);
    }
}

fn now_iso8601() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_emitter() -> (EventEmitter, mpsc::Receiver<EngineEvent>) {
        let (tx, rx) = mpsc::channel();
        let emitter = EventEmitter::new(tx, EngineId::Rust);
        (emitter, rx)
    }

    #[test]
    fn progress_event_has_correct_payload_and_incrementing_seq() {
        let (emitter, rx) = make_emitter();

        emitter.progress("parsing", 5, 10);
        emitter.progress("parsing", 6, 10);

        let e1 = rx.recv().unwrap();
        assert_eq!(e1.event_type, EventType::Progress);
        assert_eq!(e1.seq, 0);
        assert_eq!(e1.engine, EngineId::Rust);
        match &e1.payload {
            EventPayload::Progress {
                phase,
                current,
                total,
            } => {
                assert_eq!(phase, "parsing");
                assert_eq!(*current, 5);
                assert_eq!(*total, 10);
            }
            _ => panic!("expected Progress payload"),
        }

        let e2 = rx.recv().unwrap();
        assert_eq!(e2.seq, 1);
    }

    #[test]
    fn snapshot_event_includes_graph_stats() {
        let (emitter, rx) = make_emitter();

        use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "foo".to_string(),
                visibility: Visibility::Internal,
                file: "a.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            })
            .unwrap();
        graph
            .add_symbol(SymbolNode {
                id: 2,
                kind: SymbolKind::Function,
                name: "bar".to_string(),
                visibility: Visibility::Internal,
                file: "b.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            })
            .unwrap();

        emitter.snapshot(&graph, 42);

        let event = rx.recv().unwrap();
        assert_eq!(event.event_type, EventType::Snapshot);
        match &event.payload {
            EventPayload::Snapshot {
                node_count,
                edge_count,
                files_watched,
            } => {
                assert_eq!(*node_count, 2);
                assert_eq!(*edge_count, 0);
                assert_eq!(*files_watched, 42);
            }
            _ => panic!("expected Snapshot payload"),
        }
    }

    #[test]
    fn violation_event_maps_policy_violation_fields() {
        let (emitter, rx) = make_emitter();

        use crate::policy::engine::Severity;
        let v = Violation {
            policy_id: "cross-layer".to_string(),
            file: "src/a.ts".to_string(),
            symbol: "foo".to_string(),
            message: "bad import".to_string(),
            severity: Severity::High,
        };

        emitter.violation(&v);

        let event = rx.recv().unwrap();
        assert_eq!(event.event_type, EventType::Violation);
        match &event.payload {
            EventPayload::Violation {
                policy_id,
                file,
                symbol,
                message,
            } => {
                assert_eq!(policy_id, "cross-layer");
                assert_eq!(file, "src/a.ts");
                assert_eq!(symbol, "foo");
                assert_eq!(message, "bad import");
            }
            _ => panic!("expected Violation payload"),
        }
    }

    #[test]
    fn error_event_includes_error_code_and_file() {
        let (emitter, rx) = make_emitter();

        emitter.error(
            ErrorCode::ParseError,
            Some("broken.ts"),
            "unexpected token",
            true,
        );

        let event = rx.recv().unwrap();
        assert_eq!(event.event_type, EventType::Error);
        match &event.payload {
            EventPayload::Error(err) => {
                assert_eq!(err.code, ErrorCode::ParseError);
                assert_eq!(err.file.as_deref(), Some("broken.ts"));
                assert_eq!(err.message, "unexpected token");
                assert!(err.recoverable);
            }
            _ => panic!("expected Error payload"),
        }
    }

    #[test]
    fn sequence_numbers_increment_monotonically() {
        let (emitter, rx) = make_emitter();

        emitter.progress("a", 0, 1);
        emitter.progress("b", 0, 1);
        emitter.progress("c", 0, 1);
        emitter.error(ErrorCode::Internal, None, "oops", false);

        let mut seqs = Vec::new();
        for _ in 0..4 {
            seqs.push(rx.recv().unwrap().seq);
        }

        assert_eq!(seqs, vec![0, 1, 2, 3]);
    }
}
