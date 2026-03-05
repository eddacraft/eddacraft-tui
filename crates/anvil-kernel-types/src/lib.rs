mod events;
mod graph;
mod trust;

pub use events::{EngineEvent, ErrorCode, ErrorPayload, EventPayload, EventType};
pub use graph::{EdgeType, SymbolEdge, SymbolKind, SymbolNode, Visibility};
pub use trust::TrustLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EngineId {
    Rust,
    Legacy,
}
