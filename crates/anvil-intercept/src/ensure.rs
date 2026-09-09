//! Bring a worktree/session to a usable protection readiness state.

#[cfg(unix)]
use crate::ipc::{LIVE_ENDPOINT_RECORD, read_live_endpoint_socket};
#[cfg(unix)]
use std::env;
#[cfg(unix)]
use std::ffi::OsStr;
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

/// Shared wall-clock budget for one ensure attempt: coordinator lock, start
/// lock, endpoint discovery, spawn and bind-wait (JREL-011). Slow partial
/// replies cannot extend it. Lock wait is capped separately so a stuck holder
/// fails fast instead of consuming the whole bind budget.
#[cfg(any(unix, windows))]
const LIFECYCLE_BUDGET: Duration = Duration::from_secs(12);

/// Cap on rendezvous-coordinator and per-install start-lock wait. A stuck
/// holder must not block ensure indefinitely (JREL-004 residual / JREL-011).
#[cfg(any(unix, windows))]
const LOCK_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(2);

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

/// Whether the caller already serialises daemon start against every endpoint
/// candidate of this execution scope (ADR-036).
///
/// `ensure_daemon` holds the same rendezvous coordinator that doctor's socket
/// repair holds (`intercept.rendezvous-repair.lock` in the physical state-home
/// directory when `ANVIL_HOME` is unset, or in the isolated prefix when it is
/// set) before the per-install start lock, so two shells whose environments
/// disagree about the canonical endpoint cannot both cold-start.
/// The coordinator is an advisory lock on an open file description, so a
/// process that already holds it must say so rather than re-open it and wait
/// on itself (JREL-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendezvousCoordination {
    /// Acquire the cross-candidate rendezvous coordinator before starting.
    Acquire,
    /// The caller holds the coordinator for these candidates already
    /// (`anvil doctor --fix` socket rendezvous repair).
    HeldByCaller,
}

/// The typed result of an [`ensure_daemon`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureOutcome {
    /// A live daemon already answers one of this scope's endpoint candidates
    /// (or a listener is present but slow and must not be torn down).
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

    /// Operator-facing label for this endpoint (path or pipe name).
    fn describe(&self) -> String {
        "endpoint".to_owned()
    }

    /// Filesystem path of this endpoint, when the probe is path-backed.
    /// Used to collapse ancestor-aliased sockets to one physical identity.
    #[cfg(unix)]
    fn endpoint_path(&self) -> Option<&Path> {
        None
    }
}

/// Launches a detached background daemon. Abstracted so the ensure state machine
/// is tested without spawning real processes.
pub trait DaemonLauncher {
    /// Spawn a detached background daemon, redirecting its stdout/stderr to
    /// `log_path`. Returns the spawned child's PID once the child is spawned —
    /// **not** once it has bound; the caller bound-waits via the probe. The
    /// ensure state machine records the PID in the live-endpoint advertisement;
    /// the save-time driver supervisor (DSV-047) also records it for later
    /// termination and liveness reporting.
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
/// Reuse considers every endpoint candidate of this execution scope — the
/// canonical bind path first, then the `XDG_RUNTIME_DIR` / state-home sibling
/// (`ANVIL_HOME` alone when set) — plus the owner-only live-endpoint record
/// published at the rendezvous coordinator, so a shell that disagrees about
/// the canonical bind still finds a daemon started at another runtime dir.
/// A daemon is only ever spawned at this process's canonical path.
///
/// `launcher` is how a detached daemon is spawned; the CLI passes a
/// [`DetachedCommandLauncher`] built from `current_exe()` and
/// `intercept start --foreground`.
#[cfg(unix)]
pub fn ensure_daemon(capability: StartCapability, launcher: &dyn DaemonLauncher) -> EnsureOutcome {
    ensure_daemon_coordinated(capability, launcher, RendezvousCoordination::Acquire)
}

/// [`ensure_daemon`] for a caller that states whether it already holds the
/// cross-candidate rendezvous coordinator (see [`RendezvousCoordination`]).
#[cfg(unix)]
pub fn ensure_daemon_coordinated(
    capability: StartCapability,
    launcher: &dyn DaemonLauncher,
    coordination: RendezvousCoordination,
) -> EnsureOutcome {
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
    let Ok(candidates) = crate::ipc::resolve_socket_connect_candidates() else {
        return EnsureOutcome::Failed {
            recovery: "could not resolve the per-user daemon socket candidates; \
                       check $XDG_RUNTIME_DIR / $HOME or set ANVIL_HOME"
                .to_owned(),
        };
    };
    let runtime_dir = pid_path.parent().unwrap_or_else(|| Path::new("."));
    ensure_daemon_at(
        capability,
        launcher,
        &socket_path,
        runtime_dir,
        &candidates,
        coordination,
    )
}

/// Path-explicit core of [`ensure_daemon_coordinated`]: `socket_path` is the
/// canonical bind path inside `runtime_dir`, and `candidates` is the ordered
/// connect set (canonical first) that reuse verifies and the rendezvous
/// coordinator is keyed on. Split from the environment resolution so both
/// environment orders (runtime dir set/unset) run in-process against real
/// sockets.
#[cfg(unix)]
pub(crate) fn ensure_daemon_at(
    capability: StartCapability,
    launcher: &dyn DaemonLauncher,
    socket_path: &Path,
    runtime_dir: &Path,
    candidates: &[PathBuf],
    coordination: RendezvousCoordination,
) -> EnsureOutcome {
    // #3220: pre-flight the runtime/socket directory before spawn so a
    // wrong-mode `ANVIL_HOME` surfaces as a chmod recovery instead of a
    // false "daemon did not become ready" + intercept-start hint after
    // the bind timeout. Owner-matched loose modes are tightened inside
    // `ensure_secure_runtime_dir`; remaining failures keep a cause-specific
    // recovery (chmod for mode, recreate-as-self for ownership).
    if let Err(err) = crate::ensure_secure_runtime_dir(runtime_dir) {
        let err_text = format!("{err:#}");
        let recovery = if err_text.contains("owned by uid") {
            format!(
                "runtime directory is not usable ({err_text}). \
                 When ANVIL_HOME re-roots the daemon, the prefix must be owned by you \
                 — recreate it as yourself (`install -d -m 700 '{}'`) and do not start \
                 a second foreground daemon to 'fix' permissions",
                runtime_dir.display()
            )
        } else if err_text.contains("symlink") {
            format!(
                "runtime directory is not usable ({err_text}). \
                 Point ANVIL_HOME at a real directory you own (`install -d -m 700 '…'`), \
                 not a symlink"
            )
        } else {
            format!(
                "runtime directory is not usable ({err_text}). \
                 When ANVIL_HOME re-roots the daemon, the prefix must be mode 0700 \
                 and owned by you — run: chmod 700 '{}'",
                runtime_dir.display()
            )
        };
        return EnsureOutcome::Failed { recovery };
    }
    // Both the lock and the log live beside the PID file, so they inherit its
    // per-`ANVIL_HOME` scoping (ADR-060): two re-rooted instances of the same
    // user do not share a lock.
    let lock_path = runtime_dir.join("intercept.ensure.lock");
    let log_path = runtime_dir.join("intercept.daemon.log");

    let probe = SocketProbe::new(socket_path.to_path_buf());
    let coordinator_dir = select_rendezvous_coordinator_dir(candidates);
    // Siblings are verified through the client-side owner-only gate before a
    // connection counts as a live daemon; a planted inode at a sibling path is
    // skipped, never reused, and never a reason to spawn elsewhere. Physical
    // duplicates of the canonical socket (ancestor aliases) are not probed
    // twice. The coordinator's advertised endpoint is re-read on every probe
    // so a daemon bound at another runtime dir becomes visible under the lock.
    let sibling_probes: Vec<SocketProbe> = candidates
        .iter()
        .filter(|candidate| {
            candidate.as_path() != socket_path
                && !crate::ipc::unix_sockets_are_same_physical(candidate, socket_path)
        })
        .map(|candidate| SocketProbe::for_sibling(candidate.clone()))
        .collect();
    let sibling_refs: Vec<&dyn DaemonProbe> = sibling_probes
        .iter()
        .map(|probe| probe as &dyn DaemonProbe)
        .collect();
    let rendezvous_candidates = match coordination {
        RendezvousCoordination::Acquire => Some(candidates),
        RendezvousCoordination::HeldByCaller => None,
    };
    let params = EnsureParams {
        probe: &probe,
        sibling_probes: &sibling_refs,
        launcher,
        lock_path: &lock_path,
        rendezvous_candidates,
        coordinator_dir: coordinator_dir.as_deref(),
        canonical_socket: Some(socket_path),
        log_path: &log_path,
        bind_timeout: DAEMON_BIND_TIMEOUT,
        poll_interval: BIND_POLL_INTERVAL,
        lifecycle_budget: LIFECYCLE_BUDGET,
        lock_timeout: LOCK_ACQUIRE_TIMEOUT,
    };
    ensure_with(&params, capability)
}

/// Windows entry: same state machine as Unix, probing the per-user named pipe
/// instead of the Unix socket (CIB-072 / GH #2609). The pipe namespace has one
/// location per user, so there is no sibling endpoint to verify and no
/// cross-candidate coordinator to hold.
#[cfg(windows)]
pub fn ensure_daemon_coordinated(
    capability: StartCapability,
    launcher: &dyn DaemonLauncher,
    _coordination: RendezvousCoordination,
) -> EnsureOutcome {
    ensure_daemon(capability, launcher)
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
        sibling_probes: &[],
        launcher,
        lock_path: &lock_path,
        rendezvous_candidates: None,
        coordinator_dir: None,
        canonical_socket: None,
        log_path: &log_path,
        bind_timeout: DAEMON_BIND_TIMEOUT,
        poll_interval: BIND_POLL_INTERVAL,
        lifecycle_budget: LIFECYCLE_BUDGET,
        lock_timeout: LOCK_ACQUIRE_TIMEOUT,
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

/// Platforms without a detached launcher implementation.
#[cfg(all(not(unix), not(windows)))]
pub fn ensure_daemon_coordinated(
    _capability: StartCapability,
    _launcher: &dyn DaemonLauncher,
    _coordination: RendezvousCoordination,
) -> EnsureOutcome {
    platform_unsupported_outcome()
}

/// Inputs to the platform-agnostic ensure state machine.
#[cfg(any(unix, windows))]
struct EnsureParams<'a> {
    /// The canonical endpoint: the only one a daemon is ever spawned at.
    probe: &'a dyn DaemonProbe,
    /// Same-scope sibling endpoints (already gated): a live one is reused.
    sibling_probes: &'a [&'a dyn DaemonProbe],
    launcher: &'a dyn DaemonLauncher,
    /// Per-install start lock beside the canonical PID file.
    lock_path: &'a Path,
    /// Every socket candidate the rendezvous coordinator is keyed on, or
    /// `None` when the caller already holds that coordinator.
    rendezvous_candidates: Option<&'a [PathBuf]>,
    /// Directory that holds the scope-stable coordinator lock and the
    /// live-endpoint record. `None` on Windows and in path-free unit tests.
    coordinator_dir: Option<&'a Path>,
    /// Canonical bind path this process would spawn at. Used to advertise a
    /// freshly started daemon.
    canonical_socket: Option<&'a Path>,
    log_path: &'a Path,
    bind_timeout: Duration,
    poll_interval: Duration,
    /// Overall monotonic budget for this ensure attempt (JREL-011).
    lifecycle_budget: Duration,
    /// Cap on coordinator / start-lock wait within that budget.
    lock_timeout: Duration,
}

/// The platform-agnostic ensure state machine. Pure but for the lock files, the
/// injected probes, and the injected launcher — so every branch is unit-tested.
#[cfg(any(unix, windows))]
fn ensure_with(params: &EnsureParams<'_>, capability: StartCapability) -> EnsureOutcome {
    let deadline = Instant::now() + params.lifecycle_budget;
    let remaining = || deadline.saturating_duration_since(Instant::now());

    // 1. Probes are read-only and always allowed, even for non-spawning
    //    callers: a live daemon at any verified endpoint of this scope is
    //    reused regardless of capability.
    match live_endpoints(params) {
        EndpointLiveness::One => return EnsureOutcome::Reused,
        EndpointLiveness::Conflict { live, endpoints } => {
            return conflict_outcome(live, &endpoints);
        }
        EndpointLiveness::Unresponsive { endpoint } => {
            return unresponsive_outcome(&endpoint);
        }
        EndpointLiveness::None => {}
    }

    // 2. No live daemon. Only callers with a consent surface may spawn.
    if let StartCapability::NoSpawn(reason) = capability {
        return EnsureOutcome::NoStart { reason };
    }

    // 3. Serialise the spawn critical section. Coordinator then per-install
    //    start lock, both inside the remaining lifecycle budget (JREL-011).
    let (_rendezvous, _lock) = match hold_spawn_serialisation(params, deadline) {
        Ok(locks) => locks,
        Err(outcome) => return outcome,
    };

    // 4. Re-probe under the locks: a racing caller — from this environment or
    //    a sibling one — may have started a daemon while we waited, and may
    //    have advertised it at the coordinator.
    match live_endpoints(params) {
        EndpointLiveness::One => {
            publish_observed_live_endpoint(params);
            return EnsureOutcome::Reused;
        }
        EndpointLiveness::Conflict { live, endpoints } => {
            return conflict_outcome(live, &endpoints);
        }
        EndpointLiveness::Unresponsive { endpoint } => {
            return unresponsive_outcome(&endpoint);
        }
        EndpointLiveness::None => {}
    }

    // 5. Spawn the detached daemon. Its own IpcListener bind unlinks any stale
    //    socket it owns, so we never unlink an endpoint here.
    let spawned_pid = match params.launcher.spawn_detached(params.log_path) {
        Ok(pid) => pid,
        Err(err) => {
            return EnsureOutcome::Failed {
                recovery: format!(
                    "failed to launch the background daemon: {err}. \
                     See the daemon log at {} or retry with bare `anvil`.",
                    params.log_path.display()
                ),
            };
        }
    };

    // 6. Bound-wait for the new daemon to bind and answer, then advertise the
    //    canonical endpoint at the coordinator so later shells can find it.
    //    Bind-wait shares the remaining lifecycle budget so lock wait cannot
    //    extend the overall ceiling (JREL-011).
    let bind_wait = remaining().min(params.bind_timeout);
    if wait_until_answered(params.probe, bind_wait, params.poll_interval) {
        if let (Some(dir), Some(socket)) = (params.coordinator_dir, params.canonical_socket) {
            publish_live_endpoint_record(dir, socket, spawned_pid);
        }
        EnsureOutcome::Started
    } else {
        EnsureOutcome::Failed {
            recovery: format!(
                "the daemon did not become ready within {}s. \
                 See the daemon log at {} or retry with bare `anvil`.",
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

#[cfg(any(unix, windows))]
fn unresponsive_outcome(endpoint: &str) -> EnsureOutcome {
    EnsureOutcome::Failed {
        recovery: format!(
            "the intercept daemon at {endpoint} accepted a connection but did not \
             answer within the lifecycle budget; not spawning a second daemon. \
             Run `anvil doctor --fix` or retry with bare `anvil`"
        ),
    }
}

/// Take the rendezvous coordinator then the per-install start lock, sharing
/// the remaining lifecycle budget. Failure is a typed [`EnsureOutcome::Failed`]
/// so a stuck holder cannot spawn a duplicate (JREL-011).
#[cfg(any(unix, windows))]
fn hold_spawn_serialisation(
    params: &EnsureParams<'_>,
    deadline: Instant,
) -> Result<(Option<std::fs::File>, std::fs::File), EnsureOutcome> {
    let remaining = || deadline.saturating_duration_since(Instant::now());
    if remaining().is_zero() {
        return Err(EnsureOutcome::Failed {
            recovery: "the daemon lifecycle budget elapsed before the start lock \
                       could be taken. Retry with bare `anvil`"
                .to_owned(),
        });
    }
    let lock_wait = remaining().min(params.lock_timeout);
    let rendezvous = match hold_spawn_rendezvous(params.rendezvous_candidates, lock_wait) {
        Ok(lock) => lock,
        Err(err) => {
            return Err(EnsureOutcome::Failed {
                recovery: format!(
                    "could not hold the daemon rendezvous coordinator within the \
                     lifecycle budget: {err}. Not spawning a second daemon. Retry \
                     with bare `anvil` or run `anvil doctor --fix`"
                ),
            });
        }
    };
    if remaining().is_zero() {
        return Err(EnsureOutcome::Failed {
            recovery: "the daemon lifecycle budget elapsed while waiting for the \
                       rendezvous coordinator. Retry with bare `anvil`"
                .to_owned(),
        });
    }
    let lock_wait = remaining().min(params.lock_timeout);
    let lock =
        acquire_ensure_lock(params.lock_path, lock_wait).map_err(|err| EnsureOutcome::Failed {
            recovery: format!(
                "could not acquire the daemon-start lock at {} within the \
                 lifecycle budget: {err}",
                params.lock_path.display()
            ),
        })?;
    Ok((rendezvous, lock))
}

/// Hold the cross-candidate rendezvous coordinator across the spawn critical
/// section, or `None` when the caller already holds it.
///
/// A sibling directory this environment cannot establish must not block the
/// canonical start: same-environment callers remain serialised by the
/// per-install start lock the caller takes next.
#[cfg(unix)]
fn hold_spawn_rendezvous(
    candidates: Option<&[PathBuf]>,
    timeout: Duration,
) -> io::Result<Option<std::fs::File>> {
    let Some(candidates) = candidates else {
        return Ok(None);
    };
    acquire_daemon_rendezvous_repair_lock_for_socket_candidates_within(candidates, timeout)
        .map(Some)
}

/// Windows has one pipe location per user, so there is no sibling candidate to
/// coordinate and no coordinator to hold.
#[cfg(windows)]
fn hold_spawn_rendezvous(
    candidates: Option<&[PathBuf]>,
    _timeout: Duration,
) -> io::Result<Option<std::fs::File>> {
    debug_assert!(
        candidates.is_none(),
        "the Windows pipe namespace has no sibling candidate to coordinate"
    );
    Ok(None)
}

/// How many of this scope's endpoints carry a daemon to reuse.
#[cfg(any(unix, windows))]
#[derive(Debug, Clone, PartialEq, Eq)]
enum EndpointLiveness {
    /// No endpoint answers or listens.
    None,
    /// Exactly one endpoint carries a daemon: reuse it.
    One,
    /// A listener accepted a connection but never answered. Distinct from
    /// absence: spawning would risk a duplicate (JREL-011).
    Unresponsive {
        /// Operator-facing endpoint label.
        endpoint: String,
    },
    /// More than one same-scope endpoint carries a daemon. ADR-036 allows one
    /// daemon per execution scope, so this is reported, never resolved by
    /// silently picking one (JREL-004).
    Conflict {
        /// How many endpoints answered.
        live: usize,
        /// Canonical/sibling labels that answered, for the recovery hint and log.
        endpoints: Vec<String>,
    },
}

/// Probe the canonical endpoint and every verified sibling endpoint of this
/// scope. Only an **answering** daemon counts as live: a listener that accepts
/// but never answers is not a daemon we can reuse, and must not inflate a
/// same-scope conflict (any same-user process binding a sibling path used to
/// wedge every command).
#[cfg(any(unix, windows))]
fn live_endpoints(params: &EnsureParams<'_>) -> EndpointLiveness {
    let mut answered = Vec::new();
    let mut unresponsive: Option<String> = None;
    #[cfg(unix)]
    let mut seen_inodes: Vec<(u64, u64)> = Vec::new();
    let mut consider = |probe: &dyn DaemonProbe, label: String| {
        match probe.probe() {
            Liveness::Answered => {}
            Liveness::ConnectedNoAnswer => {
                if unresponsive.is_none() {
                    unresponsive = Some(label);
                }
                return;
            }
            Liveness::Unreachable => return,
        }
        #[cfg(unix)]
        {
            if let Some(path) = probe.endpoint_path()
                && let Some(id) = crate::ipc::socket_physical_identity(path)
            {
                if seen_inodes.contains(&id) {
                    return;
                }
                seen_inodes.push(id);
            }
        }
        answered.push(label);
    };
    consider(
        params.probe,
        format!("canonical ({})", params.probe.describe()),
    );
    for (index, sibling) in params.sibling_probes.iter().enumerate() {
        consider(
            *sibling,
            format!("sibling-{index} ({})", sibling.describe()),
        );
    }
    #[cfg(unix)]
    if let Some(dir) = params.coordinator_dir
        && let Some(advertised) = read_live_endpoint_socket(dir)
    {
        let already_known = params.canonical_socket.is_some_and(|canonical| {
            advertised.as_path() == canonical
                || crate::ipc::unix_sockets_are_same_physical(&advertised, canonical)
        }) || params.sibling_probes.iter().any(|sibling| {
            sibling.endpoint_path().is_some_and(|path| {
                path == advertised.as_path()
                    || crate::ipc::unix_sockets_are_same_physical(path, &advertised)
            })
        });
        if !already_known {
            let probe = SocketProbe::for_sibling(advertised);
            consider(&probe, format!("advertised ({})", probe.describe()));
        }
    }
    match answered.len() {
        0 => match unresponsive {
            Some(endpoint) => EndpointLiveness::Unresponsive { endpoint },
            None => EndpointLiveness::None,
        },
        1 => EndpointLiveness::One,
        live => EndpointLiveness::Conflict {
            live,
            endpoints: answered,
        },
    }
}

/// The bounded-recovery outcome for a same-scope daemon conflict.
#[cfg(any(unix, windows))]
fn conflict_outcome(live: usize, endpoints: &[String]) -> EnsureOutcome {
    let named = endpoints.join(", ");
    tracing::warn!(
        target: "anvil_intercept::ensure",
        live,
        endpoints = %named,
        "same-scope daemon conflict: more than one live daemon answers this execution scope"
    );
    EnsureOutcome::Failed {
        recovery: format!(
            "{live} live daemons answer this execution scope where one is expected ({named}); \
             none was reused or stopped. Run `anvil doctor --fix` to repair the \
             endpoints, or `anvil intercept stop` then `anvil start`"
        ),
    }
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
/// to spawn. Waits up to `timeout` then fails rather than blocking indefinitely
/// (JREL-011). Released when the returned guard drops.
///
/// The lock file is opened with the default close-on-exec flag, so a detached
/// daemon child spawned while the lock is held never inherits (and therefore
/// never wedges) it.
#[cfg(any(unix, windows))]
fn acquire_ensure_lock(lock_path: &Path, timeout: Duration) -> io::Result<std::fs::File> {
    use std::fs::{OpenOptions, TryLockError};

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
    let deadline = Instant::now() + timeout;
    let mut backoff = Duration::from_millis(5);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) => {
                let now = Instant::now();
                if now >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!(
                            "timed out after {}ms waiting for the daemon-start lock",
                            timeout.as_millis()
                        ),
                    ));
                }
                let remaining = deadline.saturating_duration_since(now);
                std::thread::sleep(backoff.min(remaining).min(Duration::from_millis(50)));
                backoff = (backoff * 2).min(Duration::from_millis(50));
            }
            Err(TryLockError::Error(err)) => return Err(err),
        }
    }
}

/// Hold the per-install daemon-start lock while an operator repairs endpoint
/// state. This serialises cleanup with every background start path.
///
/// # Errors
///
/// Returns an I/O error when the owner-only runtime directory or advisory lock
/// cannot be opened or locked.
#[cfg(unix)]
pub fn acquire_daemon_start_lock_for_pid_file(pid_path: &Path) -> io::Result<std::fs::File> {
    let runtime_dir = pid_path.parent().unwrap_or_else(|| Path::new("."));
    acquire_ensure_lock(
        &runtime_dir.join("intercept.ensure.lock"),
        LOCK_ACQUIRE_TIMEOUT,
    )
}

/// Serialise operator repair and background start across every socket
/// candidate, independent of which candidate the current process considers
/// canonical.
///
/// When `ANVIL_HOME` is unset, the coordinator is always the physical
/// state-home directory (`$HOME/.local/state/anvil`) if that parent is in the
/// candidate set — not the lexicographic minimum of this process's runtime
/// dirs. Isolated `ANVIL_HOME` keeps the lock in that prefix only (ADR-060).
/// Only the chosen anchor is created; a sibling directory this environment
/// cannot establish must not skip the scope lock.
///
/// The coordinator uses a distinct lock file from daemon start, allowing
/// doctor to retain sibling start/PID fences while starting the canonical
/// daemon without an opposite-canonical AB/BA cycle. [`ensure_daemon`] holds
/// the same coordinator before its per-install start lock; doctor therefore
/// launches through [`RendezvousCoordination::HeldByCaller`] rather than
/// re-opening a lock its own process holds.
///
/// # Errors
///
/// Returns an invalid-input error when no candidate has a parent, or an I/O
/// error when the coordinator directory cannot be securely established and
/// resolved to a physical identity, or when the advisory lock cannot be
/// opened or locked.
#[cfg(unix)]
pub fn acquire_daemon_rendezvous_repair_lock_for_socket_candidates(
    socket_candidates: &[PathBuf],
) -> io::Result<std::fs::File> {
    acquire_daemon_rendezvous_repair_lock_for_socket_candidates_within(
        socket_candidates,
        LOCK_ACQUIRE_TIMEOUT,
    )
}

/// Acquire the rendezvous coordinator with an explicit wait budget (JREL-011).
#[cfg(unix)]
pub fn acquire_daemon_rendezvous_repair_lock_for_socket_candidates_within(
    socket_candidates: &[PathBuf],
    timeout: Duration,
) -> io::Result<std::fs::File> {
    let coordinator = select_rendezvous_coordinator_dir(socket_candidates).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "intercept rendezvous repair needs a socket candidate with a parent",
        )
    })?;
    crate::ensure_secure_runtime_dir(&coordinator)
        .map_err(|err| io::Error::other(format!("{err:#}")))?;
    let physical = coordinator.canonicalize().map_err(|err| {
        io::Error::new(
            err.kind(),
            format!(
                "failed to resolve intercept rendezvous coordinator {}: {err}",
                coordinator.display()
            ),
        )
    })?;
    acquire_ensure_lock(&physical.join("intercept.rendezvous-repair.lock"), timeout)
}

/// Prefer physical `$HOME/.local/state/anvil` when that parent is in the
/// candidate set; otherwise the first parent (isolated `ANVIL_HOME`, or `HOME`
/// unset).
///
/// Suffix matching alone is not enough: `XDG_RUNTIME_DIR` values such as
/// `/tmp/.../.local/state` produce a parent that also ends with
/// `.local/state/anvil`. Prefer the actual `$HOME` state-home candidate when
/// it is present (lexical or physical). When `$HOME` does not match any
/// candidate, keep a unique suffix match so in-process tests need not mutate
/// process `HOME`. When `$HOME` is unavailable, or several suffix matches
/// remain, fall back to the first parent.
#[cfg(unix)]
fn select_rendezvous_coordinator_dir(socket_candidates: &[PathBuf]) -> Option<PathBuf> {
    let home = env::var_os("HOME");
    select_rendezvous_coordinator_dir_from(socket_candidates, home.as_deref())
}

#[cfg(unix)]
fn select_rendezvous_coordinator_dir_from(
    socket_candidates: &[PathBuf],
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    let parents: Vec<PathBuf> = socket_candidates
        .iter()
        .filter_map(|candidate| candidate.parent().map(Path::to_path_buf))
        .collect();
    if parents.is_empty() {
        return None;
    }
    if let Some(home) = home {
        let home_state = Path::new(home).join(".local/state/anvil");
        if let Some(parent) = parents.iter().find(|parent| {
            parent.as_path() == home_state.as_path()
                || crate::ipc::dirs_share_physical_identity(parent, &home_state)
        }) {
            return Some(parent.clone());
        }
    }
    let suffix_matches: Vec<&PathBuf> = parents
        .iter()
        .filter(|parent| parent.ends_with(".local/state/anvil"))
        .collect();
    if suffix_matches.len() == 1 {
        return Some(suffix_matches[0].clone());
    }
    Some(parents[0].clone())
}

#[cfg(unix)]
fn publish_observed_live_endpoint(params: &EnsureParams<'_>) {
    let Some(dir) = params.coordinator_dir else {
        return;
    };
    let Some(socket) = observed_live_socket_path(params) else {
        return;
    };
    publish_live_endpoint_record(dir, &socket, pid_beside_socket(&socket));
}

#[cfg(not(unix))]
fn publish_observed_live_endpoint(_params: &EnsureParams<'_>) {}

#[cfg(unix)]
fn observed_live_socket_path(params: &EnsureParams<'_>) -> Option<PathBuf> {
    if let Some(path) = first_live_socket_path(params) {
        return Some(path.to_path_buf());
    }
    let advertised = read_live_endpoint_socket(params.coordinator_dir?)?;
    let probe = SocketProbe::for_sibling(advertised.clone());
    (probe.probe() == Liveness::Answered).then_some(advertised)
}

#[cfg(unix)]
fn first_live_socket_path<'a>(params: &'a EnsureParams<'a>) -> Option<&'a Path> {
    std::iter::once(params.probe)
        .chain(params.sibling_probes.iter().copied())
        .find(|probe| probe.probe() == Liveness::Answered)
        .and_then(DaemonProbe::endpoint_path)
}

#[cfg(unix)]
fn pid_beside_socket(socket: &Path) -> u32 {
    socket
        .parent()
        .and_then(|dir| std::fs::read_to_string(dir.join("intercept.pid")).ok())
        .and_then(|record| record.lines().next()?.trim().parse().ok())
        .unwrap_or(0)
}

#[cfg(unix)]
fn publish_live_endpoint_record(dir: &Path, socket: &Path, pid: u32) {
    if let Err(err) = write_live_endpoint_record(dir, socket, pid) {
        tracing::warn!(
            target: "anvil_intercept::ensure",
            error = %err,
            coordinator = %dir.display(),
            socket = %socket.display(),
            "could not publish the daemon live-endpoint record"
        );
    }
}

#[cfg(not(unix))]
fn publish_live_endpoint_record(_dir: &Path, _socket: &Path, _pid: u32) {}

#[cfg(unix)]
fn write_live_endpoint_record(dir: &Path, socket: &Path, pid: u32) -> io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    crate::ensure_secure_runtime_dir(dir).map_err(|err| io::Error::other(format!("{err:#}")))?;
    let dest = dir.join(LIVE_ENDPOINT_RECORD);
    let tmp = dir.join("intercept.rendezvous-endpoint.tmp");
    match std::fs::remove_file(&tmp) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(nix::libc::O_NOFOLLOW)
            .open(&tmp)?;
        write!(file, "{}\n{pid}\n", socket.display())?;
        file.sync_all()?;
    }
    std::fs::rename(tmp, dest)?;
    Ok(())
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
    /// Apply the client-side owner-only path gate before connecting. Sibling
    /// endpoints are gated (a planted inode there is skipped, never reused);
    /// the canonical endpoint keeps its connect-only contract because the
    /// daemon's own bind owns that path.
    gate_path: bool,
}

#[cfg(unix)]
impl SocketProbe {
    /// Probe `socket_path` for a live, answering daemon.
    #[must_use]
    pub(crate) fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            timeout: PROBE_TIMEOUT,
            gate_path: false,
        }
    }

    /// Probe a same-scope sibling endpoint. The path must pass
    /// [`crate::ipc::validate_socket_path_for_client`] before a connection
    /// counts; any other metadata is treated as no daemon here.
    #[must_use]
    pub(crate) fn for_sibling(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            timeout: PROBE_TIMEOUT,
            gate_path: true,
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
            gate_path: false,
        }
    }
}

#[cfg(unix)]
impl DaemonProbe for SocketProbe {
    fn probe(&self) -> Liveness {
        use std::os::unix::net::UnixStream;

        if self.gate_path && crate::ipc::validate_socket_path_for_client(&self.socket_path).is_err()
        {
            return Liveness::Unreachable;
        }
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

    fn describe(&self) -> String {
        self.socket_path.display().to_string()
    }

    fn endpoint_path(&self) -> Option<&Path> {
        Some(&self.socket_path)
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
    #[cfg_attr(
        windows,
        allow(
            clippy::unnested_or_patterns,
            reason = "Windows-only clippy debt baselined by CIB-204; clearing it restructures named-pipe transport code that only a Windows runner can build and test."
        )
    )]
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

    fn describe(&self) -> String {
        self.pipe_name.clone()
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
#[cfg_attr(
    windows,
    allow(
        clippy::manual_let_else,
        reason = "Windows-only clippy debt baselined by CIB-204; clearing it restructures named-pipe transport code that only a Windows runner can build and test."
    )
)]
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
            sibling_probes: &[],
            launcher,
            lock_path,
            rendezvous_candidates: None,
            coordinator_dir: None,
            canonical_socket: None,
            log_path,
            bind_timeout: Duration::from_secs(5),
            poll_interval: Duration::from_millis(5),
            lifecycle_budget: Duration::from_secs(12),
            lock_timeout: Duration::from_secs(2),
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
    fn connected_but_silent_endpoint_is_not_reused() {
        // A listener that accepts but never answers is unresponsive, not absent.
        // Spawning would risk a duplicate (JREL-011).
        let fx = fixture();
        let probe = FlagProbe {
            ready: Arc::new(AtomicBool::new(false)),
            connected_no_answer: true,
        };
        let launcher = FakeLauncher::never_binds();
        let p = EnsureParams {
            bind_timeout: Duration::from_millis(40),
            ..params(&probe, &launcher, &fx.lock, &fx.log)
        };

        match ensure_with(&p, StartCapability::MaySpawn) {
            EnsureOutcome::Failed { recovery } => {
                assert!(
                    recovery.contains("not spawning a second daemon"),
                    "recovery must refuse a duplicate spawn: {recovery}"
                );
            }
            other => panic!("silent listener must not be reused, got {other:?}"),
        }
        assert_eq!(
            launcher.spawns(),
            0,
            "an unresponsive listener must not be spawned over"
        );
    }

    #[test]
    fn delayed_start_lock_fails_within_lifecycle_budget_without_spawning() {
        let fx = fixture();
        let _holder =
            acquire_ensure_lock(&fx.lock, Duration::from_secs(2)).expect("hold start lock");
        let (probe, _ready) = FlagProbe::absent();
        let launcher = FakeLauncher::never_binds();
        let p = EnsureParams {
            lock_timeout: Duration::from_millis(80),
            lifecycle_budget: Duration::from_millis(120),
            bind_timeout: Duration::from_millis(80),
            ..params(&probe, &launcher, &fx.lock, &fx.log)
        };
        let started = Instant::now();
        match ensure_with(&p, StartCapability::MaySpawn) {
            EnsureOutcome::Failed { recovery } => {
                assert!(
                    recovery.contains("lifecycle budget") || recovery.contains("timed out"),
                    "recovery must name the budget/timeout: {recovery}"
                );
            }
            other => panic!("expected Failed on delayed lock, got {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_millis(800),
            "delayed lock must not block past the lifecycle budget"
        );
        assert_eq!(
            launcher.spawns(),
            0,
            "must not spawn while the lock is held"
        );
    }

    #[test]
    fn slow_drip_bind_wait_cannot_extend_lifecycle_budget() {
        let fx = fixture();
        let (probe, _ready) = FlagProbe::absent();
        let launcher = FakeLauncher::never_binds();
        let p = EnsureParams {
            bind_timeout: Duration::from_millis(60),
            lifecycle_budget: Duration::from_millis(80),
            poll_interval: Duration::from_millis(15),
            ..params(&probe, &launcher, &fx.lock, &fx.log)
        };
        let started = Instant::now();
        match ensure_with(&p, StartCapability::MaySpawn) {
            EnsureOutcome::Failed { .. } => {}
            other => panic!("expected Failed when bind wait expires, got {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_millis(800),
            "slow bind-wait polls must not extend the lifecycle budget"
        );
        assert_eq!(launcher.spawns(), 1);
    }

    #[test]
    fn delayed_rendezvous_coordinator_fails_within_budget_without_spawning() {
        let fx = fixture();
        let socket = fx.lock.parent().expect("rt").join("intercept.sock");
        let candidates = [socket];
        let _holder = acquire_daemon_rendezvous_repair_lock_for_socket_candidates_within(
            &candidates,
            Duration::from_secs(2),
        )
        .expect("hold coordinator");
        let (probe, _ready) = FlagProbe::absent();
        let launcher = FakeLauncher::never_binds();
        let p = EnsureParams {
            rendezvous_candidates: Some(&candidates),
            lock_timeout: Duration::from_millis(80),
            lifecycle_budget: Duration::from_millis(150),
            bind_timeout: Duration::from_millis(80),
            ..params(&probe, &launcher, &fx.lock, &fx.log)
        };
        let started = Instant::now();
        match ensure_with(&p, StartCapability::MaySpawn) {
            EnsureOutcome::Failed { recovery } => {
                assert!(
                    recovery.contains("rendezvous coordinator")
                        || recovery.contains("lifecycle budget"),
                    "recovery must name the coordinator budget: {recovery}"
                );
            }
            other => panic!("expected Failed on delayed coordinator, got {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_millis(800),
            "coordinator wait must not block past the lifecycle budget"
        );
        assert_eq!(launcher.spawns(), 0);
    }

    #[test]
    fn answering_daemon_plus_silent_sibling_is_reused_not_a_conflict() {
        let fx = fixture();
        let canonical = FlagProbe {
            ready: Arc::new(AtomicBool::new(true)),
            connected_no_answer: false,
        };
        let sibling = FlagProbe {
            ready: Arc::new(AtomicBool::new(false)),
            connected_no_answer: true,
        };
        let launcher = FakeLauncher::never_binds();
        let siblings: [&dyn DaemonProbe; 1] = [&sibling];
        let p = EnsureParams {
            sibling_probes: &siblings,
            ..params(&canonical, &launcher, &fx.lock, &fx.log)
        };

        assert_eq!(
            ensure_with(&p, StartCapability::MaySpawn),
            EnsureOutcome::Reused
        );
        assert_eq!(launcher.spawns(), 0);
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

    // ----- JREL-004: one daemon identity across endpoint candidates -----

    /// JREL-004: a daemon discovered live at a same-scope sibling endpoint is
    /// the daemon; the canonical path being empty is not a reason to spawn.
    #[test]
    fn live_sibling_is_reused_without_spawning_a_duplicate() {
        let fx = fixture();
        let (canonical, _never_ready) = FlagProbe::absent();
        let sibling = FlagProbe {
            ready: Arc::new(AtomicBool::new(true)),
            connected_no_answer: false,
        };
        let launcher = FakeLauncher::never_binds();
        let siblings: [&dyn DaemonProbe; 1] = [&sibling];
        let p = EnsureParams {
            sibling_probes: &siblings,
            ..params(&canonical, &launcher, &fx.lock, &fx.log)
        };

        assert_eq!(
            ensure_with(&p, StartCapability::MaySpawn),
            EnsureOutcome::Reused
        );
        assert_eq!(
            launcher.spawns(),
            0,
            "a live sibling daemon must be reused, never duplicated"
        );
    }

    #[test]
    fn two_live_same_scope_daemons_are_reported_not_chosen() {
        let fx = fixture();
        let canonical = FlagProbe {
            ready: Arc::new(AtomicBool::new(true)),
            connected_no_answer: false,
        };
        let sibling = FlagProbe {
            ready: Arc::new(AtomicBool::new(true)),
            connected_no_answer: false,
        };
        let launcher = FakeLauncher::never_binds();
        let siblings: [&dyn DaemonProbe; 1] = [&sibling];
        let p = EnsureParams {
            sibling_probes: &siblings,
            ..params(&canonical, &launcher, &fx.lock, &fx.log)
        };

        match ensure_with(&p, StartCapability::MaySpawn) {
            EnsureOutcome::Failed { recovery } => assert!(
                recovery.contains("2 live daemons")
                    && recovery.contains("canonical")
                    && recovery.contains("sibling-0")
                    && recovery.contains("anvil doctor --fix"),
                "the conflict must name the endpoints and the bounded recovery: {recovery}"
            ),
            other => panic!("a same-scope conflict must be reported, got {other:?}"),
        }
        assert_eq!(
            launcher.spawns(),
            0,
            "a conflict must never be resolved by spawning a third daemon"
        );
    }

    /// A minimal same-user listener that answers the ensure probe's
    /// `anvil/status/query` with an id-matched `result`, standing in for a
    /// daemon bound at an arbitrary endpoint path.
    struct FakeDaemon {
        stop: Arc<AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
        path: PathBuf,
    }

    impl FakeDaemon {
        fn bind(path: &Path) -> Self {
            use std::io::{BufRead, BufReader, Write};
            use std::os::unix::fs::PermissionsExt;
            use std::os::unix::net::UnixListener;

            let parent = path.parent().expect("socket parent");
            crate::ensure_secure_runtime_dir(parent).expect("owner-only endpoint dir");
            let listener = UnixListener::bind(path).expect("bind fake daemon");
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .expect("owner-only socket");
            listener.set_nonblocking(true).expect("non-blocking accept");
            let stop = Arc::new(AtomicBool::new(false));
            let stop_flag = Arc::clone(&stop);
            let thread = std::thread::spawn(move || {
                while !stop_flag.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let _ = stream.set_nonblocking(false);
                            let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                            let mut reader = BufReader::new(&stream);
                            let mut line = String::new();
                            if reader.read_line(&mut line).is_ok() {
                                let mut writer = &stream;
                                let _ = writeln!(
                                    writer,
                                    "{{\"jsonrpc\":\"2.0\",\"id\":\"anvil-ensure-probe\",\"result\":{{}}}}"
                                );
                            }
                        }
                        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                stop,
                thread: Some(thread),
                path: path.to_path_buf(),
            }
        }
    }

    impl Drop for FakeDaemon {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }

    /// A launcher that, after `delay`, binds a [`FakeDaemon`] at the canonical
    /// socket path it was built for — a real-socket stand-in for
    /// `intercept start --foreground` reaching its bind.
    struct FakeSocketDaemonLauncher {
        socket_path: PathBuf,
        delay: Duration,
        count: Arc<AtomicUsize>,
        daemons: Arc<std::sync::Mutex<Vec<FakeDaemon>>>,
    }

    impl FakeSocketDaemonLauncher {
        fn new(
            socket_path: &Path,
            delay: Duration,
            count: &Arc<AtomicUsize>,
            daemons: &Arc<std::sync::Mutex<Vec<FakeDaemon>>>,
        ) -> Self {
            Self {
                socket_path: socket_path.to_path_buf(),
                delay,
                count: Arc::clone(count),
                daemons: Arc::clone(daemons),
            }
        }
    }

    impl DaemonLauncher for FakeSocketDaemonLauncher {
        fn spawn_detached(&self, _log_path: &Path) -> io::Result<u32> {
            self.count.fetch_add(1, Ordering::SeqCst);
            let socket_path = self.socket_path.clone();
            let delay = self.delay;
            let daemons = Arc::clone(&self.daemons);
            std::thread::spawn(move || {
                if !delay.is_zero() {
                    std::thread::sleep(delay);
                }
                let daemon = FakeDaemon::bind(&socket_path);
                daemons.lock().expect("daemon list").push(daemon);
            });
            Ok(1)
        }
    }

    /// The two shell environments of one execution scope: a shell with
    /// `XDG_RUNTIME_DIR` (runtime dir canonical, state home sibling) and a
    /// shell without it (state home canonical, runtime dir found through the
    /// implicit `/run/user/<uid>` sibling), resolved through the real
    /// candidate resolver against per-test directories.
    struct ScopeEnvironments {
        dir: tempfile::TempDir,
        xdg_candidates: Vec<PathBuf>,
        plain_candidates: Vec<PathBuf>,
    }

    fn scope_environments() -> ScopeEnvironments {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("home");
        let runtime = dir.path().join("runtime");
        let xdg_candidates = crate::ipc::resolve_socket_connect_candidates_with_env(
            None,
            Some(runtime.clone().into_os_string()),
            Some(home.clone().into_os_string()),
            None,
        )
        .expect("runtime-dir shell candidates");
        let plain_candidates = crate::ipc::resolve_socket_connect_candidates_with_env(
            None,
            None,
            Some(home.into_os_string()),
            Some(runtime),
        )
        .expect("plain shell candidates");
        assert_eq!(xdg_candidates.len(), 2, "{xdg_candidates:?}");
        assert_eq!(plain_candidates.len(), 2, "{plain_candidates:?}");
        assert_eq!(xdg_candidates[0], plain_candidates[1]);
        assert_eq!(xdg_candidates[1], plain_candidates[0]);
        ScopeEnvironments {
            dir,
            xdg_candidates,
            plain_candidates,
        }
    }

    /// Two shells that share `HOME` but not a runtime dir: the candidate sets
    /// intersect only at state-home, so discovery cannot rely on the implicit
    /// `/run/user/<uid>` sibling.
    struct DisjointRuntimeEnvironments {
        _dir: tempfile::TempDir,
        first_set: Vec<PathBuf>,
        other_set: Vec<PathBuf>,
        state_home: PathBuf,
    }

    fn disjoint_runtime_environments() -> DisjointRuntimeEnvironments {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("home");
        let runtime_one = dir.path().join("xdg-a");
        let runtime_two = dir.path().join("xdg-b");
        let first_set = crate::ipc::resolve_socket_connect_candidates_with_env(
            None,
            Some(runtime_one.into_os_string()),
            Some(home.clone().into_os_string()),
            None,
        )
        .expect("first runtime candidates");
        let other_set = crate::ipc::resolve_socket_connect_candidates_with_env(
            None,
            Some(runtime_two.into_os_string()),
            Some(home.clone().into_os_string()),
            None,
        )
        .expect("other runtime candidates");
        assert_eq!(first_set.len(), 2, "{first_set:?}");
        assert_eq!(other_set.len(), 2, "{other_set:?}");
        assert_ne!(first_set[0], other_set[0]);
        assert_eq!(first_set[1], other_set[1]);
        let state_home = home.join(".local/state/anvil");
        DisjointRuntimeEnvironments {
            _dir: dir,
            first_set,
            other_set,
            state_home,
        }
    }

    fn ensure_in(
        candidates: &[PathBuf],
        launcher: &dyn DaemonLauncher,
        coordination: RendezvousCoordination,
    ) -> EnsureOutcome {
        let canonical = &candidates[0];
        let runtime_dir = canonical.parent().expect("canonical parent");
        ensure_daemon_at(
            StartCapability::MaySpawn,
            launcher,
            canonical,
            runtime_dir,
            candidates,
            coordination,
        )
    }

    /// JREL-004, "unset then set": a daemon bound under the state home by a
    /// shell without `XDG_RUNTIME_DIR` is reused by a shell that has one.
    #[test]
    fn xdg_shell_reuses_state_home_daemon_through_the_sibling_candidate() {
        let env = scope_environments();
        let _daemon = FakeDaemon::bind(&env.plain_candidates[0]);
        let launcher = FakeLauncher::never_binds();

        assert_eq!(
            ensure_in(
                &env.xdg_candidates,
                &launcher,
                RendezvousCoordination::Acquire
            ),
            EnsureOutcome::Reused
        );
        assert_eq!(launcher.spawns(), 0, "must not start a second daemon");
        assert!(
            !env.xdg_candidates[0].exists(),
            "nothing may be bound at the runtime-dir endpoint"
        );
    }

    /// JREL-004, "set then unset": a daemon bound under the runtime dir by a
    /// shell with `XDG_RUNTIME_DIR` is reused by a plain shell through the
    /// implicit runtime-dir sibling.
    #[test]
    fn plain_shell_reuses_runtime_dir_daemon_through_the_implicit_sibling() {
        let env = scope_environments();
        let _daemon = FakeDaemon::bind(&env.xdg_candidates[0]);
        let launcher = FakeLauncher::never_binds();

        assert_eq!(
            ensure_in(
                &env.plain_candidates,
                &launcher,
                RendezvousCoordination::Acquire
            ),
            EnsureOutcome::Reused
        );
        assert_eq!(launcher.spawns(), 0, "must not start a second daemon");
        assert!(
            !env.plain_candidates[0].exists(),
            "nothing may be bound at the state-home endpoint"
        );
    }

    /// JREL-004: a stale canonical socket inode (crashed daemon, no listener)
    /// beside a live sibling converges on the sibling; the stale inode is
    /// neither reused nor a reason to spawn over the live daemon.
    #[test]
    fn stale_canonical_socket_with_live_sibling_reuses_the_sibling() {
        use std::os::unix::net::UnixListener;

        let env = scope_environments();
        let stale = &env.xdg_candidates[0];
        crate::ensure_secure_runtime_dir(stale.parent().unwrap()).expect("runtime dir");
        drop(UnixListener::bind(stale).expect("bind stale socket"));
        assert!(stale.exists(), "stale inode must remain for the scenario");
        let _daemon = FakeDaemon::bind(&env.xdg_candidates[1]);
        let launcher = FakeLauncher::never_binds();

        assert_eq!(
            ensure_in(
                &env.xdg_candidates,
                &launcher,
                RendezvousCoordination::Acquire
            ),
            EnsureOutcome::Reused
        );
        assert_eq!(launcher.spawns(), 0);
    }

    /// JREL-004: a sibling endpoint is verified through the client owner-only
    /// gate before it counts. A planted symlink at the sibling path is skipped
    /// (not reused), and the daemon is still started at the canonical path.
    #[test]
    fn planted_sibling_symlink_is_not_reused() {
        use std::os::unix::fs::symlink;

        let env = scope_environments();
        let elsewhere = env.dir.path().join("elsewhere/intercept.sock");
        let _foreign = FakeDaemon::bind(&elsewhere);
        let sibling = &env.xdg_candidates[1];
        crate::ensure_secure_runtime_dir(sibling.parent().unwrap()).expect("sibling dir");
        symlink(&elsewhere, sibling).expect("plant sibling symlink");

        let count = Arc::new(AtomicUsize::new(0));
        let daemons = Arc::new(std::sync::Mutex::new(Vec::new()));
        let launcher =
            FakeSocketDaemonLauncher::new(&env.xdg_candidates[0], Duration::ZERO, &count, &daemons);

        assert_eq!(
            ensure_in(
                &env.xdg_candidates,
                &launcher,
                RendezvousCoordination::Acquire
            ),
            EnsureOutcome::Started
        );
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "a planted sibling inode is not a daemon; start at the canonical path"
        );
    }

    /// JREL-004 residual (#4432): concurrent cold starts from shells that share
    /// only state-home (disjoint runtime dirs) must converge on one daemon via
    /// the state-home coordinator and live-endpoint record.
    #[test]
    fn concurrent_cross_environment_cold_starts_converge_on_one_daemon() {
        let env = disjoint_runtime_environments();
        let count = Arc::new(AtomicUsize::new(0));
        let daemons = Arc::new(std::sync::Mutex::new(Vec::new()));

        let outcomes: Vec<EnsureOutcome> = std::thread::scope(|scope| {
            let handles: Vec<_> = [
                &env.first_set,
                &env.other_set,
                &env.first_set,
                &env.other_set,
            ]
            .into_iter()
            .map(|candidates| {
                let count = Arc::clone(&count);
                let daemons = Arc::clone(&daemons);
                scope.spawn(move || {
                    let launcher = FakeSocketDaemonLauncher::new(
                        &candidates[0],
                        Duration::from_millis(50),
                        &count,
                        &daemons,
                    );
                    ensure_in(candidates, &launcher, RendezvousCoordination::Acquire)
                })
            })
            .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "disjoint-runtime concurrent ensure must spawn exactly one daemon: {outcomes:?}"
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|o| **o == EnsureOutcome::Started)
                .count(),
            1,
            "{outcomes:?}"
        );
        assert!(
            outcomes
                .iter()
                .all(|o| matches!(o, EnsureOutcome::Started | EnsureOutcome::Reused)),
            "{outcomes:?}"
        );
        drop(daemons);
    }

    /// #4432: candidate sets that share only state-home contend on the
    /// state-home lock, not `min(runtime)`.
    #[test]
    fn disjoint_runtime_sets_contend_on_state_home_lock() {
        use std::sync::mpsc;

        let env = disjoint_runtime_environments();
        let first = acquire_daemon_rendezvous_repair_lock_for_socket_candidates(&env.first_set)
            .expect("first runtime repair lock");
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let other_set = env.other_set.clone();
        let waiter = std::thread::spawn(move || {
            let second = acquire_daemon_rendezvous_repair_lock_for_socket_candidates(&other_set)
                .expect("other runtime repair lock");
            acquired_tx.send(second).expect("report lock acquisition");
        });

        assert!(
            acquired_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "disjoint runtime dirs that share state-home must contend on one lock",
        );
        let state_lock = env.state_home.join("intercept.rendezvous-repair.lock");
        assert!(
            state_lock.exists(),
            "coordinator lock must live at state-home, not min(runtime): {}",
            state_lock.display()
        );
        let runtime_lock = env.first_set[0]
            .parent()
            .unwrap()
            .join("intercept.rendezvous-repair.lock");
        assert!(
            !runtime_lock.exists(),
            "runtime dir must not be the coordinator: {}",
            runtime_lock.display()
        );
        drop(first);
        drop(
            acquired_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("second lock"),
        );
        waiter.join().expect("repair-lock waiter");
    }

    /// JREL-004 residual: two lexical candidates that are one physical socket
    /// (ancestor alias of state-home) are one endpoint, never a false conflict.
    #[test]
    fn ancestor_alias_of_state_home_is_one_endpoint_not_a_conflict() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().expect("tempdir");
        let physical_home = dir.path().join("a-home/u");
        let alias_home = dir.path().join("z-home");
        let state_dir = physical_home.join(".local/state/anvil");
        std::fs::create_dir_all(&state_dir).expect("state directory");
        symlink(&physical_home, &alias_home).expect("ancestor alias");

        let physical_socket = state_dir.join("intercept.sock");
        let aliased_socket = alias_home.join(".local/state/anvil/intercept.sock");
        let _daemon = FakeDaemon::bind(&physical_socket);
        assert!(
            crate::ipc::unix_sockets_are_same_physical(&physical_socket, &aliased_socket),
            "aliased state-home must be one socket inode"
        );

        let launcher = FakeLauncher::never_binds();
        let candidates = vec![physical_socket.clone(), aliased_socket];
        assert_eq!(
            ensure_in(&candidates, &launcher, RendezvousCoordination::Acquire),
            EnsureOutcome::Reused
        );
        assert_eq!(
            launcher.spawns(),
            0,
            "one physical socket is not a conflict"
        );
    }

    /// JREL-004: intentionally isolated `ANVIL_HOME` installations are
    /// separate scopes with exclusive single-candidate sets; each starts and
    /// keeps its own daemon.
    #[test]
    fn isolated_homes_start_distinct_daemons() {
        let dir = tempfile::tempdir().expect("tempdir");
        let count = Arc::new(AtomicUsize::new(0));
        let daemons = Arc::new(std::sync::Mutex::new(Vec::new()));
        for home in ["home-a", "home-b"] {
            let candidates = crate::ipc::resolve_socket_connect_candidates_with_env(
                Some(dir.path().join(home).into_os_string()),
                Some(dir.path().join("runtime").into_os_string()),
                Some(dir.path().join("user-home").into_os_string()),
                None,
            )
            .expect("isolated home candidates");
            assert_eq!(
                candidates.len(),
                1,
                "ANVIL_HOME is exclusive: {candidates:?}"
            );
            let launcher =
                FakeSocketDaemonLauncher::new(&candidates[0], Duration::ZERO, &count, &daemons);
            assert_eq!(
                ensure_in(&candidates, &launcher, RendezvousCoordination::Acquire),
                EnsureOutcome::Started,
                "{home} must start its own daemon"
            );
        }
        assert_eq!(count.load(Ordering::SeqCst), 2);
        assert_eq!(daemons.lock().unwrap().len(), 2);
    }

    /// ADR-060: an isolated `ANVIL_HOME` coordinator stays in that prefix and
    /// never falls through to default-scope state-home.
    #[test]
    fn isolated_home_coordinator_stays_in_the_prefix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let isolated = dir.path().join("anvil-home");
        let user_home = dir.path().join("user-home");
        let candidates = crate::ipc::resolve_socket_connect_candidates_with_env(
            Some(isolated.clone().into_os_string()),
            Some(dir.path().join("runtime").into_os_string()),
            Some(user_home.clone().into_os_string()),
            None,
        )
        .expect("isolated home candidates");
        assert_eq!(candidates.len(), 1, "{candidates:?}");
        let _lock = acquire_daemon_rendezvous_repair_lock_for_socket_candidates(&candidates)
            .expect("isolated coordinator");
        assert!(
            isolated.join("intercept.rendezvous-repair.lock").exists(),
            "isolated ANVIL_HOME must own its coordinator"
        );
        assert!(
            !user_home
                .join(".local/state/anvil/intercept.rendezvous-repair.lock")
                .exists(),
            "isolated coordinator must not be default state-home"
        );
    }

    /// CIB-382: two doctor processes can see the same runtime candidates in
    /// opposite canonical order. They must enter rendezvous repair through one
    /// order-independent lock instead of each retaining the other's start lock.
    #[test]
    fn rendezvous_repair_lock_serialises_reversed_candidate_order() {
        use std::sync::mpsc;

        let dir = tempfile::tempdir().expect("tempdir");
        let runtime_socket = dir.path().join("runtime/anvil/intercept.sock");
        let state_socket = dir.path().join("home/.local/state/anvil/intercept.sock");
        let forward = vec![runtime_socket.clone(), state_socket.clone()];
        let reversed = vec![state_socket, runtime_socket];

        let first = acquire_daemon_rendezvous_repair_lock_for_socket_candidates(&forward)
            .expect("first repair lock");
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let waiter = std::thread::spawn(move || {
            let second = acquire_daemon_rendezvous_repair_lock_for_socket_candidates(&reversed)
                .expect("reversed repair lock");
            acquired_tx
                .send(second)
                .expect("report reversed lock acquisition");
        });

        assert!(
            acquired_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "reversed canonical order must contend on the same repair lock",
        );
        drop(first);
        let second = acquired_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("reversed repair should proceed after the first lock releases");
        drop(second);
        waiter.join().expect("repair-lock waiter");
    }

    /// CIB-382/C-003: raw path sorting is not a shared identity when one
    /// process reaches state home through an ancestor alias. Both physical
    /// candidate sets must still choose the same repair coordinator.
    #[test]
    fn rendezvous_repair_lock_normalises_ancestor_aliases() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        use std::sync::mpsc;

        let dir = tempfile::tempdir().expect("tempdir");
        let physical_home = dir.path().join("a-home/u");
        let alias_home = dir.path().join("z-home");
        let runtime_dir = dir.path().join("m-runtime/anvil");
        let state_dir = physical_home.join(".local/state/anvil");
        std::fs::create_dir_all(&runtime_dir).expect("runtime directory");
        std::fs::create_dir_all(&state_dir).expect("state directory");
        for candidate_dir in [&runtime_dir, &state_dir] {
            std::fs::set_permissions(candidate_dir, std::fs::Permissions::from_mode(0o700))
                .expect("owner-only candidate directory");
        }
        symlink(&physical_home, &alias_home).expect("ancestor alias");

        let runtime_socket = runtime_dir.join("intercept.sock");
        let physical_state_socket = state_dir.join("intercept.sock");
        let aliased_state_socket = alias_home.join(".local/state/anvil/intercept.sock");
        let physical_candidates = vec![runtime_socket.clone(), physical_state_socket];
        let aliased_reversed = vec![aliased_state_socket, runtime_socket];

        let first =
            acquire_daemon_rendezvous_repair_lock_for_socket_candidates(&physical_candidates)
                .expect("first repair lock");
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let waiter = std::thread::spawn(move || {
            let second =
                acquire_daemon_rendezvous_repair_lock_for_socket_candidates(&aliased_reversed)
                    .expect("aliased repair lock");
            acquired_tx.send(second).expect("report lock acquisition");
        });

        assert!(
            acquired_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "ancestor aliases for the same directory must contend on one repair lock",
        );
        drop(first);
        drop(
            acquired_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("second lock"),
        );
        waiter.join().expect("repair-lock waiter");
    }

    /// #4432 review: `XDG_RUNTIME_DIR` ending in `.local/state` must not beat
    /// the actual `$HOME/.local/state/anvil` candidate.
    #[test]
    fn coordinator_prefers_home_state_over_runtime_dir_suffix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("home");
        let deceptive_runtime = dir.path().join("xdg/.local/state");
        let home_state = home.join(".local/state/anvil");
        let runtime_socket = deceptive_runtime.join("anvil/intercept.sock");
        let home_socket = home_state.join("intercept.sock");
        let selected = select_rendezvous_coordinator_dir_from(
            &[runtime_socket, home_socket],
            Some(home.as_os_str()),
        )
        .expect("coordinator");
        assert_eq!(selected, home_state);
    }

    /// In-process tests do not mutate process `HOME`; a unique
    /// `.local/state/anvil` suffix still selects state-home.
    #[test]
    fn coordinator_keeps_unique_suffix_when_home_is_elsewhere() {
        let dir = tempfile::tempdir().expect("tempdir");
        let runtime_socket = dir.path().join("xdg-a/anvil/intercept.sock");
        let state_socket = dir.path().join("home/.local/state/anvil/intercept.sock");
        let elsewhere = dir.path().join("other-home");
        let selected = select_rendezvous_coordinator_dir_from(
            &[runtime_socket, state_socket.clone()],
            Some(elsewhere.as_os_str()),
        )
        .expect("coordinator");
        assert_eq!(selected, state_socket.parent().unwrap());
    }

    /// When `$HOME` is unavailable, colliding suffixes fall back to the first
    /// parent rather than an arbitrary `.local/state/anvil` match.
    #[test]
    fn coordinator_falls_back_to_first_parent_when_home_is_unavailable_and_suffixes_collide() {
        let dir = tempfile::tempdir().expect("tempdir");
        let runtime_socket = dir.path().join("xdg/.local/state/anvil/intercept.sock");
        let home_socket = dir.path().join("home/.local/state/anvil/intercept.sock");
        let selected =
            select_rendezvous_coordinator_dir_from(&[runtime_socket.clone(), home_socket], None)
                .expect("coordinator");
        assert_eq!(selected, runtime_socket.parent().unwrap());
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
