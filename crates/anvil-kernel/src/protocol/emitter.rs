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

    /// Emit a graph snapshot.
    ///
    /// `changed_path` is the repo-absolute path of the single file whose save
    /// produced this snapshot (RLB-007). Pass `Some(path)` from a watch
    /// create/modify change so the CLI can scope its per-save `anvil check` to
    /// that file; pass `None` for the initial scan, embedded one-shot scans,
    /// and delete-driven snapshots (where a full re-walk is the safe default).
    pub fn snapshot(&self, graph: &SymbolGraph, files_watched: u64, changed_path: Option<&str>) {
        let stats = graph.stats();
        self.emit(
            EventType::Snapshot,
            EventPayload::Snapshot {
                node_count: stats.node_count as u64,
                edge_count: stats.edge_count as u64,
                files_watched,
                changed_path: changed_path.map(String::from),
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
    const SECS_PER_DAY: u64 = 86_400;
    const DAYS_PER_400Y: u64 = 146_097;
    const DAYS_PER_100Y: u64 = 36_524;
    const DAYS_PER_4Y: u64 = 1_461;
    const DAYS_PER_YEAR: u64 = 365;

    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();

    let time_of_day = secs % SECS_PER_DAY;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;

    // Days since 1970-01-01, shifted to civil calendar epoch (0000-03-01)
    let mut days = secs / SECS_PER_DAY;
    days += 719_468;

    let era = days / DAYS_PER_400Y;
    let day_of_era = days % DAYS_PER_400Y;
    let year_of_era = (day_of_era - day_of_era / (DAYS_PER_4Y - 1) + day_of_era / DAYS_PER_100Y
        - day_of_era / (DAYS_PER_400Y - 1))
        / DAYS_PER_YEAR;
    let mut year = year_of_era + era * 400;
    let day_of_year =
        day_of_era - (DAYS_PER_YEAR * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
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
        use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};

        let (emitter, rx) = make_emitter();
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "foo".to_string(),
                visibility: Visibility::Internal,
                file: "a.ts".to_string(),
                trust_level: TrustLevel::Unknown,
                span: None,
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
                span: None,
            })
            .unwrap();

        emitter.snapshot(&graph, 42, Some("/repo/b.ts"));

        let event = rx.recv().unwrap();
        assert_eq!(event.event_type, EventType::Snapshot);
        match &event.payload {
            EventPayload::Snapshot {
                node_count,
                edge_count,
                files_watched,
                changed_path,
            } => {
                assert_eq!(*node_count, 2);
                assert_eq!(*edge_count, 0);
                assert_eq!(*files_watched, 42);
                assert_eq!(changed_path.as_deref(), Some("/repo/b.ts"));
            }
            _ => panic!("expected Snapshot payload"),
        }
    }

    #[test]
    fn violation_event_maps_policy_violation_fields() {
        use crate::policy::engine::Severity;

        let (emitter, rx) = make_emitter();
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
    fn timestamp_is_iso8601_format() {
        let (emitter, rx) = make_emitter();
        emitter.progress("test", 0, 1);
        let event = rx.recv().unwrap();

        let ts = &event.timestamp;
        assert!(
            ts.len() == 20,
            "timestamp should be 20 chars (YYYY-MM-DDTHH:MM:SSZ), got {ts}"
        );
        assert!(ts.ends_with('Z'), "timestamp should end with Z: {ts}");
        assert_eq!(&ts[4..5], "-", "expected dash at position 4: {ts}");
        assert_eq!(&ts[7..8], "-", "expected dash at position 7: {ts}");
        assert_eq!(&ts[10..11], "T", "expected T at position 10: {ts}");
        assert_eq!(&ts[13..14], ":", "expected colon at position 13: {ts}");
        assert_eq!(&ts[16..17], ":", "expected colon at position 16: {ts}");

        let year: u32 = ts[0..4].parse().expect("year should be numeric");
        assert!(year >= 2020, "year should be >= 2020, got {year}");
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
