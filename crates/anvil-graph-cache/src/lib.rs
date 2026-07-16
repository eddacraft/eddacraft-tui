//! Parser-free semantic graph state and algorithms for the Anvil daemon.
//!
//! Hosts the `SymbolGraph` / `DependencyGraph` pair, the incremental
//! apply-delta logic, and trust annotation — extracted from `anvil-kernel`
//! (ADR-064) so the resident intercept daemon can hold and mutate the graph
//! without depending on the tree-sitter parser surface. The crate consumes only
//! already-parsed `FileSymbols` / `ImportEdge` (from `anvil-kernel-types`); it
//! never parses. `anvil-kernel` re-exports this crate as its `graph` module.

pub mod call_graph;
pub mod certify;
pub mod compose;
pub mod dependency;
pub mod hot_index;
pub mod incremental;
pub mod overlay;
pub mod rebase;
pub mod registry;
pub mod snapshot;
pub mod symbol_graph;
pub mod tokens;
pub mod trust;

pub use call_graph::{CallerResult, CallersReport, MAX_CALLERS_WALK, callers_of, symbol_at_offset};
pub use certify::{
    Certifiability, CertifyStale, ChangeKind, ExportSurfaceDiff, certify,
    clamp_reverse_impact_depth, export_surface_changed, export_surface_diff,
};
pub use compose::compose;
pub use dependency::DependencyGraph;
pub use hot_index::{
    BackgroundReadApi, HotPathSurface, HotRead, HotReadApi, HotReadMiss, MAX_REVERSE_IMPACT_DEPTH,
};
pub use incremental::{
    CallResolution, GraphDelta, re_resolve_calls, re_resolve_calls_tracked, re_resolve_imports,
    re_resolve_imports_tracked, re_resolve_reexports, remove_file, update_file,
};
pub use overlay::{ChangedSet, OverlayCoverage, OverlayFragment, classify_changes};
pub use rebase::{BaseReresolve, ComposePlan, InvalidatedEdge, OverlayIdAllocator, plan_compose};
pub use registry::GraphRegistry;
pub use snapshot::{
    MAX_SNAPSHOT_BYTES, SNAPSHOT_BACKING_SCHEMA_VERSION, SNAPSHOT_BASE_MAGIC,
    SNAPSHOT_FORMAT_VERSION, SNAPSHOT_MAGIC, SnapshotBuildError, SnapshotLoadError,
    SnapshotPayload, is_workspace_root_relative, persist_graph_enabled, snapshot_filename,
};
pub use symbol_graph::{GraphError, GraphStats, SymbolGraph};
pub use tokens::{
    GCTX_TOKEN_ESTIMATOR_VERSION, MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES, TokenEstimate,
    TokenEstimateError, estimate_gctx_tokens,
};
pub use trust::{TrustGraph, TrustPostureChange, annotate_trust, policy_profiles};
