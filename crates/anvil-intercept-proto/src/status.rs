//! INTD-011: wire types for the `query_status` JSON-RPC method.
//!
//! These types are authoritative for `DriverClient` consumers — both
//! the daemon and the launcher / driver speak this shape on the wire.
//! The daemon's in-memory view (`anvil_intercept::status::DaemonStatus`)
//! converts into the wire shape via `to_wire`; consumers parse the
//! wire shape directly.
//!
//! See `plans/modules/intercept-daemon.aps.md` task INTD-011 and
//! `plans/decisions/031-validation-latency-rubric.md` (ADR-031) for
//! the measurement vocabulary.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{SessionId, SessionRecord};

/// Wire-level snapshot of the daemon's state, returned by
/// `query_status`. Field names use `snake_case` for stability with the
/// rest of the JSON-RPC surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonStatusV1 {
    /// Live registered sessions. Empty when no driver has connected.
    pub sessions: Vec<SessionRecord>,
    /// Per-worktree status overlay — one entry per active session,
    /// with the fence flag derived from the persisted fence store.
    pub worktrees: Vec<WorktreeStatusV1>,
    /// Fenced worktrees (one entry per persisted fence).
    pub fences: Vec<FenceStateV1>,
    /// Daemon health summary.
    pub health: HealthStateV1,
    /// Latency rollups, one slot per ADR-031 mode. v1 only exposes
    /// `mid_edit`. Future modes (`save`, `pre_write`, `watch`) MAY
    /// be added as additional fields without breaking existing
    /// consumers.
    pub latency: LatencyMidEditMapV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeStatusV1 {
    pub worktree: PathBuf,
    pub session_id: SessionId,
    pub fenced: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FenceStateV1 {
    pub worktree: PathBuf,
    pub reason: String,
    pub fenced_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStateV1 {
    pub uptime_seconds: u64,
    pub version: String,
    pub ipc_state: IpcStateV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IpcStateV1 {
    /// Listener is bound and serving.
    Serving,
    /// Listener is in shutdown drain.
    Draining,
}

/// Latency rollup map — only `mid_edit` is exposed in v1. Adding new
/// modes is additive: a future driver that does not understand a new
/// field MUST tolerate it (serde does this by default for non-
/// `deny_unknown_fields` structs).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LatencyMidEditMapV1 {
    /// `None` (wire: `null`) when no mid-edit traffic has been
    /// observed yet. Consumers MUST treat null and absent as the
    /// same: "no traffic"; they MUST NOT default to zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid_edit: Option<LatencyRollupV1>,
}

/// Wire shape of the rolled-up p50/p95 percentiles for a single
/// validation-mode + boundary pair. ADR-031 pins the dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LatencyRollupV1 {
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub sample_count: usize,
    pub window_seconds: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_status_round_trips_through_json_with_traffic() {
        let status = DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 3,
                version: "0.5.1-beta".to_owned(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 {
                mid_edit: Some(LatencyRollupV1 {
                    p50_ms: 12.5,
                    p95_ms: 48.2,
                    sample_count: 17,
                    window_seconds: 22.4,
                }),
            },
        };
        let line = serde_json::to_string(&status).expect("serialise");
        let back: DaemonStatusV1 = serde_json::from_str(&line).expect("deserialise");
        assert_eq!(back, status);
    }

    #[test]
    fn no_traffic_serialises_to_null_or_absent() {
        let status = DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 0,
                version: "0.5.1-beta".to_owned(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 { mid_edit: None },
        };
        let json: serde_json::Value = serde_json::to_value(&status).expect("serialise");
        // mid_edit must be either absent or null — never zero.
        let mid_edit = &json["latency"]["mid_edit"];
        assert!(
            mid_edit.is_null() || mid_edit == &serde_json::Value::Null,
            "mid_edit must wire as null when None; got {mid_edit:?}",
        );
    }

    #[test]
    fn ipc_state_is_kebab_case() {
        let line = serde_json::to_string(&IpcStateV1::Serving).expect("serialise");
        assert_eq!(line, "\"serving\"");
        let line = serde_json::to_string(&IpcStateV1::Draining).expect("serialise");
        assert_eq!(line, "\"draining\"");
    }

    /// Adding new fields to `DaemonStatusV1` MUST stay additive — a
    /// driver pinned to today's shape must still be able to parse
    /// tomorrow's status when extra fields appear. Pin the
    /// "tolerates unknown top-level keys" property here so a future
    /// `deny_unknown_fields` change cannot silently break consumers.
    #[test]
    fn deserialise_tolerates_unknown_top_level_fields() {
        let json = serde_json::json!({
            "sessions": [],
            "worktrees": [],
            "fences": [],
            "health": {
                "uptime_seconds": 0,
                "version": "0.5.1-beta",
                "ipc_state": "serving",
            },
            "latency": { "mid_edit": null },
            "future_field_added_in_wave_4": { "anything": 42 },
        });
        let parsed: DaemonStatusV1 =
            serde_json::from_value(json).expect("forward-compat deserialise");
        assert!(parsed.latency.mid_edit.is_none());
    }
}
