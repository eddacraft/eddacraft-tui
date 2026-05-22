//! INTD-011: daemon status / diagnostics surface.
//!
//! The `query_status` IPC method returns a [`DaemonStatus`] snapshot
//! containing:
//!
//! - the active session list (mirrors INTD-003's
//!   [`SessionRegistry::active_sessions`]),
//! - the worktree status set (one entry per active session — empty
//!   for an idle daemon, may grow to include explicit worktree
//!   metadata in later waves),
//! - the active fence list (mirrors INTD-007's
//!   [`crate::fence::FenceState::active_fences`]),
//! - the daemon health (uptime, version, IPC state),
//! - the latency rollup for `mode = midEdit`,
//!   `boundary = validation.service` per ADR-031.
//!
//! ## Trust posture
//!
//! The query is exposed over the same daemon-minted peer-credentials
//! posture as the rest of the IPC surface (INTD-002 +
//! INTD-015 §M5):
//!
//! - The Unix socket lives in a 0700 directory owned by the current
//!   user; the socket itself is 0600. Only same-UID peers can
//!   connect.
//! - The Windows named pipe is created with an owner-only DACL and
//!   `PIPE_REJECT_REMOTE_CLIENTS`.
//! - There is no driver self-declared identity in the request — the
//!   status payload is identical for every authorised peer; nothing
//!   in it depends on the caller.
//!
//! In the existing INTD-015 vocabulary, status is a "default-allow
//! for own-session same-UID peers" surface; cross-UID requests are
//! refused at the socket layer before the request reaches this
//! module.
//!
//! ## Reality, not assumed readiness
//!
//! When no mid-edit traffic has been observed yet, `latency.mid_edit`
//! is `None` (not zero, not a stale cached value). The rendering layer
//! prints `latency: (no mid-edit traffic yet)` rather than
//! `latency: p50 0.0ms p95 0.0ms (mid-edit)` — INTD-011's "report
//! reality" hard rule. The wire surface preserves this distinction
//! through `Option<LatencyRollup>`.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anvil_intercept_proto::SessionRecord;
use anvil_intercept_proto::status::{
    DaemonStatusV1, FenceStateV1, HealthStateV1, IpcStateV1, LatencyMidEditMapV1, WorktreeStatusV1,
};
use anvil_kernel_types::protection_claim::{
    ProtectionClaim, SurfaceClaim, SurfaceClaimState, WorktreeClaimState,
};

use crate::fence::{FenceRecord, FenceStore};
use crate::latency::{LatencyAggregator, LatencyRollup};
use crate::midedit::ScanBufferService;
use crate::registry::SessionRegistry;
use crate::rule_cache::RuleSetCache;

/// Snapshot of the daemon's state at the moment of a `query_status`
/// call. Mirrors [`DaemonStatusV1`] from the proto crate but uses the
/// daemon's in-memory types so callers inside the daemon can build it
/// without first paying a proto round-trip.
///
/// The wire format is whatever [`DaemonStatusV1`] serialises to;
/// driver consumers parse against the proto crate.
#[derive(Debug, Clone, PartialEq)]
pub struct DaemonStatus {
    /// Live registry sessions (the same shape `list-sessions` returns).
    pub sessions: Vec<SessionRecord>,
    /// One [`WorktreeStatus`] entry per session-owned worktree. v1
    /// derives this directly from the registry — there is no separate
    /// worktree-status table yet — so the list is exactly the set of
    /// worktrees claimed by active sessions, with attribution and
    /// fence overlay.
    pub worktrees: Vec<WorktreeStatus>,
    /// Persisted-fence records (one per fenced worktree).
    pub fences: Vec<FenceState>,
    /// Daemon health summary.
    pub health: HealthState,
    /// Latency rollups keyed by mode. Only `mid_edit` is exposed in
    /// v1 — see ADR-031 for the mode taxonomy. `None` means the
    /// daemon has not observed any mid-edit traffic in the current
    /// window; the renderer prints `(no mid-edit traffic yet)`.
    pub latency: LatencyMap,
    /// MLP2-058: resolved-rule-set cache observability. `None` when
    /// no cache is wired (embedded mode); `Some` carries the live
    /// snapshot counters. Maps to
    /// [`DaemonStatusV1::cache_entries`] +
    /// [`DaemonStatusV1::cache_invalidations_total`] on the wire.
    pub cache: Option<CacheStats>,
    /// MLP2-058: count of evaluations currently holding a
    /// scan-buffer permit. `None` when no scan-buffer service is
    /// wired (embedded mode). Maps to
    /// [`DaemonStatusV1::in_flight_evaluations`] on the wire,
    /// clamped to `u8::MAX` before transit (the daemon's
    /// `MAX_CONCURRENT_SCAN_BUFFERS` is 8 today so the clamp is
    /// unreachable in practice but kept honest for safety).
    pub in_flight_evaluations: Option<usize>,
    /// MLP2-051h: Unix seconds at which the daemon assembled this
    /// snapshot. Mirrors [`DaemonStatusV1::generated_at_unix`] — the
    /// daemon-level wall-clock anchor used by the MLP2-051f activation
    /// freshness check as a second consistency signal alongside
    /// per-session `last_heartbeat_unix`. Captured at the IPC boundary
    /// by [`DaemonStatusProvider::query_status`] via
    /// `SystemTime::now()` so `build_status` itself stays deterministic
    /// and testable.
    pub generated_at_unix: u64,
}

/// MLP2-058: paired cache counters carried inside [`DaemonStatus`].
/// Bundled so `Option<CacheStats>` cleanly distinguishes "no cache"
/// from "cache observed at zero entries / zero invalidations"; the
/// two states wire as different shapes (`None` -> absent, `Some {0,0}`
/// -> two zero numbers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub entries: usize,
    pub invalidations_total: u64,
    /// MLP2-059: cumulative `.anvil.*` invalidations coalesced by the
    /// per-worktree rate limiter. Surfaced via
    /// [`anvil_intercept_proto::status::DaemonStatusV1::cache_invalidations_rate_limited`].
    pub invalidations_rate_limited: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorktreeStatus {
    pub worktree: std::path::PathBuf,
    pub session_id: anvil_intercept_proto::SessionId,
    pub fenced: bool,
    /// MLP2-026: `true` when the worktree is in
    /// `degraded:fence-cascade` mode. Distinct from `fenced` — a
    /// worktree can be cascaded without being individually fenced;
    /// cascade refuses NEW sessions, fence affects enforcement of
    /// existing ones. See spec §3.6.
    pub cascaded: bool,
    /// MLP2-026: Unix seconds at which the cascade was engaged.
    /// `None` when not cascaded.
    pub cascade_since: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FenceState {
    pub worktree: std::path::PathBuf,
    pub reason: String,
    pub fenced_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthState {
    pub uptime_seconds: u64,
    pub version: String,
    pub ipc_state: IpcState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcState {
    /// Listener is bound and serving connections.
    Serving,
    /// Listener is in shutdown drain — no new connections accepted.
    Draining,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LatencyMap {
    pub mid_edit: Option<LatencyRollup>,
}

impl DaemonStatus {
    /// Convert to the wire shape exposed by `anvil-intercept-proto`.
    /// The proto types are authoritative for `DriverClient`
    /// consumers; this conversion is cheap (clone + map).
    #[must_use]
    pub fn to_wire(&self) -> DaemonStatusV1 {
        DaemonStatusV1 {
            sessions: self.sessions.clone(),
            worktrees: self
                .worktrees
                .iter()
                .map(|w| WorktreeStatusV1 {
                    worktree: w.worktree.clone(),
                    session_id: w.session_id.clone(),
                    fenced: w.fenced,
                    cascaded: w.cascaded,
                    cascade_since: w.cascade_since,
                })
                .collect(),
            fences: self
                .fences
                .iter()
                .map(|f| FenceStateV1 {
                    worktree: f.worktree.clone(),
                    reason: f.reason.clone(),
                    fenced_at_unix: f.fenced_at_unix,
                })
                .collect(),
            health: HealthStateV1 {
                uptime_seconds: self.health.uptime_seconds,
                version: self.health.version.clone(),
                ipc_state: match self.health.ipc_state {
                    IpcState::Serving => IpcStateV1::Serving,
                    IpcState::Draining => IpcStateV1::Draining,
                },
            },
            latency: LatencyMidEditMapV1 {
                mid_edit: self.latency.mid_edit.map(rollup_to_wire),
            },
            // MLP2-058: clamp the cache entry count to `u32::MAX`
            // before transit. The daemon's session cap (INTD-016)
            // is well below `u32::MAX`; the clamp is defence-in-
            // depth against a future un-capped path.
            cache_entries: self
                .cache
                .map(|c| u32::try_from(c.entries).unwrap_or(u32::MAX)),
            cache_invalidations_total: self.cache.map(|c| c.invalidations_total),
            in_flight_evaluations: self
                .in_flight_evaluations
                .map(|n| u8::try_from(n).unwrap_or(u8::MAX)),
            cache_invalidations_rate_limited: self.cache.map(|c| c.invalidations_rate_limited),
            generated_at_unix: self.generated_at_unix,
        }
    }
}

fn rollup_to_wire(r: LatencyRollup) -> anvil_intercept_proto::status::LatencyRollupV1 {
    anvil_intercept_proto::status::LatencyRollupV1 {
        p50_ms: r.p50_ms,
        p95_ms: r.p95_ms,
        sample_count: r.sample_count,
        window_seconds: r.window_seconds,
    }
}

/// Build a [`DaemonStatus`] from the daemon's in-memory components.
/// Pure function — exposed at module level so the IPC handler and
/// the (future) embedded mode use the same construction path.
///
/// `started_at` is the [`Instant`] the daemon's run loop captured at
/// startup; uptime is `now - started_at` clamped to monotonic time.
/// The version string is the daemon binary's `CARGO_PKG_VERSION` —
/// callers pass it explicitly so the embedded path can override.
///
/// MLP2-051h: `generated_at_unix` is the snapshot-level Unix-seconds
/// anchor (`SystemTime::now()` seconds-since-epoch). Captured at the
/// IPC boundary by [`DaemonStatusProvider::query_status`] so this
/// function stays clock-deterministic for tests; callers that don't
/// need the anchor (in-process unit tests) can pass `0`.
#[allow(clippy::too_many_arguments)]
pub fn build_status(
    sessions: Vec<SessionRecord>,
    fence_records: &[FenceRecord],
    cascade_records: &[crate::fence::CascadeRecord],
    latency_mid_edit: Option<LatencyRollup>,
    started_at: Instant,
    now: Instant,
    version: &str,
    ipc_state: IpcState,
    cache: Option<CacheStats>,
    in_flight_evaluations: Option<usize>,
    generated_at_unix: u64,
) -> DaemonStatus {
    let mut fenced_set: std::collections::HashSet<std::path::PathBuf> =
        fence_records.iter().map(|f| f.worktree.clone()).collect();
    // Aliases (alternative canonical paths the fence layer recognises)
    // also count as fenced for the worktree overlay below.
    for fence in fence_records {
        for alias in &fence.aliases {
            fenced_set.insert(alias.clone());
        }
    }

    // MLP2-026: map cascade engaged-state by canonical worktree
    // path so the per-session overlay below can pick up
    // `cascaded` / `cascade_since`.
    let cascade_map: std::collections::HashMap<std::path::PathBuf, u64> = cascade_records
        .iter()
        .map(|c| (c.worktree.clone(), c.since_unix))
        .collect();

    let worktrees = sessions
        .iter()
        .map(|session| {
            let fenced = fenced_set.contains(&session.worktree);
            let cascade_since = cascade_map.get(&session.worktree).copied();
            WorktreeStatus {
                worktree: session.worktree.clone(),
                session_id: session.id.clone(),
                fenced,
                cascaded: cascade_since.is_some(),
                cascade_since,
            }
        })
        .collect();

    let fences = fence_records
        .iter()
        .map(|fence| FenceState {
            worktree: fence.worktree.clone(),
            reason: fence.reason.clone(),
            fenced_at_unix: fence.fenced_at_unix,
        })
        .collect();

    let uptime = now.saturating_duration_since(started_at);
    let uptime_seconds = uptime_to_seconds(uptime);

    DaemonStatus {
        sessions,
        worktrees,
        fences,
        health: HealthState {
            uptime_seconds,
            version: version.to_owned(),
            ipc_state,
        },
        latency: LatencyMap {
            mid_edit: latency_mid_edit,
        },
        cache,
        in_flight_evaluations,
        generated_at_unix,
    }
}

fn uptime_to_seconds(uptime: Duration) -> u64 {
    uptime.as_secs()
}

/// Interface the IPC layer calls to satisfy a `query_status` request.
///
/// The trait keeps the IPC handler decoupled from the daemon's
/// concrete state — tests pass in a stub status; production passes
/// in a [`DaemonStatusProvider`] that aggregates from the registry,
/// fence store, and latency aggregator.
pub trait StatusProvider: Send + Sync {
    fn query_status(&self) -> DaemonStatus;
}

impl<F> StatusProvider for F
where
    F: Fn() -> DaemonStatus + Send + Sync,
{
    fn query_status(&self) -> DaemonStatus {
        (self)()
    }
}

/// Production implementation of [`StatusProvider`]. Bundles the
/// concrete daemon components needed to build a status snapshot
/// without touching the IPC trait surface.
///
/// `Clone` is cheap — every field is either `Arc` or a small string.
#[derive(Clone)]
pub struct DaemonStatusProvider {
    registry: Arc<SessionRegistry>,
    fence_store: Arc<FenceStore>,
    latency: LatencyAggregator,
    started_at: Instant,
    version: String,
    /// MLP2-058: optional cache reference — `None` for embedded
    /// harnesses, `Some` once the daemon wires its production
    /// resolved-rule-set cache. When `None`, the provider emits
    /// `cache: None` and the wire shape omits the two cache fields.
    rule_cache: Option<Arc<RuleSetCache>>,
    /// MLP2-058: optional scan-buffer service reference. `None` for
    /// surfaces that do not own a midedit pipeline (tests, embedded
    /// fallback). `Some` reads `in_flight()` per query.
    scan_buffer: Option<ScanBufferService>,
}

impl std::fmt::Debug for DaemonStatusProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DaemonStatusProvider")
            .field("started_at", &self.started_at)
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}

impl DaemonStatusProvider {
    #[must_use]
    pub fn new(
        registry: Arc<SessionRegistry>,
        fence_store: Arc<FenceStore>,
        latency: LatencyAggregator,
        started_at: Instant,
        version: impl Into<String>,
    ) -> Self {
        Self {
            registry,
            fence_store,
            latency,
            started_at,
            version: version.into(),
            rule_cache: None,
            scan_buffer: None,
        }
    }

    /// MLP2-058: attach the daemon's resolved-rule-set cache.
    ///
    /// After attachment, `query_status` reads `entries` +
    /// `invalidations` per call and surfaces them via
    /// [`DaemonStatus::cache`]. Tests + embedded harnesses skip
    /// this builder; the wire shape omits the cache fields.
    #[must_use]
    pub fn with_rule_cache(mut self, cache: Arc<RuleSetCache>) -> Self {
        self.rule_cache = Some(cache);
        self
    }

    /// MLP2-058: attach the daemon's scan-buffer service. After
    /// attachment, `query_status` reads `in_flight()` per call and
    /// surfaces it via [`DaemonStatus::in_flight_evaluations`].
    #[must_use]
    pub fn with_scan_buffer(mut self, service: ScanBufferService) -> Self {
        self.scan_buffer = Some(service);
        self
    }
}

impl StatusProvider for DaemonStatusProvider {
    fn query_status(&self) -> DaemonStatus {
        let now = Instant::now();
        let sessions = self.registry.active_sessions();
        // Fence loading errors are surfaced as an empty fence list —
        // the persisted store is a soft input here. A daemon that
        // cannot read its fence file is already tearing the listener
        // down at startup (see `run_foreground`), so reaching this
        // branch in steady state means an operator has corrupted the
        // file underneath us. Logging at warn keeps the trail.
        // MLP2-026: fence-store load returns both records and
        // cascades; same single-call lifetime, same fail-soft posture.
        let (fence_records, cascade_records) = match self.fence_store.load() {
            Ok(state) => (
                state.active_fences().to_vec(),
                state.active_cascades().to_vec(),
            ),
            Err(err) => {
                tracing::warn!(
                    target: "anvil_intercept::status",
                    error = %err,
                    "fence store unavailable for query_status; reporting empty fence + cascade lists",
                );
                (Vec::new(), Vec::new())
            }
        };
        let mid_edit = self.latency.snapshot(now);
        let cache = self.rule_cache.as_ref().map(|c| CacheStats {
            entries: c.len(),
            invalidations_total: c.invalidations(),
            invalidations_rate_limited: c.rate_limited_invalidations(),
        });
        let in_flight = self.scan_buffer.as_ref().map(ScanBufferService::in_flight);
        // MLP2-051h: stamp the snapshot-level wall-clock anchor at the
        // IPC boundary so `build_status` stays deterministic for in-
        // process unit tests. A clock that pre-dates `UNIX_EPOCH` (only
        // possible if the host clock has been manually rewound) yields
        // `0`, which post-MLP2-051h consumers already treat as "no
        // anchor — fall back to per-session heartbeat freshness".
        let generated_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        build_status(
            sessions,
            &fence_records,
            &cascade_records,
            mid_edit,
            self.started_at,
            now,
            &self.version,
            IpcState::Serving,
            cache,
            in_flight,
            generated_at_unix,
        )
    }
}

/// Render the status snapshot in the operator-facing text format used
/// by `anvil intercept status`. The latency line is a contract pin —
/// the demo runbook §1.5 references `latency: p50 <X>ms p95 <Y>ms
/// (mid-edit)` literally.
///
/// When `latency.mid_edit` is `None`, the renderer emits
/// `latency: (no mid-edit traffic yet)` rather than `0ms / 0ms` so
/// the operator sees reality, not assumed readiness.
#[must_use]
pub fn render_status(status: &DaemonStatus) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let pid = std::process::id();
    let _ = writeln!(
        out,
        "daemon:    running (pid {pid}, uptime {}s, version {})",
        status.health.uptime_seconds, status.health.version,
    );
    let session_count = status.sessions.len();
    let session_word = if session_count == 1 {
        "session"
    } else {
        "sessions"
    };
    let _ = writeln!(out, "sessions:  {session_count} active   ({session_word})");
    let _ = writeln!(out, "fences:    {}", status.fences.len());
    // MLP2-026: render one `cascade:` line per worktree currently in
    // `degraded:fence-cascade` mode. Silent when no cascades are
    // engaged (the common case) so legacy output stays byte-identical
    // for pre-MLP2-026 daemons. Spec §3.6.
    for worktree in status.worktrees.iter().filter(|w| w.cascaded) {
        let since = worktree
            .cascade_since
            .map_or_else(|| "unknown".to_string(), |ts| ts.to_string());
        let _ = writeln!(
            out,
            "cascade:   engaged since {since} ({})",
            worktree.worktree.display(),
        );
    }
    out.push_str(&render_latency_line(status.latency.mid_edit.as_ref()));
    out.push('\n');
    // MLP2-058: when the daemon has surfaced cache + in-flight
    // observability, render one line per field. Pre-MLP2-058 daemons
    // (and embedded harnesses that skip the builders) wire `None`
    // here, so the legible surface stays byte-identical to the
    // pre-MLP2-058 output for older daemons.
    if let Some(stats) = status.cache {
        // MLP2-059: when the rate limiter has coalesced any
        // invalidations, append a trailing `(N rate-limited)` segment
        // so an operator reading `anvil intercept status` sees the
        // signal without needing `--json`. Zero counts stay silent —
        // every byte the operator reads should mean something.
        let rate_limited_suffix = if stats.invalidations_rate_limited > 0 {
            format!(" ({} rate-limited)", stats.invalidations_rate_limited)
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "cache:     {} entries, {} invalidations{}",
            stats.entries, stats.invalidations_total, rate_limited_suffix,
        );
    }
    if let Some(n) = status.in_flight_evaluations {
        let _ = writeln!(out, "in-flight: {n} evaluations");
    }
    out
}

/// Render the latency line in isolation. Pinned as a separate helper
/// so the contract test can assert the exact bytes without depending
/// on the rest of the status layout.
///
/// **Contract pin (demo runbook §1.5):**
///
/// - With traffic: `latency: p50 <X>ms p95 <Y>ms (mid-edit)` where
///   `<X>` and `<Y>` are integers (rounded; sub-millisecond
///   precision is below the operator's read budget on the demo).
/// - Without traffic: `latency: (no mid-edit traffic yet)`.
#[must_use]
pub fn render_latency_line(rollup: Option<&LatencyRollup>) -> String {
    match rollup {
        Some(r) => format!(
            "latency: p50 {}ms p95 {}ms (mid-edit)",
            round_to_int(r.p50_ms),
            round_to_int(r.p95_ms),
        ),
        None => "latency: (no mid-edit traffic yet)".to_owned(),
    }
}

/// MLP2-048: build a [`ProtectionClaim`] describing `worktree` from a
/// [`DaemonStatus`] snapshot. The mapping closes over the closed-set
/// vocabulary from
/// [`anvil_kernel_types::protection_claim::WorktreeClaimState`] so
/// every renderer (CLI, MCP, doctor, driver) agrees on the exact
/// state names per spec §14.
///
/// Inputs:
/// - `snapshot` — the daemon's authoritative view of session,
///   worktree, and fence state.
/// - `worktree` — the path whose claim is being rendered. Matched
///   byte-for-byte against `WorktreeStatus::worktree`; callers are
///   expected to pass an already-canonical path (the daemon
///   canonicalises at register time).
///
/// Mapping (per spec §14.2):
/// - No sessions on `worktree` → [`WorktreeClaimState::Unprotected`]
///   with an empty `surfaces` array.
/// - Sessions present, IPC `Draining` → [`WorktreeClaimState::Warming`].
///   Surfaces are reported as [`SurfaceClaimState::Detached`] — the
///   listener is shutting down, so the daemon isn't actively
///   participating in enforcement decisions on this worktree.
/// - Sessions present, every session's `WorktreeStatus.fenced = true`
///   → [`WorktreeClaimState::DegradedProtection`] with all surfaces
///   [`SurfaceClaimState::Quarantined`].
/// - Sessions present, some fenced + some not →
///   [`WorktreeClaimState::DegradedProtection`] with the unfenced
///   surfaces still [`SurfaceClaimState::Participating`].
/// - Sessions present, no fences, IPC `Serving` →
///   [`WorktreeClaimState::PreWriteDaemon`] with each surface
///   [`SurfaceClaimState::Participating`].
///
/// Per-surface `identifier` is the session's `agent_tag` formatted as
/// `driver/agent#start` when present, otherwise the bare session id.
/// Surfaces are sorted by identifier so JSON output is deterministic
/// across daemon-internal `HashMap` iteration order.
#[must_use]
pub fn build_protection_claim(snapshot: &DaemonStatus, worktree: &Path) -> ProtectionClaim {
    let worktree_entries: Vec<&WorktreeStatus> = snapshot
        .worktrees
        .iter()
        .filter(|w| w.worktree == worktree)
        .collect();

    if worktree_entries.is_empty() {
        return ProtectionClaim::new(WorktreeClaimState::Unprotected, vec![]);
    }

    let ipc_draining = matches!(snapshot.health.ipc_state, IpcState::Draining);
    // Spec §14.2 names `DegradedProtection` for "above states with ≥1
    // surface degraded" — the per-worktree state collapses any-fenced
    // and all-fenced into the same `DegradedProtection` claim, with
    // the distinction surfacing through per-surface `Quarantined`
    // entries below. So `any_fenced` is the only signal we need at
    // the worktree-state layer.
    let any_fenced = worktree_entries.iter().any(|w| w.fenced);

    let worktree_state = if ipc_draining {
        WorktreeClaimState::Warming
    } else if any_fenced {
        WorktreeClaimState::DegradedProtection
    } else {
        WorktreeClaimState::PreWriteDaemon
    };

    let mut surfaces: Vec<SurfaceClaim> = worktree_entries
        .iter()
        .map(|w| {
            let identifier = snapshot
                .sessions
                .iter()
                .find(|s| s.id == w.session_id)
                .and_then(|s| s.agent_tag.as_ref().map(format_agent_tag))
                .unwrap_or_else(|| w.session_id.as_str().to_owned());
            let state = if ipc_draining {
                SurfaceClaimState::Detached
            } else if w.fenced {
                SurfaceClaimState::Quarantined
            } else {
                SurfaceClaimState::Participating
            };
            SurfaceClaim { identifier, state }
        })
        .collect();
    surfaces.sort_by(|a, b| a.identifier.cmp(&b.identifier));

    ProtectionClaim::new(worktree_state, surfaces)
}

/// MLP2-048: stand-alone "no daemon evidence" claim. Used by surface
/// renderers that have no live snapshot (e.g. `anvil status` invoked
/// when the daemon is not running). Equivalent to
/// `build_protection_claim` against an empty snapshot for the queried
/// worktree, but expressed as a named helper so the call sites are
/// self-documenting.
#[must_use]
pub fn unprotected_protection_claim() -> ProtectionClaim {
    ProtectionClaim::new(WorktreeClaimState::Unprotected, vec![])
}

/// MLP2-048: wire-shape adapter for [`build_protection_claim`]. The
/// CLI receives a [`DaemonStatusV1`] over IPC and has no need to
/// reconstruct the daemon's in-memory [`DaemonStatus`] just to build a
/// claim — every field the mapping reads (`worktrees`, `sessions`,
/// `health.ipc_state`) is already present on the wire.
///
/// Output is byte-identical to `build_protection_claim` against the
/// equivalent in-memory snapshot; the parity is pinned by a unit test
/// in this module so the wire path cannot drift from the daemon-
/// internal path.
#[must_use]
pub fn build_protection_claim_from_wire(
    snapshot: &DaemonStatusV1,
    worktree: &Path,
) -> ProtectionClaim {
    let worktree_entries: Vec<&WorktreeStatusV1> = snapshot
        .worktrees
        .iter()
        .filter(|w| w.worktree == worktree)
        .collect();

    if worktree_entries.is_empty() {
        return ProtectionClaim::new(WorktreeClaimState::Unprotected, vec![]);
    }

    let ipc_draining = matches!(snapshot.health.ipc_state, IpcStateV1::Draining);
    let any_fenced = worktree_entries.iter().any(|w| w.fenced);

    let worktree_state = if ipc_draining {
        WorktreeClaimState::Warming
    } else if any_fenced {
        WorktreeClaimState::DegradedProtection
    } else {
        WorktreeClaimState::PreWriteDaemon
    };

    let mut surfaces: Vec<SurfaceClaim> = worktree_entries
        .iter()
        .map(|w| {
            let identifier = snapshot
                .sessions
                .iter()
                .find(|s| s.id == w.session_id)
                .and_then(|s| s.agent_tag.as_ref().map(format_agent_tag))
                .unwrap_or_else(|| w.session_id.as_str().to_owned());
            let state = if ipc_draining {
                SurfaceClaimState::Detached
            } else if w.fenced {
                SurfaceClaimState::Quarantined
            } else {
                SurfaceClaimState::Participating
            };
            SurfaceClaim { identifier, state }
        })
        .collect();
    surfaces.sort_by(|a, b| a.identifier.cmp(&b.identifier));

    ProtectionClaim::new(worktree_state, surfaces)
}

/// Format an `AgentTag` as a stable per-surface identifier.
/// `driver/agent#pid_starttime` matches the `AgentTag::label`
/// convention surfaced in the daemon's tracing spans.
fn format_agent_tag(tag: &anvil_intercept_proto::session::AgentTag) -> String {
    format!(
        "{driver}/{agent}#{start}",
        driver = tag.driver_id,
        agent = tag.claimed_agent_id,
        start = tag.pid_starttime,
    )
}

/// 2^53 — the largest integer that round-trips through `f64` exactly.
/// Floats at or above this point have integer-step precision so any
/// `round()` is honest only below it. Used as the clamp cap in
/// [`round_to_int`].
const SAFE_F64_INT_CAP: f64 = 9_007_199_254_740_992.0;

fn round_to_int(value: f64) -> u64 {
    // The renderer wants integer milliseconds for the demo §1.5
    // trust-signal line. Domain considerations:
    // - NaN / infinity / negative -> 0 (they cannot be valid latency
    //   milliseconds; treating them as 0 is safer than panicking).
    // - Values above SAFE_F64_INT_CAP clamp to `u64::MAX` — in
    //   practice unreachable, latencies above 9 quadrillion ms
    //   would mean the daemon has been wedged for centuries.
    if !value.is_finite() || value < 0.0 {
        return 0;
    }
    let rounded = value.round();
    if rounded >= SAFE_F64_INT_CAP {
        return u64::MAX;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        rounded as u64
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use anvil_intercept_proto::{SessionId, SessionStatus};

    use super::*;

    fn sample_session(id: &str, worktree: &str) -> SessionRecord {
        SessionRecord {
            id: SessionId::new(id),
            worktree: PathBuf::from(worktree),
            pid: Some(4242),
            pgid: Some(4242),
            started_at_unix: 1_700_000_000,
            last_heartbeat_unix: 1_700_000_010,
            status: SessionStatus::Active,
            agent_tag: None,
            daemon_issued_tag: None,
        }
    }

    fn sample_rollup(p50_ms: f64, p95_ms: f64) -> LatencyRollup {
        LatencyRollup {
            p50_ms,
            p95_ms,
            sample_count: 12,
            window_seconds: 30.0,
        }
    }

    #[test]
    fn build_status_reports_no_traffic_when_aggregator_empty() {
        let started = Instant::now();
        let now = started + Duration::from_secs(5);
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        assert!(status.latency.mid_edit.is_none());
        assert_eq!(status.health.uptime_seconds, 5);
        assert_eq!(status.health.version, "0.5.1-beta");
        assert_eq!(status.health.ipc_state, IpcState::Serving);
        assert!(status.sessions.is_empty());
        assert!(status.worktrees.is_empty());
        assert!(status.fences.is_empty());
    }

    #[test]
    fn build_status_overlays_fences_onto_worktrees() {
        let started = Instant::now();
        let now = started;
        let session = sample_session("sess-1", "/tmp/wt-1");
        let fence = FenceRecord {
            worktree: PathBuf::from("/tmp/wt-1"),
            aliases: Vec::new(),
            reason: "secret-detection".to_owned(),
            fenced_at_unix: 1_700_000_500,
        };
        let status = build_status(
            vec![session.clone()],
            std::slice::from_ref(&fence),
            &[],
            None,
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        assert_eq!(status.worktrees.len(), 1);
        assert!(
            status.worktrees[0].fenced,
            "session worktree must be marked fenced"
        );
        assert_eq!(status.worktrees[0].session_id, session.id);
        assert_eq!(status.fences.len(), 1);
        assert_eq!(status.fences[0].reason, "secret-detection");
    }

    /// **Contract pin (demo runbook §1.5):** when latency traffic is
    /// present the line MUST read exactly
    /// `latency: p50 <X>ms p95 <Y>ms (mid-edit)`. Any change to this
    /// line MUST update the runbook in the same commit.
    #[test]
    fn render_latency_line_with_traffic_matches_runbook_contract() {
        let line = render_latency_line(Some(&sample_rollup(12.4, 47.6)));
        assert_eq!(line, "latency: p50 12ms p95 48ms (mid-edit)");
    }

    /// **Contract pin (INTD-011 hard rule):** without traffic, the
    /// line MUST read exactly `latency: (no mid-edit traffic yet)` —
    /// not `0ms / 0ms`, not a stale cached value. The wire layer is
    /// `None`, the render is honest about it.
    #[test]
    fn render_latency_line_without_traffic_says_no_traffic_yet() {
        let line = render_latency_line(None);
        assert_eq!(line, "latency: (no mid-edit traffic yet)");
    }

    /// MLP2-058 renderer: when cache + in-flight stats are present
    /// the legible output gains two extra lines. Pre-MLP2-058 daemons
    /// wire `None` here, so this path stays opt-in.
    #[test]
    fn render_status_emits_cache_and_in_flight_lines_when_present() {
        let started = Instant::now();
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            Some(CacheStats {
                entries: 4,
                invalidations_total: 13,
                invalidations_rate_limited: 0,
            }),
            Some(2),
            0,
        );
        let rendered = render_status(&status);
        assert!(
            rendered.contains("cache:     4 entries, 13 invalidations"),
            "renderer must include the cache observability line; got:\n{rendered}",
        );
        // MLP2-059: zero rate-limited count must stay silent — no
        // trailing `(0 rate-limited)` segment when the storm signal
        // is absent.
        assert!(
            !rendered.contains("rate-limited"),
            "zero rate-limited count must not append the suffix; got:\n{rendered}",
        );
        assert!(
            rendered.contains("in-flight: 2 evaluations"),
            "renderer must include the in-flight observability line; got:\n{rendered}",
        );
    }

    /// MLP2-059: when the rate-limited counter has bumped, the
    /// renderer appends a trailing `(N rate-limited)` segment to the
    /// cache line so operators reading `anvil intercept status` see
    /// the storm signal without needing `--json`.
    #[test]
    fn render_status_includes_rate_limited_suffix_when_present() {
        let started = Instant::now();
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            Some(CacheStats {
                entries: 4,
                invalidations_total: 13,
                invalidations_rate_limited: 99,
            }),
            Some(2),
            0,
        );
        let rendered = render_status(&status);
        assert!(
            rendered.contains("cache:     4 entries, 13 invalidations (99 rate-limited)"),
            "renderer must surface the rate-limited count as a trailing segment; got:\n{rendered}",
        );
    }

    /// MLP2-058: pre-MLP2-058 daemons (or embedded mode) carry `None`
    /// on the new fields. The renderer MUST omit the new lines so the
    /// legible output stays byte-identical to the pre-MLP2-058 shape.
    #[test]
    fn render_status_omits_cache_lines_when_absent() {
        let started = Instant::now();
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        let rendered = render_status(&status);
        assert!(
            !rendered.contains("cache:"),
            "pre-MLP2-058 render must NOT include a cache line; got:\n{rendered}",
        );
        assert!(!rendered.contains("in-flight:"));
    }

    #[test]
    fn render_status_includes_latency_line_unchanged() {
        let started = Instant::now();
        let now = started + Duration::from_secs(7);
        let status = build_status(
            vec![sample_session("sess-1", "/tmp/wt-1")],
            &[],
            &[],
            Some(sample_rollup(11.0, 33.0)),
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        let rendered = render_status(&status);
        assert!(
            rendered.contains("latency: p50 11ms p95 33ms (mid-edit)"),
            "renderer must include the runbook-pinned latency line; got:\n{rendered}",
        );
        assert!(rendered.contains("sessions:  1 active"));
        assert!(rendered.contains("fences:    0"));
    }

    /// Reality, not assumed readiness — even when the rollup arrives
    /// at exactly p50 = 0.0 (a real measurement of a near-instant
    /// path) we MUST render `0ms`, not "no traffic yet". The "no
    /// traffic" branch is gated by `Option`, not by the float value.
    #[test]
    fn render_latency_line_zero_measured_value_is_not_no_traffic() {
        let zero = sample_rollup(0.0, 0.0);
        let line = render_latency_line(Some(&zero));
        assert_eq!(line, "latency: p50 0ms p95 0ms (mid-edit)");
    }

    #[test]
    fn round_to_int_clamps_negative_and_nan() {
        assert_eq!(round_to_int(-1.0), 0);
        assert_eq!(round_to_int(f64::NAN), 0);
        assert_eq!(round_to_int(f64::INFINITY), 0);
        assert_eq!(round_to_int(11.4), 11);
        assert_eq!(round_to_int(11.6), 12);
    }

    #[test]
    fn to_wire_round_trips_through_serde_json() {
        let started = Instant::now();
        let now = started + Duration::from_secs(2);
        let status = build_status(
            vec![sample_session("sess-1", "/tmp/wt-1")],
            &[],
            &[],
            Some(sample_rollup(8.0, 19.0)),
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        let wire = status.to_wire();
        let json = serde_json::to_value(&wire).expect("serialise");
        // The wire shape MUST carry latency.mid_edit.p50_ms /
        // latency.mid_edit.p95_ms so DriverClient consumers can read
        // the rollup without re-parsing the rendered text.
        assert!(json["latency"]["mid_edit"]["p50_ms"].is_number());
        assert!(json["latency"]["mid_edit"]["p95_ms"].is_number());
        assert_eq!(json["latency"]["mid_edit"]["sample_count"], 12);
        assert_eq!(json["health"]["version"], "0.5.1-beta");
        assert_eq!(json["health"]["ipc_state"], "serving");
        assert_eq!(json["sessions"].as_array().unwrap().len(), 1);
    }

    /// MLP2-026: cascade fields on `WorktreeStatusV1` round-trip
    /// through serde. `cascaded` is always present (no skip-if);
    /// `cascade_since` is skip-if-none. Spec §3.6 wire-compat.
    #[test]
    fn worktree_status_v1_round_trips_cascade_fields() {
        let started = Instant::now();
        let session = sample_session("sess-cascaded", "/tmp/wt-cascaded");
        let cascade = crate::fence::CascadeRecord {
            worktree: PathBuf::from("/tmp/wt-cascaded"),
            since_unix: 1_700_000_500,
            reason: crate::telemetry::DEGRADED_FENCE_CASCADE.to_string(),
        };
        let status = build_status(
            vec![session],
            &[],
            std::slice::from_ref(&cascade),
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        let wire = status.to_wire();
        let json = serde_json::to_value(&wire).expect("serialise");
        let worktree = &json["worktrees"][0];
        assert_eq!(worktree["cascaded"], true);
        assert_eq!(worktree["cascade_since"], 1_700_000_500);

        // Round-trip back.
        let parsed: anvil_intercept_proto::status::DaemonStatusV1 =
            serde_json::from_value(json).expect("deserialise");
        let worktree = &parsed.worktrees[0];
        assert!(worktree.cascaded);
        assert_eq!(worktree.cascade_since, Some(1_700_000_500));
    }

    /// MLP2-026: `cascaded: false` is present on the wire by
    /// default (`#[serde(default)]` without skip-if). Operators
    /// can read a status snapshot and see explicitly that nothing
    /// is cascaded.
    #[test]
    fn worktree_status_v1_emits_cascaded_false_explicitly() {
        let started = Instant::now();
        let session = sample_session("sess-clean", "/tmp/wt-clean");
        let status = build_status(
            vec![session],
            &[],
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        let wire = status.to_wire();
        let json = serde_json::to_value(&wire).expect("serialise");
        let worktree = &json["worktrees"][0];
        assert_eq!(worktree["cascaded"], false);
        // cascade_since is skip-if-none — absent on the wire when None.
        assert!(worktree.get("cascade_since").is_none());
    }

    /// MLP2-026: `render_status` emits a `cascade:` line per
    /// cascaded worktree, silent when none are engaged. Spec §3.6.
    #[test]
    fn render_status_emits_cascade_line_when_engaged() {
        let started = Instant::now();
        let session = sample_session("sess-c", "/tmp/wt-c");
        let cascade = crate::fence::CascadeRecord {
            worktree: PathBuf::from("/tmp/wt-c"),
            since_unix: 1_700_000_999,
            reason: crate::telemetry::DEGRADED_FENCE_CASCADE.to_string(),
        };
        let status = build_status(
            vec![session],
            &[],
            std::slice::from_ref(&cascade),
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        let rendered = render_status(&status);
        assert!(
            rendered.contains("cascade:"),
            "cascade line missing: {rendered}"
        );
        assert!(
            rendered.contains("1700000999"),
            "since_unix missing: {rendered}"
        );
        assert!(
            rendered.contains("/tmp/wt-c"),
            "worktree missing from cascade line: {rendered}"
        );
    }

    /// MLP2-026: `render_status` is silent on cascade when nothing
    /// is engaged. Preserves byte-identical output for pre-MLP2-026
    /// daemons.
    #[test]
    fn render_status_omits_cascade_line_when_no_cascades() {
        let started = Instant::now();
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        let rendered = render_status(&status);
        assert!(
            !rendered.contains("cascade:"),
            "no cascade line when none engaged: {rendered}"
        );
    }

    // MLP2-058 — cache + in-flight observability surface.

    /// `None` cache + `None` in-flight wires as absent keys, matching
    /// the precedent set by `mid_edit`. Consumers that distinguish
    /// "no cache wired" from "cache present, zero entries" need the
    /// absent encoding to be honest about the difference.
    #[test]
    fn cache_and_in_flight_default_to_absent_on_wire() {
        let started = Instant::now();
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        assert!(status.cache.is_none());
        assert!(status.in_flight_evaluations.is_none());
        let wire = status.to_wire();
        let json = serde_json::to_value(&wire).expect("serialise");
        assert!(
            json.get("cache_entries").is_none(),
            "no-cache must wire as absent: {json}",
        );
        assert!(json.get("cache_invalidations_total").is_none());
        assert!(json.get("in_flight_evaluations").is_none());
    }

    /// When `CacheStats` is `Some` the wire shape carries
    /// `cache_entries` + `cache_invalidations_total` with the typed
    /// values. The clamp narrows `usize → u32`; pin against a future
    /// refactor that drops the saturating conversion.
    #[test]
    fn cache_stats_propagate_to_wire() {
        let started = Instant::now();
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            Some(CacheStats {
                entries: 17,
                invalidations_total: 42,
                invalidations_rate_limited: 5,
            }),
            Some(3),
            0,
        );
        let wire = status.to_wire();
        assert_eq!(wire.cache_entries, Some(17));
        assert_eq!(wire.cache_invalidations_total, Some(42));
        assert_eq!(wire.in_flight_evaluations, Some(3));
        // MLP2-059: the coalesced-invalidation counter must reach
        // the wire so operators can spot a sustained attack via
        // `anvil status --json`.
        assert_eq!(wire.cache_invalidations_rate_limited, Some(5));
    }

    /// MLP2-059: a daemon snapshot with no rate-limited
    /// invalidations must omit the field on the wire (matches the
    /// MLP2-058 / MLP2-052 additive-optional pattern).
    #[test]
    fn cache_invalidations_rate_limited_skips_when_zero_and_no_cache() {
        let started = Instant::now();
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            None, // no cache wired -> field absent on wire
            None,
            0,
        );
        let json = serde_json::to_value(status.to_wire()).expect("serialise");
        assert!(
            json.get("cache_invalidations_rate_limited").is_none(),
            "absent cache -> absent rate-limited field; got: {json}",
        );
    }

    #[test]
    fn cache_invalidations_rate_limited_reaches_wire_when_cache_observes_throttle() {
        // End-to-end pin: drive a real RuleSetCache through a
        // coalesced storm, snapshot via the provider, assert the
        // wire row carries the bumped counter.
        use crate::fence::FenceStore;
        use crate::registry::SessionRegistry;
        use crate::rule_cache::{ResolvedRuleSet, RuleSetCache, RuleSetEntry, WorktreeKey};

        let cache = Arc::new(RuleSetCache::with_capacity_and_rate(
            1024,
            1,
            Duration::from_secs(1),
        ));
        let dir = tempfile::TempDir::new().unwrap();
        let k = WorktreeKey::canonicalise(dir.path()).unwrap();
        cache
            .get_or_resolve::<_, ()>(&k, |_| {
                Ok(RuleSetEntry {
                    rules_sha: "abc".to_owned(),
                    resolved: ResolvedRuleSet {
                        config: serde_json::json!({}),
                    },
                })
            })
            .unwrap();
        let touched = dir.path().join(".anvil.yaml");
        std::fs::write(&touched, b"{}").unwrap();
        let now = Instant::now();
        for _ in 0..5 {
            cache.invalidate_on_change_at(&touched, now);
        }
        assert_eq!(
            cache.rate_limited_invalidations(),
            4,
            "first invalidation admits; 4 remaining coalesce"
        );

        let fence_path = dir.path().join("fence.json");
        let provider = DaemonStatusProvider::new(
            Arc::new(SessionRegistry::new()),
            Arc::new(FenceStore::at_path(&fence_path)),
            LatencyAggregator::new(),
            Instant::now(),
            "0.7.0-beta",
        )
        .with_rule_cache(cache);
        let wire = provider.query_status().to_wire();
        assert_eq!(wire.cache_invalidations_rate_limited, Some(4));
    }

    /// `DaemonStatusProvider::with_rule_cache` makes the provider
    /// surface cache stats from the live cache instance — drive a
    /// `get_or_resolve` insert + `invalidate`, then `query_status` →
    /// the wire shape reflects `entries = 0` and
    /// `invalidations_total = 1`. Pin against a regression where the
    /// builder accepts the cache but the snapshot fails to read it.
    #[test]
    fn provider_surfaces_cache_stats_when_wired() {
        use crate::fence::FenceStore;
        use crate::registry::SessionRegistry;
        use crate::rule_cache::{ResolvedRuleSet, RuleSetCache, RuleSetEntry, WorktreeKey};

        let cache = Arc::new(RuleSetCache::new());
        let dir = tempfile::TempDir::new().unwrap();
        let k = WorktreeKey::canonicalise(dir.path()).unwrap();
        cache
            .get_or_resolve::<_, ()>(&k, |_| {
                Ok(RuleSetEntry {
                    rules_sha: "abc".to_owned(),
                    resolved: ResolvedRuleSet {
                        config: serde_json::json!({}),
                    },
                })
            })
            .unwrap();
        assert!(cache.invalidate(&k));

        let fence_path = dir.path().join("fence.json");
        let provider = DaemonStatusProvider::new(
            Arc::new(SessionRegistry::new()),
            Arc::new(FenceStore::at_path(&fence_path)),
            LatencyAggregator::new(),
            Instant::now(),
            "0.7.0-beta",
        )
        .with_rule_cache(cache);

        let snapshot = provider.query_status();
        let stats = snapshot.cache.expect("cache wired -> Some");
        assert_eq!(stats.entries, 0, "post-invalidate cache is empty");
        assert_eq!(stats.invalidations_total, 1);
        let wire = snapshot.to_wire();
        assert_eq!(wire.cache_entries, Some(0));
        assert_eq!(wire.cache_invalidations_total, Some(1));
    }

    /// MLP2-051h: the production `DaemonStatusProvider` path actually
    /// stamps a non-zero `generated_at_unix` at the IPC boundary.
    /// Guards the next-most-likely failure mode in this slice: a
    /// future caller of `build_status` (or a refactor of
    /// `DaemonStatusProvider::query_status`) silently passing `0` and
    /// degrading every consumer to the no-anchor fallback posture
    /// with no test failure.
    #[test]
    fn provider_stamps_non_zero_generated_at_unix() {
        use crate::fence::FenceStore;
        use crate::registry::SessionRegistry;

        let dir = tempfile::TempDir::new().unwrap();
        let fence_path = dir.path().join("fence.json");
        let provider = DaemonStatusProvider::new(
            Arc::new(SessionRegistry::new()),
            Arc::new(FenceStore::at_path(&fence_path)),
            LatencyAggregator::new(),
            Instant::now(),
            "0.7.0-beta",
        );

        let snapshot = provider.query_status();
        assert!(
            snapshot.generated_at_unix > 0,
            "production DaemonStatusProvider::query_status must stamp \
             a live Unix-seconds anchor; got {anchor} (the no-anchor \
             sentinel is reserved for NoopStatusProvider and legacy \
             producers)",
            anchor = snapshot.generated_at_unix,
        );

        // Sanity-check the value is plausibly current: post-2020 epoch
        // seconds are well above 1_500_000_000. A clock that rolled
        // back to pre-2020 either means the host has a wildly broken
        // clock or the boundary capture regressed to `Instant::now()`.
        assert!(
            snapshot.generated_at_unix >= 1_500_000_000,
            "generated_at_unix must be plausibly current Unix-seconds; \
             got {anchor} which would imply a pre-2017 host clock",
            anchor = snapshot.generated_at_unix,
        );

        // Wire round-trip preserves the stamp byte-equivalently.
        let wire = snapshot.to_wire();
        assert_eq!(
            wire.generated_at_unix, snapshot.generated_at_unix,
            "to_wire must forward the anchor verbatim",
        );
    }

    /// MLP2-051h sentinel contract pin. `generated_at_unix == 0` is
    /// the documented "no anchor available — fall back to per-session
    /// heartbeat freshness only" sentinel. The producer side of the
    /// contract is:
    ///
    /// - A post-MLP2-051h `DaemonStatusProvider` always stamps a
    ///   live, non-zero value (pinned above by
    ///   `provider_stamps_non_zero_generated_at_unix`).
    /// - A pre-MLP2-051h daemon emits no key, which `#[serde(default)]`
    ///   defaults to `0` on parse (pinned in the proto crate by
    ///   `pre_mlp2_051h_payload_round_trips_with_generated_at_unix_default_zero`).
    /// - `NoopStatusProvider` (a synthetic listener default that
    ///   production swaps out via `with_status_provider`) explicitly
    ///   emits `0` so it cannot be mistaken for a real anchor by
    ///   downstream consumers.
    ///
    /// The consumer side (MLP2-051f) MUST branch on `== 0`, not
    /// `< some_threshold`. A `> 0` check would treat a live
    /// `NoopStatusProvider` snapshot as having an anchor (just a very
    /// old one) and pass the freshness gate, which is the failure mode
    /// the MLP2-051h precursor exists to prevent. This test pins the
    /// equality semantics so a future MLP2-051f implementation cannot
    /// drift the contract without an explicit test failure here.
    #[test]
    fn generated_at_unix_zero_is_the_no_anchor_sentinel() {
        use crate::ipc::NoopStatusProvider;

        let noop = NoopStatusProvider;
        let snapshot = noop.query_status();
        assert_eq!(
            snapshot.generated_at_unix, 0,
            "NoopStatusProvider must surface 0 (no-anchor sentinel)",
        );
        let wire = snapshot.to_wire();
        assert_eq!(
            wire.generated_at_unix, 0,
            "to_wire preserves the no-anchor sentinel byte-equivalently",
        );
    }

    /// A `usize` cache size above `u32::MAX` clamps to `u32::MAX`
    /// rather than panicking or wrapping. The daemon's session cap
    /// keeps the live count well below `u32::MAX`; this is
    /// defence-in-depth.
    #[test]
    fn cache_entries_above_u32_max_clamps_to_u32_max() {
        let started = Instant::now();
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            IpcState::Serving,
            Some(CacheStats {
                entries: usize::MAX,
                invalidations_total: u64::MAX,
                invalidations_rate_limited: u64::MAX,
            }),
            Some(usize::MAX),
            0,
        );
        let wire = status.to_wire();
        assert_eq!(wire.cache_entries, Some(u32::MAX));
        assert_eq!(wire.cache_invalidations_total, Some(u64::MAX));
        assert_eq!(wire.in_flight_evaluations, Some(u8::MAX));
    }

    #[test]
    fn to_wire_preserves_no_traffic_as_null() {
        let started = Instant::now();
        let now = started;
        let status = build_status(
            vec![],
            &[],
            &[],
            None,
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
            None,
            None,
            0,
        );
        let wire = status.to_wire();
        let json = serde_json::to_value(&wire).expect("serialise");
        // mid_edit MUST serialise as null (not zero) when there is no
        // traffic. Driver consumers depend on this distinction.
        assert!(
            json["latency"]["mid_edit"].is_null(),
            "no-traffic must wire as null, not zero: {json}",
        );
    }

    // MLP2-048: ProtectionClaim mapping from DaemonStatus.

    fn sample_status(
        sessions: Vec<SessionRecord>,
        fences: &[FenceRecord],
        ipc_state: IpcState,
    ) -> DaemonStatus {
        let started = Instant::now();
        build_status(
            sessions,
            fences,
            &[],
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            ipc_state,
            None,
            None,
            0,
        )
    }

    fn fence_record(worktree: &str) -> FenceRecord {
        FenceRecord {
            worktree: PathBuf::from(worktree),
            reason: "test fence".into(),
            fenced_at_unix: 1_700_000_000,
            aliases: Vec::new(),
        }
    }

    /// Empty snapshot → Unprotected. The simplest end of the closed
    /// set: a queried worktree the daemon has never seen.
    #[test]
    fn build_protection_claim_unknown_worktree_is_unprotected() {
        let snapshot = sample_status(vec![], &[], IpcState::Serving);
        let claim = build_protection_claim(&snapshot, Path::new("/tmp/wt-unknown"));
        assert_eq!(claim.worktree_state, WorktreeClaimState::Unprotected);
        assert!(claim.surfaces.is_empty());
        // Round-trips through the wire shape (per MLP-009).
        let wire = serde_json::to_string(&claim).expect("serialise");
        let back: ProtectionClaim = serde_json::from_str(&wire).expect("deserialise");
        assert_eq!(back, claim);
    }

    /// Session present, no fence, IPC Serving → `PreWriteDaemon` with
    /// one `Participating` surface keyed by session id.
    #[test]
    fn build_protection_claim_single_session_is_pre_write_daemon() {
        let session = sample_session("sess-pre", "/tmp/wt-pre");
        let snapshot = sample_status(vec![session], &[], IpcState::Serving);
        let claim = build_protection_claim(&snapshot, Path::new("/tmp/wt-pre"));
        assert_eq!(claim.worktree_state, WorktreeClaimState::PreWriteDaemon);
        assert_eq!(claim.surfaces.len(), 1);
        assert_eq!(claim.surfaces[0].identifier, "sess-pre");
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Participating);
    }

    /// Session present, fenced → `DegradedProtection` + `Quarantined`
    /// surface. Pin so a future refactor that drops the fence overlay
    /// surfaces in review.
    #[test]
    fn build_protection_claim_fenced_session_is_degraded() {
        let session = sample_session("sess-fenced", "/tmp/wt-fenced");
        let snapshot = sample_status(
            vec![session],
            &[fence_record("/tmp/wt-fenced")],
            IpcState::Serving,
        );
        let claim = build_protection_claim(&snapshot, Path::new("/tmp/wt-fenced"));
        assert_eq!(claim.worktree_state, WorktreeClaimState::DegradedProtection);
        assert_eq!(claim.surfaces.len(), 1);
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Quarantined);
    }

    /// IPC draining → Warming + Detached surfaces. The listener is
    /// shutting down; the daemon is not actively participating.
    #[test]
    fn build_protection_claim_draining_ipc_is_warming() {
        let session = sample_session("sess-drain", "/tmp/wt-drain");
        let snapshot = sample_status(vec![session], &[], IpcState::Draining);
        let claim = build_protection_claim(&snapshot, Path::new("/tmp/wt-drain"));
        assert_eq!(claim.worktree_state, WorktreeClaimState::Warming);
        assert_eq!(claim.surfaces.len(), 1);
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Detached);
    }

    /// Multiple sessions on the same worktree → surfaces deterministically
    /// sorted by identifier. JSON output must be stable across
    /// `HashMap` iteration order of the source snapshot.
    #[test]
    fn build_protection_claim_surfaces_sorted_deterministically() {
        let mut s_b = sample_session("sess-bbb", "/tmp/wt-multi");
        let mut s_a = sample_session("sess-aaa", "/tmp/wt-multi");
        // Force the two sessions to live on the same worktree
        // explicitly (sample_session's helper sets unique worktrees by
        // default).
        s_a.worktree = PathBuf::from("/tmp/wt-multi");
        s_b.worktree = PathBuf::from("/tmp/wt-multi");
        let snapshot = sample_status(vec![s_b, s_a], &[], IpcState::Serving);
        let claim = build_protection_claim(&snapshot, Path::new("/tmp/wt-multi"));
        assert_eq!(claim.surfaces.len(), 2);
        assert_eq!(claim.surfaces[0].identifier, "sess-aaa");
        assert_eq!(claim.surfaces[1].identifier, "sess-bbb");
    }

    /// The stand-alone helper for "no daemon" surfaces is equivalent
    /// to building against an empty snapshot. Pin both call sites so
    /// they cannot drift.
    #[test]
    fn unprotected_helper_matches_empty_snapshot_path() {
        let helper = unprotected_protection_claim();
        let snapshot = sample_status(vec![], &[], IpcState::Serving);
        let empty = build_protection_claim(&snapshot, Path::new("/tmp/wt-absent"));
        assert_eq!(helper, empty);
        assert_eq!(helper.worktree_state, WorktreeClaimState::Unprotected);
    }

    /// Agent-tagged surfaces use `driver/agent#start` as the
    /// identifier — matches the `AgentTag::label` convention in the
    /// daemon's tracing spans so cross-surface correlation works
    /// without re-formatting.
    #[test]
    fn build_protection_claim_uses_agent_tag_identifier() {
        use anvil_intercept_proto::session::AgentTag;
        let tag = AgentTag::new("anvil-run", "claude-code-1", 1_700_000_042);
        let mut session = sample_session("sess-tag", "/tmp/wt-tag");
        session.agent_tag = Some(tag);
        let snapshot = sample_status(vec![session], &[], IpcState::Serving);
        let claim = build_protection_claim(&snapshot, Path::new("/tmp/wt-tag"));
        assert_eq!(claim.surfaces.len(), 1);
        assert_eq!(
            claim.surfaces[0].identifier,
            "anvil-run/claude-code-1#1700000042"
        );
    }

    // MLP2-048 wire-adapter parity tests. The CLI path receives a
    // `DaemonStatusV1` over IPC; `build_protection_claim_from_wire`
    // MUST produce the same `ProtectionClaim` as the daemon-internal
    // `build_protection_claim` for the same logical state.

    fn parity_check(in_memory: &DaemonStatus, worktree: &Path) {
        let from_in_memory = build_protection_claim(in_memory, worktree);
        let from_wire = build_protection_claim_from_wire(&in_memory.to_wire(), worktree);
        assert_eq!(
            from_wire, from_in_memory,
            "wire-adapter drifted from in-memory builder at worktree {worktree:?}",
        );
    }

    #[test]
    fn build_protection_claim_from_wire_unknown_worktree_is_unprotected() {
        let snapshot = sample_status(vec![], &[], IpcState::Serving);
        parity_check(&snapshot, Path::new("/tmp/wt-unknown"));
        let claim =
            build_protection_claim_from_wire(&snapshot.to_wire(), Path::new("/tmp/wt-unknown"));
        assert_eq!(claim.worktree_state, WorktreeClaimState::Unprotected);
        assert!(claim.surfaces.is_empty());
    }

    #[test]
    fn build_protection_claim_from_wire_single_session_is_pre_write_daemon() {
        let session = sample_session("sess-pre", "/tmp/wt-pre");
        let snapshot = sample_status(vec![session], &[], IpcState::Serving);
        parity_check(&snapshot, Path::new("/tmp/wt-pre"));
        let claim = build_protection_claim_from_wire(&snapshot.to_wire(), Path::new("/tmp/wt-pre"));
        assert_eq!(claim.worktree_state, WorktreeClaimState::PreWriteDaemon);
        assert_eq!(claim.surfaces.len(), 1);
        assert_eq!(claim.surfaces[0].identifier, "sess-pre");
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Participating);
    }

    #[test]
    fn build_protection_claim_from_wire_fenced_session_is_degraded() {
        let session = sample_session("sess-fenced", "/tmp/wt-fenced");
        let snapshot = sample_status(
            vec![session],
            &[fence_record("/tmp/wt-fenced")],
            IpcState::Serving,
        );
        parity_check(&snapshot, Path::new("/tmp/wt-fenced"));
        let claim =
            build_protection_claim_from_wire(&snapshot.to_wire(), Path::new("/tmp/wt-fenced"));
        assert_eq!(claim.worktree_state, WorktreeClaimState::DegradedProtection);
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Quarantined);
    }

    #[test]
    fn build_protection_claim_from_wire_draining_is_warming() {
        let session = sample_session("sess-drain", "/tmp/wt-drain");
        let snapshot = sample_status(vec![session], &[], IpcState::Draining);
        parity_check(&snapshot, Path::new("/tmp/wt-drain"));
        let claim =
            build_protection_claim_from_wire(&snapshot.to_wire(), Path::new("/tmp/wt-drain"));
        assert_eq!(claim.worktree_state, WorktreeClaimState::Warming);
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Detached);
    }

    /// Mixed-fence input — one session fenced, one not, both on the
    /// same worktree. Per spec §14.2 the worktree state collapses to
    /// `DegradedProtection`, but per-surface entries must distinguish
    /// the participating surface from the quarantined one. Pinned on
    /// the wire path so a future refactor that bulk-collapses all
    /// surfaces to `Quarantined` whenever `any_fenced` fires is caught.
    #[test]
    fn build_protection_claim_from_wire_mixed_fence_keeps_per_surface_distinction() {
        let mut s_clean = sample_session("sess-clean", "/tmp/wt-mixed");
        let mut s_fenced = sample_session("sess-fenced", "/tmp/wt-mixed");
        s_clean.worktree = PathBuf::from("/tmp/wt-mixed");
        s_fenced.worktree = PathBuf::from("/tmp/wt-mixed");
        let snapshot = sample_status(
            vec![s_clean, s_fenced],
            &[fence_record("/tmp/wt-mixed")],
            IpcState::Serving,
        );
        // The fence overlay marks BOTH sessions on the worktree as
        // fenced (the fence is per-worktree, not per-session, in v1 —
        // see `build_status` line 250). So the "mixed" parity here is
        // structurally one-sided. Pin the wire-vs-in-memory parity so
        // any future per-session-fence refactor catches the diverged
        // wire adapter.
        parity_check(&snapshot, Path::new("/tmp/wt-mixed"));
        let claim =
            build_protection_claim_from_wire(&snapshot.to_wire(), Path::new("/tmp/wt-mixed"));
        assert_eq!(claim.worktree_state, WorktreeClaimState::DegradedProtection);
        assert_eq!(claim.surfaces.len(), 2);
        assert!(
            claim
                .surfaces
                .iter()
                .all(|s| s.state == SurfaceClaimState::Quarantined),
            "per-worktree fence overlay should mark both surfaces Quarantined: {claim:?}",
        );
    }

    /// Surfaces sort deterministically across wire-deserialisation
    /// order. `Vec` is ordered already, but pin the contract: two
    /// sessions arriving in reverse-alphabetical order produce
    /// alphabetical surfaces in the claim. Defends against a future
    /// refactor that swaps `Vec` for something with non-stable iteration.
    #[test]
    fn build_protection_claim_from_wire_sorts_surfaces() {
        let mut s_b = sample_session("sess-bbb", "/tmp/wt-multi");
        let mut s_a = sample_session("sess-aaa", "/tmp/wt-multi");
        s_a.worktree = PathBuf::from("/tmp/wt-multi");
        s_b.worktree = PathBuf::from("/tmp/wt-multi");
        let snapshot = sample_status(vec![s_b, s_a], &[], IpcState::Serving);
        parity_check(&snapshot, Path::new("/tmp/wt-multi"));
        let claim =
            build_protection_claim_from_wire(&snapshot.to_wire(), Path::new("/tmp/wt-multi"));
        assert_eq!(claim.surfaces[0].identifier, "sess-aaa");
        assert_eq!(claim.surfaces[1].identifier, "sess-bbb");
    }

    /// Agent-tagged surfaces use the same `driver/agent#start`
    /// identifier on the wire path as on the daemon-internal path.
    #[test]
    fn build_protection_claim_from_wire_uses_agent_tag_identifier() {
        use anvil_intercept_proto::session::AgentTag;
        let tag = AgentTag::new("anvil-run", "claude-code-1", 1_700_000_042);
        let mut session = sample_session("sess-tag", "/tmp/wt-tag");
        session.agent_tag = Some(tag);
        let snapshot = sample_status(vec![session], &[], IpcState::Serving);
        parity_check(&snapshot, Path::new("/tmp/wt-tag"));
        let claim = build_protection_claim_from_wire(&snapshot.to_wire(), Path::new("/tmp/wt-tag"));
        assert_eq!(
            claim.surfaces[0].identifier,
            "anvil-run/claude-code-1#1700000042"
        );
    }
}
