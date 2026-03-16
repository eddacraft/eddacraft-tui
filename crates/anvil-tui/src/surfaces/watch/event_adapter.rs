use anvil_kernel_types::{EngineEvent, EventPayload};

use super::{QueuedChange, RunHistory, WatchData, WatchStatus};

/// Converts `EngineEvent` stream into `WatchData` updates for the watch
/// dashboard. Bridges kernel events to TUI state.
#[allow(clippy::struct_field_names)]
pub struct WatchEventAdapter {
    violation_count: usize,
    error_count: usize,
    check_count: usize,
}

impl WatchEventAdapter {
    pub fn new() -> Self {
        Self {
            violation_count: 0,
            error_count: 0,
            check_count: 0,
        }
    }

    /// Process an engine event and update the watch data accordingly.
    pub fn handle_event(&mut self, event: &EngineEvent, data: &mut WatchData) {
        match &event.payload {
            EventPayload::Progress {
                phase,
                current,
                total,
            } => {
                self.handle_progress(phase, *current, *total, data);
            }
            EventPayload::Snapshot {
                node_count: _,
                edge_count: _,
                files_watched,
            } => {
                self.handle_snapshot(*files_watched, data);
            }
            EventPayload::Violation {
                policy_id: _,
                file,
                symbol: _,
                message,
            } => {
                self.handle_violation(file, message, &event.timestamp, data);
            }
            EventPayload::Error(err) => {
                self.handle_error(&err.message, &event.timestamp, data);
            }
        }
    }

    fn handle_progress(&mut self, phase: &str, current: u64, total: u64, data: &mut WatchData) {
        if current == 0 {
            self.violation_count = 0;
            self.error_count = 0;
            self.check_count = 0;
        }

        data.status = WatchStatus::Running;

        if current >= total && total > 0 {
            #[allow(clippy::cast_possible_truncation)]
            {
                self.check_count = total as usize;
            }
            let passed = self.violation_count == 0 && self.error_count == 0;
            data.status = if passed {
                WatchStatus::Passing
            } else {
                WatchStatus::Failing
            };

            let checks_passed = self.check_count.saturating_sub(self.violation_count);
            data.history.insert(
                0,
                RunHistory {
                    passed,
                    checks_run: self.check_count,
                    checks_passed,
                    duration_ms: 0,
                    timestamp: format!("{phase} complete"),
                },
            );

            data.stats.total_runs += 1;
            Self::update_pass_rate(data);
        }
    }

    fn handle_snapshot(&mut self, files_watched: u64, data: &mut WatchData) {
        #[allow(clippy::cast_possible_truncation)]
        {
            data.stats.files_watched = files_watched as usize;
        }

        // Each snapshot represents a completed watch cycle.
        // Record the result and reset violation/error state for the next cycle.
        let passed = self.violation_count == 0 && self.error_count == 0;
        let checks_passed = self.check_count.saturating_sub(self.violation_count);

        data.stats.total_runs += 1;
        data.status = if passed {
            WatchStatus::Passing
        } else {
            WatchStatus::Failing
        };

        data.history.insert(
            0,
            RunHistory {
                passed,
                checks_run: self.check_count,
                checks_passed,
                duration_ms: 0,
                timestamp: "snapshot complete".to_string(),
            },
        );
        Self::update_pass_rate(data);

        // Reset for next watch cycle
        self.violation_count = 0;
        self.error_count = 0;
        self.check_count = 0;
    }

    fn handle_violation(
        &mut self,
        file: &str,
        message: &str,
        timestamp: &str,
        data: &mut WatchData,
    ) {
        self.violation_count += 1;
        data.status = WatchStatus::Failing;

        data.queue.push(QueuedChange {
            file: file.to_string(),
            kind: message.to_string(),
            timestamp: timestamp.to_string(),
        });
    }

    fn handle_error(&mut self, message: &str, timestamp: &str, data: &mut WatchData) {
        self.error_count += 1;
        data.status = WatchStatus::Failing;
        data.queue.push(QueuedChange {
            file: "(error)".to_string(),
            kind: message.to_string(),
            timestamp: timestamp.to_string(),
        });
    }

    #[allow(clippy::cast_precision_loss)]
    fn update_pass_rate(data: &mut WatchData) {
        if data.stats.total_runs == 0 {
            data.stats.pass_rate = 0.0;
        } else {
            let passing = data.history.iter().filter(|h| h.passed).count();
            data.stats.pass_rate = passing as f64 / data.stats.total_runs as f64;
        }
    }
}

impl Default for WatchEventAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{EngineId, ErrorCode, ErrorPayload, EventType};

    use crate::surfaces::watch::WatchStats;

    fn empty_data() -> WatchData {
        WatchData {
            status: WatchStatus::Idle,
            queue: Vec::new(),
            history: Vec::new(),
            stats: WatchStats {
                total_runs: 0,
                pass_rate: 0.0,
                avg_duration_ms: 0,
                files_watched: 0,
            },
        }
    }

    fn progress_event(phase: &str, current: u64, total: u64) -> EngineEvent {
        EngineEvent {
            event_type: EventType::Progress,
            seq: 0,
            timestamp: "10:00:00".to_string(),
            engine: EngineId::Rust,
            payload: EventPayload::Progress {
                phase: phase.to_string(),
                current,
                total,
            },
        }
    }

    fn snapshot_event(files_watched: u64) -> EngineEvent {
        EngineEvent {
            event_type: EventType::Snapshot,
            seq: 0,
            timestamp: "10:00:00".to_string(),
            engine: EngineId::Rust,
            payload: EventPayload::Snapshot {
                node_count: 10,
                edge_count: 5,
                files_watched,
            },
        }
    }

    fn violation_event(file: &str, message: &str) -> EngineEvent {
        EngineEvent {
            event_type: EventType::Violation,
            seq: 0,
            timestamp: "10:00:01".to_string(),
            engine: EngineId::Rust,
            payload: EventPayload::Violation {
                policy_id: "test-policy".to_string(),
                file: file.to_string(),
                symbol: "sym".to_string(),
                message: message.to_string(),
            },
        }
    }

    fn error_event(message: &str) -> EngineEvent {
        EngineEvent {
            event_type: EventType::Error,
            seq: 0,
            timestamp: "10:00:02".to_string(),
            engine: EngineId::Rust,
            payload: EventPayload::Error(ErrorPayload {
                code: ErrorCode::Internal,
                file: None,
                message: message.to_string(),
                recoverable: false,
            }),
        }
    }

    #[test]
    fn progress_event_updates_status_to_running() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&progress_event("scan", 1, 10), &mut data);

        assert_eq!(data.status, WatchStatus::Running);
    }

    #[test]
    fn progress_completion_sets_passing() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&progress_event("scan", 0, 5), &mut data);
        adapter.handle_event(&progress_event("scan", 5, 5), &mut data);

        assert_eq!(data.status, WatchStatus::Passing);
        assert_eq!(data.history.len(), 1);
        assert!(data.history[0].passed);
        assert_eq!(data.stats.total_runs, 1);
    }

    #[test]
    fn snapshot_event_updates_stats() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&snapshot_event(42), &mut data);

        assert_eq!(data.stats.files_watched, 42);
    }

    #[test]
    fn snapshot_without_violations_updates_run_state() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&snapshot_event(10), &mut data);

        assert_eq!(data.status, WatchStatus::Passing);
        assert_eq!(data.stats.total_runs, 1);
        assert_eq!(data.history.len(), 1);
        assert!(data.history[0].passed);
    }

    #[test]
    fn snapshot_after_violation_records_failing_run() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(
            &violation_event("src/bad.ts", "cross-layer import"),
            &mut data,
        );
        adapter.handle_event(&snapshot_event(10), &mut data);

        assert_eq!(data.status, WatchStatus::Failing);
        assert_eq!(data.stats.total_runs, 1);
        assert_eq!(data.history.len(), 1);
        assert!(!data.history[0].passed);
    }

    #[test]
    fn violation_state_resets_after_snapshot() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        // First cycle: violation then snapshot
        adapter.handle_event(
            &violation_event("src/bad.ts", "cross-layer import"),
            &mut data,
        );
        adapter.handle_event(&snapshot_event(10), &mut data);
        assert_eq!(data.status, WatchStatus::Failing);

        // Second cycle: clean snapshot — should transition to Passing
        adapter.handle_event(&snapshot_event(10), &mut data);
        assert_eq!(data.status, WatchStatus::Passing);
        assert_eq!(data.stats.total_runs, 2);
    }

    #[test]
    fn violation_event_adds_to_queue() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(
            &violation_event("src/bad.ts", "cross-layer import"),
            &mut data,
        );

        assert_eq!(data.status, WatchStatus::Failing);
        assert_eq!(data.queue.len(), 1);
        assert_eq!(data.queue[0].file, "src/bad.ts");
    }

    #[test]
    fn error_event_updates_status() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&error_event("parse failed"), &mut data);

        assert_eq!(data.status, WatchStatus::Failing);
        assert_eq!(data.queue.len(), 1);
        assert_eq!(data.queue[0].file, "(error)");
    }

    #[test]
    fn snapshot_after_error_records_failing_run() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&error_event("parse failed"), &mut data);
        adapter.handle_event(&snapshot_event(10), &mut data);

        assert_eq!(data.status, WatchStatus::Failing);
        assert_eq!(data.stats.total_runs, 1);
        assert!(!data.history[0].passed);
    }

    #[test]
    fn error_state_resets_after_snapshot() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        // First cycle: error then snapshot
        adapter.handle_event(&error_event("parse failed"), &mut data);
        adapter.handle_event(&snapshot_event(10), &mut data);
        assert_eq!(data.status, WatchStatus::Failing);

        // Second cycle: clean snapshot — should transition to Passing
        adapter.handle_event(&snapshot_event(10), &mut data);
        assert_eq!(data.status, WatchStatus::Passing);
        assert_eq!(data.stats.total_runs, 2);
    }

    #[test]
    fn run_with_violations_produces_failing_history() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&progress_event("scan", 0, 3), &mut data);
        adapter.handle_event(&violation_event("src/bad.ts", "violation"), &mut data);
        adapter.handle_event(&progress_event("scan", 3, 3), &mut data);

        assert_eq!(data.status, WatchStatus::Failing);
        assert_eq!(data.history.len(), 1);
        assert!(!data.history[0].passed);
    }
}
