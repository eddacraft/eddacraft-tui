//! MLP-009 protection-claim wire types (spec §14).
//!
//! Closed-set enums and JSON shape for `status` / MCP / doctor. Unknown states
//! fail at deserialise. Optional fields may extend within
//! [`PROTECTION_CLAIM_SCHEMA_VERSION`]; breaking changes bump the major.
//! Conformance: `crates/anvil-cli/tests/protection_claim_states.rs`.

use serde::{Deserialize, Serialize};

/// Schema version pinned for the JSON wire shape. Forward-compat
/// rule: additions of optional fields ride this version; semantically
/// breaking changes (state-name renames, field-type changes, removed
/// states) bump the major component.
pub const PROTECTION_CLAIM_SCHEMA_VERSION: &str = "anvil.protection-claim.v1";

/// Per-worktree protection-claim state from spec §14.2. Ten closed-
/// set variants. Tooling treats unknown variants as a hard error at
/// deserialise time (no silent fallthrough to a `default`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorktreeClaimState {
    /// No daemon, no embedded fallback; `ensure()` failed.
    Unprotected,
    /// Daemon up but `ready: false`, OR no surfaces attached yet.
    Warming,
    /// MCP shims active; all on `embedded` backend.
    PreWriteEmbedded,
    /// MCP shims active; ≥1 daemon-backed.
    PreWriteDaemon,
    /// Editor driver Participating; no MCP.
    SaveTimeOnly,
    /// ≥1 daemon-backed MCP + ≥1 Participating editor driver.
    Full,
    /// Above states with ≥1 surface degraded.
    DegradedProtection,
    /// Multiple surfaces detected on different `os_locality_token`s.
    CrossBoundaryMixed,
    /// Two `info.json` records observed.
    MultiDaemonDetected,
    /// Daemon canonicalisation differs from registered path.
    PathUncertain,
}

impl WorktreeClaimState {
    /// Canonical wire string for this state. Pinned by tests so
    /// renaming a variant in Rust forces an explicit string update
    /// and a schema version review.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unprotected => "unprotected",
            Self::Warming => "warming",
            Self::PreWriteEmbedded => "pre-write-embedded",
            Self::PreWriteDaemon => "pre-write-daemon",
            Self::SaveTimeOnly => "save-time-only",
            Self::Full => "full",
            Self::DegradedProtection => "degraded-protection",
            Self::CrossBoundaryMixed => "cross-boundary-mixed",
            Self::MultiDaemonDetected => "multi-daemon-detected",
            Self::PathUncertain => "path-uncertain",
        }
    }

    /// Every variant in declaration order. Drives contract tests so
    /// adding a new variant without updating the test surface is a
    /// compile-time discovery (the `match` exhaustiveness check in
    /// [`Self::as_str`]) plus a runtime test failure (this slice
    /// length disagrees with the constant pin in the test module).
    #[must_use]
    pub const fn all() -> &'static [WorktreeClaimState] {
        &[
            WorktreeClaimState::Unprotected,
            WorktreeClaimState::Warming,
            WorktreeClaimState::PreWriteEmbedded,
            WorktreeClaimState::PreWriteDaemon,
            WorktreeClaimState::SaveTimeOnly,
            WorktreeClaimState::Full,
            WorktreeClaimState::DegradedProtection,
            WorktreeClaimState::CrossBoundaryMixed,
            WorktreeClaimState::MultiDaemonDetected,
            WorktreeClaimState::PathUncertain,
        ]
    }
}

/// Per-surface protection-claim state from spec §14.1. Eight closed-
/// set variants. Surfaces include MCP shims, editor drivers, and
/// future amplifiers; the daemon assigns one of these to every
/// surface in its registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceClaimState {
    /// Surface known but not yet registered with the daemon.
    Unbound,
    /// Registered, handshake complete, but not actively participating
    /// in the enforcement pipeline.
    Attached,
    /// Surface is contributing to enforcement decisions.
    Participating,
    /// Daemon unreachable; surface running an embedded fallback.
    EmbeddedFallback,
    /// Surface is up but missing one or more capabilities for full
    /// protection (e.g., rule pack mismatch).
    Degraded,
    /// Surface refused at registration time because it lives across
    /// an `os_locality_token` boundary.
    CrossBoundaryRefused,
    /// Surface is fenced — outputs are not honoured until an
    /// explicit unblock.
    Quarantined,
    /// Surface previously attached but is no longer responding;
    /// pending eviction.
    Detached,
}

impl SurfaceClaimState {
    /// Canonical wire string for this state.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unbound => "unbound",
            Self::Attached => "attached",
            Self::Participating => "participating",
            Self::EmbeddedFallback => "embedded-fallback",
            Self::Degraded => "degraded",
            Self::CrossBoundaryRefused => "cross-boundary-refused",
            Self::Quarantined => "quarantined",
            Self::Detached => "detached",
        }
    }

    /// Every variant in declaration order.
    #[must_use]
    pub const fn all() -> &'static [SurfaceClaimState] {
        &[
            SurfaceClaimState::Unbound,
            SurfaceClaimState::Attached,
            SurfaceClaimState::Participating,
            SurfaceClaimState::EmbeddedFallback,
            SurfaceClaimState::Degraded,
            SurfaceClaimState::CrossBoundaryRefused,
            SurfaceClaimState::Quarantined,
            SurfaceClaimState::Detached,
        ]
    }
}

/// Single surface's claim entry — identifier plus state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceClaim {
    /// Driver / surface identifier. Opaque to this contract; the
    /// daemon decides naming.
    pub identifier: String,
    /// One of the eight §14.1 states.
    pub state: SurfaceClaimState,
}

/// Aggregate protection claim for a worktree.
///
/// This is the wire shape `anvil status --json` / the MCP response
/// surface / `anvil doctor` will all emit. Tooling deserialises this
/// instead of pattern-matching strings.
///
/// Deserialisation enforces the `schema_version` invariant at the
/// type boundary: a wire payload with any value other than
/// [`PROTECTION_CLAIM_SCHEMA_VERSION`] is rejected with a serde
/// error. Consumers never see a `ProtectionClaim` instance carrying
/// a future / unknown major version, so the docstring rule
/// "consumers MUST refuse claims with an unknown / future major
/// version" is structurally guaranteed rather than just documented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ProtectionClaimRaw")]
pub struct ProtectionClaim {
    /// Schema version pinned to [`PROTECTION_CLAIM_SCHEMA_VERSION`].
    /// The deserialise path refuses any other value; consumers
    /// holding a `ProtectionClaim` can rely on this being the
    /// current major.
    pub schema_version: String,
    /// One of the ten §14.2 states.
    pub worktree_state: WorktreeClaimState,
    /// All surfaces the daemon knows about for this worktree. Empty
    /// when `worktree_state` is [`WorktreeClaimState::Unprotected`]
    /// or [`WorktreeClaimState::Warming`] without attachments yet.
    pub surfaces: Vec<SurfaceClaim>,
}

impl ProtectionClaim {
    /// Build a claim at the pinned schema version. Mostly used by
    /// tests and the (deferred) status-render path.
    #[must_use]
    pub fn new(worktree_state: WorktreeClaimState, surfaces: Vec<SurfaceClaim>) -> Self {
        Self {
            schema_version: PROTECTION_CLAIM_SCHEMA_VERSION.to_string(),
            worktree_state,
            surfaces,
        }
    }
}

/// Wire-shape intermediate used by [`ProtectionClaim`]'s
/// `#[serde(try_from = ...)]` so the schema-version check runs at
/// deserialise time. Not part of the public API — consumers always
/// see the validated [`ProtectionClaim`] type.
#[derive(Deserialize)]
struct ProtectionClaimRaw {
    schema_version: String,
    worktree_state: WorktreeClaimState,
    surfaces: Vec<SurfaceClaim>,
}

impl TryFrom<ProtectionClaimRaw> for ProtectionClaim {
    type Error = String;

    fn try_from(raw: ProtectionClaimRaw) -> Result<Self, Self::Error> {
        if raw.schema_version != PROTECTION_CLAIM_SCHEMA_VERSION {
            return Err(format!(
                "unknown protection-claim schema_version: {:?} (expected {:?})",
                raw.schema_version, PROTECTION_CLAIM_SCHEMA_VERSION,
            ));
        }
        Ok(Self {
            schema_version: raw.schema_version,
            worktree_state: raw.worktree_state,
            surfaces: raw.surfaces,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pinned count of per-worktree states. If a variant is added,
    /// this test fails so the conformance fixture and the
    /// per-state mapping table are updated together.
    #[test]
    fn worktree_state_count_matches_spec() {
        assert_eq!(
            WorktreeClaimState::all().len(),
            10,
            "spec §14.2 names ten per-worktree states",
        );
    }

    /// Pinned count of per-surface states from spec §14.1.
    #[test]
    fn surface_state_count_matches_spec() {
        assert_eq!(
            SurfaceClaimState::all().len(),
            8,
            "spec §14.1 names eight per-surface states",
        );
    }

    /// Per-worktree state → canonical wire string. Adding or
    /// renaming a state breaks this map first, forcing an explicit
    /// schema-version review.
    #[test]
    fn worktree_state_canonical_strings_match_spec() {
        let expected: &[(WorktreeClaimState, &str)] = &[
            (WorktreeClaimState::Unprotected, "unprotected"),
            (WorktreeClaimState::Warming, "warming"),
            (WorktreeClaimState::PreWriteEmbedded, "pre-write-embedded"),
            (WorktreeClaimState::PreWriteDaemon, "pre-write-daemon"),
            (WorktreeClaimState::SaveTimeOnly, "save-time-only"),
            (WorktreeClaimState::Full, "full"),
            (
                WorktreeClaimState::DegradedProtection,
                "degraded-protection",
            ),
            (
                WorktreeClaimState::CrossBoundaryMixed,
                "cross-boundary-mixed",
            ),
            (
                WorktreeClaimState::MultiDaemonDetected,
                "multi-daemon-detected",
            ),
            (WorktreeClaimState::PathUncertain, "path-uncertain"),
        ];
        for (variant, canonical) in expected {
            assert_eq!(
                variant.as_str(),
                *canonical,
                "WorktreeClaimState::{variant:?} as_str()"
            );
            let serialized = serde_json::to_string(variant).expect("serialise");
            assert_eq!(
                serialized,
                format!("\"{canonical}\""),
                "WorktreeClaimState::{variant:?} JSON",
            );
        }
    }

    /// Per-surface state → canonical wire string.
    #[test]
    fn surface_state_canonical_strings_match_spec() {
        let expected: &[(SurfaceClaimState, &str)] = &[
            (SurfaceClaimState::Unbound, "unbound"),
            (SurfaceClaimState::Attached, "attached"),
            (SurfaceClaimState::Participating, "participating"),
            (SurfaceClaimState::EmbeddedFallback, "embedded-fallback"),
            (SurfaceClaimState::Degraded, "degraded"),
            (
                SurfaceClaimState::CrossBoundaryRefused,
                "cross-boundary-refused",
            ),
            (SurfaceClaimState::Quarantined, "quarantined"),
            (SurfaceClaimState::Detached, "detached"),
        ];
        for (variant, canonical) in expected {
            assert_eq!(
                variant.as_str(),
                *canonical,
                "SurfaceClaimState::{variant:?} as_str()"
            );
            let serialized = serde_json::to_string(variant).expect("serialise");
            assert_eq!(
                serialized,
                format!("\"{canonical}\""),
                "SurfaceClaimState::{variant:?} JSON",
            );
        }
    }

    /// Spec §14.2: "`pre-write-embedded` ≠ `pre-write-daemon` —
    /// tooling MUST treat them distinct". Pin so a future helper
    /// that collapses them surfaces in review.
    #[test]
    fn pre_write_embedded_distinct_from_pre_write_daemon() {
        assert_ne!(
            WorktreeClaimState::PreWriteEmbedded,
            WorktreeClaimState::PreWriteDaemon,
            "spec §14.2 pin",
        );
        assert_ne!(
            WorktreeClaimState::PreWriteEmbedded.as_str(),
            WorktreeClaimState::PreWriteDaemon.as_str(),
            "wire strings must also disagree",
        );
    }

    /// Every variant is distinct via `PartialEq` — defends against
    /// an accidental duplicate variant introduced during refactoring.
    #[test]
    fn worktree_states_are_pairwise_distinct() {
        let all = WorktreeClaimState::all();
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "WorktreeClaimState variants {i} and {j} collide");
                    assert_ne!(
                        a.as_str(),
                        b.as_str(),
                        "WorktreeClaimState wire strings {i} and {j} collide",
                    );
                }
            }
        }
    }

    /// Same pairwise-distinct invariant for the surface enum.
    #[test]
    fn surface_states_are_pairwise_distinct() {
        let all = SurfaceClaimState::all();
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "SurfaceClaimState variants {i} and {j} collide");
                    assert_ne!(
                        a.as_str(),
                        b.as_str(),
                        "SurfaceClaimState wire strings {i} and {j} collide",
                    );
                }
            }
        }
    }

    /// Schema version constant is pinned. The text matches the
    /// project-wide `anvil.<dotted-name>.vN` convention used by
    /// `anvil.diagnostic.v1`, `anvil.audit-chain.v1`, etc.
    #[test]
    fn schema_version_constant_is_pinned() {
        assert_eq!(PROTECTION_CLAIM_SCHEMA_VERSION, "anvil.protection-claim.v1");
    }

    /// `ProtectionClaim` round-trips through JSON with all fields
    /// preserved. Pinned wire shape: `schema_version` /
    /// `worktree_state` / `surfaces`.
    #[test]
    fn protection_claim_round_trips_through_json() {
        let claim = ProtectionClaim::new(
            WorktreeClaimState::Full,
            vec![
                SurfaceClaim {
                    identifier: "mcp-shim-claude".into(),
                    state: SurfaceClaimState::Participating,
                },
                SurfaceClaim {
                    identifier: "editor-driver-vscode".into(),
                    state: SurfaceClaimState::Attached,
                },
            ],
        );
        let line = serde_json::to_string(&claim).expect("serialise");
        let back: ProtectionClaim = serde_json::from_str(&line).expect("deserialise");
        assert_eq!(back, claim);
    }

    /// JSON wire shape pin: every required field is present and
    /// uses the documented key. Tests as a `serde_json::Value` so
    /// the assertion errors point at field names rather than a
    /// stringly-compared blob.
    #[test]
    fn protection_claim_json_uses_documented_field_names() {
        let claim = ProtectionClaim::new(WorktreeClaimState::Warming, vec![]);
        let value: serde_json::Value = serde_json::to_value(&claim).expect("serialise");
        assert_eq!(value["schema_version"], PROTECTION_CLAIM_SCHEMA_VERSION);
        assert_eq!(value["worktree_state"], "warming");
        assert!(
            value["surfaces"].is_array(),
            "surfaces is an array even when empty",
        );
        assert_eq!(value["surfaces"].as_array().unwrap().len(), 0);
    }

    /// Surfaces field is an array of `{identifier, state}` objects
    /// with documented keys.
    #[test]
    fn surface_claim_json_uses_documented_field_names() {
        let claim = ProtectionClaim::new(
            WorktreeClaimState::SaveTimeOnly,
            vec![SurfaceClaim {
                identifier: "editor-driver-vscode".into(),
                state: SurfaceClaimState::Participating,
            }],
        );
        let value: serde_json::Value = serde_json::to_value(&claim).expect("serialise");
        let surfaces = value["surfaces"].as_array().expect("surfaces array");
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0]["identifier"], "editor-driver-vscode");
        assert_eq!(surfaces[0]["state"], "participating");
    }

    /// Tooling MUST refuse unknown worktree-state strings rather
    /// than silently mapping them to a default. Pinned so the
    /// `#[serde(rename_all = "kebab-case")]` derive's closed-enum
    /// behaviour doesn't accidentally regress into a permissive
    /// `#[serde(other)]` fallback.
    #[test]
    fn unknown_worktree_state_fails_to_deserialise() {
        let result: Result<WorktreeClaimState, _> = serde_json::from_str("\"future-state\"");
        assert!(
            result.is_err(),
            "unknown worktree states must reject at deserialise: {result:?}",
        );
    }

    /// Same closed-enum invariant for the surface enum.
    #[test]
    fn unknown_surface_state_fails_to_deserialise() {
        let result: Result<SurfaceClaimState, _> = serde_json::from_str("\"future-surface\"");
        assert!(
            result.is_err(),
            "unknown surface states must reject at deserialise: {result:?}",
        );
    }

    /// `ProtectionClaim`'s deserialise path enforces the
    /// schema-version invariant at the type boundary. A wire payload
    /// carrying a future / unknown major version is rejected, so
    /// consumers holding an instance can rely on the current major
    /// without checking again. The PR review pinned this gap; this
    /// test pins the structural enforcement.
    #[test]
    fn unknown_schema_version_fails_to_deserialise() {
        let wire = r#"{"schema_version":"anvil.protection-claim.v999","worktree_state":"full","surfaces":[]}"#;
        let result: Result<ProtectionClaim, _> = serde_json::from_str(wire);
        assert!(
            result.is_err(),
            "future schema_version values must reject at deserialise: {result:?}",
        );
        let err_text = result.unwrap_err().to_string();
        assert!(
            err_text.contains("schema_version"),
            "diagnostic should mention schema_version, got: {err_text}",
        );
    }

    /// And empty / malformed `schema_version` is also rejected.
    #[test]
    fn empty_schema_version_fails_to_deserialise() {
        let wire = r#"{"schema_version":"","worktree_state":"full","surfaces":[]}"#;
        let result: Result<ProtectionClaim, _> = serde_json::from_str(wire);
        assert!(
            result.is_err(),
            "empty schema_version must reject at deserialise: {result:?}",
        );
    }

    /// Sanity: the pinned current `schema_version` still round-trips.
    #[test]
    fn pinned_schema_version_round_trips() {
        let claim = ProtectionClaim::new(WorktreeClaimState::Full, vec![]);
        let line = serde_json::to_string(&claim).expect("serialise");
        let back: ProtectionClaim = serde_json::from_str(&line).expect("deserialise round-trip");
        assert_eq!(back.schema_version, PROTECTION_CLAIM_SCHEMA_VERSION);
    }

    /// MLP2-052 — additive-optional-fields forward-compat. A wire
    /// payload carrying an unknown optional top-level field (here
    /// `degraded_reasons`, one of the field names the spec earmarks
    /// for v1.1 addition) deserialises successfully against the
    /// current Rust struct. The unknown field's data is dropped —
    /// it's not part of the type — but the known fields retain
    /// semantic identity. This is the structural pin against a
    /// future regression that adds `#[serde(deny_unknown_fields)]`
    /// to [`ProtectionClaimRaw`].
    #[test]
    fn additive_optional_top_level_field_deserialises_ok() {
        let wire = r#"{
            "schema_version": "anvil.protection-claim.v1",
            "worktree_state": "full",
            "surfaces": [],
            "degraded_reasons": ["surface-drift", "rule-pack-mismatch"]
        }"#;
        let claim: ProtectionClaim = serde_json::from_str(wire)
            .expect("unknown optional top-level field must be silently ignored");
        assert_eq!(claim.schema_version, PROTECTION_CLAIM_SCHEMA_VERSION);
        assert_eq!(claim.worktree_state, WorktreeClaimState::Full);
        assert!(claim.surfaces.is_empty());
    }

    /// The unknown optional field is not re-emitted by serialise —
    /// its data has nowhere to go in the v1 struct, so it drops on
    /// round-trip. Documents the "old consumers parsing a newer
    /// payload" half of the additivity rule.
    #[test]
    fn additive_optional_top_level_field_drops_on_serialise() {
        let wire = r#"{
            "schema_version": "anvil.protection-claim.v1",
            "worktree_state": "warming",
            "surfaces": [],
            "cross_boundary_token": "future-token-abc123"
        }"#;
        let claim: ProtectionClaim = serde_json::from_str(wire).expect("deserialise");
        let re_emitted: serde_json::Value = serde_json::to_value(&claim).expect("serialise back");
        assert!(
            re_emitted.get("cross_boundary_token").is_none(),
            "unknown field must drop on re-serialise (no synthetic emission); got: {re_emitted}",
        );
    }

    /// Same forward-compat rule on the surface-claim entries: an
    /// unknown optional field on a per-surface object deserialises
    /// without error and the known fields keep semantic identity.
    #[test]
    fn additive_optional_field_on_surface_claim_deserialises_ok() {
        let wire = r#"{
            "schema_version": "anvil.protection-claim.v1",
            "worktree_state": "save-time-only",
            "surfaces": [
                {
                    "identifier": "editor-driver-vscode",
                    "state": "participating",
                    "last_evaluated_at": "2026-05-14T12:34:56Z"
                }
            ]
        }"#;
        let claim: ProtectionClaim = serde_json::from_str(wire)
            .expect("unknown optional field on surface must be silently ignored");
        assert_eq!(claim.surfaces.len(), 1);
        assert_eq!(claim.surfaces[0].identifier, "editor-driver-vscode");
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Participating);
    }

    /// Adding an optional field is, by the rule, NOT a breaking
    /// change — so the `schema_version` stays pinned at v1. This is
    /// the "`schema_version` stays `anvil.protection-claim.v1`" half
    /// of the MLP2-052 acceptance criterion: any future PR that adds
    /// an optional field MUST NOT alter [`PROTECTION_CLAIM_SCHEMA_VERSION`].
    #[test]
    fn additive_optional_field_does_not_bump_schema_version() {
        // The pin: the major component is "v1". If a future change
        // bumps to v2 it MUST do so deliberately and document why in
        // the module docstring's additivity rule, not as a drive-by.
        assert_eq!(
            PROTECTION_CLAIM_SCHEMA_VERSION, "anvil.protection-claim.v1",
            "additive-optional changes must not bump the schema_version major",
        );
        // And a fixture carrying an optional field still deserialises
        // at this exact version constant.
        let wire = r#"{
            "schema_version": "anvil.protection-claim.v1",
            "worktree_state": "degraded-protection",
            "surfaces": [],
            "degraded_reasons": ["mock-future-field"]
        }"#;
        let claim: ProtectionClaim = serde_json::from_str(wire).expect("deserialise");
        assert_eq!(claim.schema_version, PROTECTION_CLAIM_SCHEMA_VERSION);
    }

    /// Combined: the new field appears on both the envelope and a
    /// surface entry in the same payload. Pins that the two
    /// silently-ignore paths compose without interaction.
    #[test]
    fn additive_optional_fields_compose_across_envelope_and_surface() {
        let wire = r#"{
            "schema_version": "anvil.protection-claim.v1",
            "worktree_state": "full",
            "surfaces": [
                {
                    "identifier": "mcp-shim-claude",
                    "state": "participating",
                    "rule_pack_sha": "abc123"
                }
            ],
            "degraded_reasons": []
        }"#;
        let claim: ProtectionClaim = serde_json::from_str(wire).expect("deserialise");
        assert_eq!(claim.worktree_state, WorktreeClaimState::Full);
        assert_eq!(claim.surfaces.len(), 1);
        assert_eq!(claim.surfaces[0].state, SurfaceClaimState::Participating);
    }
}
