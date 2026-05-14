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
use std::time::{Duration, Instant};

use anvil_intercept_proto::SessionRecord;
use anvil_intercept_proto::status::{
    DaemonStatusV1, FenceStateV1, HealthStateV1, IpcStateV1, LatencyMidEditMapV1, WorktreeStatusV1,
};
use anvil_kernel_types::protection_claim::{
    ProtectionClaim, SurfaceClaim, SurfaceClaimState, WorktreeClaimState,
};

use crate::fence::{FenceRecord, FenceStore};
use crate::latency::{LatencyAggregator, LatencyRollup};
use crate::registry::SessionRegistry;

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorktreeStatus {
    pub worktree: std::path::PathBuf,
    pub session_id: anvil_intercept_proto::SessionId,
    pub fenced: bool,
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
pub fn build_status(
    sessions: Vec<SessionRecord>,
    fence_records: &[FenceRecord],
    latency_mid_edit: Option<LatencyRollup>,
    started_at: Instant,
    now: Instant,
    version: &str,
    ipc_state: IpcState,
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

    let worktrees = sessions
        .iter()
        .map(|session| {
            let fenced = fenced_set.contains(&session.worktree);
            WorktreeStatus {
                worktree: session.worktree.clone(),
                session_id: session.id.clone(),
                fenced,
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
        }
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
        let fence_records = match self.fence_store.load() {
            Ok(state) => state.active_fences().to_vec(),
            Err(err) => {
                tracing::warn!(
                    target: "anvil_intercept::status",
                    error = %err,
                    "fence store unavailable for query_status; reporting empty fence list",
                );
                Vec::new()
            }
        };
        let mid_edit = self.latency.snapshot(now);
        build_status(
            sessions,
            &fence_records,
            mid_edit,
            self.started_at,
            now,
            &self.version,
            IpcState::Serving,
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
    out.push_str(&render_latency_line(status.latency.mid_edit.as_ref()));
    out.push('\n');
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
            None,
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
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
            None,
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
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

    #[test]
    fn render_status_includes_latency_line_unchanged() {
        let started = Instant::now();
        let now = started + Duration::from_secs(7);
        let status = build_status(
            vec![sample_session("sess-1", "/tmp/wt-1")],
            &[],
            Some(sample_rollup(11.0, 33.0)),
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
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
            Some(sample_rollup(8.0, 19.0)),
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
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

    #[test]
    fn to_wire_preserves_no_traffic_as_null() {
        let started = Instant::now();
        let now = started;
        let status = build_status(
            vec![],
            &[],
            None,
            started,
            now,
            "0.5.1-beta",
            IpcState::Serving,
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
            None,
            started,
            started + Duration::from_secs(1),
            "0.7.0-beta",
            ipc_state,
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
}
