use serde::{Deserialize, Serialize};

use super::resolver::ResolutionReason;

// -------------------------------------------------------------------------
// Session telemetry (emitted once at session start)
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagSessionTelemetry {
    pub snapshot_version: u64,
    pub environment: String,
    pub runtime: String,
    pub timestamp: String,
}

pub fn create_session_telemetry(
    snapshot_version: u64,
    environment: &str,
    runtime: &str,
) -> FlagSessionTelemetry {
    FlagSessionTelemetry {
        snapshot_version,
        environment: environment.to_string(),
        runtime: runtime.to_string(),
        timestamp: now_iso(),
    }
}

// -------------------------------------------------------------------------
// Evaluation event (emitted on first use per flag per session)
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagEvaluationEvent {
    pub flag_key: String,
    pub variant: String,
    pub reason: ResolutionReason,
    pub timestamp: String,
}

pub fn create_evaluation_event(
    flag_key: &str,
    variant: &str,
    reason: ResolutionReason,
) -> FlagEvaluationEvent {
    FlagEvaluationEvent {
        flag_key: flag_key.to_string(),
        variant: variant.to_string(),
        reason,
        timestamp: now_iso(),
    }
}

// -------------------------------------------------------------------------
// Override event (emitted when an override is applied)
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverrideSource {
    Emergency,
    Local,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagOverrideEvent {
    pub flag_key: String,
    pub variant: String,
    pub source: OverrideSource,
    pub timestamp: String,
}

pub fn create_override_event(
    flag_key: &str,
    variant: &str,
    source: OverrideSource,
) -> FlagOverrideEvent {
    FlagOverrideEvent {
        flag_key: flag_key.to_string(),
        variant: variant.to_string(),
        source,
        timestamp: now_iso(),
    }
}

// -------------------------------------------------------------------------
// Timestamp helper
// -------------------------------------------------------------------------

fn now_iso() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    super::snapshot::format_epoch_secs(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_telemetry_fields() {
        let tel = create_session_telemetry(42, "production", "rust");
        assert_eq!(tel.snapshot_version, 42);
        assert_eq!(tel.environment, "production");
        assert_eq!(tel.runtime, "rust");
        assert!(tel.timestamp.ends_with('Z'));
    }

    #[test]
    fn session_telemetry_no_pii() {
        let tel = create_session_telemetry(1, "development", "rust");
        let json = serde_json::to_string(&tel).unwrap();
        assert!(!json.contains("email"));
        assert!(!json.contains("userId"));
    }

    #[test]
    fn evaluation_event_fields() {
        let event = create_evaluation_event(
            "cli.licence-gate",
            "enabled",
            ResolutionReason::TargetingMatch,
        );
        assert_eq!(event.flag_key, "cli.licence-gate");
        assert_eq!(event.variant, "enabled");
        assert_eq!(event.reason, ResolutionReason::TargetingMatch);
    }

    #[test]
    fn evaluation_event_no_context() {
        let event = create_evaluation_event("test.flag", "disabled", ResolutionReason::Default);
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("targetingKey"));
        assert!(!json.contains("audience"));
    }

    #[test]
    fn override_event_fields() {
        let event =
            create_override_event("cli.licence-gate", "disabled", OverrideSource::Emergency);
        assert_eq!(event.flag_key, "cli.licence-gate");
        assert_eq!(event.source, OverrideSource::Emergency);
    }

    #[test]
    fn override_source_serde() {
        let json = serde_json::to_string(&OverrideSource::Emergency).unwrap();
        assert_eq!(json, "\"emergency\"");
        let json = serde_json::to_string(&OverrideSource::Local).unwrap();
        assert_eq!(json, "\"local\"");
    }
}
