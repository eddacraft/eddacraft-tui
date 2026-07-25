use serde_json::{Value, json};

pub fn openapi_document() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "anvil local dashboard API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Loopback-only, read-only dashboard data for one local workspace."
        },
        "servers": [{ "url": "/", "description": "Current loopback server" }],
        "paths": {
            "/healthz": {
                "get": operation("Health", "HealthResponse", "Dashboard server health", false)
            },
            "/api/v1/protection": {
                "get": operation("Protection", "ProtectionOverview", "Current local protection evidence", true)
            },
            "/api/v1/patterns": {
                "get": operation("Patterns", "PatternCatalogue", "Compiled anti-pattern catalogue", true)
            },
            "/api/v1/plans": {
                "get": {
                    "operationId": "listPlans",
                    "summary": "Indexed APS plans",
                    "tags": ["Plans"],
                    "responses": {
                        "200": {
                            "description": "Plan summaries",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": { "$ref": "#/components/schemas/PlanSummary" }
                                    }
                                }
                            }
                        },
                        "403": response("Cross-origin request rejected", "Error"),
                        "421": response("Loopback Host required", "Error"),
                        "500": response("Dashboard worker failed", "Error"),
                        "503": response("Plan data unavailable", "Error")
                    }
                }
            },
            "/api/v1/plans/{id}": {
                "get": {
                    "operationId": "getPlan",
                    "summary": "Selected APS plan detail",
                    "tags": ["Plans"],
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string", "pattern": "^[A-Za-z0-9_-]+$" }
                    }],
                    "responses": {
                        "200": response("Plan detail", "PlanDetail"),
                        "400": response("Invalid plan identifier", "Error"),
                        "403": response("Cross-origin request rejected", "Error"),
                        "404": response("Plan not found", "Error"),
                        "421": response("Loopback Host required", "Error"),
                        "500": response("Dashboard worker failed", "Error"),
                        "503": response("Plan data unavailable", "Error")
                    }
                }
            },
            "/openapi.json": {
                "get": {
                    "operationId": "getOpenApi",
                    "summary": "This OpenAPI document",
                    "tags": ["Contract"],
                    "responses": {
                        "200": { "description": "OpenAPI 3.1 document" },
                        "403": response("Cross-origin request rejected", "Error"),
                        "421": response("Loopback Host required", "Error")
                    }
                }
            }
        },
        "components": {
            "schemas": schemas()
        }
    })
}

fn operation(tag: &str, schema: &str, summary: &str, worker_failure: bool) -> Value {
    let operation_id = match schema {
        "HealthResponse" => "getHealth",
        "ProtectionOverview" => "getProtectionOverview",
        "PatternCatalogue" => "getPatternCatalogue",
        _ => "getResource",
    };
    let mut responses = serde_json::Map::from_iter([
        ("200".to_owned(), response("Successful response", schema)),
        (
            "403".to_owned(),
            response("Cross-origin request rejected", "Error"),
        ),
        (
            "421".to_owned(),
            response("Loopback Host required", "Error"),
        ),
    ]);
    if worker_failure {
        responses.insert(
            "500".to_owned(),
            response("Dashboard worker failed", "Error"),
        );
    }
    json!({
        "operationId": operation_id,
        "summary": summary,
        "tags": [tag],
        "responses": responses
    })
}

fn response(description: &str, schema: &str) -> Value {
    json!({
        "description": description,
        "content": {
            "application/json": {
                "schema": { "$ref": format!("#/components/schemas/{schema}") }
            }
        }
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "the deterministic contract is intentionally kept in one visible schema map"
)]
fn schemas() -> Value {
    json!({
        "HealthResponse": {
            "type": "object",
            "additionalProperties": false,
            "required": ["status", "access"],
            "properties": {
                "status": { "type": "string", "const": "ok" },
                "access": { "type": "string", "const": "read-only" }
            }
        },
        "DataState": {
            "type": "string",
            "enum": ["complete", "partial", "unavailable"]
        },
        "ProtectionClaim": {
            "type": "object",
            "required": ["schema_version", "worktree_state", "surfaces"],
            "properties": {
                "schema_version": { "type": "string", "const": "anvil.protection-claim.v1" },
                "worktree_state": {
                    "type": "string",
                    "enum": ["unprotected", "warming", "pre-write-embedded", "pre-write-daemon", "save-time-only", "full", "degraded-protection", "cross-boundary-mixed", "multi-daemon-detected", "path-uncertain"]
                },
                "surfaces": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["identifier", "state"],
                        "properties": {
                            "identifier": { "type": "string" },
                            "state": { "$ref": "#/components/schemas/SurfaceClaimState" }
                        }
                    }
                }
            }
        },
        "SurfaceClaimState": {
            "type": "string",
            "enum": ["unbound", "attached", "participating", "embedded-fallback", "degraded", "cross-boundary-refused", "quarantined", "detached"]
        },
        "GateCheckSummary": {
            "type": "object",
            "required": ["name", "status", "score", "message"],
            "properties": {
                "name": { "type": "string" },
                "status": { "type": "string" },
                "score": { "type": ["string", "null"] },
                "message": { "type": "string" }
            }
        },
        "GateRunSummary": {
            "type": "object",
            "required": ["id", "result", "label", "score", "warning_count", "duration_seconds", "started_at", "new_warning_count", "changed_file_count", "checks"],
            "properties": {
                "id": { "type": "string" },
                "result": { "type": "string" },
                "label": { "type": "string" },
                "score": { "type": ["number", "null"] },
                "warning_count": { "type": "integer", "minimum": 0 },
                "duration_seconds": { "type": ["number", "null"] },
                "started_at": { "type": ["string", "null"] },
                "new_warning_count": { "type": ["integer", "null"], "minimum": 0 },
                "changed_file_count": { "type": ["integer", "null"], "minimum": 0 },
                "checks": { "type": "array", "items": { "$ref": "#/components/schemas/GateCheckSummary" } }
            }
        },
        "EvidenceLine": {
            "type": "object",
            "additionalProperties": false,
            "required": ["number", "text", "highlighted"],
            "properties": {
                "number": { "type": "integer", "minimum": 0 },
                "text": { "type": "string" },
                "highlighted": { "type": "boolean" }
            }
        },
        "WarningSummary": {
            "type": "object",
            "required": ["id", "severity", "category", "message", "file_path", "age_label", "evidence_id", "rule", "line", "explanation", "matched_pattern", "evidence_excerpt"],
            "properties": {
                "id": { "type": "string" },
                "severity": { "type": "string" },
                "category": { "type": "string" },
                "message": { "type": "string" },
                "file_path": { "type": ["string", "null"] },
                "age_label": { "type": "string" },
                "evidence_id": { "type": "string" },
                "rule": { "type": "string" },
                "line": { "type": ["integer", "null"], "minimum": 0 },
                "explanation": { "type": "string" },
                "matched_pattern": { "type": "string" },
                "evidence_excerpt": { "type": "array", "items": { "$ref": "#/components/schemas/EvidenceLine" } }
            }
        },
        "AttentionItem": {
            "type": "object",
            "required": ["title", "detail", "evidence_id"],
            "properties": {
                "title": { "type": "string" },
                "detail": { "type": "string" },
                "evidence_id": { "type": ["string", "null"] }
            }
        },
        "AffectedFile": {
            "type": "object",
            "required": ["path", "highest_severity", "warning_count", "first_seen", "last_seen", "warning_id"],
            "properties": {
                "path": { "type": "string" },
                "highest_severity": { "type": "string" },
                "warning_count": { "type": "integer", "minimum": 0 },
                "first_seen": { "type": "string" },
                "last_seen": { "type": "string" },
                "warning_id": { "type": "string" }
            }
        },
        "AssuranceSummary": {
            "type": "object",
            "additionalProperties": false,
            "required": ["state", "reason", "generation", "last_full_scan", "scanned_files", "total_files"],
            "properties": {
                "state": { "type": "string", "enum": ["clean", "stale", "pending", "running", "bounded", "unavailable", "unknown"] },
                "reason": { "type": ["string", "null"] },
                "generation": { "type": "integer", "minimum": 0 },
                "last_full_scan": { "type": ["string", "null"] },
                "scanned_files": { "type": ["integer", "null"], "minimum": 0 },
                "total_files": { "type": ["integer", "null"], "minimum": 0 }
            }
        },
        "SaveTimeSummary": {
            "type": "object",
            "additionalProperties": false,
            "required": ["state", "active", "failure_count"],
            "properties": {
                "state": { "type": "string", "enum": ["attached", "absent", "failed", "unknown"] },
                "active": { "type": "boolean" },
                "failure_count": { "type": "integer", "minimum": 0 }
            }
        },
        "DataGap": {
            "type": "object",
            "additionalProperties": false,
            "required": ["component", "reason"],
            "properties": {
                "component": { "type": "string" },
                "reason": { "type": "string" }
            }
        },
        "ProtectionOverview": {
            "type": "object",
            "required": ["schema_version", "data_state", "source_message", "claim", "assurance", "save_time", "observed_at_unix", "latest_run", "recent_runs", "next_attention", "warnings_state", "warnings", "affected_files_state", "affected_files", "gaps"],
            "properties": {
                "schema_version": { "type": "string", "const": "anvil.dashboard.protection.v1" },
                "data_state": { "$ref": "#/components/schemas/DataState" },
                "source_message": { "type": "string" },
                "claim": { "oneOf": [{ "$ref": "#/components/schemas/ProtectionClaim" }, { "type": "null" }] },
                "assurance": { "oneOf": [{ "$ref": "#/components/schemas/AssuranceSummary" }, { "type": "null" }] },
                "save_time": { "oneOf": [{ "$ref": "#/components/schemas/SaveTimeSummary" }, { "type": "null" }] },
                "observed_at_unix": { "type": ["integer", "null"], "minimum": 0 },
                "latest_run": { "oneOf": [{ "$ref": "#/components/schemas/GateRunSummary" }, { "type": "null" }] },
                "recent_runs": { "type": "array", "items": { "$ref": "#/components/schemas/GateRunSummary" } },
                "next_attention": { "oneOf": [{ "$ref": "#/components/schemas/AttentionItem" }, { "type": "null" }] },
                "warnings_state": { "$ref": "#/components/schemas/DataState" },
                "warnings": { "type": "array", "items": { "$ref": "#/components/schemas/WarningSummary" } },
                "affected_files_state": { "$ref": "#/components/schemas/DataState" },
                "affected_files": { "type": "array", "items": { "$ref": "#/components/schemas/AffectedFile" } },
                "gaps": { "type": "array", "items": { "$ref": "#/components/schemas/DataGap" } }
            }
        },
        "PatternSummary": {
            "type": "object",
            "required": ["id", "title", "family", "severity", "enabled", "instance_count", "description"],
            "properties": {
                "id": { "type": "string" },
                "title": { "type": "string" },
                "family": { "type": "string" },
                "severity": { "type": "string" },
                "enabled": { "type": "boolean" },
                "instance_count": { "type": "integer", "minimum": 0 },
                "description": { "type": "string" }
            }
        },
        "PatternCatalogue": {
            "type": "object",
            "required": ["schema_version", "data_state", "source_message", "patterns"],
            "properties": {
                "schema_version": { "type": "string", "const": "anvil.dashboard.patterns.v1" },
                "data_state": { "$ref": "#/components/schemas/DataState" },
                "source_message": { "type": "string" },
                "patterns": { "type": "array", "items": { "$ref": "#/components/schemas/PatternSummary" } }
            }
        },
        "PlanSummary": {
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "scope", "title", "status", "progress"],
            "properties": {
                "id": { "type": "string" },
                "scope": { "type": "string" },
                "title": { "type": "string" },
                "status": { "type": "string" },
                "progress": { "type": "string" }
            }
        },
        "PlanDetail": {
            "type": "object",
            "additionalProperties": false,
            "required": ["schema_version", "summary", "purpose", "actions_enabled", "action_message", "timeline"],
            "properties": {
                "schema_version": { "type": "string", "const": "anvil.dashboard.plans.v1" },
                "summary": { "$ref": "#/components/schemas/PlanSummary" },
                "purpose": { "type": "string" },
                "actions_enabled": { "type": "boolean", "const": false },
                "action_message": { "type": "string" },
                "timeline": { "type": "array", "items": { "$ref": "#/components/schemas/PlanTimelineEntry" } }
            }
        },
        "PlanTimelineEntry": {
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "title", "status", "validation_contract", "readiness"],
            "properties": {
                "id": { "type": "string" },
                "title": { "type": "string" },
                "status": { "type": "string" },
                "validation_contract": { "type": ["string", "null"] },
                "readiness": { "type": "boolean" }
            }
        },
        "Error": {
            "type": "object",
            "additionalProperties": false,
            "required": ["code", "message"],
            "properties": {
                "code": { "type": "string" },
                "message": { "type": "string" }
            }
        }
    })
}
