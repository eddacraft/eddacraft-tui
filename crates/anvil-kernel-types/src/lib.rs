pub mod diagnostics;
mod events;
pub mod feature_flags;
pub mod feature_flags_catalogue;
mod graph;
pub mod hooks;
mod notifications;
pub mod protection_claim;
mod trust;
pub mod watch_event;

pub use diagnostics::{
    Category, DIAGNOSTIC_SCHEMA_VERSION, Diagnostic, DiagnosticSource, Location, Mode, Severity,
};
pub use events::{EngineEvent, ErrorCode, ErrorPayload, EventPayload, EventType};
pub use feature_flags::{
    AudienceContext, Channel, ConditionValue, EnvironmentContext, EnvironmentName,
    EvaluationContext, FEATURE_FLAG_SCHEMA_VERSION, FeatureFlagDefinition, FeatureFlagManifest,
    FlagClass, FlagStatus, FlagValue, FlagValueType, FlagVariant, TargetingCondition,
    TargetingOperator, TargetingRule,
};
pub use graph::{
    EdgeType, FileSymbols, ImportEdge, SymbolEdge, SymbolIdentity, SymbolKind, SymbolNode,
    Visibility,
};
pub use hooks::{ANVIL_CONFIG_HOOK_PATTERN, is_anvil_managed_command};
pub use notifications::{
    Notification, NotificationClass, NotificationContext, NotificationPriority,
};
pub use protection_claim::{
    PROTECTION_CLAIM_SCHEMA_VERSION, ProtectionClaim, SurfaceClaim, SurfaceClaimState,
    WorktreeClaimState,
};
pub use trust::TrustLevel;
pub use watch_event::{
    WATCH_EVENT_SCHEMA_VERSION, WatchEventEnvelope, WatchEventPayload, WatchEventType,
};

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
