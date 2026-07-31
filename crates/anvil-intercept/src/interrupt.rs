//! INTD-006: process-group / Job Object interrupt ladder.
//!
//! Escalating stop signals when enforcement decides `interrupt`, from soft
//! cancel through hard kill of the agent process group.

use std::time::{Duration, Instant};

use anvil_intercept_proto::SessionRecord;

/// Outcome of an interrupt attempt against a registered session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterruptOutcome {
    /// At least one signal in the ladder reached the target and the
    /// leader is no longer running. Carries the stage that finally
    /// resolved the process — useful for telemetry that distinguishes
    /// "agent quit gracefully on SIGTERM" from "we had to SIGKILL".
    Stopped { stage: InterruptStage },
    /// The interrupt path could not safely complete — PID-reuse
    /// detected, leader vanished, or any signal call surfaced an
    /// error. The daemon's enforcement pipeline MUST fence the
    /// worktree on this outcome regardless of operator config (AD-7).
    FenceImmediately { reason: FenceReason },
}

/// Stages of the interrupt ladder. `Liveness` is the "no signal
/// needed" early-exit when the leader was already gone; subsequent
/// variants name the signal that actually stopped the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptStage {
    /// The leader was already gone before we sent any signal —
    /// nothing to do. Caller still applies the enforcement decision
    /// (the change has already happened) but skips the fence.
    AlreadyExited,
    Sigint,
    Sigterm,
    Sigkill,
    /// Windows: Job Object termination succeeded.
    JobObjectTerminated,
}

/// Why the daemon decided to fence rather than (or in addition to)
/// signalling. Surfaced in telemetry so operators can audit what
/// happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenceReason {
    /// The session record had no PID — typically means the launcher
    /// crashed before it could update the registry. Without a PID we
    /// cannot signal anything; fence and surface the failure.
    LeaderPidUnknown,
    /// PID-reuse defence: the leader PID exists but its start time
    /// no longer matches the value the registry recorded at
    /// registration time. Some other process now wears that PID;
    /// signalling it would attack the wrong workload.
    PidReuseMismatch,
    /// A signal call failed with an error other than `ESRCH`. The
    /// inner string is the OS error message — surfaced to telemetry
    /// for triage, never re-parsed for control flow.
    SignalDeliveryFailed {
        stage: InterruptStage,
        error: String,
    },
    /// Windows: `TerminateJobObject` returned non-zero. The string
    /// is the OS error message.
    JobObjectTerminationFailed { error: String },
}

/// Configuration for the interrupt ladder. Defaults match
/// pitchfork's adaptive 10 ms / 50 ms poll, and a worst-case 1.5 s
/// total budget that fits inside ADR-031's enforcement-latency cap.
#[derive(Debug, Clone, Copy)]
pub struct InterruptConfig {
    pub sigint_to_sigterm: Duration,
    pub sigterm_to_sigkill: Duration,
    /// Per-stage poll cadence after sending a signal — how often
    /// the loop checks whether the leader has exited before
    /// escalating.
    pub poll_interval_short: Duration,
    pub poll_interval_long: Duration,
}

impl Default for InterruptConfig {
    fn default() -> Self {
        Self {
            sigint_to_sigterm: Duration::from_millis(500),
            sigterm_to_sigkill: Duration::from_secs(1),
            poll_interval_short: Duration::from_millis(10),
            poll_interval_long: Duration::from_millis(50),
        }
    }
}

/// Synchronous trait the interrupt ladder calls into. Real
/// implementations live in [`unix`] and [`windows_impl`]; tests
/// substitute a recording double so we can assert the ladder shape
/// without actually killing processes.
///
/// The trait is split per-method because the ladder needs them at
/// different times — keeping them as separate methods means a test
/// double can drive each step independently.
pub trait InterruptOps: Send + Sync {
    /// Verify the leader PID is still live AND that its recorded
    /// start time matches `expected_start_time` (PID-reuse defence).
    /// Returns:
    /// - `Ok(true)`: the PID is alive and the start time matches; safe to signal.
    /// - `Ok(false)`: the PID is gone (already exited).
    /// - `Err(FenceReason::PidReuseMismatch)`: PID is alive but the
    ///   start time is different — some other process wears it now.
    fn verify_leader(
        &self,
        pid: u32,
        expected_start_time: Option<u64>,
    ) -> Result<bool, FenceReason>;

    /// Send `signal` to the leader PID and the process group.
    /// Returns `Ok(())` on success or if the target was already
    /// gone (`ESRCH` on the leader is a natural success because the
    /// process exited before we got there).
    fn send_signal(
        &self,
        pid: u32,
        process_group_id: Option<i32>,
        signal: Signal,
    ) -> Result<(), String>;

    /// Poll whether the leader is still running. Used between
    /// stages of the ladder to decide if we need to escalate.
    /// `Ok(false)` means "leader exited"; an error here is treated
    /// as "still alive" so a transient `/proc` read failure does
    /// not silently skip escalation.
    fn leader_alive(&self, pid: u32) -> bool;
}

/// Signal vocabulary used by the ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    Int,
    Term,
    Kill,
}

/// Run the Unix process-group interrupt ladder against `record`. The
/// ladder is synchronous: each stage sends the signal, then polls
/// `leader_alive` until either the leader exits or the
/// stage-specific timeout expires.
///
/// Tests construct a recording [`InterruptOps`] double and verify
/// the sequence of operations; production wires
/// [`unix::SystemInterruptOps`] (Unix) or
/// [`windows_impl::SystemJobObjectOps`] (Windows).
pub fn run_unix_ladder<O: InterruptOps>(
    ops: &O,
    record: &SessionRecord,
    config: InterruptConfig,
) -> InterruptOutcome {
    let Some(pid) = record.pid else {
        return InterruptOutcome::FenceImmediately {
            reason: FenceReason::LeaderPidUnknown,
        };
    };

    // PID-reuse defence: refuse to signal if the leader's start time
    // does not match what the registry recorded. `started_at_unix`
    // on the wire is registration time; we compare against the OS-
    // reported process start time inside `verify_leader`.
    let expected_start = if record.started_at_unix == 0 {
        None
    } else {
        Some(record.started_at_unix)
    };
    match ops.verify_leader(pid, expected_start) {
        Ok(true) => {}
        Ok(false) => {
            return InterruptOutcome::Stopped {
                stage: InterruptStage::AlreadyExited,
            };
        }
        Err(reason) => return InterruptOutcome::FenceImmediately { reason },
    }

    // Stage 1: SIGINT.
    if let Err(error) = ops.send_signal(pid, record.pgid, Signal::Int) {
        return InterruptOutcome::FenceImmediately {
            reason: FenceReason::SignalDeliveryFailed {
                stage: InterruptStage::Sigint,
                error,
            },
        };
    }
    if poll_until_exit(
        ops,
        pid,
        config.sigint_to_sigterm,
        config.poll_interval_short,
    ) {
        return InterruptOutcome::Stopped {
            stage: InterruptStage::Sigint,
        };
    }

    // Stage 2: SIGTERM.
    if let Err(error) = ops.send_signal(pid, record.pgid, Signal::Term) {
        return InterruptOutcome::FenceImmediately {
            reason: FenceReason::SignalDeliveryFailed {
                stage: InterruptStage::Sigterm,
                error,
            },
        };
    }
    if poll_until_exit(
        ops,
        pid,
        config.sigterm_to_sigkill,
        config.poll_interval_long,
    ) {
        return InterruptOutcome::Stopped {
            stage: InterruptStage::Sigterm,
        };
    }

    // Stage 3: SIGKILL — the always-final stop.
    if let Err(error) = ops.send_signal(pid, record.pgid, Signal::Kill) {
        return InterruptOutcome::FenceImmediately {
            reason: FenceReason::SignalDeliveryFailed {
                stage: InterruptStage::Sigkill,
                error,
            },
        };
    }
    InterruptOutcome::Stopped {
        stage: InterruptStage::Sigkill,
    }
}

fn poll_until_exit<O: InterruptOps>(
    ops: &O,
    pid: u32,
    deadline: Duration,
    interval: Duration,
) -> bool {
    let started = Instant::now();
    while Instant::now().duration_since(started) < deadline {
        if !ops.leader_alive(pid) {
            return true;
        }
        std::thread::sleep(interval);
    }
    !ops.leader_alive(pid)
}

#[cfg(unix)]
pub mod unix {
    //! Production [`InterruptOps`] for Unix targets. Uses `nix`'s
    //! safe wrappers around `libc::kill` so the crate-level
    //! `forbid(unsafe_code)` lint stays honest. The PID-reuse
    //! defence reads `/proc/PID/stat` (Linux) or `proc_pidinfo`
    //! (macOS) — see `process_start_time` in `crate::lib_helpers`.

    use super::{FenceReason, InterruptOps, Signal};

    use nix::errno::Errno;
    use nix::sys::signal::{Signal as NixSignal, kill, killpg};
    use nix::unistd::Pid;

    /// Production `InterruptOps`. Stateless — instantiate with
    /// `SystemInterruptOps` and pass it into [`super::run_unix_ladder`].
    #[derive(Debug, Default, Clone, Copy)]
    pub struct SystemInterruptOps;

    impl InterruptOps for SystemInterruptOps {
        fn verify_leader(
            &self,
            pid: u32,
            expected_start_time: Option<u64>,
        ) -> Result<bool, FenceReason> {
            let Ok(raw) = i32::try_from(pid) else {
                return Ok(false);
            };
            // Liveness probe — `kill(pid, 0)` does not deliver a
            // signal; ESRCH means "no such process".
            match kill(Pid::from_raw(raw), None) {
                Ok(()) => {}
                Err(Errno::ESRCH) => return Ok(false),
                Err(err) => {
                    return Err(FenceReason::SignalDeliveryFailed {
                        stage: super::InterruptStage::Sigint,
                        error: format!("liveness probe failed: {err}"),
                    });
                }
            }
            // PID-reuse defence: only enforce when the registry
            // had a known start time. A `0` start (registration
            // before the process info update) cannot enforce.
            if let Some(expected) = expected_start_time {
                if let Some(actual) = current_process_start_time(pid) {
                    if actual != expected {
                        return Err(FenceReason::PidReuseMismatch);
                    }
                } else {
                    // Could not read the start time; conservative —
                    // surface as PID-reuse mismatch rather than
                    // signalling blind.
                    return Err(FenceReason::PidReuseMismatch);
                }
            }
            Ok(true)
        }

        fn send_signal(
            &self,
            pid: u32,
            process_group_id: Option<i32>,
            signal: Signal,
        ) -> Result<(), String> {
            let nix_signal = match signal {
                Signal::Int => NixSignal::SIGINT,
                Signal::Term => NixSignal::SIGTERM,
                Signal::Kill => NixSignal::SIGKILL,
            };
            let raw_pid =
                i32::try_from(pid).map_err(|_| format!("pid {pid} does not fit in i32"))?;
            // Leader first; ESRCH there is "process already gone"
            // and not a delivery failure for our purposes.
            match kill(Pid::from_raw(raw_pid), nix_signal) {
                Ok(()) => {}
                Err(Errno::ESRCH) => {
                    return Ok(());
                }
                Err(err) => return Err(format!("kill leader: {err}")),
            }
            // Then the process group (if known) so any children
            // the agent spawned also receive the signal. ESRCH on
            // killpg is acceptable too — children may have exited.
            if let Some(group) = process_group_id {
                let group_pid = Pid::from_raw(group);
                match killpg(group_pid, nix_signal) {
                    Ok(()) | Err(Errno::ESRCH) => {}
                    Err(err) => return Err(format!("killpg: {err}")),
                }
            }
            Ok(())
        }

        fn leader_alive(&self, pid: u32) -> bool {
            let Ok(raw) = i32::try_from(pid) else {
                return false;
            };
            !matches!(kill(Pid::from_raw(raw), None), Err(Errno::ESRCH))
        }
    }

    /// Read the current start-time `u64` for `pid` from the OS.
    /// Linux uses `/proc/PID/stat` (field 22); macOS uses
    /// `proc_pidinfo`. Returns `None` if the process cannot be
    /// queried (already exited, permission denied, etc.).
    fn current_process_start_time(pid: u32) -> Option<u64> {
        #[cfg(target_os = "linux")]
        {
            let stat = std::fs::read_to_string(
                std::path::Path::new("/proc")
                    .join(pid.to_string())
                    .join("stat"),
            )
            .ok()?;
            let after_command = stat.rsplit_once(") ")?.1;
            after_command.split_whitespace().nth(19)?.parse().ok()
        }
        #[cfg(target_os = "macos")]
        {
            // V060F-004: read the start time via `proc_pidinfo` so the
            // macOS interrupt ladder runs the same SIGINT→SIGTERM→SIGKILL
            // sequence as Linux instead of the conservative fence-first
            // fallback (which skewed macOS fence telemetry toward
            // `SignalDeliveryFailed`). The `unsafe` FFI is isolated in
            // `crate::macos_process_start_time`.
            crate::macos_process_start_time(pid)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = pid;
            None
        }
    }
}

#[cfg(windows)]
pub mod windows_impl {
    //! Windows interrupt path: terminate the session's Job Object.
    //! All `unsafe` for the underlying Win32 calls is in the
    //! `anvil-intercept-win32` helper crate.

    use super::{FenceReason, InterruptOutcome, InterruptStage};
    use anvil_intercept_proto::SessionRecord;
    use anvil_intercept_win32::{JobObject, terminate_job_object};

    /// Run the Windows interrupt path. Unlike Unix there is no
    /// SIGINT / SIGTERM / SIGKILL ladder — `TerminateJobObject`
    /// stops every process in the job atomically — so the function
    /// signature differs from `run_unix_ladder`. Production
    /// callers fetch the per-session [`JobObject`] from a registry
    /// the daemon owns; the registry lookup is out of scope for
    /// INTD-006 and lives behind the future
    /// `crate::registry::SessionRegistry::job_object_for_session`.
    #[cfg_attr(
        windows,
        allow(
            clippy::collapsible_if,
            reason = "Windows-only clippy debt baselined by CIB-204; clearing it restructures named-pipe transport code that only a Windows runner can build and test."
        )
    )]
    pub fn run_windows_termination(record: &SessionRecord, job: &JobObject) -> InterruptOutcome {
        // PID-reuse defence is still required: the launcher records
        // `started_at_unix`, and we read it against the live
        // process before terminating. If the leader has been
        // recycled, the JOB still owns whatever processes it was
        // assigned to at creation, so terminating the job is safe
        // even on PID reuse — but we surface the mismatch rather
        // than silently terminating to keep the audit trail honest.
        if let Some(pid) = record.pid {
            if record.started_at_unix != 0 {
                match anvil_intercept_win32::process_creation_time(pid) {
                    Ok(Some(actual)) if actual != record.started_at_unix => {
                        return InterruptOutcome::FenceImmediately {
                            reason: FenceReason::PidReuseMismatch,
                        };
                    }
                    Ok(_) => {}
                    Err(err) => {
                        return InterruptOutcome::FenceImmediately {
                            reason: FenceReason::SignalDeliveryFailed {
                                stage: InterruptStage::JobObjectTerminated,
                                error: format!("process_creation_time: {err}"),
                            },
                        };
                    }
                }
            }
        }
        if let Err(err) = terminate_job_object(job) {
            return InterruptOutcome::FenceImmediately {
                reason: FenceReason::JobObjectTerminationFailed {
                    error: err.to_string(),
                },
            };
        }
        InterruptOutcome::Stopped {
            stage: InterruptStage::JobObjectTerminated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_intercept_proto::{SessionId, SessionStatus};
    use std::path::PathBuf;
    use std::sync::Mutex;

    fn record_with(pid: Option<u32>, started: u64) -> SessionRecord {
        SessionRecord {
            id: SessionId::new("sess-test"),
            worktree: PathBuf::from("/tmp"),
            pid,
            pgid: pid.map(|p| i32::try_from(p).unwrap_or(i32::MAX)),
            started_at_unix: started,
            last_heartbeat_unix: started,
            status: SessionStatus::Active,
            agent_tag: None,
            daemon_issued_tag: None,
        }
    }

    /// Recording double for `InterruptOps`. Each method appends to
    /// a vector so tests can assert the exact sequence the ladder
    /// produces; the booleans / start-time fixture configure the
    /// double's behaviour.
    ///
    /// Liveness behaviour is **stage-driven**, not call-count-driven:
    /// the double tracks the most recent signal it was asked to send
    /// and returns the configured liveness for that stage. This keeps
    /// tests robust to tiny timing differences in `poll_until_exit`'s
    /// inner loop — the property under test is "stop after stage X",
    /// not "stop on the Nth poll".
    struct Recording {
        sequence: Mutex<Vec<String>>,
        verify_result: Mutex<Result<bool, FenceReason>>,
        send_results: Mutex<Vec<Result<(), String>>>,
        /// Liveness answer for each post-signal poll stage. If the
        /// daemon never sent a signal yet, we use `Signal::Int`'s
        /// answer as a default starting point — but verify is the
        /// only call that runs before the first signal, and verify
        /// uses `verify_result` not `leader_alive`.
        liveness_after: Mutex<std::collections::HashMap<Signal, bool>>,
        /// The most recently sent signal; advances as the ladder
        /// progresses through stages.
        last_signal: Mutex<Option<Signal>>,
    }

    impl Recording {
        fn new(verify: Result<bool, FenceReason>) -> Self {
            Self {
                sequence: Mutex::new(Vec::new()),
                verify_result: Mutex::new(verify),
                send_results: Mutex::new(Vec::new()),
                liveness_after: Mutex::new(std::collections::HashMap::new()),
                last_signal: Mutex::new(None),
            }
        }

        fn with_send_results(self, results: Vec<Result<(), String>>) -> Self {
            *self.send_results.lock().unwrap() = results;
            self
        }

        /// Configure liveness per stage. A stage absent from the map
        /// defaults to `true` (still alive) so the ladder escalates;
        /// the only stage that can return `false` (resolved) is the
        /// one explicitly mapped to `false`.
        fn with_liveness_after(self, stage: Signal, alive: bool) -> Self {
            self.liveness_after.lock().unwrap().insert(stage, alive);
            self
        }

        fn calls(&self) -> Vec<String> {
            self.sequence.lock().unwrap().clone()
        }
    }

    impl InterruptOps for Recording {
        fn verify_leader(&self, pid: u32, expected: Option<u64>) -> Result<bool, FenceReason> {
            self.sequence
                .lock()
                .unwrap()
                .push(format!("verify({pid}, {expected:?})"));
            let mut guard = self.verify_result.lock().unwrap();
            std::mem::replace(&mut *guard, Ok(true))
        }

        fn send_signal(
            &self,
            pid: u32,
            process_group_id: Option<i32>,
            signal: Signal,
        ) -> Result<(), String> {
            self.sequence.lock().unwrap().push(format!(
                "send({pid}, pgid={process_group_id:?}, {signal:?})"
            ));
            *self.last_signal.lock().unwrap() = Some(signal);
            let mut results = self.send_results.lock().unwrap();
            if results.is_empty() {
                Ok(())
            } else {
                results.remove(0)
            }
        }

        fn leader_alive(&self, pid: u32) -> bool {
            let alive = match *self.last_signal.lock().unwrap() {
                Some(stage) => self
                    .liveness_after
                    .lock()
                    .unwrap()
                    .get(&stage)
                    .copied()
                    .unwrap_or(true),
                None => true, // Before any signal: default alive.
            };
            self.sequence
                .lock()
                .unwrap()
                .push(format!("alive({pid}) -> {alive}"));
            alive
        }
    }

    fn fast_config() -> InterruptConfig {
        // Tiny stage timeouts so the ladder under test runs in
        // milliseconds; the unit tests do not depend on the wall-clock
        // absolute values, only on which stages fire.
        InterruptConfig {
            sigint_to_sigterm: Duration::from_millis(2),
            sigterm_to_sigkill: Duration::from_millis(2),
            poll_interval_short: Duration::from_millis(1),
            poll_interval_long: Duration::from_millis(1),
        }
    }

    /// Test (a): happy path SIGTERM delivery. The leader stays
    /// alive through the SIGINT poll, exits when SIGTERM lands.
    /// Outcome is `Stopped { stage: Sigterm }`.
    #[test]
    fn unix_happy_path_sigterm_stops_leader() {
        let record = record_with(Some(1234), 100);
        let ops = Recording::new(Ok(true))
            // After SIGINT: still alive (escalate to SIGTERM).
            .with_liveness_after(Signal::Int, true)
            // After SIGTERM: exited cleanly.
            .with_liveness_after(Signal::Term, false);

        let outcome = run_unix_ladder(&ops, &record, fast_config());
        assert_eq!(
            outcome,
            InterruptOutcome::Stopped {
                stage: InterruptStage::Sigterm,
            }
        );
        let calls = ops.calls();
        assert!(calls.iter().any(|c| c.contains("Int")));
        assert!(calls.iter().any(|c| c.contains("Term")));
        assert!(
            !calls.iter().any(|c| c.contains("Kill")),
            "SIGKILL must not be reached when SIGTERM resolves: {calls:?}",
        );
    }

    /// Test (b): PID-reuse defence rejects mismatch — the recorded
    /// start time differs from the OS-reported start time, so the
    /// outcome is `FenceImmediately { PidReuseMismatch }`.
    /// Pinned because pitchfork does not implement this defence;
    /// it is INTD-006's load-bearing addition.
    #[test]
    fn pid_reuse_mismatch_fences_without_signalling() {
        let record = record_with(Some(1234), 100);
        let ops = Recording::new(Err(FenceReason::PidReuseMismatch));

        let outcome = run_unix_ladder(&ops, &record, fast_config());
        assert_eq!(
            outcome,
            InterruptOutcome::FenceImmediately {
                reason: FenceReason::PidReuseMismatch
            }
        );
        let calls = ops.calls();
        assert_eq!(
            calls.len(),
            1,
            "no signal must be sent on PID-reuse mismatch: {calls:?}",
        );
        assert!(calls[0].starts_with("verify("));
    }

    /// Test (c): SIGTERM unanswered escalates to SIGKILL. Pin the
    /// stage explicitly because regression here would mean an
    /// uninterruptible agent could hold the daemon hostage at
    /// SIGTERM.
    #[test]
    fn unix_sigterm_unanswered_escalates_to_sigkill() {
        let record = record_with(Some(1234), 100);
        // Default-alive (no entries); leader survives every stage.
        let ops = Recording::new(Ok(true));
        let outcome = run_unix_ladder(&ops, &record, fast_config());
        assert_eq!(
            outcome,
            InterruptOutcome::Stopped {
                stage: InterruptStage::Sigkill,
            }
        );
        let calls = ops.calls();
        assert!(
            calls.iter().any(|c| c.contains("Int"))
                && calls.iter().any(|c| c.contains("Term"))
                && calls.iter().any(|c| c.contains("Kill")),
            "all three stages must fire: {calls:?}",
        );
    }

    /// Test (d): a signal-delivery failure lands as `FenceImmediately`
    /// with the failing stage recorded — the daemon must fence
    /// regardless of where in the ladder we hit the failure.
    #[test]
    fn signal_delivery_failure_at_sigint_fences() {
        let record = record_with(Some(1234), 100);
        let ops =
            Recording::new(Ok(true)).with_send_results(vec![Err("permission denied".to_string())]);

        let outcome = run_unix_ladder(&ops, &record, fast_config());
        match outcome {
            InterruptOutcome::FenceImmediately {
                reason: FenceReason::SignalDeliveryFailed { stage, error },
            } => {
                assert_eq!(stage, InterruptStage::Sigint);
                assert!(error.contains("permission denied"));
            }
            other => panic!("expected fence on signal failure, got {other:?}"),
        }
    }

    /// Leader was already gone before any signal — the outcome is
    /// `AlreadyExited` (success-equivalent) and no signal fires.
    #[test]
    fn leader_already_exited_before_signal_returns_already_exited() {
        let record = record_with(Some(1234), 100);
        let ops = Recording::new(Ok(false));
        let outcome = run_unix_ladder(&ops, &record, fast_config());
        assert_eq!(
            outcome,
            InterruptOutcome::Stopped {
                stage: InterruptStage::AlreadyExited,
            }
        );
    }

    /// Missing PID on the session record fences immediately —
    /// without a PID the daemon cannot signal anyone, and the
    /// safe action is to fence.
    #[test]
    fn missing_pid_fences_immediately() {
        let record = record_with(None, 0);
        let ops = Recording::new(Ok(true));
        let outcome = run_unix_ladder(&ops, &record, fast_config());
        assert_eq!(
            outcome,
            InterruptOutcome::FenceImmediately {
                reason: FenceReason::LeaderPidUnknown
            }
        );
        // verify_leader must NOT have been called — without a pid
        // there is nothing to verify against.
        assert!(ops.calls().is_empty());
    }

    /// SIGINT itself can resolve the agent — pin that path so a
    /// future refactor cannot collapse "SIGINT exited" and "SIGTERM
    /// exited" into the same observable outcome.
    #[test]
    fn unix_sigint_alone_can_stop_a_well_behaved_agent() {
        let record = record_with(Some(1234), 100);
        let ops = Recording::new(Ok(true)).with_liveness_after(Signal::Int, false);

        let outcome = run_unix_ladder(&ops, &record, fast_config());
        assert_eq!(
            outcome,
            InterruptOutcome::Stopped {
                stage: InterruptStage::Sigint,
            }
        );
        let calls = ops.calls();
        assert!(
            !calls.iter().any(|c| c.contains("Term")),
            "SIGTERM must not fire after SIGINT resolves: {calls:?}",
        );
    }
}
