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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_id_variants_distinct() {
        assert_ne!(EngineId::Rust, EngineId::Legacy);
    }

    #[test]
    fn engine_id_copy_semantics() {
        let a = EngineId::Rust;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn engine_id_serde_round_trip() {
        for id in [EngineId::Rust, EngineId::Legacy] {
            let json = serde_json::to_string(&id).unwrap();
            let back: EngineId = serde_json::from_str(&json).unwrap();
            assert_eq!(id, back);
        }
    }

    #[test]
    fn engine_id_debug_format() {
        assert_eq!(format!("{:?}", EngineId::Rust), "Rust");
        assert_eq!(format!("{:?}", EngineId::Legacy), "Legacy");
    }

    #[test]
    fn engine_id_invalid_variant_fails() {
        let result = serde_json::from_str::<EngineId>("\"V8\"");
        assert!(result.is_err());
    }

    // Verify re-exports are accessible
    #[test]
    fn re_exports_accessible() {
        let _: EventType = EventType::Progress;
        let _: ErrorCode = ErrorCode::Internal;
        let _: SymbolKind = SymbolKind::Function;
        let _: Visibility = Visibility::Public;
        let _: EdgeType = EdgeType::Calls;
        let _: TrustLevel = TrustLevel::default();
    }
}
