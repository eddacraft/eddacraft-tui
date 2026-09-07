//! Recycle the Anvil-owned intercept daemon when CLI and daemon versions diverge.
//!
//! MCPLH-004 automates the existing operator guidance from
//! `anvil intercept status` (stop → wait for the reported PID to exit →
//! start the current binary). Harness MCP children are never signalled.
//!
//! [`recycle_daemon_if_version_skew`] is the reusable helper later MCP
//! refresh (MCPLH-003) can call. The ensure path wraps it so bare `anvil`
//! and `anvil start` recycle a live mismatched daemon.

use anvil_intercept::ensure::{EnsureOutcome, StartCapability};

/// How long to wait for a SIGTERM'd daemon PID to exit before failing recycle.
#[cfg(any(unix, windows))]
const PID_EXIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Snapshot of a live daemon: its version string and the verified PID-file
/// instances observed at the same time. A recycle decided against this
/// snapshot may stop only these instances (JREL-004): a replacement started
/// by a concurrent caller is never signalled with this stale identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunningDaemon {
    pub version: String,
    pub instances: Vec<anvil_intercept::DaemonPidRecord>,
}

impl RunningDaemon {
    /// The distinct live daemon processes observed across this scope's PID
    /// files (one process recorded from two candidate directories counts
    /// once). A version probe answers on one connection only, so a recycle
    /// may act only when exactly one process is behind it (JREL-004).
    #[must_use]
    pub(crate) fn distinct_instance_count(&self) -> usize {
        let mut seen: Vec<(u32, Option<u64>)> = Vec::new();
        for record in &self.instances {
            let key = (record.pid, record.start_time);
            if !seen.contains(&key) {
                seen.push(key);
            }
        }
        seen.len()
    }

    /// The conflict recovery hint when more than one live daemon answers this
    /// scope, or `None` when at most one does.
    #[must_use]
    pub(crate) fn conflict_recovery(&self) -> Option<String> {
        if self.distinct_instance_count() <= 1 {
            return None;
        }
        let mut pids: Vec<u32> = self.instances.iter().map(|record| record.pid).collect();
        pids.sort_unstable();
        pids.dedup();
        let pids = pids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!(
            "more than one live daemon answers this scope (PIDs {pids}); the version \
             probe cannot be attributed to one instance, so nothing was stopped. Run \
             `anvil doctor --fix` to repair, or `anvil intercept stop` then `anvil start`"
        ))
    }
}

/// Complete result of stopping every per-user daemon candidate.
#[derive(Debug, Default)]
pub(crate) struct DaemonStopBatch {
    pub(crate) signalled_pids: Vec<u32>,
    pub(crate) canonical_error: Option<String>,
    pub(crate) sibling_errors: Vec<String>,
}

/// Successful stop → wait → start recycle, with versions for operator report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DaemonRecycleReport {
    pub before: String,
    pub after: String,
}

/// Typed result of [`recycle_daemon_if_version_skew`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DaemonRecycleOutcome {
    /// Versions already match; daemon left running.
    Skipped { version: String },
    /// No live daemon answered a version probe.
    NotRunning,
    /// Stopped the skewed daemon, waited for PID exit, started the current binary.
    Recycled { before: String, after: String },
    /// Recycle was attempted but stop, wait, or start failed.
    Failed {
        before: Option<String>,
        recovery: String,
    },
}

/// Ensure outcome plus optional recycle report (before/after versions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveTimeDaemonOutcome {
    pub ensure: EnsureOutcome,
    pub recycle: Option<DaemonRecycleReport>,
}

impl SaveTimeDaemonOutcome {
    #[must_use]
    pub(crate) fn from_ensure(ensure: EnsureOutcome) -> Self {
        Self {
            ensure,
            recycle: None,
        }
    }

    #[must_use]
    pub(crate) fn failed(&self) -> bool {
        matches!(self.ensure, EnsureOutcome::Failed { .. })
    }
}

/// Injected probe + lifecycle so recycle is unit-testable without a real daemon.
pub(crate) trait DaemonRecycleHooks {
    fn running_daemon(&self) -> Option<RunningDaemon>;
    /// Stop the instances in `running` and nothing else.
    fn stop_daemon(&self, running: &RunningDaemon) -> Result<DaemonStopBatch, String>;
    fn wait_for_pid_exit(&self, pid: u32) -> Result<(), String>;
    fn start_current_binary(&self) -> Result<String, String>;
}

/// Recycle the Anvil-owned daemon when its version differs from `cli_version`.
///
/// Matching versions skip (no stop). A missing daemon is [`NotRunning`] so
/// the caller can fall through to ordinary ensure/start. Does not touch
/// harness MCP children. The stop is bound to the instances observed with the
/// skewed version; if a concurrent caller has already replaced them, the
/// replacement is re-probed rather than signalled, and the replacement this
/// recycle starts must report exactly `cli_version` before it counts.
pub(crate) fn recycle_daemon_if_version_skew(
    cli_version: &str,
    hooks: &dyn DaemonRecycleHooks,
) -> DaemonRecycleOutcome {
    let Some(running) = hooks.running_daemon() else {
        return DaemonRecycleOutcome::NotRunning;
    };
    if running.version == cli_version {
        return DaemonRecycleOutcome::Skipped {
            version: running.version,
        };
    }
    // Two live instances behind one version probe: the skew seen on one
    // connection says nothing about the other process, so refuse to signal
    // either and report the conflict instead (JREL-004).
    if let Some(recovery) = running.conflict_recovery() {
        return DaemonRecycleOutcome::Failed {
            before: Some(running.version),
            recovery,
        };
    }

    let stop = match hooks.stop_daemon(&running) {
        Ok(stop) => stop,
        Err(recovery) => {
            return DaemonRecycleOutcome::Failed {
                before: Some(running.version),
                recovery,
            };
        }
    };
    let before = running.version;
    let DaemonStopBatch {
        signalled_pids,
        canonical_error,
        sibling_errors,
    } = stop;
    if signalled_pids.is_empty() {
        if let Some(recovery) = canonical_error {
            return DaemonRecycleOutcome::Failed {
                before: Some(before),
                recovery,
            };
        }
        // Nothing observed was still there to signal: the skewed daemon
        // exited, or a concurrent caller already replaced it. Re-probe and
        // decide on what answers now — a replacement at the CLI's version is
        // the outcome this recycle wanted, so it is reused, never stopped.
        return match hooks.running_daemon() {
            None => DaemonRecycleOutcome::NotRunning,
            Some(now) if now.version == cli_version => DaemonRecycleOutcome::Skipped {
                version: now.version,
            },
            Some(_) => DaemonRecycleOutcome::Failed {
                before: Some(before),
                recovery: "could not stop the skewed daemon (no PID file names the \
                           instance that answered); run `anvil intercept stop` then \
                           `anvil start`"
                    .to_owned(),
            },
        };
    }

    let mut stop_failures = Vec::new();
    for pid in signalled_pids {
        if let Err(recovery) = hooks.wait_for_pid_exit(pid) {
            stop_failures.push(recovery);
        }
    }
    if let Some(recovery) = canonical_error {
        stop_failures.push(recovery);
    }
    // Sibling PID-file record errors (malformed/unproven/ungated) are not a live
    // daemon. Spec item 9 / MF-1: they must not abort restart after a signal.
    if !sibling_errors.is_empty() {
        tracing::debug!(
            skipped = sibling_errors.len(),
            errors = ?sibling_errors,
            "recycle observed sibling PID-file record errors; not aborting restart"
        );
    }
    if !stop_failures.is_empty() {
        return DaemonRecycleOutcome::Failed {
            before: Some(before),
            recovery: format!(
                "could not safely stop every daemon candidate: {}",
                stop_failures.join("; ")
            ),
        };
    }

    match hooks.start_current_binary() {
        Ok(after) if after == before => DaemonRecycleOutcome::Failed {
            before: Some(before),
            recovery: format!(
                "recycled daemon still reports version {after}; \
                 run `anvil intercept stop` then `anvil start`"
            ),
        },
        // The replacement is only the intended instance when it answers with
        // this binary's version; an unreadable or third version means some
        // other daemon took the endpoint and the operator must look.
        Ok(after) if after != cli_version => DaemonRecycleOutcome::Failed {
            before: Some(before),
            recovery: format!(
                "recycled daemon reports version {after}, expected {cli_version}; \
                 run `anvil intercept status` to identify it, then `anvil intercept stop` \
                 and `anvil start`"
            ),
        },
        Ok(after) => DaemonRecycleOutcome::Recycled { before, after },
        Err(recovery) => DaemonRecycleOutcome::Failed {
            before: Some(before),
            recovery,
        },
    }
}

/// Ensure the save-time daemon, recycling first when `MaySpawn` and versions diverge.
pub(crate) fn ensure_save_time_daemon_with_recycle(
    capability: StartCapability,
    cli_version: &str,
    hooks: &dyn DaemonRecycleHooks,
    launch: impl FnOnce(StartCapability) -> EnsureOutcome,
) -> SaveTimeDaemonOutcome {
    if matches!(capability, StartCapability::MaySpawn) {
        match recycle_daemon_if_version_skew(cli_version, hooks) {
            DaemonRecycleOutcome::Recycled { before, after } => {
                return SaveTimeDaemonOutcome {
                    ensure: EnsureOutcome::Started,
                    recycle: Some(DaemonRecycleReport { before, after }),
                };
            }
            DaemonRecycleOutcome::Failed { recovery, .. } => {
                return SaveTimeDaemonOutcome {
                    ensure: EnsureOutcome::Failed { recovery },
                    recycle: None,
                };
            }
            DaemonRecycleOutcome::Skipped { .. } | DaemonRecycleOutcome::NotRunning => {}
        }
    }
    SaveTimeDaemonOutcome::from_ensure(launch(capability))
}

/// Live hooks: status probe, `request_daemon_stop`, PID wait, detached ensure.
#[cfg(any(unix, windows))]
pub(crate) struct LiveDaemonRecycleHooks;

#[cfg(any(unix, windows))]
impl DaemonRecycleHooks for LiveDaemonRecycleHooks {
    fn running_daemon(&self) -> Option<RunningDaemon> {
        // Snapshot the PID-file instances before the version probe: a daemon
        // replaced between the two steps then shows as a mismatch at stop
        // time (never signalled) instead of a stale instance being trusted.
        let instances = anvil_intercept::snapshot_live_daemon_pid_records().unwrap_or_default();
        let status = crate::commands::intercept::query_daemon_status().ok()?;
        Some(RunningDaemon {
            version: status.health.version,
            instances,
        })
    }

    #[cfg(unix)]
    fn stop_daemon(&self, running: &RunningDaemon) -> Result<DaemonStopBatch, String> {
        let reports = anvil_intercept::request_daemon_stop_all_matching(&running.instances)
            .map_err(|err| format!("{err:#}"))?;
        Ok(daemon_stop_batch_from_reports(reports))
    }

    #[cfg(windows)]
    fn stop_daemon(&self, running: &RunningDaemon) -> Result<DaemonStopBatch, String> {
        use anvil_intercept::StopOutcome;

        match anvil_intercept::request_daemon_stop_matching(&running.instances) {
            Ok(StopOutcome::Signalled { pid }) => Ok(DaemonStopBatch {
                signalled_pids: vec![pid],
                canonical_error: None,
                sibling_errors: Vec::new(),
            }),
            Ok(StopOutcome::StaleCleared { .. } | StopOutcome::NotRunning) => {
                Ok(DaemonStopBatch::default())
            }
            Err(err) => Err(format!("{err:#}")),
        }
    }

    fn wait_for_pid_exit(&self, pid: u32) -> Result<(), String> {
        if anvil_intercept::wait_for_pid_exit(pid, PID_EXIT_TIMEOUT) {
            Ok(())
        } else {
            Err(format!(
                "daemon pid {pid} did not exit after stop; wait until that \
                 process has exited, then run `anvil start`"
            ))
        }
    }

    fn start_current_binary(&self) -> Result<String, String> {
        match crate::commands::intercept::launch_save_time_daemon(StartCapability::MaySpawn) {
            EnsureOutcome::Started | EnsureOutcome::Reused => Ok(query_version_or_unknown()),
            EnsureOutcome::Failed { recovery } => Err(recovery),
            EnsureOutcome::NoStart { reason } => {
                Err(format!("daemon not started ({})", reason.as_str()))
            }
        }
    }
}

#[cfg(unix)]
fn daemon_stop_batch_from_reports(reports: Vec<anvil_intercept::StopReport>) -> DaemonStopBatch {
    use anvil_intercept::StopOutcome;

    let mut batch = DaemonStopBatch::default();
    for (index, report) in reports.into_iter().enumerate() {
        match report.outcome {
            Ok(StopOutcome::Signalled { pid }) => batch.signalled_pids.push(pid),
            Ok(StopOutcome::StaleCleared { .. } | StopOutcome::NotRunning) => {}
            Err(error) => {
                let error = format!("{}: {error}", report.pid_file.display());
                if index == 0 {
                    batch.canonical_error = Some(error);
                } else {
                    batch.sibling_errors.push(error);
                }
            }
        }
    }
    batch
}

#[cfg(any(unix, windows))]
fn query_version_or_unknown() -> String {
    crate::commands::intercept::query_daemon_status()
        .map_or_else(|_| "unknown".to_owned(), |status| status.health.version)
}

/// Human-readable ensure line, including before/after versions when recycled.
#[must_use]
pub(crate) fn format_save_time_daemon_outcome(outcome: &SaveTimeDaemonOutcome) -> String {
    if let Some(recycle) = &outcome.recycle {
        return format!("daemon: recycled ({} → {})", recycle.before, recycle.after);
    }
    match &outcome.ensure {
        EnsureOutcome::Reused => "daemon: running".to_owned(),
        EnsureOutcome::Started => "daemon: started".to_owned(),
        EnsureOutcome::NoStart { reason } => {
            format!("daemon: not started ({})", reason.as_str())
        }
        EnsureOutcome::Failed { recovery } => format!("daemon: failed — {recovery}"),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use anvil_intercept::ensure::NoStartReason;

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum RecycleCall {
        Stop,
        Wait(u32),
        Start,
    }

    fn observed_instance(pid: u32, start_time: u64) -> anvil_intercept::DaemonPidRecord {
        anvil_intercept::DaemonPidRecord {
            pid_file: std::path::PathBuf::from("/runtime/anvil/intercept.pid"),
            pid,
            start_time: Some(start_time),
        }
    }

    struct RecordingHooks {
        running: Option<RunningDaemon>,
        stop_pids: Vec<u32>,
        stop_err: Option<String>,
        stop_canonical_error: Option<String>,
        stop_sibling_errors: Vec<String>,
        wait_ok: bool,
        gone_after_stop: bool,
        /// What `running_daemon` answers after a stop, modelling a concurrent
        /// caller's replacement.
        replaced_after_stop: Option<RunningDaemon>,
        start_after: Result<String, String>,
        calls: RefCell<Vec<RecycleCall>>,
        stop_targets: RefCell<Vec<RunningDaemon>>,
    }

    impl Default for RecordingHooks {
        fn default() -> Self {
            Self {
                running: None,
                stop_pids: Vec::new(),
                stop_err: None,
                stop_canonical_error: None,
                stop_sibling_errors: Vec::new(),
                wait_ok: false,
                gone_after_stop: false,
                replaced_after_stop: None,
                start_after: Err("start not configured".into()),
                calls: RefCell::new(Vec::new()),
                stop_targets: RefCell::new(Vec::new()),
            }
        }
    }

    impl RecordingHooks {
        fn skewed() -> Self {
            Self {
                running: Some(RunningDaemon {
                    version: "0.5.1-beta".into(),
                    instances: vec![observed_instance(4242, 99)],
                }),
                stop_pids: vec![4242],
                wait_ok: true,
                start_after: Ok("0.9.2-beta".into()),
                ..Self::default()
            }
        }

        fn matching() -> Self {
            Self {
                running: Some(RunningDaemon {
                    version: "0.9.2-beta".into(),
                    instances: vec![observed_instance(4242, 99)],
                }),
                ..Self::default()
            }
        }

        fn calls(&self) -> Vec<RecycleCall> {
            self.calls.borrow().clone()
        }
    }

    impl DaemonRecycleHooks for RecordingHooks {
        fn running_daemon(&self) -> Option<RunningDaemon> {
            let stopped = self.calls.borrow().contains(&RecycleCall::Stop);
            if stopped && self.gone_after_stop {
                return None;
            }
            if stopped && let Some(replacement) = &self.replaced_after_stop {
                return Some(replacement.clone());
            }
            self.running.clone()
        }

        fn stop_daemon(&self, running: &RunningDaemon) -> Result<DaemonStopBatch, String> {
            self.calls.borrow_mut().push(RecycleCall::Stop);
            self.stop_targets.borrow_mut().push(running.clone());
            if let Some(err) = &self.stop_err {
                return Err(err.clone());
            }
            Ok(DaemonStopBatch {
                signalled_pids: self.stop_pids.clone(),
                canonical_error: self.stop_canonical_error.clone(),
                sibling_errors: self.stop_sibling_errors.clone(),
            })
        }

        fn wait_for_pid_exit(&self, pid: u32) -> Result<(), String> {
            self.calls.borrow_mut().push(RecycleCall::Wait(pid));
            if self.wait_ok {
                Ok(())
            } else {
                Err(format!("pid {pid} did not exit"))
            }
        }

        fn start_current_binary(&self) -> Result<String, String> {
            self.calls.borrow_mut().push(RecycleCall::Start);
            self.start_after.clone()
        }
    }

    #[test]
    fn recycle_on_version_skew_stops_waits_and_starts() {
        let hooks = RecordingHooks::skewed();
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert_eq!(
            outcome,
            DaemonRecycleOutcome::Recycled {
                before: "0.5.1-beta".into(),
                after: "0.9.2-beta".into(),
            }
        );
        assert_eq!(
            hooks.calls(),
            vec![
                RecycleCall::Stop,
                RecycleCall::Wait(4242),
                RecycleCall::Start
            ]
        );
    }

    /// JREL-004: the stop is bound to the instances observed with the skewed
    /// version, never to whatever the PID file names later.
    #[test]
    fn recycle_stops_only_the_instances_it_observed() {
        let hooks = RecordingHooks::skewed();
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(matches!(outcome, DaemonRecycleOutcome::Recycled { .. }));
        assert_eq!(
            hooks.stop_targets.borrow().as_slice(),
            &[RunningDaemon {
                version: "0.5.1-beta".into(),
                instances: vec![observed_instance(4242, 99)],
            }],
            "stop must carry exactly the observed identity"
        );
    }

    /// JREL-004: a concurrent recycle replaced the skewed daemon between this
    /// caller's probe and its stop. Nothing observed is signalled, and the
    /// replacement at the CLI's version is reused — never stopped, never
    /// duplicated by a second start.
    #[test]
    fn recycle_refuses_to_stop_when_two_live_instances_answer_the_scope() {
        let mut hooks = RecordingHooks::skewed();
        hooks.running = Some(RunningDaemon {
            version: "0.5.1-beta".into(),
            instances: vec![observed_instance(4242, 99), observed_instance(5151, 120)],
        });
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        match outcome {
            DaemonRecycleOutcome::Failed { before, recovery } => {
                assert_eq!(before.as_deref(), Some("0.5.1-beta"));
                assert!(
                    recovery.contains("PIDs 4242, 5151") && recovery.contains("anvil doctor --fix"),
                    "the conflict must name both instances and the bounded recovery: {recovery}"
                );
            }
            other => panic!("expected a reported conflict, got {other:?}"),
        }
        assert!(
            hooks.calls().is_empty(),
            "no instance may be signalled when the version probe cannot be attributed"
        );
    }

    #[test]
    fn one_process_recorded_from_two_candidate_directories_is_one_instance() {
        let running = RunningDaemon {
            version: "0.5.1-beta".into(),
            instances: vec![observed_instance(4242, 99), observed_instance(4242, 99)],
        };
        assert_eq!(running.distinct_instance_count(), 1);
        assert!(running.conflict_recovery().is_none());
    }

    #[test]
    fn recycle_reuses_a_concurrent_replacement_at_the_cli_version() {
        let mut hooks = RecordingHooks::skewed();
        hooks.stop_pids.clear();
        hooks.replaced_after_stop = Some(RunningDaemon {
            version: "0.9.2-beta".into(),
            instances: vec![observed_instance(5151, 120)],
        });
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert_eq!(
            outcome,
            DaemonRecycleOutcome::Skipped {
                version: "0.9.2-beta".into(),
            }
        );
        assert_eq!(
            hooks.calls(),
            vec![RecycleCall::Stop],
            "no wait on a PID that was never signalled and no second start"
        );
    }

    /// JREL-004: a replacement that answers with neither the old nor this
    /// binary's version is not the intended instance; readiness alone is not
    /// success.
    #[test]
    fn recycle_fails_when_replacement_reports_an_unexpected_version() {
        let mut hooks = RecordingHooks::skewed();
        hooks.start_after = Ok("0.7.0-beta".into());
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(
            matches!(
                outcome,
                DaemonRecycleOutcome::Failed { ref recovery, .. }
                    if recovery.contains("0.7.0-beta") && recovery.contains("expected 0.9.2-beta")
            ),
            "{outcome:?}"
        );
    }

    /// JREL-004: a replacement whose version cannot be read after start is
    /// not verified; the recycle must not claim success on `unknown`.
    #[test]
    fn recycle_fails_when_replacement_version_is_unknown() {
        let mut hooks = RecordingHooks::skewed();
        hooks.start_after = Ok("unknown".into());
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(
            matches!(outcome, DaemonRecycleOutcome::Failed { .. }),
            "{outcome:?}"
        );
    }

    #[test]
    fn recycle_waits_for_every_signalled_daemon_before_restart() {
        let mut hooks = RecordingHooks::skewed();
        hooks.stop_pids = vec![4242, 4343];
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(matches!(outcome, DaemonRecycleOutcome::Recycled { .. }));
        assert_eq!(
            hooks.calls(),
            vec![
                RecycleCall::Stop,
                RecycleCall::Wait(4242),
                RecycleCall::Wait(4343),
                RecycleCall::Start,
            ],
        );
    }

    #[test]
    fn recycle_waits_for_signalled_daemons_then_starts_despite_sibling_record_error() {
        let mut hooks = RecordingHooks::skewed();
        hooks.stop_pids = vec![4242];
        hooks.stop_sibling_errors = vec!["sibling PID was unproven".into()];
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(matches!(outcome, DaemonRecycleOutcome::Recycled { .. }));
        assert_eq!(
            hooks.calls(),
            vec![
                RecycleCall::Stop,
                RecycleCall::Wait(4242),
                RecycleCall::Start,
            ],
            "sibling PID-file record errors must not abort restart after a signalled daemon exits",
        );
    }

    #[test]
    fn recycle_waits_but_does_not_restart_after_canonical_pid_refusal() {
        let mut hooks = RecordingHooks::skewed();
        hooks.stop_pids = vec![4343];
        hooks.stop_canonical_error = Some("canonical PID instruction is unsafe".into());
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(
            matches!(
                outcome,
                DaemonRecycleOutcome::Failed { ref recovery, .. }
                    if recovery.contains("canonical PID instruction is unsafe")
            ),
            "canonical refusal must fail recycle: {outcome:?}",
        );
        assert_eq!(
            hooks.calls(),
            vec![RecycleCall::Stop, RecycleCall::Wait(4343)],
            "recycle must wait for an already-signalled sibling but never restart after canonical refusal",
        );
    }

    #[cfg(unix)]
    #[test]
    fn stop_batch_preserves_canonical_and_sibling_error_identity() {
        use std::path::PathBuf;

        use anvil_intercept::{StopOutcome, StopReport};

        let batch = daemon_stop_batch_from_reports(vec![
            StopReport {
                pid_file: PathBuf::from("/runtime/anvil/intercept.pid"),
                outcome: Err("canonical PID instruction is unsafe".into()),
            },
            StopReport {
                pid_file: PathBuf::from("/state/anvil/intercept.pid"),
                outcome: Ok(StopOutcome::Signalled { pid: 4343 }),
            },
            StopReport {
                pid_file: PathBuf::from("/legacy/anvil/intercept.pid"),
                outcome: Err("sibling PID was unproven".into()),
            },
        ]);

        assert_eq!(batch.signalled_pids, vec![4343]);
        assert!(
            batch
                .canonical_error
                .as_deref()
                .is_some_and(|error| error.contains("canonical PID instruction is unsafe")),
        );
        assert_eq!(batch.sibling_errors.len(), 1);
        assert!(batch.sibling_errors[0].contains("sibling PID was unproven"));
    }

    #[test]
    fn matching_versions_skip_recycle() {
        let hooks = RecordingHooks::matching();
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert_eq!(
            outcome,
            DaemonRecycleOutcome::Skipped {
                version: "0.9.2-beta".into(),
            }
        );
        assert!(
            hooks.calls().is_empty(),
            "matching versions must not stop or start: {:?}",
            hooks.calls()
        );
    }

    #[test]
    fn missing_daemon_is_not_running() {
        let hooks = RecordingHooks::default();
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert_eq!(outcome, DaemonRecycleOutcome::NotRunning);
        assert!(hooks.calls().is_empty());
    }

    #[test]
    fn stop_none_when_daemon_already_gone_is_not_running() {
        let mut hooks = RecordingHooks::skewed();
        hooks.stop_pids.clear();
        hooks.gone_after_stop = true;
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert_eq!(outcome, DaemonRecycleOutcome::NotRunning);
        assert_eq!(hooks.calls(), vec![RecycleCall::Stop]);
    }

    #[test]
    fn stop_none_while_daemon_still_visible_fails() {
        let mut hooks = RecordingHooks::skewed();
        hooks.stop_pids.clear();
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(
            matches!(outcome, DaemonRecycleOutcome::Failed { .. }),
            "{outcome:?}"
        );
        assert_eq!(hooks.calls(), vec![RecycleCall::Stop]);
    }

    #[test]
    fn start_that_still_reports_old_version_fails() {
        let mut hooks = RecordingHooks::skewed();
        hooks.start_after = Ok("0.5.1-beta".into());
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(
            matches!(outcome, DaemonRecycleOutcome::Failed { .. }),
            "{outcome:?}"
        );
        assert_eq!(
            hooks.calls(),
            vec![
                RecycleCall::Stop,
                RecycleCall::Wait(4242),
                RecycleCall::Start
            ]
        );
    }

    #[test]
    fn wait_failure_does_not_start() {
        let mut hooks = RecordingHooks::skewed();
        hooks.wait_ok = false;
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(
            matches!(outcome, DaemonRecycleOutcome::Failed { .. }),
            "{outcome:?}"
        );
        assert_eq!(
            hooks.calls(),
            vec![RecycleCall::Stop, RecycleCall::Wait(4242)]
        );
    }

    #[test]
    fn recycle_attempts_every_pid_wait_when_one_daemon_sticks() {
        let mut hooks = RecordingHooks::skewed();
        hooks.stop_pids = vec![4242, 4343];
        hooks.wait_ok = false;
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert!(matches!(outcome, DaemonRecycleOutcome::Failed { .. }));
        assert_eq!(
            hooks.calls(),
            vec![
                RecycleCall::Stop,
                RecycleCall::Wait(4242),
                RecycleCall::Wait(4343),
            ],
        );
    }

    #[test]
    fn ensure_may_spawn_recycles_on_skew() {
        let hooks = RecordingHooks::skewed();
        let launched = RefCell::new(false);
        let outcome = ensure_save_time_daemon_with_recycle(
            StartCapability::MaySpawn,
            "0.9.2-beta",
            &hooks,
            |_| {
                *launched.borrow_mut() = true;
                EnsureOutcome::Started
            },
        );
        assert_eq!(outcome.ensure, EnsureOutcome::Started);
        assert_eq!(
            outcome.recycle,
            Some(DaemonRecycleReport {
                before: "0.5.1-beta".into(),
                after: "0.9.2-beta".into(),
            })
        );
        assert!(
            !*launched.borrow(),
            "recycle already starts; ensure must not launch again"
        );
        assert_eq!(
            hooks.calls(),
            vec![
                RecycleCall::Stop,
                RecycleCall::Wait(4242),
                RecycleCall::Start
            ]
        );
    }

    #[test]
    fn ensure_may_spawn_skips_recycle_when_versions_match() {
        let hooks = RecordingHooks::matching();
        let launched = RefCell::new(false);
        let outcome = ensure_save_time_daemon_with_recycle(
            StartCapability::MaySpawn,
            "0.9.2-beta",
            &hooks,
            |_| {
                *launched.borrow_mut() = true;
                EnsureOutcome::Reused
            },
        );
        assert_eq!(outcome.ensure, EnsureOutcome::Reused);
        assert_eq!(outcome.recycle, None);
        assert!(
            *launched.borrow(),
            "matching versions fall through to ensure"
        );
        assert!(hooks.calls().is_empty());
    }

    #[test]
    fn ensure_no_spawn_does_not_recycle_skewed_daemon() {
        let hooks = RecordingHooks::skewed();
        let outcome = ensure_save_time_daemon_with_recycle(
            StartCapability::NoSpawn(NoStartReason::NonInteractive),
            "0.9.2-beta",
            &hooks,
            |cap| {
                assert_eq!(cap, StartCapability::NoSpawn(NoStartReason::NonInteractive));
                EnsureOutcome::Reused
            },
        );
        assert_eq!(outcome.ensure, EnsureOutcome::Reused);
        assert_eq!(outcome.recycle, None);
        assert!(
            hooks.calls().is_empty(),
            "NoSpawn must not stop a live daemon it cannot restart: {:?}",
            hooks.calls()
        );
    }

    #[test]
    fn format_reports_before_and_after_versions() {
        let outcome = SaveTimeDaemonOutcome {
            ensure: EnsureOutcome::Started,
            recycle: Some(DaemonRecycleReport {
                before: "0.5.1-beta".into(),
                after: "0.9.2-beta".into(),
            }),
        };
        let line = format_save_time_daemon_outcome(&outcome);
        assert_eq!(line, "daemon: recycled (0.5.1-beta → 0.9.2-beta)");
    }
}
