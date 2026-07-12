use serde_json::{Value, json};

pub fn openapi_document() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Anvil Local Dashboard API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Loopback-only, read-only dashboard data for one local workspace."
        },
        "servers": [{ "url": "/", "description": "Current loopback server" }],
        "paths": {
            "/healthz": {
                "get": operation("Health", "HealthResponse", "Dashboard server health")
            },
            "/api/v1/protection": {
                "get": operation("Protection", "ProtectionOverview", "Current local protection evidence")
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
                        }
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
                        "404": response("Plan not found", "Error")
                    }
                }
            },
            "/openapi.json": {
                "get": {
                    "operationId": "getOpenApi",
                    "summary": "This OpenAPI document",
                    "tags": ["Contract"],
                    "responses": { "200": { "description": "OpenAPI 3.1 document" } }
                }
            }
        },
        "components": {
            "schemas": schemas()
        }
    })
}

fn operation(tag: &str, schema: &str, summary: &str) -> Value {
    let operation_id = match schema {
        "HealthResponse" => "getHealth",
        "ProtectionOverview" => "getProtectionOverview",
        _ => "getResource",
    };
    json!({
        "operationId": operation_id,
        "summary": summary,
        "tags": [tag],
        "responses": { "200": response("Successful response", schema) }
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
                            "state": { "type": "string" }
                        }
                    }
                }
            }
        },
        "GateRunSummary": {
            "type": "object",
            "required": ["id", "result", "label", "score", "warning_count", "duration_seconds"],
            "properties": {
                "id": { "type": "string" },
                "result": { "type": "string" },
                "label": { "type": "string" },
                "score": { "type": ["number", "null"] },
                "warning_count": { "type": "integer", "minimum": 0 },
                "duration_seconds": { "type": ["number", "null"] }
            }
        },
        "WarningSummary": {
            "type": "object",
            "required": ["id", "severity", "category", "message", "file_path", "age_label", "evidence_id"],
            "properties": {
                "id": { "type": "string" },
                "severity": { "type": "string" },
                "category": { "type": "string" },
                "message": { "type": "string" },
                "file_path": { "type": ["string", "null"] },
                "age_label": { "type": "string" },
                "evidence_id": { "type": "string" }
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
            "required": ["path", "highest_severity", "warning_count"],
            "properties": {
                "path": { "type": "string" },
                "highest_severity": { "type": "string" },
                "warning_count": { "type": "integer", "minimum": 0 }
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
            "required": ["schema_version", "data_state", "source_message", "claim", "assurance", "save_time", "observed_at_unix", "latest_run", "next_attention", "warnings_state", "warnings", "affected_files_state", "affected_files", "gaps"],
            "properties": {
                "schema_version": { "type": "string", "const": "anvil.dashboard.protection.v1" },
                "data_state": { "$ref": "#/components/schemas/DataState" },
                "source_message": { "type": "string" },
                "claim": { "oneOf": [{ "$ref": "#/components/schemas/ProtectionClaim" }, { "type": "null" }] },
                "assurance": { "oneOf": [{ "$ref": "#/components/schemas/AssuranceSummary" }, { "type": "null" }] },
                "save_time": { "oneOf": [{ "$ref": "#/components/schemas/SaveTimeSummary" }, { "type": "null" }] },
                "observed_at_unix": { "type": ["integer", "null"], "minimum": 0 },
                "latest_run": { "oneOf": [{ "$ref": "#/components/schemas/GateRunSummary" }, { "type": "null" }] },
                "next_attention": { "oneOf": [{ "$ref": "#/components/schemas/AttentionItem" }, { "type": "null" }] },
                "warnings_state": { "$ref": "#/components/schemas/DataState" },
                "warnings": { "type": "array", "items": { "$ref": "#/components/schemas/WarningSummary" } },
                "affected_files_state": { "$ref": "#/components/schemas/DataState" },
                "affected_files": { "type": "array", "items": { "$ref": "#/components/schemas/AffectedFile" } },
                "gaps": { "type": "array", "items": { "$ref": "#/components/schemas/DataGap" } }
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
            "required": ["schema_version", "summary", "purpose", "actions_enabled"],
            "properties": {
                "schema_version": { "type": "string", "const": "anvil.dashboard.plans.v1" },
                "summary": { "$ref": "#/components/schemas/PlanSummary" },
                "purpose": { "type": "string" },
                "actions_enabled": { "type": "boolean", "const": false }
            }
        },
        "Error": {
            "type": "object",
            "required": ["code", "message"],
            "properties": {
                "code": { "type": "string" },
                "message": { "type": "string" }
            }
        }
    })
}
