//! MLP-009 contract conformance: every state in spec §14 round-trips
//! through the closed-set wire shape.
//!
//! The vocabulary itself lives in `crates/anvil-kernel-types/src/
//! protection_claim.rs`. This test surface is the cross-crate gate
//! that downstream surfaces (anvil-cli status command, MCP shim,
//! doctor) will eventually conform to. It asserts:
//!
//! - Every §14.2 worktree state and §14.1 surface state can be
//!   reached, serialised, and deserialised back to the same variant.
//! - The aggregate `ProtectionClaim` envelope round-trips with each
//!   worktree state as the headline.
//! - Distinctness invariants (§14.2 `pre-write-embedded ≠
//!   pre-write-daemon`) hold at the contract boundary.
//!
//! Per the MLP-009 APS entry, this is the **hard release gate** —
//! no MLP item moves to Complete in `plans/index.aps.md` until the
//! reachability checks here are green for every variant.

use anvil_kernel_types::protection_claim::{
    PROTECTION_CLAIM_SCHEMA_VERSION, ProtectionClaim, SurfaceClaim, SurfaceClaimState,
    WorktreeClaimState,
};

/// Every §14.2 state is reachable and survives a JSON round-trip
/// without losing its identity.
#[test]
fn every_worktree_state_round_trips_through_json() {
    for state in WorktreeClaimState::all() {
        let claim = ProtectionClaim::new(*state, vec![]);
        let line = serde_json::to_string(&claim).expect("serialise");
        let back: ProtectionClaim = serde_json::from_str(&line).expect("deserialise round-trip");
        assert_eq!(
            back.worktree_state, *state,
            "worktree state {state:?} did not round-trip",
        );
        assert_eq!(
            back.schema_version, PROTECTION_CLAIM_SCHEMA_VERSION,
            "schema_version preserved across round-trip for {state:?}",
        );
    }
}

/// Every §14.1 state is reachable and survives a JSON round-trip
/// inside the `surfaces` array.
#[test]
fn every_surface_state_round_trips_through_json() {
    for state in SurfaceClaimState::all() {
        let claim = ProtectionClaim::new(
            WorktreeClaimState::Full,
            vec![SurfaceClaim {
                identifier: format!("contract-test-{}", state.as_str()),
                state: *state,
            }],
        );
        let line = serde_json::to_string(&claim).expect("serialise");
        let back: ProtectionClaim = serde_json::from_str(&line).expect("deserialise round-trip");
        let surfaces = back.surfaces;
        assert_eq!(surfaces.len(), 1, "surface state {state:?}");
        assert_eq!(
            surfaces[0].state, *state,
            "surface state {state:?} did not round-trip",
        );
    }
}

/// Headline shape: `anvil status --json` consumers parse the JSON
/// against documented field names. This is the contract a future
/// `anvil status --json` consumer compiles against.
#[test]
fn protection_claim_documented_field_names() {
    let claim = ProtectionClaim::new(
        WorktreeClaimState::DegradedProtection,
        vec![SurfaceClaim {
            identifier: "mcp-shim-claude".into(),
            state: SurfaceClaimState::EmbeddedFallback,
        }],
    );
    let value: serde_json::Value = serde_json::to_value(&claim).expect("serialise to value");
    assert_eq!(value["schema_version"], PROTECTION_CLAIM_SCHEMA_VERSION);
    assert_eq!(value["worktree_state"], "degraded-protection");
    let surfaces = value["surfaces"].as_array().expect("surfaces array");
    assert_eq!(surfaces.len(), 1);
    assert_eq!(surfaces[0]["identifier"], "mcp-shim-claude");
    assert_eq!(surfaces[0]["state"], "embedded-fallback");
}

/// Spec §14.2 pin: tooling MUST treat `pre-write-embedded` and
/// `pre-write-daemon` as distinct states. A future helper that
/// collapses them would fail this assertion before any production
/// surface starts mis-reporting.
#[test]
fn pre_write_embedded_distinct_from_pre_write_daemon_at_contract_boundary() {
    let embedded = ProtectionClaim::new(WorktreeClaimState::PreWriteEmbedded, vec![]);
    let daemon = ProtectionClaim::new(WorktreeClaimState::PreWriteDaemon, vec![]);
    let embedded_line = serde_json::to_string(&embedded).expect("serialise");
    let daemon_line = serde_json::to_string(&daemon).expect("serialise");
    assert_ne!(
        embedded_line, daemon_line,
        "wire encodings must disagree at the contract boundary",
    );
    assert!(
        embedded_line.contains("\"pre-write-embedded\""),
        "embedded claim wire string: {embedded_line}",
    );
    assert!(
        daemon_line.contains("\"pre-write-daemon\""),
        "daemon claim wire string: {daemon_line}",
    );
}

/// Unknown worktree-state strings MUST fail to deserialise. The
/// closed-set contract loses meaning if `serde` accepts
/// `"future-state"` as some default. Pin so a future refactor that
/// introduces `#[serde(other)]` falls foul of this gate.
#[test]
fn unknown_worktree_state_rejects_at_contract_boundary() {
    let wire = r#"{"schema_version":"anvil.protection-claim.v1","worktree_state":"future-state","surfaces":[]}"#;
    let result: Result<ProtectionClaim, _> = serde_json::from_str(wire);
    assert!(
        result.is_err(),
        "unknown worktree states must reject at the contract boundary; got Ok({:?})",
        result.ok(),
    );
}

/// Same closed-enum invariant for the surface side.
#[test]
fn unknown_surface_state_rejects_at_contract_boundary() {
    let wire = r#"{"schema_version":"anvil.protection-claim.v1","worktree_state":"full","surfaces":[{"identifier":"x","state":"future-surface"}]}"#;
    let result: Result<ProtectionClaim, _> = serde_json::from_str(wire);
    assert!(
        result.is_err(),
        "unknown surface states must reject at the contract boundary; got Ok({:?})",
        result.ok(),
    );
}

/// Pin the total counts at the contract boundary too. If a variant
/// is added to either enum, this test fails so the contract fixture
/// surface (the deferred `crates/anvil-cli/tests/fixtures/
/// status_v1/` files) is updated alongside.
#[test]
fn closed_set_counts_match_spec() {
    assert_eq!(
        WorktreeClaimState::all().len(),
        10,
        "spec §14.2 names ten per-worktree states",
    );
    assert_eq!(
        SurfaceClaimState::all().len(),
        8,
        "spec §14.1 names eight per-surface states",
    );
}
