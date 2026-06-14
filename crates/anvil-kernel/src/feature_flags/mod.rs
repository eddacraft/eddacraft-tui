pub(crate) mod resolver;
mod snapshot;
mod telemetry;

pub use resolver::{
    CapturedResolution, FlagOverrides, ResolutionDetails, ResolutionReason, begin_flag_capture,
    evaluate_percentage, resolve_flag, take_captured_flags,
};

pub use snapshot::{
    FeatureFlagSnapshot, SnapshotConfig, SnapshotError, create_snapshot, is_snapshot_fresh,
    load_snapshot,
};

pub use telemetry::{
    FlagEvaluationEvent, FlagOverrideEvent, FlagSessionTelemetry, OverrideSource,
    create_evaluation_event, create_override_event, create_session_telemetry,
};
