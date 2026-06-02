pub mod embedded;
pub mod engine_mode;
pub mod feature_flags;
// The graph state + algorithms were extracted to `anvil-graph-cache` (ADR-064)
// so the intercept daemon can depend on them without the parser surface.
// Re-exported as a module alias (not item re-exports) to preserve submodule
// paths like `anvil_kernel::graph::incremental::GraphDelta`.
pub use anvil_graph_cache as graph;
pub mod parser;
pub mod policy;
pub mod protocol;
pub mod watch;
pub mod watcher;
