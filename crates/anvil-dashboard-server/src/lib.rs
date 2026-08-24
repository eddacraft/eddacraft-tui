//! Loopback-only read API for the local Anvil dashboard.

#![forbid(unsafe_code)]
#![cfg_attr(
    not(any(unix, windows)),
    allow(unused, reason = "unsupported hosts are rejected below")
)]

#[cfg(not(any(unix, windows)))]
compile_error!("anvil-dashboard-server supports Unix and Windows hosts only");

mod api;
mod assets;
mod capabilities;
mod error;
mod openapi;
mod server;
mod workspace;

pub use api::{
    AffectedFile, AssuranceSummary, AttentionItem, DataGap, DataState, EvidenceLine,
    GateCheckSummary, GateRunSummary, HealthResponse, PatternCatalogue, PatternSummary, PlanDetail,
    PlanSummary, PlanTimelineEntry, ProtectionHistory, ProtectionHistoryPoint,
    ProtectionHistoryRange, ProtectionOverview, SaveTimeSummary, WarningSummary,
};
pub use assets::is_bundled;
pub use capabilities::history::load_protection_history;
pub use capabilities::patterns::load_pattern_catalogue;
pub use capabilities::plans::{
    MAX_PLAN_MODULES, MAX_PLAN_SOURCE_BYTES, MAX_PLAN_WORK_ITEMS, PlanReadError, load_plan,
    load_plans,
};
pub use capabilities::protection::{load_persisted_protection_overview, load_protection_overview};
pub use error::{ApiError, ServerError};
pub use openapi::openapi_document;
pub use server::{dashboard_route_paths, ensure_loopback, serve};
pub use workspace::{MAX_ARTEFACT_BYTES, Workspace, WorkspaceReadError};
