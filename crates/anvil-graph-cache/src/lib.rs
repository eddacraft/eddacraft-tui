//! Parser-free semantic graph state and algorithms for the Anvil daemon.
//!
//! Hosts the `SymbolGraph` / `DependencyGraph` pair, the incremental
//! apply-delta logic, and trust annotation — extracted from `anvil-kernel`
//! (ADR-064) so the resident intercept daemon can hold and mutate the graph
//! without depending on the tree-sitter parser surface. The crate consumes only
//! already-parsed `FileSymbols` / `ImportEdge` (from `anvil-kernel-types`); it
//! never parses. `anvil-kernel` re-exports this crate as its `graph` module.

pub mod certify;
pub mod dependency;
pub mod hot_index;
pub mod incremental;
pub mod symbol_graph;
pub mod trust;

pub use certify::{
    Certifiability, CertifyStale, ChangeKind, ExportSurfaceDiff, certify,
    clamp_reverse_impact_depth, export_surface_changed, export_surface_diff,
};
pub use dependency::DependencyGraph;
pub use hot_index::{
    BackgroundReadApi, HotPathSurface, HotRead, HotReadApi, HotReadMiss, MAX_REVERSE_IMPACT_DEPTH,
};
pub use incremental::{
    GraphDelta, re_resolve_imports, re_resolve_imports_tracked, remove_file, update_file,
};
pub use symbol_graph::{GraphError, GraphStats, SymbolGraph};
pub use trust::{TrustGraph, TrustPostureChange, annotate_trust, policy_profiles};
