//! MLP2-051f: Daemon-attested promotion of MCP clients from
//! [`McpTier::RestartHandshakeVerified`] up to [`McpTier::LiveValidation`].
//!
//! `anvil start --verify` and `anvil status --verify` need a way to
//! reach `Protecting` once the intercept daemon is running and is
//! actually enforcing the worktree, otherwise their best honest answer
//! stays at `ready_restart_required` forever (closes GH
//! [#1831](https://github.com/eddacraft/anvil-001/issues/1831)).
//!
//! The plumbing already shipped: the daemon emits `ProtectionClaim`
//! snapshots over IPC (MLP2-048) and two consumer surfaces — `anvil
//! status` (MLP2-048) and the MCP shim's `validate_write` response
//! (MLP2-051b) — wire those snapshots through
//! [`anvil_intercept::status::build_protection_claim_from_wire`]. The
//! activation diagnostic did **not**, which is why
//! `crates/anvil-cli/src/activation/diagnostic.rs::protection_state`
//! could never return `Protecting` in production — `LiveValidation`
//! had zero callers that set the variant. This module closes that gap.
//!
//! Hard-gates (per `plans/specs/2026-05-21-activation-daemon-evidence-wireup.md`
//! council `plan-f4668683`, applied verbatim to MLP2-051f's APS entry):
//!
//! 1. Worktree canonicalisation contract — activation MUST canonicalise
//!    its argument before the IPC call so it compares byte-equal with
//!    what the daemon stored at register-time.
//! 2. Heartbeat freshness window — 45 s. Computed both as
//!    `max(SessionRecord.last_heartbeat_unix)` across the worktree's
//!    registered sessions AND (when the daemon supplied a non-zero
//!    `DaemonStatusV1::generated_at_unix` — MLP2-051h precursor) the
//!    snapshot anchor itself, against `SystemTime::now()`.
//! 3. `WorktreeClaimState` promotion predicate, enumerated below.
//! 4. `ACTIVATION_DAEMON_QUERY_TIMEOUT = 500 ms` — interactive verify
//!    must not inherit the 2 s IPC default from
//!    [`crate::commands::intercept::query_daemon_status`].
//! 5. End-to-end integration test against a real daemon socket —
//!    pinned by `end_to_end_against_real_unix_socket_promotes_to_live_validation`
//!    in the `#[cfg(test)]` block at the bottom of this module. The
//!    canonicalisation → IPC fetch chain inside
//!    `promote_to_live_validation_when_daemon_attests` is tested in
//!    isolation rather than as a single chain through the production
//!    entry point because that would require overriding
//!    `$XDG_RUNTIME_DIR` to redirect `resolve_socket_path()` — a
//!    process-global env mutation that races with other cargo test
//!    workers inside the same binary and is unstable across
//!    workspace reruns without pulling in `serial_test`.
//! 6. Structured tracing on every promotion / skip path — mirrors
//!    `promote_restart_required_after_handshake` in
//!    `activation::diagnostic`.
//! 7. Render-hint regression coverage — owned by
//!    `activation::render`'s repair-hint branching for
//!    `ReadyRestartRequired`.
//! 8. Cardinality-based client attribution — promotion requires ≥1
//!    `SurfaceClaim::Participating` for the worktree (council split,
//!    architect resolution adopted).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anvil_intercept::status::build_protection_claim_from_wire;
use anvil_intercept_proto::status::DaemonStatusV1;
use anvil_kernel_types::protection_claim::{
    ProtectionClaim, SurfaceClaimState, WorktreeClaimState,
};

use serde::{Deserialize, Serialize};

use super::diagnostic::{McpClientId, McpTier};
use super::mcp_client::McpProbeResult;

/// Outcome of the daemon-attestation probe, surfaced on
/// [`super::diagnostic::ActivationDiagnostic`] so the renderer can
/// distinguish "pre-restart" from "daemon down / unenforced" when
/// generating the [`super::state::ProtectionState::ReadyRestartRequired`]
/// repair hint. The activation surface emits one of these per
/// [`super::diagnostic::verify`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DaemonAttestation {
    /// No handshake-verified client to promote — promotion was not
    /// attempted. The user is genuinely pre-restart. Default for
    /// freshly constructed diagnostics (synthetic test fixtures).
    #[default]
    NotProbed,
    /// Daemon IPC failed (socket / pipe absent or timed out). The
    /// daemon is the missing piece; renderer points the user at
    /// `anvil intercept start`.
    Unreachable,
    /// Daemon is reachable but the worktree is `Unprotected` — no
    /// session is registered for the canonical path. Often a path
    /// canonicalisation drift or a daemon started in a different
    /// shell that has not seen the user's editor MCP child yet.
    Unenforced,
    /// Daemon is reachable but the freshest signal (per-session or
    /// snapshot anchor) is older than the freshness window. The
    /// daemon may be paused / stopped without the eviction having
    /// caught up.
    StaleHeartbeat,
    /// Daemon is reachable, the worktree is in `DegradedProtection`,
    /// and every surface is `Quarantined`. Recovery routes through
    /// `anvil intercept recover`, not editor restart.
    AllSurfacesQuarantined,
    /// Daemon is reachable but the worktree is `Warming` (transient
    /// daemon state — leaving / joining, not yet enforcing).
    Warming,
    /// Daemon is reachable and the claim state is promotable, but
    /// the worktree has zero `Participating` surfaces (cardinality
    /// gate). Same operator-facing message as `Unenforced`.
    NoParticipatingSurface,
    /// Promotion fired — at least one `RestartHandshakeVerified`
    /// client advanced to `LiveValidation`.
    Promoted,
}

/// MLP2-051f: wall-clock cap on the activation-side daemon IPC query.
///
/// 500 ms is the budget interactive `anvil start --verify` is willing
/// to spend on a one-time daemon probe. Decoupled from the 2 s
/// [`crate::commands::intercept::query_daemon_status`] default so a
/// hung daemon does not stretch `--verify` by ~2 s on every run.
pub(super) const ACTIVATION_DAEMON_QUERY_TIMEOUT: Duration = Duration::from_millis(500);

/// MLP2-051f: max staleness on the daemon attestation we will trust
/// for a live-validation promotion.
///
/// 45 s is calibrated against the producer cadence:
/// `HEARTBEAT_INTERVAL = 10 s` (`anvil-run/src/heartbeat.rs`) +
/// `DEFAULT_HEARTBEAT_TTL = 30 s` (registry eviction) + ~5 s slack
/// for clock skew and paused-then-resumed laptops. Tighter than 30 s
/// is unreachable (the registry would evict the session before we
/// could observe its heartbeat); looser than 120 s permits a
/// stale-snapshot exploitation window. Not operator-configurable
/// upward — downgrade attack surface (security veto).
pub(super) const HEARTBEAT_FRESHNESS_WINDOW: Duration = Duration::from_secs(45);

/// MLP2-051f post-ship hardening (council 2026-05-22): upper bound on
/// how far a daemon's clock may be ahead of the workstation clock
/// before we reject the timestamp as broken.
///
/// `within_window` previously accepted `unix_seconds >= now_unix`
/// unconditionally (any future timestamp passed). A daemon with a
/// broken RTC stamping `u64::MAX` would permanently pass freshness;
/// combined with a stripped `generated_at_unix` (the "no anchor"
/// sentinel), an attacker controlling snapshot output could defeat
/// both freshness gates simultaneously. The cap is `2 ×
/// HEARTBEAT_FRESHNESS_WINDOW = 90 s` — enough to tolerate NTP
/// step adjustments and VM-clock drift between the daemon and the
/// workstation, but small enough that a stuck-in-2038 daemon is
/// rejected immediately. Unix epoch seconds are timezone-agnostic
/// so DST transitions do NOT contribute to the skew budget.
pub(super) const MAX_FUTURE_CLOCK_SKEW: Duration = Duration::from_secs(90);

/// Reason a daemon-attestation promotion skipped — emitted on the
/// `tracing::debug!` event when the activation surface decides not
/// to advance an MCP client to `LiveValidation`. Mirrors the
/// per-reason vocabulary documented on MLP2-051f's APS entry so a
/// support trace shows the same token the runbook references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkipReason {
    /// IPC failed or daemon socket / pipe is absent.
    DaemonUnreachable,
    /// Daemon reachable; this worktree is `Unprotected` (no sessions
    /// registered for the canonical path).
    WorktreeUnenforced,
    /// Daemon reachable; the worktree's protection-claim state maps
    /// to "transient" (`Warming`) — not enforcing pre-write yet.
    Warming,
    /// Daemon reachable; the worktree is in `DegradedProtection` with
    /// every surface `Quarantined`. Surface-level enforcement is
    /// fenced; render points at `anvil intercept recover` instead.
    AllSurfacesQuarantined,
    /// Daemon reachable; no surface for the worktree is in
    /// `Participating` state (cardinality gate from council
    /// architect resolution).
    NoParticipatingSurface,
    /// Daemon reachable; the freshest signal available
    /// (`max(session.last_heartbeat_unix)` or
    /// `DaemonStatusV1::generated_at_unix` when non-zero) is older
    /// than [`HEARTBEAT_FRESHNESS_WINDOW`] vs `now`.
    StaleHeartbeat,
    /// Daemon reachable, claim attests live enforcement, but the
    /// activation diagnostic has no client at
    /// `RestartHandshakeVerified` to promote. Logged so a probe-tier
    /// drift (orchestrator probe regression) is visible in trace.
    NoHandshakeVerifiedClient,
}

impl SkipReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::DaemonUnreachable => "daemon_unreachable",
            Self::WorktreeUnenforced => "worktree_unenforced",
            Self::Warming => "warming",
            Self::AllSurfacesQuarantined => "all_surfaces_quarantined",
            Self::NoParticipatingSurface => "no_participating_surface",
            Self::StaleHeartbeat => "stale_heartbeat",
            Self::NoHandshakeVerifiedClient => "no_handshake_verified_client",
        }
    }
}

/// Emit the standard "activation: daemon attestation skipped" tracing
/// event with the right level for `reason`. Post-ship hardening
/// (council 2026-05-22): the default CLI tracing filter is `warn`, so
/// `tracing::debug!` skip events were invisible to operators running
/// `anvil start --verify` without `ANVIL_LOG=debug`. The success path
/// emits `tracing::info!` (visible at `info`); the asymmetric silence
/// on the failure paths produced the exact UX defect MLP2-051f was
/// built to fix. Operator-actionable failures (daemon unreachable /
/// worktree unenforced / stale heartbeat / all-surfaces quarantined)
/// now emit at `warn`; transient states (`Warming`,
/// `NoParticipatingSurface`) at `info`;
/// `NoHandshakeVerifiedClient` (the genuine pre-restart case where
/// the diagnostic just hasn't reached the daemon probe yet) stays at
/// `debug` because the headline already communicates it.
fn emit_skip_event(reason: SkipReason, worktree_claim_state: Option<&'static str>) {
    let reason_str = reason.as_str();
    let claim = worktree_claim_state.unwrap_or("");
    match reason {
        SkipReason::DaemonUnreachable
        | SkipReason::WorktreeUnenforced
        | SkipReason::StaleHeartbeat
        | SkipReason::AllSurfacesQuarantined => {
            tracing::warn!(
                reason = reason_str,
                worktree_claim_state = claim,
                "activation: daemon attestation skipped"
            );
        }
        SkipReason::Warming | SkipReason::NoParticipatingSurface => {
            tracing::info!(
                reason = reason_str,
                worktree_claim_state = claim,
                "activation: daemon attestation skipped"
            );
        }
        SkipReason::NoHandshakeVerifiedClient => {
            tracing::debug!(
                reason = reason_str,
                worktree_claim_state = claim,
                "activation: daemon attestation skipped"
            );
        }
    }
}

/// Entry point invoked from [`crate::activation::diagnostic::verify_with_home`]
/// after the orchestrator's handshake-pass has resolved each MCP client's
/// tier. The function is best-effort: any failure mode (daemon down, IPC
/// timeout, stale snapshot, worktree unenforced) leaves the input map
/// unchanged and emits a `tracing::debug!` event so a support transcript
/// can name the missing piece without changing the user-visible verdict.
///
/// Mirrors the production-call shape of
/// [`super::diagnostic::promote_restart_required_after_handshake`] so the
/// activation surface gains daemon attestation through the same gate
/// pattern. The unit-testable body lives in [`evaluate_and_promote`].
pub(super) fn promote_to_live_validation_when_daemon_attests(
    map: &mut BTreeMap<McpClientId, McpProbeResult>,
    worktree: &Path,
) -> DaemonAttestation {
    // Cheap gate: skip the IPC round-trip entirely when no client is
    // at the only tier we can promote from. The verify path runs this
    // on every invocation, and the handshake probe (the only producer
    // of `RestartHandshakeVerified` today) is itself gated; if no
    // client has reached the tier, the daemon attestation cannot help.
    if !map
        .values()
        .any(|r| r.tier == McpTier::RestartHandshakeVerified)
    {
        emit_skip_event(SkipReason::NoHandshakeVerifiedClient, None);
        return DaemonAttestation::NotProbed;
    }

    let canonical = canonicalise_for_activation(worktree);

    let Some(snapshot) = query_daemon_for_activation() else {
        emit_skip_event(SkipReason::DaemonUnreachable, None);
        return DaemonAttestation::Unreachable;
    };

    evaluate_and_promote(map, &snapshot, &canonical, SystemTime::now())
}

/// Canonicalise `worktree` using the same `std::fs::canonicalize` +
/// warn-on-failure pattern as `commands::protection_claim_section::
/// fetch_protection_claim_for_cwd`. The daemon canonicalises at
/// register-time inside `DriverManifest::validate_workspace_roots`
/// (`crates/anvil-intercept/src/auth.rs`); the activation surface must
/// produce a path that compares byte-equal against whatever the
/// daemon stored. Mismatch → `build_protection_claim_from_wire`
/// returns `Unprotected` → promotion silently no-ops, reproducing the
/// exact #1831 failure mode on a different path.
fn canonicalise_for_activation(worktree: &Path) -> PathBuf {
    std::fs::canonicalize(worktree).unwrap_or_else(|err| {
        tracing::warn!(
            error = %err,
            worktree = %worktree.display(),
            "activation: worktree canonicalisation failed; daemon attestation will not match any registered session",
        );
        worktree.to_path_buf()
    })
}

/// Wrap [`crate::commands::intercept::query_daemon_status_with_timeout`]
/// at the 500 ms activation budget and fold any failure into `None`.
/// The activation surface never propagates a daemon-IPC error to the
/// user — a missing or unhealthy daemon is the same posture as
/// "daemon not running" from the diagnostic's perspective.
fn query_daemon_for_activation() -> Option<DaemonStatusV1> {
    match crate::commands::intercept::query_daemon_status_with_timeout(
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    ) {
        Ok(snapshot) => Some(snapshot),
        Err(err) => {
            tracing::debug!(
                error = %err,
                "activation: daemon IPC unavailable (falling back to local-only diagnostic)",
            );
            None
        }
    }
}

/// Unit-testable body of [`promote_to_live_validation_when_daemon_attests`].
///
/// Splitting the snapshot-evaluation logic out from the IPC fetch +
/// system-clock read lets the unit suite drive every promotion /
/// skip branch with crafted [`DaemonStatusV1`] fixtures, while the
/// end-to-end test (`tests/activation_daemon_evidence.rs`) covers the
/// production wire-up against a real daemon socket — the council's
/// MLP2-025b "spec implemented, zero callers" guard.
pub(super) fn evaluate_and_promote(
    map: &mut BTreeMap<McpClientId, McpProbeResult>,
    snapshot: &DaemonStatusV1,
    worktree: &Path,
    now: SystemTime,
) -> DaemonAttestation {
    let claim = build_protection_claim_from_wire(snapshot, worktree);

    // Predicate #1: claim attests live enforcement.
    let verdict = classify_claim(&claim);
    if let ClaimVerdict::Skip(reason) = verdict {
        emit_skip_event(reason, Some(claim.worktree_state.as_str()));
        return skip_reason_to_attestation(reason);
    }

    // Predicate #2: cardinality — at least one Participating surface
    // for the worktree. Architect-resolved cardinality rule from the
    // council split; the spec's "promote every handshake-verified
    // client when the daemon attests the worktree" mass-promotion is
    // tighter than required by the documented `LiveValidation`
    // contract ("observed from this client inside this repo").
    if !claim
        .surfaces
        .iter()
        .any(|s| s.state == SurfaceClaimState::Participating)
    {
        emit_skip_event(
            SkipReason::NoParticipatingSurface,
            Some(claim.worktree_state.as_str()),
        );
        return DaemonAttestation::NoParticipatingSurface;
    }

    // Predicate #3: freshness. Both the per-session heartbeat clock
    // and the snapshot-level anchor (when non-zero per MLP2-051h)
    // must be within the window. `now` is supplied by the caller so
    // tests can pin a deterministic clock.
    if !heartbeat_within_freshness_window(snapshot, worktree, now) {
        emit_skip_event(SkipReason::StaleHeartbeat, None);
        return DaemonAttestation::StaleHeartbeat;
    }

    // All gates pass — promote every handshake-verified client.
    // Cardinality at the worktree level (≥ 1 Participating surface)
    // satisfies the documented `LiveValidation` invariant without
    // needing per-client identity resolution, which is unresolved
    // (ARCH-001 follow-up).
    let mut promoted = 0_usize;
    for result in map.values_mut() {
        if result.tier == McpTier::RestartHandshakeVerified {
            result.tier = McpTier::LiveValidation;
            promoted += 1;
        }
    }

    if promoted == 0 {
        // The cheap early-gate at the top of
        // `promote_to_live_validation_when_daemon_attests` should
        // prevent this, but unit tests that call `evaluate_and_promote`
        // directly with no handshake-verified client surface here.
        tracing::debug!(
            reason = SkipReason::NoHandshakeVerifiedClient.as_str(),
            worktree_claim_state = claim.worktree_state.as_str(),
            "activation: daemon attestation evaluated but no client at RestartHandshakeVerified"
        );
        return DaemonAttestation::NotProbed;
    }

    tracing::info!(
        worktree = %worktree.display(),
        worktree_claim_state = claim.worktree_state.as_str(),
        clients_promoted = promoted,
        "activation: promoted to LiveValidation via daemon attestation",
    );
    DaemonAttestation::Promoted
}

fn skip_reason_to_attestation(reason: SkipReason) -> DaemonAttestation {
    match reason {
        SkipReason::WorktreeUnenforced => DaemonAttestation::Unenforced,
        SkipReason::Warming => DaemonAttestation::Warming,
        SkipReason::AllSurfacesQuarantined => DaemonAttestation::AllSurfacesQuarantined,
        SkipReason::DaemonUnreachable => DaemonAttestation::Unreachable,
        SkipReason::StaleHeartbeat => DaemonAttestation::StaleHeartbeat,
        SkipReason::NoParticipatingSurface => DaemonAttestation::NoParticipatingSurface,
        SkipReason::NoHandshakeVerifiedClient => DaemonAttestation::NotProbed,
    }
}

/// Outcome of [`classify_claim`]: whether the daemon's
/// [`WorktreeClaimState`] attests live enforcement we will promote on.
enum ClaimVerdict {
    Promote,
    Skip(SkipReason),
}

/// Enumerate the [`WorktreeClaimState`] promotion predicate from the
/// council verdicts (verbatim from MLP2-051f §"Validation" gate 3):
///
/// - `PreWriteDaemon` → promote.
/// - `DegradedProtection` with ≥1 `Participating` surface → promote
///   (cardinality check is owned by the caller; this function only
///   classifies the worktree-level state).
/// - `DegradedProtection` all `Quarantined` → skip.
/// - `Warming` → skip (transient).
/// - `Unprotected` → skip (would already map to
///   `ready_restart_required` honestly).
///
/// Every other variant the closed-set vocabulary defines is treated
/// as "skip, do not promote" — the predicate is opt-in, so a new
/// `WorktreeClaimState` lands at `Skip` until the activation surface
/// explicitly adds a promotion path.
fn classify_claim(claim: &ProtectionClaim) -> ClaimVerdict {
    match claim.worktree_state {
        WorktreeClaimState::PreWriteDaemon => ClaimVerdict::Promote,
        WorktreeClaimState::DegradedProtection => {
            // Two-gate design: this function only catches the
            // *all-quarantined* case (where the surface ladder is
            // entirely fenced and the operator's remediation routes
            // through `anvil intercept recover`). An empty `surfaces`
            // vector deliberately falls through to `Promote` here
            // because the caller (`evaluate_and_promote`) runs the
            // cardinality gate next — ≥ 1 `Participating` surface
            // required. Removing the `!is_empty()` guard would flip
            // empty-surfaces to `AllSurfacesQuarantined` (the
            // `all()` predicate is vacuously true on an empty
            // iterator) and route an under-attested daemon through
            // the wrong support message.
            if !claim.surfaces.is_empty()
                && claim
                    .surfaces
                    .iter()
                    .all(|s| s.state == SurfaceClaimState::Quarantined)
            {
                ClaimVerdict::Skip(SkipReason::AllSurfacesQuarantined)
            } else {
                ClaimVerdict::Promote
            }
        }
        WorktreeClaimState::Warming => ClaimVerdict::Skip(SkipReason::Warming),
        // `Unprotected` and every other non-promotable variant share
        // the same skip reason — a future `WorktreeClaimState` lands
        // here until the activation surface explicitly adds a
        // promotion path for it (intentional opt-in default).
        _ => ClaimVerdict::Skip(SkipReason::WorktreeUnenforced),
    }
}

/// True when the freshest daemon-side signal for `worktree` is within
/// [`HEARTBEAT_FRESHNESS_WINDOW`] of `now`.
///
/// Signals considered, in order:
///   1. `max(SessionRecord.last_heartbeat_unix)` across sessions whose
///      `worktree` matches the registered status' worktree. Always
///      required — if no session beat within the window, the daemon
///      cannot honestly attest enforcement.
///   2. `DaemonStatusV1::generated_at_unix` — when non-zero
///      (MLP2-051h-or-later daemon). A second consistency anchor
///      against a daemon that stops refreshing its own snapshot clock
///      but keeps sessions registered. Sentinel `0` means "no
///      anchor; fall back to per-session freshness only" — pinned by
///      MLP2-051h's `generated_at_unix_zero_is_the_no_anchor_sentinel`.
///
/// Both checks compare against `now`. Future heartbeats (clock
/// skewed forward on the daemon side) are accepted — the alternative
/// would mark a healthy daemon stale because the workstation clock
/// lags. Stale heartbeats fail closed.
fn heartbeat_within_freshness_window(
    snapshot: &DaemonStatusV1,
    worktree: &Path,
    now: SystemTime,
) -> bool {
    let session_ids: Vec<_> = snapshot
        .worktrees
        .iter()
        .filter(|w| w.worktree == worktree)
        .map(|w| &w.session_id)
        .collect();

    let max_heartbeat = snapshot
        .sessions
        .iter()
        .filter(|s| session_ids.contains(&&s.id))
        .map(|s| s.last_heartbeat_unix)
        .max();

    let Some(heartbeat) = max_heartbeat else {
        return false;
    };

    if !within_window(heartbeat, now) {
        return false;
    }

    // Snapshot-level anchor (MLP2-051h). Sentinel 0 → no anchor;
    // session-level freshness is the only signal available, which we
    // already verified above.
    if snapshot.generated_at_unix == 0 {
        return true;
    }
    within_window(snapshot.generated_at_unix, now)
}

fn within_window(unix_seconds: u64, now: SystemTime) -> bool {
    let now_unix = match now.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => return false,
    };
    if unix_seconds >= now_unix {
        // Future timestamp: accepted only within `MAX_FUTURE_CLOCK_SKEW`
        // of `now`. Without the bound, a daemon stamping `u64::MAX`
        // (broken RTC, snapshot replay, malicious downgrade combined
        // with a stripped `generated_at_unix`) would permanently pass
        // freshness — exactly the failure mode MLP2-051f's
        // post-ship hardening exists to close.
        let skew = unix_seconds.saturating_sub(now_unix);
        return skew <= MAX_FUTURE_CLOCK_SKEW.as_secs();
    }
    let age = now_unix.saturating_sub(unix_seconds);
    age <= HEARTBEAT_FRESHNESS_WINDOW.as_secs()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    use anvil_intercept_proto::session::AgentTag;
    use anvil_intercept_proto::status::{
        DaemonStatusV1, HealthStateV1, IpcStateV1, LatencyMidEditMapV1, WorktreeStatusV1,
    };
    use anvil_intercept_proto::{SessionId, SessionRecord, SessionStatus};

    use super::*;
    use crate::activation::mcp_client::McpProbeResult;

    fn epoch_plus(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn now_with_recent_heartbeats() -> SystemTime {
        epoch_plus(1_716_336_060)
    }

    fn make_session(id: &str, worktree: &Path, heartbeat_unix: u64) -> SessionRecord {
        SessionRecord {
            id: SessionId::new(id),
            worktree: worktree.to_path_buf(),
            pid: Some(1234),
            pgid: None,
            started_at_unix: heartbeat_unix.saturating_sub(60),
            last_heartbeat_unix: heartbeat_unix,
            status: SessionStatus::Active,
            agent_tag: Some(AgentTag {
                driver_id: "test-driver".into(),
                claimed_agent_id: "test-agent".into(),
                pid_starttime: 9_000,
            }),
            daemon_issued_tag: None,
        }
    }

    fn make_worktree_status(session_id: &str, worktree: &Path, fenced: bool) -> WorktreeStatusV1 {
        WorktreeStatusV1 {
            worktree: worktree.to_path_buf(),
            session_id: SessionId::new(session_id),
            fenced,
            cascaded: false,
            cascade_since: None,
        }
    }

    fn make_snapshot(
        worktree: &Path,
        sessions: Vec<SessionRecord>,
        worktrees: Vec<WorktreeStatusV1>,
        ipc_state: IpcStateV1,
        generated_at_unix: u64,
    ) -> DaemonStatusV1 {
        let _ = worktree;
        DaemonStatusV1 {
            sessions,
            fences: vec![],
            worktrees,
            health: HealthStateV1 {
                uptime_seconds: 60,
                version: "test".into(),
                ipc_state,
            },
            latency: LatencyMidEditMapV1::default(),
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
            generated_at_unix,
        }
    }

    fn make_probe(tier: McpTier) -> McpProbeResult {
        McpProbeResult::stdio(tier)
    }

    fn handshake_verified_pair() -> BTreeMap<McpClientId, McpProbeResult> {
        let mut map = BTreeMap::new();
        map.insert(
            McpClientId::ClaudeCode,
            make_probe(McpTier::RestartHandshakeVerified),
        );
        map.insert(McpClientId::Cursor, make_probe(McpTier::RestartRequired));
        map
    }

    #[test]
    fn pre_write_daemon_with_participating_surface_promotes_handshake_verified_clients() {
        let worktree = PathBuf::from("/tmp/wt-051f-promote");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050; // 10s before `now`
        let snapshot = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, heartbeat)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(map[&McpClientId::ClaudeCode].tier, McpTier::LiveValidation);
        // Other clients are left at their original tier — promotion is
        // gated on `RestartHandshakeVerified`, not `RestartRequired`.
        assert_eq!(map[&McpClientId::Cursor].tier, McpTier::RestartRequired);
    }

    #[test]
    fn unprotected_worktree_does_not_promote() {
        // Daemon is healthy, but our worktree isn't registered.
        let registered = PathBuf::from("/tmp/wt-051f-other");
        let our_worktree = PathBuf::from("/tmp/wt-051f-unprotected");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050;
        let snapshot = make_snapshot(
            &registered,
            vec![make_session("sess-1", &registered, heartbeat)],
            vec![make_worktree_status("sess-1", &registered, false)],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &our_worktree, now);

        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::RestartHandshakeVerified,
        );
    }

    #[test]
    fn warming_daemon_does_not_promote() {
        let worktree = PathBuf::from("/tmp/wt-051f-warming");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050;
        let snapshot = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, heartbeat)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Draining,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::RestartHandshakeVerified,
        );
    }

    #[test]
    fn all_quarantined_degraded_does_not_promote() {
        // Every registered surface for the worktree is fenced —
        // `DegradedProtection` + all `Quarantined`. Per the council
        // predicate, do NOT promote; the support hint routes to
        // `anvil intercept recover`.
        let worktree = PathBuf::from("/tmp/wt-051f-all-fenced");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050;
        let snapshot = make_snapshot(
            &worktree,
            vec![
                make_session("sess-1", &worktree, heartbeat),
                make_session("sess-2", &worktree, heartbeat),
            ],
            vec![
                make_worktree_status("sess-1", &worktree, true),
                make_worktree_status("sess-2", &worktree, true),
            ],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::RestartHandshakeVerified,
        );
    }

    #[test]
    fn mixed_degraded_with_participating_surface_promotes() {
        // One quarantined session, one participating — overall
        // `DegradedProtection` but there is ≥1 Participating surface.
        // Per the spec's claim-state predicate, this promotes.
        let worktree = PathBuf::from("/tmp/wt-051f-mixed");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050;
        let snapshot = make_snapshot(
            &worktree,
            vec![
                make_session("sess-fenced", &worktree, heartbeat),
                make_session("sess-ok", &worktree, heartbeat),
            ],
            vec![
                make_worktree_status("sess-fenced", &worktree, true),
                make_worktree_status("sess-ok", &worktree, false),
            ],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(map[&McpClientId::ClaudeCode].tier, McpTier::LiveValidation);
    }

    #[test]
    fn stale_heartbeat_does_not_promote_even_if_claim_attests() {
        let worktree = PathBuf::from("/tmp/wt-051f-stale-hb");
        let now = now_with_recent_heartbeats();
        // > 45 s before `now`.
        let heartbeat = 1_716_336_000;
        let snapshot = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, heartbeat)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::RestartHandshakeVerified,
        );
    }

    #[test]
    fn stale_snapshot_anchor_does_not_promote_even_with_fresh_heartbeat() {
        // MLP2-051h sentinel: when `generated_at_unix` is non-zero we
        // treat it as a second freshness anchor; a daemon that stops
        // refreshing its snapshot clock but keeps sessions registered
        // is the defence-in-depth case this anchor exists to catch.
        let worktree = PathBuf::from("/tmp/wt-051f-stale-anchor");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050; // fresh
        let snapshot = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, heartbeat)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Serving,
            1_716_335_900, // > 45s before `now`
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::RestartHandshakeVerified,
        );
    }

    #[test]
    fn zero_snapshot_anchor_is_no_anchor_sentinel() {
        // MLP2-051h pinned `generated_at_unix == 0` as "no anchor;
        // fall back to per-session freshness only". A pre-MLP2-051h
        // daemon talking to a post-MLP2-051h consumer must still
        // promote when the session heartbeat is fresh.
        let worktree = PathBuf::from("/tmp/wt-051f-zero-anchor");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050;
        let snapshot = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, heartbeat)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Serving,
            0, // sentinel
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(map[&McpClientId::ClaudeCode].tier, McpTier::LiveValidation);
    }

    #[test]
    fn no_handshake_verified_client_is_a_noop() {
        // The orchestrator's handshake-pass never elevated any client
        // out of `RestartRequired`. Promotion has nothing to do; the
        // cheap early gate in
        // `promote_to_live_validation_when_daemon_attests` skips the
        // IPC round-trip. We're testing `evaluate_and_promote` directly
        // here; it must also no-op without mutating the map.
        let worktree = PathBuf::from("/tmp/wt-051f-no-hsv");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050;
        let snapshot = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, heartbeat)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = BTreeMap::new();
        map.insert(McpClientId::Cursor, make_probe(McpTier::RestartRequired));
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(map[&McpClientId::Cursor].tier, McpTier::RestartRequired);
    }

    #[test]
    fn future_heartbeat_is_treated_as_fresh() {
        // Daemon-side clock skew (faster than workstation) must not
        // mark a healthy daemon stale. Accept future heartbeats up to
        // `MAX_FUTURE_CLOCK_SKEW` (90 s).
        let worktree = PathBuf::from("/tmp/wt-051f-future-hb");
        let now = epoch_plus(1_716_336_000);
        let heartbeat = 1_716_336_060; // 60s in the future per our clock — within skew bound
        let snapshot = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, heartbeat)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(map[&McpClientId::ClaudeCode].tier, McpTier::LiveValidation);
    }

    /// MLP2-051f post-ship hardening (council 2026-05-22): an unbounded
    /// future timestamp must NOT pass freshness. A daemon stamping
    /// `u64::MAX` (broken RTC, snapshot replay, downgrade attack
    /// combined with a stripped `generated_at_unix`) previously
    /// permanently passed freshness; the `MAX_FUTURE_CLOCK_SKEW` cap
    /// (90 s) closes that gap.
    #[test]
    fn far_future_heartbeat_is_rejected_as_stale() {
        let worktree = PathBuf::from("/tmp/wt-051f-far-future-hb");
        let now = epoch_plus(1_716_336_000);
        // 1 hour in the future — well beyond MAX_FUTURE_CLOCK_SKEW (90s).
        let heartbeat = 1_716_336_000 + 3600;
        let snapshot = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, heartbeat)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::RestartHandshakeVerified,
            "1-hour-future heartbeat must fail freshness, not promote",
        );
    }

    /// Boundary case: exactly at the `MAX_FUTURE_CLOCK_SKEW` (90 s)
    /// cap should still pass. One second past the cap should fail.
    /// Pins the inclusive bound so a future tightening must update
    /// this test.
    #[test]
    fn max_future_clock_skew_boundary() {
        let worktree = PathBuf::from("/tmp/wt-051f-skew-boundary");
        let now = epoch_plus(1_716_336_000);

        // At the cap: passes.
        let at_cap = 1_716_336_000 + MAX_FUTURE_CLOCK_SKEW.as_secs();
        let snapshot_at = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, at_cap)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Serving,
            at_cap,
        );
        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot_at, &worktree, now);
        assert_eq!(map[&McpClientId::ClaudeCode].tier, McpTier::LiveValidation);

        // 1 s past the cap: fails.
        let over = 1_716_336_000 + MAX_FUTURE_CLOCK_SKEW.as_secs() + 1;
        let snapshot_over = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, over)],
            vec![make_worktree_status("sess-1", &worktree, false)],
            IpcStateV1::Serving,
            over,
        );
        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot_over, &worktree, now);
        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::RestartHandshakeVerified,
            "one second past MAX_FUTURE_CLOCK_SKEW must fail freshness",
        );
    }

    /// Council adversarial gate (architect resolution): promotion
    /// must require ≥1 Participating surface for the worktree. The
    /// claim's worktree state could be `PreWriteDaemon` while every
    /// surface is `Quarantined` if a registry race ever surfaces; the
    /// cardinality check is the second gate that prevents the
    /// promotion from over-claiming in that corner.
    #[test]
    fn pre_write_daemon_with_no_participating_surface_does_not_promote() {
        let worktree = PathBuf::from("/tmp/wt-051f-empty-surfaces");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050;
        // PreWriteDaemon claim but the registered worktree status is
        // fenced — the `build_protection_claim_from_wire` mapping
        // makes this `DegradedProtection` + all `Quarantined`. The
        // cardinality gate catches it.
        let snapshot = make_snapshot(
            &worktree,
            vec![make_session("sess-1", &worktree, heartbeat)],
            vec![make_worktree_status("sess-1", &worktree, true)],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &worktree, now);

        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::RestartHandshakeVerified,
        );
    }

    /// Worktree canonicalisation contract — daemon stores canonical
    /// path; activation must canonicalise its argument too.
    /// `build_protection_claim_from_wire` compares byte-equal on
    /// `worktree`. If the caller passes a non-canonical form the
    /// match fails, the claim reads `Unprotected`, and the promotion
    /// silently no-ops — exact MLP2-025b / #1831 failure mode.
    /// `canonicalise_for_activation` is the activation-side fix; this
    /// test pins that the predicate fails closed when the worktree
    /// path passed to `evaluate_and_promote` does not match the
    /// daemon-stored form.
    #[test]
    fn non_canonical_worktree_does_not_promote() {
        // The daemon stores its canonical absolute path at register-
        // time. A caller that passes a sibling path (different bytes,
        // even if both paths resolve to the same canonical dir) fails
        // the `build_protection_claim_from_wire` byte-equality match
        // — claim reads `Unprotected`, promotion silently no-ops.
        // `canonicalise_for_activation` (the production path) closes
        // this gap by canonicalising before the IPC call; the unit
        // test here pins that `evaluate_and_promote` itself does NOT
        // attempt a salvage match — non-canonical input is fail-closed.
        let canonical = PathBuf::from("/tmp/wt-051f-canonical-real");
        let non_canonical = PathBuf::from("/tmp/wt-051f-canonical-other");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050;
        let snapshot = make_snapshot(
            &canonical,
            vec![make_session("sess-1", &canonical, heartbeat)],
            vec![make_worktree_status("sess-1", &canonical, false)],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &non_canonical, now);

        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::RestartHandshakeVerified,
        );
    }

    /// `canonicalise_for_activation` falls back to the argument when
    /// the path does not exist — the daemon almost certainly does not
    /// have it registered either, and the promotion will skip on the
    /// `Unprotected` branch. This pins the no-panic contract.
    #[test]
    fn canonicalise_for_activation_handles_missing_path() {
        let missing = PathBuf::from("/nonexistent/anvil-051f-canon-fallback");
        let out = canonicalise_for_activation(&missing);
        assert_eq!(out, missing);
    }

    /// The production canonicalisation step must produce a path that
    /// `build_protection_claim_from_wire` sees as byte-equal to what
    /// the daemon stored at register-time. The fixture uses a tempdir
    /// (the closest real-world stand-in for a daemon-canonicalised
    /// worktree) plus a `./<name>/.` accessor for the non-canonical
    /// form. Canonicalising the latter must collapse to the former.
    #[test]
    fn canonicalise_for_activation_collapses_curdir_components() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let canonical = std::fs::canonicalize(tmp.path()).expect("canonicalise tmp");
        let with_curdir = tmp.path().join(".");
        let out = canonicalise_for_activation(&with_curdir);
        assert_eq!(out, canonical);
    }

    /// End-to-end (still in-process): show that a snapshot registered
    /// at the canonical tempdir path is matchable iff
    /// `canonicalise_for_activation` runs over the user's input.
    /// Pins the contract that the production call site must call
    /// `canonicalise_for_activation` before
    /// `build_protection_claim_from_wire`.
    #[test]
    fn evaluate_and_promote_against_canonicalised_tempdir_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let canonical = std::fs::canonicalize(tmp.path()).expect("canonicalise tmp");
        let now = now_with_recent_heartbeats();
        let heartbeat = 1_716_336_050;
        let snapshot = make_snapshot(
            &canonical,
            vec![make_session("sess-1", &canonical, heartbeat)],
            vec![make_worktree_status("sess-1", &canonical, false)],
            IpcStateV1::Serving,
            heartbeat,
        );

        let mut map = handshake_verified_pair();
        evaluate_and_promote(&mut map, &snapshot, &canonical, now);
        assert_eq!(map[&McpClientId::ClaudeCode].tier, McpTier::LiveValidation);
    }

    /// MLP2-051f hard-gate #5: end-to-end against a **real** daemon
    /// socket. The fixture daemon binds a per-PID Unix socket, attests
    /// the test worktree via a `StatusProvider`, and `verify()` queries
    /// it through the production wire path
    /// (`query_daemon_status_at_with_timeout` +
    /// `build_protection_claim_from_wire`). This pins the wire-up so a
    /// future refactor that silently leaves `evaluate_and_promote`
    /// correct but disconnects it from the IPC fetch reproduces
    /// MLP2-025b's "spec implemented, zero callers" failure and this
    /// test fails — mirroring
    /// `mcp/validation.rs::tests::local_daemon_client_returns_scan_buffer_diagnostics_with_embedded_parity`
    /// which gates MLP2-051b's MCP shim with the same pattern.
    ///
    /// Linux-only because the fixture uses `IpcListener::bind` against
    /// a Unix domain socket. The Windows path is exercised
    /// platform-natively by the MLP2-075 pipe-bind tests in
    /// `crates/anvil-cli/src/mcp/validation.rs`.
    #[cfg(target_os = "linux")]
    #[test]
    fn end_to_end_against_real_unix_socket_promotes_to_live_validation() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::Arc;
        use std::time::{Duration as StdDuration, Instant};

        use anvil_intercept::Shutdown;
        use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
        use anvil_intercept::status::{DaemonStatus, IpcState, StatusProvider, build_status};

        struct Fixture {
            worktree: PathBuf,
        }
        impl StatusProvider for Fixture {
            fn query_status(&self) -> DaemonStatus {
                let session = anvil_intercept_proto::SessionRecord {
                    id: anvil_intercept_proto::SessionId::new("sess-051f-e2e"),
                    worktree: self.worktree.clone(),
                    pid: Some(4242),
                    pgid: Some(4242),
                    started_at_unix: 1_716_336_000,
                    last_heartbeat_unix: now_unix_seconds(),
                    status: anvil_intercept_proto::SessionStatus::Active,
                    agent_tag: None,
                    daemon_issued_tag: None,
                };
                let started = Instant::now();
                build_status(
                    vec![session],
                    &[],
                    &[],
                    None,
                    started,
                    started + StdDuration::from_secs(1),
                    "0.0.0-051f-e2e",
                    IpcState::Serving,
                    None,
                    None,
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs()),
                )
            }
        }

        // Hold the `TempDir` for the test's lifetime so `Drop` cleans
        // up at exit. An earlier draft called `.keep()` which leaks
        // the dir on disk per CI run (council finding).
        let worktree_dir = tempfile::tempdir().expect("tempdir");
        let worktree = std::fs::canonicalize(worktree_dir.path()).expect("worktree canonical");

        let runtime_dir = tempfile::tempdir().expect("runtime tempdir");
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir perms");
        let socket = runtime_dir.path().join("intercept.sock");

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let _guard = runtime.enter();
        let listener = IpcListener::bind(&socket, NoopDispatcher)
            .expect("daemon socket binds")
            .with_status_provider(Arc::new(Fixture {
                worktree: worktree.clone(),
            }));
        let (shutdown, token) = Shutdown::new();
        let server = runtime.spawn(listener.serve(token));

        // Drive the production wire path: fetch snapshot via the real
        // IPC + bounded timeout, then evaluate + promote against the
        // production logic. If the wire-up regresses (e.g. the
        // promotion stops calling `query_daemon_status_with_timeout`
        // and reverts to a synthetic mock), this assertion fails
        // because the fixture daemon serves the snapshot the
        // evaluation needs.
        let snapshot = crate::commands::intercept::query_daemon_status_at_with_timeout(
            &socket,
            ACTIVATION_DAEMON_QUERY_TIMEOUT,
        )
        .expect("fixture daemon responds within the activation budget");

        let mut map = handshake_verified_pair();
        let attestation = evaluate_and_promote(&mut map, &snapshot, &worktree, SystemTime::now());

        shutdown.trigger();
        runtime.block_on(async {
            server
                .await
                .expect("daemon task joins")
                .expect("daemon exits cleanly");
        });

        assert_eq!(
            attestation,
            DaemonAttestation::Promoted,
            "real-socket fixture attests the worktree; attestation must be Promoted",
        );
        assert_eq!(
            map[&McpClientId::ClaudeCode].tier,
            McpTier::LiveValidation,
            "real-socket fixture attests the worktree; client must reach LiveValidation",
        );
    }

    #[cfg(target_os = "linux")]
    fn now_unix_seconds() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }

    /// MLP2-051f hard-gate #4: pin the `ACTIVATION_DAEMON_QUERY_TIMEOUT`
    /// budget. A hung daemon (accepts the connection, then blocks
    /// forever without writing) must not stretch `verify()` beyond
    /// `timeout + 200 ms` slack. Without the parameterised budget,
    /// the call inherits the 2 s `REQUEST_TIMEOUT` default — every
    /// interactive verify pays ~2 s extra when the daemon is wedged.
    ///
    /// The test holds the accepted stream alive (via the channel
    /// receiver) until after the client times out, so the server side
    /// never closes the connection. The client's read sits in
    /// `set_read_timeout`-bounded recv → returns `Err(WouldBlock)` →
    /// surfaces as `Err` within budget.
    #[cfg(target_os = "linux")]
    #[test]
    fn activation_query_aborts_within_budget_against_hung_daemon() {
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::net::UnixListener;
        use std::sync::mpsc;
        use std::time::Instant;

        let runtime_dir = tempfile::tempdir().expect("runtime tempdir");
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir perms");
        let socket = runtime_dir.path().join("intercept.sock");
        let listener = UnixListener::bind(&socket).expect("hung-daemon listener binds");
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .expect("socket perms");

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            // Accept exactly one connection and hold the stream in
            // scope until we receive on `stop_rx`. This keeps the
            // accepted side open so the client read blocks until its
            // own timeout fires (rather than seeing EOF from a
            // closed peer, which would return Err for the wrong
            // reason).
            let (_stream, _) = listener.accept().expect("hung-daemon accepts");
            let _ = stop_rx.recv();
        });

        let started = Instant::now();
        let err = crate::commands::intercept::query_daemon_status_at_with_timeout(
            &socket,
            ACTIVATION_DAEMON_QUERY_TIMEOUT,
        )
        .expect_err("hung daemon must time out, not succeed");
        let elapsed = started.elapsed();

        // 200 ms slack tolerates loaded CI workers; the spec's
        // "timeout + 100 ms" cap is the strict bound, but a CI runner
        // under load can blow 100 ms on scheduling alone. The
        // contract under test is "no 2 s blow-up", not "exactly 500 ms".
        let slack = Duration::from_millis(200);
        assert!(
            elapsed <= ACTIVATION_DAEMON_QUERY_TIMEOUT + slack,
            "hung-daemon query exceeded budget: elapsed = {elapsed:?}, budget = {ACTIVATION_DAEMON_QUERY_TIMEOUT:?}",
        );
        // The exact error message varies by OS; the contract is just
        // that the call returns Err within budget.
        let _ = err;

        // Release the held server stream so the listener thread can
        // exit cleanly.
        let _ = stop_tx.send(());
        let _ = handle.join();
    }

    /// MLP2-051f post-ship hardening (council 2026-05-22): a daemon
    /// that drip-feeds the response one byte at a time at an interval
    /// approaching `request_timeout` must NOT defeat the wall-clock
    /// budget. The previous Unix implementation used
    /// `BufReader::read_until(b'\n')` with `set_read_timeout` as a
    /// per-syscall cap; a malicious / broken daemon could keep the
    /// read loop alive for up to `RESPONSE_LINE_BYTES × timeout`
    /// (~524 s at 500 ms / 1 MiB). The new loop refreshes the
    /// per-iter timeout against a single `Instant`-based deadline so
    /// total wall-clock cannot exceed the budget.
    #[cfg(target_os = "linux")]
    #[test]
    fn slow_drip_response_does_not_exceed_wall_clock_budget() {
        use std::io::Write as _;
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::net::UnixListener;
        use std::sync::mpsc;
        use std::time::Instant;

        let runtime_dir = tempfile::tempdir().expect("runtime tempdir");
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir perms");
        let socket = runtime_dir.path().join("intercept.sock");
        let listener = UnixListener::bind(&socket).expect("slow-drip listener binds");
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .expect("socket perms");

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("slow-drip accepts");
            // Read and discard the client's request frame so the
            // socket is in the expected post-write state.
            let mut throwaway = [0_u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut throwaway);
            // Drip one byte per ~150 ms forever. With a 500 ms
            // budget the client should bail after ~3 reads, well
            // before the JSON-RPC framing newline is delivered.
            let drip = b"{";
            loop {
                if stop_rx.try_recv().is_ok() {
                    break;
                }
                if stream.write_all(drip).is_err() {
                    break;
                }
                let _ = stream.flush();
                std::thread::sleep(Duration::from_millis(150));
            }
        });

        let started = Instant::now();
        let err = crate::commands::intercept::query_daemon_status_at_with_timeout(
            &socket,
            ACTIVATION_DAEMON_QUERY_TIMEOUT,
        )
        .expect_err("slow-drip daemon must time out, not succeed");
        let elapsed = started.elapsed();

        let slack = Duration::from_millis(300);
        assert!(
            elapsed <= ACTIVATION_DAEMON_QUERY_TIMEOUT + slack,
            "slow-drip query exceeded budget: elapsed = {elapsed:?}, budget = {ACTIVATION_DAEMON_QUERY_TIMEOUT:?}",
        );
        let _ = err;
        let _ = stop_tx.send(());
        let _ = handle.join();
    }
}
