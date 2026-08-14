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

/// Snapshot of a live daemon's version string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunningDaemon {
    pub version: String,
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
    fn stop_daemon(&self) -> Result<Option<u32>, String>;
    fn wait_for_pid_exit(&self, pid: u32) -> Result<(), String>;
    fn start_current_binary(&self) -> Result<String, String>;
}

/// Recycle the Anvil-owned daemon when its version differs from `cli_version`.
///
/// Matching versions skip (no stop). A missing daemon is [`NotRunning`] so
/// the caller can fall through to ordinary ensure/start. Does not touch
/// harness MCP children.
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

    let before = running.version;
    let pid = match hooks.stop_daemon() {
        Ok(Some(pid)) => pid,
        Ok(None) => {
            // Race: the daemon can exit between the version probe and stop.
            // Re-probe; if it is gone, fall through to ordinary ensure.
            return if hooks.running_daemon().is_none() {
                DaemonRecycleOutcome::NotRunning
            } else {
                DaemonRecycleOutcome::Failed {
                    before: Some(before),
                    recovery: "could not stop the skewed daemon (no PID file); \
                               run `anvil intercept stop` then `anvil start`"
                        .to_owned(),
                }
            };
        }
        Err(recovery) => {
            return DaemonRecycleOutcome::Failed {
                before: Some(before),
                recovery,
            };
        }
    };

    if let Err(recovery) = hooks.wait_for_pid_exit(pid) {
        return DaemonRecycleOutcome::Failed {
            before: Some(before),
            recovery,
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
        let status = crate::commands::intercept::query_daemon_status().ok()?;
        Some(RunningDaemon {
            version: status.health.version,
        })
    }

    fn stop_daemon(&self) -> Result<Option<u32>, String> {
        use anvil_intercept::StopOutcome;

        match anvil_intercept::request_daemon_stop() {
            Ok(StopOutcome::Signalled { pid } | StopOutcome::StaleCleared { pid }) => Ok(Some(pid)),
            Ok(StopOutcome::NotRunning) => Ok(None),
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

    struct RecordingHooks {
        running: Option<RunningDaemon>,
        stop_pid: Option<u32>,
        stop_err: Option<String>,
        wait_ok: bool,
        gone_after_stop: bool,
        start_after: Result<String, String>,
        calls: RefCell<Vec<RecycleCall>>,
    }

    impl Default for RecordingHooks {
        fn default() -> Self {
            Self {
                running: None,
                stop_pid: None,
                stop_err: None,
                wait_ok: false,
                gone_after_stop: false,
                start_after: Err("start not configured".into()),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl RecordingHooks {
        fn skewed() -> Self {
            Self {
                running: Some(RunningDaemon {
                    version: "0.5.1-beta".into(),
                }),
                stop_pid: Some(4242),
                wait_ok: true,
                start_after: Ok("0.9.2-beta".into()),
                ..Self::default()
            }
        }

        fn matching() -> Self {
            Self {
                running: Some(RunningDaemon {
                    version: "0.9.2-beta".into(),
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
            if self.gone_after_stop && self.calls.borrow().contains(&RecycleCall::Stop) {
                return None;
            }
            self.running.clone()
        }

        fn stop_daemon(&self) -> Result<Option<u32>, String> {
            self.calls.borrow_mut().push(RecycleCall::Stop);
            if let Some(err) = &self.stop_err {
                return Err(err.clone());
            }
            Ok(self.stop_pid)
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
        hooks.stop_pid = None;
        hooks.gone_after_stop = true;
        let outcome = recycle_daemon_if_version_skew("0.9.2-beta", &hooks);
        assert_eq!(outcome, DaemonRecycleOutcome::NotRunning);
        assert_eq!(hooks.calls(), vec![RecycleCall::Stop]);
    }

    #[test]
    fn stop_none_while_daemon_still_visible_fails() {
        let mut hooks = RecordingHooks::skewed();
        hooks.stop_pid = None;
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
