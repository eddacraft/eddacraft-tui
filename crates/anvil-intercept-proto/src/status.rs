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
    /// DSV-044: currently registered telemetry subscribers. `None` means the
    /// daemon did not wire the telemetry broadcaster into status (embedded mode
    /// or an older daemon); `Some(0)` means it is wired and has no subscribers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry_subscriber_count: Option<u32>,
    /// DSV-044 / INTD-016: cumulative telemetry envelopes dropped because a
    /// subscriber channel was full. `None` follows the same availability
    /// semantics as [`Self::telemetry_subscriber_count`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry_dropped_envelopes: Option<u64>,
    /// MLP2-051h: Unix seconds at which the daemon assembled this
    /// snapshot. A daemon-level wall-clock anchor distinct from
    /// `HealthStateV1::uptime_seconds` (monotonic-since-start, no
    /// freshness signal) and from per-session `last_heartbeat_unix`
    /// (per-session, not snapshot-level).
    ///
    /// The activation-side freshness check uses this as a second
    /// consistency anchor: a snapshot whose `generated_at_unix` is
    /// further than the staleness window from `SystemTime::now()` is
    /// treated as unsafe to promote against even if per-session
    /// heartbeats are fresh, defending against a daemon whose clock
    /// has stopped but whose sessions keep refreshing.
    ///
    /// Wire-additive via `#[serde(default)]`: a pre-MLP2-051h daemon
    /// (no key on the wire) deserialises with the field at the `u64`
    /// default of `0`. **`0` is the consumer-side "no snapshot anchor
    /// available" sentinel** — fall back to per-session heartbeat
    /// freshness only (the posture before MLP2-051h existed). The
    /// consumer cannot distinguish "old daemon, omitted the key" from
    /// "new daemon, explicitly stamped 0" once the value reaches this
    /// type — both produce `generated_at_unix == 0`. That collision is
    /// intentional: a post-MLP2-051h `DaemonStatusProvider` always
    /// stamps a live, non-zero value (pinned by
    /// `crates/anvil-intercept/src/status.rs::tests::provider_stamps_non_zero_generated_at_unix`),
    /// so a `0` from any real daemon path means the snapshot has no
    /// trustworthy wall-clock anchor regardless of cause. Consumers
    /// MUST treat `== 0` as the no-anchor sentinel and MUST NOT
    /// implement a `> threshold` freshness check (which would treat a
    /// `NoopStatusProvider` snapshot — emitted with explicit `0` — as
    /// "anchor present, just very old" and pass the gate; that is the
    /// failure mode the MLP2-051h precursor exists to prevent).
    ///
    /// Unlike the MLP2-058/-059 additive-optional counters, the field
    /// is a plain `u64` (not `Option<u64>`) because the consumer's
    /// sentinel-equality contract already collapses both pre-/post-
    /// MLP2-051h producers and the noop producer to the same fallback
    /// branch — wrapping `0` in `Option::None` would encode a
    /// distinction the consumer is contractually forbidden from acting
    /// on. Pinned by the
    /// `pre_mlp2_051h_payload_round_trips_with_generated_at_unix_default_zero`,
    /// `generated_at_unix_round_trips_when_present`, and
    /// `generated_at_unix_serialises_always_when_zero` tests in this
    /// crate plus the `generated_at_unix_zero_is_the_no_anchor_sentinel`
    /// contract test in `anvil-intercept`.
    #[serde(default)]
    pub generated_at_unix: u64,
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
            telemetry_subscriber_count: None,
            telemetry_dropped_envelopes: None,
            generated_at_unix: 0,
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
            telemetry_subscriber_count: None,
            telemetry_dropped_envelopes: None,
            generated_at_unix: 0,
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
        assert_eq!(parsed.telemetry_subscriber_count, None);
        assert_eq!(parsed.telemetry_dropped_envelopes, None);
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
        assert_eq!(parsed.telemetry_subscriber_count, None);
        assert_eq!(parsed.telemetry_dropped_envelopes, None);
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
            telemetry_subscriber_count: Some(2),
            telemetry_dropped_envelopes: Some(8),
            generated_at_unix: 0,
        };
        let line = serde_json::to_string(&status).expect("serialise");
        let back: DaemonStatusV1 = serde_json::from_str(&line).expect("deserialise");
        assert_eq!(back, status);

        let json: serde_json::Value = serde_json::to_value(&status).expect("to_value");
        assert_eq!(json["cache_entries"], 17);
        assert_eq!(json["cache_invalidations_total"], 42);
        assert_eq!(json["in_flight_evaluations"], 3);
        assert_eq!(json["cache_invalidations_rate_limited"], 5);
        assert_eq!(json["telemetry_subscriber_count"], 2);
        assert_eq!(json["telemetry_dropped_envelopes"], 8);
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
            telemetry_subscriber_count: None,
            telemetry_dropped_envelopes: None,
            generated_at_unix: 0,
        };
        let json: serde_json::Value = serde_json::to_value(&status).expect("serialise");
        assert!(
            json.get("cache_entries").is_none(),
            "None must wire as absent, not null: {json}",
        );
        assert!(json.get("cache_invalidations_total").is_none());
        assert!(json.get("in_flight_evaluations").is_none());
        assert!(json.get("telemetry_subscriber_count").is_none());
        assert!(json.get("telemetry_dropped_envelopes").is_none());
    }

    /// MLP2-051h: a pre-MLP2-051h daemon (no `generated_at_unix` key on
    /// the wire) round-trips into the post-MLP2-051h `DaemonStatusV1`
    /// with the new field collapsed to `0`. Unlike the MLP2-058/-059
    /// additive-optional fields, `generated_at_unix` is required-but-
    /// defaulted (`#[serde(default)]`) — a `u64` rather than an
    /// `Option<u64>` — because every snapshot a post-MLP2-051h daemon
    /// emits MUST carry the anchor. Consumers treat `0` as "no
    /// snapshot anchor available" (a pre-MLP2-051h daemon spoke first)
    /// and fall back to per-session heartbeat freshness, which is the
    /// posture today.
    #[test]
    fn pre_mlp2_051h_payload_round_trips_with_generated_at_unix_default_zero() {
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
        assert_eq!(
            parsed.generated_at_unix, 0,
            "pre-MLP2-051h daemon must surface the new field as the u64 default",
        );
    }

    /// MLP2-051h: when the new field is present on the wire it arrives
    /// as its typed value and round-trips byte-equivalently. Pins the
    /// on-wire shape so an activation-side consumer reading
    /// `generated_at_unix` against `SystemTime::now()` sees the value
    /// the daemon stamped, not a default or a re-derivation.
    #[test]
    fn generated_at_unix_round_trips_when_present() {
        let status = DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 1,
                version: "0.7.0-beta".to_owned(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 { mid_edit: None },
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
            telemetry_subscriber_count: None,
            telemetry_dropped_envelopes: None,
            generated_at_unix: 1_716_336_000,
        };
        let line = serde_json::to_string(&status).expect("serialise");
        let back: DaemonStatusV1 = serde_json::from_str(&line).expect("deserialise");
        assert_eq!(back, status);

        let json: serde_json::Value = serde_json::to_value(&status).expect("to_value");
        assert_eq!(json["generated_at_unix"], 1_716_336_000_u64);
    }

    /// MLP2-051h: the field MUST always be present on the wire, even
    /// when its value is `0` — distinct from MLP2-058/-059's
    /// `Option`-typed fields which use `skip_serializing_if`. The
    /// freshness rationale rests on the consumer being able to tell
    /// "snapshot is from a 051h+ daemon and was stamped at unix=N"
    /// apart from "snapshot is from a pre-051h daemon and carried no
    /// anchor". The producer side of that distinction is "always emit
    /// the field"; the consumer side defaults the missing key to `0`.
    #[test]
    fn generated_at_unix_serialises_always_when_zero() {
        let status = DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 0,
                version: "0.7.0-beta".to_owned(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 { mid_edit: None },
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
            telemetry_subscriber_count: None,
            telemetry_dropped_envelopes: None,
            generated_at_unix: 0,
        };
        let json: serde_json::Value = serde_json::to_value(&status).expect("serialise");
        assert!(
            json.get("generated_at_unix").is_some(),
            "generated_at_unix must always be present on the wire (even at 0): {json}",
        );
        assert_eq!(json["generated_at_unix"], 0_u64);
    }
}
