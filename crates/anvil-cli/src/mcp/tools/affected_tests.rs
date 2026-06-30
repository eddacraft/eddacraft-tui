//! `anvil_affected_tests` MCP tool (GCTX-013 / ADR-084).
//!
//! Given a set of **changed file paths**, return the tests likely relevant to
//! the change as one structured, identity-only `AffectedTestsReport`: each test
//! file that imports a changed file (with its evidence edges — the changed
//! sources it depends on — and traversal distance), plus the changed non-test
//! files with no resident test importer (coverage gaps) — so an assistant runs
//! the right tests and spots uncovered edits.
//!
//! Like the sibling GCTX tools, this holds **no graph**: it validates the
//! workspace root (CE-8 client-side), forwards a sealed query to the running
//! `anvil-intercept` daemon over `anvil/gctx/affected_tests`, and returns the
//! daemon-projected sealed DTO verbatim. It links only `anvil-gctx-types`
//! (graph-free), so it is structurally incapable of emitting a graph internal
//! (CE-5). Input is **paths only** — never diff content (CE-6). Daemon-required
//! and degrades gracefully (CE-7).

use std::path::Path;

use serde_json::{Value, json};

use anvil_intercept_proto::protocol::{
    ANVIL_GCTX_AFFECTED_TESTS, AssuranceState, GctxAffectedTestsRequest, GctxAffectedTestsResponse,
    StaleReason, WorkspaceAssurance,
};

use crate::mcp::gctx_client::{GctxDaemonError, daemon_rpc_call};
use crate::mcp::tools::shared::{redact_workspace_root, validate_workspace_root};

pub const TOOL_NAME: &str = "anvil_affected_tests";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Report which tests are likely relevant to a change set, and which changed files have no test. Given changed file PATHS (never diff content; ≤200 files), returns a deterministic, identity-only AffectedTestsReport: tests (each test file that imports a changed file within the depth bound, with its evidence edges — the changed sources it depends on — and hop distance) and coverage_gaps (changed non-test files with no test importer). Relevance is an import heuristic (file-keyed, not execution-verified), marked `heuristic: true`. Requires the anvil daemon to be running; returns a structured `unavailable`/`not_ready`/`disabled` outcome while the graph is absent, warming, or an operator has switched the surface off (`ANVIL_GCTX_EGRESS=0`).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                },
                "changedFiles": {
                    "type": "array",
                    "description": "Workspace-root-relative changed file paths (≤200). Paths only — never diff content.",
                    "items": { "type": "string" }
                },
                "maxDepth": {
                    "type": "integer",
                    "description": "Reverse-impact traversal depth for test discovery and coverage: 1 (direct importers) or 2 (transitive). Clamped server-side; absent defaults to 1.",
                    "minimum": 1,
                    "maximum": 2
                }
            },
            "required": ["workspaceRoot", "changedFiles"],
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true
        }
    })
}

pub fn call(arguments: &Value) -> Value {
    let payload = match affected_tests_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn affected_tests_payload(arguments: &Value) -> Result<Value, String> {
    let server_root = std::env::current_dir()
        .map_err(|err| format!("MCP server cwd is not accessible: {err}"))?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot is required".to_string())?;
    let changed_files = parse_changed_files(arguments)?;

    let (server_root, workspace_path) =
        validate_workspace_root(Path::new(workspace_root), &server_root)?;
    let redacted_workspace_root = redact_workspace_root(&workspace_path, &server_root);

    let request = GctxAffectedTestsRequest {
        workspace_root: workspace_path.to_string_lossy().into_owned(),
        query: anvil_gctx_types::AffectedTestsQuery {
            changed_files,
            // An out-of-`u32`-range depth saturates to `u32::MAX`, which the
            // daemon clamps down to the GV2-026 ceiling — never a wrap.
            max_depth: arguments
                .get("maxDepth")
                .and_then(Value::as_u64)
                .map(|d| u32::try_from(d).unwrap_or(u32::MAX)),
        },
    };

    let response = match daemon_rpc_call(
        ANVIL_GCTX_AFFECTED_TESTS,
        &request,
        "mcp-gctx-affected-tests",
    ) {
        Ok(response) => response,
        Err(GctxDaemonError::Unavailable) => unavailable_response(),
        Err(GctxDaemonError::Failure) => {
            return Err("graph-context daemon request failed".to_string());
        }
    };

    // GCTX-010 C1 on-demand re-warm: a `NotReady` graph is the one outcome a
    // retry can recover from. Best-effort, fire-and-forget.
    if should_rewarm(&response.outcome) {
        let _ = crate::commands::watch_save_time::warm_up_root(&workspace_path);
    }

    Ok(render_response(&response, &redacted_workspace_root))
}

/// Parse and shallow-validate the required `changedFiles` array. An empty or
/// non-array value is a tool error surfaced here (faster than a daemon round
/// trip); per-path hygiene + the ≤200 cap are enforced daemon-side (CE-6).
fn parse_changed_files(arguments: &Value) -> Result<Vec<String>, String> {
    let array = arguments
        .get("changedFiles")
        .and_then(Value::as_array)
        .ok_or_else(|| "changedFiles is required (an array of paths)".to_string())?;
    if array.is_empty() {
        return Err("changedFiles must not be empty".to_string());
    }
    array
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| "each changedFiles entry must be a string".to_string())
        })
        .collect()
}

/// Whether an affected-tests outcome warrants an on-demand re-warm (GCTX-010 C1).
/// Only a `NotReady` graph benefits. Exhaustive match so a future
/// [`AffectedTestsOutcome`](anvil_gctx_types::AffectedTestsOutcome) variant forces
/// a compile error here rather than silently defaulting.
fn should_rewarm(outcome: &anvil_gctx_types::AffectedTestsOutcome) -> bool {
    use anvil_gctx_types::AffectedTestsOutcome as Outcome;
    match outcome {
        Outcome::NotReady { .. } => true,
        Outcome::Ready(_)
        | Outcome::Unavailable
        | Outcome::Disabled
        | Outcome::InvalidQuery { .. } => false,
    }
}

/// Merge the sealed daemon response with the redacted workspace-root echo.
fn render_response(response: &GctxAffectedTestsResponse, redacted_workspace_root: &str) -> Value {
    let mut value = serde_json::to_value(response).expect("gctx response serialises");
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "workspaceRoot".to_string(),
            Value::String(redacted_workspace_root.to_string()),
        );
    }
    value
}

/// The CE-7 degradation reply when the daemon is absent / has no GCTX surface.
fn unavailable_response() -> GctxAffectedTestsResponse {
    GctxAffectedTestsResponse {
        workspace_assurance: WorkspaceAssurance {
            state: AssuranceState::Unavailable,
            reason: Some(StaleReason::DaemonAbsent),
            generation: 0,
            last_full_scan: None,
            scan_coverage: None,
        },
        outcome: anvil_gctx_types::AffectedTestsOutcome::Unavailable,
    }
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("affected_tests payload serialises");
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": payload.get("error").is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload_of(result: &Value) -> Value {
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap())
            .expect("payload is JSON")
    }

    #[test]
    fn descriptor_advertises_tool_name() {
        assert_eq!(descriptor()["name"], TOOL_NAME);
        assert_eq!(descriptor()["annotations"]["readOnlyHint"], true);
    }

    #[test]
    fn rejects_missing_workspace_root() {
        let result = call(&json!({ "changedFiles": ["src/a.ts"] }));
        assert_eq!(result["isError"], true);
        assert_eq!(payload_of(&result)["error"], "workspaceRoot is required");
    }

    #[test]
    fn rejects_missing_changed_files() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({ "workspaceRoot": workspace.path() }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            payload_of(&result)["error"],
            "changedFiles is required (an array of paths)"
        );
    }

    #[test]
    fn rejects_empty_changed_files() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({ "workspaceRoot": workspace.path(), "changedFiles": [] }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            payload_of(&result)["error"],
            "changedFiles must not be empty"
        );
    }

    #[test]
    fn rewarm_fires_only_on_not_ready() {
        use anvil_gctx_types::{AffectedTestsOutcome, AffectedTestsReport, AffectedTestsSummary};

        assert!(should_rewarm(&AffectedTestsOutcome::NotReady {
            recovery_hint: "warming".into(),
        }));
        assert!(!should_rewarm(&AffectedTestsOutcome::Ready(
            AffectedTestsReport {
                tests: Vec::new(),
                coverage_gaps: Vec::new(),
                heuristic: true,
                summary: AffectedTestsSummary::default(),
            }
        )));
        assert!(!should_rewarm(&AffectedTestsOutcome::Unavailable));
        assert!(!should_rewarm(&AffectedTestsOutcome::Disabled));
        assert!(!should_rewarm(&AffectedTestsOutcome::InvalidQuery {
            reason: "bad".into(),
        }));
    }

    #[test]
    fn degrades_to_unavailable_without_a_daemon() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "changedFiles": ["src/a.ts"]
        }));

        assert_eq!(result["isError"], false);
        let payload = payload_of(&result);
        assert_eq!(payload["outcome"]["status"], "unavailable");
        assert_eq!(payload["workspace_assurance"]["state"], "unavailable");
        assert!(payload.get("workspaceRoot").is_some());
    }
}
