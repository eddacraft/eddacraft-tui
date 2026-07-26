//! INTD-003: in-memory session registry.
//!
//! The registry is the single authority on which sessions are active,
//! which worktree each owns, and when a session has gone silent long
//! enough to count as crashed. It is deliberately synchronous and
//! pure: the daemon's `run_foreground` loop owns scheduling and ticks
//! [`SessionRegistry::evict_stale`] from its 250 ms interval.
//!
//! Spawning a background eviction task here would couple the registry
//! to a runtime and make it harder to drive from tests; the council
//! pinned this layer as a synchronous data structure.
//!
//! See `plans/modules/intercept-daemon.aps.md` task INTD-003.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anvil_intercept_proto::session::{AgentTag, LineageAnchor};
use anvil_intercept_proto::{SessionId, SessionRecord, SessionStatus};
use thiserror::Error;

/// MLP2-025: three-state result of the env-tag spoof cross-check.
///
/// - [`Cross::Untagged`] — no env tag supplied. The pre-MLP2-025
///   enforcement path applies unchanged.
/// - [`Cross::Match`] — env tag matches the daemon-issued tag found on
///   the writer's PID lineage. Attribution survives.
/// - [`Cross::Spoofed`] — env tag is present but does not match any
///   daemon-issued tag on the lineage. The caller blocks the in-flight
///   write and records a worktree-level fence with reason
///   `degraded:spoofed-attribution`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cross {
    /// No env tag supplied.
    Untagged,
    /// Env tag matches the daemon-issued tag on the writer's lineage.
    Match,
    /// Env tag is present but no matching daemon-issued tag was
    /// found on the writer's lineage. Treat the write as malicious.
    Spoofed,
}

impl Cross {
    /// Pure classifier: given the env-supplied tag and the daemon-
    /// issued tag found on the writer's lineage, return the three-
    /// state result. Production callers obtain `registered` via
    /// [`SessionRegistry::lookup_tag_for_lineage`]; tests target this
    /// helper directly with synthetic pairs.
    ///
    /// Trust boundary: the lineage walk grants any *descendant* of a
    /// registered PID the registered tag. The classifier therefore
    /// rejects only out-of-lineage spoofs (PID reuse after launcher
    /// exit, env-tag forgery from an unrelated process tree).
    /// Intra-lineage privilege escalation — a co-process inside a
    /// legitimate launcher's process tree forging the env tag — is
    /// out of scope by design.
    #[must_use]
    pub fn classify(env_tag: Option<&AgentTag>, registered: Option<&AgentTag>) -> Self {
        match (env_tag, registered) {
            (None, _) => Cross::Untagged,
            (Some(env), Some(reg)) if env == reg => Cross::Match,
            _ => Cross::Spoofed,
        }
    }
}

/// Default session heartbeat TTL — pinned at 30 s by INTD-003 in
/// `plans/modules/intercept-daemon.aps.md`. A session that misses this
/// window is treated as crashed.
pub const DEFAULT_HEARTBEAT_TTL: Duration = Duration::from_secs(30);

/// MLP2-024: default per-worktree session cap. This bounds **live agent
/// sessions concurrently active** on one worktree — i.e. how many agents may
/// be saving/validating against the same worktree at once — not how many
/// agents may ever exist. Sized for ~6 concurrent save-time sub-agents with
/// ~3x headroom; operators can tighten via `.anvil.yaml`
/// (`enforcement.session.per_worktree_max`). Durable activation-spine
/// membership (ACTMO-014) is exempt — it is a persisted registration record,
/// not a live saver — and is bounded by [`DEFAULT_REGISTERED_WORKTREE_CAP`]
/// instead. Mirrored here (and not imported from `crate::config`) so the
/// registry stays self-contained for callers that bypass the config layer.
pub const DEFAULT_PER_WORKTREE_CAP: usize = 16;

/// ACTMO-014: default cap on the number of **distinct durably-registered
/// worktrees** (the persisted membership set), independent of
/// [`DEFAULT_PER_WORKTREE_CAP`] (which bounds live sessions on one worktree).
/// 64 distinct worktrees is a generous ceiling for a single operator's set of
/// active checkouts while still bounding the otherwise-unbounded persisted set
/// (ADR-094 decision 1 / council ops MAJOR). Configurable alongside
/// `enforcement.session.per_worktree_max`.
pub const DEFAULT_REGISTERED_WORKTREE_CAP: usize = 64;

/// Errors returned by the synchronous registry surface. The wire layer
/// (INTD-002) maps these onto JSON-RPC error codes; that mapping lives
/// outside this module so the registry stays independent of transport.
///
/// `PartialEq` is hand-written rather than derived because
/// [`std::io::Error`] is not `PartialEq` — equality on
/// `WorktreePathInvalid` compares the path and the io error kind, which
/// is the part tests actually care about.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// Another session already owns the canonicalised worktree path
    /// **for the same `AgentTag`** (MLP2-023). `existing` is the id of
    /// the live owner so the caller can decide whether to surface,
    /// retry, or refuse.
    ///
    /// For an untagged registration this still means "another untagged
    /// session already owns the worktree"; for a tagged registration
    /// it means "another session with the same tag is already
    /// registered against this worktree". Two sessions with *distinct*
    /// tags on the same worktree are allowed and do not produce this
    /// error.
    #[error("worktree already owned by session {existing:?}")]
    WorktreeAlreadyOwned { existing: SessionId },

    /// Worktree path could not be canonicalised — the path either does
    /// not exist or is otherwise unusable as a registry key. v1 refuses
    /// to register such sessions; the launcher must materialise the
    /// worktree before calling.
    #[error("worktree path could not be canonicalised: {path:?}: {source}")]
    WorktreePathInvalid {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A heartbeat / update arrived for a session id the registry has
    /// no record of (never registered, or already evicted).
    #[error("unknown session: {0:?}")]
    UnknownSession(SessionId),

    /// A `register` call reused a session id that is already in the
    /// registry (potentially against a different worktree). Reusing an
    /// id would leave the previous worktree index entry orphaned, so
    /// v1 rejects this rather than silently replacing the record. The
    /// caller must `unregister` the old id first or pick a fresh one.
    #[error("session already registered: {0:?}")]
    SessionAlreadyExists(SessionId),

    /// The worktree is under a persisted fence and must be explicitly
    /// unblocked before a new session can own it.
    #[error("worktree is fenced until explicit unblock: {worktree:?}")]
    WorktreeFenced { worktree: PathBuf },

    /// Fence state could not be loaded, so registration fails closed.
    #[error("fence state unavailable: {message}")]
    FenceStateUnavailable { message: String },

    /// MLP2-024: the worktree already has the configured maximum
    /// number of live sessions. `cap` is the per-worktree limit at
    /// the time of refusal; `live` is the count the registry
    /// observed. The launcher should surface this as
    /// `Refused::SessionCapExceeded` and back off.
    #[error(
        "worktree session cap exceeded for {worktree:?}: \
         {live} live sessions at cap={cap}"
    )]
    SessionCapExceeded {
        worktree: PathBuf,
        cap: usize,
        live: usize,
    },

    /// MLP2-026: the worktree is in `degraded:fence-cascade` mode
    /// (5 fences fired within 60 seconds) and refuses new session
    /// registrations until an operator clears the cascade via
    /// `anvil intercept unblock --acknowledge-cascade <worktree>`.
    /// Mirrors the `SessionCapExceeded` precedent. See spec §3.5.
    #[error("worktree is in degraded fence-cascade mode and refuses new sessions: {worktree:?}")]
    WorktreeCascaded { worktree: PathBuf },

    /// ACTMO-014: the durable registered-worktree membership set is at its
    /// configured cap. Distinct from [`Self::SessionCapExceeded`], which
    /// bounds live sessions on a single worktree; this bounds the number of
    /// **distinct** worktrees persisted as durable members. `cap` is the
    /// limit at refusal time; `live` is the distinct-worktree count observed.
    /// Only durable (activation-spine) registrations of a *new* worktree can
    /// trip it — refreshing an already-registered worktree never does.
    #[error("registered worktree cap exceeded: {live} registered at cap={cap}")]
    RegisteredWorktreeCapExceeded { cap: usize, live: usize },

    /// MLP2-074: a control-lane command that mutates per-session
    /// lineage state (today: `session.report_process`) was called
    /// from a peer whose authenticated pid does not match the
    /// launcher pid the session was registered with. The launcher's
    /// pid is the `record.pid` stamped at `register_with_lineage`
    /// time. A `None` `expected` (no anchor at register) is the
    /// "legacy register without lineage" case and is rejected so the
    /// daemon cannot be tricked into adopting a child anchor it
    /// cannot attribute back to a known launcher.
    #[error(
        "session {session:?} peer-ownership check failed: \
         expected launcher pid {expected:?}, peer pid {actual}"
    )]
    PeerOwnershipMismatch {
        session: SessionId,
        expected: Option<u32>,
        actual: u32,
    },

    /// MLP2-074 (PR #1895 review): `session.report_process` was
    /// called with a child `(pid, pid_starttime)` pair that is
    /// already mapped in `by_pid_lineage` to a *different* session.
    /// Re-narrowing to an already-claimed anchor would silently
    /// overwrite the existing mapping and let one session hijack
    /// lineage lookups for another (the launcher's wire body is
    /// trusted only against its own session — `child_pid` is NOT
    /// constrained to match `peer_pid`). The collision is checked
    /// before any mutation so the registry stays consistent on
    /// rejection.
    ///
    /// Note: re-narrowing to the SAME pair this session already
    /// owns is idempotent and returns `Ok` — only cross-session
    /// collisions trip this variant.
    #[error(
        "session {session:?} lineage anchor collision: \
         (pid={child_pid}, pid_starttime={child_pid_starttime}) \
         already claimed by session {existing:?}"
    )]
    LineageAnchorCollision {
        session: SessionId,
        existing: SessionId,
        child_pid: u32,
        child_pid_starttime: u64,
    },
}

impl PartialEq for RegistryError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::WorktreePathInvalid {
                    path: a,
                    source: ae,
                },
                Self::WorktreePathInvalid {
                    path: b,
                    source: be,
                },
            ) => a == b && ae.kind() == be.kind(),
            (
                Self::WorktreeAlreadyOwned { existing: a },
                Self::WorktreeAlreadyOwned { existing: b },
            )
            | (Self::UnknownSession(a), Self::UnknownSession(b))
            | (Self::SessionAlreadyExists(a), Self::SessionAlreadyExists(b)) => a == b,
            (Self::WorktreeFenced { worktree: a }, Self::WorktreeFenced { worktree: b }) => a == b,
            (Self::WorktreeCascaded { worktree: a }, Self::WorktreeCascaded { worktree: b }) => {
                a == b
            }
            (
                Self::RegisteredWorktreeCapExceeded {
                    cap: a_cap,
                    live: a_live,
                },
                Self::RegisteredWorktreeCapExceeded {
                    cap: b_cap,
                    live: b_live,
                },
            ) => a_cap == b_cap && a_live == b_live,
            (
                Self::FenceStateUnavailable { message: a },
                Self::FenceStateUnavailable { message: b },
            ) => a == b,
            (
                Self::PeerOwnershipMismatch {
                    session: a_sid,
                    expected: a_expected,
                    actual: a_actual,
                },
                Self::PeerOwnershipMismatch {
                    session: b_sid,
                    expected: b_expected,
                    actual: b_actual,
                },
            ) => a_sid == b_sid && a_expected == b_expected && a_actual == b_actual,
            (
                Self::LineageAnchorCollision {
                    session: lhs_session,
                    existing: lhs_existing,
                    child_pid: lhs_pid,
                    child_pid_starttime: lhs_start,
                },
                Self::LineageAnchorCollision {
                    session: rhs_session,
                    existing: rhs_existing,
                    child_pid: rhs_pid,
                    child_pid_starttime: rhs_start,
                },
            ) => {
                lhs_session == rhs_session
                    && lhs_existing == rhs_existing
                    && lhs_pid == rhs_pid
                    && lhs_start == rhs_start
            }
            (
                Self::SessionCapExceeded {
                    worktree: a_wt,
                    cap: a_cap,
                    live: a_live,
                },
                Self::SessionCapExceeded {
                    worktree: b_wt,
                    cap: b_cap,
                    live: b_live,
                },
            ) => a_wt == b_wt && a_cap == b_cap && a_live == b_live,
            _ => false,
        }
    }
}

impl Eq for RegistryError {}

/// Outcome of [`SessionRegistry::attribute_path`] — used by INTD-004
/// (watcher integration) to decide whether a change goes through the
/// owning session's enforcement pipeline or through INTD-010's
/// unregistered-change path.
///
/// `Unknown` carries no payload because the watcher / fan-out code
/// builds the `attribution: unknown-agent` envelope from the change
/// itself. Treating "unknown" and "no record" identically here keeps
/// the call sites linear: `match attribute_path { Owned(s) => ..., Unknown => ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    windows,
    allow(
        clippy::large_enum_variant,
        reason = "Windows-only clippy debt baselined by CIB-204; clearing it restructures named-pipe transport code that only a Windows runner can build and test."
    )
)]
pub enum Attribution {
    /// The change lives under a registered session's worktree.
    Owned { session: SessionRecord },
    /// No registered session claims this path. The watcher routes the
    /// change through INTD-010's `attribution: unknown-agent` pipeline,
    /// honouring the `on_ambiguous_ownership` policy from INTD-008
    /// (hard-capped at `Fence` per AD-3).
    Unknown,
}

/// Process info that the launcher feeds back into the registry once it
/// has spawned the agent and has a pid / pgid to report. `None` fields
/// mean "no update" — they do not clobber an existing `Some`. To
/// explicitly clear a value, the caller must surface a separate API,
/// which v1 has no use for.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessInfo {
    pub pid: Option<u32>,
    /// Unix process group id. Always `None` on Windows; the Job Object
    /// handle is tracked out-of-band by INTD-006.
    pub pgid: Option<i32>,
    /// Override the start time recorded at registration. Reserved for
    /// the launcher reconciliation flow where the daemon was restarted
    /// mid-session and the old start time is more accurate than the
    /// re-registered one. Day-to-day usage should leave this `None`.
    pub started_at_unix: Option<u64>,
}

/// Trait the IPC listener (INTD-002) calls into. Keeps the registry's
/// public surface minimal and lets the listener depend on
/// `Arc<dyn SessionDispatcher>` without binding to the concrete type.
///
/// Lives in `registry.rs` rather than the proto crate because proto is
/// wire types only; this is a daemon-internal extension point.
///
/// MLP2-023: `register` accepts an optional `agent_tag`. Pre-MLP2-023
/// envelopes deserialise with `agent_tag: None`, preserving the
/// historical "one session per worktree" path; MLP2-023+ envelopes
/// supplying a tag opt into the per-task fence isolation that
/// MLP2-024 / -025 / -026 build on. `None` callers retain the
/// pre-MLP2-023 semantics exactly.
pub trait SessionDispatcher: Send + Sync + 'static {
    /// Register a session. The legacy MLP2-023 surface is preserved
    /// — supplying `lineage = None` takes the pre-MLP2-025b path.
    ///
    /// **MLP2-025b:** when `lineage` is `Some`, the implementor seeds
    /// the registry's `(pid, pid_starttime)` lineage index so the
    /// daemon's write-time spoof cross-check can find this session
    /// later. The lineage anchor identifies the launcher's own
    /// process; trust comes from the launcher being in the daemon's
    /// trust zone (see
    /// `plans/specs/2026-05-16-mlp2-025-spoof-cross-check-control-lane.md`
    /// §7).
    fn register(
        &self,
        id: &SessionId,
        worktree: &Path,
        agent_tag: Option<&AgentTag>,
        lineage: Option<&LineageAnchor>,
    ) -> Result<(), RegistryError>;
    /// Refresh a session's heartbeat. CIB-153: bound to the
    /// registering peer — implementations that maintain a lineage
    /// anchor (the canonical [`SessionRegistry`] impl) reject a
    /// `peer_pid` that does not match the launcher pid stamped at
    /// `register` time, and fail closed (`peer_pid = None`, or a
    /// session with no anchor) exactly like [`Self::report_process`].
    /// Anchorless implementations (`NoopDispatcher`, test recorders)
    /// treat the parameter as a no-op.
    fn heartbeat(&self, id: &SessionId, peer_pid: Option<u32>) -> Result<(), RegistryError>;
    /// Remove a session. CIB-153: bound to the registering peer with
    /// the same ownership contract as [`Self::heartbeat`] — a
    /// same-UID neighbour that guesses a session id cannot
    /// force-unregister a session it never registered.
    fn unregister(&self, id: &SessionId, peer_pid: Option<u32>) -> Result<bool, RegistryError>;
    fn list(&self) -> Vec<SessionRecord>;

    /// MLP2-074: narrow the session's lineage anchor onto the
    /// spawned child's `(pid, pid_starttime)`. The dispatcher must
    /// reject calls whose authenticated `peer_pid` does not match
    /// the launcher pid stamped at `register` time —
    /// [`SessionRegistry::update_lineage_anchor`] enforces this in
    /// the canonical impl. Implementations that do not maintain a
    /// lineage index (`NoopDispatcher`, test recorders) may treat
    /// the call as a no-op as long as the IPC dispatch arm continues
    /// to fail closed on missing peer credentials.
    fn report_process(
        &self,
        id: &SessionId,
        child_pid: u32,
        child_pid_starttime: u64,
        peer_pid: u32,
    ) -> Result<(), RegistryError>;
}

/// In-memory session registry. Cheap to clone via `Arc`; the internal
/// state is guarded by a single `std::sync::Mutex` because every
/// operation is a small `HashMap` mutation under microseconds — the
/// extra dependency surface of `parking_lot` is not justified at this
/// scale, and `std::sync::Mutex` poisons cleanly under panic which
/// gives us a cheap crash-safety property.
pub struct SessionRegistry {
    inner: Mutex<Inner>,
    ttl: Duration,
    /// MLP2-024: per-worktree session cap. Default
    /// [`DEFAULT_PER_WORKTREE_CAP`] (16); set by the daemon's
    /// startup from `Resolved::session_per_worktree_max`. Tests
    /// override via [`SessionRegistry::with_per_worktree_cap`].
    per_worktree_cap: usize,
    /// ACTMO-014: cap on the number of **distinct** durably-registered
    /// worktrees (the persisted membership set). Default
    /// [`DEFAULT_REGISTERED_WORKTREE_CAP`] (64); the daemon overrides via
    /// [`SessionRegistry::with_registered_worktree_cap`]. Bounds the otherwise
    /// unbounded persisted set; only a *new* durable worktree past the cap is
    /// refused (a refresh of an existing member is always admitted).
    registered_worktree_cap: usize,
    /// ACTMO-014 (ADR-094 decision 7): opt-in producer for durable-membership
    /// changes — register / unregister / reaper-drop of an activation-spine
    /// registration. DSV-046's headless driver subscribes here to attach /
    /// detach observation per worktree; ACTMO-013 only owns the *signal*, not
    /// the driver. A `OnceLock` mirroring [`Self::unregister_hook`]: installed
    /// post-construction via [`SessionRegistry::set_membership_hook`]. Empty by
    /// default, so callers that do not need the seam pay nothing.
    membership_hook: OnceLock<MembershipHook>,
    /// MLP2-057 / DSV: opt-in hook fired with the canonical worktree
    /// path each time a session is unregistered (deliberate
    /// `unregister` or TTL-driven `evict_stale`). The daemon wires
    /// this to [`crate::save_time::SaveTimeState::invalidate`] (drop a
    /// worktree's warm graph cache + assurance machine on the last
    /// session leaving) — and, when MLP2-014 lands the rule cache, the
    /// same closure additionally calls
    /// [`crate::rule_cache::RuleSetCache::invalidate`]. A `OnceLock`
    /// (not `Option`) so the daemon can install the hook *after* it has
    /// `Arc`-wrapped the registry, via
    /// [`SessionRegistry::set_unregister_hook`] — the warm cache it
    /// reclaims is constructed later in `run_foreground` than the
    /// registry. Empty by default; embedded-mode tests that construct
    /// no cache aren't forced to plumb one through.
    unregister_hook: OnceLock<WorktreeUnregisterHook>,
}

/// MLP2-057: callback fired with a canonical worktree path when the
/// **last** session for that worktree leaves the registry — via
/// `unregister` or TTL-driven `evict_stale`. It fires once per worktree
/// that has fully drained, NOT once per removed session: a worktree with
/// a surviving peer session (MLP2-023 multi-tag) is not signalled, so a
/// consumer can treat the call as "this worktree's per-worktree state is
/// now reclaimable" (DSV-040).
///
/// The hook fires **outside** the registry's internal lock (see
/// `unregister` / `evict_stale`), so it may take its own separate locks;
/// keep the body short regardless. The daemon composes its per-worktree
/// invalidators here — today [`crate::save_time::SaveTimeState::invalidate`],
/// and [`crate::rule_cache::RuleSetCache::invalidate`] when MLP2-014 lands —
/// each `O(1)` under its own mutex.
pub type WorktreeUnregisterHook = Arc<dyn Fn(&Path) + Send + Sync>;

/// ACTMO-014 (ADR-094 decision 7): the kind of durable-membership transition
/// reported to a [`MembershipHook`]. The registry is the **sole producer** of
/// these events; DSV-046's headless driver is the intended consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipChange {
    /// A worktree entered the durable membership set (first durable session
    /// for that canonical path).
    Registered,
    /// A worktree left the durable set by explicit `unregister` of its last
    /// durable session.
    Unregistered,
    /// A worktree left the durable set because the reaper found its directory
    /// gone (e.g. `git worktree remove`).
    Reaped,
}

/// ACTMO-014: callback fired with a [`MembershipChange`] and the canonical
/// worktree path each time the durable membership set changes. Like
/// [`WorktreeUnregisterHook`] it fires **outside** the registry's internal
/// lock and should stay short. Only durable (activation-spine) membership
/// transitions fire it — live agent-session leases do not.
pub type MembershipHook = Arc<dyn Fn(MembershipChange, &Path) + Send + Sync>;

struct Inner {
    /// `SessionId` -> record. Sole source of truth for the record body.
    sessions: HashMap<SessionId, RegistryEntry>,
    /// Composite `(canonical worktree, Option<AgentTag>)` -> session id
    /// (MLP2-023). Index for the "single session per (worktree, tag)"
    /// constraint and for `attribute_path` / `sessions_for_worktree`.
    ///
    /// The `Option<AgentTag>` half makes untagged registrations (the
    /// pre-MLP2-023 path) share the same map as tagged ones —
    /// `(wt, None)` and `(wt, Some(tag))` are different keys. Two
    /// distinct tags can coexist on the same worktree; a second
    /// untagged registration on the same worktree still returns
    /// `WorktreeAlreadyOwned`.
    by_composite: HashMap<(PathBuf, Option<AgentTag>), SessionId>,
    /// MLP2-025: `(pid, pid_starttime)` -> session id. The lineage
    /// anchor used by [`SessionRegistry::lookup_tag_for_lineage`] for
    /// the spoof cross-check. Populated by
    /// [`SessionRegistry::register_with_lineage`]; the legacy
    /// [`SessionRegistry::register`] path leaves it untouched (those
    /// sessions are not reachable by lineage lookup).
    ///
    /// Keying on the `(pid, pid_starttime)` pair, not bare PID, is the
    /// anti-spoof guarantee: after a launcher exits, its PID may be
    /// reused by a hostile process, but the new incarnation's
    /// `pid_starttime` will differ and the lookup will miss.
    by_pid_lineage: HashMap<(u32, u64), SessionId>,
}

struct RegistryEntry {
    record: SessionRecord,
    /// Monotonic timestamp the registry compares against the TTL.
    /// `Instant` is monotonic on every supported platform, so unlike
    /// `last_heartbeat_unix` (wall-clock, can jump backwards on
    /// NTP step) it is safe for liveness checks.
    last_heartbeat: Instant,
    /// MLP2-071 (INTD-015 wire-up): opaque post-mint `SubscriberId`
    /// string for the peer that owns this session's telemetry. Set by
    /// the IPC accept-loop at `RegisterSession` time using the
    /// connecting peer's credentials (`SO_PEERCRED` / equivalent);
    /// NEVER set from a wire-supplied field, mirroring the MLP2-070
    /// lineage-anchor pattern. `None` for sessions registered through
    /// code paths that do not (yet) carry peer credentials —
    /// embedded mode, the legacy register path, tests that drive
    /// `SessionRegistry::register` directly. Phase E wires the IPC
    /// accept-loop call; Phase D adds the field + accessors.
    subscriber_binding: Option<String>,
    /// CIB-153: the authenticated pid of the peer that **registered**
    /// this session (the launcher). Stamped once at
    /// [`SessionRegistry::register_with_lineage`] from the daemon's
    /// `SO_PEERCRED` view of the connecting peer — never a wire value —
    /// and, unlike `record.pid`, is **not** narrowed by
    /// [`SessionRegistry::update_lineage_anchor`] when the launcher
    /// reports its spawned child. It is the stable owner identity the
    /// session-lifecycle ownership check ([`SessionRegistry::peer_ownership_check`])
    /// binds `Heartbeat` / `UnregisterSession` to, so the launcher can
    /// still manage the session after `report_process` has re-pointed
    /// the lineage anchor at the child. `None` for the lineage-less
    /// `register` path (no authenticated launcher was ever attributed):
    /// such sessions are **exempt** from the ownership check and keep
    /// same-uid socket permission as their authorization boundary,
    /// because they have no single owner pid to bind to — durable
    /// worktree memberships are separate one-shot CLI processes, and
    /// Windows registers `None` pending CIB-114. See
    /// [`SessionRegistry::peer_ownership_check`] for the full reasoning.
    /// Distinct from `subscriber_binding` (telemetry ownership) per the
    /// CIB-153 Intent.
    launcher_pid: Option<u32>,
    /// ACTMO-014: `true` when this entry is **durable worktree membership**
    /// (registered with an activation-spine [`AgentTag`], see
    /// [`AgentTag::is_durable_membership`]) rather than a live agent-session
    /// lease. Durable entries are exempt from [`SessionRegistry::evict_stale`]
    /// and are the set persisted under `ANVIL_HOME` and reloaded on startup.
    durable: bool,
}

impl Inner {
    /// ACTMO-014: `true` when at least one durable session already owns this
    /// canonical worktree — i.e. the worktree is already a durable member.
    fn is_durable_member(&self, canonical: &Path) -> bool {
        self.sessions
            .values()
            .any(|entry| entry.durable && entry.record.worktree == canonical)
    }

    /// ACTMO-014: the number of **distinct** worktrees in the durable
    /// membership set, the quantity bounded by `registered_worktree_cap`.
    fn distinct_durable_worktrees(&self) -> usize {
        self.sessions
            .values()
            .filter(|entry| entry.durable)
            .map(|entry| &entry.record.worktree)
            .collect::<HashSet<_>>()
            .len()
    }
}

impl SessionRegistry {
    /// Construct a registry with the default 30 s TTL.
    #[must_use]
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_HEARTBEAT_TTL)
    }

    /// Construct a registry with a custom TTL. Tests and embedded-mode
    /// callers (INTD-009) use this; the daemon entry point sticks with
    /// the default.
    #[must_use]
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(Inner {
                sessions: HashMap::new(),
                by_composite: HashMap::new(),
                by_pid_lineage: HashMap::new(),
            }),
            ttl,
            per_worktree_cap: DEFAULT_PER_WORKTREE_CAP,
            registered_worktree_cap: DEFAULT_REGISTERED_WORKTREE_CAP,
            membership_hook: OnceLock::new(),
            unregister_hook: OnceLock::new(),
        }
    }

    /// MLP2-057: builder-style hook registration. The hook fires
    /// once per session removed via `unregister` or `evict_stale`,
    /// receiving the unregistered session's canonical worktree
    /// path. The daemon's `run_foreground` installs the hook through
    /// the post-construction [`SessionRegistry::set_unregister_hook`]
    /// instead (the warm cache it reclaims outlives this builder);
    /// this builder remains the seam tests use to drive the hook on a
    /// freshly-constructed registry.
    ///
    /// Calling this method a second time replaces the prior hook —
    /// the builder owns `self`, so it resets the `OnceLock`. Only one
    /// hook is supported, since the daemon's call site composes every
    /// invalidator into a single closure.
    #[must_use]
    pub fn with_unregister_hook(mut self, hook: WorktreeUnregisterHook) -> Self {
        self.unregister_hook = OnceLock::new();
        // The lock was just reset, so the set always succeeds; assert it so a
        // future refactor that drops the reset can't silently no-op the hook.
        debug_assert!(
            self.unregister_hook.set(hook).is_ok(),
            "freshly-reset unregister hook OnceLock must accept the hook",
        );
        self
    }

    /// MLP2-057 / DSV: install the unregister hook on an already
    /// `Arc`-wrapped registry. Unlike the consuming
    /// [`SessionRegistry::with_unregister_hook`] builder, this works
    /// after construction — `run_foreground` builds the registry
    /// first and the warm [`crate::save_time::SaveTimeState`] the hook
    /// reclaims only later, so the hook cannot be supplied at build
    /// time.
    ///
    /// Set-once: returns `true` if the hook was installed, `false` if
    /// one was already present (the caller composes every invalidator
    /// into a single closure, so a second install is a wiring bug, not
    /// a runtime condition).
    pub fn set_unregister_hook(&self, hook: WorktreeUnregisterHook) -> bool {
        self.unregister_hook.set(hook).is_ok()
    }

    /// MLP2-024: builder-style override of the per-worktree cap.
    /// The daemon wires this from
    /// `Resolved::session_per_worktree_max` at startup; tests use it
    /// to drive the cap-exceeded path on a small fixture.
    ///
    /// `cap` is clamped to a minimum of 1 — a zero cap would refuse
    /// every registration and is treated as an operator typo. The
    /// resolution layer in `crate::config` applies the same clamp
    /// upstream so the registry receives a sane value, but the
    /// defensive clamp here keeps the surface honest for hand-built
    /// fixtures.
    #[must_use]
    pub fn with_per_worktree_cap(mut self, cap: usize) -> Self {
        self.per_worktree_cap = cap.max(1);
        self
    }

    /// ACTMO-014: builder-style override of the distinct-registered-worktree
    /// cap. The daemon wires this from config at startup; tests use it to drive
    /// the cap-exceeded path on a small fixture. Clamped to a minimum of 1,
    /// mirroring [`Self::with_per_worktree_cap`].
    #[must_use]
    pub fn with_registered_worktree_cap(mut self, cap: usize) -> Self {
        self.registered_worktree_cap = cap.max(1);
        self
    }

    /// ACTMO-014 (ADR-094 decision 7): install the durable-membership hook on
    /// an already `Arc`-wrapped registry. Set-once, mirroring
    /// [`Self::set_unregister_hook`]: returns `true` if installed, `false` if a
    /// hook was already present.
    pub fn set_membership_hook(&self, hook: MembershipHook) -> bool {
        self.membership_hook.set(hook).is_ok()
    }

    /// ACTMO-014: fire the membership hook outside the registry lock. No-op
    /// when no hook is installed (the common cut-line posture until DSV-046
    /// wires a consumer).
    fn signal_membership(&self, change: MembershipChange, worktree: &Path) {
        if let Some(hook) = self.membership_hook.get() {
            hook(change, worktree);
        }
    }

    /// Register a new session against a worktree path.
    ///
    /// **Canonicalisation policy:** the worktree is run through
    /// `std::fs::canonicalize` before use as a registry key, so two
    /// clients spelling the same worktree differently (trailing slash,
    /// `..` segments, symlinks) cannot each "own" the same worktree.
    /// A path that does not exist yields
    /// [`RegistryError::WorktreePathInvalid`] — v1 refuses to register
    /// sessions for missing worktrees rather than silently storing a
    /// relative path.
    ///
    /// **Crash-safety:** crashed launchers (where `Drop`-fired
    /// unregister never runs because the process was `SIGKILL`-ed or
    /// `TerminateProcess`-ed) are NOT handled here. They are evicted
    /// by [`SessionRegistry::evict_stale`], which the daemon ticks
    /// every 250 ms.
    ///
    /// **MLP2-023 composite-key semantics.** The uniqueness invariant
    /// is one session per `(canonical worktree, Option<AgentTag>)`.
    /// Concretely:
    ///
    /// - `(wt, None)` — the pre-MLP2-023 path. Only one untagged
    ///   session per worktree. A second untagged registration returns
    ///   [`RegistryError::WorktreeAlreadyOwned`].
    /// - `(wt, Some(tag_a))` and `(wt, Some(tag_b))` — two tagged
    ///   sub-agents on the same worktree coexist. Per-task fence
    ///   keying (MLP2-026) distinguishes them.
    /// - `(wt, None)` plus `(wt, Some(tag))` also coexist: the
    ///   untagged session represents worktree-level enforcement
    ///   context, while the tagged session represents a specific
    ///   sub-agent.
    ///
    /// Reusing a `SessionId` already in the registry — even against a
    /// different `(worktree, tag)` — still returns
    /// [`RegistryError::SessionAlreadyExists`]; the caller must
    /// `unregister` the old id first.
    pub fn register(
        &self,
        id: &SessionId,
        worktree: &Path,
        agent_tag: Option<&AgentTag>,
        now: Instant,
    ) -> Result<SessionRecord, RegistryError> {
        let canonical = canonicalise(worktree)?;
        let cap = self.per_worktree_cap;
        let registered_cap = self.registered_worktree_cap;
        // ACTMO-014: an activation-spine tag marks durable membership, which is
        // TTL-exempt and persisted. Any other tag (or none) is a live lease.
        //
        // CIB-150 trust model: this predicate keys durability off the tag
        // verbatim, but `AgentTag` is not authenticated identity — any
        // same-UID process can mint the activation-spine `claimed_agent_id`.
        // The IPC dispatch surface (`ipc::verify_durable_membership_claim`)
        // is the trust boundary: it authorises a durable claim against the
        // connection's authenticated peer (the peer must run the daemon's own
        // `anvil` binary) and downgrades an unauthorised claim to a live tag
        // *before* calling `register`. The daemon's in-process
        // `register_on_start` path is already trusted (it does not cross the
        // wire), so it may pass a durable tag directly.
        let durable = agent_tag.is_some_and(AgentTag::is_durable_membership);

        let (record, signal_registered) = {
            let mut inner = self.lock();

            if inner.sessions.contains_key(id) {
                return Err(RegistryError::SessionAlreadyExists(id.clone()));
            }
            let composite_key = (canonical.clone(), agent_tag.cloned());
            if let Some(existing) = inner.by_composite.get(&composite_key) {
                return Err(RegistryError::WorktreeAlreadyOwned {
                    existing: existing.clone(),
                });
            }

            // MLP2-024: per-worktree session cap. This bounds the number of
            // **live agent sessions** concurrently active on one worktree (the
            // save-time concurrency budget), counted across all agent_tags.
            //
            // ACTMO-014 (adversarial review F4): durable activation-spine
            // membership is exempt. A durable registration is a persisted
            // membership record, not a live agent that saves files, so (a) it
            // does not count toward the live budget and (b) registering it is
            // never refused by this cap — durable membership is bounded by the
            // separate `registered_worktree_cap` instead. This keeps the full
            // `cap` live-agent slots available on a registered worktree.
            if !durable {
                let live: usize = inner
                    .sessions
                    .values()
                    .filter(|entry| !entry.durable && entry.record.worktree == canonical)
                    .count();
                if live >= cap {
                    return Err(RegistryError::SessionCapExceeded {
                        worktree: canonical,
                        cap,
                        live,
                    });
                }
            }

            // ACTMO-014: distinct durable-worktree membership cap. Only a NEW
            // durable worktree past the cap is refused — refreshing an
            // already-registered worktree (or adding a peer session to it) is
            // always admitted, so the cap bounds the persisted set without
            // making re-registration flaky near the ceiling.
            let already_member = durable && inner.is_durable_member(&canonical);
            if durable && !already_member {
                // Compute the distinct-worktree count once (it scans sessions).
                let live_durable = inner.distinct_durable_worktrees();
                if live_durable >= registered_cap {
                    return Err(RegistryError::RegisteredWorktreeCapExceeded {
                        cap: registered_cap,
                        live: live_durable,
                    });
                }
            }

            let now_unix = unix_seconds_now();
            let record = SessionRecord {
                id: id.clone(),
                worktree: canonical.clone(),
                pid: None,
                pgid: None,
                started_at_unix: now_unix,
                last_heartbeat_unix: now_unix,
                status: SessionStatus::Active,
                agent_tag: agent_tag.cloned(),
                // MLP2-025: populated by `lookup_tag_for_lineage` /
                // `cross_check_env_tag` plumbing in subsequent subtasks (A3,
                // A4). Until then `None` mirrors the legacy untagged path.
                daemon_issued_tag: None,
            };

            inner.sessions.insert(
                id.clone(),
                RegistryEntry {
                    record: record.clone(),
                    last_heartbeat: now,
                    subscriber_binding: None,
                    // CIB-153: the lineage-less `register` path carries
                    // no authenticated launcher pid — sessions registered
                    // this way have no single owner pid to bind to and
                    // are exempt from the `Heartbeat` /
                    // `UnregisterSession` ownership check (durable
                    // memberships are separate one-shot CLI processes;
                    // Windows registers `None` pending CIB-114). The
                    // lineage path (`register_with_lineage`) stamps
                    // `Some(pid)` below and gets strict enforcement.
                    launcher_pid: None,
                    durable,
                },
            );
            inner.by_composite.insert(composite_key, id.clone());
            // Signal a membership *gain* only when this is the first durable
            // session for the worktree — symmetric with the "last session
            // leaves" rule on `unregister` / `evict_stale`.
            (record, durable && !already_member)
        };

        if signal_registered {
            self.signal_membership(MembershipChange::Registered, &canonical);
        }
        Ok(record)
    }

    /// MLP2-025: register a session with a lineage anchor. Mirrors
    /// [`Self::register`] but additionally captures the daemon-issued
    /// `AgentTag`, the registering PID, and that PID's
    /// `pid_starttime`. The `(pid, pid_starttime)` pair is the
    /// authoritative anchor for [`Self::lookup_tag_for_lineage`] —
    /// PID reuse after a launcher exit produces a different
    /// `pid_starttime` and the lookup misses by design.
    ///
    /// Until the daemon control-lane wire-up (MLP2-025 subtask A5),
    /// the legacy [`Self::register`] path is what most call sites use;
    /// this method exists so the daemon can opt in to the lineage
    /// index at register time without breaking the legacy callers.
    #[allow(clippy::too_many_arguments)] // surface mirrors the daemon's call site verbatim
    pub fn register_with_lineage(
        &self,
        id: &SessionId,
        worktree: &Path,
        agent_tag: Option<&AgentTag>,
        daemon_issued_tag: Option<&AgentTag>,
        pid: u32,
        pid_starttime: u64,
        now: Instant,
    ) -> Result<SessionRecord, RegistryError> {
        // Delegate the worktree-uniqueness and cap checks to the
        // legacy path, then overwrite the daemon-issued tag and PID
        // fields, and finally seed the lineage index.
        let mut record = self.register(id, worktree, agent_tag, now)?;
        record.daemon_issued_tag = daemon_issued_tag.cloned();
        record.pid = Some(pid);
        record.started_at_unix = pid_starttime;
        {
            let mut inner = self.lock();
            if let Some(entry) = inner.sessions.get_mut(id) {
                entry.record = record.clone();
                // CIB-153: bind the registering peer as the stable
                // lifecycle owner. `pid` is the launcher's authenticated
                // pid (dispatch's `verify_lineage_claim` constrains the
                // wire anchor to `peer_pid`), and this field survives the
                // `update_lineage_anchor` narrowing to the child.
                entry.launcher_pid = Some(pid);
            }
            inner
                .by_pid_lineage
                .insert((pid, pid_starttime), id.clone());
        }
        Ok(record)
    }

    /// MLP2-025: pure lookup helper. Given a `(pid, pid_starttime)`
    /// pair, return the daemon-issued tag of any registered session
    /// whose lineage anchor matches. The pair must match exactly:
    /// PID reuse with a different `pid_starttime` returns `None`.
    ///
    /// This is the building block for
    /// [`Self::lookup_tag_for_lineage`], which composes it with the
    /// real `anvil_attribution::walk::walk_ancestors` pass to walk a
    /// writer's PID lineage. Tests can target this helper directly
    /// with synthetic anchors so they stay platform-portable.
    #[must_use]
    pub fn lookup_tag_by_pid_starttime(&self, pid: u32, pid_starttime: u64) -> Option<AgentTag> {
        let inner = self.lock();
        let sid = inner.by_pid_lineage.get(&(pid, pid_starttime))?;
        inner.sessions.get(sid)?.record.daemon_issued_tag.clone()
    }

    /// MLP2-025: walk the writer's PID lineage and return the
    /// daemon-issued tag of any registered ancestor. Each ancestor's
    /// `pid_starttime` is read live via
    /// `anvil_attribution::process::pid_starttime` and compared
    /// against the lineage index — a PID match with a stale
    /// `pid_starttime` (the canonical PID-reuse spoof) is rejected.
    ///
    /// Returns `None` when the walk reaches init without a match,
    /// when the depth cap fires, or when an ancestor's process info
    /// cannot be read. Failure-to-read is treated as "not matched"
    /// rather than as an error because the cross-check path must be
    /// best-effort: a missing `pid_starttime` for an ancestor (e.g.
    /// the process exited mid-walk) cannot grant attribution.
    #[must_use]
    pub fn lookup_tag_for_lineage(&self, start_pid: u32) -> Option<AgentTag> {
        use anvil_attribution::process::pid_starttime;
        use anvil_attribution::walk::{DEFAULT_MAX_DEPTH, WalkOutcome, walk_ancestors};

        // Snapshot the lineage index so the walk (which may probe
        // `/proc` repeatedly) does not hold the registry lock.
        let snapshot: HashMap<(u32, u64), SessionId> = {
            let inner = self.lock();
            inner.by_pid_lineage.clone()
        };

        let outcome = walk_ancestors(start_pid, DEFAULT_MAX_DEPTH, |pid| {
            let starttime = pid_starttime(pid).ok()?;
            snapshot.get(&(pid, starttime)).cloned()
        })
        .ok()?;

        let matched_sid = match outcome {
            WalkOutcome::Matched { value, .. } => value,
            WalkOutcome::ReachedRoot | WalkOutcome::DepthExhausted { .. } => return None,
        };

        let inner = self.lock();
        inner
            .sessions
            .get(&matched_sid)?
            .record
            .daemon_issued_tag
            .clone()
    }

    /// MLP2-025: classify an env-supplied `AgentTag` against the
    /// daemon-issued tag found on the writer's PID lineage.
    /// Convenience wrapper that composes
    /// [`Self::lookup_tag_for_lineage`] with [`Cross::classify`].
    ///
    /// Returns [`Cross::Untagged`] when `env_tag` is `None`,
    /// [`Cross::Match`] when the env tag matches a registered ancestor,
    /// and [`Cross::Spoofed`] when the env tag is present but no
    /// matching daemon-issued tag was found on the lineage.
    #[must_use]
    pub fn cross_check_env_tag(&self, env_tag: Option<&AgentTag>, writer_pid: u32) -> Cross {
        let registered = self.lookup_tag_for_lineage(writer_pid);
        Cross::classify(env_tag, registered.as_ref())
    }

    /// MLP2-025b: walk the writer's PID lineage and return the
    /// **worktree** of any registered ancestor, regardless of tag
    /// match. Differs from [`Self::lookup_tag_for_lineage`] in that
    /// it does not care which tag the ancestor was registered with —
    /// it just needs to find SOME registered session on the lineage
    /// so the daemon control-lane has a worktree to fence on
    /// `Cross::Spoofed`.
    ///
    /// Returns `None` when no ancestor is registered at all. The
    /// caller (the daemon control-lane) falls back to the file's
    /// canonical parent directory in that case.
    #[must_use]
    pub fn worktree_for_lineage(&self, start_pid: u32) -> Option<PathBuf> {
        use anvil_attribution::process::pid_starttime;
        use anvil_attribution::walk::{DEFAULT_MAX_DEPTH, WalkOutcome, walk_ancestors};

        // Snapshot the lineage index + sessions so the walk runs
        // outside the registry lock. The visitor returns the
        // SessionId on first match; we look up the worktree
        // afterwards under a fresh lock acquisition.
        let snapshot: HashMap<(u32, u64), SessionId> = {
            let inner = self.lock();
            inner.by_pid_lineage.clone()
        };

        let outcome = walk_ancestors(start_pid, DEFAULT_MAX_DEPTH, |pid| {
            let starttime = pid_starttime(pid).ok()?;
            snapshot.get(&(pid, starttime)).cloned()
        })
        .ok()?;

        let matched_sid = match outcome {
            WalkOutcome::Matched { value, .. } => value,
            WalkOutcome::ReachedRoot | WalkOutcome::DepthExhausted { .. } => return None,
        };

        let inner = self.lock();
        Some(inner.sessions.get(&matched_sid)?.record.worktree.clone())
    }

    /// CLAWP-065: walk the writer's PID lineage and return the
    /// `SessionId` of any registered ancestor, regardless of tag.
    /// Shares the `(pid, pid_starttime)` anti-PID-reuse guarantee with
    /// [`Self::worktree_for_lineage`] and [`Self::lookup_tag_for_lineage`]
    /// — a reused PID with a stale `pid_starttime` misses. The daemon's
    /// `scan_buffer` session-ownership check (`ipc::scan_buffer_from_jsonrpc`)
    /// uses this to bind a request's claimed `session_id` to the
    /// authenticated peer lineage of the connection it arrived on:
    /// when the resolved owner differs from the claim, the write is a
    /// cross-session forgery and is rejected.
    ///
    /// Returns `None` when no registered session sits on the lineage
    /// (walk reaches init, the depth cap fires, or an ancestor's
    /// process info cannot be read). The caller treats `None` as a
    /// failed binding and fails closed.
    #[must_use]
    pub fn session_for_lineage(&self, start_pid: u32) -> Option<SessionId> {
        use anvil_attribution::process::pid_starttime;
        use anvil_attribution::walk::{DEFAULT_MAX_DEPTH, WalkOutcome, walk_ancestors};

        // Snapshot the lineage index so the `/proc` walk runs outside
        // the registry lock (mirrors `worktree_for_lineage`).
        let snapshot: HashMap<(u32, u64), SessionId> = {
            let inner = self.lock();
            inner.by_pid_lineage.clone()
        };

        let outcome = walk_ancestors(start_pid, DEFAULT_MAX_DEPTH, |pid| {
            let starttime = pid_starttime(pid).ok()?;
            snapshot.get(&(pid, starttime)).cloned()
        })
        .ok()?;

        let matched_sid = match outcome {
            WalkOutcome::Matched { value, .. } => value,
            WalkOutcome::ReachedRoot | WalkOutcome::DepthExhausted { .. } => return None,
        };

        // Re-acquire the lock and confirm the matched session is still
        // live — the lineage snapshot was taken before the walk, so an
        // `unregister` (which also clears `by_pid_lineage`) racing the
        // walk could otherwise let a just-evicted session authorise a
        // scan. Mirrors `worktree_for_lineage` / `lookup_tag_for_lineage`,
        // and keeps the ownership check fail-closed.
        let inner = self.lock();
        inner.sessions.get(&matched_sid).map(|_| matched_sid)
    }

    /// MLP2-074: narrow the lineage anchor from the launcher's
    /// `(pid, pid_starttime)` to the spawned child's. The launcher
    /// calls `session.report_process` after spawn; the daemon
    /// authenticates the call via the peer's authenticated pid
    /// (`SO_PEERCRED` / equivalent) against the launcher pid stamped
    /// on the session record at `register_with_lineage` time, then
    /// drops the old `by_pid_lineage` entry and inserts a fresh one
    /// keyed on the child's anchor. The record's `pid` /
    /// `started_at_unix` fields follow the swap so MLP-014's
    /// PID-reuse defence compares against the child rather than the
    /// wrapping launcher.
    ///
    /// Returns:
    /// - `RegistryError::UnknownSession` if `id` is not registered
    ///   (or has been evicted between the launcher's calls).
    /// - `RegistryError::PeerOwnershipMismatch` when the
    ///   authenticated `peer_pid` does not match the launcher pid
    ///   stamped on the session, or when the session was registered
    ///   via the legacy lineage-less `register` path (no anchor was
    ///   ever taken, so no narrowing can be attributed back to a
    ///   known launcher).
    ///
    /// Trust model: the launcher pid trust is anchored at register
    /// time by MLP2-070's `verify_lineage_claim`, which already
    /// requires `claim.pid == peer_pid`. By the time this method
    /// runs, `record.pid` is guaranteed to be the daemon's view of
    /// the authenticated launcher pid — never a client-supplied
    /// value — so the same-UID-neighbour-reattaches-someone-else's-
    /// session forgery vector is closed without needing to re-walk
    /// the registration trail.
    pub fn update_lineage_anchor(
        &self,
        id: &SessionId,
        child_pid: u32,
        child_pid_starttime: u64,
        peer_pid: u32,
    ) -> Result<SessionRecord, RegistryError> {
        let mut inner = self.lock();
        // All validity checks run BEFORE we touch `entry.record` so
        // a rejection leaves both the record and the lineage index
        // untouched (PR #1895 review). Look up the session
        // immutably first to grab the launcher anchor we need for
        // both the ownership check and the lineage-index remove,
        // then take a mutable borrow only when we have committed
        // to the mutation.
        let (launcher_pid_opt, launcher_starttime) = {
            let entry = inner
                .sessions
                .get(id)
                .ok_or_else(|| RegistryError::UnknownSession(id.clone()))?;
            (entry.record.pid, entry.record.started_at_unix)
        };

        if launcher_pid_opt != Some(peer_pid) {
            return Err(RegistryError::PeerOwnershipMismatch {
                session: id.clone(),
                expected: launcher_pid_opt,
                actual: peer_pid,
            });
        }
        // Per the check above, `launcher_pid_opt` is `Some(peer_pid)`.
        let launcher_pid = peer_pid;

        // Cross-session collision defence (PR #1895 review): the
        // launcher supplies `child_pid` / `child_pid_starttime` on
        // the wire, and `child_pid` is NOT constrained to equal
        // `peer_pid`. A malicious or buggy launcher could submit a
        // pair already mapped to another session and silently
        // overwrite that session's lineage anchor. Refuse the
        // mutation if the index already maps to a different
        // session; re-narrowing to the SAME (pid, pid_starttime)
        // pair this session already owns is idempotent and falls
        // through to the swap below.
        if let Some(existing) = inner.by_pid_lineage.get(&(child_pid, child_pid_starttime))
            && existing != id
        {
            return Err(RegistryError::LineageAnchorCollision {
                session: id.clone(),
                existing: existing.clone(),
                child_pid,
                child_pid_starttime,
            });
        }

        // Validity confirmed — now mutate.
        let entry = inner
            .sessions
            .get_mut(id)
            .expect("session presence proven by the immutable lookup above");
        entry.record.pid = Some(child_pid);
        entry.record.started_at_unix = child_pid_starttime;
        let updated = entry.record.clone();

        inner
            .by_pid_lineage
            .remove(&(launcher_pid, launcher_starttime));
        inner
            .by_pid_lineage
            .insert((child_pid, child_pid_starttime), id.clone());

        Ok(updated)
    }

    /// Update process info for a registered session. `None` fields are
    /// no-ops — they do NOT clobber an existing `Some`.
    pub fn update_process_info(
        &self,
        id: &SessionId,
        info: ProcessInfo,
    ) -> Result<SessionRecord, RegistryError> {
        let mut inner = self.lock();
        let entry = inner
            .sessions
            .get_mut(id)
            .ok_or_else(|| RegistryError::UnknownSession(id.clone()))?;

        if let Some(pid) = info.pid {
            entry.record.pid = Some(pid);
        }
        if let Some(pgid) = info.pgid {
            entry.record.pgid = Some(pgid);
        }
        if let Some(start) = info.started_at_unix {
            entry.record.started_at_unix = start;
        }

        Ok(entry.record.clone())
    }

    /// CIB-153: verify that an authenticated `peer_pid` owns the
    /// session before a lifecycle mutation (`Heartbeat` /
    /// `UnregisterSession`) is allowed to proceed. Mirrors the
    /// [`Self::update_lineage_anchor`] peer-ownership contract, but
    /// binds against the **stable** launcher pid recorded at
    /// [`Self::register_with_lineage`] (`RegistryEntry::launcher_pid`)
    /// rather than the mutable `record.pid`: `report_process` narrows
    /// `record.pid` onto the spawned child, yet the launcher keeps
    /// sending the session's heartbeats and the final unregister, so
    /// the owner identity must not move with the lineage anchor.
    ///
    /// This is a **pure predicate** over an `entry` the caller has
    /// already looked up while holding the registry lock — it takes no
    /// lock of its own. Copilot review (PR #3188) flagged that running
    /// the check under one lock and the mutation under a *separate*
    /// later lock is a TOCTOU: the id could be evicted and
    /// re-registered between them, letting a stale pass authorise a
    /// mutation on a different entry. The dispatcher `heartbeat` /
    /// `unregister` paths therefore call this inline within the SAME
    /// locked critical section as the mutation, exactly as
    /// [`Self::update_lineage_anchor`] runs all checks before any
    /// mutation under a single lock.
    ///
    /// The check applies **only to sessions that carry a verified
    /// lineage anchor** (`launcher_pid == Some(_)`): those are live,
    /// single-continuous-process sessions where one launcher registers,
    /// heartbeats, and finally unregisters — the exact CIB-153 threat
    /// model of a same-UID neighbour guessing that live id. For them a
    /// mismatched or absent peer pid is rejected with
    /// [`RegistryError::PeerOwnershipMismatch`].
    ///
    /// Sessions with `launcher_pid == None` are **exempt** (returns
    /// `Ok(())`): no verified lineage claim was ever established, so
    /// there is no owner pid to bind against, and same-uid Unix-socket
    /// permission is their authorization boundary — unchanged from
    /// pre-CIB-153 behaviour. Two real cases require this:
    /// - **Durable memberships** (`anvil workspace register` /
    ///   `unregister`) are separate one-shot CLI process invocations;
    ///   `session_register_params` sends no `lineage`, so there is no
    ///   single continuously-running owner process to bind to. Binding
    ///   to any one invocation's pid would permanently strand the
    ///   membership — no later `unregister`/heartbeat process could
    ///   ever match. This is the exact non-obvious invariant that must
    ///   not be "tightened" back into a rejection by a future refactor.
    /// - **Windows** sessions: the named-pipe accept loop hardcodes
    ///   `peer_pid = None` pending CIB-114's peer-credential work, so
    ///   every Windows session registers with `launcher_pid == None`;
    ///   rejecting them would fail-close all Windows heartbeat/
    ///   unregister traffic (a pure regression vs pre-CIB-153).
    ///
    /// An unknown session id returns `Ok(())` so the downstream
    /// operation keeps its established semantics (heartbeat surfaces
    /// [`RegistryError::UnknownSession`]; unregister is an idempotent
    /// `Ok(false)` no-op) — there is no owned state to protect.
    fn peer_ownership_check(
        entry: Option<&RegistryEntry>,
        id: &SessionId,
        peer_pid: Option<u32>,
    ) -> Result<(), RegistryError> {
        let Some(entry) = entry else {
            return Ok(());
        };
        match entry.launcher_pid {
            // No verified lineage anchor: same-uid socket permission is
            // the boundary (durable memberships, Windows pending
            // CIB-114). See the doc comment for why this must stay a
            // pass-through and not a rejection.
            None => Ok(()),
            Some(owner) => match peer_pid {
                Some(caller) if owner == caller => Ok(()),
                // A `None` peer carries no pid. We surface that in the
                // error's `actual` field as `0`, used purely as a
                // sentinel for "no authenticated peer" because
                // `PeerOwnershipMismatch::actual` is a bare `u32` with
                // no `Option` to represent absence — it is NOT a claim
                // that 0 cannot be a valid pid (on Linux 0 is the
                // scheduler/swapper and never a real IPC peer, but the
                // guarantee relied on here is only that we use it as the
                // unauthenticated sentinel).
                _ => Err(RegistryError::PeerOwnershipMismatch {
                    session: id.clone(),
                    expected: Some(owner),
                    actual: peer_pid.unwrap_or(0),
                }),
            },
        }
    }

    /// Refresh the heartbeat for a session. Returns
    /// [`RegistryError::UnknownSession`] if the id is not registered
    /// (or has already been evicted).
    pub fn heartbeat(&self, id: &SessionId, now: Instant) -> Result<(), RegistryError> {
        let mut inner = self.lock();
        Self::heartbeat_locked(&mut inner, id, now)
    }

    /// Heartbeat mutation applied to an already-locked guard. Factored
    /// out so the `SessionDispatcher::heartbeat` path can run the
    /// peer-ownership check ([`Self::peer_ownership_check`]) and this
    /// mutation under a single lock (Copilot PR #3188 TOCTOU fix),
    /// while the lock-free public [`Self::heartbeat`] keeps its shape.
    fn heartbeat_locked(
        inner: &mut Inner,
        id: &SessionId,
        now: Instant,
    ) -> Result<(), RegistryError> {
        let entry = inner
            .sessions
            .get_mut(id)
            .ok_or_else(|| RegistryError::UnknownSession(id.clone()))?;
        entry.last_heartbeat = now;
        entry.record.last_heartbeat_unix = unix_seconds_now();
        Ok(())
    }

    /// Look up the record owning a worktree, if any. The caller is
    /// expected to pass an already-canonicalised path (the listener
    /// canonicalises once at the boundary); paths that fail to
    /// canonicalise yield `None`.
    ///
    /// MLP2-023: when multiple sessions are registered against the
    /// same worktree (any combination of tagged + untagged), this
    /// returns the **deterministic** first one — sorted by
    /// `started_at_unix` then `SessionId`. Callers needing all
    /// matches must use [`Self::sessions_for_worktree`] instead.
    /// The pre-MLP2-023 single-session call sites are unaffected:
    /// when only one session matches, the returned record is the
    /// same as before.
    #[must_use]
    pub fn session_for_worktree(&self, worktree: &Path) -> Option<SessionRecord> {
        let canonical = std::fs::canonicalize(worktree).ok()?;
        let inner = self.lock();
        let mut matches: Vec<&RegistryEntry> = inner
            .by_composite
            .iter()
            .filter(|((wt, _), _)| wt == &canonical)
            .filter_map(|(_, id)| inner.sessions.get(id))
            .collect();
        matches.sort_by(|a, b| {
            a.record
                .started_at_unix
                .cmp(&b.record.started_at_unix)
                .then_with(|| a.record.id.as_str().cmp(b.record.id.as_str()))
        });
        matches.first().map(|entry| entry.record.clone())
    }

    /// All sessions registered against a worktree, sorted deterministically
    /// by `started_at_unix` then `SessionId`. MLP2-023: a multi-agent
    /// worktree can host several tagged sub-agents plus an optional
    /// untagged worktree-level session; this method surfaces them all.
    #[must_use]
    pub fn sessions_for_worktree(&self, worktree: &Path) -> Vec<SessionRecord> {
        let Ok(canonical) = std::fs::canonicalize(worktree) else {
            return Vec::new();
        };
        let inner = self.lock();
        let mut records: Vec<SessionRecord> = inner
            .by_composite
            .iter()
            .filter(|((wt, _), _)| wt == &canonical)
            .filter_map(|(_, id)| inner.sessions.get(id))
            .map(|entry| entry.record.clone())
            .collect();
        records.sort_by(|a, b| {
            a.started_at_unix
                .cmp(&b.started_at_unix)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        records
    }

    /// All sessions registered against an **already-canonical** worktree,
    /// matched directly against the stored canonical key **without** a
    /// filesystem `canonicalize` round-trip. Sorted deterministically by
    /// `started_at_unix` then `SessionId`, mirroring
    /// [`Self::sessions_for_worktree`].
    ///
    /// ADR-090 (CIB-098): the ownership resolver routes a daemon-health
    /// notification that fires precisely in degraded states (disk full,
    /// EROFS, the worktree deleted/unmounted). [`Self::sessions_for_worktree`]
    /// calls `std::fs::canonicalize` and returns empty on error, so a session
    /// registered against a now-unstattable worktree path would silently lose
    /// its subscriber — defeating the notification in the exact case it is for.
    /// This method skips the `canonicalize` step entirely: the registration
    /// path already stores the canonical worktree (via [`canonicalise`]), and
    /// both production callers (`save_time.rs`, `full_scan_executor.rs`) pass
    /// an already-canonical worktree, so an exact match against the stored
    /// canonical key is correct and carries no on-disk dependency at lookup
    /// time. Mis-delivery is impossible: the match is still the same exact
    /// canonical composite/worktree key equality `sessions_for_worktree` uses.
    #[must_use]
    pub fn sessions_for_canonical_worktree(&self, worktree: &Path) -> Vec<SessionRecord> {
        let inner = self.lock();
        let mut records: Vec<SessionRecord> = inner
            .by_composite
            .iter()
            .filter(|((wt, _), _)| wt.as_path() == worktree)
            .filter_map(|(_, id)| inner.sessions.get(id))
            .map(|entry| entry.record.clone())
            .collect();
        records.sort_by(|a, b| {
            a.started_at_unix
                .cmp(&b.started_at_unix)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        records
    }

    /// Resolve the [`Attribution`] for an arbitrary changed path —
    /// used by INTD-004 (watcher integration) and INTD-010
    /// (unregistered change handling). The caller passes a path that
    /// may live anywhere on disk (e.g. a child of a registered
    /// worktree); the registry walks the path's ancestor chain
    /// looking for the longest prefix that maps to an active session.
    ///
    /// **Canonicalisation:** the path is canonicalised once before
    /// the ancestor walk so symlinked / dotted spellings do not
    /// silently miss. A path that cannot be canonicalised (missing
    /// file on a `Removed` event, broken link, etc.) is treated as
    /// `Unknown` rather than producing a registry error — the
    /// watcher must still surface unattributed events under
    /// INTD-010's `attribution: unknown-agent` policy.
    ///
    /// **Multiple matches:** if the changed path is nested under
    /// more than one registered worktree (a worktree-inside-a-
    /// worktree configuration that v1 does not actively support but
    /// also does not refuse), the **longest matching prefix** wins.
    /// Returning the deepest match keeps attribution correct when an
    /// operator registers both a parent and a child worktree, and
    /// matches the v1 "single session per canonicalised worktree"
    /// rule the constructor enforces.
    #[must_use]
    pub fn attribute_path(&self, changed: &Path) -> Attribution {
        let canonical = std::fs::canonicalize(changed).ok().or_else(|| {
            // Canonicalisation can fail for `Removed` events
            // (the file no longer exists). Walk up to the first
            // ancestor that does exist, canonicalise that, and
            // re-attach the missing tail. The exact filename
            // does not affect prefix-matching.
            let mut probe = changed.parent();
            while let Some(p) = probe {
                if let Ok(c) = std::fs::canonicalize(p) {
                    return Some(c);
                }
                probe = p.parent();
            }
            None
        });
        let Some(canonical) = canonical else {
            return Attribution::Unknown;
        };

        let inner = self.lock();

        // MLP2-023: walk the composite index to find the longest
        // worktree prefix. With multi-session worktrees the same
        // prefix may yield several `(wt, tag)` keys; the choice of
        // which one to return is **deliberately the untagged session
        // when present**, otherwise the deterministic earliest-by-
        // start-time tagged session. Reason: a path-only attribution
        // call has no tag hint, so falling back to the worktree-level
        // session preserves the pre-MLP2-023 contract; tagged callers
        // wanting per-tag attribution must use the worktree-level
        // result plus their own `agent_tag` knowledge (or
        // [`Self::sessions_for_worktree`] for the full set).
        let mut best_prefix: Option<&PathBuf> = None;
        for (wt, _) in inner.by_composite.keys() {
            if canonical.starts_with(wt)
                && best_prefix
                    .is_none_or(|current| wt.as_os_str().len() > current.as_os_str().len())
            {
                best_prefix = Some(wt);
            }
        }
        let Some(wt) = best_prefix else {
            return Attribution::Unknown;
        };
        let wt = wt.clone();

        // Collect all sessions that share the winning prefix and pick
        // deterministically: untagged first; else earliest-started,
        // tiebreak on SessionId asc.
        let mut candidates: Vec<&RegistryEntry> = inner
            .by_composite
            .iter()
            .filter(|((p, _), _)| p == &wt)
            .filter_map(|(_, id)| inner.sessions.get(id))
            .collect();
        candidates.sort_by(|a, b| {
            // Untagged < tagged so the untagged wins after sort.
            let tag_a = a.record.agent_tag.is_some();
            let tag_b = b.record.agent_tag.is_some();
            tag_a.cmp(&tag_b).then_with(|| {
                a.record
                    .started_at_unix
                    .cmp(&b.record.started_at_unix)
                    .then_with(|| a.record.id.as_str().cmp(b.record.id.as_str()))
            })
        });
        match candidates.first() {
            Some(entry) => Attribution::Owned {
                session: entry.record.clone(),
            },
            // Should not happen: invariants keep `by_composite` and
            // `sessions` aligned. Surfacing as `Unknown` rather than
            // panicking keeps the watcher resilient to a transient
            // inconsistency.
            None => Attribution::Unknown,
        }
    }

    /// All currently active sessions, sorted deterministically by
    /// `started_at_unix` then `SessionId` so tests can compare lists
    /// without flakes under concurrent registration.
    #[must_use]
    pub fn active_sessions(&self) -> Vec<SessionRecord> {
        let inner = self.lock();
        let mut records: Vec<SessionRecord> = inner
            .sessions
            .values()
            .map(|entry| entry.record.clone())
            .collect();
        records.sort_by(|a, b| {
            a.started_at_unix
                .cmp(&b.started_at_unix)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        records
    }

    /// Remove a session by id. Returns `Ok(true)` if a session was
    /// removed, `Ok(false)` if the id was unknown — the latter is not
    /// an error, since the launcher's Drop guard may race the daemon's
    /// eviction tick.
    ///
    /// MLP2-023: only the specific `(worktree, agent_tag)` entry the
    /// session owns is removed from the composite index — sibling
    /// sessions with different tags on the same worktree stay
    /// registered.
    /// MLP2-071 (INTD-015 wire-up): bind a subscriber identity to an
    /// already-registered session. The daemon's IPC accept-loop calls
    /// this on `RegisterSession` once it has minted a `SubscriberId`
    /// from the connecting peer's credentials (`SO_PEERCRED` /
    /// equivalent + `pid_starttime` + binary-path HMAC). The opaque
    /// `binding` is exactly the `SubscriberId`'s post-mint string;
    /// the registry stores it as a `String` to keep the registry
    /// free of a dependency on `fanout::SubscriberId`.
    ///
    /// Returns `true` if the session existed and the binding was
    /// set (or updated); `false` if the session id is unknown.
    /// Re-binding overwrites — a reconnecting subscriber from the
    /// same peer will mint an identical binding, but a session that
    /// changes hands (e.g. reassigned via DRVR-007 capability grant)
    /// can be re-bound.
    pub fn bind_subscriber(&self, id: &SessionId, binding: String) -> bool {
        let mut inner = self.lock();
        inner.sessions.get_mut(id).is_some_and(|entry| {
            entry.subscriber_binding = Some(binding);
            true
        })
    }

    /// MLP2-071 (INTD-015 wire-up): read the subscriber binding for a
    /// session id, if any. The fan-out's
    /// [`crate::fanout::RegistryOwnershipResolver`] calls this to
    /// answer `is_authorised(subscriber, originating_session_id)` —
    /// the subscriber is authorised iff the binding matches the
    /// subscriber's daemon-minted id.
    #[must_use]
    pub fn lookup_subscriber_binding(&self, originating_session_id: &str) -> Option<String> {
        let inner = self.lock();
        let id = SessionId::new(originating_session_id);
        inner
            .sessions
            .get(&id)
            .and_then(|entry| entry.subscriber_binding.clone())
    }

    /// MLP2-071 D6: return the worktree path for an originating
    /// session id, if the session is registered. The production
    /// [`crate::fanout::RegistryOwnershipResolver`] uses this to map a
    /// session id to the worktree whose spoof-fence state it then
    /// consults ([`crate::fence::FenceState::is_spoof_fenced`]).
    /// Returns `None` for unknown session ids — the resolver treats
    /// that as "not degraded" (an unknown session is already
    /// default-denied by the ownership check).
    #[must_use]
    pub fn worktree_for_session_id(&self, originating_session_id: &str) -> Option<PathBuf> {
        let inner = self.lock();
        let id = SessionId::new(originating_session_id);
        inner
            .sessions
            .get(&id)
            .map(|entry| entry.record.worktree.clone())
    }

    pub fn unregister(&self, id: &SessionId) -> Result<bool, RegistryError> {
        let outcome = {
            let mut inner = self.lock();
            Self::remove_session_locked(&mut inner, id)
        };
        let Some(outcome) = outcome else {
            return Ok(false);
        };
        self.fire_unregister_side_effects(outcome);
        Ok(true)
    }

    /// Remove a session and update every derived index, returning the
    /// deferred side-effect targets (`(warm_reclaim, membership_lost)`)
    /// or `None` if the id was absent. Runs entirely on the caller's
    /// already-held guard so the `SessionDispatcher::unregister` path
    /// can fold its peer-ownership check
    /// ([`Self::peer_ownership_check`]) into the SAME lock as the
    /// removal (Copilot PR #3188 TOCTOU fix): the id cannot be evicted
    /// and re-registered between check and mutate.
    fn remove_session_locked(
        inner: &mut Inner,
        id: &SessionId,
    ) -> Option<(Option<PathBuf>, Option<PathBuf>)> {
        let entry = inner.sessions.remove(id)?;
        let worktree = entry.record.worktree.clone();
        let was_durable = entry.durable;
        let key = (worktree.clone(), entry.record.agent_tag.clone());
        inner.by_composite.remove(&key);
        // MLP2-025: drop the lineage anchor too, if any. Linear
        // scan over `by_pid_lineage` because we don't carry the
        // (pid, starttime) on the SessionRecord — the index is the
        // authoritative anchor.
        inner.by_pid_lineage.retain(|_, sid| sid != id);
        // DSV-040: signal warm-state reclamation only when the LAST
        // session for this canonical worktree leaves. A still-live peer
        // (MLP2-023 lets distinct agent tags coexist on one worktree)
        // must keep the shared warm cache + assurance machine — firing
        // per-session would thrash a live sibling's warm state into a
        // cold rebuild. Computed under the lock so the survivor check sees
        // a consistent `by_composite`.
        let warm = if inner.by_composite.keys().any(|(wt, _)| wt == &worktree) {
            None
        } else {
            Some(worktree.clone())
        };
        // ACTMO-014: durable membership is lost only when the removed
        // session was durable AND no durable session for the worktree
        // survives (symmetric with the warm-state last-session rule).
        let membership_lost =
            (was_durable && !inner.is_durable_member(&worktree)).then_some(worktree);
        Some((warm, membership_lost))
    }

    /// Fire the post-removal hooks recorded by
    /// [`Self::remove_session_locked`] once the registry lock has been
    /// released.
    fn fire_unregister_side_effects(&self, outcome: (Option<PathBuf>, Option<PathBuf>)) {
        let (worktree_to_signal, membership_lost) = outcome;
        // MLP2-057: fire the hook AFTER the inner lock is released so a
        // slow consumer (a `SaveTimeState::invalidate` running under
        // its own mutex) does not extend the registry-lock window.
        if let Some(worktree) = worktree_to_signal
            && let Some(hook) = self.unregister_hook.get()
        {
            hook(&worktree);
        }
        // ACTMO-014: membership-change producer (ADR-094 decision 7), also
        // outside the lock.
        if let Some(worktree) = membership_lost {
            self.signal_membership(MembershipChange::Unregistered, &worktree);
        }
    }

    /// ACTMO-014: the distinct worktrees in the durable membership set,
    /// sorted. Backs `anvil status` surfacing (ACTMO-017), the reaper's
    /// "drop + report" log, and the startup reload count.
    #[must_use]
    pub fn registered_worktrees(&self) -> Vec<PathBuf> {
        let inner = self.lock();
        let mut worktrees: Vec<PathBuf> = inner
            .sessions
            .values()
            .filter(|entry| entry.durable)
            .map(|entry| entry.record.worktree.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        drop(inner);
        worktrees.sort();
        worktrees
    }

    /// ACTMO-014: the canonical worktree of a durable session, if `id` names
    /// one. Lets the daemon's IPC dispatcher decide whether an `unregister`
    /// should prune the persisted store (only durable membership is
    /// persisted). Returns `None` for an unknown id or a live agent session.
    #[must_use]
    pub fn durable_worktree_for(&self, id: &SessionId) -> Option<PathBuf> {
        let inner = self.lock();
        inner
            .sessions
            .get(id)
            .filter(|entry| entry.durable)
            .map(|entry| entry.record.worktree.clone())
    }

    /// ACTMO-014: `true` when a durable session still owns `worktree` — used
    /// after an `unregister` to decide whether the worktree left the durable
    /// set entirely (and so should be pruned from the persisted store).
    #[must_use]
    pub fn is_registered(&self, worktree: &Path) -> bool {
        self.lock().is_durable_member(worktree)
    }

    /// ACTMO-014: drop durable registrations whose worktree directory no
    /// longer exists (e.g. `git worktree remove`d) and return the distinct
    /// reaped worktrees, sorted, so the daemon can report them and prune the
    /// persisted store. Live (non-durable) sessions are left to the TTL — the
    /// reaper only governs durable membership.
    ///
    /// `exists` is injected rather than calling the filesystem directly so the
    /// sweep is unit-testable without real directories; it is evaluated at
    /// most once per distinct worktree. Fires the membership `Reaped` signal
    /// (ADR-094 decision 7) and the warm-state unregister hook for
    /// fully-drained worktrees, both outside the registry lock.
    pub fn reap_missing(&self, exists: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
        let (warm_drained, mut reaped) = {
            let mut inner = self.lock();
            let mut gone: HashSet<PathBuf> = HashSet::new();
            let mut present: HashSet<PathBuf> = HashSet::new();
            let to_remove: Vec<SessionId> = inner
                .sessions
                .iter()
                .filter(|(_, entry)| entry.durable)
                .filter_map(|(id, entry)| {
                    let worktree = &entry.record.worktree;
                    let missing = if gone.contains(worktree) {
                        true
                    } else if present.contains(worktree) {
                        false
                    } else if exists(worktree) {
                        present.insert(worktree.clone());
                        false
                    } else {
                        gone.insert(worktree.clone());
                        true
                    };
                    missing.then(|| id.clone())
                })
                .collect();
            for id in &to_remove {
                if let Some(entry) = inner.sessions.remove(id) {
                    let key = (
                        entry.record.worktree.clone(),
                        entry.record.agent_tag.clone(),
                    );
                    inner.by_composite.remove(&key);
                    inner.by_pid_lineage.retain(|_, sid| sid != id);
                }
            }
            // A gone worktree drained of *all* sessions reclaims warm state;
            // drained of *durable* membership emits the `Reaped` signal.
            let mut warm_drained = Vec::new();
            let mut reaped = Vec::new();
            for worktree in &gone {
                if !inner.by_composite.keys().any(|(wt, _)| wt == worktree) {
                    warm_drained.push(worktree.clone());
                }
                if !inner.is_durable_member(worktree) {
                    reaped.push(worktree.clone());
                }
            }
            (warm_drained, reaped)
        };
        if let Some(hook) = self.unregister_hook.get() {
            for worktree in &warm_drained {
                hook(worktree);
            }
        }
        for worktree in &reaped {
            self.signal_membership(MembershipChange::Reaped, worktree);
        }
        reaped.sort();
        reaped
    }

    /// Evict every session whose heartbeat is older than `ttl`.
    /// Returns the ids that were removed so the daemon can fan out
    /// `session.ended` notifications (INTD-013).
    ///
    /// **Boundary policy:** a session at exactly `ttl` is still alive;
    /// at `ttl + 1ns` it evicts. Pinned by the `ttl_boundary` test —
    /// changing this requires updating both the test and the docstring.
    ///
    /// **Tombstone policy:** evicted records are removed from both the
    /// session map and the worktree index. The registry does not keep
    /// `Ended` tombstones — INTD-013 owns the post-eviction
    /// notification, and the wire-level `SessionStatus::Ended` is
    /// reserved for that downstream surface.
    pub fn evict_stale(&self, now: Instant) -> Vec<SessionId> {
        // Two-phase: collect the (id, worktree) pairs under the lock,
        // release the lock, then fan the hook out. Keeps the
        // registry's critical section bounded by hashmap operations,
        // not by however long the cache's invalidate takes.
        let (mut stale, worktrees): (Vec<SessionId>, Vec<PathBuf>) = {
            let mut inner = self.lock();
            let ttl = self.ttl;
            let to_evict: Vec<SessionId> = inner
                .sessions
                .iter()
                .filter_map(|(id, entry)| {
                    // ACTMO-014: durable membership is exempt from the
                    // heartbeat TTL — it is membership, not liveness, and is
                    // removed only by explicit unregister or the reaper.
                    if entry.durable {
                        return None;
                    }
                    let age = now.saturating_duration_since(entry.last_heartbeat);
                    if age > ttl { Some(id.clone()) } else { None }
                })
                .collect();

            let mut evicted_worktrees = Vec::with_capacity(to_evict.len());
            for id in &to_evict {
                if let Some(entry) = inner.sessions.remove(id) {
                    let key = (
                        entry.record.worktree.clone(),
                        entry.record.agent_tag.clone(),
                    );
                    inner.by_composite.remove(&key);
                    evicted_worktrees.push(entry.record.worktree);
                }
            }
            // MLP2-025: drop lineage anchors for every evicted session.
            inner
                .by_pid_lineage
                .retain(|_, sid| !to_evict.contains(sid));
            // DSV-040: signal warm-state reclamation once per DISTINCT evicted
            // worktree that has NO surviving session — same last-session rule
            // as `unregister`. A worktree with two evicted sessions signals
            // once; a worktree with one evicted + one live session does not
            // signal at all (the live peer keeps the warm state).
            let mut to_signal: Vec<PathBuf> = Vec::new();
            for wt in &evicted_worktrees {
                if to_signal.contains(wt) {
                    continue;
                }
                if !inner.by_composite.keys().any(|(w, _)| w == wt) {
                    to_signal.push(wt.clone());
                }
            }
            (to_evict, to_signal)
        };

        // MLP2-057: fire the hook outside the lock, once per fully-drained
        // worktree (computed under the lock above).
        if let Some(hook) = self.unregister_hook.get() {
            for worktree in &worktrees {
                hook(worktree);
            }
        }

        stale.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        stale
    }

    /// Rebuild the `by_composite` index from `sessions`. Used by
    /// [`SessionRegistry::lock`] on poison recovery — `register` /
    /// `unregister` mutate two maps in sequence, so a panic between
    /// the inserts can leave the indices inconsistent. We re-derive
    /// the composite index from the sessions map (which is the sole
    /// source of truth for record bodies) and panic loudly if the
    /// recovery surfaces a true invariant violation (two records
    /// claiming the same `(worktree, agent_tag)` composite).
    fn repair_after_poison(inner: &mut Inner) {
        let mut by_composite = HashMap::with_capacity(inner.sessions.len());
        for (id, entry) in &inner.sessions {
            let key = (
                entry.record.worktree.clone(),
                entry.record.agent_tag.clone(),
            );
            let key_for_msg = key.clone();
            if let Some(existing) = by_composite.insert(key, id.clone()) {
                panic!(
                    "session registry mutex poisoned and recovery surfaced duplicate \
                     (worktree, agent_tag) ownership for ({}, {:?}): {} and {}",
                    key_for_msg.0.display(),
                    key_for_msg.1,
                    existing.as_str(),
                    id.as_str(),
                );
            }
        }
        inner.by_composite = by_composite;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // `std::sync::Mutex` poisons on panic. `register` / `unregister`
        // mutate `sessions` and `by_worktree` in two separate
        // statements, so a panic between them could leave the indices
        // inconsistent. On poison recovery we rebuild `by_worktree`
        // from `sessions` (the sole authority on record bodies),
        // **clear the poison flag**, then hand the guard back. Without
        // `clear_poison` every later `lock()` would keep taking the
        // poison path and repaying the `O(n)` repair cost forever —
        // turning a one-off recovery into permanent per-operation
        // overhead. Carrying poisoning forward (the `std::sync::Mutex`
        // default) would let one panicking caller take the whole
        // daemon offline, which is worse than a single rebuild.
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                Self::repair_after_poison(&mut guard);
                self.inner.clear_poison();
                guard
            }
        }
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionDispatcher for SessionRegistry {
    fn register(
        &self,
        id: &SessionId,
        worktree: &Path,
        agent_tag: Option<&AgentTag>,
        lineage: Option<&LineageAnchor>,
    ) -> Result<(), RegistryError> {
        match lineage {
            Some(anchor) => SessionRegistry::register_with_lineage(
                self,
                id,
                worktree,
                agent_tag,
                // Daemon-issued tag mirror = the client-supplied tag at
                // register time. The launcher's claim about its own
                // tag is trusted at register time (§7 trust model);
                // the spoof cross-check only ever rejects WRITE-time
                // env tags that don't appear on any registered
                // ancestor.
                agent_tag,
                anchor.pid,
                anchor.pid_starttime,
                Instant::now(),
            )
            .map(|_| ()),
            None => {
                SessionRegistry::register(self, id, worktree, agent_tag, Instant::now()).map(|_| ())
            }
        }
    }

    fn heartbeat(&self, id: &SessionId, peer_pid: Option<u32>) -> Result<(), RegistryError> {
        // CIB-153 / Copilot PR #3188: check ownership and apply the
        // heartbeat mutation in ONE locked critical section so the id
        // cannot be evicted and re-registered between them (mirrors
        // `update_lineage_anchor`'s all-checks-before-mutation, single-lock
        // pattern).
        let mut inner = self.lock();
        SessionRegistry::peer_ownership_check(inner.sessions.get(id), id, peer_pid)?;
        SessionRegistry::heartbeat_locked(&mut inner, id, Instant::now())
    }

    fn unregister(&self, id: &SessionId, peer_pid: Option<u32>) -> Result<bool, RegistryError> {
        // CIB-153 / Copilot PR #3188: the ownership check and the
        // removal share ONE lock guard — a stale successful check can no
        // longer authorise a removal on a different entry re-registered
        // under the same id in the gap.
        let outcome = {
            let mut inner = self.lock();
            SessionRegistry::peer_ownership_check(inner.sessions.get(id), id, peer_pid)?;
            SessionRegistry::remove_session_locked(&mut inner, id)
        };
        let Some(outcome) = outcome else {
            return Ok(false);
        };
        self.fire_unregister_side_effects(outcome);
        Ok(true)
    }

    fn list(&self) -> Vec<SessionRecord> {
        SessionRegistry::active_sessions(self)
    }

    fn report_process(
        &self,
        id: &SessionId,
        child_pid: u32,
        child_pid_starttime: u64,
        peer_pid: u32,
    ) -> Result<(), RegistryError> {
        SessionRegistry::update_lineage_anchor(self, id, child_pid, child_pid_starttime, peer_pid)
            .map(|_| ())
    }
}

fn canonicalise(path: &Path) -> Result<PathBuf, RegistryError> {
    std::fs::canonicalize(path).map_err(|source| RegistryError::WorktreePathInvalid {
        path: path.to_path_buf(),
        source,
    })
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Barrier;
    use std::thread;

    use tempfile::TempDir;

    use super::*;

    fn make_worktree() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn sid(name: &str) -> SessionId {
        SessionId::new(name)
    }

    /// MLP2-023 helper: build an `AgentTag` with a fixed `pid_starttime`
    /// so equality checks across the test suite are reproducible.
    fn tag(driver: &str, agent: &str, pid_start: u64) -> AgentTag {
        AgentTag::new(driver, agent, pid_start)
    }

    #[test]
    fn register_list_unregister_round_trip() {
        let registry = SessionRegistry::new();
        let wt_a = make_worktree();
        let wt_b = make_worktree();
        let now = Instant::now();

        registry
            .register(&sid("a"), wt_a.path(), None, now)
            .expect("register a");
        registry
            .register(&sid("b"), wt_b.path(), None, now)
            .expect("register b");

        let listed = registry.active_sessions();
        assert_eq!(listed.len(), 2);
        // Sorted by (started_at_unix, id); both started in the same
        // second under test, so ids decide.
        assert_eq!(listed[0].id, sid("a"));
        assert_eq!(listed[1].id, sid("b"));

        let removed = registry.unregister(&sid("a")).expect("unregister");
        assert!(removed);

        let listed = registry.active_sessions();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, sid("b"));
    }

    #[test]
    fn unregister_unknown_id_is_not_an_error() {
        let registry = SessionRegistry::new();
        let removed = registry.unregister(&sid("ghost")).expect("call");
        assert!(!removed, "unknown id returns false, not an error");
    }

    #[test]
    fn second_session_on_same_worktree_returns_already_owned() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();

        registry
            .register(&sid("first"), wt.path(), None, now)
            .expect("first wins");

        let err = registry
            .register(&sid("second"), wt.path(), None, now)
            .expect_err("second must lose");

        assert_eq!(
            err,
            RegistryError::WorktreeAlreadyOwned {
                existing: sid("first")
            },
        );
    }

    /// A registration that reuses an active session id is rejected even
    /// if the worktree differs. Letting it through would orphan the
    /// previous worktree index entry — the second worktree would be
    /// "owned" by `id` while the first stayed flagged as taken with no
    /// way for `unregister(id)` to release it. v1 makes the caller
    /// `unregister` first.
    #[test]
    fn duplicate_session_id_on_different_worktree_is_rejected() {
        let registry = SessionRegistry::new();
        let wt_a = make_worktree();
        let wt_b = make_worktree();
        let now = Instant::now();

        registry
            .register(&sid("dup"), wt_a.path(), None, now)
            .expect("first wins");

        let err = registry
            .register(&sid("dup"), wt_b.path(), None, now)
            .expect_err("duplicate id must lose");
        assert_eq!(err, RegistryError::SessionAlreadyExists(sid("dup")));

        // Both invariants must hold afterwards: the original worktree
        // is still marked as owned, and the second worktree is not.
        assert!(
            registry.session_for_worktree(wt_a.path()).is_some(),
            "first worktree should remain owned",
        );
        assert!(
            registry.session_for_worktree(wt_b.path()).is_none(),
            "second worktree must NOT be silently registered",
        );
    }

    /// A path with a trailing slash, a `..` segment, or a symlink
    /// indirection canonicalises to the same registry key — so a
    /// second register with a different spelling still detects the
    /// collision.
    #[test]
    fn worktree_path_canonicalisation_collapses_alternative_spellings() {
        let parent = make_worktree();
        let real = parent.path().join("real");
        std::fs::create_dir(&real).expect("real subdir");

        let registry = SessionRegistry::new();
        let now = Instant::now();
        registry
            .register(&sid("first"), &real, None, now)
            .expect("register canonical");

        // `real/.` should resolve to the same entry.
        let dotted = real.join(".");
        let err = registry
            .register(&sid("second"), &dotted, None, now)
            .expect_err("dotted form must collide");
        assert_eq!(
            err,
            RegistryError::WorktreeAlreadyOwned {
                existing: sid("first")
            },
        );

        // `parent/real/../real` should also collide.
        let dotdot = parent.path().join("real").join("..").join("real");
        let err = registry
            .register(&sid("third"), &dotdot, None, now)
            .expect_err("dotdot form must collide");
        assert_eq!(
            err,
            RegistryError::WorktreeAlreadyOwned {
                existing: sid("first")
            },
        );

        // session_for_worktree honours canonicalisation too.
        let found = registry
            .session_for_worktree(&dotted)
            .expect("lookup via dotted");
        assert_eq!(found.id, sid("first"));
    }

    #[test]
    fn worktree_path_missing_on_disk_is_rejected() {
        let registry = SessionRegistry::new();
        let now = Instant::now();
        let missing = std::path::Path::new("/definitely/not/here/anvil-intd-003-ghost");
        let err = registry
            .register(&sid("a"), missing, None, now)
            .expect_err("missing path");
        match err {
            RegistryError::WorktreePathInvalid { path, .. } => {
                assert_eq!(path, missing);
            }
            other => panic!("expected WorktreePathInvalid, got {other:?}"),
        }
    }

    #[test]
    fn heartbeat_unknown_id_returns_unknown_session() {
        let registry = SessionRegistry::new();
        let err = registry
            .heartbeat(&sid("ghost"), Instant::now())
            .expect_err("must fail");
        assert_eq!(err, RegistryError::UnknownSession(sid("ghost")));
    }

    #[test]
    fn heartbeat_refresh_keeps_session_alive_past_ttl_window() {
        let ttl = Duration::from_secs(30);
        let registry = SessionRegistry::with_ttl(ttl);
        let wt = make_worktree();
        let t0 = Instant::now();
        registry
            .register(&sid("a"), wt.path(), None, t0)
            .expect("register");

        let t10 = t0 + Duration::from_secs(10);
        registry
            .heartbeat(&sid("a"), t10)
            .expect("heartbeat at t=10s");

        // t=20s, last heartbeat 10s old, TTL 30s — still alive.
        let t20 = t0 + Duration::from_secs(20);
        assert!(registry.evict_stale(t20).is_empty());
        assert_eq!(registry.active_sessions().len(), 1);

        // t=45s, last heartbeat 35s old — evicted.
        let t45 = t0 + Duration::from_secs(45);
        let evicted = registry.evict_stale(t45);
        assert_eq!(evicted, vec![sid("a")]);
        assert!(registry.active_sessions().is_empty());
    }

    /// CIB-153: a session registered under launcher pid A cannot be
    /// heartbeat-refreshed by a same-UID peer B — the
    /// `SessionDispatcher::heartbeat` ownership check rejects the
    /// mismatch with the typed `PeerOwnershipMismatch`, exactly as
    /// `update_lineage_anchor` does for `report_process`.
    #[test]
    fn dispatcher_heartbeat_rejects_peer_pid_mismatch() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let issued = tag("anvil-run", "launcher", 1_700_002_000);
        let launcher_pid: u32 = 4242;
        registry
            .register_with_lineage(
                &sid("victim"),
                wt.path(),
                None,
                Some(&issued),
                launcher_pid,
                1_700_002_000,
                Instant::now(),
            )
            .expect("register");

        let err = SessionDispatcher::heartbeat(&registry, &sid("victim"), Some(9_999))
            .expect_err("peer pid 9999 != launcher pid 4242");
        assert_eq!(
            err,
            RegistryError::PeerOwnershipMismatch {
                session: sid("victim"),
                expected: Some(launcher_pid),
                actual: 9_999,
            }
        );

        // The owning peer is still accepted.
        SessionDispatcher::heartbeat(&registry, &sid("victim"), Some(launcher_pid))
            .expect("owner heartbeat");
    }

    /// CIB-153: heartbeat fails closed when the call carries no
    /// authenticated peer (`peer_pid = None`, the legacy NDJSON / no
    /// `SO_PEERCRED` path) — mirrors `report_process`'s "requires
    /// authenticated peer credentials" arm.
    #[test]
    fn dispatcher_heartbeat_requires_peer_credentials() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let issued = tag("anvil-run", "launcher", 1_700_002_100);
        let launcher_pid: u32 = 4242;
        registry
            .register_with_lineage(
                &sid("s"),
                wt.path(),
                None,
                Some(&issued),
                launcher_pid,
                1_700_002_100,
                Instant::now(),
            )
            .expect("register");

        let err = SessionDispatcher::heartbeat(&registry, &sid("s"), None)
            .expect_err("no peer credential must fail closed");
        assert_eq!(
            err,
            RegistryError::PeerOwnershipMismatch {
                session: sid("s"),
                expected: Some(launcher_pid),
                actual: 0,
            }
        );
    }

    /// CIB-153: a session registered via the lineage-less `register`
    /// path (`launcher_pid == None`) never established a verified
    /// lineage anchor, so it is **exempt** from the peer-ownership
    /// check — same-uid socket permission remains its authorization
    /// boundary (pre-CIB-153 behaviour). This covers durable
    /// memberships (separate one-shot CLI invocations, each its own
    /// process) and Windows sessions (no `SO_PEERCRED` yet, pending
    /// CIB-114): any authenticated peer, including one whose pid never
    /// touched the session, may heartbeat it.
    #[test]
    fn dispatcher_heartbeat_permits_session_registered_without_lineage() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        registry
            .register(&sid("legacy"), wt.path(), None, Instant::now())
            .expect("legacy register");

        // An arbitrary peer pid — one that never registered this
        // session — is accepted, proving the no-lineage exemption is
        // genuinely permissive rather than accidentally still matching.
        SessionDispatcher::heartbeat(&registry, &sid("legacy"), Some(4242))
            .expect("no-lineage session must be exempt from peer-ownership check");
    }

    /// CIB-153 regression: durable worktree memberships (`anvil
    /// workspace register` / `unregister`) are separate one-shot CLI
    /// process invocations — each is its own OS process with its own
    /// peer pid, and `session_register_params` sends no `lineage`
    /// field — so they register with `launcher_pid == None`. They must
    /// remain heartbeat-refreshable (ADR-094 decision-3 idempotent
    /// re-register-as-heartbeat) and unregisterable by a *later*,
    /// entirely different process. This mirrors the production wire
    /// shape end-to-end: register the durable-membership way (durable
    /// `agent_tag`, no lineage anchor), then drive `heartbeat` and
    /// `unregister` through the `SessionDispatcher` trait with a peer
    /// pid that differs from any prior caller.
    #[test]
    fn dispatcher_permits_durable_membership_lifecycle_from_any_peer() {
        let registry: Arc<dyn SessionDispatcher> = Arc::new(SessionRegistry::new());
        let wt = make_worktree();

        // `anvil workspace register`: durable activation-spine tag, no
        // lineage anchor on the wire — exactly what
        // `register_worktree_with_daemon` sends.
        registry
            .register(&sid("member"), wt.path(), Some(&spine_tag()), None)
            .expect("durable register");

        // A subsequent `anvil workspace register` (idempotent
        // re-register-as-heartbeat) is a brand-new process: peer pid
        // 7777 never touched the original registration.
        registry
            .heartbeat(&sid("member"), Some(7777))
            .expect("durable membership heartbeat must be permitted from any same-uid peer");

        // `anvil workspace unregister`: yet another distinct process.
        assert!(
            registry
                .unregister(&sid("member"), Some(8888))
                .expect("durable membership unregister must be permitted from any same-uid peer"),
        );
        assert!(registry.list().is_empty());
    }

    /// CIB-153: `unregister` carries the same registering-peer binding
    /// as `heartbeat`. Peer B cannot force-unregister peer A's
    /// session; the owning peer still can, and the session is only
    /// removed on the authorised call.
    #[test]
    fn dispatcher_unregister_rejects_peer_pid_mismatch_then_accepts_owner() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let issued = tag("anvil-run", "launcher", 1_700_002_200);
        let launcher_pid: u32 = 4242;
        registry
            .register_with_lineage(
                &sid("victim"),
                wt.path(),
                None,
                Some(&issued),
                launcher_pid,
                1_700_002_200,
                Instant::now(),
            )
            .expect("register");

        let err = SessionDispatcher::unregister(&registry, &sid("victim"), Some(9_999))
            .expect_err("peer pid 9999 != launcher pid 4242");
        assert_eq!(
            err,
            RegistryError::PeerOwnershipMismatch {
                session: sid("victim"),
                expected: Some(launcher_pid),
                actual: 9_999,
            }
        );
        // The rejected removal left the session in place.
        assert_eq!(registry.active_sessions().len(), 1);

        // No peer credential also fails closed.
        let err_none = SessionDispatcher::unregister(&registry, &sid("victim"), None)
            .expect_err("no peer credential must fail closed");
        assert_eq!(
            err_none,
            RegistryError::PeerOwnershipMismatch {
                session: sid("victim"),
                expected: Some(launcher_pid),
                actual: 0,
            }
        );

        // The owning peer removes it.
        let removed = SessionDispatcher::unregister(&registry, &sid("victim"), Some(launcher_pid))
            .expect("owner unregister");
        assert!(removed);
        assert!(registry.active_sessions().is_empty());
    }

    /// CIB-153 regression: the registering launcher keeps ownership of
    /// its session's lifecycle **after** `report_process` narrows the
    /// lineage anchor onto the spawned child. This mirrors the real
    /// anvil-run ordering (register → `report_process` → heartbeats →
    /// unregister, all emitted by the launcher process): binding
    /// against `record.pid` would strand the launcher once the anchor
    /// moved to the child, so the check binds against the stable
    /// `launcher_pid`. The child pid — now `record.pid` — is NOT an
    /// owner and is rejected.
    #[test]
    fn dispatcher_lifecycle_survives_report_process_narrowing() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let issued = tag("anvil-run", "launcher", 1_700_002_300);
        let launcher_pid: u32 = 4242;
        let child_pid: u32 = 6_666;
        registry
            .register_with_lineage(
                &sid("s"),
                wt.path(),
                None,
                Some(&issued),
                launcher_pid,
                1_700_002_300,
                Instant::now(),
            )
            .expect("register");

        // Launcher narrows the anchor onto the spawned child; this
        // rewrites `record.pid` to `child_pid`.
        registry
            .update_lineage_anchor(&sid("s"), child_pid, 1_700_002_400, launcher_pid)
            .expect("narrow to child");

        // The launcher still owns lifecycle: heartbeat + unregister
        // from `launcher_pid` succeed even though `record.pid` is now
        // the child.
        SessionDispatcher::heartbeat(&registry, &sid("s"), Some(launcher_pid))
            .expect("launcher heartbeat after narrowing");

        // The child pid (now `record.pid`) is not the lifecycle owner.
        let err = SessionDispatcher::heartbeat(&registry, &sid("s"), Some(child_pid))
            .expect_err("child pid is not the lifecycle owner");
        assert_eq!(
            err,
            RegistryError::PeerOwnershipMismatch {
                session: sid("s"),
                expected: Some(launcher_pid),
                actual: child_pid,
            }
        );

        assert!(
            SessionDispatcher::unregister(&registry, &sid("s"), Some(launcher_pid))
                .expect("launcher unregister after narrowing")
        );
        assert!(registry.active_sessions().is_empty());
    }

    /// CIB-153: unregistering an unknown/already-evicted id stays an
    /// idempotent `Ok(false)` no-op regardless of `peer_pid` — there
    /// is no owned state to protect, so the ownership check defers to
    /// the operation's established semantics.
    #[test]
    fn dispatcher_unregister_unknown_id_is_idempotent_noop() {
        let registry = SessionRegistry::new();
        let removed = SessionDispatcher::unregister(&registry, &sid("ghost"), Some(4242))
            .expect("unknown id is not an error");
        assert!(!removed);
        // Also with no peer credential.
        let removed_none = SessionDispatcher::unregister(&registry, &sid("ghost"), None)
            .expect("unknown id is not an error");
        assert!(!removed_none);
    }

    /// Pin the boundary: a session at exactly TTL is alive; at TTL +
    /// 1 ns it evicts. The docstring on `evict_stale` declares this
    /// policy; changing it should require updating both.
    #[test]
    fn ttl_boundary_is_inclusive_at_ttl_exclusive_just_after() {
        let ttl = Duration::from_secs(30);
        let registry = SessionRegistry::with_ttl(ttl);
        let wt = make_worktree();
        let t0 = Instant::now();
        registry
            .register(&sid("a"), wt.path(), None, t0)
            .expect("register");

        let exactly = t0 + ttl;
        assert!(
            registry.evict_stale(exactly).is_empty(),
            "session at exactly TTL must still be alive",
        );

        let just_after = t0 + ttl + Duration::from_nanos(1);
        let evicted = registry.evict_stale(just_after);
        assert_eq!(evicted, vec![sid("a")], "session past TTL must evict");
    }

    // ---- ACTMO-014: durable worktree membership -------------------------

    /// An activation-spine tag marks durable membership.
    fn spine_tag() -> AgentTag {
        AgentTag::new(
            "anvil-start",
            anvil_intercept_proto::session::ACTIVATION_SPINE_CLAIMED_AGENT_ID,
            0,
        )
    }

    /// ACTMO-014 D4: a durable (activation-spine) registration is exempt from
    /// the heartbeat TTL — it survives arbitrarily far past the window with no
    /// heartbeat, because it is membership, not liveness.
    #[test]
    fn durable_membership_is_exempt_from_ttl_eviction() {
        let registry = SessionRegistry::with_ttl(Duration::from_secs(30));
        let durable_wt = make_worktree();
        let live_wt = make_worktree();
        let t0 = Instant::now();

        registry
            .register(&sid("durable"), durable_wt.path(), Some(&spine_tag()), t0)
            .expect("register durable");
        registry
            .register(&sid("live"), live_wt.path(), None, t0)
            .expect("register live");

        // Far past the TTL: the live lease evicts, the durable member stays.
        let way_past = t0 + Duration::from_mins(10);
        let evicted = registry.evict_stale(way_past);
        assert_eq!(evicted, vec![sid("live")]);
        assert_eq!(registry.registered_worktrees().len(), 1);
        assert!(
            registry
                .active_sessions()
                .iter()
                .any(|s| s.id == sid("durable")),
            "durable membership must survive the TTL sweep",
        );
    }

    /// ACTMO-014 D4: the distinct-registered-worktree cap refuses a NEW
    /// durable worktree past the ceiling, but never refuses a refresh of an
    /// existing member or a live (non-durable) session.
    #[test]
    fn distinct_registered_worktree_cap_refuses_new_durable_worktree() {
        let registry = SessionRegistry::new().with_registered_worktree_cap(2);
        let wts: Vec<TempDir> = (0..3).map(|_| make_worktree()).collect();
        let now = Instant::now();

        registry
            .register(&sid("d0"), wts[0].path(), Some(&spine_tag()), now)
            .expect("first durable");
        registry
            .register(&sid("d1"), wts[1].path(), Some(&spine_tag()), now)
            .expect("second durable");

        let err = registry
            .register(&sid("d2"), wts[2].path(), Some(&spine_tag()), now)
            .expect_err("third distinct durable worktree must exceed the cap");
        assert_eq!(
            err,
            RegistryError::RegisteredWorktreeCapExceeded { cap: 2, live: 2 },
        );

        // A live session on a brand-new worktree is unaffected by the
        // durable-membership cap.
        registry
            .register(&sid("live"), wts[2].path(), None, now)
            .expect("live session is not bound by the durable cap");
    }

    /// ACTMO-014: `registered_worktrees` reports durable members only, never
    /// the live agent-session leases that share the registry.
    #[test]
    fn registered_worktrees_lists_durable_members_only() {
        let registry = SessionRegistry::new();
        let durable_wt = make_worktree();
        let live_wt = make_worktree();
        let now = Instant::now();

        registry
            .register(&sid("durable"), durable_wt.path(), Some(&spine_tag()), now)
            .expect("durable");
        registry
            .register(&sid("live"), live_wt.path(), None, now)
            .expect("live");

        let listed = registry.registered_worktrees();
        let canonical = std::fs::canonicalize(durable_wt.path()).expect("canonicalise");
        assert_eq!(listed, vec![canonical]);
    }

    /// ACTMO-014 D4 (reaper): a durable registration whose directory is gone
    /// is dropped and reported; a present one is retained. The probe is
    /// injected so the sweep is deterministic without touching the FS.
    #[test]
    fn reaper_drops_durable_registration_whose_dir_is_gone() {
        let registry = SessionRegistry::new();
        let present = make_worktree();
        let gone = make_worktree();
        let now = Instant::now();
        registry
            .register(&sid("present"), present.path(), Some(&spine_tag()), now)
            .expect("present");
        registry
            .register(&sid("gone"), gone.path(), Some(&spine_tag()), now)
            .expect("gone");

        let gone_canonical = std::fs::canonicalize(gone.path()).expect("canonicalise gone");
        let reaped = registry.reap_missing(|wt| wt != gone_canonical);

        assert_eq!(reaped, vec![gone_canonical]);
        assert_eq!(registry.registered_worktrees().len(), 1);
        assert!(
            registry
                .active_sessions()
                .iter()
                .all(|s| s.id != sid("gone")),
            "the reaped session must be gone from the registry",
        );
    }

    /// ACTMO-014 (ADR-094 decision 7): the membership hook is the sole
    /// producer of register / unregister / reaper transitions for DSV-046.
    #[test]
    fn membership_hook_fires_on_register_unregister_and_reap() {
        let registry = SessionRegistry::new();
        let events: Arc<Mutex<Vec<MembershipChange>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        assert!(registry.set_membership_hook(Arc::new(move |change, _wt| {
            sink.lock().unwrap().push(change);
        })));

        let kept = make_worktree();
        let dropped = make_worktree();
        let now = Instant::now();

        registry
            .register(&sid("kept"), kept.path(), Some(&spine_tag()), now)
            .expect("kept");
        registry
            .register(&sid("dropped"), dropped.path(), Some(&spine_tag()), now)
            .expect("dropped");
        registry.unregister(&sid("kept")).expect("unregister kept");
        let dropped_canonical =
            std::fs::canonicalize(dropped.path()).expect("canonicalise dropped");
        registry.reap_missing(|wt| wt != dropped_canonical);

        let seen = events.lock().unwrap().clone();
        assert_eq!(
            seen,
            vec![
                MembershipChange::Registered,
                MembershipChange::Registered,
                MembershipChange::Unregistered,
                MembershipChange::Reaped,
            ],
        );
    }

    /// `pid` / `pgid` / `started_at_unix` update; supplying `None` does
    /// NOT clobber an existing `Some`.
    #[test]
    fn process_info_update_is_partial_and_idempotent() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();
        registry
            .register(&sid("a"), wt.path(), None, now)
            .expect("register");

        let updated = registry
            .update_process_info(
                &sid("a"),
                ProcessInfo {
                    pid: Some(1234),
                    pgid: Some(1234),
                    started_at_unix: Some(1_700_000_000),
                },
            )
            .expect("first update");
        assert_eq!(updated.pid, Some(1234));
        assert_eq!(updated.pgid, Some(1234));
        assert_eq!(updated.started_at_unix, 1_700_000_000);

        // None fields are no-ops — Some values from the prior update
        // survive.
        let again = registry
            .update_process_info(&sid("a"), ProcessInfo::default())
            .expect("noop update");
        assert_eq!(again.pid, Some(1234));
        assert_eq!(again.pgid, Some(1234));
        assert_eq!(again.started_at_unix, 1_700_000_000);

        // Re-applying the same update is idempotent.
        let same = registry
            .update_process_info(
                &sid("a"),
                ProcessInfo {
                    pid: Some(1234),
                    pgid: Some(1234),
                    started_at_unix: Some(1_700_000_000),
                },
            )
            .expect("idempotent update");
        assert_eq!(same, updated);
    }

    #[test]
    fn update_process_info_unknown_session_is_unknown_error() {
        let registry = SessionRegistry::new();
        let err = registry
            .update_process_info(&sid("ghost"), ProcessInfo::default())
            .expect_err("must fail");
        assert_eq!(err, RegistryError::UnknownSession(sid("ghost")));
    }

    /// Two threads racing on `register` for the same canonicalised
    /// worktree — exactly one wins; the other gets
    /// `WorktreeAlreadyOwned`. The registry must be `Send + Sync`.
    #[test]
    fn concurrent_register_on_same_worktree_picks_exactly_one_winner() {
        let registry = Arc::new(SessionRegistry::new());
        let wt = Arc::new(make_worktree());
        let barrier = Arc::new(Barrier::new(2));

        let r1 = Arc::clone(&registry);
        let w1 = Arc::clone(&wt);
        let b1 = Arc::clone(&barrier);
        let h1 = thread::spawn(move || {
            b1.wait();
            r1.register(&sid("a"), w1.path(), None, Instant::now())
        });

        let r2 = Arc::clone(&registry);
        let w2 = Arc::clone(&wt);
        let b2 = Arc::clone(&barrier);
        let h2 = thread::spawn(move || {
            b2.wait();
            r2.register(&sid("b"), w2.path(), None, Instant::now())
        });

        let result_1 = h1.join().expect("thread 1");
        let result_2 = h2.join().expect("thread 2");

        let oks = [&result_1, &result_2].iter().filter(|r| r.is_ok()).count();
        let errs = [&result_1, &result_2].iter().filter(|r| r.is_err()).count();
        assert_eq!(oks, 1, "exactly one register must succeed");
        assert_eq!(errs, 1, "exactly one register must fail");

        let listed = registry.active_sessions();
        assert_eq!(listed.len(), 1, "registry holds exactly one entry");
    }

    /// Active-list ordering is deterministic under concurrent
    /// registration: list it twice between identical state and the
    /// orderings match.
    #[test]
    fn active_sessions_ordering_is_deterministic_under_concurrency() {
        let registry = Arc::new(SessionRegistry::new());
        let dirs: Vec<TempDir> = (0..8).map(|_| make_worktree()).collect();
        let dirs = Arc::new(dirs);

        let mut handles = Vec::new();
        for i in 0..8 {
            let r = Arc::clone(&registry);
            let d = Arc::clone(&dirs);
            handles.push(thread::spawn(move || {
                r.register(&sid(&format!("s{i}")), d[i].path(), None, Instant::now())
                    .expect("register");
            }));
        }
        for h in handles {
            h.join().expect("join");
        }

        let first = registry.active_sessions();
        let second = registry.active_sessions();
        assert_eq!(
            first, second,
            "list ordering must be deterministic between calls",
        );

        // Sorted by (started_at_unix, id). Two threads racing on
        // `register` near a Unix-second boundary can land in different
        // seconds, so compute the expected order with the same key the
        // registry uses rather than assuming the id-only ordering.
        let mut expected = first.clone();
        expected.sort_by(|a, b| {
            a.started_at_unix
                .cmp(&b.started_at_unix)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        let expected_ids: Vec<String> =
            expected.iter().map(|r| r.id.as_str().to_string()).collect();
        let actual: Vec<String> = first.iter().map(|r| r.id.as_str().to_string()).collect();
        assert_eq!(actual, expected_ids);
    }

    /// Crash-safe Drop is NOT this layer's job. A launcher whose
    /// process was SIGKILL-ed never calls `unregister`; we rely on
    /// `evict_stale` to reap. This test is a doc test for that
    /// contract — without it, a future refactor that adds a Drop
    /// guard could silently change the behaviour.
    #[test]
    fn crashed_launcher_recovery_relies_on_evict_stale_not_drop() {
        let ttl = Duration::from_millis(1);
        let registry = SessionRegistry::with_ttl(ttl);
        let wt = make_worktree();
        let t0 = Instant::now();
        registry
            .register(&sid("crashed"), wt.path(), None, t0)
            .expect("register");

        // Simulate "process gone" by simply not calling unregister and
        // letting the heartbeat go stale.
        let later = t0 + Duration::from_millis(5);
        let evicted = registry.evict_stale(later);
        assert_eq!(evicted, vec![sid("crashed")]);

        // After eviction, the worktree is free for a fresh registration.
        registry
            .register(&sid("recovered"), wt.path(), None, later)
            .expect("worktree freed by eviction");
    }

    /// Poisoning the mutex (panic while a guard is held) does not
    /// take the registry offline — `lock()` recovers and rebuilds the
    /// `by_worktree` index from `sessions`. The post-recovery view is
    /// coherent: prior sessions remain queryable, and a fresh
    /// `register` succeeds on a free worktree.
    #[test]
    fn poisoned_mutex_recovery_yields_coherent_state() {
        use std::panic::{AssertUnwindSafe, catch_unwind};

        let registry = Arc::new(SessionRegistry::new());
        let wt_a = make_worktree();
        let wt_b = make_worktree();
        registry
            .register(&sid("alive"), wt_a.path(), None, Instant::now())
            .expect("register");

        // Poison the mutex by panicking while a guard is held.
        let registry_clone = Arc::clone(&registry);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry_clone.inner.lock().expect("first lock cannot fail");
            panic!("poison");
        }));
        assert!(
            registry.inner.is_poisoned(),
            "test setup expected the mutex to be poisoned",
        );

        // Subsequent operations recover the guard and keep working.
        assert!(
            registry.session_for_worktree(wt_a.path()).is_some(),
            "previously-registered session must survive poison recovery",
        );
        // After the first recovery, the poison flag is cleared so
        // later lock attempts don't keep paying the repair cost.
        assert!(
            !registry.inner.is_poisoned(),
            "poison flag must be cleared after the first recovery",
        );
        registry
            .register(&sid("after-poison"), wt_b.path(), None, Instant::now())
            .expect("registry remains usable after poison");
        assert_eq!(registry.active_sessions().len(), 2);
    }

    /// `SessionDispatcher` trait dispatch works against the concrete
    /// registry — this is the surface INTD-002 calls into via
    /// `Arc<dyn SessionDispatcher>`. CIB-153: the session is
    /// registered with a lineage anchor so the launcher pid is stamped,
    /// and the lifecycle calls carry the owning peer's pid.
    #[test]
    fn session_dispatcher_trait_dispatches_to_registry() {
        use anvil_intercept_proto::session::LineageAnchor;
        let registry: Arc<dyn SessionDispatcher> = Arc::new(SessionRegistry::new());
        let wt = make_worktree();
        let launcher_pid: u32 = 4242;

        registry
            .register(
                &sid("a"),
                wt.path(),
                None,
                Some(&LineageAnchor {
                    pid: launcher_pid,
                    pid_starttime: 1_700_003_100,
                }),
            )
            .expect("register");
        assert_eq!(registry.list().len(), 1);

        registry
            .heartbeat(&sid("a"), Some(launcher_pid))
            .expect("heartbeat");
        assert!(
            registry
                .unregister(&sid("a"), Some(launcher_pid))
                .expect("unregister")
        );
        assert!(registry.list().is_empty());
    }

    // ───────────────────────────────────────────────────────────
    // MLP2-023: composite (WorktreeKey, Option<AgentTag>) keying.
    // ───────────────────────────────────────────────────────────

    /// Two tagged sessions with **distinct** `AgentTag`s register
    /// against the same worktree without conflict — the registry's
    /// uniqueness invariant is per-composite, not per-worktree.
    #[test]
    fn two_distinct_tags_on_same_worktree_coexist() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();

        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);
        let tag_b = tag("anvil-run", "claude-2", 1_700_000_002);

        registry
            .register(&sid("sa"), wt.path(), Some(&tag_a), now)
            .expect("register sa");
        registry
            .register(&sid("sb"), wt.path(), Some(&tag_b), now)
            .expect("register sb");

        let live = registry.sessions_for_worktree(wt.path());
        assert_eq!(live.len(), 2, "both tagged sessions live on the worktree");
        assert!(live.iter().any(|r| r.agent_tag.as_ref() == Some(&tag_a)));
        assert!(live.iter().any(|r| r.agent_tag.as_ref() == Some(&tag_b)));
    }

    /// MLP2-023: re-registering the **same** tag on the same worktree
    /// still surfaces `WorktreeAlreadyOwned` — the composite key is
    /// what's unique, not the worktree alone.
    #[test]
    fn same_tag_on_same_worktree_returns_already_owned() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);

        registry
            .register(&sid("first"), wt.path(), Some(&tag_a), now)
            .expect("first registration");
        let err = registry
            .register(&sid("second"), wt.path(), Some(&tag_a), now)
            .expect_err("duplicate composite must be rejected");
        match err {
            RegistryError::WorktreeAlreadyOwned { existing } => {
                assert_eq!(existing, sid("first"));
            }
            other => panic!("expected WorktreeAlreadyOwned, got {other:?}"),
        }
    }

    /// ADR-090 (CIB-098): `sessions_for_canonical_worktree` matches the
    /// stored canonical key directly. It returns the same set as
    /// `sessions_for_worktree` for a live worktree, but does NOT depend on
    /// the path being stattable at lookup time.
    #[test]
    fn sessions_for_canonical_worktree_matches_stored_key() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();
        registry
            .register(&sid("s1"), wt.path(), None, now)
            .expect("register s1");

        // The canonical key the registration stored is what we must look up.
        let canonical = canonicalise(wt.path()).expect("canonicalise live worktree");
        let live = registry.sessions_for_canonical_worktree(&canonical);
        assert_eq!(
            live.len(),
            1,
            "the registered session is found by canonical key"
        );
        assert_eq!(live[0].id, sid("s1"));

        // A different worktree default-misses.
        let other = make_worktree();
        let other_canonical = canonicalise(other.path()).expect("canonicalise other");
        assert!(
            registry
                .sessions_for_canonical_worktree(&other_canonical)
                .is_empty(),
            "an unregistered canonical worktree yields no sessions"
        );
    }

    /// ADR-090 (CIB-098), Finding 1: a session registered against a worktree
    /// that is later removed from disk is STILL resolvable via
    /// `sessions_for_canonical_worktree` — the lookup performs no
    /// `fs::canonicalize`, so it does not silently return empty in the
    /// degraded states (deleted/unmounted/EROFS) the daemon-health
    /// notification is built for. The pre-fix `sessions_for_worktree` would
    /// return empty here because its `fs::canonicalize` fails on a missing
    /// path.
    #[test]
    fn sessions_for_canonical_worktree_survives_deleted_dir() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();
        // Capture the canonical key BEFORE the dir is removed.
        let canonical = canonicalise(wt.path()).expect("canonicalise live worktree");
        registry
            .register(&sid("orphan"), wt.path(), None, now)
            .expect("register orphan");

        // Make the worktree path un-canonicalizable by removing it on disk.
        let path = wt.path().to_path_buf();
        drop(wt);
        assert!(
            !path.exists(),
            "worktree dir removed for the degraded scenario"
        );

        // The fs-bound lookup now silently misses (this is the bug Finding 1
        // describes).
        assert!(
            registry.sessions_for_worktree(&path).is_empty(),
            "sessions_for_worktree canonicalizes and so loses the session once the dir is gone"
        );

        // The no-fs lookup against the already-canonical key still resolves
        // the session — the subscriber keeps its notification.
        let live = registry.sessions_for_canonical_worktree(&canonical);
        assert_eq!(
            live.len(),
            1,
            "a still-registered session on a now-unstattable worktree is found by canonical key"
        );
        assert_eq!(live[0].id, sid("orphan"));
    }

    /// MLP2-023: an untagged session and a tagged session on the same
    /// worktree coexist. The untagged registration represents the
    /// worktree-level enforcement context (matching the pre-MLP2-023
    /// path); the tagged registration represents a specific sub-agent.
    #[test]
    fn untagged_and_tagged_on_same_worktree_coexist() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);

        registry
            .register(&sid("worktree-level"), wt.path(), None, now)
            .expect("untagged register");
        registry
            .register(&sid("sub-agent"), wt.path(), Some(&tag_a), now)
            .expect("tagged register");

        let live = registry.sessions_for_worktree(wt.path());
        assert_eq!(live.len(), 2);
        let untagged = live.iter().find(|r| r.agent_tag.is_none()).unwrap();
        let tagged = live.iter().find(|r| r.agent_tag.is_some()).unwrap();
        assert_eq!(untagged.id, sid("worktree-level"));
        assert_eq!(tagged.id, sid("sub-agent"));
    }

    /// MLP2-023: a second **untagged** session on the same worktree
    /// still fails — there can be at most one worktree-level entry.
    /// This preserves the pre-MLP2-023 semantics for callers that
    /// haven't opted into tags.
    #[test]
    fn second_untagged_session_on_same_worktree_returns_already_owned() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();

        registry
            .register(&sid("first"), wt.path(), None, now)
            .expect("first untagged");
        let err = registry
            .register(&sid("second"), wt.path(), None, now)
            .expect_err("second untagged must be rejected");
        assert!(matches!(err, RegistryError::WorktreeAlreadyOwned { .. }));
    }

    /// MLP2-023: `attribute_path` prefers the **untagged** session on
    /// a multi-session worktree (the worktree-level enforcement
    /// context), then deterministically picks the earliest-started
    /// tagged session if no untagged session exists.
    #[test]
    fn attribute_path_prefers_untagged_session_then_earliest_tag() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();

        // Register a tagged session first, then an untagged one. The
        // untagged should win attribution regardless of insertion order.
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);
        let t0 = Instant::now();
        registry
            .register(&sid("tagged"), wt.path(), Some(&tag_a), t0)
            .expect("tagged");
        registry
            .register(
                &sid("untagged"),
                wt.path(),
                None,
                t0 + Duration::from_millis(5),
            )
            .expect("untagged");

        let child = wt.path().join("src.rs");
        std::fs::write(&child, b"x").unwrap();
        match registry.attribute_path(&child) {
            Attribution::Owned { session } => assert_eq!(session.id, sid("untagged")),
            Attribution::Unknown => panic!("attribute_path returned Unknown"),
        }
    }

    /// MLP2-023: with only tagged sessions present, `attribute_path`
    /// returns a deterministic answer for a hint-less caller. Sort
    /// order is `started_at_unix` ascending then `SessionId`
    /// ascending. Two sessions registered within the same Unix second
    /// therefore fall back to lexicographic `SessionId` order — the
    /// test pins both behaviours by registering two sessions in the
    /// same tick and asserting the lexicographically-smaller id wins.
    /// Per-tag attribution is the caller's responsibility once
    /// MLP2-026 wires the fence-key path.
    #[test]
    fn attribute_path_deterministic_tiebreak_across_tagged_only() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();

        let tag_a = tag("anvil-run", "claude-a", 1_700_000_001);
        let tag_b = tag("anvil-run", "claude-b", 1_700_000_002);
        // Register B first then A within the same tick — both get the
        // same `started_at_unix`, so the `SessionId` tiebreak applies.
        registry
            .register(&sid("session-b"), wt.path(), Some(&tag_b), now)
            .expect("tag b");
        registry
            .register(&sid("session-a"), wt.path(), Some(&tag_a), now)
            .expect("tag a");

        let child = wt.path().join("src.rs");
        std::fs::write(&child, b"x").unwrap();
        // Lexicographic tiebreak: session-a < session-b → wins.
        match registry.attribute_path(&child) {
            Attribution::Owned { session } => {
                assert_eq!(
                    session.id,
                    sid("session-a"),
                    "SessionId tiebreak picks the lexicographically-smaller id when start times match"
                );
            }
            Attribution::Unknown => panic!("attribute_path returned Unknown"),
        }
    }

    /// MLP2-023: unregistering one tagged session leaves siblings on
    /// the same worktree intact. This is the precondition for
    /// MLP2-026's per-task fence isolation — one bad sub-agent
    /// finishing (or being fenced individually) must not affect its
    /// peers' registrations.
    #[test]
    fn unregister_one_tagged_session_leaves_sibling_alive() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);
        let tag_b = tag("anvil-run", "claude-2", 1_700_000_002);

        registry
            .register(&sid("a"), wt.path(), Some(&tag_a), now)
            .unwrap();
        registry
            .register(&sid("b"), wt.path(), Some(&tag_b), now)
            .unwrap();
        assert!(registry.unregister(&sid("a")).unwrap());

        let live = registry.sessions_for_worktree(wt.path());
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].id, sid("b"));
        assert_eq!(live[0].agent_tag.as_ref(), Some(&tag_b));
    }

    /// MLP2-023: `evict_stale` removes only the specific session whose
    /// heartbeat expired. The sibling tagged session on the same
    /// worktree keeps its registration. Uses a 25 ms TTL with a 30 ms
    /// gap between the stale and alive registrations so `evict_stale`
    /// at `t0 + 30ms` finds the stale session 30 ms old (> 25 ms TTL)
    /// while the alive session is 0 ms old (under TTL).
    #[test]
    fn evict_stale_removes_only_the_expired_tagged_session() {
        let registry = SessionRegistry::with_ttl(Duration::from_millis(25));
        let wt = make_worktree();
        let t0 = Instant::now();
        let tag_a = tag("anvil-run", "claude-stale", 1);
        let tag_b = tag("anvil-run", "claude-alive", 2);

        registry
            .register(&sid("stale"), wt.path(), Some(&tag_a), t0)
            .unwrap();
        let alive_at = t0 + Duration::from_millis(30);
        registry
            .register(&sid("alive"), wt.path(), Some(&tag_b), alive_at)
            .unwrap();

        let evicted = registry.evict_stale(alive_at);
        assert_eq!(evicted, vec![sid("stale")]);

        let live = registry.sessions_for_worktree(wt.path());
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].id, sid("alive"));
    }

    /// MLP2-023: the `agent_tag` field round-trips through registry
    /// state — `register` → `active_sessions` → wire-shape
    /// `SessionRecord.agent_tag` matches what the caller supplied.
    #[test]
    fn agent_tag_round_trips_through_active_sessions() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let now = Instant::now();
        let tag_a = tag("anvil-run", "claude-tag", 1_700_000_042);

        registry
            .register(&sid("tagged"), wt.path(), Some(&tag_a), now)
            .unwrap();
        let records = registry.active_sessions();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].agent_tag.as_ref(), Some(&tag_a));
    }

    // ───────────────────────────────────────────────────────────
    // MLP2-024: per-worktree session cap.
    // ───────────────────────────────────────────────────────────

    /// MLP2-024: with the cap at 2, a third registration on the same
    /// canonical worktree is refused with `SessionCapExceeded`. Cap
    /// is counted across all `agent_tag` values, so two tagged
    /// sub-agents + one attempted untagged session would also trip
    /// the limit.
    #[test]
    fn third_session_on_capped_worktree_is_refused() {
        let registry = SessionRegistry::new().with_per_worktree_cap(2);
        let wt = make_worktree();
        let now = Instant::now();
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);
        let tag_b = tag("anvil-run", "claude-2", 1_700_000_002);
        let tag_c = tag("anvil-run", "claude-3", 1_700_000_003);

        registry
            .register(&sid("a"), wt.path(), Some(&tag_a), now)
            .unwrap();
        registry
            .register(&sid("b"), wt.path(), Some(&tag_b), now)
            .unwrap();
        let err = registry
            .register(&sid("c"), wt.path(), Some(&tag_c), now)
            .expect_err("third registration must be refused");

        let canonical = std::fs::canonicalize(wt.path()).unwrap();
        match err {
            RegistryError::SessionCapExceeded {
                worktree,
                cap,
                live,
            } => {
                assert_eq!(worktree, canonical);
                assert_eq!(cap, 2);
                assert_eq!(live, 2);
            }
            other => panic!("expected SessionCapExceeded, got {other:?}"),
        }
    }

    /// ACTMO-014 (adversarial review F4): durable activation-spine membership
    /// is exempt from the per-worktree **live-session** cap. A registered
    /// worktree keeps the full `cap` live-agent slots, and a durable
    /// registration is itself never refused by this cap (it is bounded by the
    /// distinct-registered-worktree cap instead).
    #[test]
    fn durable_membership_is_exempt_from_per_worktree_cap() {
        let registry = SessionRegistry::new().with_per_worktree_cap(2);
        let wt = make_worktree();
        let now = Instant::now();
        let canonical = std::fs::canonicalize(wt.path()).unwrap();

        // Durable membership does not consume a live-session slot.
        registry
            .register(&sid("durable"), wt.path(), Some(&spine_tag()), now)
            .expect("durable membership registers");

        // The full cap of live agent sessions remains available alongside it.
        registry
            .register(&sid("live1"), wt.path(), None, now)
            .expect("first live session");
        let tag_b = tag("anvil-run", "claude-2", 1_700_000_002);
        registry
            .register(&sid("live2"), wt.path(), Some(&tag_b), now)
            .expect("second live session fills the cap");

        // The (cap+1)-th LIVE session is refused — the durable one is not
        // counted, so `live` is 2, not 3.
        let tag_c = tag("anvil-run", "claude-3", 1_700_000_003);
        let err = registry
            .register(&sid("live3"), wt.path(), Some(&tag_c), now)
            .expect_err("third live session must be refused");
        assert_eq!(
            err,
            RegistryError::SessionCapExceeded {
                worktree: canonical,
                cap: 2,
                live: 2,
            },
        );

        // A durable registration is never refused by the per-worktree cap,
        // even on a worktree already saturated with live sessions.
        let wt2 = make_worktree();
        registry
            .register(&sid("p"), wt2.path(), None, now)
            .expect("live p");
        registry
            .register(&sid("q"), wt2.path(), Some(&tag_b), now)
            .expect("live q saturates the cap");
        registry
            .register(&sid("durable2"), wt2.path(), Some(&spine_tag()), now)
            .expect("durable membership is exempt even at the live-session cap");
    }

    /// MLP2-024: unregistering a session frees a slot — the next
    /// registration succeeds at the same cap.
    #[test]
    fn cap_freed_by_unregister_admits_next_registration() {
        let registry = SessionRegistry::new().with_per_worktree_cap(2);
        let wt = make_worktree();
        let now = Instant::now();
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);
        let tag_b = tag("anvil-run", "claude-2", 1_700_000_002);
        let tag_c = tag("anvil-run", "claude-3", 1_700_000_003);

        registry
            .register(&sid("a"), wt.path(), Some(&tag_a), now)
            .unwrap();
        registry
            .register(&sid("b"), wt.path(), Some(&tag_b), now)
            .unwrap();
        assert!(registry.unregister(&sid("a")).unwrap());
        registry
            .register(&sid("c"), wt.path(), Some(&tag_c), now)
            .expect("new slot opens after unregister");
        assert_eq!(registry.sessions_for_worktree(wt.path()).len(), 2);
    }

    /// MLP2-024: cap is scoped per worktree — registering against a
    /// second worktree does NOT trip the first worktree's cap.
    #[test]
    fn cap_is_scoped_per_worktree() {
        let registry = SessionRegistry::new().with_per_worktree_cap(1);
        let wt_a = make_worktree();
        let wt_b = make_worktree();
        let now = Instant::now();

        registry
            .register(&sid("a"), wt_a.path(), None, now)
            .unwrap();
        registry
            .register(&sid("b"), wt_b.path(), None, now)
            .expect("different worktree must not trip the cap");
    }

    /// MLP2-024: `with_per_worktree_cap(0)` clamps to 1 — a zero
    /// cap would refuse every registration, which is never the
    /// operator's intent. Uses distinct tags so the cap path
    /// fires before the composite-key duplicate path.
    #[test]
    fn zero_cap_is_clamped_to_one() {
        let registry = SessionRegistry::new().with_per_worktree_cap(0);
        let wt = make_worktree();
        let now = Instant::now();
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);
        let tag_b = tag("anvil-run", "claude-2", 1_700_000_002);
        // First registration succeeds because the clamp lifted the cap to 1.
        registry
            .register(&sid("a"), wt.path(), Some(&tag_a), now)
            .unwrap();
        // Second is refused — different tag, so it goes past the
        // composite-key check straight to the cap check.
        let err = registry
            .register(&sid("b"), wt.path(), Some(&tag_b), now)
            .expect_err("second registration above clamp must be refused");
        match err {
            RegistryError::SessionCapExceeded { cap, live, .. } => {
                assert_eq!(cap, 1, "zero must be clamped to 1");
                assert_eq!(live, 1);
            }
            other => panic!("expected SessionCapExceeded, got {other:?}"),
        }
    }

    /// MLP2-024: the cap counts both tagged and untagged sessions
    /// against the same canonical worktree.
    #[test]
    fn cap_counts_tagged_and_untagged_together() {
        let registry = SessionRegistry::new().with_per_worktree_cap(2);
        let wt = make_worktree();
        let now = Instant::now();
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);

        registry.register(&sid("u"), wt.path(), None, now).unwrap();
        registry
            .register(&sid("t1"), wt.path(), Some(&tag_a), now)
            .unwrap();
        let err = registry
            .register(&sid("t2"), wt.path(), None, now)
            .expect_err("untagged+tagged together hit the cap");
        // Either correctness path is acceptable: a duplicate-
        // untagged registration returns `WorktreeAlreadyOwned` (the
        // composite-key check fires first), while a third distinct
        // tag would return `SessionCapExceeded`. Both prove the cap
        // counts tagged + untagged together and refuses the third
        // attempt.
        assert!(
            matches!(err, RegistryError::WorktreeAlreadyOwned { .. })
                || matches!(
                    err,
                    RegistryError::SessionCapExceeded {
                        cap: 2,
                        live: 2,
                        ..
                    }
                ),
            "expected SessionCapExceeded or WorktreeAlreadyOwned, got {err:?}"
        );
    }

    /// MLP2-023: `SessionDispatcher::register` honours `agent_tag`
    /// through the trait surface (the IPC listener and the fence-
    /// gated `RegistryDispatcher` in `lib.rs` route through this trait).
    #[test]
    fn session_dispatcher_trait_propagates_agent_tag() {
        let registry: Arc<dyn SessionDispatcher> = Arc::new(SessionRegistry::new());
        let wt = make_worktree();
        let tag_a = tag("anvil-run", "via-dispatcher", 1_700_000_007);

        registry
            .register(&sid("d1"), wt.path(), Some(&tag_a), None)
            .unwrap();
        let listed = registry.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].agent_tag.as_ref(), Some(&tag_a));
    }

    /// MLP2-025b: `SessionDispatcher::register` with a non-`None`
    /// `lineage` seeds the registry's `(pid, pid_starttime)` index so
    /// a subsequent `lookup_tag_by_pid_starttime` finds the
    /// daemon-issued tag. Pins the trait-through path the daemon
    /// control-lane (B7) depends on.
    #[test]
    fn session_dispatcher_register_with_lineage_seeds_index() {
        let registry_arc = Arc::new(SessionRegistry::new());
        let dispatcher: Arc<dyn SessionDispatcher> = registry_arc.clone();
        let wt = make_worktree();
        let claimed = tag("anvil-run", "launcher-1", 1_700_000_100);
        let anchor = LineageAnchor {
            pid: 31337,
            pid_starttime: 1_700_000_100,
        };

        dispatcher
            .register(&sid("via-anchor"), wt.path(), Some(&claimed), Some(&anchor))
            .expect("register through trait with lineage");

        // Lookup the registry directly (concrete type) — the daemon-
        // issued tag mirror should now be set to the launcher's claim.
        let found = registry_arc
            .lookup_tag_by_pid_starttime(anchor.pid, anchor.pid_starttime)
            .expect("anchor populated by trait register");
        assert_eq!(found, claimed);
    }

    /// MLP2-025b: `SessionDispatcher::register` with `lineage = None`
    /// keeps the pre-MLP2-025b path — the lineage index stays empty
    /// and the legacy single-arg `register` semantics are preserved.
    #[test]
    fn session_dispatcher_register_without_lineage_skips_index() {
        let registry_arc = Arc::new(SessionRegistry::new());
        let dispatcher: Arc<dyn SessionDispatcher> = registry_arc.clone();
        let wt = make_worktree();

        dispatcher
            .register(&sid("legacy"), wt.path(), None, None)
            .expect("legacy register through trait");

        // Nothing in the lineage index.
        assert!(
            registry_arc.lookup_tag_by_pid_starttime(0, 0).is_none(),
            "no anchor was seeded for the legacy register path"
        );
    }

    /// MLP2-025b: `worktree_for_lineage` returns the worktree of any
    /// registered ancestor regardless of tag match. Used by the
    /// daemon control-lane on `Cross::Spoofed` to find a worktree to
    /// fence even when no tag matched.
    #[test]
    fn worktree_for_lineage_returns_registered_session_worktree() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let canonical = wt.path().canonicalize().expect("canonicalise");
        let issued = tag("anvil-run", "launcher", 1_700_000_900);

        registry
            .register_with_lineage(
                &sid("anchor"),
                wt.path(),
                None,
                Some(&issued),
                4242,
                1_700_000_900,
                Instant::now(),
            )
            .expect("register");

        // Direct lookup by (pid, starttime) returns the worktree.
        let inner = registry.lock();
        let sid_match = inner
            .by_pid_lineage
            .get(&(4242, 1_700_000_900))
            .expect("anchor present")
            .clone();
        drop(inner);
        let worktree = registry
            .lock()
            .sessions
            .get(&sid_match)
            .expect("session")
            .record
            .worktree
            .clone();
        assert_eq!(worktree, canonical);

        // No registered ancestor for a totally unrelated pid → None.
        assert!(
            registry.worktree_for_lineage(99_999).is_none(),
            "worktree_for_lineage returns None when no ancestor is registered"
        );
    }

    // MLP2-074: post-spawn lineage-anchor narrowing — the launcher
    // calls `session.report_process` after spawning the agent child,
    // and the daemon swings the `by_pid_lineage` index from the
    // launcher's anchor to the child's so the cross-check resolves
    // against the agent process, not the wrapping launcher.

    /// Happy-path narrowing: a session registered with the launcher's
    /// lineage has its anchor moved onto the child's
    /// `(pid, pid_starttime)` after `update_lineage_anchor`. The old
    /// launcher key disappears from the index; the new child key
    /// resolves to the same session id; and `record.pid` /
    /// `record.started_at_unix` follow the swap so MLP-014's
    /// PID-reuse defence compares against the child.
    #[test]
    fn update_lineage_anchor_narrows_from_launcher_to_child() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let issued = tag("anvil-run", "launcher", 1_700_000_900);
        let launcher_pid: u32 = 4242;
        let launcher_starttime: u64 = 1_700_000_900;
        let child_pid: u32 = 5151;
        let child_starttime: u64 = 1_700_000_950;

        registry
            .register_with_lineage(
                &sid("anchor"),
                wt.path(),
                None,
                Some(&issued),
                launcher_pid,
                launcher_starttime,
                Instant::now(),
            )
            .expect("register");

        // Pre-state: launcher anchor seeded; child anchor empty.
        {
            let inner = registry.lock();
            assert!(
                inner
                    .by_pid_lineage
                    .contains_key(&(launcher_pid, launcher_starttime))
            );
            assert!(
                !inner
                    .by_pid_lineage
                    .contains_key(&(child_pid, child_starttime))
            );
        }

        let updated = registry
            .update_lineage_anchor(&sid("anchor"), child_pid, child_starttime, launcher_pid)
            .expect("narrow to child");

        assert_eq!(updated.pid, Some(child_pid));
        assert_eq!(updated.started_at_unix, child_starttime);

        // Post-state: launcher key gone, child key resolves to the
        // same session id.
        let inner = registry.lock();
        assert!(
            !inner
                .by_pid_lineage
                .contains_key(&(launcher_pid, launcher_starttime)),
            "launcher lineage key must be dropped after narrowing"
        );
        assert_eq!(
            inner.by_pid_lineage.get(&(child_pid, child_starttime)),
            Some(&sid("anchor")),
            "child lineage key resolves to the same session"
        );
    }

    /// Peer-pid mismatch: a same-UID neighbour trying to mint a
    /// child anchor against someone else's registered session is
    /// rejected with the typed error, and the registry's lineage
    /// index is left unchanged.
    #[test]
    fn update_lineage_anchor_rejects_peer_pid_mismatch() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let issued = tag("anvil-run", "launcher", 1_700_001_000);
        let launcher_pid: u32 = 4242;
        let launcher_starttime: u64 = 1_700_001_000;
        registry
            .register_with_lineage(
                &sid("victim"),
                wt.path(),
                None,
                Some(&issued),
                launcher_pid,
                launcher_starttime,
                Instant::now(),
            )
            .expect("register");

        let err = registry
            .update_lineage_anchor(&sid("victim"), 6_666, 1_700_001_500, 9_999)
            .expect_err("peer pid 9999 != launcher pid 4242");
        assert_eq!(
            err,
            RegistryError::PeerOwnershipMismatch {
                session: sid("victim"),
                expected: Some(launcher_pid),
                actual: 9_999,
            }
        );

        // Index untouched: launcher anchor still present, no child
        // anchor inserted.
        let inner = registry.lock();
        assert!(
            inner
                .by_pid_lineage
                .contains_key(&(launcher_pid, launcher_starttime))
        );
        assert!(!inner.by_pid_lineage.contains_key(&(6_666, 1_700_001_500)));
    }

    /// Legacy-register path: a session registered without a lineage
    /// anchor (`record.pid == None`) has no launcher pid to verify
    /// against, so `update_lineage_anchor` rejects with
    /// `PeerOwnershipMismatch { expected: None, .. }` rather than
    /// silently adopting an unattributable child anchor.
    #[test]
    fn update_lineage_anchor_rejects_session_registered_without_lineage() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        registry
            .register(&sid("legacy"), wt.path(), None, Instant::now())
            .expect("legacy register");

        let err = registry
            .update_lineage_anchor(&sid("legacy"), 1234, 1_700_001_111, 4242)
            .expect_err("legacy register has no launcher pid");
        assert_eq!(
            err,
            RegistryError::PeerOwnershipMismatch {
                session: sid("legacy"),
                expected: None,
                actual: 4242,
            }
        );
    }

    /// Unknown session id surfaces `UnknownSession` — the registry
    /// may have evicted the session between the launcher's register
    /// and `report_process` calls; the launcher must see the typed
    /// error rather than a generic mismatch.
    #[test]
    fn update_lineage_anchor_unknown_session_returns_unknown_session() {
        let registry = SessionRegistry::new();
        let err = registry
            .update_lineage_anchor(&sid("ghost"), 1, 1, 1)
            .expect_err("ghost session id");
        assert_eq!(err, RegistryError::UnknownSession(sid("ghost")));
    }

    /// PR #1895 review: a cross-session collision on the child
    /// anchor must be rejected with the typed error rather than
    /// silently overwriting the victim's lineage index entry.
    /// The launcher controls `child_pid` on the wire (not
    /// constrained to `peer_pid`), so without this defence a
    /// malicious or buggy launcher could hijack lineage lookups
    /// for an unrelated session.
    #[test]
    fn update_lineage_anchor_rejects_cross_session_collision() {
        let registry = SessionRegistry::new();
        let wt_victim = make_worktree();
        let wt_attacker = make_worktree();
        let victim_launcher_pid: u32 = 1111;
        let victim_launcher_starttime: u64 = 1_700_002_000;
        let victim_child_pid: u32 = 2222;
        let victim_child_starttime: u64 = 1_700_002_100;
        let attacker_launcher_pid: u32 = 3333;
        let attacker_launcher_starttime: u64 = 1_700_002_200;

        // Victim narrows its anchor onto its child.
        let issued_v = tag("anvil-run", "victim", victim_launcher_starttime);
        registry
            .register_with_lineage(
                &sid("victim"),
                wt_victim.path(),
                None,
                Some(&issued_v),
                victim_launcher_pid,
                victim_launcher_starttime,
                Instant::now(),
            )
            .expect("register victim");
        registry
            .update_lineage_anchor(
                &sid("victim"),
                victim_child_pid,
                victim_child_starttime,
                victim_launcher_pid,
            )
            .expect("victim narrows to its own child");

        // Attacker registers, then tries to narrow onto the
        // victim's child anchor.
        let issued_a = tag("anvil-run", "attacker", attacker_launcher_starttime);
        registry
            .register_with_lineage(
                &sid("attacker"),
                wt_attacker.path(),
                None,
                Some(&issued_a),
                attacker_launcher_pid,
                attacker_launcher_starttime,
                Instant::now(),
            )
            .expect("register attacker");
        let err = registry
            .update_lineage_anchor(
                &sid("attacker"),
                victim_child_pid,
                victim_child_starttime,
                attacker_launcher_pid,
            )
            .expect_err("attacker must not steal victim's anchor");
        assert_eq!(
            err,
            RegistryError::LineageAnchorCollision {
                session: sid("attacker"),
                existing: sid("victim"),
                child_pid: victim_child_pid,
                child_pid_starttime: victim_child_starttime,
            }
        );

        // Victim's anchor still intact; attacker record still on
        // its launcher anchor (the swap never ran).
        let inner = registry.lock();
        assert_eq!(
            inner
                .by_pid_lineage
                .get(&(victim_child_pid, victim_child_starttime)),
            Some(&sid("victim")),
            "victim's child anchor must be preserved",
        );
        let attacker_record = &inner
            .sessions
            .get(&sid("attacker"))
            .expect("attacker still registered")
            .record;
        assert_eq!(attacker_record.pid, Some(attacker_launcher_pid));
        assert_eq!(
            attacker_record.started_at_unix, attacker_launcher_starttime,
            "attacker record must be unchanged after rejection",
        );
        assert_eq!(
            inner
                .by_pid_lineage
                .get(&(attacker_launcher_pid, attacker_launcher_starttime)),
            Some(&sid("attacker")),
            "attacker's launcher anchor must remain because the swap never ran",
        );
    }

    /// PR #1895 review: re-narrowing to the same
    /// `(child_pid, child_pid_starttime)` pair this session
    /// already owns is idempotent — the index already maps to
    /// `self`, so the collision check passes through and the
    /// swap runs harmlessly. Pin this so a future tightening of
    /// the collision rule does not break legitimate launchers
    /// that retry on a transient IPC error.
    #[test]
    fn update_lineage_anchor_idempotent_for_same_session() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let launcher_pid: u32 = 4242;
        let launcher_starttime: u64 = 1_700_003_000;
        let child_pid: u32 = 5151;
        let child_starttime: u64 = 1_700_003_100;
        let issued = tag("anvil-run", "launcher", launcher_starttime);

        registry
            .register_with_lineage(
                &sid("retry"),
                wt.path(),
                None,
                Some(&issued),
                launcher_pid,
                launcher_starttime,
                Instant::now(),
            )
            .expect("register");
        registry
            .update_lineage_anchor(&sid("retry"), child_pid, child_starttime, launcher_pid)
            .expect("first narrowing");

        // Second call with the same anchor — the index already
        // maps `(child_pid, child_starttime) -> sid("retry")`.
        // `record.pid` is now `child_pid` after the first swap,
        // so the peer-ownership check expects the peer to
        // present `child_pid` rather than the original launcher
        // pid. Real launchers see this on retry over the same
        // authenticated peer socket; the test forwards
        // `child_pid` as `peer_pid` to model that.
        registry
            .update_lineage_anchor(&sid("retry"), child_pid, child_starttime, child_pid)
            .expect("idempotent retry");

        let inner = registry.lock();
        assert_eq!(
            inner.by_pid_lineage.get(&(child_pid, child_starttime)),
            Some(&sid("retry")),
        );
    }

    // MLP2-057: unregister hook fires the daemon's cache-invalidation
    // callback on deliberate unregister and TTL eviction.

    /// `unregister` fires the hook with the unregistered session's
    /// canonical worktree path. Pinned so the cache-invalidation
    /// wire-up cannot regress without the test breaking first.
    #[test]
    fn unregister_fires_worktree_hook() {
        let hits = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let hits_for_hook = Arc::clone(&hits);
        let registry = SessionRegistry::new().with_unregister_hook(Arc::new(move |worktree| {
            hits_for_hook.lock().unwrap().push(worktree.to_path_buf());
        }));
        let wt = make_worktree();
        let canonical = wt.path().canonicalize().unwrap();

        registry
            .register(&sid("u1"), wt.path(), None, Instant::now())
            .unwrap();
        assert!(hits.lock().unwrap().is_empty());

        let removed = registry.unregister(&sid("u1")).unwrap();
        assert!(removed);
        let observed = hits.lock().unwrap().clone();
        assert_eq!(observed, vec![canonical]);
    }

    /// DSV: the post-construction `set_unregister_hook` installs the
    /// same hook the builder does, on an already-`Arc`-wrapped
    /// registry — the path `run_foreground` uses because the warm
    /// cache the hook reclaims is built after the registry. First
    /// install wins; a second returns `false` and does not replace.
    #[test]
    fn set_unregister_hook_installs_post_construction() {
        let hits = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let registry = Arc::new(SessionRegistry::new());

        let hits_for_hook = Arc::clone(&hits);
        let installed = registry.set_unregister_hook(Arc::new(move |worktree| {
            hits_for_hook.lock().unwrap().push(worktree.to_path_buf());
        }));
        assert!(installed, "first install must succeed on an empty OnceLock");

        // A second install is refused — the daemon composes every
        // invalidator into one closure, so a second set is a wiring bug.
        let refused = registry.set_unregister_hook(Arc::new(|_| {}));
        assert!(!refused, "second install must be refused, not replace");

        let wt = make_worktree();
        let canonical = wt.path().canonicalize().unwrap();
        registry
            .register(&sid("p1"), wt.path(), None, Instant::now())
            .unwrap();
        assert!(hits.lock().unwrap().is_empty());

        assert!(registry.unregister(&sid("p1")).unwrap());
        assert_eq!(hits.lock().unwrap().clone(), vec![canonical]);
    }

    /// DSV-040: the hook fires only when the LAST session for a worktree
    /// leaves. Two tagged sessions (MLP2-023) share one worktree;
    /// unregistering the first must NOT signal reclamation (the peer still
    /// holds warm state), and unregistering the second fires exactly once.
    #[test]
    fn unregister_hook_fires_only_on_last_session_for_worktree() {
        let hits = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let hits_for_hook = Arc::clone(&hits);
        let registry = SessionRegistry::new().with_unregister_hook(Arc::new(move |worktree| {
            hits_for_hook.lock().unwrap().push(worktree.to_path_buf());
        }));
        let wt = make_worktree();
        let canonical = wt.path().canonicalize().unwrap();
        let now = Instant::now();
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);
        let tag_b = tag("anvil-run", "claude-2", 1_700_000_002);
        registry
            .register(&sid("s-a"), wt.path(), Some(&tag_a), now)
            .unwrap();
        registry
            .register(&sid("s-b"), wt.path(), Some(&tag_b), now)
            .unwrap();

        // First leaves — a live peer remains, so no reclamation signal.
        assert!(registry.unregister(&sid("s-a")).unwrap());
        assert!(
            hits.lock().unwrap().is_empty(),
            "hook must not fire while a peer session still holds the worktree",
        );

        // Last leaves — fire exactly once.
        assert!(registry.unregister(&sid("s-b")).unwrap());
        assert_eq!(hits.lock().unwrap().clone(), vec![canonical]);
    }

    /// DSV-040: `evict_stale` likewise signals once per fully-drained
    /// worktree. Two tagged sessions on one worktree, both evicted, fire
    /// the hook a single time (not once per session).
    #[test]
    fn evict_stale_fires_hook_once_per_drained_worktree() {
        let hits = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let hits_for_hook = Arc::clone(&hits);
        let registry = SessionRegistry::with_ttl(Duration::from_millis(1)).with_unregister_hook(
            Arc::new(move |worktree| {
                hits_for_hook.lock().unwrap().push(worktree.to_path_buf());
            }),
        );
        let wt = make_worktree();
        let canonical = wt.path().canonicalize().unwrap();
        let registered_at = Instant::now();
        let tag_a = tag("anvil-run", "claude-1", 1_700_000_001);
        let tag_b = tag("anvil-run", "claude-2", 1_700_000_002);
        registry
            .register(&sid("e-a"), wt.path(), Some(&tag_a), registered_at)
            .unwrap();
        registry
            .register(&sid("e-b"), wt.path(), Some(&tag_b), registered_at)
            .unwrap();

        let evicted = registry.evict_stale(registered_at + Duration::from_millis(2));
        assert_eq!(evicted.len(), 2, "both sessions evicted");
        assert_eq!(
            hits.lock().unwrap().clone(),
            vec![canonical],
            "one signal for the drained worktree, not one per evicted session",
        );
    }

    /// `unregister` on an unknown id is a no-op and MUST NOT fire
    /// the hook — the wire layer races the daemon's eviction tick,
    /// and a spurious cache-invalidate on every unknown id would
    /// flush hot entries unnecessarily.
    #[test]
    fn unregister_unknown_id_does_not_fire_hook() {
        let hits = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let hits_for_hook = Arc::clone(&hits);
        let registry = SessionRegistry::new().with_unregister_hook(Arc::new(move |worktree| {
            hits_for_hook.lock().unwrap().push(worktree.to_path_buf());
        }));
        let removed = registry.unregister(&sid("never-registered")).unwrap();
        assert!(!removed);
        assert!(hits.lock().unwrap().is_empty());
    }

    /// `evict_stale` fires the hook once per evicted session. Two
    /// sessions on different worktrees → two callbacks with two
    /// distinct paths.
    #[test]
    fn evict_stale_fires_hook_per_evicted_session() {
        let hits = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let hits_for_hook = Arc::clone(&hits);
        let registry = SessionRegistry::with_ttl(Duration::from_millis(1)).with_unregister_hook(
            Arc::new(move |worktree| {
                hits_for_hook.lock().unwrap().push(worktree.to_path_buf());
            }),
        );
        let wt_a = make_worktree();
        let wt_b = make_worktree();
        let canon_a = wt_a.path().canonicalize().unwrap();
        let canon_b = wt_b.path().canonicalize().unwrap();
        let registered_at = Instant::now();
        registry
            .register(&sid("e1"), wt_a.path(), None, registered_at)
            .unwrap();
        registry
            .register(&sid("e2"), wt_b.path(), None, registered_at)
            .unwrap();

        // Move past the TTL window.
        let evicted = registry.evict_stale(registered_at + Duration::from_secs(1));
        assert_eq!(evicted.len(), 2);
        let mut observed = hits.lock().unwrap().clone();
        observed.sort();
        let mut expected = vec![canon_a, canon_b];
        expected.sort();
        assert_eq!(observed, expected);
    }

    /// Default `new()` produces a registry with no hook. Pre-MLP2-057
    /// embedded-mode callers that never wire a cache through still
    /// work — `unregister` is a pure data-structure mutation and
    /// must not require a hook.
    #[test]
    fn default_registry_has_no_hook_and_unregister_is_no_op_externally() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        registry
            .register(&sid("h1"), wt.path(), None, Instant::now())
            .unwrap();
        // Without a hook the unregister path runs to completion and
        // returns `true` with no side effects beyond the inner
        // hashmap mutation.
        assert!(registry.unregister(&sid("h1")).unwrap());
    }

    // ---- MLP2-025: lineage lookup + spoof rejection ----------------

    /// A session registered with `register_with_lineage` becomes
    /// findable via `lookup_tag_by_pid_starttime` for the exact
    /// `(pid, pid_starttime)` it was anchored with. This is the
    /// happy path: an ancestor of a writer is present in the registry
    /// and the lookup returns the daemon-issued tag.
    #[test]
    fn lineage_walk_finds_registered_ancestor() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let issued = tag("anvil-run", "claude-code-9", 1_700_000_042);
        registry
            .register_with_lineage(
                &sid("ancestor"),
                wt.path(),
                None,
                Some(&issued),
                12345,
                1_700_000_042,
                Instant::now(),
            )
            .expect("register with lineage");

        let found = registry
            .lookup_tag_by_pid_starttime(12345, 1_700_000_042)
            .expect("matching (pid, starttime) yields Some");
        assert_eq!(found, issued);

        // A different PID returns None.
        assert!(
            registry
                .lookup_tag_by_pid_starttime(99999, 1_700_000_042)
                .is_none(),
            "no session indexed at pid 99999 — must return None"
        );
    }

    /// MLP2-025 anti-spoof core: a registered session at
    /// `(pid=12345, pid_starttime=A)` must NOT be returned by a lookup
    /// for `(pid=12345, pid_starttime=B)`. PID reuse after a legitimate
    /// launcher exit is the canonical spoof scenario, and the
    /// `pid_starttime` component is what distinguishes the two process
    /// incarnations.
    #[test]
    fn lineage_walk_rejects_pid_reuse_without_starttime_match() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let issued = tag("anvil-run", "claude-code-9", 1_700_000_100);
        registry
            .register_with_lineage(
                &sid("legit"),
                wt.path(),
                None,
                Some(&issued),
                42,
                1_700_000_100,
                Instant::now(),
            )
            .expect("register legit");

        // Same PID, different start-time → not the same process.
        assert!(
            registry
                .lookup_tag_by_pid_starttime(42, 1_700_000_200)
                .is_none(),
            "PID reuse with different starttime must not match"
        );
        // Same PID, matching start-time → still the same process.
        assert_eq!(
            registry.lookup_tag_by_pid_starttime(42, 1_700_000_100),
            Some(issued),
        );
    }

    /// `register_with_lineage` populates the daemon-issued tag on the
    /// stored `SessionRecord`, distinct from the client-supplied
    /// `agent_tag` field. Pinning the contract so consumers reading
    /// `SessionRecord::daemon_issued_tag` after a registration see the
    /// daemon's value, not the client's.
    #[test]
    fn register_with_lineage_records_daemon_issued_tag_on_record() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let client = tag("anvil-run", "client-claim", 1_700_000_000);
        let issued = tag("anvil-run", "claude-code-9", 1_700_000_500);

        let record = registry
            .register_with_lineage(
                &sid("dual-tag"),
                wt.path(),
                Some(&client),
                Some(&issued),
                4242,
                1_700_000_500,
                Instant::now(),
            )
            .expect("register with lineage");

        assert_eq!(record.agent_tag, Some(client));
        assert_eq!(record.daemon_issued_tag, Some(issued));
    }

    // ---- MLP2-025: cross_check_env_tag classifier --------------------

    /// `Cross::classify` returns `Untagged` whenever the env tag is
    /// absent, regardless of what the registry knows. Untagged writes
    /// follow the pre-MLP2-025 enforcement path unchanged.
    #[test]
    fn missing_env_tag_leaves_session_untagged() {
        let registered = tag("anvil-run", "claude-code-9", 1_700_000_042);
        assert_eq!(Cross::classify(None, None), Cross::Untagged);
        assert_eq!(Cross::classify(None, Some(&registered)), Cross::Untagged);
    }

    /// `Cross::classify` returns `Match` when the env tag equals the
    /// daemon-issued tag found on the writer's lineage. Attribution
    /// is preserved.
    #[test]
    fn env_tag_match_preserves_attribution() {
        let env = tag("anvil-run", "claude-code-9", 1_700_000_042);
        let registered = env.clone();
        assert_eq!(Cross::classify(Some(&env), Some(&registered)), Cross::Match);
    }

    /// `Cross::classify` returns `Spoofed` when an env tag is present
    /// but no daemon-issued tag exists on the writer's lineage
    /// (out-of-lineage forgery) OR the lineage tag differs (mismatched
    /// claim). Both arms collapse to the same enforcement decision:
    /// strip attribution and downgrade to a worktree-level fence.
    #[test]
    fn env_tag_mismatch_strips_attribution() {
        let env = tag("anvil-run", "claude-code-9", 1_700_000_042);
        // No registered tag on the lineage.
        assert_eq!(Cross::classify(Some(&env), None), Cross::Spoofed);
        // Different driver.
        let other_driver = tag("malicious-driver", "claude-code-9", 1_700_000_042);
        assert_eq!(
            Cross::classify(Some(&env), Some(&other_driver)),
            Cross::Spoofed
        );
        // Different agent id.
        let other_agent = tag("anvil-run", "different-agent", 1_700_000_042);
        assert_eq!(
            Cross::classify(Some(&env), Some(&other_agent)),
            Cross::Spoofed
        );
        // Different pid_starttime (same name, different incarnation).
        let other_starttime = tag("anvil-run", "claude-code-9", 1_700_000_999);
        assert_eq!(
            Cross::classify(Some(&env), Some(&other_starttime)),
            Cross::Spoofed
        );
    }

    /// Unregistering a session also drops it from the lineage index;
    /// a subsequent `lookup_tag_by_pid_starttime` for the same
    /// `(pid, starttime)` returns `None`.
    #[test]
    fn unregister_drops_lineage_index_entry() {
        let registry = SessionRegistry::new();
        let wt = make_worktree();
        let issued = tag("anvil-run", "claude-code-9", 1_700_000_700);
        registry
            .register_with_lineage(
                &sid("temp"),
                wt.path(),
                None,
                Some(&issued),
                5000,
                1_700_000_700,
                Instant::now(),
            )
            .expect("register");
        assert!(
            registry
                .lookup_tag_by_pid_starttime(5000, 1_700_000_700)
                .is_some()
        );

        registry.unregister(&sid("temp")).expect("unregister");
        assert!(
            registry
                .lookup_tag_by_pid_starttime(5000, 1_700_000_700)
                .is_none(),
            "lineage index must drop entries on unregister"
        );
    }
}
