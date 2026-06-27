use std::time::Instant;

use anvil_kernel_types::{
    EngineEvent, EventPayload, Notification, NotificationClass, NotificationContext,
    NotificationPriority,
};

use super::{
    ActionResultLine, DaemonNotice, QueuedNotification, RunHistory, WatchData, WatchStatus,
    WatchWarmup,
};

/// Maximum number of entries retained in the change queue.
const MAX_QUEUE_LEN: usize = 200;
/// Maximum number of run history entries retained.
const MAX_HISTORY_LEN: usize = 100;

/// Converts `EngineEvent` stream into `WatchData` updates for the watch
/// dashboard. Bridges kernel events to TUI state.
#[allow(clippy::struct_field_names)]
pub struct WatchEventAdapter {
    violation_count: usize,
    error_count: usize,
    check_count: usize,
    cycle_start: Option<Instant>,
}

impl WatchEventAdapter {
    pub fn new() -> Self {
        Self {
            violation_count: 0,
            error_count: 0,
            check_count: 0,
            cycle_start: None,
        }
    }

    /// Process an engine event and update the watch data accordingly.
    ///
    /// **Kernel contract**: every completed Progress sequence is followed by
    /// a Snapshot event that serves as the authoritative end-of-cycle marker.
    /// If a Snapshot never arrives (kernel crash, channel disconnect), the
    /// cycle's history entry is lost. This is acceptable because the watch
    /// session is already terminated in that scenario.
    pub fn handle_event(&mut self, event: &EngineEvent, data: &mut WatchData) {
        // Start the cycle timer on the first non-Snapshot event. Snapshot is
        // the end-of-cycle marker and resets cycle_start via .take(), so any
        // event arriving after that is the start of a new cycle. This works
        // whether the kernel emits Progress events or not.
        if self.cycle_start.is_none() && !matches!(&event.payload, EventPayload::Snapshot { .. }) {
            self.violation_count = 0;
            self.error_count = 0;
            self.check_count = 0;
            self.cycle_start = Some(Instant::now());
        }

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
                changed_path: _,
            } => {
                self.handle_snapshot(*files_watched, &event.timestamp, data);
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
                self.handle_error(&err.message, err.file.as_deref(), &event.timestamp, data);
            }
        }
    }

    /// Fold an `--action` outcome into the dashboard (LAUNCH-002).
    ///
    /// **Isolation invariant:** writes only the action/footer state
    /// (`last_action` and the TUI-only daemon fallback notice). Action outcomes
    /// must not flip the kernel-derived status icon or pollute `WatchStats`
    /// arithmetic — those are kernel-event-only fields. A failing `gate` is not
    /// the same signal as a kernel violation, and the Status pane must not
    /// conflate them.
    pub fn handle_action_result(line: &ActionResultLine, data: &mut WatchData) {
        match &line.daemon_notice {
            Some(DaemonNotice::Fallback { message }) => {
                data.daemon_fallback_notice = Some(message.clone());
            }
            Some(DaemonNotice::ClearFallback) => {
                data.daemon_fallback_notice = None;
            }
            None => {}
        }
        data.last_action = Some(line.clone());
    }

    fn handle_progress(&mut self, phase: &str, current: u64, total: u64, data: &mut WatchData) {
        data.status = WatchStatus::Running;
        data.warmup = Some(WatchWarmup {
            phase: phase.to_string(),
            current,
            total,
        });

        if current >= total && total > 0 {
            #[allow(clippy::cast_possible_truncation)]
            {
                self.check_count = total as usize;
            }
        }
    }

    fn handle_snapshot(&mut self, files_watched: u64, timestamp: &str, data: &mut WatchData) {
        #[allow(clippy::cast_possible_truncation)]
        {
            data.stats.files_watched = files_watched as usize;
        }
        data.warmup = None;

        // Each snapshot represents a completed watch cycle.
        // Record the result and reset violation/error state for the next cycle.
        let passed = self.error_count == 0;
        let checks_passed = self.check_count.saturating_sub(self.error_count);

        let duration_ms = self.cycle_start.take().map_or(0, |s| {
            let ms = s.elapsed().as_millis();
            u64::try_from(ms).unwrap_or(u64::MAX)
        });

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
                duration_ms,
                timestamp: timestamp.to_string(),
            },
        );
        data.history.truncate(MAX_HISTORY_LEN);
        Self::update_pass_rate(data);
        Self::update_avg_duration(data);

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

        Self::push_queue(
            data,
            Notification::new(
                NotificationClass::Warning,
                NotificationPriority::Normal,
                file,
                message,
            )
            .with_context(NotificationContext {
                file: Some(file.to_string()),
                source: Some("watch".to_string()),
            }),
            timestamp,
        );
    }

    fn handle_error(
        &mut self,
        message: &str,
        file: Option<&str>,
        timestamp: &str,
        data: &mut WatchData,
    ) {
        self.error_count += 1;
        data.status = WatchStatus::Failing;
        data.warmup = None;
        // Prefer the offending file as the title when available so the watch
        // queue is immediately actionable; fall back to a generic label.
        let title = file.unwrap_or("Watch error");
        Self::push_queue(
            data,
            Notification::new(
                NotificationClass::Failure,
                NotificationPriority::High,
                title,
                message,
            )
            .with_context(NotificationContext {
                file: file.map(str::to_string),
                source: Some("watch".to_string()),
            }),
            timestamp,
        );
    }

    /// Push an entry to the change queue, dropping the oldest if at capacity.
    fn push_queue(data: &mut WatchData, notification: Notification, timestamp: &str) {
        if data.queue.len() >= MAX_QUEUE_LEN {
            data.queue.pop_front();
        }
        data.queue.push_back(QueuedNotification {
            notification,
            timestamp: timestamp.to_string(),
        });
    }

    /// Recompute pass rate as a rolling window over retained history.
    /// Uses `history.len()` as the denominator (not `total_runs`) because
    /// history is capped at `MAX_HISTORY_LEN` — dividing by the uncapped
    /// lifetime count would cause the rate to converge toward zero.
    #[allow(clippy::cast_precision_loss)]
    fn update_pass_rate(data: &mut WatchData) {
        if data.history.is_empty() {
            data.stats.pass_rate = 0.0;
        } else {
            let passing = data.history.iter().filter(|h| h.passed).count();
            data.stats.pass_rate = passing as f64 / data.history.len() as f64;
        }
    }

    /// Recompute average duration over retained history entries.
    #[allow(clippy::cast_possible_truncation)]
    fn update_avg_duration(data: &mut WatchData) {
        if data.history.is_empty() {
            data.stats.avg_duration_ms = 0;
        } else {
            let total: u64 = data.history.iter().map(|h| h.duration_ms).sum();
            data.stats.avg_duration_ms = total / data.history.len() as u64;
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
            queue: std::collections::VecDeque::new(),
            history: Vec::new(),
            stats: WatchStats {
                total_runs: 0,
                pass_rate: 0.0,
                avg_duration_ms: 0,
                files_watched: 0,
            },
            warmup: None,
            last_action: None,
            update_hint: None,
            insights_hint: None,
            daemon_fallback_notice: None,
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
                changed_path: None,
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
        error_event_for(message, None)
    }

    fn error_event_for(message: &str, file: Option<&str>) -> EngineEvent {
        EngineEvent {
            event_type: EventType::Error,
            seq: 0,
            timestamp: "10:00:02".to_string(),
            engine: EngineId::Rust,
            payload: EventPayload::Error(ErrorPayload {
                code: ErrorCode::Internal,
                file: file.map(str::to_string),
                message: message.to_string(),
                recoverable: false,
            }),
        }
    }

    #[test]
    fn progress_event_updates_status_to_running() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&progress_event("Building graph", 1, 10), &mut data);

        assert_eq!(data.status, WatchStatus::Running);
        let warmup = data.warmup.as_ref().expect("warm-up progress stored");
        assert_eq!(warmup.phase, "Building graph");
        assert_eq!(warmup.current, 1);
        assert_eq!(warmup.total, 10);
    }

    #[test]
    fn progress_completion_keeps_warmup_status_until_snapshot() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&progress_event("scan", 0, 5), &mut data);
        adapter.handle_event(&progress_event("scan", 5, 5), &mut data);

        assert_eq!(data.status, WatchStatus::Running);
        assert!(data.warmup.is_some());
        // Progress completion updates status but does not record history
        // — the subsequent Snapshot is the authoritative end-of-cycle marker.
        assert_eq!(data.history.len(), 0);
        assert_eq!(data.stats.total_runs, 0);
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
        assert!(data.warmup.is_none());
        assert_eq!(data.stats.total_runs, 1);
        assert_eq!(data.history.len(), 1);
        assert!(data.history[0].passed);
    }

    #[test]
    fn snapshot_after_violation_records_warning_run() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(
            &violation_event("src/bad.ts", "cross-layer import"),
            &mut data,
        );
        adapter.handle_event(&snapshot_event(10), &mut data);

        assert_eq!(data.status, WatchStatus::Passing);
        assert_eq!(data.stats.total_runs, 1);
        assert_eq!(data.history.len(), 1);
        assert!(data.history[0].passed);
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
        assert_eq!(data.status, WatchStatus::Passing);

        // Second cycle: clean snapshot — should transition to Passing
        adapter.handle_event(&snapshot_event(10), &mut data);
        assert_eq!(data.status, WatchStatus::Passing);
        assert_eq!(data.stats.total_runs, 2);
    }

    #[test]
    fn violation_event_adds_advisory_warning_to_queue() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(
            &violation_event("src/bad.ts", "cross-layer import"),
            &mut data,
        );

        assert_eq!(data.status, WatchStatus::Idle);
        assert_eq!(data.queue.len(), 1);
        assert_eq!(data.queue[0].notification.title, "src/bad.ts");
        assert_eq!(data.queue[0].notification.class, NotificationClass::Warning);
        assert_eq!(
            data.queue[0].notification.priority,
            NotificationPriority::Normal
        );
    }

    #[test]
    fn error_event_updates_status() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&error_event("parse failed"), &mut data);

        assert_eq!(data.status, WatchStatus::Failing);
        assert_eq!(data.queue.len(), 1);
        assert_eq!(data.queue[0].notification.title, "Watch error");
        assert_eq!(data.queue[0].notification.class, NotificationClass::Failure);
        assert!(
            data.queue[0]
                .notification
                .context
                .as_ref()
                .is_some_and(|ctx| ctx.file.is_none())
        );
    }

    #[test]
    fn error_event_with_file_populates_notification_context() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(
            &error_event_for("parse failed", Some("src/bad.ts")),
            &mut data,
        );

        assert_eq!(data.queue.len(), 1);
        assert_eq!(data.queue[0].notification.title, "src/bad.ts");
        let ctx = data.queue[0]
            .notification
            .context
            .as_ref()
            .expect("context populated");
        assert_eq!(ctx.file.as_deref(), Some("src/bad.ts"));
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

    // --- RCLI-034: no double-counting ---

    #[test]
    fn progress_complete_then_snapshot_counts_one_run() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&progress_event("scan", 0, 5), &mut data);
        adapter.handle_event(&progress_event("scan", 5, 5), &mut data);
        adapter.handle_event(&snapshot_event(10), &mut data);

        assert_eq!(data.stats.total_runs, 1);
        assert_eq!(data.history.len(), 1);
    }

    #[test]
    fn two_full_cycles_count_two_runs() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        // Cycle 1
        adapter.handle_event(&progress_event("scan", 0, 3), &mut data);
        adapter.handle_event(&progress_event("scan", 3, 3), &mut data);
        adapter.handle_event(&snapshot_event(10), &mut data);

        // Cycle 2
        adapter.handle_event(&progress_event("scan", 0, 3), &mut data);
        adapter.handle_event(&progress_event("scan", 3, 3), &mut data);
        adapter.handle_event(&snapshot_event(10), &mut data);

        assert_eq!(data.stats.total_runs, 2);
        assert_eq!(data.history.len(), 2);
    }

    // --- RCLI-033: bounded collections ---

    #[test]
    fn queue_capped_at_max_len() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        for i in 0..(MAX_QUEUE_LEN + 50) {
            adapter.handle_event(
                &violation_event(&format!("file_{i}.ts"), "violation"),
                &mut data,
            );
        }

        assert_eq!(data.queue.len(), MAX_QUEUE_LEN);
        // Oldest entries were dropped — first entry should be file_50
        assert_eq!(data.queue[0].notification.title, "file_50.ts");
    }

    #[test]
    fn history_capped_at_max_len() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        for _ in 0..(MAX_HISTORY_LEN + 20) {
            adapter.handle_event(&snapshot_event(10), &mut data);
        }

        assert_eq!(data.history.len(), MAX_HISTORY_LEN);
        assert_eq!(data.stats.total_runs, MAX_HISTORY_LEN + 20);
    }

    #[test]
    fn pass_rate_correct_after_history_capped() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        // All passing — fill beyond the cap
        for _ in 0..(MAX_HISTORY_LEN + 50) {
            adapter.handle_event(&snapshot_event(10), &mut data);
        }

        // pass_rate is a rolling window over retained history, not lifetime
        assert!((data.stats.pass_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn run_with_only_violations_produces_warning_history() {
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();

        adapter.handle_event(&progress_event("scan", 0, 3), &mut data);
        adapter.handle_event(&violation_event("src/bad.ts", "violation"), &mut data);
        adapter.handle_event(&progress_event("scan", 3, 3), &mut data);
        adapter.handle_event(&snapshot_event(10), &mut data);

        assert_eq!(data.status, WatchStatus::Passing);
        assert_eq!(data.history.len(), 1);
        assert!(data.history[0].passed);
        assert_eq!(data.stats.total_runs, 1);
    }

    // --- LAUNCH-002: action result isolation ---

    fn action_result(action: &str, exit_code: Option<i32>) -> ActionResultLine {
        ActionResultLine {
            action: action.to_string(),
            exit_code,
            duration_ms: 1234,
            timestamp: "10:30:00".to_string(),
            error_detail: None,
            daemon_notice: None,
        }
    }

    #[test]
    fn action_result_writes_last_action() {
        let mut data = empty_data();
        WatchEventAdapter::handle_action_result(&action_result("gate", Some(0)), &mut data);

        let last = data.last_action.as_ref().expect("last_action set");
        assert_eq!(last.action, "gate");
        assert_eq!(last.exit_code, Some(0));
        assert_eq!(last.duration_ms, 1234);
        assert!(last.passed());
    }

    #[test]
    fn action_result_overwrites_previous() {
        let mut data = empty_data();
        WatchEventAdapter::handle_action_result(&action_result("gate", Some(0)), &mut data);
        WatchEventAdapter::handle_action_result(&action_result("check", Some(1)), &mut data);

        let last = data.last_action.as_ref().expect("last_action set");
        assert_eq!(last.action, "check");
        assert_eq!(last.exit_code, Some(1));
        assert!(!last.passed());
    }

    #[test]
    fn action_result_does_not_mutate_kernel_state() {
        // Drive a full kernel cycle so the adapter has populated state.
        let mut adapter = WatchEventAdapter::new();
        let mut data = empty_data();
        adapter.handle_event(&progress_event("scan", 0, 5), &mut data);
        adapter.handle_event(&progress_event("scan", 5, 5), &mut data);
        adapter.handle_event(&snapshot_event(42), &mut data);

        let status_before = data.status;
        let total_runs_before = data.stats.total_runs;
        let pass_rate_before = data.stats.pass_rate;
        let avg_duration_before = data.stats.avg_duration_ms;
        let files_watched_before = data.stats.files_watched;
        let history_len_before = data.history.len();
        let queue_len_before = data.queue.len();

        // A failing action must not touch any kernel-derived field.
        WatchEventAdapter::handle_action_result(&action_result("gate", Some(1)), &mut data);

        assert_eq!(data.status, status_before, "status must not change");
        assert_eq!(
            data.stats.total_runs, total_runs_before,
            "total_runs must not change"
        );
        assert!(
            (data.stats.pass_rate - pass_rate_before).abs() < f64::EPSILON,
            "pass_rate must not change"
        );
        assert_eq!(
            data.stats.avg_duration_ms, avg_duration_before,
            "avg_duration_ms must not change"
        );
        assert_eq!(
            data.stats.files_watched, files_watched_before,
            "files_watched must not change"
        );
        assert_eq!(
            data.history.len(),
            history_len_before,
            "history must not change"
        );
        assert_eq!(data.queue.len(), queue_len_before, "queue must not change");
    }

    #[test]
    fn action_result_sets_daemon_fallback_notice() {
        let mut data = empty_data();
        let mut line = action_result("check", Some(0));
        line.daemon_notice = Some(super::DaemonNotice::Fallback {
            message: "daemon: unavailable -- scoped fallback".to_string(),
        });

        WatchEventAdapter::handle_action_result(&line, &mut data);

        assert_eq!(
            data.daemon_fallback_notice.as_deref(),
            Some("daemon: unavailable -- scoped fallback")
        );
    }

    #[test]
    fn action_result_clears_daemon_fallback_notice_on_reconnect() {
        let mut data = empty_data();
        data.daemon_fallback_notice = Some("daemon: unavailable -- scoped fallback".to_string());
        let mut line = action_result("check", Some(0));
        line.daemon_notice = Some(super::DaemonNotice::ClearFallback);

        WatchEventAdapter::handle_action_result(&line, &mut data);

        assert_eq!(data.daemon_fallback_notice, None);
    }
}
