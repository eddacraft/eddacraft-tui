//! INTL-002: daemon connectivity + fence preflight.
//!
//! Two daemon states make the launcher refuse to spawn:
//!
//! 1. The daemon is not reachable on the per-user IPC rendezvous —
//!    socket file is missing on Unix, the named pipe has not been
//!    bound on Windows, or peer credentials fail.
//! 2. The target worktree is fenced. The daemon owns the fence
//!    store; this code only consults the snapshot returned by
//!    `query_status`.
//!
//! The preflight is the cheapest possible signal to gate spawn on,
//! so we keep it deliberately small: a `query_status` round-trip plus
//! a single membership check against `status.fences`. Any richer
//! capability negotiation belongs in INTD / DRVR territory.

use std::path::{Path, PathBuf};

use anvil_intercept_proto::status::DaemonStatusV1;
use anyhow::Result;
use serde_json::Value;

use crate::ipc;

/// Outcome of the preflight check. The launcher dispatches on this
/// to either proceed with registration or render a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightDecision {
    /// All clear — proceed to session registration.
    Proceed,
    /// The worktree is fenced. The reason string mirrors what the
    /// daemon recorded in its fence store.
    Fenced { worktree: PathBuf, reason: String },
}

/// JSON-RPC method we ask the daemon. The daemon dual-routes both
/// names; the legacy form keeps continuity with the existing
/// `anvil intercept status` client. New consumers (this is one)
/// prefer the namespaced `anvil/status/query`, but the legacy name
/// is what the integration test fixtures speak today, so we keep
/// it here.
const QUERY_STATUS_METHOD: &str = "query_status";
const QUERY_STATUS_REQUEST_ID: &str = "anvil-run-preflight";

/// Run the preflight: pull the daemon status snapshot and compute the
/// decision for `worktree`. Returns an error only when the daemon
/// itself is unreachable / misbehaving — a `Fenced` outcome is a
/// successful decision, not an error.
pub fn run(worktree: &Path) -> Result<PreflightDecision> {
    let status: DaemonStatusV1 =
        ipc::request(QUERY_STATUS_METHOD, &Value::Null, QUERY_STATUS_REQUEST_ID)?;
    Ok(decision_for(&status, worktree))
}

/// Pure decision helper — exposed so tests can pin the gating logic
/// without a live daemon. Kept narrow on purpose: anything more
/// sophisticated (e.g. capability checks) belongs on the daemon
/// side.
#[must_use]
pub fn decision_for(status: &DaemonStatusV1, worktree: &Path) -> PreflightDecision {
    if let Some(fence) = status
        .fences
        .iter()
        .find(|f| paths_equal(&f.worktree, worktree))
    {
        return PreflightDecision::Fenced {
            worktree: fence.worktree.clone(),
            reason: fence.reason.clone(),
        };
    }
    PreflightDecision::Proceed
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    // The daemon and launcher both canonicalise before recording,
    // but the fence store is plain-text JSON so trailing
    // separators can drift on Windows. Compare components instead
    // of raw bytes to be robust to that.
    a.components().eq(b.components())
        || a == b
        || canonical_components(a) == canonical_components(b)
}

fn canonical_components(p: &Path) -> Vec<std::path::Component<'_>> {
    p.components().collect()
}

/// Convenience: wrap a [`ClientError::DaemonNotRunning`] into an
/// actionable launcher message. Pulled out so the `run` orchestrator
/// can format the same wording on both transports.
pub fn refusal_message_for(err: &anyhow::Error) -> String {
    if let Some(client_err) = err.downcast_ref::<ipc::ClientError>() {
        match client_err {
            ipc::ClientError::DaemonNotRunning { path } => format!(
                "the anvil intercept daemon is not running (no rendezvous at {}).\n\
                 Start it with `anvil intercept start --foreground`.",
                path.display()
            ),
            ipc::ClientError::DaemonRefused { reason } => {
                format!("the anvil intercept daemon refused the launcher's connection: {reason}")
            }
            other => format!("daemon preflight failed: {other}"),
        }
    } else {
        format!("daemon preflight failed: {err}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_intercept_proto::status::{
        FenceStateV1, HealthStateV1, IpcStateV1, LatencyMidEditMapV1,
    };

    fn empty_status() -> DaemonStatusV1 {
        DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 1,
                version: "test".into(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 { mid_edit: None },
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
            generated_at_unix: 0,
        }
    }

    #[test]
    fn proceeds_when_no_fences_recorded() {
        let status = empty_status();
        let decision = decision_for(&status, Path::new("/tmp/wt"));
        assert_eq!(decision, PreflightDecision::Proceed);
    }

    #[test]
    fn refuses_when_target_worktree_is_in_the_fence_list() {
        let mut status = empty_status();
        status.fences.push(FenceStateV1 {
            worktree: PathBuf::from("/tmp/wt"),
            reason: "rule violation".into(),
            fenced_at_unix: 0,
        });
        let decision = decision_for(&status, Path::new("/tmp/wt"));
        assert_eq!(
            decision,
            PreflightDecision::Fenced {
                worktree: PathBuf::from("/tmp/wt"),
                reason: "rule violation".into(),
            },
        );
    }

    #[test]
    fn fence_for_a_different_worktree_does_not_block_this_launch() {
        let mut status = empty_status();
        status.fences.push(FenceStateV1 {
            worktree: PathBuf::from("/tmp/other"),
            reason: "manual review".into(),
            fenced_at_unix: 0,
        });
        let decision = decision_for(&status, Path::new("/tmp/wt"));
        assert_eq!(decision, PreflightDecision::Proceed);
    }

    #[test]
    fn refusal_message_for_daemon_not_running_is_actionable() {
        let err = anyhow::Error::new(ipc::ClientError::DaemonNotRunning {
            path: PathBuf::from("/run/user/1000/anvil/intercept.sock"),
        });
        let msg = refusal_message_for(&err);
        assert!(
            msg.contains("not running") && msg.contains("anvil intercept start"),
            "operator-facing wording must name both the failure and the fix: {msg}",
        );
    }

    #[test]
    fn refusal_message_falls_back_for_non_client_errors() {
        let err = anyhow::anyhow!("some other error");
        let msg = refusal_message_for(&err);
        assert!(msg.starts_with("daemon preflight failed"));
    }
}
