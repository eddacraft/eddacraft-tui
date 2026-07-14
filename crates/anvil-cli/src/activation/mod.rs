//! Shared activation vocabulary.
//!
//! This module is the load-bearing contract for the wow-start activation
//! flow. Every surface that renders activation status — `anvil start`,
//! `anvil status`, `anvil doctor`, the tutorial — reads the same
//! [`ProtectionState`] vocabulary and the same [`ActivationDiagnostic`]
//! shape so user-facing copy can never claim pre-write protection unless
//! the diagnostic literally backs it.
//!
//! The state model and verification path together implement
//! `LAUNCH-008` and `LAUNCH-012` from
//! `plans/modules/launch-flow-readiness.aps.md`. Surfaces that probe
//! deeper layers (MCP attachment, daemon liveness, watch process
//! identity) populate the diagnostic with extra evidence; the state
//! mapping in [`ActivationDiagnostic::protection_state`] derives the
//! single literal vocabulary word users see.

pub mod agent_registry;
pub mod baseline;
pub(crate) mod daemon_evidence;
pub mod detect_agents;
pub mod diagnostic;
pub mod identity;
pub mod language_profile;
pub mod mcp_client;
pub mod orchestrator;
pub mod render;
pub mod state;

// Re-exports kept narrow to the surface the binary currently consumes
// (status.rs). Each downstream PR (LAUNCH-006/-009/-010/-011) is
// expected to extend this list as it wires in further consumers —
// `ProtectionState`, `McpClientId`, `McpTier`, `WatchTier`, and
// `ConfigStatus` remain accessible as `activation::{diagnostic,state}::…`
// until then.
pub use diagnostic::{ActivationDiagnostic, verify};
#[allow(unused_imports)] // contract surface for downstream PRs
pub use language_profile::{CoverageTier, LanguageProfileEntry, RepoLanguageProfile, profile_repo};
pub use render::{
    has_repair_hint, headline_for_diagnostic, render_human, render_human_verbose,
    render_human_with_install, render_json, repair_hint_for,
};
