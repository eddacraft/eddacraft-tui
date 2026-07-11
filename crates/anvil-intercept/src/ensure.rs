//! DLIFE-002: idempotent, race-safe daemon ensure primitive.
//!
//! [`ensure_daemon`] is the internal entry point that user-facing CLI surfaces
//! (`anvil start`, `anvil watch` — wired in DLIFE-003/-004) call to bring up the
//! per-user save-time daemon. It reuses a live daemon, never double-starts under
//! concurrency, and degrades honestly when it must not (or cannot) start one.
//!
//! Design (ADR-082, the accepted tiered startup posture):
//!
//! 1. **probe** the per-user save-time endpoint for a live status answer → if a
//!    daemon answers (or a listener is present but slow), report [`Reused`];
//! 2. otherwise, if the caller is not allowed to spawn, report a typed
//!    [`NoStart`] — `start`/`watch` render a platform-specific advisory distinct
//!    from a deliberate opt-out;
//! 3. otherwise acquire a same-user advisory lock around the spawn critical
//!    section so concurrent `start`/`watch` callers serialise, **re-probe under
//!    the lock** (a racing caller may have started one → [`Reused`]), then
//! 4. spawn a detached background daemon with stdout/stderr redirected to a log
//!    file, and **bound-wait** — to a named timeout — for it to bind and answer
//!    the status verb → [`Started`]; a spawn that never binds → [`Failed`].
//!
//! The sharpest correctness edge is **stale detection**: a *dead* endpoint
//! (connect fails fast — absent or stale socket) is the only case that triggers a
//! respawn; a *live-but-slow* endpoint (connect succeeds but no status answer
//! within the probe timeout) is never torn down, so a daemon under graph/GC load
//! is not ripped out from under its own listener. The spawned daemon's own
//! [`crate::ipc::IpcListener`] bind unlinks any stale socket it owns, so this
//! primitive never unlinks an endpoint itself.
//!
//! Unix-first landing (DLIFE-002); Windows background launch followed in
//! CIB-072 once the named-pipe IPC and save-time verb surface were proven
//! (DSV-010b). Platforms without a detached launcher still return
//! [`NoStartReason::PlatformUnsupported`] deterministically.
//!
//! [`Reused`]: EnsureOutcome::Reused
//! [`Started`]: EnsureOutcome::Started
//! [`NoStart`]: EnsureOutcome::NoStart
//! [`Failed`]: EnsureOutcome::Failed

use std::io;
use std::path::Path;
#[cfg(any(unix, windows))]
use std::path::PathBuf;
#[cfg(any(unix, windows))]
use std::time::{Duration, Instant};

/// Per-request wall-clock budget for the status probe. A listener that accepts
/// the connection but does not answer within this window is treated as
/// *live-but-slow* (never torn down), matching the save-time client budget.
#[cfg(any(unix, windows))]
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// How long to wait for a freshly-spawned daemon to bind its endpoint and answer
/// the status verb before declaring the launch [`EnsureOutcome::Failed`].
#[cfg(any(unix, windows))]
const DAEMON_BIND_TIMEOUT: Duration = Duration::from_secs(10);

/// Poll cadence while bound-waiting for a spawned daemon to come up.
#[cfg(any(unix, windows))]
const BIND_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Why a caller must not spawn a daemon, even though no live one was found.
///
/// Kept as a typed enum so `start`/`watch` can render a platform-specific
/// advisory distinct from a deliberate opt-out — a Windows user must not see the
/// opt-out hint, and a CI run must not be told it opted out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoStartReason {
    /// Deliberate opt-out (`--no-daemon` / `ANVIL_WATCH_DAEMON=0`).
    OptOut,
    /// No consent surface to start in: headless / `--json` / CI / MCP / hook /
    /// `--verify`. Never spawns or prompts.
    NonInteractive,
    /// Background launch is not yet implemented for this platform.
    PlatformUnsupported,
}

impl NoStartReason {
    /// A stable, lower-case discriminator suitable for `--json` output and
    /// telemetry enums.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            NoStartReason::OptOut => "opt-out",
            NoStartReason::NonInteractive => "non-interactive",
            NoStartReason::PlatformUnsupported => "platform-unsupported",
        }
    }
}

/// Whether the calling surface is allowed to launch a daemon.
///
/// The capability is decided by the caller (TTY / flag / platform), not sniffed
/// by the primitive, so `ensure_daemon` stays deterministic and unit-testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartCapability {
    /// The caller has a consent surface and may spawn a background daemon.
    MaySpawn,
    /// The caller decided up front not to spawn; carries the reason to render.
    NoSpawn(NoStartReason),
}

/// The typed result of an [`ensure_daemon`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureOutcome {
    /// A live daemon already answers the per-user endpoint (or a listener is
    /// present but slow and must not be torn down).
    Reused,
    /// Exactly one daemon was launched and now answers the status verb.
    Started,
    /// No daemon was started, by design. The caller renders `reason`.
    NoStart {
        /// Why no daemon was started.
        reason: NoStartReason,
    },
    /// Launch or bind failed. `recovery` is an actionable hint that names the log
    /// path so the operator can inspect why the daemon did not come up.
    Failed {
        /// Actionable recovery hint (names the daemon log path).
        recovery: String,
    },
}

/// The liveness of the per-user daemon endpoint as seen by a single probe.
#[cfg(any(unix, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Liveness {
    /// Connected and received a valid status answer — a healthy daemon.
    Answered,
    /// Connected, but no valid answer within the probe budget. A listener is
    /// present (a daemon process exists, possibly under load); it must **not** be
    /// unlinked or duplicated.
    ConnectedNoAnswer,
    /// Connect failed fast — the endpoint is absent or a stale socket with no
    /// listener. Safe to (re)spawn; the new daemon's bind cleans up any stale
    /// socket file it owns.
    Unreachable,
}

/// Reads the liveness of the per-user daemon endpoint. Abstracted so the ensure
/// state machine is tested without real sockets/pipes. Internal — callers consume
/// the typed [`EnsureOutcome`], not the probe.
#[cfg(any(unix, windows))]
pub(crate) trait DaemonProbe {
    /// Perform one liveness probe of the endpoint.
    fn probe(&self) -> Liveness;
}

/// Launches a detached background daemon. Abstracted so the ensure state machine
/// is tested without spawning real processes.
pub trait DaemonLauncher {
    /// Spawn a detached background daemon, redirecting its stdout/stderr to
    /// `log_path`. Returns the spawned child's PID once the child is spawned —
    /// **not** once it has bound; the caller bound-waits via the probe. The
    /// ensure state machine ignores the PID; the save-time driver supervisor
    /// (DSV-047) records it for later termination and liveness reporting.
    fn spawn_detached(&self, log_path: &Path) -> io::Result<u32>;
}

/// The deterministic outcome of `ensure_daemon` on platforms without background
/// launch support. Exposed so the documented platform split is asserted on every
/// platform's test run, not only the Windows CI leg.
#[must_use]
pub fn platform_unsupported_outcome() -> EnsureOutcome {
    EnsureOutcome::NoStart {
        reason: NoStartReason::PlatformUnsupported,
    }
}

/// Bring up the per-user save-time daemon, honouring the caller's `capability`.
/// See the module docs for the full state machine.
///
/// The daemon is per-user and serves every worktree, so bring-up takes no
/// workspace argument: liveness is the workspace-independent `anvil/status/query`
/// verb, not a per-workspace admission check (a daemon that is up but has not yet
/// admitted the caller's worktree is still a live daemon to reuse).
///
/// `launcher` is how a detached daemon is spawned; the CLI passes a
/// [`DetachedCommandLauncher`] built from `current_exe()` and
/// `intercept start --foreground`.
#[cfg(unix)]
pub fn ensure_daemon(capability: StartCapability, launcher: &dyn DaemonLauncher) -> EnsureOutcome {
    let Ok(socket_path) = crate::ipc::resolve_socket_path() else {
        return EnsureOutcome::Failed {
            recovery: "could not resolve the per-user daemon socket path; \
                       check $XDG_RUNTIME_DIR / $HOME or set ANVIL_HOME"
                .to_owned(),
        };
    };
    let Ok(pid_path) = crate::default_pid_file_path() else {
        return EnsureOutcome::Failed {
            recovery: "could not resolve the per-user runtime directory; \
                       check $XDG_RUNTIME_DIR / $HOME or set ANVIL_HOME"
                .to_owned(),
        };
    };
    let runtime_dir = pid_path.parent().unwrap_or_else(|| Path::new("."));
    // #3220: pre-flight the runtime/socket directory before spawn so a
    // wrong-mode `ANVIL_HOME` surfaces as a chmod recovery instead of a
    // false "daemon did not become ready" + intercept-start hint after
    // the bind timeout. Owner-matched loose modes are tightened inside
    // `ensure_secure_runtime_dir`; remaining failures keep the path and
    // `chmod 700` instruction in the recovery string.
    if let Err(err) = crate::ensure_secure_runtime_dir(runtime_dir) {
        return EnsureOutcome::Failed {
            recovery: format!(
                "runtime directory is not usable ({err}). \
                 When ANVIL_HOME re-roots the daemon, the prefix must be mode 0700 \
                 and owned by you — run: chmod 700 '{}'",
                runtime_dir.display()
            ),
        };
    }
    // Both the lock and the log live beside the PID file, so they inherit its
    // per-`ANVIL_HOME` scoping (ADR-060): two re-rooted instances of the same
    // user do not share a lock.
    let lock_path = runtime_dir.join("intercept.ensure.lock");
    let log_path = runtime_dir.join("intercept.daemon.log");

    let probe = SocketProbe::new(socket_path);
    let params = EnsureParams {
        probe: &probe,
        launcher,
        lock_path: &lock_path,
        log_path: &log_path,
        bind_timeout: DAEMON_BIND_TIMEOUT,
        poll_interval: BIND_POLL_INTERVAL,
    };
    ensure_with(&params, capability)
}

/// Windows entry: same state machine as Unix, probing the per-user named pipe
/// instead of the Unix socket (CIB-072 / GH #2609).
#[cfg(windows)]
pub fn ensure_daemon(capability: StartCapability, launcher: &dyn DaemonLauncher) -> EnsureOutcome {
    let Ok(pipe_name) = crate::ipc::resolve_pipe_name() else {
        return EnsureOutcome::Failed {
            recovery: "could not resolve the per-user intercept daemon pipe name; \
                       check that the current user SID is readable"
                .to_owned(),
        };
    };
    let Ok(pid_path) = crate::default_pid_file_path() else {
        return EnsureOutcome::Failed {
            recovery: "could not resolve the per-user runtime directory; \
                       check %LOCALAPPDATA% / %USERPROFILE% or set ANVIL_HOME"
                .to_owned(),
        };
    };
    let runtime_dir = pid_path.parent().unwrap_or_else(|| Path::new("."));
    let lock_path = runtime_dir.join("intercept.ensure.lock");
    let log_path = runtime_dir.join("intercept.daemon.log");

    let probe = PipeProbe::new(pipe_name);
    let params = EnsureParams {
        probe: &probe,
        launcher,
        lock_path: &lock_path,
        log_path: &log_path,
        bind_timeout: DAEMON_BIND_TIMEOUT,
        poll_interval: BIND_POLL_INTERVAL,
    };
    ensure_with(&params, capability)
}

/// Platforms without a detached launcher implementation.
#[cfg(all(not(unix), not(windows)))]
pub fn ensure_daemon(
    _capability: StartCapability,
    _launcher: &dyn DaemonLauncher,
) -> EnsureOutcome {
    platform_unsupported_outcome()
}

/// Inputs to the platform-agnostic ensure state machine.
#[cfg(any(unix, windows))]
struct EnsureParams<'a> {
    probe: &'a dyn DaemonProbe,
    launcher: &'a dyn DaemonLauncher,
    lock_path: &'a Path,
    log_path: &'a Path,
    bind_timeout: Duration,
    poll_interval: Duration,
}

/// The platform-agnostic ensure state machine. Pure but for the lock file, the
/// injected probe, and the injected launcher — so every branch is unit-tested.
#[cfg(any(unix, windows))]
fn ensure_with(params: &EnsureParams<'_>, capability: StartCapability) -> EnsureOutcome {
    // 1. Probe is read-only and always allowed, even for non-spawning callers:
    //    a live daemon is reused regardless of capability.
    if reuse_if_live(params.probe) {
        return EnsureOutcome::Reused;
    }

    // 2. No live daemon. Only callers with a consent surface may spawn.
    if let StartCapability::NoSpawn(reason) = capability {
        return EnsureOutcome::NoStart { reason };
    }

    // 3. Serialise the spawn critical section across concurrent start/watch
    //    callers on the same per-`ANVIL_HOME` lock.
    let _lock = match acquire_ensure_lock(params.lock_path) {
        Ok(lock) => lock,
        Err(err) => {
            return EnsureOutcome::Failed {
                recovery: format!(
                    "could not acquire the daemon-start lock at {}: {err}",
                    params.lock_path.display()
                ),
            };
        }
    };

    // 4. Re-probe under the lock: a racing caller may have started one while we
    //    waited for the lock.
    if reuse_if_live(params.probe) {
        return EnsureOutcome::Reused;
    }

    // 5. Spawn the detached daemon. Its own IpcListener bind unlinks any stale
    //    socket it owns, so we never unlink an endpoint here.
    if let Err(err) = params.launcher.spawn_detached(params.log_path) {
        return EnsureOutcome::Failed {
            recovery: format!(
                "failed to launch the background daemon: {err}. \
                 See the daemon log at {} or run `anvil intercept start --foreground`.",
                params.log_path.display()
            ),
        };
    }

    // 6. Bound-wait for the new daemon to bind and answer.
    if wait_until_answered(params.probe, params.bind_timeout, params.poll_interval) {
        EnsureOutcome::Started
    } else {
        EnsureOutcome::Failed {
            recovery: format!(
                "the daemon did not become ready within {}s. \
                 See the daemon log at {} or run `anvil intercept start --foreground`.",
                // Print the effective wall-clock ceiling: an in-flight probe can
                // overrun `bind_timeout` by one `PROBE_TIMEOUT` (see
                // `wait_until_answered` — the overrun is intentional), so the
                // real bound is `bind_timeout + PROBE_TIMEOUT`, not `bind_timeout`
                // alone (CIB-174).
                (params.bind_timeout + PROBE_TIMEOUT).as_secs(),
                params.log_path.display()
            ),
        }
    }
}

/// `true` when a probe shows a daemon endpoint we must reuse rather than spawn
/// over: either a healthy answer or a present-but-slow listener.
#[cfg(any(unix, windows))]
fn reuse_if_live(probe: &dyn DaemonProbe) -> bool {
    matches!(
        probe.probe(),
        Liveness::Answered | Liveness::ConnectedNoAnswer
    )
}

/// Poll the probe until a daemon answers or the deadline passes. A
/// `ConnectedNoAnswer` during start-up (the daemon has bound but not finished
/// loading) is treated as still-coming-up and keeps polling.
///
/// The deadline is checked *before* each probe so we never start a fresh probe
/// once the budget is spent; a probe already in flight when the deadline passes
/// can still overrun by at most one `PROBE_TIMEOUT` (the in-flight socket read),
/// so the effective wall-clock ceiling is `timeout + PROBE_TIMEOUT`.
#[cfg(any(unix, windows))]
fn wait_until_answered(probe: &dyn DaemonProbe, timeout: Duration, interval: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            return false;
        }
        if matches!(probe.probe(), Liveness::Answered) {
            return true;
        }
        std::thread::sleep(interval);
    }
}

/// Acquire the same-user advisory lock around the spawn critical section. Mirrors
/// the daemon's own PID-file lock pattern (`lib.rs`), but on a distinct
/// `intercept.ensure.lock` file so it never contends with the daemon it is about
/// to spawn. Blocks until acquired; released when the returned guard drops.
///
/// The lock file is opened with the default close-on-exec flag, so a detached
/// daemon child spawned while the lock is held never inherits (and therefore
/// never wedges) it.
#[cfg(any(unix, windows))]
fn acquire_ensure_lock(lock_path: &Path) -> io::Result<std::fs::File> {
    use std::fs::OpenOptions;

    if let Some(parent) = lock_path.parent() {
        crate::ensure_secure_runtime_dir(parent)
            .map_err(|err| io::Error::other(format!("{err:#}")))?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)?;
    file.lock()?;
    Ok(file)
}

// ---------------------------------------------------------------------------
// Real Unix probe + launcher
// ---------------------------------------------------------------------------

/// Probes the per-user Unix save-time socket with a workspace-independent
/// `anvil/status/query` round-trip, distinguishing an absent/stale endpoint
/// (connect fails) from a present listener (connect succeeds), per the
/// stale-detection contract.
///
/// Liveness deliberately uses the workspace-independent status verb, not
/// `anvil/workspace_status`: the latter would return a JSON-RPC error for a
/// daemon that is up but has not admitted this worktree, which the probe would
/// misread as "present but not answering" and the bound-wait would never accept
/// as `Answered`.
#[cfg(unix)]
pub(crate) struct SocketProbe {
    socket_path: PathBuf,
    timeout: Duration,
}

#[cfg(unix)]
impl SocketProbe {
    /// Probe `socket_path` for a live, answering daemon.
    #[must_use]
    pub(crate) fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            timeout: PROBE_TIMEOUT,
        }
    }

    /// Probe with an explicit per-request timeout (tests use a short budget to
    /// exercise the `ConnectedNoAnswer` path quickly).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_timeout(socket_path: PathBuf, timeout: Duration) -> Self {
        Self {
            socket_path,
            timeout,
        }
    }
}

#[cfg(unix)]
impl DaemonProbe for SocketProbe {
    fn probe(&self) -> Liveness {
        use std::os::unix::net::UnixStream;

        // Connect failure is the *only* signal that the endpoint is absent or
        // stale (no listener) — the case it is safe to respawn over.
        let Ok(stream) = UnixStream::connect(&self.socket_path) else {
            return Liveness::Unreachable;
        };

        // A listener accepted us: a daemon process exists. From here on, any
        // failure is `ConnectedNoAnswer` (present-but-unusable), never
        // `Unreachable`, so a live-but-slow daemon is never torn down. The
        // per-user runtime dir is 0700, so a foreign peer should not occur; if
        // one somehow did we conservatively treat it as present rather than
        // claim it as ours or unlink it.
        if crate::ipc::validate_connected_peer_for_client(&stream).is_err() {
            return Liveness::ConnectedNoAnswer;
        }
        if stream.set_read_timeout(Some(self.timeout)).is_err()
            || stream.set_write_timeout(Some(self.timeout)).is_err()
        {
            return Liveness::ConnectedNoAnswer;
        }

        match status_query_round_trip(&stream) {
            Ok(()) => Liveness::Answered,
            Err(()) => Liveness::ConnectedNoAnswer,
        }
    }
}

/// Send one NDJSON JSON-RPC `anvil/status/query` request and confirm a
/// well-formed, id-matched `result` came back. A valid result means the daemon
/// is up and answering — this is a workspace-independent liveness check, so a
/// daemon that has not yet admitted any particular worktree still answers.
///
/// The NDJSON framing intentionally mirrors the save-time client wire
/// (`anvil-cli`'s `watch_save_time::framing::round_trip_over` and the status
/// frame in `commands::intercept::build_query_status_frame_bytes`). It is
/// duplicated here, not shared, because `anvil-cli` depends on `anvil-intercept`
/// (not the reverse) so the framing helper cannot be imported upward; a change
/// to the wire must update both. Errors collapse to `Err(())` — "connected but
/// did not answer".
#[cfg(unix)]
fn status_query_round_trip(stream: &std::os::unix::net::UnixStream) -> Result<(), ()> {
    use std::io::{BufRead, BufReader, Read, Write};

    use anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY;

    /// Cap the single NDJSON response line so a buggy/hostile daemon cannot make
    /// the probe buffer unboundedly. Matches the save-time client cap.
    const RESPONSE_LINE_BYTES: u64 = 1 << 20;
    const PROBE_ID: &str = "anvil-ensure-probe";

    // `anvil/status/query` takes no params (matches
    // `build_query_status_frame_bytes`).
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": ANVIL_STATUS_QUERY,
        "id": PROBE_ID,
    });

    let mut writer = stream.try_clone().map_err(|_| ())?;
    writeln!(writer, "{frame}").map_err(|_| ())?;
    writer.flush().map_err(|_| ())?;

    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_BYTES + 1)
        .read_until(b'\n', &mut buf)
        .map_err(|_| ())?;
    if read == 0 || buf.len() as u64 > RESPONSE_LINE_BYTES || !buf.ends_with(b"\n") {
        return Err(());
    }
    let line = String::from_utf8(buf).map_err(|_| ())?;
    let envelope: serde_json::Value = serde_json::from_str(&line).map_err(|_| ())?;
    if envelope.get("id").and_then(serde_json::Value::as_str) != Some(PROBE_ID) {
        return Err(());
    }
    // A JSON-RPC error (no `result`) means the daemon cannot serve the verb; for
    // a liveness probe that still counts as "present but not answering", not a
    // healthy answer.
    if envelope.get("result").is_none() {
        return Err(());
    }
    Ok(())
}

/// Spawns the daemon as a detached background child by re-executing the anvil
/// binary (the CLI builds this from `current_exe()` +
/// `intercept start --foreground`), with stdout/stderr redirected to the daemon
/// log and its own process group (not a new session — the crate forbids
/// `unsafe_code`, so `setsid` is unavailable) so a parent Ctrl-C delivered to the
/// terminal's foreground process group never reaches it.
#[cfg(unix)]
pub struct DetachedCommandLauncher {
    program: PathBuf,
    args: Vec<std::ffi::OsString>,
    envs: Vec<(std::ffi::OsString, std::ffi::OsString)>,
}

#[cfg(unix)]
impl DetachedCommandLauncher {
    /// Build a launcher that runs `program` with `args` detached.
    #[must_use]
    pub fn new(program: PathBuf, args: Vec<std::ffi::OsString>) -> Self {
        Self {
            program,
            args,
            envs: Vec::new(),
        }
    }

    /// Add an environment variable set on the spawned child (on top of the
    /// inherited environment). The save-time driver supervisor (DSV-047) hands
    /// the findings-log path to its child this way.
    #[must_use]
    pub fn with_env(
        mut self,
        key: impl Into<std::ffi::OsString>,
        value: impl Into<std::ffi::OsString>,
    ) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }
}

#[cfg(unix)]
impl DaemonLauncher for DetachedCommandLauncher {
    fn spawn_detached(&self, log_path: &Path) -> io::Result<u32> {
        use std::fs::OpenOptions;
        use std::os::unix::fs::OpenOptionsExt;
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};

        if let Some(parent) = log_path.parent() {
            crate::ensure_secure_runtime_dir(parent)
                .map_err(|err| io::Error::other(format!("{err:#}")))?;
        }
        // Rotate the previous run's log out of the way before a fresh spawn so the
        // file does not grow without bound across daemon restarts. A single
        // generation (`<log>.1`) is kept — the daemon is a singleton, so a respawn
        // only happens after the previous instance exited and its log was
        // available to inspect. (Within-lifetime rotation on a size cap is a
        // daemon-side concern, tracked separately.) `append` is retained so the
        // shared stdout/stderr descriptors interleave correctly.
        if log_path.exists() {
            let mut rotated = log_path.as_os_str().to_owned();
            rotated.push(".1");
            let _ = std::fs::rename(log_path, PathBuf::from(rotated));
        }
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(log_path)?;
        let log_err = log.try_clone()?;

        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args)
            .envs(self.envs.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err))
            // Put the daemon in its own process group so a SIGINT (Ctrl-C)
            // delivered to the parent's foreground process group never reaches
            // it. `process_group` is the safe detachment primitive — the crate
            // forbids `unsafe_code`, so `pre_exec`/`setsid` is unavailable; a
            // distinct process group still shields the daemon from
            // terminal-generated signals while keeping the launch allocation-
            // and fork-handler-free. The lock/log descriptors are close-on-exec
            // by default, so the child never inherits (and cannot wedge) them.
            .process_group(0);
        // Detached: deliberately drop the Child handle without waiting. The
        // parent bound-waits via the probe, not via the child; the daemon
        // outlives a short-lived `start` and reparents to init on parent exit.
        let child = cmd.spawn()?;
        Ok(child.id())
    }
}

// ---------------------------------------------------------------------------
// Real Windows probe + launcher (CIB-072 / GH #2609)
// ---------------------------------------------------------------------------

/// Probes the per-user Windows named pipe with a workspace-independent
/// `anvil/status/query` round-trip, mirroring [`SocketProbe`]'s stale-detection
/// contract on the Unix socket path.
///
/// Windows-only integration coverage lives in
/// `crates/anvil-cli/src/activation/daemon_evidence.rs`
/// (`end_to_end_against_real_named_pipe_promotes_to_live_validation`).
#[cfg(windows)]
pub(crate) struct PipeProbe {
    pipe_name: String,
    timeout: Duration,
}

#[cfg(windows)]
impl PipeProbe {
    #[must_use]
    pub(crate) fn new(pipe_name: String) -> Self {
        Self {
            pipe_name,
            timeout: PROBE_TIMEOUT,
        }
    }

    #[allow(dead_code)]
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_timeout(pipe_name: String, timeout: Duration) -> Self {
        Self { pipe_name, timeout }
    }
}

#[cfg(windows)]
impl DaemonProbe for PipeProbe {
    fn probe(&self) -> Liveness {
        use std::sync::mpsc;
        use std::thread;
        use std::time::Instant;

        // Connect failure is the *only* signal that the endpoint is absent or
        // stale (no listener) — the case it is safe to respawn over. Mirrors
        // [`SocketProbe`] and the CLI's `query_daemon_status_windows_at_with_timeout`
        // connect/read timeout split (CIB-072 / Copilot review #1840).
        let deadline_started = Instant::now();
        let connect_timeout = self.timeout;

        let pipe_name = self.pipe_name.clone();
        let (connect_tx, connect_rx) = mpsc::sync_channel::<std::io::Result<_>>(1);
        let connect_thread = thread::spawn(move || {
            let _ = connect_tx.send(anvil_intercept_win32::connect_owner_only_pipe_client(
                &pipe_name,
            ));
        });
        let connect_outcome = match connect_rx.recv_timeout(connect_timeout) {
            Ok(outcome) => outcome,
            Err(mpsc::RecvTimeoutError::Timeout) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                // The connect worker may still be blocked in WaitNamedPipe; dropping
                // the JoinHandle detaches it so the probe caller does not wedge
                // (mirrors the CLI's single-shot exit semantics).
                drop(connect_thread);
                // A hung or busy pipe server is present-but-unusable, not absent.
                return Liveness::ConnectedNoAnswer;
            }
        };
        let _ = connect_thread.join();

        let client = match connect_outcome {
            Ok(client) => client,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Liveness::Unreachable;
            }
            // ERROR_PIPE_BUSY — every server instance is talking to another client.
            Err(err) if err.raw_os_error() == Some(231) => {
                return Liveness::ConnectedNoAnswer;
            }
            Err(_) => return Liveness::ConnectedNoAnswer,
        };

        // A listener accepted us: from here on, any failure is
        // `ConnectedNoAnswer` (present-but-unusable), never `Unreachable`.
        let request_timeout = self.timeout.saturating_sub(deadline_started.elapsed());
        match pipe_status_query_round_trip(client, request_timeout) {
            Ok(()) => Liveness::Answered,
            Err(()) => Liveness::ConnectedNoAnswer,
        }
    }
}

/// Send one NDJSON `anvil/status/query` request over a connected pipe client
/// and confirm a well-formed, id-matched `result` came back. Mirrors the Unix
/// [`status_query_round_trip`] helper — duplicated here because
/// `anvil-intercept` cannot depend upward on `anvil-cli`.
///
/// `timeout` is the remaining wall-clock budget for write + read (connect is
/// handled by the caller). Synchronous `ReadFile` has no native timeout, so the
/// read runs on a worker thread with `recv_timeout`, matching the CLI client.
#[cfg(windows)]
fn pipe_status_query_round_trip(
    mut client: anvil_intercept_win32::OwnerOnlyPipeClient,
    timeout: Duration,
) -> Result<(), ()> {
    use std::sync::mpsc;
    use std::thread;

    use anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY;

    const RESPONSE_LINE_BYTES: u64 = 1 << 20;
    const PROBE_ID: &str = "anvil-ensure-probe";

    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": ANVIL_STATUS_QUERY,
        "id": PROBE_ID,
    });
    let mut payload = serde_json::to_vec(&frame).map_err(|_| ())?;
    payload.push(b'\n');
    client.write_all(&payload).map_err(|_| ())?;

    let (read_tx, read_rx) = mpsc::sync_channel::<Result<Vec<u8>, ()>>(1);
    let read_thread = thread::spawn(move || {
        let mut buf: Vec<u8> = Vec::with_capacity(4096);
        let mut chunk = [0_u8; 4096];
        // Scan cursor: only search bytes appended this iteration (Copilot #1848).
        let mut scan_from = 0_usize;
        let outcome = loop {
            let n = match client.read(&mut chunk) {
                Ok(n) => n,
                Err(_) => break Err(()),
            };
            if n == 0 {
                break Err(());
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Some(rel_idx) = buf[scan_from..].iter().position(|b| *b == b'\n') {
                let newline_idx = scan_from + rel_idx;
                buf.truncate(newline_idx + 1);
                break Ok(buf);
            }
            scan_from = buf.len();
            if (buf.len() as u64) > RESPONSE_LINE_BYTES {
                break Err(());
            }
        };
        let _ = read_tx.send(outcome);
    });

    let buf = match read_rx.recv_timeout(timeout) {
        Ok(Ok(buf)) => buf,
        Ok(Err(())) => {
            let _ = read_thread.join();
            return Err(());
        }
        Err(_) => {
            // ReadFile has no native timeout; the worker may stay blocked until
            // the daemon responds. Dropping the JoinHandle detaches it.
            drop(read_thread);
            return Err(());
        }
    };
    let _ = read_thread.join();

    let line = String::from_utf8(buf).map_err(|_| ())?;
    let envelope: serde_json::Value = serde_json::from_str(&line).map_err(|_| ())?;
    if envelope.get("id").and_then(serde_json::Value::as_str) != Some(PROBE_ID) {
        return Err(());
    }
    if envelope.get("result").is_none() {
        return Err(());
    }
    Ok(())
}

/// Spawns the daemon as a detached background child on Windows (`CREATE_NO_WINDOW`),
/// redirecting stdout/stderr to the daemon log beside the PID file.
#[cfg(windows)]
pub struct DetachedCommandLauncher {
    program: PathBuf,
    args: Vec<std::ffi::OsString>,
    envs: Vec<(std::ffi::OsString, std::ffi::OsString)>,
}

#[cfg(windows)]
impl DetachedCommandLauncher {
    #[must_use]
    pub fn new(program: PathBuf, args: Vec<std::ffi::OsString>) -> Self {
        Self {
            program,
            args,
            envs: Vec::new(),
        }
    }

    /// Add an environment variable set on the spawned child (on top of the
    /// inherited environment). The save-time driver supervisor (DSV-047) hands
    /// the findings-log path to its child this way.
    #[must_use]
    pub fn with_env(
        mut self,
        key: impl Into<std::ffi::OsString>,
        value: impl Into<std::ffi::OsString>,
    ) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }
}

#[cfg(windows)]
impl DaemonLauncher for DetachedCommandLauncher {
    fn spawn_detached(&self, log_path: &Path) -> io::Result<u32> {
        use std::fs::OpenOptions;
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        if let Some(parent) = log_path.parent() {
            crate::ensure_secure_runtime_dir(parent)
                .map_err(|err| io::Error::other(format!("{err:#}")))?;
        }
        if log_path.exists() {
            let mut rotated = log_path.as_os_str().to_owned();
            rotated.push(".1");
            // Windows `rename` fails over an existing destination (unlike
            // POSIX), so drop the prior generation first; a failed rotation
            // must not block the spawn (`append` keeps the log usable).
            let rotated = PathBuf::from(rotated);
            let _ = std::fs::remove_file(&rotated);
            let _ = std::fs::rename(log_path, rotated);
        }
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        let log_err = log.try_clone()?;

        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args)
            .envs(self.envs.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err))
            .creation_flags(CREATE_NO_WINDOW);
        let child = cmd.spawn()?;
        Ok(child.id())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::*;

    /// A probe whose liveness is read from a shared flag, flipped by the fake
    /// launcher to model "absent until spawned, then answering".
    struct FlagProbe {
        ready: Arc<AtomicBool>,
        connected_no_answer: bool,
    }

    impl FlagProbe {
        fn absent() -> (Self, Arc<AtomicBool>) {
            let ready = Arc::new(AtomicBool::new(false));
            (
                Self {
                    ready: Arc::clone(&ready),
                    connected_no_answer: false,
                },
                ready,
            )
        }
    }

    impl DaemonProbe for FlagProbe {
        fn probe(&self) -> Liveness {
            if self.ready.load(Ordering::SeqCst) {
                Liveness::Answered
            } else if self.connected_no_answer {
                Liveness::ConnectedNoAnswer
            } else {
                Liveness::Unreachable
            }
        }
    }

    /// Models a daemon that binds (accepts connections) but never answers the
    /// status verb: `Unreachable` until the `bound` flag flips, then
    /// `ConnectedNoAnswer` forever — never `Answered`.
    struct BindsButSilentProbe {
        bound: Arc<AtomicBool>,
    }

    impl DaemonProbe for BindsButSilentProbe {
        fn probe(&self) -> Liveness {
            if self.bound.load(Ordering::SeqCst) {
                Liveness::ConnectedNoAnswer
            } else {
                Liveness::Unreachable
            }
        }
    }

    /// A launcher that counts spawns and optionally flips a readiness flag (after
    /// an optional delay, to model bind latency) or fails outright.
    struct FakeLauncher {
        count: Arc<AtomicUsize>,
        flips: Option<Arc<AtomicBool>>,
        delay: Duration,
        fail: bool,
    }

    impl FakeLauncher {
        fn that_starts(flips: &Arc<AtomicBool>) -> Self {
            Self {
                count: Arc::new(AtomicUsize::new(0)),
                flips: Some(Arc::clone(flips)),
                delay: Duration::ZERO,
                fail: false,
            }
        }
        fn never_binds() -> Self {
            Self {
                count: Arc::new(AtomicUsize::new(0)),
                flips: None,
                delay: Duration::ZERO,
                fail: false,
            }
        }
        fn failing() -> Self {
            Self {
                count: Arc::new(AtomicUsize::new(0)),
                flips: None,
                delay: Duration::ZERO,
                fail: true,
            }
        }
        fn spawns(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }
    }

    impl DaemonLauncher for FakeLauncher {
        fn spawn_detached(&self, _log_path: &Path) -> io::Result<u32> {
            if self.fail {
                return Err(io::Error::other("boom"));
            }
            self.count.fetch_add(1, Ordering::SeqCst);
            if let Some(flips) = &self.flips {
                let flips = Arc::clone(flips);
                let delay = self.delay;
                std::thread::spawn(move || {
                    if !delay.is_zero() {
                        std::thread::sleep(delay);
                    }
                    flips.store(true, Ordering::SeqCst);
                });
            }
            Ok(1)
        }
    }

    fn params<'a>(
        probe: &'a dyn DaemonProbe,
        launcher: &'a dyn DaemonLauncher,
        lock_path: &'a Path,
        log_path: &'a Path,
    ) -> EnsureParams<'a> {
        EnsureParams {
            probe,
            launcher,
            lock_path,
            log_path,
            bind_timeout: Duration::from_secs(5),
            poll_interval: Duration::from_millis(5),
        }
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        lock: PathBuf,
        log: PathBuf,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir");
        // Use a not-yet-existing subdir so `ensure_secure_runtime_dir` creates it
        // 0700 (the tempdir root itself is 0755 and would be rejected — exactly as
        // production rejects a world-readable runtime dir).
        let rt = dir.path().join("rt");
        let lock = rt.join("intercept.ensure.lock");
        let log = rt.join("intercept.daemon.log");
        Fixture {
            _dir: dir,
            lock,
            log,
        }
    }

    #[test]
    fn live_daemon_is_reused_without_spawning() {
        let fx = fixture();
        let ready = Arc::new(AtomicBool::new(true));
        let probe = FlagProbe {
            ready,
            connected_no_answer: false,
        };
        let launcher = FakeLauncher::never_binds();
        let p = params(&probe, &launcher, &fx.lock, &fx.log);

        assert_eq!(
            ensure_with(&p, StartCapability::MaySpawn),
            EnsureOutcome::Reused
        );
        assert_eq!(launcher.spawns(), 0, "must not spawn over a live daemon");
    }

    #[test]
    fn connected_but_slow_endpoint_is_reused_and_not_torn_down() {
        // The live-but-slow guarantee: a listener that connects but does not
        // answer is reused, never spawned over (and so never unlinked).
        let fx = fixture();
        let probe = FlagProbe {
            ready: Arc::new(AtomicBool::new(false)),
            connected_no_answer: true,
        };
        let launcher = FakeLauncher::never_binds();
        let p = params(&probe, &launcher, &fx.lock, &fx.log);

        assert_eq!(
            ensure_with(&p, StartCapability::MaySpawn),
            EnsureOutcome::Reused
        );
        assert_eq!(launcher.spawns(), 0, "live-but-slow must not be respawned");
    }

    #[test]
    fn absent_daemon_is_started() {
        let fx = fixture();
        let (probe, ready) = FlagProbe::absent();
        let launcher = FakeLauncher::that_starts(&ready);
        let p = params(&probe, &launcher, &fx.lock, &fx.log);

        assert_eq!(
            ensure_with(&p, StartCapability::MaySpawn),
            EnsureOutcome::Started
        );
        assert_eq!(launcher.spawns(), 1, "exactly one daemon launched");
    }

    #[test]
    fn opt_out_caller_never_spawns() {
        let fx = fixture();
        let (probe, _ready) = FlagProbe::absent();
        let launcher = FakeLauncher::never_binds();
        let p = params(&probe, &launcher, &fx.lock, &fx.log);

        assert_eq!(
            ensure_with(&p, StartCapability::NoSpawn(NoStartReason::OptOut)),
            EnsureOutcome::NoStart {
                reason: NoStartReason::OptOut
            }
        );
        assert_eq!(launcher.spawns(), 0);
    }

    #[test]
    fn non_interactive_caller_returns_distinct_reason() {
        let fx = fixture();
        let (probe, _ready) = FlagProbe::absent();
        let launcher = FakeLauncher::never_binds();
        let p = params(&probe, &launcher, &fx.lock, &fx.log);

        assert_eq!(
            ensure_with(&p, StartCapability::NoSpawn(NoStartReason::NonInteractive)),
            EnsureOutcome::NoStart {
                reason: NoStartReason::NonInteractive
            }
        );
    }

    #[test]
    fn spawn_that_never_binds_fails_naming_the_log() {
        let fx = fixture();
        let (probe, _ready) = FlagProbe::absent();
        let launcher = FakeLauncher::never_binds();
        let p = EnsureParams {
            bind_timeout: Duration::from_millis(80),
            ..params(&probe, &launcher, &fx.lock, &fx.log)
        };

        match ensure_with(&p, StartCapability::MaySpawn) {
            EnsureOutcome::Failed { recovery } => {
                assert!(
                    recovery.contains(&fx.log.display().to_string()),
                    "recovery must name the daemon log: {recovery}"
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        assert_eq!(launcher.spawns(), 1, "spawned once before giving up");
    }

    #[test]
    fn timeout_copy_names_the_real_ceiling_not_just_bind_timeout() {
        // The recovery copy must name the *effective* wall-clock ceiling
        // (`bind_timeout + PROBE_TIMEOUT`): an in-flight probe can overrun the
        // bind_timeout by one `PROBE_TIMEOUT` (see `wait_until_answered` docs),
        // so printing `bind_timeout` alone under-reports the real bound. CIB-174.
        let fx = fixture();
        let (probe, _ready) = FlagProbe::absent();
        let launcher = FakeLauncher::never_binds();
        let bind_timeout = Duration::from_millis(80);
        let p = EnsureParams {
            bind_timeout,
            ..params(&probe, &launcher, &fx.lock, &fx.log)
        };

        match ensure_with(&p, StartCapability::MaySpawn) {
            EnsureOutcome::Failed { recovery } => {
                let ceiling = (bind_timeout + PROBE_TIMEOUT).as_secs();
                // Guard the fixture: the bare bind_timeout and the real ceiling
                // must round to different whole seconds, or this asserts nothing.
                assert_ne!(
                    bind_timeout.as_secs(),
                    ceiling,
                    "test setup: bind_timeout seconds must differ from the ceiling"
                );
                assert!(
                    recovery.contains(&format!("within {ceiling}s")),
                    "recovery must name the real ceiling ({ceiling}s), got: {recovery}"
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn spawn_error_is_reported_as_failed() {
        let fx = fixture();
        let (probe, _ready) = FlagProbe::absent();
        let launcher = FakeLauncher::failing();
        let p = params(&probe, &launcher, &fx.lock, &fx.log);

        match ensure_with(&p, StartCapability::MaySpawn) {
            EnsureOutcome::Failed { recovery } => {
                assert!(
                    recovery.contains("boom"),
                    "surfaces the spawn error: {recovery}"
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn spawn_that_binds_but_never_answers_times_out_to_failed() {
        // A daemon that binds (accepts) but never answers the status verb must
        // not be mistaken for `Started`: every bound-wait probe returns
        // `ConnectedNoAnswer`, so the wait must time out to `Failed`.
        let fx = fixture();
        let bound = Arc::new(AtomicBool::new(false));
        let probe = BindsButSilentProbe {
            bound: Arc::clone(&bound),
        };
        let launcher = FakeLauncher::that_starts(&bound);
        let p = EnsureParams {
            bind_timeout: Duration::from_millis(80),
            ..params(&probe, &launcher, &fx.lock, &fx.log)
        };

        match ensure_with(&p, StartCapability::MaySpawn) {
            EnsureOutcome::Failed { .. } => {}
            other => panic!("expected Failed when the daemon never answers, got {other:?}"),
        }
        assert_eq!(launcher.spawns(), 1, "spawned once, then timed out waiting");
    }

    #[test]
    fn concurrent_ensure_converges_on_one_daemon() {
        // Four callers race on the same lock file. The lock holder spawns; the
        // rest re-probe under the lock and reuse. Exactly one spawn.
        let fx = fixture();
        let ready = Arc::new(AtomicBool::new(false));
        let count = Arc::new(AtomicUsize::new(0));

        let outcomes: Vec<EnsureOutcome> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let ready = Arc::clone(&ready);
                    let count = Arc::clone(&count);
                    let lock = fx.lock.clone();
                    let log = fx.log.clone();
                    scope.spawn(move || {
                        let probe = FlagProbe {
                            ready: Arc::clone(&ready),
                            connected_no_answer: false,
                        };
                        let launcher = FakeLauncher {
                            count: Arc::clone(&count),
                            flips: Some(ready),
                            delay: Duration::from_millis(20),
                            fail: false,
                        };
                        let p = params(&probe, &launcher, &lock, &log);
                        ensure_with(&p, StartCapability::MaySpawn)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "concurrent ensure must spawn exactly one daemon, got {outcomes:?}"
        );
        assert!(
            outcomes.contains(&EnsureOutcome::Started),
            "one caller started it: {outcomes:?}"
        );
        assert!(
            outcomes.contains(&EnsureOutcome::Reused),
            "the other reused it: {outcomes:?}"
        );
    }

    #[test]
    fn platform_unsupported_outcome_is_no_start() {
        assert_eq!(
            platform_unsupported_outcome(),
            EnsureOutcome::NoStart {
                reason: NoStartReason::PlatformUnsupported
            }
        );
    }

    #[test]
    fn no_start_reason_discriminators_are_stable() {
        assert_eq!(NoStartReason::OptOut.as_str(), "opt-out");
        assert_eq!(NoStartReason::NonInteractive.as_str(), "non-interactive");
        assert_eq!(
            NoStartReason::PlatformUnsupported.as_str(),
            "platform-unsupported"
        );
    }

    // ----- Real SocketProbe integration tests (Unix) -----

    #[cfg(unix)]
    #[test]
    fn socket_probe_unreachable_when_no_endpoint() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("absent.sock");
        let probe = SocketProbe::new(socket);
        assert_eq!(probe.probe(), Liveness::Unreachable);
    }

    #[cfg(unix)]
    #[test]
    fn socket_probe_unreachable_when_stale_socket_file_has_no_listener() {
        use std::os::unix::net::UnixListener;
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("stale.sock");
        // Bind then drop the listener; std leaves the socket file in place, so
        // the file exists but nothing listens → connect is refused → Unreachable
        // (the only case that is safe to respawn over).
        let listener = UnixListener::bind(&socket).unwrap();
        drop(listener);
        assert!(socket.exists(), "stale socket file should remain");
        let probe = SocketProbe::new(socket);
        assert_eq!(probe.probe(), Liveness::Unreachable);
    }

    #[cfg(unix)]
    #[test]
    fn socket_probe_connected_no_answer_against_silent_listener() {
        use std::os::unix::net::UnixListener;
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("silent.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        // Accept connections but never answer — models a live-but-slow daemon.
        let accepter = std::thread::spawn(move || {
            // Hold the accepted stream so the connection stays open without a
            // reply for the duration of the probe.
            if let Ok((stream, _)) = listener.accept() {
                std::thread::sleep(Duration::from_millis(400));
                drop(stream);
            }
        });
        let probe = SocketProbe::with_timeout(socket, Duration::from_millis(150));
        assert_eq!(
            probe.probe(),
            Liveness::ConnectedNoAnswer,
            "a present-but-silent listener is never torn down"
        );
        let _ = accepter.join();
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn socket_probe_answered_against_live_daemon() {
        use crate::{ForegroundOpts, Shutdown, run_foreground};

        let dir = tempfile::tempdir().unwrap();
        // Runtime files go in a not-yet-existing subdir so the daemon's own
        // `ensure_secure_runtime_dir` creates it 0700 (the tempdir root is 0755).
        let rt = dir.path().join("rt");
        let pid_file = rt.join("intercept.pid");
        let socket = rt.join("intercept.sock");
        let fence_store = rt.join("state/intercept-fences.json");

        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(run_foreground(
            ForegroundOpts::with_pid_file_and_ipc_socket(&pid_file, &socket)
                .with_fence_store_file(&fence_store),
            token,
        ));

        // Wait for the daemon to bind, then probe from a blocking thread so the
        // synchronous socket I/O does not sit on a runtime worker.
        let probe_socket = socket.clone();
        let liveness = tokio::task::spawn_blocking(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let probe = SocketProbe::new(probe_socket.clone());
                match probe.probe() {
                    Liveness::Answered => return Liveness::Answered,
                    other => {
                        if Instant::now() >= deadline {
                            return other;
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
            }
        })
        .await
        .unwrap();

        shutdown.trigger();
        let _ = handle.await;

        assert_eq!(liveness, Liveness::Answered);
    }
}
