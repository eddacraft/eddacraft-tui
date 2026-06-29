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

// DSV-003 ingest-spine modules are Unix-only (nix/openat2/std::os::unix) — the
// daemon's save-time read path is a Linux/macOS concern; Windows named-pipe
// `validate_paths` is tracked separately as out of scope. Gated so the
// `x86_64-pc-windows-msvc` build (which lacks the `cfg(unix)` `nix` dep) stays
// green.
// DSV-010b: the operator antipattern-config loader is platform-neutral (it reads
// through `confinement::read_trusted`, which has a `cfg(not(unix))` arm); only
// its tests are `cfg(all(test, unix))`. `run_foreground` loads it on Windows too
// now that the save-time path is served there.
#[cfg(any(unix, windows))]
pub mod antipattern_config;
pub mod assurance;
pub mod auth;
pub mod broadcaster;
// DSV-010a / ADR-069: `change_class` is no longer Unix-gated as a whole — its
// `CanonicalChange` enum is platform-neutral (the verdict spine needs it on
// Windows); the inode-based identity/classification inside it stays `cfg(unix)`.
pub mod change_class;
pub mod config;
pub mod confinement;
pub mod dos;
pub mod egress_consent;
pub mod embedded;
pub mod enforcement;
pub mod ensure;
pub mod fanout;
pub mod fence;
pub mod full_scan_executor;
pub mod interrupt;
pub mod ipc;
pub mod kernel_cache;
pub mod kindling_observation;
pub mod latency;
pub mod midedit;
#[cfg(unix)]
pub mod path_safety;
pub mod rate_window;
pub mod registration_store;
pub mod registry;
pub mod rule_cache;
#[cfg(unix)]
pub mod snapshot_io;
// DSV-010b / ADR-070 Stage 2: the save-time verbs are served on both Unix and
// Windows. `save_time` / `workspace_admission` code against the neutral
// [`workspace_anchor::WorkspaceAnchor`] (Unix dirfd / the Windows ADR-068 guard)
// rather than a bare fd, so they are no longer Unix-only.
#[cfg(any(unix, windows))]
pub mod save_time;
pub mod status;
pub mod store_io;
pub mod tag_env;
pub mod telemetry;
pub mod unregistered;
pub mod validate_paths;
pub mod watcher;
#[cfg(any(unix, windows))]
pub mod workspace_admission;
/// The platform-neutral workspace read anchor (Unix dirfd / Windows directory
/// handle) the verdict path codes against (DSV-010b / ADR-070).
#[cfg(any(unix, windows))]
pub mod workspace_anchor;
pub mod workspace_pool;

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
    /// ACTMO-014: the durable-registration persistence shadow. `None` for
    /// embedded-mode / test dispatchers that do not persist; production wires
    /// it in `run_foreground`. A durable (activation-spine) register upserts
    /// here and a durable unregister prunes here, so the membership set
    /// survives a daemon restart.
    registration_store: Option<Arc<registration_store::RegistrationStore>>,
}

impl RegistryDispatcher {
    fn new(registry: Arc<SessionRegistry>, fence_store: Arc<fence::FenceStore>) -> Self {
        Self {
            registry,
            fence_store,
            registration_store: None,
        }
    }

    /// ACTMO-014: attach the durable-registration store so durable
    /// registrations are persisted under `ANVIL_HOME`.
    #[must_use]
    fn with_registration_store(
        mut self,
        store: Arc<registration_store::RegistrationStore>,
    ) -> Self {
        self.registration_store = Some(store);
        self
    }

    /// ACTMO-014: persist a successful durable registration. Best-effort — a
    /// disk failure logs loudly but does not fail the registration (the
    /// worktree is protected for this daemon's lifetime regardless; the
    /// reload-on-start net covers the normal case). Non-durable (live agent)
    /// sessions are never persisted.
    fn persist_durable_register(
        &self,
        id: &anvil_intercept_proto::SessionId,
        worktree: &Path,
        agent_tag: Option<&anvil_intercept_proto::session::AgentTag>,
    ) {
        let Some(store) = self.registration_store.as_ref() else {
            return;
        };
        if !agent_tag.is_some_and(anvil_intercept_proto::session::AgentTag::is_durable_membership) {
            return;
        }
        // Read the canonical path the registry actually stored (review F2),
        // rather than re-canonicalising the raw wire path — the latter could
        // diverge if the directory is removed in the window after the registry
        // accepted the session. `durable_worktree_for` returns `None` only if
        // the session was already evicted/unregistered, in which case there is
        // nothing to persist.
        let Some(canonical) = self.registry.durable_worktree_for(id) else {
            return;
        };
        let record =
            registration_store::RegistrationRecord::new(id.clone(), canonical, agent_tag.cloned());
        if let Err(err) = store.upsert(record) {
            tracing::error!(
                target: "anvil_intercept::registration",
                error = %err,
                worktree = %worktree.display(),
                "failed to persist durable worktree registration — it will not \
                 survive a daemon restart",
            );
        }
    }

    /// ACTMO-014: prune a durably-registered worktree from the persisted store
    /// once its last durable session has been unregistered. Best-effort.
    fn persist_durable_unregister(&self, worktree: &Path) {
        let Some(store) = self.registration_store.as_ref() else {
            return;
        };
        if let Err(err) = store.remove(worktree) {
            tracing::error!(
                target: "anvil_intercept::registration",
                error = %err,
                worktree = %worktree.display(),
                "failed to prune durable worktree registration from the store",
            );
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
        SessionDispatcher::register(self.registry.as_ref(), id, worktree, agent_tag, lineage)?;
        // ACTMO-014: persist durable membership after the registry accepts it.
        self.persist_durable_register(id, worktree, agent_tag);
        Ok(())
    }

    fn heartbeat(&self, id: &anvil_intercept_proto::SessionId) -> Result<(), RegistryError> {
        SessionDispatcher::heartbeat(self.registry.as_ref(), id)
    }

    fn unregister(&self, id: &anvil_intercept_proto::SessionId) -> Result<bool, RegistryError> {
        // ACTMO-014: capture the durable worktree (if any) BEFORE removal so we
        // can prune the persisted store once the last durable session leaves.
        let durable_worktree = self.registry.durable_worktree_for(id);
        let removed = SessionDispatcher::unregister(self.registry.as_ref(), id)?;
        if removed
            && let Some(worktree) = durable_worktree
            && !self.registry.is_registered(&worktree)
        {
            self.persist_durable_unregister(&worktree);
        }
        Ok(removed)
    }

    fn list(&self) -> Vec<anvil_intercept_proto::SessionRecord> {
        SessionDispatcher::list(self.registry.as_ref())
    }

    fn report_process(
        &self,
        id: &anvil_intercept_proto::SessionId,
        child_pid: u32,
        child_pid_starttime: u64,
        peer_pid: u32,
    ) -> Result<(), RegistryError> {
        SessionDispatcher::report_process(
            self.registry.as_ref(),
            id,
            child_pid,
            child_pid_starttime,
            peer_pid,
        )
    }
}

struct DaemonState {
    registry: Arc<SessionRegistry>,
    fence_store: Arc<fence::FenceStore>,
    fences: Arc<fence::FenceState>,
    /// MLP2-071 (INTD-015 wire-up): per-startup fan-out filter,
    /// constructed once with the resolved cross-session policy and a
    /// fresh per-startup HMAC salt. The fan-out is the authorisation +
    /// redaction core; the [`Self::broadcaster`] wraps it with the
    /// per-subscriber delivery channels. Held here so the
    /// `daemon_state_constructs_fanout_with_configured_cross_session_policy`
    /// pin can assert the operator-configured policy reached the
    /// instance. The production reader is [`Self::broadcaster`] (which
    /// holds its own clone of this `Arc`); this field is read only by
    /// that pin, so `#[allow(dead_code)]` is stamped for non-test builds.
    #[allow(dead_code)]
    fanout: Arc<fanout::Fanout>,
    /// MLP2-071 Phase 2: the telemetry broadcaster built over
    /// [`Self::fanout`]. `run_foreground` clones this into the IPC
    /// listener so `SubscribeTelemetry` connections register against it,
    /// and it is the handle a producer (DSV-044's transition emitters)
    /// calls `broadcast` on. Closes the "no reader" half of #1722: the
    /// fan-out is no longer constructed-but-unread.
    broadcaster: Arc<broadcaster::TelemetryBroadcaster>,
}

impl DaemonState {
    fn new(
        fence_store: fence::FenceStore,
        fences: fence::FenceState,
        enforcement_config: &config::Resolved,
    ) -> anyhow::Result<Self> {
        // MLP2-024: build the registry with the operator-configured
        // per-worktree cap. Pre-fix the cap shipped at the
        // compile-time default regardless of `.anvil.yaml`; the
        // `daemon_config_wired::run_foreground_applies_session_per_worktree_cap_from_config`
        // regression pins this chained builder call. The
        // `with_unregister_hook` companion (MLP2-057) stays unwired —
        // its only consumer is `RuleSetCache::invalidate`, which
        // requires the cache `run_foreground` does not construct
        // until MLP2-014 lands.
        let registry = Arc::new(
            SessionRegistry::new()
                .with_per_worktree_cap(enforcement_config.session_per_worktree_max),
        );

        // MLP2-071 (INTD-015 wire-up): mint a per-startup HMAC salt
        // (closes v0.6.0-beta-security-note §H2) and construct the
        // fan-out with the operator-configured cross-session policy.
        // A getrandom failure here is fatal — there is no acceptable
        // fallback to a deterministic salt for the §H2 redaction
        // primitive, so we surface the OS-RNG error instead of
        // silently degrading.
        let redaction_key = fanout::TelemetryRedactionKey::new_random().map_err(|err| {
            anyhow::anyhow!("mint per-startup telemetry redaction salt for INTD-015 fanout: {err}")
        })?;
        let fence_store = Arc::new(fence_store);
        let fences = Arc::new(fences);
        fence_store.cache_loaded_state(&fences);
        // MLP2-071 D6: the resolver needs the live fence-store cache to answer
        // `is_degraded_origin`, so startup fences and newly written spoof
        // fences affect fanout routing without disk I/O under the route lock.
        let resolver =
            fanout::RegistryOwnershipResolver::new(Arc::clone(&registry), Arc::clone(&fence_store));
        let fanout = Arc::new(fanout::Fanout::with_cross_session_policy_and_key(
            Box::new(resolver),
            enforcement_config.cross_session_policy(),
            redaction_key,
        ));
        // MLP2-071 Phase 2: the broadcaster shares the fan-out so the
        // IPC subscriber surface and any producer route through the same
        // operator-configured policy + redaction salt.
        let broadcaster = Arc::new(broadcaster::TelemetryBroadcaster::new(Arc::clone(&fanout)));

        fence_store.set_telemetry(Arc::clone(&registry), Arc::clone(&broadcaster));

        Ok(Self {
            registry,
            fence_store,
            fences,
            fanout,
            broadcaster,
        })
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
    /// INTD-016 / MLP2-024: the resolved enforcement config. Defaults
    /// to [`config::Resolved::default`] (the no-config baseline) so
    /// existing callers — tests, embedded mode, the legacy binary
    /// entry — keep working. `main.rs` and `anvil-cli` load via
    /// [`config::Resolved::load`] and pass the result through
    /// [`Self::with_enforcement_config`].
    ///
    /// Non-`Option` by design: keeping the field always-present
    /// makes the wire-up in [`run_foreground`] unconditional, which
    /// is what the post-#1671 audit closure rule asks for — see the
    /// regression test `daemon_config_wired::*` for the contract.
    enforcement_config: config::Resolved,
    /// DSV-005: the dependency-inverted kernel-backed symbol parser the daemon
    /// enriches its verdict with (a Messaging Gateway — the daemon never links
    /// tree-sitter). `None` ⇒ verdicts stay `Partial`. `anvil-cli` injects the
    /// real impl via [`Self::with_symbol_parser`]. DSV-010b: the plumbing is
    /// symmetric on Windows (the save-time path is served there now); the
    /// tree-sitter injection itself stays Unix in `anvil-cli` for now, so the
    /// Windows daemon currently runs parser-less (`Partial` verdicts) — the
    /// documented degraded mode Unix also uses when no parser is wired.
    #[cfg(any(unix, windows))]
    symbol_parser: Option<Arc<dyn save_time::SymbolParser>>,
    /// USAGE-004: the command-invocation usage producer the daemon emits
    /// `command.invoked` rows through. `None` ⇒ usage export off (the
    /// default; tests and embedded mode). `anvil-cli` injects a real
    /// NDJSON-backed emitter via [`Self::with_usage_emitter`].
    #[cfg(any(unix, windows))]
    usage_emitter: Option<Arc<kindling_observation::CommandInvokedEmitter>>,
    /// DPO-001: the save-time `gate_evaluated` producer the daemon emits a
    /// row through after every `validate_paths` verdict. `None` ⇒ no
    /// save-time observation export (the default; tests, embedded mode).
    /// `anvil-cli` injects an NDJSON-backed emitter via
    /// [`Self::with_observation_emitter`].
    #[cfg(any(unix, windows))]
    observation_emitter: Option<Arc<kindling_observation::SaveTimeObservationEmitter>>,
    /// DPO-002: the Kindling sink the fence surface emits a
    /// `constraint_applied` row through on every successful engage. `None`
    /// ⇒ no fence observation export (the default). `anvil-cli` injects the
    /// same shared non-blocking sink the save-time emitter writes through
    /// via [`Self::with_observation_sink`].
    #[cfg(any(unix, windows))]
    observation_sink: Option<Arc<dyn kindling_observation::KindlingObservationSink>>,
    /// DPO-002 (council C): whether the fence `constraint_applied` row may
    /// carry the absolute worktree path. Set alongside [`Self::observation_sink`]
    /// by [`Self::with_observation_sink`]; defaults to `false` (path
    /// suppressed) so a host that wires a sink without opting into paths
    /// never leaks one.
    #[cfg(any(unix, windows))]
    observation_include_paths: bool,
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
            enforcement_config: config::Resolved::default(),
            #[cfg(any(unix, windows))]
            symbol_parser: None,
            #[cfg(any(unix, windows))]
            usage_emitter: None,
            #[cfg(any(unix, windows))]
            observation_emitter: None,
            #[cfg(any(unix, windows))]
            observation_sink: None,
            #[cfg(any(unix, windows))]
            observation_include_paths: false,
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
            enforcement_config: config::Resolved::default(),
            symbol_parser: None,
            usage_emitter: None,
            observation_emitter: None,
            observation_sink: None,
            observation_include_paths: false,
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
            enforcement_config: config::Resolved::default(),
            symbol_parser: None,
            usage_emitter: None,
            observation_emitter: None,
            observation_sink: None,
            observation_include_paths: false,
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

    /// Install the resolved enforcement config. `main.rs` and
    /// `anvil-cli` load via [`config::Resolved::load`] at daemon
    /// startup; tests construct a [`config::Resolved`] inline to drive
    /// specific cap / limit / mode values.
    ///
    /// Wires two previously-inert builders in [`run_foreground`]:
    ///
    /// * `SessionRegistry::with_per_worktree_cap` (MLP2-024 — reads
    ///   `enforcement.session.per_worktree_max`).
    /// * `IpcListener::with_limits` (INTD-016 — reads
    ///   `enforcement.dos.*`).
    ///
    /// MLP2-071 Phase 1: `Fanout` / cross-session telemetry policy
    /// (INTD-015) is **now** wired here via [`DaemonState::new`] —
    /// `run_foreground` constructs a `Fanout` with the operator-
    /// configured policy and a per-startup HMAC salt, closing the
    /// literal "configured-but-ignored" half of GH issue #1722. The
    /// `daemon_state_constructs_fanout_with_configured_cross_session_policy`
    /// regression pin proves the policy flows from
    /// `Resolved::cross_session_policy()` through to the constructed
    /// instance. Phase 2 (IPC subscriber surface + production
    /// broadcaster) is the separate slice that makes the configured
    /// flag operator-visible end-to-end.
    ///
    /// The post-#1671 audit closed the gap where each of those
    /// builders had its definition + doc-comment claiming the daemon
    /// wires it up, but zero production callers. The regression suite
    /// in `crates/anvil-intercept/tests/daemon_config_wired.rs` pins
    /// the wire-up so a future refactor trips a test rather than
    /// silently resurrecting the bug.
    #[must_use]
    pub fn with_enforcement_config(mut self, enforcement_config: config::Resolved) -> Self {
        self.enforcement_config = enforcement_config;
        self
    }

    /// DSV-005: inject the kernel-backed [`save_time::SymbolParser`]. `anvil-cli`
    /// calls this (it deps both the kernel and the daemon, so the tree-sitter
    /// parser links into the *binary*, never the `anvil-intercept` crate —
    /// ADR-064 holds). Without it the daemon answers `validate_paths` with safe
    /// `Partial` verdicts only.
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_symbol_parser(mut self, parser: Arc<dyn save_time::SymbolParser>) -> Self {
        self.symbol_parser = Some(parser);
        self
    }

    /// USAGE-004: inject the command-invocation usage producer. `anvil-cli`
    /// builds an NDJSON-backed emitter (see `usage::daemon_usage_emitter`)
    /// and wires it here so the daemon records `command.invoked` rows for
    /// allowlisted JSON-RPC methods. Without it the daemon serves
    /// normally and records no usage rows (the default).
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_usage_emitter(
        mut self,
        emitter: Arc<kindling_observation::CommandInvokedEmitter>,
    ) -> Self {
        self.usage_emitter = Some(emitter);
        self
    }

    /// DPO-001: inject the save-time `gate_evaluated` producer. `anvil-cli`
    /// builds an NDJSON-backed emitter (see
    /// `usage::daemon_observation_producers`) and wires it here so the
    /// daemon records a row after every `validate_paths` verdict (pass and
    /// fail). Without it the daemon serves normally and records no
    /// save-time rows (the default). Mirrors [`Self::with_usage_emitter`].
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_observation_emitter(
        mut self,
        emitter: Arc<kindling_observation::SaveTimeObservationEmitter>,
    ) -> Self {
        self.observation_emitter = Some(emitter);
        self
    }

    /// DPO-002: inject the Kindling sink the fence surface emits
    /// `constraint_applied` rows through on every successful engage.
    /// `anvil-cli` passes the same shared non-blocking sink the save-time
    /// emitter writes through, so both producers fan into one drain
    /// thread. Without it fence engages produce no observation row (the
    /// default). Mirrors [`Self::with_usage_emitter`].
    ///
    /// `include_paths` (council C) gates whether the engage row carries the
    /// absolute worktree path; `anvil-cli` passes the same value the
    /// save-time emitter derives from `ANVIL_OBSERVATION_INCLUDE_PATHS` so
    /// both surfaces honour one privacy posture.
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_observation_sink(
        mut self,
        sink: Arc<dyn kindling_observation::KindlingObservationSink>,
        include_paths: bool,
    ) -> Self {
        self.observation_sink = Some(sink);
        self.observation_include_paths = include_paths;
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

    /// ACTMO-014: the durable registration store sits beside the fence store
    /// under `ANVIL_HOME`, so any fence-store override (tests, embedded mode)
    /// keeps the two co-located in one state directory.
    fn registration_store_path(&self) -> Result<PathBuf> {
        Ok(self
            .fence_store_path()?
            .with_file_name("registered-worktrees.json"))
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
    default_pid_file_path_from(
        anvil_home_prefix(),
        non_empty_env("XDG_RUNTIME_DIR"),
        if cfg!(windows) {
            non_empty_env("LOCALAPPDATA")
        } else {
            None
        },
        non_empty_env("HOME").or_else(|| non_empty_env("USERPROFILE")),
    )
}

/// Pure resolver for [`default_pid_file_path`] — takes the candidate roots
/// explicitly so it unit-tests without mutating the process environment.
fn default_pid_file_path_from(
    anvil_home: Option<PathBuf>,
    xdg_runtime_dir: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf> {
    // DISTRIB-006 (ADR-060): `ANVIL_HOME` re-roots the PID file directly under the
    // prefix, alongside the daemon socket, so a candidate daemon's PID-file
    // exclusive-create (ADR-036) does not collide with production's. Precedence
    // over the runtime dir; unset = byte-for-byte default below.
    if let Some(prefix) = anvil_home {
        return Ok(prefix.join("intercept.pid"));
    }

    if let Some(runtime_dir) = xdg_runtime_dir {
        return Ok(runtime_dir.join("anvil").join("intercept.pid"));
    }

    if let Some(local_app_data) = local_app_data {
        return Ok(local_app_data.join("anvil").join("intercept.pid"));
    }

    let home = home.context("cannot resolve home directory for anvil intercept PID file")?;
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

/// DISTRIB-006 (ADR-060): the install-root prefix from a non-empty `ANVIL_HOME`,
/// absolutised against the current directory if relative. anvil-cli's resolver
/// absolutises the same way, so the CLI client and the separately-spawned daemon
/// agree on the socket/PID path even when `ANVIL_HOME` is set as a relative env
/// var (`ANVIL_HOME` is expected absolute in practice; this only guards the
/// relative case). Returns `None` for unset/empty (platform default applies).
pub(crate) fn anvil_home_prefix() -> Option<PathBuf> {
    let raw = env::var_os("ANVIL_HOME").filter(|v| !v.is_empty())?;
    // Mirror the CLI resolver (`install_root::resolve_install_root_from`): a UTF-8
    // whitespace-only value is treated as unset, so the daemon and CLI agree on
    // the socket/PID path when `ANVIL_HOME` is accidentally exported blank.
    // Non-UTF-8 values can't be trimmed and are taken as-is.
    if raw.to_str().is_some_and(|s| s.trim().is_empty()) {
        return None;
    }
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        Some(p)
    } else {
        // Best effort: if cwd is unavailable, fall back to the raw relative path
        // rather than dropping the override entirely.
        Some(env::current_dir().map_or_else(|_| p.clone(), |cwd| cwd.join(&p)))
    }
}

#[derive(Debug)]
struct PidFileGuard {
    _lock: File,
    path: PathBuf,
    identity: PidFileIdentity,
}

impl PidFileGuard {
    fn acquire(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            ensure_secure_runtime_dir(parent)?;
        }

        let lock = acquire_pid_file_lock(path)?;

        match Self::create_identity(path) {
            Ok(identity) => Ok(Self::new(path, identity, lock)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                recover_stale_pid_file(path)?;
                Self::create_identity(path)
                    .map(|identity| Self::new(path, identity, lock))
                    .with_context(|| format!("failed to re-create PID file {}", path.display()))
            }
            Err(err) => {
                Err(err).with_context(|| format!("failed to create PID file {}", path.display()))
            }
        }
    }

    fn create_identity(path: &Path) -> std::io::Result<PidFileIdentity> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let record = write_pid_record(&mut file)?;
        PidFileIdentity::from_file(&file, record)
    }

    fn new(path: &Path, identity: PidFileIdentity, lock: File) -> Self {
        Self {
            _lock: lock,
            path: path.to_path_buf(),
            identity,
        }
    }
}

fn acquire_pid_file_lock(path: &Path) -> Result<File> {
    let lock_path = path.with_extension("pid.lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .with_context(|| format!("failed to open PID file lock {}", lock_path.display()))?;

    lock.try_lock().with_context(|| {
        format!(
            "anvil intercept daemon is already running or PID file is locked at {}; \
             run `anvil intercept stop` to stop it, then retry",
            path.display()
        )
    })?;
    Ok(lock)
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

pub(crate) fn ensure_secure_runtime_dir(path: &Path) -> Result<()> {
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
                "anvil intercept daemon is already running or PID file cannot be proven stale \
                 at {}; run `anvil intercept stop` to stop it, then retry",
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

    match (recorded_start_time, process_start_time(pid)) {
        (Some(expected), Some(actual)) if expected == actual => ExistingPidStatus::Live,
        (Some(_), Some(_)) => ExistingPidStatus::Stale,
        _ => ExistingPidStatus::Unknown,
    }
}

/// Outcome of an `anvil intercept stop` request (V060F-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// A stop request was delivered to a live daemon with this PID. Unix uses
    /// SIGTERM so the daemon can flush fence state, unbind the IPC listener,
    /// and remove its own PID file. Windows terminates the headless daemon
    /// process and the stop primitive clears the PID file.
    Signalled { pid: u32 },
    /// No PID file was present — the daemon is not running. Idempotent no-op.
    NotRunning,
    /// The PID file pointed at a process that is no longer alive; the stale
    /// file was removed. Idempotent no-op as far as the daemon is concerned.
    StaleCleared { pid: u32 },
}

/// The pure decision for [`stop_daemon_at`]: what to do given the PID-file
/// contents, with liveness classification injected so the branch selection is
/// unit-testable without real processes or signals (mirrors the
/// inject-the-effect style used by [`default_pid_file_path_from`] and the
/// `ensure` primitive).
#[derive(Debug, PartialEq, Eq)]
enum StopPlan {
    NotRunning,
    Signal { pid: u32 },
    ClearStale { pid: u32 },
    Malformed,
    Unproven,
}

fn parse_pid_record(record: &str) -> Option<u32> {
    record
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
}

fn plan_stop(record: Option<&str>, classify: impl Fn(&str) -> ExistingPidStatus) -> StopPlan {
    let Some(record) = record else {
        return StopPlan::NotRunning;
    };
    let Some(pid) = parse_pid_record(record) else {
        return StopPlan::Malformed;
    };
    match classify(record) {
        ExistingPidStatus::Live => StopPlan::Signal { pid },
        ExistingPidStatus::Stale => StopPlan::ClearStale { pid },
        ExistingPidStatus::Unknown => StopPlan::Unproven,
    }
}

/// Stop the per-user intercept daemon recorded in the [`default_pid_file_path`].
/// Unix sends SIGTERM and lets the daemon's `run_foreground` handler flush
/// state, unbind the IPC listener, and remove the PID file. Windows terminates
/// the headless daemon process and then removes the PID file (ACTMO-008 /
/// V060F-002).
///
/// Idempotent: a missing PID file yields [`StopOutcome::NotRunning`]; a PID
/// file whose process has already exited yields [`StopOutcome::StaleCleared`]
/// after removing the stale file. Neither is an error.
#[cfg(any(unix, windows))]
pub fn request_daemon_stop() -> Result<StopOutcome> {
    let path = default_pid_file_path()?;
    stop_daemon_at(&path)
}

#[cfg(any(unix, windows))]
fn stop_daemon_at(path: &Path) -> Result<StopOutcome> {
    let record = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                anyhow::bail!("refusing symlink PID file {}", path.display());
            }
            Some(
                fs::read_to_string(path)
                    .with_context(|| format!("failed to read PID file {}", path.display()))?,
            )
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(anyhow::Error::new(err))
                .with_context(|| format!("failed to inspect PID file {}", path.display()));
        }
    };
    match plan_stop(record.as_deref(), existing_pid_status) {
        StopPlan::NotRunning => Ok(StopOutcome::NotRunning),
        StopPlan::Signal { pid } => {
            stop_live_daemon(pid, path)?;
            Ok(StopOutcome::Signalled { pid })
        }
        StopPlan::ClearStale { pid } => {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(anyhow::Error::new(err)).with_context(|| {
                        format!("failed to remove stale PID file {}", path.display())
                    });
                }
            }
            Ok(StopOutcome::StaleCleared { pid })
        }
        StopPlan::Malformed => anyhow::bail!(
            "PID file {} is malformed (no parseable daemon PID); remove it manually if the \
             daemon is not running",
            path.display(),
        ),
        StopPlan::Unproven => anyhow::bail!(
            "PID file {} cannot be proven to identify the live daemon; refusing to signal it. \
             Remove it manually if the daemon is not running",
            path.display(),
        ),
    }
}

#[cfg(unix)]
fn stop_live_daemon(pid: u32, _path: &Path) -> Result<()> {
    send_sigterm(pid)
}

#[cfg(windows)]
fn stop_live_daemon(pid: u32, path: &Path) -> Result<()> {
    anvil_intercept_win32::terminate_process(pid)
        .with_context(|| format!("failed to stop daemon PID {pid}"))?;
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(anyhow::Error::new(err))
                .with_context(|| format!("failed to remove PID file {}", path.display()));
        }
    }
    Ok(())
}

/// Send SIGTERM to `pid`. A process that has already exited (`ESRCH`) between
/// the liveness classification and the signal is treated as success — the
/// daemon is gone either way.
#[cfg(unix)]
fn send_sigterm(pid: u32) -> Result<()> {
    let raw = i32::try_from(pid)
        .map_err(|_| anyhow::anyhow!("daemon PID {pid} is out of range for signalling"))?;
    match kill(Pid::from_raw(raw), Some(nix::sys::signal::Signal::SIGTERM)) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(err) => Err(anyhow::anyhow!(
            "failed to send SIGTERM to daemon PID {pid}: {err}"
        )),
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

#[cfg(target_os = "macos")]
fn process_start_time(pid: u32) -> Option<u64> {
    anvil_intercept_macos::process_start_time(pid)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn process_start_time(_pid: u32) -> Option<u64> {
    None
}

/// macOS process start time as microseconds since the Unix epoch
/// (`pbi_start_tvsec * 1_000_000 + pbi_start_tvusec`), read via
/// `proc_pidinfo(PROC_PIDTBSDINFO)` (V060F-004).
///
/// Used purely as a PID-reuse discriminator, mirroring the Linux
/// `/proc/<pid>/stat` field-22 `starttime` read. The unit differs from
/// Linux (epoch µs here vs boot ticks there), but comparisons are always
/// same-host / same-platform — only stability and per-process uniqueness
/// matter. Microsecond resolution keeps a PID reused within the same
/// wall-clock second distinguishable.
///
/// Shared by [`process_start_time`] (PID-file staleness) and the interrupt
/// ladder's PID-reuse guard (`crate::interrupt`), so macOS runs the full
/// SIGINT→SIGTERM→SIGKILL ladder instead of the conservative fence-first
/// fallback that previously skewed macOS fence telemetry.
#[cfg(target_os = "macos")]
pub(crate) fn macos_process_start_time(pid: u32) -> Option<u64> {
    anvil_intercept_macos::process_start_time(pid)
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
/// ACTMO-014: outcome of a durable-registration reload — how many worktrees
/// were re-seeded into the registry and how many were reaped (directory gone
/// or no longer registerable).
struct ReloadOutcome {
    reloaded: usize,
    reaped: usize,
}

/// ACTMO-014: reload the persisted durable registration set into `registry`
/// before the daemon accepts connections (ADR-094 decision 1). Entries whose
/// worktree directory is gone — or that no longer register cleanly — are
/// reaped, and the pruned set is persisted so the on-disk shadow matches what
/// was actually reloaded. Factored out of `run_foreground` so the load-bearing
/// restart-recovery path is unit-testable without spinning the IPC listener.
fn reload_durable_registrations(
    store: &registration_store::RegistrationStore,
    registry: &SessionRegistry,
) -> Result<ReloadOutcome> {
    let persisted = store.load().with_context(|| {
        format!(
            "failed to load registration store {}",
            store.path().display()
        )
    })?;
    let mut survivors = Vec::with_capacity(persisted.len());
    let mut reaped = 0usize;
    for record in persisted {
        // `is_dir` (not `exists`): a worktree is a directory. If a regular file
        // now occupies the former path (e.g. removed then `touch`ed), it is not
        // a registerable worktree and must be reaped, not retained.
        if !record.worktree.is_dir() {
            reaped += 1;
            continue;
        }
        match registry.register(
            &record.session_id,
            &record.worktree,
            record.agent_tag.as_ref(),
            Instant::now(),
        ) {
            Ok(_) => survivors.push(record),
            Err(err) => {
                // A worktree that no longer canonicalises the same way, or
                // collides with another reloaded entry, is dropped from the set
                // rather than failing startup.
                tracing::warn!(
                    target: "anvil_intercept::registration",
                    error = %err,
                    worktree = %record.worktree.display(),
                    "skipped reloading a durable worktree registration",
                );
                reaped += 1;
            }
        }
    }
    if reaped > 0 {
        // Best-effort prune of the on-disk shadow; a write failure here logs
        // but does not fail startup (the in-memory set is already correct).
        if let Err(err) = store.replace_all(&survivors) {
            tracing::error!(
                target: "anvil_intercept::registration",
                error = %err,
                "failed to persist the pruned registration set on startup",
            );
        }
    }
    Ok(ReloadOutcome {
        reloaded: survivors.len(),
        reaped,
    })
}

/// ACTMO-014 (review F3): drop `reaped` worktrees from the persisted store in
/// one atomic write, instead of one load-modify-save per worktree.
fn prune_registrations(
    store: &registration_store::RegistrationStore,
    reaped: &[PathBuf],
) -> Result<(), registration_store::RegistrationStoreError> {
    let survivors: Vec<_> = store
        .load()?
        .into_iter()
        .filter(|record| !reaped.contains(&record.worktree))
        .collect();
    store.replace_all(&survivors)
}

/// ACTMO-019: outcome of registering the operator's `register_on_start`
/// worktrees on startup — how many are now durably registered (newly or already)
/// and how many were skipped (directory gone, or the daemon refused).
struct ConfiguredRegistrationOutcome {
    registered: usize,
    skipped: usize,
}

/// ACTMO-019: durably register the operator's `register_on_start` worktrees into
/// `registry` on startup, **atop** the reloaded ACTMO-014 persisted set.
///
/// Each path is canonicalised exactly as the CLI client does
/// ([`registration_store::canonicalise_for_registration`]) and keyed by the same
/// deterministic activation session id, so a configured worktree and a later
/// `anvil workspace register` of the same path share one membership (the second
/// heartbeats the first). A path whose directory is gone is skipped + reported
/// (reaper parity); a duplicate of an already-reloaded entry is idempotent; a
/// fenced/cap-exceeded/otherwise-refused entry is skipped, never fatal — a bad
/// operator config must not stop the daemon (per-connection admission fails
/// closed independently). There is **no filesystem scan**: only the exact listed
/// paths are touched (ADR-094 decision 5).
fn register_configured_worktrees(
    registry: &SessionRegistry,
    paths: &[PathBuf],
) -> ConfiguredRegistrationOutcome {
    let tag = registration_store::activation_agent_tag();
    let mut registered = 0usize;
    let mut skipped = 0usize;
    for path in paths {
        let canonical = registration_store::canonicalise_for_registration(path);
        // `is_dir` (not `exists`): a worktree is a directory. A `register_on_start`
        // entry whose directory is gone is skipped and logged (not registered);
        // the operator's config is left untouched — this mirrors the
        // persisted-store reaper's directory-gone check but does not mutate config.
        if !canonical.is_dir() {
            tracing::warn!(
                target: "anvil_intercept::registration",
                worktree = %canonical.display(),
                "skipping register_on_start entry — not a directory",
            );
            skipped += 1;
            continue;
        }
        let session_id = registration_store::activation_session_id(&canonical);
        match registry.register(&session_id, &canonical, Some(&tag), Instant::now()) {
            // Already a durable member (its id was reloaded from the persisted
            // set, or the path is listed twice). Idempotent — it is registered.
            Ok(_)
            | Err(
                RegistryError::SessionAlreadyExists(_) | RegistryError::WorktreeAlreadyOwned { .. },
            ) => registered += 1,
            Err(err) => {
                tracing::warn!(
                    target: "anvil_intercept::registration",
                    error = %err,
                    worktree = %canonical.display(),
                    "skipping register_on_start entry — daemon refused registration",
                );
                skipped += 1;
            }
        }
    }
    ConfiguredRegistrationOutcome {
        registered,
        skipped,
    }
}

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
        &opts.enforcement_config,
    )?;
    if daemon_state.active_fence_count() > 0 {
        tracing::info!(
            target: "anvil_intercept::fence",
            count = daemon_state.active_fence_count(),
            "loaded persisted intercept fences before accepting connections",
        );
    }

    // ACTMO-014: reload the durable registration set before accepting
    // connections — analogous to the fence reload above. Reap entries whose
    // worktree directory is gone, seed survivors into the registry as durable
    // (TTL-exempt) members, and persist the pruned set so the on-disk shadow
    // matches what was actually reloaded.
    let registration_store_path = opts.registration_store_path()?;
    let registration_store = Arc::new(registration_store::RegistrationStore::at_path(
        &registration_store_path,
    ));
    let reload = reload_durable_registrations(&registration_store, &daemon_state.registry)
        .with_context(|| {
            format!(
                "failed to load registration store {}",
                registration_store_path.display()
            )
        })?;
    if reload.reaped > 0 {
        tracing::info!(
            target: "anvil_intercept::registration",
            count = reload.reaped,
            "reaped durable worktree registrations whose directory is gone on startup",
        );
    }
    if reload.reloaded > 0 {
        tracing::info!(
            target: "anvil_intercept::registration",
            count = reload.reloaded,
            "registered worktrees on startup",
        );
    }

    // ACTMO-019: durably register the operator's `register_on_start` worktrees
    // (atop the reloaded set) so a curated in-scope set survives a daemon
    // restart without an `anvil start` in each. A malformed/untrusted config is
    // logged and skipped — per-connection admission independently fails closed,
    // so the daemon still starts. Only the exact listed paths are touched; no
    // filesystem scan (ADR-094 decision 5).
    match confinement::load_register_on_start() {
        Ok(paths) if !paths.is_empty() => {
            let outcome = register_configured_worktrees(&daemon_state.registry, &paths);
            if outcome.registered > 0 {
                tracing::info!(
                    target: "anvil_intercept::registration",
                    count = outcome.registered,
                    "registered worktrees from register_on_start config on startup",
                );
            }
            if outcome.skipped > 0 {
                tracing::warn!(
                    target: "anvil_intercept::registration",
                    count = outcome.skipped,
                    "skipped register_on_start entries on startup (missing directory or refused)",
                );
            }
        }
        Ok(_) => {}
        Err(err) => {
            tracing::warn!(
                target: "anvil_intercept::confinement",
                error = %err,
                "could not load register_on_start config — skipping startup auto-registration",
            );
        }
    }

    #[cfg(any(unix, windows))]
    {
        let dispatcher = RegistryDispatcher::new(
            Arc::clone(&daemon_state.registry),
            Arc::clone(&daemon_state.fence_store),
        )
        .with_registration_store(Arc::clone(&registration_store));
        let scan_buffer = opts.scan_buffer.clone();
        // USAGE-004: the command-invocation usage producer, injected by
        // `anvil-cli`. `None` ⇒ no usage rows.
        let usage_emitter = opts.usage_emitter.clone();
        // DPO-002: wire the fence-engage constraint_applied producer when
        // the host injected a sink. The fence's `set_telemetry` runs in
        // `DaemonState::new` (no access to `opts`), so the observation sink
        // is set here on the same store. The daemon session id is shared
        // with the save-time emitter where one is wired, so a
        // `gate_evaluated(save-time)` row and a `constraint_applied` row
        // from the same daemon process carry an identical `session_id`; if
        // no save-time emitter is present a fence-local UUID is minted.
        if let Some(sink) = opts.observation_sink.clone() {
            let daemon_session_id = opts.observation_emitter.as_ref().map_or_else(
                || uuid::Uuid::new_v4().to_string(),
                |e| e.daemon_session_id().to_owned(),
            );
            daemon_state.fence_store.set_observation_sink(
                sink,
                daemon_session_id,
                opts.observation_include_paths,
            );
        }
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
            .with_scan_buffer(scan_buffer.clone())
            .with_broadcaster(Arc::clone(&daemon_state.broadcaster)),
        );

        // MLP2-025b: install the production cross-check capability.
        // Currently Linux-only — MLP2-027 (macOS) and MLP2-028
        // (Windows) add the platform-specific peer-PID and lineage
        // support the cross-check depends on. Wiring it on non-Linux
        // today would classify every env-tagged write as
        // `Cross::Spoofed` (Windows accept passes `peer_pid: None`,
        // and on macOS `pid_starttime` / `parent_pid` return
        // `io::ErrorKind::Unsupported` so the lineage walk fails
        // shut), blocking legitimate sessions. The cfg gate widens
        // when those tickets land.
        // INTD-016: clone the resolved limits once for the listener
        // chain. Capturing into a local also keeps the closure below
        // `Copy`-friendly without borrowing `opts` across the `map`.
        let ipc_limits = opts.enforcement_config.ipc_limits;

        // DSV-005: build the shared save-time verdict state — the warm graph
        // cache, the per-worktree assurance machines, the antipattern config,
        // the two cooperating rayon pools (the antipattern scan runs on the
        // interactive pool, B7), and the operator confinement policy (open by
        // default; fail-closed on an untrusted config). Injected into the
        // listener so the three save-time verbs are served from the first
        // connection. The registry unregister hook that reclaims a
        // worktree's warm state (cache + assurance machine) on session
        // unregister is installed below, once both the registry and
        // `save_time_state` exist.
        // DSV-010b / ADR-070 Stage 2: served on both Unix and Windows. The
        // verdict spine + the config loaders are platform-neutral; the read
        // anchor is the only platform-split (a Unix dirfd / the Windows ADR-068
        // guard, behind `WorkspaceAnchor`). The tree-sitter parser injection is
        // still Unix-only in `anvil-cli`, so the Windows daemon runs parser-less
        // (safe `Partial` verdicts — the documented degraded mode) for now.
        #[cfg(any(unix, windows))]
        let save_time_state = {
            let scheduler = workspace_pool::WorkScheduler::new().with_context(|| {
                format!(
                    "failed to build the save-time work scheduler (budget: {:?})",
                    workspace_pool::PoolBudget::from_host()
                )
            })?;
            // The antipattern family runs the operator-configured pattern set
            // (DSV-041), loaded owner-only from `antipattern.yaml` beside the
            // confinement config. Fail-safe + loud: a missing config ⇒ the full
            // default set; an untrusted/malformed config ⇒ the full default set
            // with an `error` log — a broken config never silently disables
            // save-time checks, and never silently degrades.
            let mut state = save_time::SaveTimeState::new(
                scheduler,
                antipattern_config::load_or_fail_safe(),
                confinement::load_or_fail_closed(),
            );
            // DSV-010b hardening: the operator config is now read through the
            // per-platform owner-only `confinement::read_trusted` on Windows too
            // (reparse refusal + `GetSecurityInfo` owner-SID match), so the
            // earlier "unverified on Windows" warn no longer applies.
            // DSV-005: inject the dependency-inverted kernel parser if anvil-cli
            // wired one.
            if let Some(parser) = opts.symbol_parser.clone() {
                state = state.with_parser(parser);
            }
            state = state.with_broadcaster(Arc::clone(&daemon_state.broadcaster));
            // DPO-001: wire the save-time gate_evaluated producer when the
            // host injected one. With it, every validate_paths verdict
            // emits a Kindling row (pass and fail) through the shared
            // non-blocking sink; without it the daemon stays silent.
            if let Some(emitter) = opts.observation_emitter.clone() {
                state = state.with_observation_emitter(emitter);
            }
            // DSV-030 (ADR-069 §7): warm-graph persistence, **default-off +
            // fail-closed**. Only an affirmative `ANVIL_PERSIST_GRAPH` with a
            // resolvable state dir wires the snapshot directory; unset/garbage/no
            // home ⇒ no persistence (byte-for-byte today's rebuild-on-restart).
            // Unix-only for now (the Windows daemon's persistence is a follow-up,
            // mirroring the DSV-010/011 Windows-parity split).
            #[cfg(unix)]
            if anvil_graph_cache::snapshot::persist_graph_enabled(
                env::var("ANVIL_PERSIST_GRAPH").ok().as_deref(),
            ) {
                if let Some(dir) = snapshot_io::graph_cache_dir() {
                    tracing::info!(
                        target: "anvil_intercept::snapshot",
                        dir = %dir.display(),
                        "warm-graph persistence enabled (ANVIL_PERSIST_GRAPH)",
                    );
                    state = state.with_snapshot_dir(dir);
                } else {
                    tracing::warn!(
                        target: "anvil_intercept::snapshot",
                        "ANVIL_PERSIST_GRAPH set but no state dir resolved; persistence off",
                    );
                }
            }
            // Without a parser, verdicts stay `Partial` — warn so the degraded
            // mode is observable, not a silent feature-off.
            if !state.has_parser() {
                tracing::warn!(
                    target: "anvil_intercept::save_time",
                    "no symbol parser injected — validate_paths returns Partial verdicts only \
                     (no Certified); the kernel-backed parser is wired by anvil-cli",
                );
            }
            Arc::new(state)
        };

        // DSV-030 (ADR-069 §10): sweep orphaned `*.tmp` files left by an
        // interrupted snapshot write on a prior run. No-op when persistence off.
        save_time_state.sweep_snapshot_temps_on_start();

        // CIB-096 (ADR-069 §10): reclaim orphaned `*.snap` snapshots whose worktree
        // was deleted while the daemon was down (its unregister hook never fired).
        // Existence-based (per-snapshot `<hash>.root` companion), so it is SAFE at
        // cold boot — it cannot wipe a live, not-yet-reattached snapshot. No-op when
        // persistence off (no snapshot dir wired).
        let reclaimed_orphans = save_time_state.sweep_orphan_snapshots_on_start();
        if reclaimed_orphans > 0 {
            tracing::info!(
                target: "anvil_intercept::snapshot",
                reclaimed = reclaimed_orphans,
                "reclaimed orphaned warm-graph snapshots on start (CIB-096)",
            );
        }

        // DSV: reclaim a worktree's warm state (graph cache + assurance
        // machine) when its last session leaves the registry. The hook is
        // installed post-construction because `save_time_state` is built
        // after the registry. The registry fires it with the canonical
        // worktree path, which matches the key `validate_paths` warmed the
        // cache under (both canonicalise via `std::fs::canonicalize`), so
        // the `invalidate` lands on the right key. This is the single
        // composition point for unregister-time invalidators — when
        // MLP2-014 lands the rule cache, its `invalidate` joins this same
        // closure rather than competing for the registry's one hook slot.
        #[cfg(any(unix, windows))]
        {
            let warm_state = Arc::clone(&save_time_state);
            let installed = daemon_state
                .registry
                .set_unregister_hook(Arc::new(move |worktree| {
                    warm_state.invalidate(&rule_cache::WorktreeKey::from_canonical(
                        worktree.to_path_buf(),
                    ));
                }));
            // The registry is freshly built in `DaemonState::new` without a
            // hook, so this set is the first and must succeed. A `false` means
            // a second install site was wired (a bug) — surface it in release
            // logs (the failure mode is silent warm-state growth, not a crash),
            // not only under `debug_assert`.
            if !installed {
                tracing::error!(
                    target: "anvil_intercept::save_time",
                    "unregister hook already installed before this call — warm-state \
                     reclamation may be wired twice or in the wrong place; review the \
                     registry hook composition",
                );
            }
            debug_assert!(
                installed,
                "the unregister hook must install on a freshly-built registry",
            );
        }

        #[cfg(unix)]
        let listener = if let Some(socket_path) = opts.ipc_socket_path() {
            ipc::IpcListener::bind_with_scan_buffer_service(socket_path, dispatcher, scan_buffer)
        } else {
            ipc::IpcListener::bind_default_with_scan_buffer_service(dispatcher, scan_buffer)
        }
        .map(|listener| {
            let listener = listener
                .with_status_provider(Arc::clone(&status_provider))
                .with_limits(ipc_limits)
                .with_save_time_state(Arc::clone(&save_time_state))
                // MLP2-071 Phase 2: wire the telemetry broadcaster so
                // `SubscribeTelemetry` connections register against the
                // daemon's per-startup fan-out.
                .with_broadcaster(Arc::clone(&daemon_state.broadcaster));
            // USAGE-004: wire the command-invocation usage producer when
            // the host injected one.
            let listener = match usage_emitter.clone() {
                Some(emitter) => listener.with_usage_emitter(emitter),
                None => listener,
            };
            #[cfg(target_os = "linux")]
            let listener = listener.with_cross_check_context(ipc::CrossCheckContext {
                registry: Arc::clone(&daemon_state.registry),
                fence_store: Arc::clone(&daemon_state.fence_store),
            });
            listener
        })
        .context("failed to bind intercept IPC listener")?;

        #[cfg(windows)]
        let listener = if let Some(pipe_name) = opts.ipc_pipe_name() {
            ipc::IpcListener::bind_with_scan_buffer_service(pipe_name, dispatcher, scan_buffer)
        } else {
            ipc::IpcListener::bind_default_with_scan_buffer_service(dispatcher, scan_buffer)
        }
        .map(|listener| {
            // DSV-010b: serve the three save-time verbs on Windows too. The
            // cross-check context stays Linux-only (`SO_PEERCRED` PID-based);
            // the Windows peer boundary is the owner-only pipe DACL plus the
            // explicit peer-SID compare in the accept loop.
            let listener = listener
                .with_status_provider(Arc::clone(&status_provider))
                .with_limits(ipc_limits)
                .with_save_time_state(Arc::clone(&save_time_state))
                // MLP2-071 Phase 2: wire the telemetry broadcaster (served
                // on Windows too; the subscriber surface is platform-neutral).
                .with_broadcaster(Arc::clone(&daemon_state.broadcaster));
            // USAGE-004: wire the command-invocation usage producer when
            // the host injected one.
            match usage_emitter.clone() {
                Some(emitter) => listener.with_usage_emitter(emitter),
                None => listener,
            }
        })
        .context("failed to bind intercept IPC listener")?;

        let listener_token = token.clone();
        let mut listener_handle = AbortOnDropJoinHandle::new(tokio::spawn(async move {
            listener.serve(listener_token).await
        }));
        let mut tick = tokio::time::interval(Duration::from_millis(250));
        // ACTMO-014: a slow periodic reaper that drops durable registrations
        // whose worktree directory is gone (e.g. `git worktree remove`d while
        // the daemon ran). Far slower than the 250 ms eviction tick because it
        // stats the filesystem per durable worktree; the set is capped (≤64) so
        // a 60 s sweep is cheap and bounded.
        let mut reaper_tick = tokio::time::interval(Duration::from_mins(1));
        reaper_tick.reset(); // skip the immediate first fire; startup already reaped
        // CIB-095d + N2: persist every warm worktree's graph on **either** exit
        // path (graceful `token.cancelled()` AND listener-failure) so a warm
        // graph is never silently lost when persistence is enabled. Offloaded to
        // `spawn_blocking` (N2) so the synchronous snapshot writes never block the
        // single-thread tokio runtime; awaited so shutdown still completes the
        // writes before exit. No-op when persistence is off.
        let persist_on_shutdown = || {
            let state = Arc::clone(&save_time_state);
            async move {
                if let Err(err) =
                    tokio::task::spawn_blocking(move || state.persist_all_on_shutdown()).await
                {
                    let detail = err.to_string();
                    tracing::warn!(
                        target: "anvil_intercept::snapshot",
                        error = %detail,
                        "shutdown snapshot flush task panicked",
                    );
                }
            }
        };
        loop {
            tokio::select! {
                biased;
                () = token.cancelled() => break,
                result = listener_handle.join() => {
                    // CIB-095d: persist before propagating the listener failure —
                    // this path previously `return`ed without flushing, dropping
                    // every warm graph even with persistence enabled.
                    persist_on_shutdown().await;
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
                _ = reaper_tick.tick() => {
                    // `is_dir` (not `exists`): only a real directory is a live
                    // worktree; a file or dangling symlink at the path is reaped.
                    let reaped = daemon_state.registry.reap_missing(Path::is_dir);
                    if !reaped.is_empty() {
                        tracing::info!(
                            target: "anvil_intercept::registration",
                            count = reaped.len(),
                            "reaped durable worktree registrations whose directory is gone",
                        );
                        // Prune the persisted shadow in a single atomic write
                        // (review F3) rather than N load-modify-save cycles.
                        if let Err(err) = prune_registrations(&registration_store, &reaped) {
                            tracing::error!(
                                target: "anvil_intercept::registration",
                                error = %err,
                                "failed to prune reaped registrations from the store",
                            );
                        }
                    }
                }
            }
        }

        // ACTMO-017: one INFO event recording how many durable worktrees lose
        // live protection on this graceful stop (they remain persisted and
        // reload on next start; this is the operator-visible shutdown count).
        let registered_on_stop = daemon_state.registry.registered_worktrees().len();
        if registered_on_stop > 0 {
            tracing::info!(
                target: "anvil_intercept::registration",
                count = registered_on_stop,
                "stopping daemon; registered worktrees will reload on next start",
            );
        }

        // DSV-030 (ADR-069 §4): graceful shutdown (the `token.cancelled()` path
        // above) — persist every warm worktree's graph so the next start re-warms
        // from disk. No-op when persistence is off. A crash skips it, so a
        // crash-then-restart still pays one cold rebuild per key.
        persist_on_shutdown().await;

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

    /// ACTMO-014: a secure (`0700`) state dir under a tempdir, mirroring
    /// `$ANVIL_HOME`, so the registration store's owner-only-parent check
    /// accepts a hand-built fixture.
    fn secure_store_path(dir: &tempfile::TempDir) -> PathBuf {
        let state = dir.path().join("state");
        fs::create_dir_all(&state).expect("create state dir");
        #[cfg(unix)]
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).expect("chmod");
        state.join("registered-worktrees.json")
    }

    /// ACTMO-014 D4 (restart recovery + reaper): a persisted registration whose
    /// worktree still exists is reloaded into the registry as durable
    /// membership; one whose directory is gone is reaped and pruned from the
    /// store. This is the load-bearing "survive a daemon restart" path.
    #[test]
    fn reload_durable_registrations_reloads_survivors_and_reaps_missing() {
        let home = tempfile::tempdir().expect("home");
        let store = registration_store::RegistrationStore::at_path(secure_store_path(&home));
        let live = tempfile::tempdir().expect("live worktree");
        let live_canonical = fs::canonicalize(live.path()).expect("canonicalise");
        let spine = anvil_intercept_proto::session::AgentTag::new(
            "anvil-start",
            anvil_intercept_proto::session::ACTIVATION_SPINE_CLAIMED_AGENT_ID,
            0,
        );

        store
            .upsert(registration_store::RegistrationRecord::new(
                SessionId::new("sess_activation_live"),
                live_canonical.clone(),
                Some(spine.clone()),
            ))
            .expect("persist live");
        store
            .upsert(registration_store::RegistrationRecord::new(
                SessionId::new("sess_activation_gone"),
                home.path().join("worktree-gone"),
                Some(spine.clone()),
            ))
            .expect("persist gone");
        // Adversarial review F4: a regular FILE now occupies a former worktree
        // path. `is_dir` (not `exists`) must reap it, not retain it.
        let file_path = home.path().join("was-a-worktree");
        fs::write(&file_path, b"not a worktree").expect("write file");
        store
            .upsert(registration_store::RegistrationRecord::new(
                SessionId::new("sess_activation_file"),
                fs::canonicalize(&file_path).expect("canonicalise file"),
                Some(spine),
            ))
            .expect("persist file");

        let registry = SessionRegistry::new();
        let outcome = reload_durable_registrations(&store, &registry).expect("reload");

        assert_eq!(outcome.reloaded, 1, "only the live worktree reloads");
        assert_eq!(
            outcome.reaped, 2,
            "the gone path and the file are both reaped"
        );
        assert_eq!(registry.registered_worktrees(), vec![live_canonical]);
        // The on-disk shadow was pruned to match.
        let remaining = store.load().expect("reload store");
        assert_eq!(remaining.len(), 1);
        assert_eq!(
            remaining[0].session_id,
            SessionId::new("sess_activation_live")
        );
    }

    /// ACTMO-019: the daemon durably registers the operator's `register_on_start`
    /// worktrees on startup. A live worktree is registered as durable membership
    /// (using the same activation session id the CLI client derives, so a later
    /// `anvil workspace register` heartbeats it); a missing path is skipped +
    /// counted, never fatal; and re-running is idempotent (no duplicate).
    #[test]
    fn register_configured_worktrees_registers_live_and_skips_missing() {
        let wt_a = tempfile::tempdir().expect("worktree a");
        let wt_b = tempfile::tempdir().expect("worktree b");
        let a_canonical = registration_store::canonicalise_for_registration(wt_a.path());
        let b_canonical = registration_store::canonicalise_for_registration(wt_b.path());
        let paths = vec![
            wt_a.path().to_path_buf(),
            PathBuf::from("/nonexistent/anvil/register-on-start-gone"),
            wt_b.path().to_path_buf(),
        ];

        let registry = SessionRegistry::new();
        let outcome = register_configured_worktrees(&registry, &paths);

        assert_eq!(outcome.registered, 2, "both live worktrees registered");
        assert_eq!(outcome.skipped, 1, "the missing path is skipped + counted");
        let mut registered = registry.registered_worktrees();
        registered.sort();
        let mut expected = vec![a_canonical.clone(), b_canonical];
        expected.sort();
        assert_eq!(
            registered, expected,
            "both live worktrees are durable members"
        );

        // The id is the deterministic activation id, so a CLI re-register of the
        // same path collides on the session id (heartbeat, not a fresh session).
        let id_a = registration_store::activation_session_id(&a_canonical);
        assert!(
            matches!(
                registry.register(
                    &id_a,
                    &a_canonical,
                    Some(&registration_store::activation_agent_tag()),
                    Instant::now(),
                ),
                Err(RegistryError::SessionAlreadyExists(_))
            ),
            "the configured worktree owns the deterministic activation session id"
        );

        // Idempotent: a second startup pass does not duplicate membership.
        let again = register_configured_worktrees(&registry, &paths);
        assert_eq!(
            again.registered, 2,
            "already-registered worktrees stay registered"
        );
        assert_eq!(again.skipped, 1);
        assert_eq!(
            registry.registered_worktrees().len(),
            2,
            "no duplicate membership on re-run"
        );
    }

    // DISTRIB-006 (ADR-060): ANVIL_HOME re-roots the daemon PID file directly
    // under the prefix, taking precedence over the runtime dir, so a candidate
    // daemon's PID-file exclusive-create cannot collide with production's.
    #[test]
    fn default_pid_file_path_anvil_home_re_roots_under_prefix() {
        let p = default_pid_file_path_from(
            Some(PathBuf::from("/opt/anvil-beta")),
            Some(PathBuf::from("/run/user/1000")),
            None,
            Some(PathBuf::from("/home/somebody")),
        )
        .expect("resolve");
        assert_eq!(p, PathBuf::from("/opt/anvil-beta/intercept.pid"));
    }

    #[test]
    fn default_pid_file_path_falls_back_to_runtime_dir_when_anvil_home_unset() {
        let p = default_pid_file_path_from(
            None,
            Some(PathBuf::from("/run/user/1000")),
            None,
            Some(PathBuf::from("/home/somebody")),
        )
        .expect("resolve");
        assert_eq!(p, PathBuf::from("/run/user/1000/anvil/intercept.pid"));
    }

    #[test]
    fn default_pid_file_path_falls_back_to_home_state_dir() {
        let p = default_pid_file_path_from(None, None, None, Some(PathBuf::from("/home/somebody")))
            .expect("resolve");
        assert_eq!(
            p,
            PathBuf::from("/home/somebody/.local/state/anvil/intercept.pid")
        );
    }

    // V060F-002: `anvil intercept stop` plan resolution. The branch
    // selection is pure with liveness classification injected, so each
    // outcome is pinned without spawning a process or sending a signal.
    #[test]
    fn plan_stop_missing_pid_file_is_not_running() {
        assert_eq!(
            plan_stop(None, |_| ExistingPidStatus::Live),
            StopPlan::NotRunning
        );
    }

    #[test]
    fn plan_stop_live_daemon_signals_its_pid() {
        assert_eq!(
            plan_stop(Some("4321\nstart_time=99\n"), |_| ExistingPidStatus::Live),
            StopPlan::Signal { pid: 4321 }
        );
    }

    #[test]
    fn plan_stop_dead_daemon_clears_stale_pid() {
        assert_eq!(
            plan_stop(Some("4321\n"), |_| ExistingPidStatus::Stale),
            StopPlan::ClearStale { pid: 4321 }
        );
    }

    #[test]
    fn plan_stop_unparseable_pid_is_malformed() {
        assert_eq!(
            plan_stop(Some("not-a-pid\n"), |_| ExistingPidStatus::Live),
            StopPlan::Malformed
        );
    }

    #[test]
    fn plan_stop_unprovable_liveness_is_malformed() {
        // An unparseable/old record that `existing_pid_status` cannot prove
        // either way must not blindly signal an arbitrary PID.
        assert_eq!(
            plan_stop(Some("4321\n"), |_| ExistingPidStatus::Unknown),
            StopPlan::Unproven
        );
    }

    #[cfg(unix)]
    #[test]
    fn stop_daemon_refuses_symlink_pid_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("target.pid");
        let link = dir.path().join("intercept.pid");
        fs::write(&target, "4321\n").expect("write target");
        std::os::unix::fs::symlink(&target, &link).expect("symlink pid file");

        let err = stop_daemon_at(&link).expect_err("symlink should be refused");
        assert!(
            err.to_string().contains("refusing symlink PID file"),
            "unexpected error: {err:#}",
        );
    }

    // V060F-004: runtime check on the macOS CI leg — the helper must
    // resolve a non-zero start time for the current (definitely live)
    // process, and must agree with `process_start_time`. Compiled only on
    // macOS; the apple-darwin CI job is the runtime proof for the FFI.
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_process_start_time_resolves_for_current_process() {
        let pid = process::id();
        let direct = macos_process_start_time(pid);
        assert!(
            direct.is_some_and(|t| t > 0),
            "own process start time should resolve on macOS, got {direct:?}",
        );
        assert_eq!(direct, process_start_time(pid));
    }

    // MLP2-071: pin that DaemonState::new constructs a Fanout whose
    // cross-session policy mirrors the operator-configured value. The
    // gap #1722 surfaced was literal: `Fanout::with_cross_session_policy`
    // existed with zero production callers, so the documented flag was
    // configured-but-ignored. This test is the unit-level pin that
    // proves DaemonState now reads `enforcement.telemetry.allow_cross_session`
    // through to the fan-out — the end-to-end SubscribeTelemetry path
    // is pinned separately by the Phase F integration test.
    #[test]
    fn daemon_state_constructs_fanout_with_configured_cross_session_policy() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // FenceStore::load enforces "parent must be private to the
        // current user" — wrap our path in a nested dir we control
        // so the parent-owner check passes without us having to
        // chmod the tempdir itself.
        let nested = tmp.path().join("intercept");
        std::fs::create_dir(&nested).expect("create nested dir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o700))
                .expect("chmod private");
        }
        let fence_store_path = nested.join("fences.json");
        let fence_store = fence::FenceStore::at_path(&fence_store_path);
        let fences = fence::FenceState::default();

        // Default policy: Deny.
        let default_state = DaemonState::new(
            fence_store.clone(),
            fences.clone(),
            &config::Resolved::default(),
        )
        .expect("daemon state construction must succeed");
        assert_eq!(
            default_state.fanout.cross_session_policy(),
            fanout::CrossSessionPolicy::Deny,
            "default config must produce a deny-by-default fanout",
        );

        // Opt-in: Redact.
        let opt_in_config = config::Resolved {
            telemetry_allow_cross_session: true,
            ..config::Resolved::default()
        };
        let opt_in_state = DaemonState::new(fence_store.clone(), fences.clone(), &opt_in_config)
            .expect("daemon state construction must succeed");
        assert_eq!(
            opt_in_state.fanout.cross_session_policy(),
            fanout::CrossSessionPolicy::Redact,
            "telemetry.allow_cross_session: true must flow into a Redact-policy fanout — \
             this is the literal closure of #1722's configured-but-ignored gap",
        );
    }

    // MLP2-071 Phase D: pin that `RegistryOwnershipResolver` consults
    // the registry's `subscriber_binding` for the authorisation check.
    // Without the binding (or with a mismatched one), the resolver
    // default-denies even for sessions that are otherwise registered.
    // With the matching binding, the resolver authorises and the
    // fan-out delivers `Allow`. This is the unit-level pin for D3 +
    // D4 of the design pass; Phase E covers the IPC-side credential
    // minting that wires the binding in production.
    #[test]
    fn registry_ownership_resolver_consults_subscriber_binding() {
        use crate::fanout::OwnershipResolver;
        use anvil_intercept_proto::SessionId;
        use anvil_intercept_proto::session::AgentTag;
        use std::sync::Arc;
        use std::time::Instant;

        let registry = Arc::new(SessionRegistry::new());
        let session_id = SessionId::new("sess-bind-A");
        let tmp = tempfile::tempdir().expect("tempdir");
        let worktree = tmp.path().join("worktree");
        std::fs::create_dir(&worktree).expect("create worktree dir");
        let agent_tag = AgentTag {
            driver_id: "test-driver".into(),
            claimed_agent_id: "claude-D".into(),
            pid_starttime: 1_700_000_000,
        };
        registry
            .register(&session_id, &worktree, Some(&agent_tag), Instant::now())
            .expect("register session");

        // D6 is exercised by its own test below; this test pins the
        // binding/authorisation contract, so an empty live fence store is
        // sufficient (no worktree is spoof-fenced here).
        let fences_dir = tempfile::tempdir().expect("fence tempdir");
        let fence_store = Arc::new(fence::FenceStore::at_path(
            fences_dir.path().join("state/intercept-fences.json"),
        ));
        let resolver =
            fanout::RegistryOwnershipResolver::new(Arc::clone(&registry), Arc::clone(&fence_store));
        let owner = fanout::SubscriberId::new("peer:uid=1000:pid=4242:start=42:bin=hash");
        let stranger = fanout::SubscriberId::new("peer:uid=1000:pid=9999:start=99:bin=other");

        // No binding yet → default-deny for everyone, including the
        // would-be owner.
        assert!(
            !resolver.is_authorised(&owner, session_id.as_str()),
            "without a binding, even the prospective owner must be denied — the \
             registry has no peer-credential proof yet"
        );

        // Bind the owner.
        let bound = registry.bind_subscriber(&session_id, owner.as_str().to_string());
        assert!(
            bound,
            "bind_subscriber must succeed for a registered session"
        );

        // Owner authorised; stranger denied.
        assert!(
            resolver.is_authorised(&owner, session_id.as_str()),
            "owner with matching binding must be authorised"
        );
        assert!(
            !resolver.is_authorised(&stranger, session_id.as_str()),
            "stranger with non-matching binding must be denied — this is the \
             defence against a hostile same-UID peer trying to subscribe to \
             another session's telemetry"
        );

        // Binding an unknown session is a no-op (returns false).
        let unknown = SessionId::new("sess-unknown");
        assert!(
            !registry.bind_subscriber(&unknown, "anything".into()),
            "binding an unregistered session must return false, not silently \
             create a binding for a ghost session id"
        );
    }

    // MLP2-071 D6 (production path): the `RegistryOwnershipResolver`
    // maps an originating session id to its worktree and reports the
    // session as degraded iff that worktree carries a
    // `degraded:spoofed-attribution` fence (MLP2-025). This is the
    // production counterpart to the `fanout::tests` StubResolver D6
    // pins — it proves the real registry→fence consultation, not just
    // the `decide()` branch.
    #[test]
    fn registry_ownership_resolver_flags_spoof_fenced_origin_as_degraded() {
        use crate::fanout::OwnershipResolver;
        use anvil_intercept_proto::SessionId;
        use std::sync::Arc;
        use std::time::Instant;

        let registry = Arc::new(SessionRegistry::new());
        let tmp = tempfile::tempdir().expect("tempdir");

        // A clean session on a non-fenced worktree.
        let clean_id = SessionId::new("sess-clean");
        let clean_wt = tmp.path().join("clean");
        std::fs::create_dir(&clean_wt).expect("create clean worktree");
        registry
            .register(&clean_id, &clean_wt, None, Instant::now())
            .expect("register clean session");

        // A session whose worktree the MLP2-025 cross-check fenced as
        // spoofed.
        let spoofed_id = SessionId::new("sess-spoofed");
        let spoofed_wt = tmp.path().join("spoofed");
        std::fs::create_dir(&spoofed_wt).expect("create spoofed worktree");
        registry
            .register(&spoofed_id, &spoofed_wt, None, Instant::now())
            .expect("register spoofed session");

        // The store creates its own `state/` parent with 0700; placing
        // the file there satisfies the secure-store-parent guard.
        let store = Arc::new(fence::FenceStore::at_path(
            tmp.path().join("state/intercept-fences.json"),
        ));
        store
            .fence_worktree_for_spoof(&spoofed_wt)
            .expect("spoof-fence the worktree");

        let resolver =
            fanout::RegistryOwnershipResolver::new(Arc::clone(&registry), Arc::clone(&store));

        assert!(
            resolver.is_degraded_origin(spoofed_id.as_str()),
            "a session on a spoof-fenced worktree must report as degraded so \
             its envelopes are denied to cross-session subscribers (D6)"
        );
        assert!(
            !resolver.is_degraded_origin(clean_id.as_str()),
            "a session on a clean worktree must NOT be flagged degraded"
        );
        assert!(
            !resolver.is_degraded_origin("sess-unknown"),
            "an unknown session id is not degraded — it is already \
             default-denied by the ownership check"
        );
    }

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

    #[test]
    fn pid_file_guard_keeps_stale_recovery_locked_for_lifetime() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_file = test_pid_file(&tmp);
        create_secure_test_pid_dir(pid_file.parent().expect("pid parent"));
        fs::write(&pid_file, "999999999\nstart_time=1\n").expect("write stale pid");

        let guard = PidFileGuard::acquire(&pid_file).expect("recover stale pid file");
        let err = PidFileGuard::acquire(&pid_file)
            .expect_err("second guard should not race stale recovery while first is live");
        let message = format!("{err:#}");
        assert!(
            message.contains("already running") || message.contains("locked"),
            "second acquisition should report live ownership, got: {message}",
        );
        assert_eq!(
            fs::read_to_string(&pid_file)
                .expect("live pid file should remain")
                .lines()
                .next(),
            Some(std::process::id().to_string().as_str())
        );

        drop(guard);
        assert!(
            !pid_file.exists(),
            "owned pid file should be removed on drop"
        );
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
