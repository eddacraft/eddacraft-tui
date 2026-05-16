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
    /// MLP2-058: current entry count of the daemon's resolved-rule-set
    /// cache. `None` (wire: absent) when the daemon has not surfaced a
    /// cache (embedded mode, or a pre-MLP2-058 daemon talking to a
    /// post-MLP2-058 consumer). Consumers MUST treat absent and
    /// `null` as the same — "no cache observability available".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_entries: Option<u32>,
    /// MLP2-058: cumulative `.anvil.*` rule-set cache invalidations
    /// since the daemon started. Rate of change is the operator
    /// signal — a steady non-zero rate indicates an attacker or
    /// runaway writer (MLP2-059 caps it). `None` when no cache is
    /// wired (see [`Self::cache_entries`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_invalidations_total: Option<u64>,
    /// MLP2-058: evaluations currently holding a `scan_buffer`
    /// permit. `None` when no scan-buffer service is wired (embedded
    /// mode). The daemon caps concurrent evaluations at 8 today so
    /// `u8` is the natural-fit width; future scaling either tightens
    /// the cap or bumps the type in v2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_flight_evaluations: Option<u8>,
    /// MLP2-059: cumulative `.anvil.*` rule-set cache invalidations
    /// coalesced by the per-worktree rate limiter since the daemon
    /// started. A steady non-zero rate of change indicates an
    /// attacker (or runaway writer) driving `.anvil.*` writes faster
    /// than the burst window admits; the cache stays warm, the
    /// counter records the storm. `None` when no cache is wired
    /// (embedded mode, or a pre-MLP2-059 daemon talking to a newer
    /// consumer — additive-optional, byte-compat for older shapes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_invalidations_rate_limited: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeStatusV1 {
    pub worktree: PathBuf,
    pub session_id: SessionId,
    pub fenced: bool,
    /// MLP2-026: `true` when the worktree is in
    /// `degraded:fence-cascade` mode. Always present on the wire
    /// (`#[serde(default)]`) so operators reading status snapshots
    /// see `cascaded: false` explicitly, not absence. See spec §3.6.
    #[serde(default)]
    pub cascaded: bool,
    /// MLP2-026: Unix seconds at which the cascade was engaged.
    /// Wire-additive via `skip_serializing_if` to keep the common
    /// case (None) compact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cascade_since: Option<u64>,
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
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
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
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
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

    /// MLP2-058: a pre-MLP2-058 daemon (no `cache_entries` /
    /// `cache_invalidations_total` / `in_flight_evaluations` keys on
    /// the wire) round-trips into the post-MLP2-058 `DaemonStatusV1`
    /// with the three new fields collapsed to `None`. Pins the
    /// "older daemon → newer consumer" direction of the additive
    /// contract.
    #[test]
    fn pre_mlp2_058_payload_round_trips_with_new_fields_absent() {
        let json = serde_json::json!({
            "sessions": [],
            "worktrees": [],
            "fences": [],
            "health": {
                "uptime_seconds": 12,
                "version": "0.6.0",
                "ipc_state": "serving",
            },
            "latency": { "mid_edit": null },
        });
        let parsed: DaemonStatusV1 = serde_json::from_value(json).expect("deserialise");
        assert_eq!(parsed.cache_entries, None);
        assert_eq!(parsed.cache_invalidations_total, None);
        assert_eq!(parsed.in_flight_evaluations, None);
        // MLP2-059 introduced a fourth additive-optional field; the
        // pre-MLP2-058 payload above has no key for it, so the
        // parsed value must collapse to None.
        assert_eq!(parsed.cache_invalidations_rate_limited, None);
    }

    /// MLP2-059: a post-MLP2-058 / pre-MLP2-059 daemon (no
    /// `cache_invalidations_rate_limited` key on the wire) round-
    /// trips into the post-MLP2-059 `DaemonStatusV1` with the new
    /// field collapsed to `None`. Pins the same additive contract for
    /// the rate-limited counter as MLP2-058 did for the cache trio.
    #[test]
    fn pre_mlp2_059_payload_round_trips_with_rate_limit_field_absent() {
        let json = serde_json::json!({
            "sessions": [],
            "worktrees": [],
            "fences": [],
            "health": {
                "uptime_seconds": 12,
                "version": "0.6.0",
                "ipc_state": "serving",
            },
            "latency": { "mid_edit": null },
            "cache_entries": 7,
            "cache_invalidations_total": 0,
            "in_flight_evaluations": 0,
        });
        let parsed: DaemonStatusV1 = serde_json::from_value(json).expect("deserialise");
        assert_eq!(parsed.cache_entries, Some(7));
        assert_eq!(parsed.cache_invalidations_total, Some(0));
        assert_eq!(parsed.in_flight_evaluations, Some(0));
        assert_eq!(
            parsed.cache_invalidations_rate_limited, None,
            "pre-MLP2-059 daemon must surface the new field as None"
        );
    }

    /// MLP2-058: when the new fields are present on the wire they
    /// arrive as their typed values. Pins the on-wire shape so a
    /// downstream parser is byte-compatible.
    #[test]
    fn cache_and_in_flight_fields_round_trip_when_present() {
        let status = DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 1,
                version: "0.6.0".to_owned(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 { mid_edit: None },
            cache_entries: Some(17),
            cache_invalidations_total: Some(42),
            in_flight_evaluations: Some(3),
            cache_invalidations_rate_limited: Some(5),
        };
        let line = serde_json::to_string(&status).expect("serialise");
        let back: DaemonStatusV1 = serde_json::from_str(&line).expect("deserialise");
        assert_eq!(back, status);

        let json: serde_json::Value = serde_json::to_value(&status).expect("to_value");
        assert_eq!(json["cache_entries"], 17);
        assert_eq!(json["cache_invalidations_total"], 42);
        assert_eq!(json["in_flight_evaluations"], 3);
        assert_eq!(json["cache_invalidations_rate_limited"], 5);
    }

    /// MLP2-058: `None` on the new fields wires as absent, not `null`
    /// or `0`. Operators (and tests) need to distinguish "no cache
    /// observed" from "cache observed at zero entries"; the absent
    /// encoding preserves that distinction across producer/consumer
    /// pairs.
    #[test]
    fn none_on_new_fields_serialises_to_absent_keys() {
        let status = DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 0,
                version: "0.6.0".to_owned(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 { mid_edit: None },
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
        };
        let json: serde_json::Value = serde_json::to_value(&status).expect("serialise");
        assert!(
            json.get("cache_entries").is_none(),
            "None must wire as absent, not null: {json}",
        );
        assert!(json.get("cache_invalidations_total").is_none());
        assert!(json.get("in_flight_evaluations").is_none());
    }
}
