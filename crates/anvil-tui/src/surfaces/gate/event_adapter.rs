use anvil_kernel_types::{EngineEvent, EventPayload};

use super::{GateCheck, GateCheckStatus, GateResult};

/// Convert a set of kernel events (from an embedded run) into a `GateResult`
/// for the gate explorer surface.
pub fn events_to_gate_result(
    events: &[EngineEvent],
    duration_ms: u64,
    timestamp: &str,
) -> GateResult {
    let mut checks = Vec::new();

    for event in events {
        match &event.payload {
            EventPayload::Violation {
                policy_id,
                file,
                symbol: _,
                message,
            } => {
                checks.push(GateCheck {
                    id: policy_id.clone(),
                    name: policy_id.clone(),
                    status: GateCheckStatus::Failed,
                    score: 0.0,
                    message: message.clone(),
                    details: None,
                    file: Some(file.clone()),
                    line: None,
                });
            }
            EventPayload::Error(err) => {
                checks.push(GateCheck {
                    id: format!("error:{:?}", err.code),
                    name: format!("Error ({:?})", err.code),
                    status: GateCheckStatus::Failed,
                    score: 0.0,
                    message: err.message.clone(),
                    details: None,
                    file: err.file.clone(),
                    line: None,
                });
            }
            _ => {}
        }
    }

    let overall_passed = checks.is_empty();
    let total = checks.len();
    let passed_count = checks
        .iter()
        .filter(|c| c.status == GateCheckStatus::Passed)
        .count();

    #[allow(clippy::cast_precision_loss)]
    let score = if total == 0 {
        1.0
    } else {
        passed_count as f64 / total as f64
    };

    GateResult {
        plan_id: "kernel".to_string(),
        overall_passed,
        score,
        checks,
        duration_ms,
        timestamp: timestamp.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{EngineId, ErrorCode, ErrorPayload, EventType};

    fn violation_event(policy_id: &str, file: &str, message: &str) -> EngineEvent {
        EngineEvent {
            event_type: EventType::Violation,
            seq: 0,
            timestamp: "10:00:00".to_string(),
            engine: EngineId::Rust,
            payload: EventPayload::Violation {
                policy_id: policy_id.to_string(),
                file: file.to_string(),
                symbol: "sym".to_string(),
                message: message.to_string(),
            },
        }
    }

    fn progress_event() -> EngineEvent {
        EngineEvent {
            event_type: EventType::Progress,
            seq: 0,
            timestamp: "10:00:00".to_string(),
            engine: EngineId::Rust,
            payload: EventPayload::Progress {
                phase: "scan".to_string(),
                current: 5,
                total: 5,
            },
        }
    }

    fn error_event() -> EngineEvent {
        EngineEvent {
            event_type: EventType::Error,
            seq: 0,
            timestamp: "10:00:00".to_string(),
            engine: EngineId::Rust,
            payload: EventPayload::Error(ErrorPayload {
                code: ErrorCode::ParseError,
                file: Some("bad.ts".to_string()),
                message: "parse failed".to_string(),
                recoverable: true,
            }),
        }
    }

    #[test]
    fn violations_map_to_failed_checks() {
        let events = vec![
            violation_event("cross-layer", "src/domain/user.ts", "bad import"),
            violation_event("privilege-expansion", "src/infra/shell.ts", "privileged"),
        ];

        let result = events_to_gate_result(&events, 500, "2026-03-16T10:00:00Z");

        assert!(!result.overall_passed);
        assert_eq!(result.checks.len(), 2);
        assert!(
            result
                .checks
                .iter()
                .all(|c| c.status == GateCheckStatus::Failed)
        );
        assert_eq!(result.checks[0].file.as_deref(), Some("src/domain/user.ts"));
        assert_eq!(result.checks[1].id, "privilege-expansion");
    }

    #[test]
    fn empty_violations_produce_passing_result() {
        let events = vec![progress_event()];

        let result = events_to_gate_result(&events, 100, "2026-03-16T10:00:00Z");

        assert!(result.overall_passed);
        assert!(result.checks.is_empty());
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn error_events_treated_as_failures() {
        let events = vec![
            progress_event(),
            error_event(),
            violation_event("public-api-expansion", "src/api.ts", "new export"),
        ];

        let result = events_to_gate_result(&events, 200, "2026-03-16T10:00:00Z");

        assert_eq!(result.checks.len(), 2);
        assert!(!result.overall_passed);
        assert_eq!(result.checks[0].id, "error:ParseError");
        assert_eq!(result.checks[0].file.as_deref(), Some("bad.ts"));
        assert_eq!(result.checks[1].id, "public-api-expansion");
    }

    #[test]
    fn duration_and_timestamp_passed_through() {
        let result = events_to_gate_result(&[], 1234, "2026-03-16T12:00:00Z");

        assert_eq!(result.duration_ms, 1234);
        assert_eq!(result.timestamp, "2026-03-16T12:00:00Z");
    }

    #[test]
    fn plan_id_is_kernel() {
        let result = events_to_gate_result(&[], 0, "now");
        assert_eq!(result.plan_id, "kernel");
    }
}
