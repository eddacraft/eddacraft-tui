use std::fs;
use std::path::PathBuf;

use anvil_dashboard_server::{
    ApiError, DataGap, DataState, GateRunSummary, HealthResponse, PlanDetail, PlanReadError,
    PlanSummary, PlanTimelineEntry, ProtectionHistory, ProtectionHistoryPoint,
    ProtectionHistoryRange, ProtectionOverview, openapi_document,
};
use anvil_kernel_types::{ProtectionClaim, SurfaceClaim, SurfaceClaimState, WorktreeClaimState};
use axum::body::to_bytes;
use axum::response::IntoResponse;
use serde_json::{Value, json};

#[test]
fn committed_openapi_contract_matches_the_rust_export() {
    let contract = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/dashboard/src/api/generated/openapi.json");
    let committed = fs::read_to_string(contract).expect("committed dashboard OpenAPI contract");
    let committed: serde_json::Value =
        serde_json::from_str(&committed).expect("parse committed dashboard OpenAPI contract");

    assert_eq!(committed, openapi_document());
}

#[test]
fn contract_exposes_protection_and_plan_driver_reads() {
    let document = openapi_document();
    let paths = document["paths"].as_object().expect("OpenAPI paths");

    assert_eq!(
        paths["/api/v1/protection"]["get"]["operationId"],
        "getProtectionOverview"
    );
    assert_eq!(
        paths["/api/v1/protection/history"]["get"]["operationId"],
        "getProtectionHistory"
    );
    assert_eq!(paths["/api/v1/plans"]["get"]["operationId"], "listPlans");
    assert_eq!(paths["/api/v1/plans/{id}"]["get"]["operationId"], "getPlan");
    assert!(paths.values().all(|path| path.get("post").is_none()));
}

fn schema_validator(name: &str) -> jsonschema::Validator {
    fn rewrite_refs(value: &mut Value) {
        match value {
            Value::Object(object) => {
                if let Some(Value::String(reference)) = object.get_mut("$ref")
                    && let Some(schema) = reference.strip_prefix("#/components/schemas/")
                {
                    *reference = format!("#/$defs/{schema}");
                }
                object.values_mut().for_each(rewrite_refs);
            }
            Value::Array(values) => values.iter_mut().for_each(rewrite_refs),
            _ => {}
        }
    }

    let document = openapi_document();
    let mut definitions = document["components"]["schemas"].clone();
    rewrite_refs(&mut definitions);
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": format!("#/$defs/{name}"),
        "$defs": definitions,
    });
    jsonschema::validator_for(&schema).expect("OpenAPI component schema")
}

#[test]
fn serialised_handler_dtos_conform_to_the_committed_schemas() {
    let health = serde_json::to_value(HealthResponse::ready()).expect("health DTO");
    assert!(schema_validator("HealthResponse").is_valid(&health));

    let mut protection = ProtectionOverview::unavailable("canonical gate summary only");
    protection.claim = Some(ProtectionClaim::new(
        WorktreeClaimState::PreWriteDaemon,
        vec![SurfaceClaim {
            identifier: "codex".to_owned(),
            state: SurfaceClaimState::Participating,
        }],
    ));
    protection.latest_run = Some(GateRunSummary {
        id: "latest-gate".to_owned(),
        result: "warn".to_owned(),
        label: "PASSED — score 100/100".to_owned(),
        score: Some(100.0),
        warning_count: 1,
        duration_seconds: Some(0.5),
        started_at: None,
        new_warning_count: None,
        changed_file_count: None,
        checks: Vec::new(),
    });
    let protection = serde_json::to_value(protection).expect("protection DTO");
    assert!(schema_validator("ProtectionOverview").is_valid(&protection));

    let history = ProtectionHistory {
        schema_version: "anvil.dashboard.protection-history.v1".to_owned(),
        data_state: DataState::Complete,
        source_message: "one point".to_owned(),
        actual_range: Some(ProtectionHistoryRange {
            first_recorded_at: "2026-07-27T12:00:00Z".to_owned(),
            last_recorded_at: "2026-07-27T12:00:00Z".to_owned(),
        }),
        points: vec![ProtectionHistoryPoint {
            recorded_at: "2026-07-27T12:00:00Z".to_owned(),
            score: 100.0,
            status: "pass".to_owned(),
            status_label: "PASSED".to_owned(),
            warning_count: 0,
            duration_seconds: Some("0.5".to_owned()),
            checks_run: Some("4".to_owned()),
        }],
        gaps: vec![DataGap {
            component: "drift-history".to_owned(),
            reason: "not produced".to_owned(),
        }],
    };
    assert!(
        schema_validator("ProtectionHistory").is_valid(&serde_json::to_value(history).unwrap())
    );

    let summary = PlanSummary {
        id: "dashboard".to_owned(),
        scope: "DASH".to_owned(),
        title: "Dashboard".to_owned(),
        status: "Ready".to_owned(),
        progress: "1/11".to_owned(),
    };
    let plan = PlanDetail::read_only(
        summary,
        "Ship the dashboard.".to_owned(),
        vec![PlanTimelineEntry {
            id: "DASH-001".to_owned(),
            title: "Scaffold".to_owned(),
            status: "Done".to_owned(),
            validation_contract: Some("pnpm test".to_owned()),
            readiness: false,
        }],
    );
    let plan = serde_json::to_value(plan).expect("plan DTO");
    assert!(schema_validator("PlanDetail").is_valid(&plan));
    assert!(schema_validator("PlanSummary").is_valid(&plan["summary"]));
}

#[test]
fn protection_surface_state_is_a_closed_contract() {
    let validator = schema_validator("SurfaceClaimState");
    for state in anvil_kernel_types::SurfaceClaimState::all() {
        assert!(validator.is_valid(&json!(state.as_str())));
    }
    assert!(!validator.is_valid(&json!("future-state")));
}

#[test]
fn protection_history_score_contract_is_bounded() {
    let validator = schema_validator("ProtectionHistoryPoint");
    let point = |score| {
        json!({
            "recorded_at": "2026-07-27T12:00:00Z",
            "score": score,
            "status": "pass",
            "status_label": "PASSED",
            "warning_count": 0,
            "duration_seconds": null,
            "checks_run": null
        })
    };
    assert!(validator.is_valid(&point(0)));
    assert!(validator.is_valid(&point(100)));
    assert!(!validator.is_valid(&point(-1)));
    assert!(!validator.is_valid(&point(101)));
}

#[tokio::test]
async fn documented_error_statuses_and_bodies_match_runtime_mapping() {
    let document = openapi_document();
    let plan_responses = &document["paths"]["/api/v1/plans/{id}"]["get"]["responses"];
    let list_responses = &document["paths"]["/api/v1/plans"]["get"]["responses"];
    let protection_responses = &document["paths"]["/api/v1/protection"]["get"]["responses"];
    for status in ["400", "500", "503"] {
        assert!(
            plan_responses.get(status).is_some(),
            "plan detail must document {status}"
        );
    }
    assert!(list_responses.get("500").is_some());
    assert!(list_responses.get("503").is_some());
    assert!(protection_responses.get("500").is_some());

    let cases = [
        (ApiError::Plan(PlanReadError::InvalidId), 400),
        (ApiError::Worker, 500),
        (ApiError::Plan(PlanReadError::InvalidUtf8), 503),
    ];
    let validator = schema_validator("Error");
    for (error, expected) in cases {
        let response = error.into_response();
        assert_eq!(response.status().as_u16(), expected);
        let body = to_bytes(response.into_body(), 16 * 1024)
            .await
            .expect("error body");
        let body: Value = serde_json::from_slice(&body).expect("error JSON");
        assert!(validator.is_valid(&body), "invalid Error DTO: {body}");
    }
}
