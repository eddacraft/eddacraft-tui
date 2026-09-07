//! DSV-047 (ADR-101 decision 1): the daemon-side supervisor for detached
//! save-time driver children.
//!
//! One detached `anvil watch --save-time-driver --worktree <canonical>` child
//! runs per durable registered worktree (ADR-064 keeps tree-sitter/notify out
//! of the daemon; the driver child carries them instead). The supervisor:
//!
//! - consumes [`MembershipChange`] events from the session registry's
//!   membership hook (ACTMO-014). The hook only **enqueues** — it fires
//!   synchronously inside `session.register` handling, so spawn and PID-file
//!   I/O must never run on the registry call path. The supervisor drains the
//!   queue on its own task.
//! - spawns children through the [`DaemonLauncher`] seam (the DLIFE launcher
//!   pattern; [`crate::ensure::DetachedCommandLauncher`] in production, a fake
//!   in tests) and terminates them through the [`ProcessControl`] seam.
//! - maintains a PID registry under `<runtime>/save-time-drivers/` —
//!   `<stem>.pid` beside the child's findings log (`<stem>.log`, owned by the
//!   child end-to-end) and its crash-capture file (`<stem>.spawn.log`, the
//!   supervisor-redirected stdout/stderr — never the findings log).
//! - never respawns a child on its own: a child that dies while the daemon
//!   lives reports `failed` honestly until the registry signals the worktree
//!   again. JREL-003: that signal is a durable **refresh** — the public
//!   re-run path (`anvil`, `anvil start`, `anvil workspace register`)
//!   heartbeats the existing membership — as well as a fresh `Registered`.
//!   Either restores exactly one child. Spawn failure (including a stale
//!   `current_exe` path after a binary upgrade) marks the driver `failed`
//!   and never panics the supervisor; a persistently failing worktree is
//!   bounded by [`MAX_CONSECUTIVE_SPAWN_FAILURES`] and a backoff, so repeated
//!   refreshes cannot become an unbounded respawn loop.
//! - reads readiness evidence the child writes beside its findings log
//!   (`<stem>.ready`): a live PID alone is only [`DriverEvidence::Spawned`];
//!   the child upgrades it to [`DriverEvidence::WatchesInstalled`] once its
//!   initial scan completes and to [`DriverEvidence::FreshActivity`] while
//!   it is serving saves.

use std::collections::{HashMap, VecDeque};
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use crate::ensure::DaemonLauncher;
use crate::registry::{MembershipChange, MembershipHook};

/// Environment variable through which the supervisor hands the findings-log
/// path to the driver child. Contract shared with the `anvil watch
/// --save-time-driver` implementation in `anvil-cli` (`commands/watch_driver`).
pub const DRIVER_LOG_ENV: &str = "ANVIL_SAVE_TIME_DRIVER_LOG";

/// Operator opt-out: a non-empty value disables driver supervision for the
/// daemon's lifetime.
pub const NO_DRIVER_ENV: &str = "ANVIL_NO_SAVE_TIME_DRIVER";

/// Whether the [`NO_DRIVER_ENV`] opt-out is engaged: any non-empty value
/// counts (matching how the CLI treats its own opt-out envs).
#[must_use]
pub fn driver_disabled(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

/// JREL-003: extension of the readiness marker the driver child writes beside
/// its findings log (`<stem>.ready`). Contract shared with `anvil watch
/// --save-time-driver` (`commands/watch_driver` in `anvil-cli`): the child
/// derives the path from the [`DRIVER_LOG_ENV`] log path by swapping the
/// extension; the supervisor removes the marker before every spawn so a
/// previous child's evidence can never be read as the new child's.
pub const READY_MARKER_EXTENSION: &str = "ready";

/// Marker content the child writes once its initial scan completed and its
/// file watches are installed.
pub const READY_MARKER_WATCHING: &str = "watching";

/// Marker content the child rewrites each time it obtains a save-time verdict
/// for a save batch. The file's modification time dates the activity.
pub const READY_MARKER_ACTIVITY: &str = "activity";

/// JREL-003: how recent an activity marker must be to count as
/// [`DriverEvidence::FreshActivity`]; older activity degrades to
/// [`DriverEvidence::WatchesInstalled`] (the watches are still there).
pub const FRESH_ACTIVITY_WINDOW: Duration = Duration::from_mins(1);

/// JREL-003: consecutive failed driver generations (a refused spawn, or a
/// child that died within the backoff of being spawned) after which the
/// supervisor refuses further respawns until [`DEFAULT_SPAWN_FAILURE_BACKOFF`]
/// has elapsed since the last failure. Bounds the respawn a durable refresh
/// can trigger: a persistently failing worktree stays `failed` instead of
/// being respawned on every `anvil` run.
pub const MAX_CONSECUTIVE_SPAWN_FAILURES: u32 = 3;

/// JREL-003: the default respawn backoff once the failure cap is reached, and
/// the lifetime below which a child's death counts as a failed generation.
pub const DEFAULT_SPAWN_FAILURE_BACKOFF: Duration = Duration::from_mins(1);

/// JREL-003: readiness evidence for an attached driver, weakest to strongest.
/// The supervisor can prove only that the PID is alive; everything stronger
/// is reported by the child through its readiness marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriverEvidence {
    /// The child was spawned and its PID is alive; it has not yet reported
    /// that its watches are installed.
    Spawned,
    /// The child completed its initial scan and installed its watches.
    WatchesInstalled,
    /// The child obtained a save-time verdict within
    /// [`FRESH_ACTIVITY_WINDOW`].
    FreshActivity,
}

/// Observable per-worktree driver state. A worktree with no entry is
/// `absent` — DSV-049 maps that to the wire default rather than modelling it
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverStatus {
    /// The child was spawned and its PID is still alive.
    Attached {
        /// The spawned child's PID, as recorded at spawn time.
        pid: u32,
        /// JREL-003: how much readiness the child has reported.
        evidence: DriverEvidence,
    },
    /// The spawn failed (or was refused by the failure bound), or the child
    /// died while the daemon lives. Reported honestly; restored only by the
    /// next durable registration or refresh.
    Failed,
}

/// Builds one launcher per driver spawn. The production impl re-execs
/// `current_exe()`; tests substitute a recording fake. Errors surface as a
/// `failed` driver, never a panic.
pub trait DriverLauncherFactory: Send + Sync {
    /// Build the launcher that will spawn the driver for `worktree`, with the
    /// findings log at `findings_log` (handed to the child via
    /// [`DRIVER_LOG_ENV`]).
    fn launcher_for(
        &self,
        worktree: &Path,
        findings_log: &Path,
    ) -> io::Result<Box<dyn DaemonLauncher + Send + Sync>>;
}

/// Process liveness + termination seam so supervisor tests never signal real
/// PIDs.
pub trait ProcessControl: Send + Sync {
    /// The platform PID-reuse discriminator for `pid`, if one is readable
    /// (Linux `/proc` starttime, Windows creation time, macOS `proc_pidinfo`).
    fn start_time(&self, pid: u32) -> Option<u64>;
    /// Whether `pid` is alive **and** still the process recorded at spawn
    /// time: when both the recorded and current start times are readable they
    /// must match, so a recycled PID is not mistaken for a live driver.
    fn is_alive(&self, pid: u32, recorded_start_time: Option<u64>) -> bool;
    /// Whether this platform can read process start times at all. Where it
    /// can, a record *missing* its start time means the spawn-time read
    /// transiently failed — such a record must never be signalled (the bare
    /// PID could have been recycled); where it cannot, PID liveness is the
    /// only evidence there will ever be.
    fn supports_start_time(&self) -> bool;
    /// Terminate `pid` (SIGTERM on Unix; `TerminateProcess` on Windows). A
    /// process that already exited is success — the driver is gone either way.
    fn terminate(&self, pid: u32) -> io::Result<()>;
}

/// Production [`ProcessControl`] over the crate's existing PID helpers.
pub struct SystemProcessControl;

impl ProcessControl for SystemProcessControl {
    fn start_time(&self, pid: u32) -> Option<u64> {
        crate::process_start_time(pid)
    }

    fn is_alive(&self, pid: u32, recorded_start_time: Option<u64>) -> bool {
        // JREL-003: the supervisor never waits on its detached children, so
        // a child that died would linger as a zombie — `kill(pid, 0)` still
        // succeeds and `/proc/<pid>/stat` still carries the spawn-time start
        // time — and be reported attached forever. Reap it first.
        if reap_if_exited(pid) {
            return false;
        }
        if !crate::process_exists(pid) {
            return false;
        }
        match (recorded_start_time, crate::process_start_time(pid)) {
            (Some(recorded), Some(current)) => recorded == current,
            // A missing discriminator falls back to PID liveness as the best
            // evidence available. On start-time-capable platforms the
            // supervisor never *tracks* a `None`-recorded PID (`spawn_driver`
            // stops such a child immediately) and the startup reconcile
            // refuses to signal bare records, so this arm is only reached on
            // platforms without a start-time read, or when the *current*
            // read transiently fails for a PID recorded with one.
            _ => true,
        }
    }

    fn supports_start_time(&self) -> bool {
        cfg!(any(target_os = "linux", target_os = "macos", windows))
    }

    #[cfg(unix)]
    fn terminate(&self, pid: u32) -> io::Result<()> {
        crate::send_sigterm(pid).map_err(io::Error::other)
    }

    #[cfg(windows)]
    fn terminate(&self, pid: u32) -> io::Result<()> {
        anvil_intercept_win32::terminate_process(pid).map_err(io::Error::other)
    }

    #[cfg(not(any(unix, windows)))]
    fn terminate(&self, _pid: u32) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}

/// JREL-003: non-blocking reap of one of **this process's** children. `true`
/// when `pid` had exited (it is now reaped and gone); `false` when it is
/// still running or is not our child at all (`ECHILD` — a leftover from a
/// previous daemon life, which falls back to the PID/start-time probes).
#[cfg(unix)]
fn reap_if_exited(pid: u32) -> bool {
    use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};

    let Ok(raw) = i32::try_from(pid) else {
        return false;
    };
    matches!(
        waitpid(nix::unistd::Pid::from_raw(raw), Some(WaitPidFlag::WNOHANG)),
        Ok(WaitStatus::Exited(..) | WaitStatus::Signaled(..))
    )
}

/// Windows has no zombie state: a terminated process with no open handle is
/// simply gone, so `process_exists` is already truthful.
#[cfg(not(unix))]
fn reap_if_exited(_pid: u32) -> bool {
    false
}

/// Production [`DriverLauncherFactory`]: re-exec `current_exe()` as
/// `anvil watch --save-time-driver --worktree <canonical>`, findings-log path
/// in [`DRIVER_LOG_ENV`]. `current_exe()` is resolved per spawn so a stale
/// path after a binary upgrade fails that one spawn (→ `failed`) instead of
/// poisoning the supervisor.
///
/// The re-exec contract assumes the daemon process **is** the `anvil` CLI
/// binary (production starts it via `anvil intercept start --foreground`);
/// [`crate::ForegroundOpts::with_save_time_drivers`] is only set on that path.
#[cfg(any(unix, windows))]
pub struct CurrentExeDriverFactory;

/// The driver child's argv (everything after the program path). Split out so
/// the spawn contract pinned by DSV-048's `watch_save_time_driver_argv_contract`
/// test is unit-testable here without spawning.
#[must_use]
pub fn driver_args(worktree: &Path) -> Vec<std::ffi::OsString> {
    vec![
        "watch".into(),
        "--save-time-driver".into(),
        "--worktree".into(),
        worktree.as_os_str().to_owned(),
    ]
}

#[cfg(any(unix, windows))]
impl DriverLauncherFactory for CurrentExeDriverFactory {
    fn launcher_for(
        &self,
        worktree: &Path,
        findings_log: &Path,
    ) -> io::Result<Box<dyn DaemonLauncher + Send + Sync>> {
        let exe = std::env::current_exe()?;
        Ok(Box::new(
            crate::ensure::DetachedCommandLauncher::new(exe, driver_args(worktree))
                .with_env(DRIVER_LOG_ENV, findings_log),
        ))
    }
}

#[derive(Debug, Clone, Copy)]
struct DriverEntry {
    /// `None` ⇒ the spawn failed or was refused (reported as
    /// [`DriverStatus::Failed`]).
    pid: Option<u32>,
    start_time: Option<u64>,
    /// When the tracked child was spawned; `None` for a failed generation.
    spawned_at: Option<Instant>,
    /// JREL-003: failed generations in a row — refused spawns plus children
    /// that died within the backoff of their spawn. Reset once a child
    /// outlives the backoff.
    consecutive_failures: u32,
    /// When the latest failed generation was recorded.
    last_failure: Option<Instant>,
}

impl DriverEntry {
    const fn failed(consecutive_failures: u32, last_failure: Option<Instant>) -> Self {
        Self {
            pid: None,
            start_time: None,
            spawned_at: None,
            consecutive_failures,
            last_failure,
        }
    }
}

struct SupervisorInner {
    /// The driver artefact directory — PID files, findings logs, spawn logs.
    dir: PathBuf,
    factory: Box<dyn DriverLauncherFactory>,
    procs: Box<dyn ProcessControl>,
    drivers: Mutex<HashMap<PathBuf, DriverEntry>>,
    /// JREL-003: respawn backoff in milliseconds (see
    /// [`DEFAULT_SPAWN_FAILURE_BACKOFF`]); a test seam via
    /// [`SaveTimeDriverSupervisor::with_spawn_failure_backoff`].
    spawn_backoff_millis: AtomicU64,
    /// Membership events enqueued by the hook, drained by the supervisor's
    /// own task. Unbounded is safe: the durable set is capped (≤64) and each
    /// transition enqueues one small tuple.
    queue: Mutex<VecDeque<(MembershipChange, PathBuf)>>,
    notify: tokio::sync::Notify,
    /// Latched by [`SaveTimeDriverSupervisor::stop_all`] before it snapshots
    /// the driver map. Checked under the `drivers` lock in `spawn_driver`, so
    /// a spawn in flight at shutdown either sees the flag (and skips) or
    /// finishes its insert before the stop-all snapshot (and is terminated) —
    /// never an orphan.
    shutdown: std::sync::atomic::AtomicBool,
}

/// Supervises the per-worktree save-time driver children. Cheaply cloneable
/// (an `Arc` around shared state) so the membership hook, the consumer task,
/// and the shutdown path can all hold it.
#[derive(Clone)]
pub struct SaveTimeDriverSupervisor {
    inner: Arc<SupervisorInner>,
}

impl SaveTimeDriverSupervisor {
    /// Build a supervisor with explicit seams. `dir` is the driver artefact
    /// directory (PID files, findings logs, spawn logs); production passes
    /// `<runtime>/save-time-drivers/`.
    #[must_use]
    pub fn new(
        dir: PathBuf,
        factory: Box<dyn DriverLauncherFactory>,
        procs: Box<dyn ProcessControl>,
    ) -> Self {
        Self {
            inner: Arc::new(SupervisorInner {
                dir,
                factory,
                procs,
                drivers: Mutex::new(HashMap::new()),
                spawn_backoff_millis: AtomicU64::new(
                    u64::try_from(DEFAULT_SPAWN_FAILURE_BACKOFF.as_millis()).unwrap_or(u64::MAX),
                ),
                queue: Mutex::new(VecDeque::new()),
                notify: tokio::sync::Notify::new(),
                shutdown: std::sync::atomic::AtomicBool::new(false),
            }),
        }
    }

    /// JREL-003: override the respawn backoff (and the early-death lifetime)
    /// — a test seam; production keeps [`DEFAULT_SPAWN_FAILURE_BACKOFF`].
    #[must_use]
    pub fn with_spawn_failure_backoff(self, backoff: Duration) -> Self {
        self.inner.spawn_backoff_millis.store(
            u64::try_from(backoff.as_millis()).unwrap_or(u64::MAX),
            Ordering::SeqCst,
        );
        self
    }

    /// Production wiring: artefacts under `dir` (resolve it with
    /// [`default_driver_dir`]), children re-exec'd from `current_exe()`, real
    /// process control.
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn production(dir: PathBuf) -> Self {
        Self::new(
            dir,
            Box::new(CurrentExeDriverFactory),
            Box::new(SystemProcessControl),
        )
    }

    /// The hook to install via `SessionRegistry::set_membership_hook`.
    /// Enqueue-only by design (review pin a): `signal_membership` fires
    /// synchronously inside `session.register` handling, so this must never
    /// spawn or touch the filesystem.
    #[must_use]
    pub fn membership_hook(&self) -> MembershipHook {
        let inner = Arc::clone(&self.inner);
        Arc::new(move |change, worktree| {
            inner
                .queue
                .lock()
                .expect("driver queue lock poisoned")
                .push_back((change, worktree.to_path_buf()));
            inner.notify.notify_one();
        })
    }

    /// Sweep PID files left by a previous daemon life: terminate any recorded
    /// child that is still alive (verified against its recorded start time,
    /// so a recycled PID is never signalled) and remove the files. Fresh
    /// drivers for the reloaded registrations are spawned by the `Registered`
    /// events the durable reload enqueues through the membership hook — this
    /// sweep only clears the previous generation.
    pub fn reconcile_on_start(&self) {
        let entries = match std::fs::read_dir(&self.inner.dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return,
            Err(err) => {
                tracing::warn!(
                    target: "anvil_intercept::save_time_driver",
                    dir = %self.inner.dir.display(),
                    error = %err,
                    "could not read the save-time driver PID directory on startup",
                );
                return;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() != Some(OsStr::new("pid")) {
                continue;
            }
            if let Some((pid, start_time)) = read_pid_record(&path)
                // A record with no start time on a platform that CAN read
                // them means the spawn-time read transiently failed — the
                // bare PID could have been recycled across the daemon
                // restart, so sweep the file without signalling.
                && (start_time.is_some() || !self.inner.procs.supports_start_time())
                && self.inner.procs.is_alive(pid, start_time)
                && let Err(err) = self.inner.procs.terminate(pid)
            {
                tracing::warn!(
                    target: "anvil_intercept::save_time_driver",
                    pid,
                    error = %err,
                    "could not terminate a leftover save-time driver on startup",
                );
            }
            if let Err(err) = std::fs::remove_file(&path)
                && err.kind() != io::ErrorKind::NotFound
            {
                tracing::warn!(
                    target: "anvil_intercept::save_time_driver",
                    path = %path.display(),
                    error = %err,
                    "could not remove a leftover save-time driver PID file",
                );
            }
        }
    }

    /// Drain and handle every queued membership event. Returns the number
    /// handled. Synchronous so tests drive the supervisor deterministically;
    /// [`Self::run`] calls it off the async runtime via `spawn_blocking`.
    pub fn process_pending(&self) -> usize {
        let mut handled = 0usize;
        loop {
            let next = self
                .inner
                .queue
                .lock()
                .expect("driver queue lock poisoned")
                .pop_front();
            let Some((change, worktree)) = next else {
                break;
            };
            self.inner.handle(change, &worktree);
            handled += 1;
        }
        handled
    }

    /// The supervisor's consumer loop: drain queued events, then sleep until
    /// the hook signals more or shutdown is requested. Spawn/terminate and
    /// PID-file I/O run inside `spawn_blocking` so the (single-threaded)
    /// daemon runtime is never blocked on process or filesystem work.
    pub async fn run(&self, mut token: crate::ShutdownToken) {
        loop {
            let supervisor = self.clone();
            if tokio::task::spawn_blocking(move || supervisor.process_pending())
                .await
                .is_err()
            {
                tracing::warn!(
                    target: "anvil_intercept::save_time_driver",
                    "save-time driver drain task panicked",
                );
            }
            tokio::select! {
                biased;
                () = token.cancelled() => break,
                () = self.inner.notify.notified() => {}
            }
        }
    }

    /// Terminate every live driver and drop its PID file. Called on daemon
    /// shutdown (both the graceful and the listener-failure exit paths).
    /// Latches the shutdown flag first, so a membership event still being
    /// drained by the consumer task after this call can no longer spawn — a
    /// child either landed in the map before the snapshot (and is terminated
    /// here) or is never spawned.
    pub fn stop_all(&self) {
        self.inner
            .shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let worktrees: Vec<PathBuf> = self
            .inner
            .drivers
            .lock()
            .expect("driver map lock poisoned")
            .keys()
            .cloned()
            .collect();
        for worktree in worktrees {
            self.inner.stop_driver(&worktree);
        }
    }

    /// [`Self::reconcile_on_start`] off the async runtime — the sweep does
    /// process and filesystem I/O, which must not run on the daemon's
    /// (single-threaded) runtime worker.
    pub async fn reconcile_on_start_blocking(&self) {
        let supervisor = self.clone();
        if tokio::task::spawn_blocking(move || supervisor.reconcile_on_start())
            .await
            .is_err()
        {
            tracing::warn!(
                target: "anvil_intercept::save_time_driver",
                "save-time driver startup reconcile task panicked",
            );
        }
    }

    /// [`Self::stop_all`] off the async runtime — mirrors the daemon's
    /// `persist_on_shutdown` pattern so terminate/PID-file I/O at shutdown
    /// never stalls in-flight IPC handlers.
    pub async fn stop_all_blocking(&self) {
        let supervisor = self.clone();
        if tokio::task::spawn_blocking(move || supervisor.stop_all())
            .await
            .is_err()
        {
            tracing::warn!(
                target: "anvil_intercept::save_time_driver",
                "save-time driver stop-all task panicked",
            );
        }
    }

    /// The current observable state for `worktree`: `None` when no driver was
    /// ever requested (wire `absent`), otherwise attached-or-failed probed
    /// against live process state — a child that died since the last probe
    /// reports [`DriverStatus::Failed`] here without any monitor task.
    #[must_use]
    pub fn driver_status(&self, worktree: &Path) -> Option<DriverStatus> {
        let drivers = self.inner.drivers.lock().expect("driver map lock poisoned");
        let entry = drivers.get(worktree)?;
        Some(self.inner.status_of(worktree, *entry))
    }

    /// Snapshot of every tracked worktree's driver state (DSV-049 renders
    /// this through the status wire).
    #[must_use]
    pub fn status_snapshot(&self) -> HashMap<PathBuf, DriverStatus> {
        let drivers = self.inner.drivers.lock().expect("driver map lock poisoned");
        drivers
            .iter()
            .map(|(worktree, entry)| (worktree.clone(), self.inner.status_of(worktree, *entry)))
            .collect()
    }
}

impl SupervisorInner {
    fn handle(&self, change: MembershipChange, worktree: &Path) {
        match change {
            // JREL-003: a durable refresh is "ensure a live driver" — a live
            // child is kept, a dead or never-spawned one is (re)spawned
            // within the failure bound.
            MembershipChange::Registered | MembershipChange::Refreshed => {
                self.spawn_driver(worktree);
            }
            MembershipChange::Unregistered | MembershipChange::Reaped => {
                self.stop_driver(worktree);
            }
        }
    }

    fn spawn_backoff(&self) -> Duration {
        Duration::from_millis(self.spawn_backoff_millis.load(Ordering::SeqCst))
    }

    fn status_of(&self, worktree: &Path, entry: DriverEntry) -> DriverStatus {
        match entry.pid {
            Some(pid) if self.procs.is_alive(pid, entry.start_time) => DriverStatus::Attached {
                pid,
                evidence: self.evidence_for(worktree),
            },
            // Spawn failed or refused, or the child died while the daemon
            // lives. Never respawned from a status probe — only the next
            // durable registration or refresh restores it.
            _ => DriverStatus::Failed,
        }
    }

    /// JREL-003: the child's readiness marker, read fresh per probe. Missing
    /// or unrecognised content is no stronger than the live PID.
    fn evidence_for(&self, worktree: &Path) -> DriverEvidence {
        let marker = self.ready_marker_path(worktree);
        let Ok(content) = std::fs::read_to_string(&marker) else {
            return DriverEvidence::Spawned;
        };
        match content.trim() {
            READY_MARKER_ACTIVITY => {
                let fresh = std::fs::metadata(&marker)
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                    .is_some_and(|age| age <= FRESH_ACTIVITY_WINDOW);
                if fresh {
                    DriverEvidence::FreshActivity
                } else {
                    DriverEvidence::WatchesInstalled
                }
            }
            READY_MARKER_WATCHING => DriverEvidence::WatchesInstalled,
            _ => DriverEvidence::Spawned,
        }
    }

    fn ready_marker_path(&self, worktree: &Path) -> PathBuf {
        self.dir.join(format!(
            "{}.{READY_MARKER_EXTENSION}",
            worktree_artifact_stem(worktree)
        ))
    }

    /// On a start-time-capable platform a driver is NEVER tracked without
    /// its PID-reuse discriminator — a bare PID could later be signalled
    /// after recycling. Right now, milliseconds after our own spawn, the PID
    /// is provably still our child, so terminating it is the one safe
    /// moment; the driver is reported failed honestly.
    fn stop_child_without_start_time(&self, worktree: &Path, pid: u32) {
        if let Err(err) = self.procs.terminate(pid) {
            tracing::warn!(
                target: "anvil_intercept::save_time_driver",
                worktree = %worktree.display(),
                pid,
                error = %err,
                "could not terminate a driver whose start time was unreadable",
            );
        }
        tracing::warn!(
            target: "anvil_intercept::save_time_driver",
            worktree = %worktree.display(),
            pid,
            "driver start time unreadable at spawn; driver stopped and marked failed",
        );
    }

    fn spawn_driver(&self, worktree: &Path) {
        // The map lock is held for the WHOLE spawn — flag check through
        // insert — so `stop_all` (which latches the flag, then takes this
        // lock to snapshot) either prevents this spawn or waits for its
        // insert and terminates the child. Status probes block for the few
        // milliseconds a spawn takes; only the consumer task ever spawns, so
        // there is no spawn-vs-spawn contention. The registry call path is
        // untouched — the hook uses the queue lock, never this one.
        let mut drivers = self.drivers.lock().expect("driver map lock poisoned");
        if self.shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        // Idempotent: a duplicate `Registered` (or a JREL-003 `Refreshed`)
        // for a worktree whose child is still alive keeps the existing child.
        if let Some(entry) = drivers.get(worktree)
            && let Some(pid) = entry.pid
            && self.procs.is_alive(pid, entry.start_time)
        {
            return;
        }
        let now = Instant::now();
        let backoff = self.spawn_backoff();
        let (failures, last_failure) = failure_history(drivers.get(worktree), now, backoff);
        let stem = worktree_artifact_stem(worktree);
        if failures >= MAX_CONSECUTIVE_SPAWN_FAILURES
            && last_failure.is_some_and(|at| now.duration_since(at) < backoff)
        {
            tracing::warn!(
                target: "anvil_intercept::save_time_driver",
                worktree = %worktree.display(),
                consecutive_failures = failures,
                backoff_secs = backoff.as_secs(),
                "save-time driver respawn refused: failure bound reached; driver stays failed until the backoff elapses",
            );
            drivers.insert(
                worktree.to_path_buf(),
                DriverEntry::failed(failures, last_failure),
            );
            remove_artifact(&self.dir.join(format!("{stem}.pid")), "PID file");
            return;
        }
        let findings_log = self.dir.join(format!("{stem}.log"));
        let spawn_log = self.dir.join(format!("{stem}.spawn.log"));
        // A previous child's readiness marker must never be read as the new
        // child's evidence: remove it before the spawn, under the map lock.
        remove_artifact(&self.ready_marker_path(worktree), "readiness marker");
        let spawned = self
            .factory
            .launcher_for(worktree, &findings_log)
            .and_then(|launcher| launcher.spawn_detached(&spawn_log));
        match spawned {
            Ok(pid) => {
                let start_time = self.procs.start_time(pid);
                if start_time.is_none() && self.procs.supports_start_time() {
                    self.stop_child_without_start_time(worktree, pid);
                    drivers.insert(
                        worktree.to_path_buf(),
                        DriverEntry::failed(failures.saturating_add(1), Some(now)),
                    );
                    return;
                }
                if let Err(err) = self.write_pid_file(&stem, pid, start_time) {
                    // The driver still runs; only reconcile-after-restart
                    // loses track of it. Loud, not fatal.
                    tracing::warn!(
                        target: "anvil_intercept::save_time_driver",
                        worktree = %worktree.display(),
                        pid,
                        error = %err,
                        "could not persist the save-time driver PID file",
                    );
                }
                tracing::info!(
                    target: "anvil_intercept::save_time_driver",
                    worktree = %worktree.display(),
                    pid,
                    "spawned save-time driver",
                );
                drivers.insert(
                    worktree.to_path_buf(),
                    DriverEntry {
                        pid: Some(pid),
                        start_time,
                        spawned_at: Some(now),
                        consecutive_failures: failures,
                        last_failure,
                    },
                );
            }
            Err(err) => {
                // Review pin (c): spawn failure — including a stale
                // `current_exe` after a binary upgrade — marks the driver
                // failed and never panics the supervisor.
                tracing::warn!(
                    target: "anvil_intercept::save_time_driver",
                    worktree = %worktree.display(),
                    error = %err,
                    consecutive_failures = failures.saturating_add(1),
                    "could not spawn the save-time driver; driver marked failed",
                );
                drivers.insert(
                    worktree.to_path_buf(),
                    DriverEntry::failed(failures.saturating_add(1), Some(now)),
                );
            }
        }
    }

    fn stop_driver(&self, worktree: &Path) {
        let entry = self
            .drivers
            .lock()
            .expect("driver map lock poisoned")
            .remove(worktree);
        if let Some(DriverEntry {
            pid: Some(pid),
            start_time,
            ..
        }) = entry
        {
            // Only signal a PID that is verifiably still our child (the
            // recorded start time must match where readable) — never a
            // recycled PID.
            if self.procs.is_alive(pid, start_time)
                && let Err(err) = self.procs.terminate(pid)
            {
                tracing::warn!(
                    target: "anvil_intercept::save_time_driver",
                    worktree = %worktree.display(),
                    pid,
                    error = %err,
                    "could not terminate the save-time driver",
                );
            }
        }
        let stem = worktree_artifact_stem(worktree);
        remove_artifact(&self.dir.join(format!("{stem}.pid")), "PID file");
        remove_artifact(&self.ready_marker_path(worktree), "readiness marker");
    }

    fn write_pid_file(&self, stem: &str, pid: u32, start_time: Option<u64>) -> io::Result<()> {
        crate::ensure_secure_runtime_dir(&self.dir)
            .map_err(|err| io::Error::other(format!("{err:#}")))?;
        let record = start_time.map_or_else(
            || format!("{pid}\n"),
            |start_time| format!("{pid}\n{start_time}\n"),
        );
        // Plain truncate-in-place write: no rename step, so the Windows
        // rename-over-existing trap does not apply here.
        std::fs::write(self.dir.join(format!("{stem}.pid")), record)
    }
}

/// JREL-003: the failure history a new generation inherits from the entry it
/// replaces. A refused or failed spawn keeps its count; a child that died
/// within `backoff` of its spawn is one more failed generation; a child that
/// outlived the backoff (or no entry at all) starts from zero.
fn failure_history(
    entry: Option<&DriverEntry>,
    now: Instant,
    backoff: Duration,
) -> (u32, Option<Instant>) {
    match entry {
        None => (0, None),
        Some(entry) if entry.pid.is_none() => (entry.consecutive_failures, entry.last_failure),
        Some(entry) => {
            let died_early = entry
                .spawned_at
                .is_some_and(|spawned_at| now.duration_since(spawned_at) < backoff);
            if died_early {
                (entry.consecutive_failures.saturating_add(1), Some(now))
            } else {
                (0, None)
            }
        }
    }
}

/// Remove one driver artefact, tolerating its absence; anything else is loud
/// but never fatal.
fn remove_artifact(path: &Path, what: &str) {
    if let Err(err) = std::fs::remove_file(path)
        && err.kind() != io::ErrorKind::NotFound
    {
        tracing::warn!(
            target: "anvil_intercept::save_time_driver",
            path = %path.display(),
            error = %err,
            "could not remove the save-time driver {what}",
        );
    }
}

/// Resolve the production driver artefact directory. Mirrors the driver
/// child's default log-path precedence (`anvil-cli` `commands/watch_driver`)
/// exactly, so the supervisor-chosen findings log and the path a manual
/// (env-absent) driver run would pick for the same worktree coincide:
/// `{ANVIL_HOME}/runtime/save-time-drivers/`, else
/// `$XDG_RUNTIME_DIR/anvil/save-time-drivers/`, else
/// `%LOCALAPPDATA%\anvil\save-time-drivers\`, else
/// `~/.local/state/anvil/save-time-drivers/`. (Deliberately NOT derived from
/// the PID-file parent: `ANVIL_HOME` re-roots `intercept.pid` directly under
/// the prefix, without the `runtime/` segment ADR-101 specifies here.)
///
/// # Errors
/// When no candidate root resolves (no `ANVIL_HOME`, runtime dir, or home).
pub fn default_driver_dir() -> anyhow::Result<PathBuf> {
    driver_dir_from(
        non_empty_env("ANVIL_HOME"),
        non_empty_env("XDG_RUNTIME_DIR"),
        if cfg!(windows) {
            non_empty_env("LOCALAPPDATA")
        } else {
            None
        },
        non_empty_env("HOME").or_else(|| non_empty_env("USERPROFILE")),
    )
}

/// Pure resolver for [`default_driver_dir`] — candidate roots are passed
/// explicitly so it unit-tests without mutating the process environment.
fn driver_dir_from(
    anvil_home: Option<PathBuf>,
    xdg_runtime_dir: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
    home: Option<PathBuf>,
) -> anyhow::Result<PathBuf> {
    use anyhow::Context as _;
    let dir = if let Some(prefix) = anvil_home {
        prefix.join("runtime").join("save-time-drivers")
    } else if let Some(runtime_dir) = xdg_runtime_dir {
        runtime_dir.join("anvil").join("save-time-drivers")
    } else if let Some(local_app_data) = local_app_data {
        local_app_data.join("anvil").join("save-time-drivers")
    } else {
        home.context("cannot resolve home directory for the save-time driver registry")?
            .join(".local")
            .join("state")
            .join("anvil")
            .join("save-time-drivers")
    };
    Ok(dir)
}

fn non_empty_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// `pid\n[start_time\n]` — the record [`SupervisorInner::write_pid_file`]
/// persists. A malformed file yields `None` and is swept without signalling.
fn read_pid_record(path: &Path) -> Option<(u32, Option<u64>)> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let pid = lines.next()?.trim().parse().ok()?;
    let start_time = lines.next().and_then(|line| line.trim().parse().ok());
    Some((pid, start_time))
}

/// Filename-safe, collision-resistant artefact stem for a worktree: the leaf
/// directory name plus a 12-hex prefix of the SHA-256 of the canonical path.
/// Deliberately the same scheme as the driver child's default log name
/// (`anvil-cli` `commands/watch_driver::worktree_log_file_name`) so the
/// supervisor-chosen findings log coincides with the path a manual
/// (env-absent) driver run would pick for the same worktree.
fn worktree_artifact_stem(worktree: &Path) -> String {
    use sha2::{Digest as _, Sha256};
    let digest = Sha256::digest(worktree.as_os_str().as_encoded_bytes());
    let mut hex = String::with_capacity(12);
    for byte in &digest[..6] {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    let leaf = worktree.file_name().map_or_else(
        || "worktree".to_owned(),
        |name| name.to_string_lossy().replace(['/', '\\', ':', ' '], "-"),
    );
    format!("{leaf}-{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Records every spawn request; hands out sequential PIDs (or fails).
    struct FakeFactory {
        state: Arc<FakeState>,
        fail_spawn: bool,
        fail_factory: bool,
    }

    #[derive(Default)]
    struct FakeState {
        /// `(worktree, findings_log, spawn_log)` per successful spawn request.
        spawns: Mutex<Vec<(PathBuf, PathBuf, PathBuf)>>,
        next_pid: AtomicU32,
        /// PIDs considered alive, with their fake start times.
        alive: Mutex<HashMap<u32, u64>>,
        terminated: Mutex<Vec<u32>>,
        /// Model a transient start-time read failure on a platform that
        /// supports start times.
        withhold_start_time: std::sync::atomic::AtomicBool,
        /// Every `launcher_for` call, including refused ones — the spawn
        /// *attempt* count the JREL-003 failure bound limits.
        factory_calls: AtomicU32,
    }

    struct FakeLauncher {
        state: Arc<FakeState>,
        worktree: PathBuf,
        findings_log: PathBuf,
        fail: bool,
    }

    impl DaemonLauncher for FakeLauncher {
        fn spawn_detached(&self, log_path: &Path) -> io::Result<u32> {
            if self.fail {
                return Err(io::Error::other("spawn refused"));
            }
            let pid = self.state.next_pid.fetch_add(1, Ordering::SeqCst);
            self.state
                .alive
                .lock()
                .expect("alive lock")
                .insert(pid, u64::from(pid) * 100);
            self.state.spawns.lock().expect("spawns lock").push((
                self.worktree.clone(),
                self.findings_log.clone(),
                log_path.to_path_buf(),
            ));
            Ok(pid)
        }
    }

    impl DriverLauncherFactory for FakeFactory {
        fn launcher_for(
            &self,
            worktree: &Path,
            findings_log: &Path,
        ) -> io::Result<Box<dyn DaemonLauncher + Send + Sync>> {
            self.state.factory_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_factory {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "current_exe gone (binary upgraded)",
                ));
            }
            Ok(Box::new(FakeLauncher {
                state: Arc::clone(&self.state),
                worktree: worktree.to_path_buf(),
                findings_log: findings_log.to_path_buf(),
                fail: self.fail_spawn,
            }))
        }
    }

    struct FakeProcs {
        state: Arc<FakeState>,
    }

    impl ProcessControl for FakeProcs {
        fn start_time(&self, pid: u32) -> Option<u64> {
            if self.state.withhold_start_time.load(Ordering::SeqCst) {
                return None;
            }
            self.state
                .alive
                .lock()
                .expect("alive lock")
                .get(&pid)
                .copied()
        }

        fn is_alive(&self, pid: u32, recorded_start_time: Option<u64>) -> bool {
            let alive = self.state.alive.lock().expect("alive lock");
            match (alive.get(&pid), recorded_start_time) {
                (Some(current), Some(recorded)) => *current == recorded,
                (Some(_), None) => true,
                (None, _) => false,
            }
        }

        fn terminate(&self, pid: u32) -> io::Result<()> {
            self.state
                .terminated
                .lock()
                .expect("terminated lock")
                .push(pid);
            self.state.alive.lock().expect("alive lock").remove(&pid);
            Ok(())
        }

        fn supports_start_time(&self) -> bool {
            true
        }
    }

    struct Harness {
        supervisor: SaveTimeDriverSupervisor,
        state: Arc<FakeState>,
        dir: PathBuf,
        _tmp: tempfile::TempDir,
    }

    fn harness() -> Harness {
        harness_with(false, false)
    }

    fn harness_with(fail_factory: bool, fail_spawn: bool) -> Harness {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("save-time-drivers");
        let state = Arc::new(FakeState {
            next_pid: AtomicU32::new(41),
            ..FakeState::default()
        });
        let supervisor = SaveTimeDriverSupervisor::new(
            dir.clone(),
            Box::new(FakeFactory {
                state: Arc::clone(&state),
                fail_spawn,
                fail_factory,
            }),
            Box::new(FakeProcs {
                state: Arc::clone(&state),
            }),
        );
        Harness {
            supervisor,
            state,
            dir,
            _tmp: tmp,
        }
    }

    fn enqueue(harness: &Harness, change: MembershipChange, worktree: &Path) {
        (harness.supervisor.membership_hook())(change, worktree);
    }

    #[test]
    fn save_time_driver_disabled_only_by_non_empty_env() {
        assert!(!driver_disabled(None));
        assert!(!driver_disabled(Some(OsStr::new(""))));
        assert!(driver_disabled(Some(OsStr::new("1"))));
        assert!(driver_disabled(Some(OsStr::new("no"))));
    }

    #[test]
    fn save_time_driver_args_match_the_dsv048_contract() {
        let args = driver_args(Path::new("/ws/repo"));
        let rendered: Vec<_> = args.iter().map(|a| a.to_string_lossy()).collect();
        assert_eq!(
            rendered,
            ["watch", "--save-time-driver", "--worktree", "/ws/repo"]
        );
    }

    #[test]
    fn save_time_driver_hook_only_enqueues() {
        let h = harness();
        enqueue(&h, MembershipChange::Registered, Path::new("/ws/repo"));
        assert!(
            h.state.spawns.lock().expect("spawns").is_empty(),
            "the hook must not spawn on the registry call path"
        );
        assert_eq!(h.supervisor.process_pending(), 1);
        assert_eq!(h.state.spawns.lock().expect("spawns").len(), 1);
    }

    #[test]
    fn save_time_driver_spawns_on_registered_and_records_pid() {
        let h = harness();
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();

        let spawns = h.state.spawns.lock().expect("spawns");
        let (spawned_worktree, findings_log, spawn_log) = &spawns[0];
        assert_eq!(spawned_worktree, worktree);
        let stem = worktree_artifact_stem(worktree);
        assert_eq!(*findings_log, h.dir.join(format!("{stem}.log")));
        assert_eq!(*spawn_log, h.dir.join(format!("{stem}.spawn.log")));
        drop(spawns);

        let pid_file = h.dir.join(format!("{stem}.pid"));
        let record = std::fs::read_to_string(&pid_file).expect("pid file written");
        assert_eq!(record, "41\n4100\n", "pid + start time recorded");
        assert_eq!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Attached {
                pid: 41,
                evidence: DriverEvidence::Spawned,
            })
        );
    }

    #[test]
    fn save_time_driver_duplicate_registered_keeps_live_child() {
        let h = harness();
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();
        assert_eq!(
            h.state.spawns.lock().expect("spawns").len(),
            1,
            "a live child is kept, not duplicated"
        );
    }

    #[test]
    fn save_time_driver_unregistered_terminates_and_cleans_up() {
        let h = harness();
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();
        let pid_file = h
            .dir
            .join(format!("{}.pid", worktree_artifact_stem(worktree)));
        assert!(pid_file.exists());

        enqueue(&h, MembershipChange::Unregistered, worktree);
        h.supervisor.process_pending();
        assert_eq!(*h.state.terminated.lock().expect("terminated"), vec![41]);
        assert!(!pid_file.exists(), "pid file removed on stop");
        assert_eq!(h.supervisor.driver_status(worktree), None, "wire absent");
    }

    #[test]
    fn save_time_driver_reaped_terminates_like_unregistered() {
        let h = harness();
        let worktree = Path::new("/ws/gone");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();
        enqueue(&h, MembershipChange::Reaped, worktree);
        h.supervisor.process_pending();
        assert_eq!(*h.state.terminated.lock().expect("terminated"), vec![41]);
        assert_eq!(h.supervisor.driver_status(worktree), None);
    }

    #[test]
    fn save_time_driver_spawn_failure_is_failed_not_panic() {
        for h in [harness_with(true, false), harness_with(false, true)] {
            let worktree = Path::new("/ws/repo");
            enqueue(&h, MembershipChange::Registered, worktree);
            h.supervisor.process_pending();
            assert_eq!(
                h.supervisor.driver_status(worktree),
                Some(DriverStatus::Failed),
                "spawn failure (stale current_exe / spawn error) reports failed"
            );
            assert!(
                !h.dir
                    .join(format!("{}.pid", worktree_artifact_stem(worktree)))
                    .exists(),
                "no pid file for a failed spawn"
            );
            // A later unregister for the failed entry is a clean no-op.
            enqueue(&h, MembershipChange::Unregistered, worktree);
            h.supervisor.process_pending();
            assert!(h.state.terminated.lock().expect("terminated").is_empty());
        }
    }

    #[test]
    fn save_time_driver_child_death_reports_failed_without_respawn() {
        let h = harness();
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();

        // The child dies while the daemon lives.
        h.state.alive.lock().expect("alive").remove(&41);
        assert_eq!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Failed),
            "death is reported honestly"
        );
        // Unrelated churn must not resurrect it (no auto-respawn, pin b).
        enqueue(&h, MembershipChange::Registered, Path::new("/ws/other"));
        h.supervisor.process_pending();
        assert_eq!(
            h.state
                .spawns
                .lock()
                .expect("spawns")
                .iter()
                .filter(|(wt, _, _)| wt == worktree)
                .count(),
            1,
            "no respawn for the dead child"
        );
    }

    #[test]
    fn save_time_driver_re_registered_after_death_respawns() {
        // A NEW membership gain for a worktree whose recorded child died is a
        // fresh spawn — distinct from auto-respawn: the registry, not a
        // monitor, drives it.
        let h = harness();
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();
        h.state.alive.lock().expect("alive").remove(&41);

        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();
        assert_eq!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Attached {
                pid: 42,
                evidence: DriverEvidence::Spawned,
            })
        );
    }

    // ── JREL-003: refresh-driven recovery, bounded failure, readiness evidence ──

    fn spawn_count_for(h: &Harness, worktree: &Path) -> usize {
        h.state
            .spawns
            .lock()
            .expect("spawns")
            .iter()
            .filter(|(wt, _, _)| wt == worktree)
            .count()
    }

    #[test]
    fn save_time_driver_refreshed_restores_a_dead_child_and_keeps_a_live_one() {
        let h = harness();
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();

        // A refresh of a live child is a no-op: exactly one driver.
        enqueue(&h, MembershipChange::Refreshed, worktree);
        h.supervisor.process_pending();
        assert_eq!(spawn_count_for(&h, worktree), 1, "live child kept");

        // The child dies; the next refresh restores exactly one child.
        h.state.alive.lock().expect("alive").remove(&41);
        enqueue(&h, MembershipChange::Refreshed, worktree);
        h.supervisor.process_pending();
        assert_eq!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Attached {
                pid: 42,
                evidence: DriverEvidence::Spawned,
            })
        );
        assert_eq!(spawn_count_for(&h, worktree), 2, "one respawn");
        let record = std::fs::read_to_string(
            h.dir
                .join(format!("{}.pid", worktree_artifact_stem(worktree))),
        )
        .expect("pid file rewritten");
        assert_eq!(record, "42\n4200\n", "the PID record follows the new child");
    }

    #[test]
    fn save_time_driver_refresh_without_a_child_is_not_readiness() {
        // Membership refresh alone is not readiness: when the spawn keeps
        // failing, a refresh leaves the driver `failed` — never attached.
        let h = harness_with(true, false);
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        enqueue(&h, MembershipChange::Refreshed, worktree);
        h.supervisor.process_pending();
        assert_eq!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Failed),
            "a refreshed membership with no live child stays failed"
        );
        assert!(
            h.state.spawns.lock().expect("spawns").is_empty(),
            "no child was ever spawned"
        );
    }

    #[test]
    fn save_time_driver_persistent_spawn_failure_is_bounded_by_cap_and_backoff() {
        let h = harness_with(true, false);
        let _ = h
            .supervisor
            .clone()
            .with_spawn_failure_backoff(Duration::from_hours(1));
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        for _ in 0..10 {
            enqueue(&h, MembershipChange::Refreshed, worktree);
        }
        h.supervisor.process_pending();
        assert_eq!(
            h.state.factory_calls.load(Ordering::SeqCst),
            MAX_CONSECUTIVE_SPAWN_FAILURES,
            "spawn attempts stop at the cap while the backoff holds"
        );
        assert_eq!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Failed)
        );

        // Once the backoff elapses, exactly one more attempt is admitted per
        // refresh — bounded again by the cap, never a loop.
        let _ = h
            .supervisor
            .clone()
            .with_spawn_failure_backoff(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        enqueue(&h, MembershipChange::Refreshed, worktree);
        h.supervisor.process_pending();
        assert_eq!(
            h.state.factory_calls.load(Ordering::SeqCst),
            MAX_CONSECUTIVE_SPAWN_FAILURES + 1,
            "one attempt after the backoff"
        );
    }

    #[test]
    fn save_time_driver_crash_loop_is_bounded_like_spawn_failure() {
        // A child that dies within the backoff of its spawn is a failed
        // generation: after the cap, refreshes stop respawning.
        let h = harness();
        let _ = h
            .supervisor
            .clone()
            .with_spawn_failure_backoff(Duration::from_hours(1));
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();
        for pid in 41..41 + MAX_CONSECUTIVE_SPAWN_FAILURES + 2 {
            h.state.alive.lock().expect("alive").remove(&pid);
            enqueue(&h, MembershipChange::Refreshed, worktree);
            h.supervisor.process_pending();
        }
        assert_eq!(
            spawn_count_for(&h, worktree),
            usize::try_from(MAX_CONSECUTIVE_SPAWN_FAILURES).expect("cap fits"),
            "respawns after early deaths stop at the cap"
        );
        assert_eq!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Failed)
        );
        assert!(
            !h.dir
                .join(format!("{}.pid", worktree_artifact_stem(worktree)))
                .exists(),
            "a refused generation leaves no stale PID record"
        );
        // The bound is per worktree: another worktree still spawns.
        enqueue(&h, MembershipChange::Registered, Path::new("/ws/other"));
        h.supervisor.process_pending();
        assert_eq!(spawn_count_for(&h, Path::new("/ws/other")), 1);
    }

    #[test]
    fn save_time_driver_child_that_outlived_the_backoff_resets_the_bound() {
        let h = harness();
        let _ = h
            .supervisor
            .clone()
            .with_spawn_failure_backoff(Duration::from_millis(1));
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();
        for pid in 41..41 + MAX_CONSECUTIVE_SPAWN_FAILURES + 2 {
            std::thread::sleep(Duration::from_millis(3));
            h.state.alive.lock().expect("alive").remove(&pid);
            enqueue(&h, MembershipChange::Refreshed, worktree);
            h.supervisor.process_pending();
        }
        assert_eq!(
            spawn_count_for(&h, worktree),
            usize::try_from(MAX_CONSECUTIVE_SPAWN_FAILURES + 3).expect("fits"),
            "deaths after a healthy lifetime never trip the bound"
        );
        assert!(matches!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Attached { .. })
        ));
    }

    #[test]
    fn save_time_driver_evidence_distinguishes_spawned_watching_and_fresh_activity() {
        let h = harness();
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();
        let evidence = || match h.supervisor.driver_status(worktree) {
            Some(DriverStatus::Attached { evidence, .. }) => evidence,
            other => panic!("expected attached, got {other:?}"),
        };
        assert_eq!(evidence(), DriverEvidence::Spawned, "a live PID alone");

        let marker = h.dir.join(format!(
            "{}.{READY_MARKER_EXTENSION}",
            worktree_artifact_stem(worktree)
        ));
        std::fs::write(&marker, format!("{READY_MARKER_WATCHING}\n")).expect("marker");
        assert_eq!(evidence(), DriverEvidence::WatchesInstalled);

        std::fs::write(&marker, format!("{READY_MARKER_ACTIVITY}\n")).expect("marker");
        assert_eq!(
            evidence(),
            DriverEvidence::FreshActivity,
            "just served a save"
        );

        // Old activity is still an installed watch set, no longer fresh.
        std::fs::File::options()
            .write(true)
            .open(&marker)
            .expect("open marker")
            .set_modified(SystemTime::now() - FRESH_ACTIVITY_WINDOW - Duration::from_secs(5))
            .expect("age the marker");
        assert_eq!(evidence(), DriverEvidence::WatchesInstalled);

        std::fs::write(&marker, "garbage\n").expect("marker");
        assert_eq!(
            evidence(),
            DriverEvidence::Spawned,
            "unrecognised content claims nothing beyond the PID"
        );
        assert!(
            DriverEvidence::Spawned < DriverEvidence::WatchesInstalled
                && DriverEvidence::WatchesInstalled < DriverEvidence::FreshActivity,
            "evidence orders weakest to strongest"
        );
    }

    #[test]
    fn save_time_driver_respawn_never_inherits_the_previous_child_marker() {
        let h = harness();
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();
        let marker = h.dir.join(format!(
            "{}.{READY_MARKER_EXTENSION}",
            worktree_artifact_stem(worktree)
        ));
        std::fs::write(&marker, READY_MARKER_ACTIVITY).expect("marker");
        assert!(matches!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Attached {
                evidence: DriverEvidence::FreshActivity,
                ..
            })
        ));

        h.state.alive.lock().expect("alive").remove(&41);
        enqueue(&h, MembershipChange::Refreshed, worktree);
        h.supervisor.process_pending();
        assert_eq!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Attached {
                pid: 42,
                evidence: DriverEvidence::Spawned,
            }),
            "the new child starts with no evidence beyond its PID"
        );
        assert!(!marker.exists(), "stale marker removed before the spawn");

        // Unregister clears the marker too.
        std::fs::write(&marker, READY_MARKER_WATCHING).expect("marker");
        enqueue(&h, MembershipChange::Unregistered, worktree);
        h.supervisor.process_pending();
        assert!(!marker.exists(), "marker removed on stop");
    }

    /// JREL-003: a real detached child that died is reaped and reported dead
    /// — never kept alive as a zombie whose PID and start time still match.
    #[cfg(unix)]
    #[test]
    // The un-waited child is the point: the supervisor never waits on its
    // detached children, and the zombie is the state under test.
    #[allow(clippy::zombie_processes)]
    fn save_time_driver_system_process_control_reaps_a_dead_child() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn a real child");
        let pid = child.id();
        let procs = SystemProcessControl;
        let start_time = procs.start_time(pid);
        assert!(procs.is_alive(pid, start_time), "a running child is alive");
        child.kill().expect("kill the child");
        // Do NOT `wait` here: the supervisor never waits on its detached
        // children, so this is exactly the zombie state it must see through.
        let deadline = Instant::now() + Duration::from_secs(5);
        while procs.is_alive(pid, start_time) {
            assert!(
                Instant::now() < deadline,
                "a killed child must be reported dead (it was a zombie)"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        // Reaped: a second probe cannot resurrect it either.
        assert!(!procs.is_alive(pid, start_time));
    }

    #[test]
    fn save_time_driver_stop_all_terminates_every_live_child() {
        let h = harness();
        enqueue(&h, MembershipChange::Registered, Path::new("/ws/a"));
        enqueue(&h, MembershipChange::Registered, Path::new("/ws/b"));
        h.supervisor.process_pending();
        h.supervisor.stop_all();
        let mut terminated = h.state.terminated.lock().expect("terminated").clone();
        terminated.sort_unstable();
        assert_eq!(terminated, vec![41, 42]);
        assert!(h.supervisor.status_snapshot().is_empty());
    }

    #[test]
    fn save_time_driver_reconcile_sweeps_stale_and_kills_leftovers() {
        let h = harness();
        std::fs::create_dir_all(&h.dir).expect("dir");
        // A leftover child from a previous daemon life, still alive with a
        // matching start time.
        h.state.alive.lock().expect("alive").insert(900, 12345);
        std::fs::write(h.dir.join("left-abc123.pid"), "900\n12345\n").expect("write");
        // A dead child's record.
        std::fs::write(h.dir.join("dead-def456.pid"), "901\n67890\n").expect("write");
        // A recycled PID: alive but with a different start time — must NOT be
        // signalled.
        h.state.alive.lock().expect("alive").insert(902, 999);
        std::fs::write(h.dir.join("recycled-0a0a0a.pid"), "902\n111\n").expect("write");
        // Malformed record: swept, never signalled.
        std::fs::write(h.dir.join("junk-ffffff.pid"), "not a pid\n").expect("write");
        // A non-PID file is left alone.
        std::fs::write(h.dir.join("left-abc123.log"), "findings\n").expect("write");

        h.supervisor.reconcile_on_start();

        assert_eq!(
            *h.state.terminated.lock().expect("terminated"),
            vec![900],
            "only the verified-live leftover is terminated"
        );
        for swept in [
            "left-abc123.pid",
            "dead-def456.pid",
            "recycled-0a0a0a.pid",
            "junk-ffffff.pid",
        ] {
            assert!(!h.dir.join(swept).exists(), "{swept} swept");
        }
        assert!(h.dir.join("left-abc123.log").exists(), "logs survive");
    }

    #[test]
    fn save_time_driver_stop_all_blocks_spawns_still_in_the_queue() {
        // A Registered event drained AFTER stop_all (the consumer task may
        // still be running on the listener-failure exit path) must not
        // create an orphan child.
        let h = harness();
        enqueue(&h, MembershipChange::Registered, Path::new("/ws/late"));
        h.supervisor.stop_all();
        h.supervisor.process_pending();
        assert!(
            h.state.spawns.lock().expect("spawns").is_empty(),
            "no spawn after shutdown latched"
        );
    }

    #[test]
    fn save_time_driver_unreadable_start_time_at_spawn_stops_and_fails() {
        // On a start-time-capable platform a driver must never be TRACKED
        // without its PID-reuse discriminator: the just-spawned child (still
        // provably ours) is stopped immediately and reported failed.
        let h = harness();
        h.state.withhold_start_time.store(true, Ordering::SeqCst);
        let worktree = Path::new("/ws/repo");
        enqueue(&h, MembershipChange::Registered, worktree);
        h.supervisor.process_pending();

        assert_eq!(
            *h.state.terminated.lock().expect("terminated"),
            vec![41],
            "the discriminator-less child is stopped at once"
        );
        assert_eq!(
            h.supervisor.driver_status(worktree),
            Some(DriverStatus::Failed)
        );
        assert!(
            !h.dir
                .join(format!("{}.pid", worktree_artifact_stem(worktree)))
                .exists(),
            "no pid file for an untracked child"
        );
    }

    #[test]
    fn save_time_driver_reconcile_never_signals_a_record_without_start_time() {
        // The spawn-time start-time read transiently failed on a platform
        // that supports it: the bare PID could be recycled across the daemon
        // restart, so the record is swept WITHOUT signalling.
        let h = harness();
        std::fs::create_dir_all(&h.dir).expect("dir");
        h.state.alive.lock().expect("alive").insert(903, 555);
        std::fs::write(h.dir.join("bare-abcdef.pid"), "903\n").expect("write");

        h.supervisor.reconcile_on_start();

        assert!(
            h.state.terminated.lock().expect("terminated").is_empty(),
            "a start-time-less record is never signalled"
        );
        assert!(!h.dir.join("bare-abcdef.pid").exists(), "record swept");
    }

    #[test]
    fn save_time_driver_default_dir_precedence_matches_the_child() {
        // ANVIL_HOME re-roots under <prefix>/runtime/ — the ADR-101 path the
        // driver child's default log resolution also uses (NOT the PID-file
        // parent, which skips the runtime/ segment under ANVIL_HOME).
        let with_home = driver_dir_from(
            Some(PathBuf::from("/anvil-home")),
            Some(PathBuf::from("/run/user/1000")),
            None,
            Some(PathBuf::from("/home/u")),
        )
        .expect("resolve");
        assert_eq!(
            with_home,
            PathBuf::from("/anvil-home/runtime/save-time-drivers")
        );

        let with_xdg = driver_dir_from(
            None,
            Some(PathBuf::from("/run/user/1000")),
            None,
            Some(PathBuf::from("/home/u")),
        )
        .expect("resolve");
        assert_eq!(
            with_xdg,
            PathBuf::from("/run/user/1000/anvil/save-time-drivers")
        );

        let with_state_home =
            driver_dir_from(None, None, None, Some(PathBuf::from("/home/u"))).expect("resolve");
        assert_eq!(
            with_state_home,
            PathBuf::from("/home/u/.local/state/anvil/save-time-drivers")
        );
    }

    #[test]
    fn save_time_driver_reconcile_missing_dir_is_a_noop() {
        let h = harness();
        h.supervisor.reconcile_on_start();
        assert!(h.state.terminated.lock().expect("terminated").is_empty());
    }

    #[test]
    fn save_time_driver_registry_register_reaches_the_queue() {
        // Integration with ACTMO-014: a durable register on a real registry
        // fires the hook synchronously; the event must land in the queue, not
        // spawn inline. Only spine-tagged (durable) sessions signal membership.
        use anvil_intercept_proto::SessionId;
        use anvil_intercept_proto::session::AgentTag;

        let h = harness();
        let tmp = tempfile::tempdir().expect("tempdir");
        let registry = crate::registry::SessionRegistry::new();
        assert!(registry.set_membership_hook(h.supervisor.membership_hook()));
        let spine = AgentTag::new(
            "anvil-start",
            anvil_intercept_proto::session::ACTIVATION_SPINE_CLAIMED_AGENT_ID,
            0,
        );
        registry
            .register(
                &SessionId::new("sess-driver-test"),
                tmp.path(),
                Some(&spine),
                std::time::Instant::now(),
            )
            .expect("register");
        assert!(h.state.spawns.lock().expect("spawns").is_empty());
        assert_eq!(h.supervisor.process_pending(), 1, "one Registered event");
        assert_eq!(h.state.spawns.lock().expect("spawns").len(), 1);
    }

    #[test]
    fn save_time_driver_durable_heartbeat_restores_a_dead_child() {
        // JREL-003: the public re-run path. A second bare `anvil` / `anvil
        // start` against a worktree whose child died is refused by
        // `session.register` (`SessionAlreadyExists`) and the client
        // heartbeats the existing owner instead. That heartbeat — not a
        // membership gain — must restore exactly one driver.
        use anvil_intercept_proto::SessionId;
        use anvil_intercept_proto::session::{ACTIVATION_SPINE_CLAIMED_AGENT_ID, AgentTag};

        use crate::registry::{RegistryError, SessionDispatcher, SessionRegistry};

        let h = harness();
        let tmp = tempfile::tempdir().expect("tempdir");
        let worktree = std::fs::canonicalize(tmp.path()).expect("canonical worktree");
        let registry = SessionRegistry::new();
        assert!(registry.set_membership_hook(h.supervisor.membership_hook()));
        let session = SessionId::new("sess-jrel003");
        let spine = AgentTag::new("anvil-start", ACTIVATION_SPINE_CLAIMED_AGENT_ID, 0);
        registry
            .register(&session, &worktree, Some(&spine), std::time::Instant::now())
            .expect("first registration");
        h.supervisor.process_pending();
        assert!(matches!(
            h.supervisor.driver_status(&worktree),
            Some(DriverStatus::Attached { pid: 41, .. })
        ));

        // The child dies while the daemon lives.
        h.state.alive.lock().expect("alive").remove(&41);
        assert_eq!(
            h.supervisor.driver_status(&worktree),
            Some(DriverStatus::Failed)
        );

        // Re-run: the same deterministic session id is refused, so the CLI
        // falls back to a heartbeat over the dispatcher (the IPC path).
        assert!(matches!(
            registry.register(&session, &worktree, Some(&spine), std::time::Instant::now()),
            Err(RegistryError::SessionAlreadyExists(_))
        ));
        SessionDispatcher::heartbeat(&registry, &session, None).expect("heartbeat");
        h.supervisor.process_pending();

        assert!(
            matches!(
                h.supervisor.driver_status(&worktree),
                Some(DriverStatus::Attached { pid: 42, .. })
            ),
            "a durable heartbeat must restore the dead driver, got {:?}",
            h.supervisor.driver_status(&worktree)
        );
        assert_eq!(
            h.state
                .spawns
                .lock()
                .expect("spawns")
                .iter()
                .filter(|(wt, _, _)| *wt == worktree)
                .count(),
            2,
            "exactly one respawn for the dead child"
        );

        // A heartbeat against a LIVE child is idempotent — no second driver.
        SessionDispatcher::heartbeat(&registry, &session, None).expect("heartbeat");
        h.supervisor.process_pending();
        assert_eq!(
            h.state
                .spawns
                .lock()
                .expect("spawns")
                .iter()
                .filter(|(wt, _, _)| *wt == worktree)
                .count(),
            2,
            "a refresh of a live driver must not spawn another"
        );
    }

    #[test]
    fn save_time_driver_artifact_stem_is_stable_and_distinct() {
        let a1 = worktree_artifact_stem(Path::new("/ws/repo"));
        let a2 = worktree_artifact_stem(Path::new("/ws/repo"));
        let b = worktree_artifact_stem(Path::new("/elsewhere/repo"));
        assert_eq!(a1, a2, "stable across runs");
        assert_ne!(a1, b, "same leaf, different roots must not collide");
        assert!(a1.starts_with("repo-"), "{a1}");
    }

    #[test]
    fn save_time_driver_pid_record_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("wt.pid");
        std::fs::write(&path, "77\n123456\n").expect("write");
        assert_eq!(read_pid_record(&path), Some((77, Some(123_456))));
        std::fs::write(&path, "77\n").expect("write");
        assert_eq!(read_pid_record(&path), Some((77, None)));
        std::fs::write(&path, "garbage\n").expect("write");
        assert_eq!(read_pid_record(&path), None);
    }
}
