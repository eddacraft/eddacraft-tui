//! `anvil_find_dependents` MCP tool (GCTX-011 / ADR-084).
//!
//! File-keyed dependency (reverse-impact) traversal for AI assistants: given a
//! workspace-relative file, return the files that import it — the local blast
//! radius — up to a depth-bounded number of hops. Like `anvil_search_symbols`,
//! this tool holds **no graph**: it validates the workspace root (CE-8
//! client-side), forwards a sealed query to the running `anvil-intercept` daemon
//! over `anvil/gctx/find_dependents`, and returns the daemon-projected sealed DTO
//! verbatim. It links only `anvil-gctx-types` (graph-free), so it is structurally
//! incapable of emitting a graph internal (CE-5).
//!
//! Dependents resolve at **file** granularity (an importing file, with the hop
//! distance) — symbol-level *caller* edges are out of scope (GCTX-014). Daemon-
//! required and degrades gracefully (CE-7): when the daemon is absent or has no
//! GCTX surface, the tool returns a structured `unavailable` outcome.

use std::path::Path;

use serde_json::{Value, json};

use anvil_intercept_proto::protocol::{
    ANVIL_GCTX_FIND_DEPENDENTS, AssuranceState, GctxFindDependentsRequest,
    GctxFindDependentsResponse, StaleReason, WorkspaceAssurance,
};

use crate::mcp::gctx_client::{GctxDaemonError, gctx_call};
use crate::mcp::tools::shared::{redact_workspace_root, validate_workspace_root};

pub const TOOL_NAME: &str = "anvil_find_dependents";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Find the files that depend on (import) a given file — its local blast radius — over the workspace's dependency graph. Returns paginated, deterministic, identity-only results at FILE granularity: each importing file with its hop distance (1 = direct importer, 2 = importer-of-an-importer). Symbol-level caller edges are out of scope. Requires the anvil daemon to be running; returns a structured `unavailable`/`not_ready`/`disabled` outcome while the graph is absent, warming, or an operator has switched the surface off (`ANVIL_GCTX_EGRESS=0`).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                },
                "file": {
                    "type": "string",
                    "description": "Workspace-root-relative path whose importers to find"
                },
                "maxDepth": {
                    "type": "integer",
                    "description": "Traversal depth in hops: 1 (direct importers) or 2 (transitive). Clamped server-side; absent defaults to 1.",
                    "minimum": 1,
                    "maximum": 2
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum dependents to return (clamped server-side)",
                    "minimum": 1
                },
                "cursor": {
                    "type": "string",
                    "description": "Opaque pagination cursor from a previous response's `next_cursor`. Echo it back verbatim to fetch the next page; treat it as an opaque token (never construct one)."
                }
            },
            "required": ["workspaceRoot", "file"],
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
    let payload = match find_dependents_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn find_dependents_payload(arguments: &Value) -> Result<Value, String> {
    let server_root = std::env::current_dir()
        .map_err(|err| format!("MCP server cwd is not accessible: {err}"))?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot is required".to_string())?;
    // `file` is required client-side too: a dependents walk has no meaningful
    // "all files" answer, and surfacing the error here (rather than as a daemon
    // round-trip `InvalidQuery`) is a faster, clearer failure.
    let file_present = arguments
        .get("file")
        .and_then(Value::as_str)
        .is_some_and(|f| !f.is_empty());
    if !file_present {
        return Err("file is required".to_string());
    }
    let (server_root, workspace_path) =
        validate_workspace_root(Path::new(workspace_root), &server_root)?;
    let redacted_workspace_root = redact_workspace_root(&workspace_path, &server_root);

    let query = parse_query(arguments)?;
    let request = GctxFindDependentsRequest {
        workspace_root: workspace_path.to_string_lossy().into_owned(),
        query,
    };

    let response = match gctx_call(
        ANVIL_GCTX_FIND_DEPENDENTS,
        &request,
        "mcp-gctx-find-dependents",
    ) {
        Ok(response) => response,
        Err(GctxDaemonError::Unavailable) => unavailable_response(),
        Err(GctxDaemonError::Failure) => {
            return Err("graph-context daemon request failed".to_string());
        }
    };

    // GCTX-010 C1 (ADR-085) on-demand re-warm: a `NotReady` graph is the one
    // outcome a retry can recover from, so enqueue a full scan to make the *next*
    // query more likely to succeed. Best-effort, fire-and-forget; the daemon-side
    // executor (DSV-045) drives and coalesces the scan.
    if should_rewarm(&response.outcome) {
        let _ = crate::commands::watch_save_time::warm_up_root(&workspace_path);
    }

    Ok(render_response(&response, &redacted_workspace_root))
}

/// Whether a dependents outcome warrants an on-demand re-warm (GCTX-010 C1). Only
/// a `NotReady` graph benefits. Written as an exhaustive match (not `matches!`) so
/// a future [`FindDependentsOutcome`](anvil_gctx_types::FindDependentsOutcome)
/// variant forces a compile error here rather than silently defaulting.
fn should_rewarm(outcome: &anvil_gctx_types::FindDependentsOutcome) -> bool {
    use anvil_gctx_types::FindDependentsOutcome as Outcome;
    match outcome {
        Outcome::NotReady { .. } => true,
        Outcome::Ready(_)
        | Outcome::Unavailable
        | Outcome::Disabled
        | Outcome::InvalidQuery { .. } => false,
    }
}

/// Build a [`FindDependentsQuery`](anvil_gctx_types::FindDependentsQuery) from the
/// MCP arguments, mapping the camel-case `maxDepth` argument to the snake-case
/// `max_depth` field. An unparseable field is a tool error.
fn parse_query(arguments: &Value) -> Result<anvil_gctx_types::FindDependentsQuery, String> {
    let mut fields = serde_json::Map::new();
    for (arg_key, field_key) in [
        ("file", "file"),
        ("maxDepth", "max_depth"),
        ("limit", "limit"),
        ("cursor", "cursor"),
    ] {
        if let Some(value) = arguments.get(arg_key)
            && !value.is_null()
        {
            fields.insert(field_key.to_string(), value.clone());
        }
    }
    serde_json::from_value(Value::Object(fields))
        .map_err(|err| format!("invalid find_dependents parameter: {err}"))
}

/// Merge the sealed daemon response with the redacted workspace-root echo.
fn render_response(response: &GctxFindDependentsResponse, redacted_workspace_root: &str) -> Value {
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
fn unavailable_response() -> GctxFindDependentsResponse {
    GctxFindDependentsResponse {
        workspace_assurance: WorkspaceAssurance {
            state: AssuranceState::Unavailable,
            reason: Some(StaleReason::DaemonAbsent),
            generation: 0,
            last_full_scan: None,
            scan_coverage: None,
        },
        outcome: anvil_gctx_types::FindDependentsOutcome::Unavailable,
    }
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("find_dependents payload serialises");
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
        let result = call(&json!({ "file": "src/a.ts" }));
        assert_eq!(result["isError"], true);
        assert_eq!(payload_of(&result)["error"], "workspaceRoot is required");
    }

    #[test]
    fn rejects_missing_file() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({ "workspaceRoot": workspace.path() }));
        assert_eq!(result["isError"], true);
        assert_eq!(payload_of(&result)["error"], "file is required");
    }

    #[test]
    fn rejects_relative_workspace_root() {
        let result = call(&json!({ "workspaceRoot": ".", "file": "src/a.ts" }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            payload_of(&result)["error"],
            "workspaceRoot must be an absolute path"
        );
    }

    #[test]
    fn rewarm_fires_only_on_not_ready() {
        use anvil_gctx_types::{FindDependentsOutcome, FindDependentsProjection, RedactionSummary};

        assert!(should_rewarm(&FindDependentsOutcome::NotReady {
            recovery_hint: "warming".into(),
        }));
        assert!(!should_rewarm(&FindDependentsOutcome::Ready(
            FindDependentsProjection {
                dependents: Vec::new(),
                next_cursor: None,
                redaction_summary: RedactionSummary::default(),
                partial: false,
            }
        )));
        assert!(!should_rewarm(&FindDependentsOutcome::Unavailable));
        assert!(!should_rewarm(&FindDependentsOutcome::Disabled));
        assert!(!should_rewarm(&FindDependentsOutcome::InvalidQuery {
            reason: "bad".into(),
        }));
    }

    #[test]
    fn degrades_to_unavailable_without_a_daemon() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "file": "src/a.ts"
        }));

        assert_eq!(result["isError"], false);
        let payload = payload_of(&result);
        assert_eq!(payload["outcome"]["status"], "unavailable");
        assert_eq!(payload["workspace_assurance"]["state"], "unavailable");
        assert!(payload.get("workspaceRoot").is_some());
    }

    #[test]
    fn accepts_max_depth_and_cursor_arguments() {
        // `maxDepth` and `cursor` must parse through to the query (not be dropped
        // or rejected). Without a daemon it still degrades to `unavailable`.
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "file": "src/a.ts",
            "maxDepth": 2,
            "cursor": "deadbeef"
        }));
        assert_eq!(result["isError"], false);
        assert_eq!(payload_of(&result)["outcome"]["status"], "unavailable");
    }
}
