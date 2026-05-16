//! INTD-001: Anvil intercept daemon library surface.
//!
//! This A1 scaffold establishes:
//!
//! - A `run_foreground` entry point with cooperative shutdown via a
//!   tokio cancellation handle. The CLI calls into this from
//!   `anvil intercept start --foreground`; tests drive it through the
//!   same path without sending real signals.
//! - A future `Daemon` lifecycle handle (INTD-002 onwards) that
//!   subsequent tasks (INTD-002 IPC listener, INTD-003 session
//!   registry, INTD-005 enforcement pipeline) attach behind without
//!   touching the CLI surface.
//! - [`wait_for_shutdown_signal`] — the single source of truth for
//!   signal handling shared by the daemon binary and the CLI
//!   subcommand, so SIGINT and (on Unix) SIGTERM cannot drift between
//!   entry points.
//!
//! Intentionally out of scope here:
//!
//! - PID files (deferred until INTD-002 lands the IPC listener that
//!   actually needs a single-instance guard).
//! - Backgrounded / double-fork daemonisation (INTD-002+).
//! - Cross-platform signal handling beyond SIGINT and Unix SIGTERM.
//!   Windows `JobObject` termination arrives with INTD-006.
//!
//! See `plans/modules/intercept-daemon.aps.md` and
//! `plans/decisions/015-intercept-loop-enforcement.md`.

#![forbid(unsafe_code)]

pub mod auth;
pub mod config;
pub mod dos;
pub mod embedded;
pub mod enforcement;
pub mod fanout;
pub mod fence;
pub mod interrupt;
pub mod ipc;
pub mod kindling_observation;
pub mod latency;
pub mod midedit;
pub mod rate_window;
pub mod registry;
pub mod rule_cache;
pub mod status;
pub mod tag_env;
pub mod telemetry;
pub mod unregistered;
pub mod watcher;

pub use auth::{
    AuthError, CapabilityDowngrade, CapabilityDowngradeReason, DriverManifest, is_driver_allowed,
    negotiate_capability,
};
pub use registry::{
    Attribution, DEFAULT_HEARTBEAT_TTL, ProcessInfo, RegistryError, SessionDispatcher,
    SessionRegistry,
};

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(unix)]
use nix::errno::Errno;
#[cfg(unix)]
use nix::sys::signal::kill;
#[cfg(unix)]
use nix::unistd::{Pid, geteuid};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};

use anyhow::{Context, Result};
use tokio::sync::watch;
use tokio::task::JoinHandle;

#[derive(Clone)]
struct RegistryDispatcher {
    registry: Arc<SessionRegistry>,
    fence_store: Arc<fence::FenceStore>,
}

impl RegistryDispatcher {
    fn new(registry: Arc<SessionRegistry>, fence_store: Arc<fence::FenceStore>) -> Self {
        Self {
            registry,
            fence_store,
        }
    }
}

impl SessionDispatcher for RegistryDispatcher {
    fn register(
        &self,
        id: &anvil_intercept_proto::SessionId,
        worktree: &Path,
        agent_tag: Option<&anvil_intercept_proto::session::AgentTag>,
        lineage: Option<&anvil_intercept_proto::session::LineageAnchor>,
    ) -> Result<(), RegistryError> {
        // MLP2-026: cascade-before-registry lock ordering (spec §6
        // inv-2). Snapshot the fence-store state in a single load
        // call; release the implicit fence-file lock by letting the
        // FenceState value go out of scope before
        // SessionRegistry::register acquires its Inner mutex inside
        // the downstream call. The fence check and the cascade check
        // share the same snapshot so they never disagree about which
        // worktree is in which mode.
        let fences =
            self.fence_store
                .load()
                .map_err(|err| RegistryError::FenceStateUnavailable {
                    message: err.to_string(),
                })?;
        if fences.is_fenced(worktree) {
            return Err(RegistryError::WorktreeFenced {
                worktree: worktree.to_path_buf(),
            });
        }
        if fences.is_cascaded(worktree) {
            return Err(RegistryError::WorktreeCascaded {
                worktree: worktree.to_path_buf(),
            });
        }
        SessionDispatcher::register(self.registry.as_ref(), id, worktree, agent_tag, lineage)
    }

    fn heartbeat(&self, id: &anvil_intercept_proto::SessionId) -> Result<(), RegistryError> {
        SessionDispatcher::heartbeat(self.registry.as_ref(), id)
    }

    fn unregister(&self, id: &anvil_intercept_proto::SessionId) -> Result<bool, RegistryError> {
        SessionDispatcher::unregister(self.registry.as_ref(), id)
    }

    fn list(&self) -> Vec<anvil_intercept_proto::SessionRecord> {
        SessionDispatcher::list(self.registry.as_ref())
    }
}

struct DaemonState {
    registry: Arc<SessionRegistry>,
    fence_store: Arc<fence::FenceStore>,
    fences: Arc<fence::FenceState>,
}

impl DaemonState {
    fn new(fence_store: fence::FenceStore, fences: fence::FenceState) -> Self {
        Self {
            registry: Arc::new(SessionRegistry::new()),
            fence_store: Arc::new(fence_store),
            fences: Arc::new(fences),
        }
    }

    fn active_fence_count(&self) -> usize {
        self.fences.active_fences().len()
    }
}

/// Options accepted by [`run_foreground`]. Future tasks add the socket
/// path, config path, and observe-only flag here.
#[derive(Debug, Default, Clone)]
pub struct ForegroundOpts {
    pid_file: Option<PathBuf>,
    fence_store: Option<PathBuf>,
    scan_buffer: midedit::ScanBufferService,
    #[cfg(unix)]
    ipc_socket: Option<PathBuf>,
    #[cfg(windows)]
    ipc_pipe_name: Option<String>,
}

impl ForegroundOpts {
    /// Override the PID file path. Used by tests and by future service
    /// managers that need to pin state into a caller-owned runtime dir.
    #[must_use]
    pub fn with_pid_file(pid_file: impl Into<PathBuf>) -> Self {
        Self {
            pid_file: Some(pid_file.into()),
            fence_store: None,
            scan_buffer: midedit::ScanBufferService::default(),
            #[cfg(unix)]
            ipc_socket: None,
            #[cfg(windows)]
            ipc_pipe_name: None,
        }
    }

    /// Override both PID file and Unix IPC socket paths. Used by tests
    /// so daemon integration can run without mutating process env.
    #[cfg(unix)]
    #[must_use]
    pub fn with_pid_file_and_ipc_socket(
        pid_file: impl Into<PathBuf>,
        ipc_socket: impl Into<PathBuf>,
    ) -> Self {
        Self {
            pid_file: Some(pid_file.into()),
            fence_store: None,
            scan_buffer: midedit::ScanBufferService::default(),
            ipc_socket: Some(ipc_socket.into()),
        }
    }

    /// Override both PID file and Windows named-pipe paths. Used by
    /// Windows tests so parallel cases do not contend on the per-user pipe.
    #[cfg(windows)]
    #[must_use]
    pub fn with_pid_file_and_ipc_pipe_name(
        pid_file: impl Into<PathBuf>,
        ipc_pipe_name: impl Into<String>,
    ) -> Self {
        Self {
            pid_file: Some(pid_file.into()),
            fence_store: None,
            scan_buffer: midedit::ScanBufferService::default(),
            ipc_pipe_name: Some(ipc_pipe_name.into()),
        }
    }

    /// Override the persistent fence state file. Tests use this to keep
    /// daemon startup away from the caller's real user-state directory.
    #[must_use]
    pub fn with_fence_store_file(mut self, fence_store: impl Into<PathBuf>) -> Self {
        self.fence_store = Some(fence_store.into());
        self
    }

    /// Override the scan-buffer service used by the IPC listener for the
    /// `scan_buffer` mid-edit RPC. Tests inject a fixture-shaped service
    /// with a known rule registry.
    #[must_use]
    pub fn with_scan_buffer_service(mut self, scan_buffer: midedit::ScanBufferService) -> Self {
        self.scan_buffer = scan_buffer;
        self
    }

    fn pid_file_path(&self) -> Result<PathBuf> {
        self.pid_file.clone().map_or_else(default_pid_file_path, Ok)
    }

    fn fence_store_path(&self) -> Result<PathBuf> {
        self.fence_store
            .clone()
            .map_or_else(fence::default_fence_state_path, Ok)
            .context("failed to resolve intercept fence store path")
    }

    #[cfg(unix)]
    fn ipc_socket_path(&self) -> Option<&Path> {
        self.ipc_socket.as_deref()
    }

    #[cfg(windows)]
    fn ipc_pipe_name(&self) -> Option<&str> {
        self.ipc_pipe_name.as_deref()
    }
}

struct AbortOnDropJoinHandle<T> {
    handle: Option<JoinHandle<T>>,
}

impl<T> AbortOnDropJoinHandle<T> {
    fn new(handle: JoinHandle<T>) -> Self {
        Self {
            handle: Some(handle),
        }
    }

    async fn join(&mut self) -> std::result::Result<T, tokio::task::JoinError> {
        self.handle.as_mut().expect("join handle missing").await
    }

    fn abort(&self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
    }
}

impl<T> Drop for AbortOnDropJoinHandle<T> {
    fn drop(&mut self) {
        if let Some(handle) = &self.handle
            && !handle.is_finished()
        {
            handle.abort();
        }
    }
}

/// Resolve the default PID file location for the current user.
///
/// The path intentionally matches the daemon runtime directory used by
/// the demo reset path: `$XDG_RUNTIME_DIR/anvil` when available, falling
/// back to `$HOME/.local/state/anvil` on Unix-like hosts and
/// `%LOCALAPPDATA%\anvil` on Windows.
pub fn default_pid_file_path() -> Result<PathBuf> {
    if let Some(runtime_dir) = non_empty_env("XDG_RUNTIME_DIR") {
        return Ok(runtime_dir.join("anvil").join("intercept.pid"));
    }

    if cfg!(windows)
        && let Some(local_app_data) = non_empty_env("LOCALAPPDATA")
    {
        return Ok(local_app_data.join("anvil").join("intercept.pid"));
    }

    let home = non_empty_env("HOME")
        .or_else(|| non_empty_env("USERPROFILE"))
        .context("cannot resolve home directory for anvil intercept PID file")?;
    Ok(home
        .join(".local")
        .join("state")
        .join("anvil")
        .join("intercept.pid"))
}

fn non_empty_env(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[derive(Debug)]
struct PidFileGuard {
    path: PathBuf,
    identity: PidFileIdentity,
}

impl PidFileGuard {
    fn acquire(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            ensure_secure_runtime_dir(parent)?;
        }

        match Self::create(path) {
            Ok(guard) => Ok(guard),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                recover_stale_pid_file(path)?;
                Self::create(path)
                    .with_context(|| format!("failed to re-create PID file {}", path.display()))
            }
            Err(err) => {
                Err(err).with_context(|| format!("failed to create PID file {}", path.display()))
            }
        }
    }

    fn create(path: &Path) -> std::io::Result<Self> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let record = write_pid_record(&mut file)?;
        let identity = PidFileIdentity::from_file(&file, record)?;
        Ok(Self {
            path: path.to_path_buf(),
            identity,
        })
    }
}

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        if !self.identity.matches_path(&self.path) {
            return;
        }

        if let Err(err) = fs::remove_file(&self.path)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "anvil-intercept: failed to remove PID file {}: {err}",
                self.path.display()
            );
        }
    }
}

fn ensure_secure_runtime_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        ensure_secure_runtime_dir_unix(path)
    }

    #[cfg(not(unix))]
    {
        fs::create_dir_all(path)
            .with_context(|| format!("failed to create PID file directory {}", path.display()))
    }
}

#[cfg(unix)]
fn ensure_secure_runtime_dir_unix(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => verify_secure_runtime_dir(path, &metadata),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create PID file parent {}", parent.display())
                })?;
            }

            fs::DirBuilder::new()
                .mode(0o700)
                .create(path)
                .or_else(|err| {
                    if err.kind() == std::io::ErrorKind::AlreadyExists {
                        Ok(())
                    } else {
                        Err(err)
                    }
                })
                .with_context(|| {
                    format!("failed to create PID file directory {}", path.display())
                })?;
            let metadata = fs::symlink_metadata(path)
                .with_context(|| format!("failed to stat PID file directory {}", path.display()))?;
            verify_secure_runtime_dir(path, &metadata)
        }
        Err(err) => Err(err)
            .with_context(|| format!("failed to stat PID file directory {}", path.display())),
    }
}

#[cfg(unix)]
fn verify_secure_runtime_dir(path: &Path, metadata: &fs::Metadata) -> Result<()> {
    if metadata.file_type().is_symlink() {
        anyhow::bail!("refusing symlink PID file directory {}", path.display());
    }
    if !metadata.is_dir() {
        anyhow::bail!("PID file directory is not a directory: {}", path.display());
    }

    let expected_uid = geteuid().as_raw();
    if metadata.uid() != expected_uid {
        anyhow::bail!(
            "PID file directory {} is owned by uid {}, expected {}",
            path.display(),
            metadata.uid(),
            expected_uid,
        );
    }

    let mode = metadata.permissions().mode() & 0o777;
    if mode != 0o700 {
        anyhow::bail!(
            "PID file directory {} has mode {:o}, expected 700",
            path.display(),
            mode,
        );
    }

    Ok(())
}

fn write_pid_record(file: &mut File) -> std::io::Result<String> {
    let mut record = format!("{}\n", process::id());
    if let Some(start_time) = process_start_time(process::id()) {
        record.push_str("start_time=");
        record.push_str(&start_time.to_string());
        record.push('\n');
    }
    file.write_all(record.as_bytes())?;
    file.sync_all()?;
    Ok(record)
}

fn recover_stale_pid_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect existing PID file {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        anyhow::bail!("refusing symlink PID file {}", path.display());
    }

    let record = fs::read_to_string(path)
        .with_context(|| format!("failed to read existing PID file {}", path.display()))?;
    match existing_pid_status(&record) {
        ExistingPidStatus::Stale => {}
        ExistingPidStatus::Live | ExistingPidStatus::Unknown => {
            anyhow::bail!(
                "anvil intercept daemon is already running or PID file cannot be proven stale at {}",
                path.display(),
            );
        }
    }

    fs::remove_file(path)
        .with_context(|| format!("failed to remove stale PID file {}", path.display()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExistingPidStatus {
    Live,
    Stale,
    Unknown,
}

fn existing_pid_status(record: &str) -> ExistingPidStatus {
    let Some(pid) = record
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
    else {
        return ExistingPidStatus::Unknown;
    };

    if pid == process::id() {
        return ExistingPidStatus::Live;
    }

    let recorded_start_time = record
        .lines()
        .find_map(|line| line.strip_prefix("start_time="))
        .and_then(|value| value.parse::<u64>().ok());

    if !process_exists(pid) {
        return ExistingPidStatus::Stale;
    }

    if let (Some(expected), Some(actual)) = (recorded_start_time, process_start_time(pid)) {
        if expected == actual {
            ExistingPidStatus::Live
        } else {
            ExistingPidStatus::Stale
        }
    } else {
        ExistingPidStatus::Live
    }
}

#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };

    !matches!(kill(Pid::from_raw(pid), None), Err(Errno::ESRCH))
}

#[cfg(windows)]
fn process_exists(pid: u32) -> bool {
    anvil_intercept_win32::process_exists(pid).unwrap_or(true)
}

#[cfg(not(any(unix, windows)))]
fn process_exists(_pid: u32) -> bool {
    true
}

#[cfg(target_os = "linux")]
fn process_start_time(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(Path::new("/proc").join(pid.to_string()).join("stat")).ok()?;
    let after_command = stat.rsplit_once(") ")?.1;
    after_command.split_whitespace().nth(19)?.parse().ok()
}

#[cfg(windows)]
fn process_start_time(pid: u32) -> Option<u64> {
    anvil_intercept_win32::process_creation_time(pid)
        .ok()
        .flatten()
}

#[cfg(not(any(target_os = "linux", windows)))]
fn process_start_time(_pid: u32) -> Option<u64> {
    None
}

#[derive(Debug)]
struct PidFileIdentity {
    record: String,
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(not(unix))]
    pid: u32,
}

impl PidFileIdentity {
    fn from_file(file: &File, record: String) -> std::io::Result<Self> {
        let metadata = file.metadata()?;
        Ok(Self::from_metadata(&metadata, record))
    }

    #[cfg(unix)]
    fn from_metadata(metadata: &fs::Metadata, record: String) -> Self {
        Self {
            record,
            dev: metadata.dev(),
            ino: metadata.ino(),
        }
    }

    #[cfg(not(unix))]
    fn from_metadata(_metadata: &fs::Metadata, record: String) -> Self {
        Self {
            record,
            pid: process::id(),
        }
    }

    fn matches_path(&self, path: &Path) -> bool {
        let Ok(record) = fs::read_to_string(path) else {
            return false;
        };
        if record != self.record {
            return false;
        }

        #[cfg(unix)]
        {
            let Ok(metadata) = fs::metadata(path) else {
                return false;
            };
            metadata.dev() == self.dev && metadata.ino() == self.ino
        }

        #[cfg(not(unix))]
        {
            record
                .lines()
                .next()
                .and_then(|line| line.trim().parse::<u32>().ok())
                == Some(self.pid)
        }
    }
}

/// Cooperative shutdown handle. Held by the caller; calling
/// [`Shutdown::trigger`] flips the watch channel and the foreground
/// loop returns at its next await point.
#[derive(Debug, Clone)]
pub struct Shutdown {
    tx: watch::Sender<bool>,
}

impl Shutdown {
    /// Build a fresh shutdown handle plus the receiver the daemon
    /// loop awaits on. Tests construct one of these directly; the
    /// `--foreground` CLI path wires the receiver to
    /// [`wait_for_shutdown_signal`].
    #[must_use]
    pub fn new() -> (Self, ShutdownToken) {
        let (tx, rx) = watch::channel(false);
        (Self { tx }, ShutdownToken { rx })
    }

    /// Mint a fresh [`ShutdownToken`] from this handle. The new token
    /// observes the current shutdown state immediately, so a token
    /// minted after [`Shutdown::trigger`] resolves on the next
    /// [`ShutdownToken::cancelled`] without waiting.
    ///
    /// Use this when a downstream consumer (an INTD-002 IPC handler,
    /// for example) needs its own token but the original receiver
    /// has already been moved into another future.
    #[must_use]
    pub fn token(&self) -> ShutdownToken {
        ShutdownToken {
            rx: self.tx.subscribe(),
        }
    }

    /// Request shutdown. Idempotent — repeated calls are a no-op.
    ///
    /// Uses `send_replace`, which never fails: it overwrites the
    /// watched value regardless of receiver count. Even after every
    /// [`ShutdownToken`] has been dropped (no one to notify), the
    /// trigger is recorded — any token minted later via
    /// [`Shutdown::token`] observes the triggered state on its first
    /// [`ShutdownToken::cancelled`] call.
    pub fn trigger(&self) {
        self.tx.send_replace(true);
    }
}

/// Receiver-side of [`Shutdown`]. Awaiting [`ShutdownToken::cancelled`]
/// resolves once `trigger` has been called.
#[derive(Debug, Clone)]
pub struct ShutdownToken {
    rx: watch::Receiver<bool>,
}

impl ShutdownToken {
    /// Resolve when shutdown has been requested.
    ///
    /// Takes `&mut self` because [`watch::Receiver::changed`] requires
    /// it. Callers that need to await cancellation from multiple
    /// `tokio::select!` arms simultaneously must clone the token —
    /// `ShutdownToken` is `Clone` and cloning a `watch::Receiver` is
    /// cheap. INTD-002 onwards is expected to hold one cloned token
    /// per spawned handler future; the registry-style "share one
    /// token across consumers" idiom needs to clone first.
    pub async fn cancelled(&mut self) {
        // Already triggered before we awaited.
        if *self.rx.borrow_and_update() {
            return;
        }
        // `changed()` resolves when the watched value transitions; if
        // every sender drops we treat that as a cancellation too,
        // because no one can flip the flag any more.
        let _ = self.rx.changed().await;
    }
}

/// Wait for the operating system to ask the daemon to stop, on every
/// platform the daemon supports.
///
/// - Unix: races SIGINT (via [`tokio::signal::ctrl_c`]) and SIGTERM
///   (via [`tokio::signal::unix`]). Either wakes the future. SIGTERM
///   is the signal `kill <pid>`, `systemd stop`, Docker, and
///   Kubernetes use; SIGINT is the controlling-terminal Ctrl+C.
/// - Windows: only Ctrl+C is wired today. Process-manager
///   termination on Windows uses `JobObject` semantics, which
///   INTD-006 owns.
///
/// Both intercept entrypoints (`anvil intercept start --foreground`
/// in the CLI, the standalone `anvil-intercept` binary) call this
/// helper. Keeping the signal logic in one place stops the two
/// entrypoints drifting — a shutdown signal that cleanly stops one
/// must cleanly stop the other.
///
/// Returns when any supported signal arrives; errors only if the
/// signal infrastructure itself fails to install (rare, generally
/// fatal).
pub async fn wait_for_shutdown_signal() -> Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate())
            .map_err(|err| anyhow::anyhow!("failed to install SIGTERM handler: {err}"))?;

        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                result.map_err(|err| anyhow::anyhow!("ctrl_c handler failed: {err}"))?;
            }
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .map_err(|err| anyhow::anyhow!("ctrl_c handler failed: {err}"))?;
    }

    Ok(())
}

/// Run the intercept daemon in the current process. Blocks until
/// `shutdown` is triggered (by SIGINT/SIGTERM in production, or by
/// the caller in tests). The foreground daemon owns the session
/// registry, serves the IPC listener, and ticks stale-session eviction.
#[allow(clippy::too_many_lines)]
pub async fn run_foreground(opts: ForegroundOpts, mut token: ShutdownToken) -> Result<()> {
    let pid_file_path = opts.pid_file_path()?;
    let fence_store_path = opts.fence_store_path()?;
    let _pid_file = PidFileGuard::acquire(&pid_file_path)?;
    let fence_store = fence::FenceStore::at_path(&fence_store_path);
    let daemon_state = DaemonState::new(
        fence_store.clone(),
        fence_store.load().with_context(|| {
            format!("failed to load fence state {}", fence_store_path.display())
        })?,
    );
    if daemon_state.active_fence_count() > 0 {
        tracing::info!(
            target: "anvil_intercept::fence",
            count = daemon_state.active_fence_count(),
            "loaded persisted intercept fences before accepting connections",
        );
    }

    #[cfg(any(unix, windows))]
    {
        let dispatcher = RegistryDispatcher::new(
            Arc::clone(&daemon_state.registry),
            Arc::clone(&daemon_state.fence_store),
        );
        let scan_buffer = opts.scan_buffer.clone();
        // INTD-011: the production status provider reads sessions from
        // the daemon's registry, fences from the persisted store, and
        // the latency rollup from the same `ScanBufferService` the
        // listener serves with — so `query_status` reflects exactly
        // the state the daemon is currently using to evaluate
        // `scan_buffer` calls. The provider is built BEFORE the
        // listener so the listener gets a status feed wired in from
        // the first connection.
        let status_provider: Arc<dyn status::StatusProvider> = Arc::new(
            status::DaemonStatusProvider::new(
                Arc::clone(&daemon_state.registry),
                Arc::clone(&daemon_state.fence_store),
                scan_buffer.latency().clone(),
                Instant::now(),
                env!("CARGO_PKG_VERSION"),
            )
            // MLP2-058: wire `in_flight_evaluations` from the same
            // service the listener serves with. The rule_cache field
            // on `DaemonStatusProvider` stays `None` until MLP2-014
            // lands its production cache wire-up — the optional
            // wire shape preserves forward-compat.
            .with_scan_buffer(scan_buffer.clone()),
        );

        #[cfg(unix)]
        let listener = if let Some(socket_path) = opts.ipc_socket_path() {
            ipc::IpcListener::bind_with_scan_buffer_service(socket_path, dispatcher, scan_buffer)
        } else {
            ipc::IpcListener::bind_default_with_scan_buffer_service(dispatcher, scan_buffer)
        }
        .map(|listener| listener.with_status_provider(Arc::clone(&status_provider)))
        .context("failed to bind intercept IPC listener")?;

        #[cfg(windows)]
        let listener = if let Some(pipe_name) = opts.ipc_pipe_name() {
            ipc::IpcListener::bind_with_scan_buffer_service(pipe_name, dispatcher, scan_buffer)
        } else {
            ipc::IpcListener::bind_default_with_scan_buffer_service(dispatcher, scan_buffer)
        }
        .map(|listener| listener.with_status_provider(Arc::clone(&status_provider)))
        .context("failed to bind intercept IPC listener")?;

        let listener_token = token.clone();
        let mut listener_handle = AbortOnDropJoinHandle::new(tokio::spawn(async move {
            listener.serve(listener_token).await
        }));
        let mut tick = tokio::time::interval(Duration::from_millis(250));
        loop {
            tokio::select! {
                biased;
                () = token.cancelled() => break,
                result = listener_handle.join() => {
                    result
                        .context("intercept IPC listener task panicked")?
                        .context("intercept IPC listener failed")?;
                    return Ok(());
                }
                _ = tick.tick() => {
                    let evicted = daemon_state.registry.evict_stale(Instant::now());
                    if !evicted.is_empty() {
                        tracing::debug!(
                            target: "anvil_intercept::registry",
                            count = evicted.len(),
                            "evicted stale intercept sessions",
                        );
                    }
                }
            }
        }

        if let Ok(result) =
            tokio::time::timeout(Duration::from_secs(1), listener_handle.join()).await
        {
            result
                .context("intercept IPC listener task panicked")?
                .context("intercept IPC listener failed")?;
        } else {
            listener_handle.abort();
        }
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    {
        let mut tick = tokio::time::interval(Duration::from_millis(250));
        loop {
            tokio::select! {
                biased;
                () = token.cancelled() => break,
                _ = tick.tick() => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    use anvil_intercept_proto::SessionId;
    #[cfg(unix)]
    use anvil_intercept_proto::{IpcCommand, IpcEnvelope};
    use tokio::time::{sleep, timeout};

    use super::*;

    #[cfg(unix)]
    fn test_opts(pid_file: impl Into<PathBuf>) -> ForegroundOpts {
        let pid_file = pid_file.into();
        let ipc_socket = pid_file
            .parent()
            .expect("pid file has parent")
            .join("intercept.sock");
        let fence_store = pid_file
            .parent()
            .expect("pid file has parent")
            .join("intercept-fences.json");
        ForegroundOpts::with_pid_file_and_ipc_socket(pid_file, ipc_socket)
            .with_fence_store_file(fence_store)
    }

    #[cfg(windows)]
    fn test_opts(pid_file: impl Into<PathBuf>) -> ForegroundOpts {
        let pid_file = pid_file.into();
        let suffix =
            format!("{}-{}", std::process::id(), pid_file.display()).replace(['/', '\\', ':'], "-");
        let pipe_name = format!(r"\\.\pipe\anvil-intercept-test-{suffix}");
        let fence_store = pid_file
            .parent()
            .expect("pid file has parent")
            .join("intercept-fences.json");
        ForegroundOpts::with_pid_file_and_ipc_pipe_name(pid_file, pipe_name)
            .with_fence_store_file(fence_store)
    }

    #[cfg(not(any(unix, windows)))]
    fn test_opts(pid_file: impl Into<PathBuf>) -> ForegroundOpts {
        let pid_file = pid_file.into();
        let fence_store = pid_file
            .parent()
            .expect("pid file has parent")
            .join("intercept-fences.json");
        ForegroundOpts::with_pid_file(pid_file).with_fence_store_file(fence_store)
    }

    fn test_pid_file(tmp: &tempfile::TempDir) -> PathBuf {
        tmp.path().join("anvil").join("intercept.pid")
    }

    #[cfg(unix)]
    fn test_ipc_socket(tmp: &tempfile::TempDir) -> PathBuf {
        tmp.path().join("ipc").join("intercept.sock")
    }

    fn create_secure_test_pid_dir(path: &Path) {
        fs::create_dir(path).expect("create secure pid dir");
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .expect("set secure pid dir mode");
    }

    async fn wait_for_pid_file(pid_file: &Path) {
        for _ in 0..20 {
            if pid_file.exists() {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
        panic!("pid file was not created at {}", pid_file.display());
    }

    #[cfg(unix)]
    async fn wait_for_current_pid_record(pid_file: &Path) {
        let expected = std::process::id().to_string();
        for _ in 0..20 {
            if fs::read_to_string(pid_file)
                .ok()
                .and_then(|record| record.lines().next().map(str::to_owned))
                == Some(expected.clone())
            {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
        panic!("pid file was not replaced at {}", pid_file.display());
    }

    #[cfg(unix)]
    async fn wait_for_socket(socket: &Path) {
        for _ in 0..20 {
            if socket.exists() {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
        panic!("ipc socket was not created at {}", socket.display());
    }

    /// `Shutdown::trigger` before `run_foreground` is awaited still
    /// stops the loop on the first poll — the cancellation flag is
    /// observed via `borrow_and_update`, not just via `changed()`.
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_returns_when_shutdown_already_triggered() {
        let tmp = tempfile::tempdir().unwrap();
        let (shutdown, token) = Shutdown::new();
        shutdown.trigger();

        let result = timeout(
            Duration::from_secs(1),
            run_foreground(test_opts(test_pid_file(&tmp)), token),
        )
        .await
        .expect("foreground loop did not return after pre-triggered shutdown");
        result.expect("foreground loop reported error");
    }

    /// Triggering shutdown after the loop has started still resolves
    /// promptly — well inside the 250 ms tick interval is fine because
    /// `cancelled` resolves on the watch transition, not on the tick.
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_returns_when_shutdown_triggered_concurrently() {
        let (shutdown, token) = Shutdown::new();
        let tmp = tempfile::tempdir().unwrap();
        let handle = tokio::spawn(run_foreground(test_opts(test_pid_file(&tmp)), token));

        // Yield once so the spawned task enters its select.
        tokio::task::yield_now().await;
        shutdown.trigger();

        let result = timeout(Duration::from_secs(1), handle)
            .await
            .expect("foreground loop did not return after shutdown trigger")
            .expect("join failure");
        result.expect("foreground loop reported error");
    }

    /// Multiple `trigger` calls are idempotent and do not panic.
    #[tokio::test(flavor = "current_thread")]
    async fn shutdown_trigger_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let (shutdown, token) = Shutdown::new();
        shutdown.trigger();
        shutdown.trigger();
        shutdown.trigger();

        let result = timeout(
            Duration::from_secs(1),
            run_foreground(test_opts(test_pid_file(&tmp)), token),
        )
        .await
        .expect("foreground loop did not return after repeated triggers");
        result.expect("foreground loop reported error");
    }

    /// Trigger applied after every receiver dropped still records the
    /// state, and a fresh token minted via [`Shutdown::token`]
    /// observes it without further work. This is the property
    /// `send_replace` (used by `trigger`) gives us over `send`, which
    /// would silently no-op when no receivers exist.
    #[tokio::test(flavor = "current_thread")]
    async fn shutdown_trigger_survives_all_tokens_dropped() {
        let (shutdown, token) = Shutdown::new();
        drop(token);
        shutdown.trigger();

        // Mint a brand-new token from the handle and verify it
        // observes the triggered state. Without this assertion the
        // test would pass even if `trigger` became a no-op.
        let mut late_token = shutdown.token();
        let result = timeout(Duration::from_secs(1), late_token.cancelled()).await;
        assert!(
            result.is_ok(),
            "fresh token did not observe pre-triggered shutdown",
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_writes_pid_file_and_removes_it_on_shutdown() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_file = test_pid_file(&tmp);
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(run_foreground(test_opts(&pid_file), token));

        wait_for_pid_file(&pid_file).await;
        let pid = fs::read_to_string(&pid_file).expect("read pid file");
        assert_eq!(
            pid.lines().next(),
            Some(std::process::id().to_string().as_str())
        );

        shutdown.trigger();
        timeout(Duration::from_secs(1), handle)
            .await
            .expect("foreground loop did not return after shutdown")
            .expect("join failure")
            .expect("foreground loop reported error");
        assert!(!pid_file.exists(), "pid file should be removed on shutdown");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_accepts_ipc_registration() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::UnixStream;

        let tmp = tempfile::tempdir().unwrap();
        let pid_file = test_pid_file(&tmp);
        let socket = test_ipc_socket(&tmp);
        let fence_store = tmp.path().join("state/intercept-fences.json");
        let worktree = tmp.path().join("worktree");
        fs::create_dir(&worktree).expect("create worktree");

        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(run_foreground(
            ForegroundOpts::with_pid_file_and_ipc_socket(&pid_file, &socket)
                .with_fence_store_file(&fence_store),
            token,
        ));

        wait_for_pid_file(&pid_file).await;
        wait_for_socket(&socket).await;

        let mut stream = UnixStream::connect(&socket).await.expect("connect");
        let envelope = IpcEnvelope::notification(IpcCommand::RegisterSession {
            session_id: SessionId::new("sess_foreground"),
            worktree,
            agent_tag: None,
            lineage: None,
        });
        let mut line = serde_json::to_string(&envelope).expect("serialise envelope");
        line.push('\n');
        stream
            .write_all(line.as_bytes())
            .await
            .expect("write register");
        stream.shutdown().await.expect("shutdown client");

        shutdown.trigger();
        timeout(Duration::from_secs(1), handle)
            .await
            .expect("foreground loop did not return after shutdown")
            .expect("join failure")
            .expect("foreground loop reported error");
        assert!(!pid_file.exists(), "pid file should be removed on shutdown");
        assert!(!socket.exists(), "ipc socket should be removed on shutdown");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_loads_fences_before_binding_ipc() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_file = test_pid_file(&tmp);
        let socket = test_ipc_socket(&tmp);
        let fence_store = tmp.path().join("state/intercept-fences.json");
        fs::create_dir_all(fence_store.parent().expect("fence store parent"))
            .expect("create fence store parent");
        fs::write(&fence_store, "not json").expect("write corrupt fence store");
        let (_shutdown, token) = Shutdown::new();

        let err = run_foreground(
            ForegroundOpts::with_pid_file_and_ipc_socket(&pid_file, &socket)
                .with_fence_store_file(&fence_store),
            token,
        )
        .await
        .expect_err("corrupt fence store should stop startup");

        assert!(
            format!("{err:#}").contains("failed to load fence state"),
            "unexpected error: {err:#}",
        );
        assert!(
            !socket.exists(),
            "ipc socket should not bind before fences load"
        );
    }

    #[test]
    fn persisted_fence_blocks_session_registration_after_restart() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("worktree");
        fs::create_dir(&worktree).expect("create worktree");
        let store = fence::FenceStore::at_path(tmp.path().join("state/intercept-fences.json"));
        store
            .fence_worktree(&worktree, "restart fence")
            .expect("fence worktree");
        let registry = Arc::new(SessionRegistry::new());
        let dispatcher = RegistryDispatcher::new(Arc::clone(&registry), Arc::new(store));

        let err = dispatcher
            .register(&SessionId::new("sess-fenced"), &worktree, None, None)
            .expect_err("fenced worktree must reject registration");

        assert!(matches!(err, RegistryError::WorktreeFenced { .. }));
        assert!(registry.active_sessions().is_empty());
    }

    /// MLP2-026: cascaded worktree refuses new session
    /// registrations with `RegistryError::WorktreeCascaded`. Pin
    /// the cascade-before-registry lock ordering (spec §6 inv-2)
    /// — `register()` returns the error BEFORE any registry-side
    /// state is touched.
    #[test]
    fn dispatcher_refuses_cascaded_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("worktree");
        fs::create_dir(&worktree).expect("create worktree");
        let store = fence::FenceStore::at_path(tmp.path().join("state/intercept-fences.json"));
        // Fire 5 fences to engage the cascade (capacity 4 → 5th
        // returns Throttle).
        for i in 0..5 {
            store
                .fence_worktree(&worktree, format!("fire {i}"))
                .expect("fence");
        }
        assert!(store.is_cascaded(&worktree));

        // unblock_worktree clears the per-fire fence but NOT the
        // cascade (spec §10 Q4: distinct affordances).
        store.unblock_worktree(&worktree).expect("unblock");

        let registry = Arc::new(SessionRegistry::new());
        let dispatcher = RegistryDispatcher::new(Arc::clone(&registry), Arc::new(store.clone()));
        let err = dispatcher
            .register(&SessionId::new("sess-cascaded"), &worktree, None, None)
            .expect_err("cascaded worktree must reject registration");
        assert!(matches!(err, RegistryError::WorktreeCascaded { .. }));
        assert!(
            registry.active_sessions().is_empty(),
            "no session created before the cascade refusal"
        );

        // After clear_cascade, registration succeeds.
        store.clear_cascade(&worktree).expect("clear cascade");
        dispatcher
            .register(&SessionId::new("sess-after-clear"), &worktree, None, None)
            .expect("clear cascade unblocks registration");
        assert_eq!(registry.active_sessions().len(), 1);
    }

    #[test]
    fn dispatcher_observes_live_fence_store_updates() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path().join("worktree");
        fs::create_dir(&worktree).expect("create worktree");
        let store = fence::FenceStore::at_path(tmp.path().join("state/intercept-fences.json"));
        let registry = Arc::new(SessionRegistry::new());
        let dispatcher = RegistryDispatcher::new(Arc::clone(&registry), Arc::new(store.clone()));

        store
            .fence_worktree(&worktree, "live fence")
            .expect("fence worktree");
        let err = dispatcher
            .register(&SessionId::new("sess-fenced"), &worktree, None, None)
            .expect_err("new fence must affect running dispatcher");
        assert!(matches!(err, RegistryError::WorktreeFenced { .. }));

        store.unblock_worktree(&worktree).expect("unblock worktree");
        dispatcher
            .register(&SessionId::new("sess-unblocked"), &worktree, None, None)
            .expect("explicit unblock must affect running dispatcher");
        assert_eq!(registry.active_sessions().len(), 1);
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_uses_configured_scan_buffer_service() {
        use anvil_intercept_rules::RuleRegistry;
        use serde_json::json;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixStream;

        let tmp = tempfile::tempdir().unwrap();
        let pid_file = test_pid_file(&tmp);
        let socket = test_ipc_socket(&tmp);
        let scan_buffer = midedit::ScanBufferService::new(enforcement::EnforcementPipeline::new(
            RuleRegistry::new(),
        ));

        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(run_foreground(
            ForegroundOpts::with_pid_file_and_ipc_socket(&pid_file, &socket)
                .with_scan_buffer_service(scan_buffer),
            token,
        ));

        wait_for_pid_file(&pid_file).await;
        wait_for_socket(&socket).await;

        let mut stream = UnixStream::connect(&socket).await.expect("connect");
        let frame = json!({
            "jsonrpc": "2.0",
            "method": "scan_buffer",
            "params": {
                "path": "src/auth/client.ts",
                "text": "const config = { api_key: 'abcdEFGH1234567890' };\n",
                "version": 9,
                "mode": "midEdit"
            },
            "id": "foreground-scan"
        });
        stream
            .write_all(format!("{frame}\n").as_bytes())
            .await
            .expect("write scan");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .expect("scan response timeout")
            .expect("read scan response");
        let response: serde_json::Value = serde_json::from_str(line.trim_end()).expect("json");
        assert_eq!(response["id"], "foreground-scan");
        assert_eq!(response["result"]["diagnostics"], json!([]));

        shutdown.trigger();
        timeout(Duration::from_secs(1), handle)
            .await
            .expect("foreground loop did not return after shutdown")
            .expect("join failure")
            .expect("foreground loop reported error");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_refuses_existing_pid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_file = test_pid_file(&tmp);
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(run_foreground(test_opts(&pid_file), token));

        wait_for_pid_file(&pid_file).await;
        let (_, second_token) = Shutdown::new();
        let err = run_foreground(test_opts(&pid_file), second_token)
            .await
            .expect_err("second foreground daemon should refuse the pid file");
        let message = format!("{err:#}");
        assert!(
            message.contains("already running")
                && message.contains(&pid_file.display().to_string()),
            "single-instance error should name the existing pid file, got: {message}",
        );

        shutdown.trigger();
        timeout(Duration::from_secs(1), handle)
            .await
            .expect("foreground loop did not return after shutdown")
            .expect("join failure")
            .expect("foreground loop reported error");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_creates_missing_pid_parent_as_owner_only() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_dir = tmp.path().join("runtime").join("anvil");
        let pid_file = pid_dir.join("intercept.pid");
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(run_foreground(test_opts(&pid_file), token));

        wait_for_pid_file(&pid_file).await;
        let mode = fs::metadata(&pid_dir)
            .expect("stat pid dir")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);

        shutdown.trigger();
        timeout(Duration::from_secs(1), handle)
            .await
            .expect("foreground loop did not return after shutdown")
            .expect("join failure")
            .expect("foreground loop reported error");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_refuses_insecure_pid_parent_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_dir = tmp.path().join("anvil");
        fs::create_dir(&pid_dir).expect("create pid dir");
        fs::set_permissions(&pid_dir, fs::Permissions::from_mode(0o755))
            .expect("set insecure mode");
        let (_, token) = Shutdown::new();

        let err = run_foreground(test_opts(pid_dir.join("intercept.pid")), token)
            .await
            .expect_err("insecure pid dir should be rejected");
        let message = format!("{err:#}");
        assert!(
            message.contains("expected 700"),
            "error should explain owner-only mode requirement, got: {message}",
        );
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_refuses_symlink_pid_parent() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        fs::create_dir(&target).expect("create symlink target");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).expect("set target mode");
        let link = tmp.path().join("anvil-link");
        symlink(&target, &link).expect("create pid dir symlink");
        let (_, token) = Shutdown::new();

        let err = run_foreground(test_opts(link.join("intercept.pid")), token)
            .await
            .expect_err("symlink pid dir should be rejected");
        let message = format!("{err:#}");
        assert!(
            message.contains("refusing symlink PID file directory"),
            "error should reject pid dir symlink, got: {message}",
        );
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_recovers_stale_pid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_file = test_pid_file(&tmp);
        create_secure_test_pid_dir(pid_file.parent().expect("pid parent"));
        fs::write(&pid_file, "999999999\nstart_time=1\n").expect("write stale pid");
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(run_foreground(test_opts(&pid_file), token));

        wait_for_current_pid_record(&pid_file).await;
        let pid = fs::read_to_string(&pid_file).expect("read pid file");
        assert_eq!(
            pid.lines().next(),
            Some(std::process::id().to_string().as_str())
        );

        shutdown.trigger();
        timeout(Duration::from_secs(1), handle)
            .await
            .expect("foreground loop did not return after shutdown")
            .expect("join failure")
            .expect("foreground loop reported error");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_preserves_unparseable_existing_pid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_file = test_pid_file(&tmp);
        create_secure_test_pid_dir(pid_file.parent().expect("pid parent"));
        fs::write(&pid_file, "not-a-pid\n").expect("write malformed pid");
        let (_, token) = Shutdown::new();

        let err = run_foreground(test_opts(&pid_file), token)
            .await
            .expect_err("malformed pid record should not be deleted as stale");
        let message = format!("{err:#}");
        assert!(
            message.contains("cannot be proven stale"),
            "error should refuse unproven stale records, got: {message}",
        );
        assert_eq!(
            fs::read_to_string(&pid_file).expect("malformed pid file should remain"),
            "not-a-pid\n",
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn run_foreground_does_not_remove_replaced_pid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_file = test_pid_file(&tmp);
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(run_foreground(test_opts(&pid_file), token));

        wait_for_pid_file(&pid_file).await;
        fs::remove_file(&pid_file).expect("remove original pid file");
        fs::write(&pid_file, "replacement\n").expect("write replacement pid file");

        shutdown.trigger();
        timeout(Duration::from_secs(1), handle)
            .await
            .expect("foreground loop did not return after shutdown")
            .expect("join failure")
            .expect("foreground loop reported error");

        assert_eq!(
            fs::read_to_string(&pid_file).expect("replacement pid file should remain"),
            "replacement\n",
        );
    }
}
