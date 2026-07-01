//! `anvil_find_callers` MCP tool (GCTX-014 / ADR-084 / GCALL-007).
//!
//! Symbol-keyed caller (reverse call-graph) traversal for AI assistants: given a
//! symbol identity, return the symbols that call it — its local caller set — up to
//! a depth-bounded number of hops. Like `anvil_find_dependents`, this tool holds
//! **no graph**: it validates the workspace root (CE-8 client-side), forwards a
//! sealed query to the running `anvil-intercept` daemon over
//! `anvil/gctx/find_callers`, and returns the daemon-projected sealed DTO
//! verbatim. It links only `anvil-gctx-types` (graph-free), so it is structurally
//! incapable of emitting a graph internal (CE-5).
//!
//! The call graph is **best-effort and static** (GCALL-007 CALL-1): each caller
//! carries a `heuristic` flag when its call is an overload fan-out, and the page a
//! `partial` flag when the result may be incomplete — an assistant must not treat
//! the result as an authoritative caller set. Daemon-required and degrades
//! gracefully (CE-7).

use std::path::Path;

use serde_json::{Value, json};

use anvil_intercept_proto::protocol::{
    ANVIL_GCTX_FIND_CALLERS, AssuranceState, GctxFindCallersRequest, GctxFindCallersResponse,
    StaleReason, WorkspaceAssurance,
};

use crate::mcp::gctx_client::{DaemonRpcError, daemon_rpc_call};
use crate::mcp::tools::shared::{redact_workspace_root, validate_workspace_root};

pub const TOOL_NAME: &str = "anvil_find_callers";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Find the symbols that call a given symbol — its local caller set — over the workspace's call graph. Returns paginated, deterministic, identity-only results at SYMBOL granularity: each calling symbol with its hop distance (1 = direct caller, 2 = caller-of-a-caller) and a `heuristic` flag (true when the call is an overload fan-out, so the caller may be over-included). File-level dependency traversal is `anvil_find_dependents`. The result is a BEST-EFFORT STATIC over-approximation — it cannot see dynamic dispatch (so callers may be missing) and a `partial` flag marks a result that may be incomplete; do not treat it as an authoritative caller set. Requires the anvil daemon to be running; returns a structured `unavailable`/`not_ready`/`disabled` outcome while the graph is absent, warming, or an operator has switched the surface off (`ANVIL_GCTX_EGRESS=0`).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                },
                "target": {
                    "type": "object",
                    "description": "The symbol whose callers to find, by stable identity.",
                    "properties": {
                        "file": { "type": "string", "description": "Workspace-root-relative file path defining the symbol" },
                        "kind": { "type": "string", "description": "Symbol kind: Function, Class, Method, etc." },
                        "name": { "type": "string", "description": "Symbol name (methods are `Owner.method`)" },
                        "ordinal": { "type": "integer", "description": "Overload disambiguator: occurrence index among same-(kind,name) symbols in the file (0 for the first/only).", "minimum": 0 }
                    },
                    "required": ["file", "kind", "name"]
                },
                "maxDepth": {
                    "type": "integer",
                    "description": "Traversal depth in hops: 1 (direct callers) or 2 (transitive). Clamped server-side; absent defaults to 1.",
                    "minimum": 1,
                    "maximum": 2
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum callers to return (clamped server-side)",
                    "minimum": 1
                },
                "cursor": {
                    "type": "string",
                    "description": "Opaque pagination cursor from a previous response's `next_cursor`. Echo it back verbatim to fetch the next page; treat it as an opaque token (never construct one)."
                }
            },
            "required": ["workspaceRoot", "target"],
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
    let payload = match find_callers_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn find_callers_payload(arguments: &Value) -> Result<Value, String> {
    let server_root = std::env::current_dir()
        .map_err(|err| format!("MCP server cwd is not accessible: {err}"))?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot is required".to_string())?;
    // `target` is required client-side too: a caller walk has no meaningful "all
    // symbols" answer. Surfacing it here is faster than a daemon round-trip, and
    // it mirrors the daemon's own validation (a non-empty `name` AND `file`).
    let target = arguments.get("target");
    let non_empty_str = |field: &str| {
        target
            .and_then(|t| t.get(field))
            .and_then(Value::as_str)
            .is_some_and(|v| !v.is_empty())
    };
    if !non_empty_str("name") || !non_empty_str("file") {
        return Err("target (with a non-empty name and file) is required".to_string());
    }
    let (server_root, workspace_path) =
        validate_workspace_root(Path::new(workspace_root), &server_root)?;
    let redacted_workspace_root = redact_workspace_root(&workspace_path, &server_root);

    let query = parse_query(arguments)?;
    let request = GctxFindCallersRequest {
        workspace_root: workspace_path.to_string_lossy().into_owned(),
        query,
    };

    let response = match daemon_rpc_call(ANVIL_GCTX_FIND_CALLERS, &request, "mcp-gctx-find-callers")
    {
        Ok(response) => response,
        Err(DaemonRpcError::Unavailable) => unavailable_response(),
        Err(DaemonRpcError::Failure) => {
            return Err("graph-context daemon request failed".to_string());
        }
    };

    // GCTX-010 C1 (ADR-085) on-demand re-warm: a `NotReady` graph is the one
    // outcome a retry can recover from. Best-effort, fire-and-forget.
    if should_rewarm(&response.outcome) {
        let _ = crate::commands::watch_save_time::warm_up_root(&workspace_path);
    }

    Ok(render_response(&response, &redacted_workspace_root))
}

/// Whether a callers outcome warrants an on-demand re-warm (GCTX-010 C1). Only a
/// `NotReady` graph benefits. Exhaustive match so a future
/// [`FindCallersOutcome`](anvil_gctx_types::FindCallersOutcome) variant forces an
/// explicit classification.
fn should_rewarm(outcome: &anvil_gctx_types::FindCallersOutcome) -> bool {
    use anvil_gctx_types::FindCallersOutcome as Outcome;
    match outcome {
        Outcome::NotReady { .. } => true,
        Outcome::Ready(_)
        | Outcome::Unavailable
        | Outcome::Disabled
        | Outcome::InvalidQuery { .. } => false,
    }
}

/// Build a [`FindCallersQuery`](anvil_gctx_types::FindCallersQuery) from the MCP
/// arguments, mapping the camel-case `maxDepth` to `max_depth`. The `target`
/// object is forwarded as-is (its field names already match `SymbolIdentity`). An
/// unparseable field is a tool error.
fn parse_query(arguments: &Value) -> Result<anvil_gctx_types::FindCallersQuery, String> {
    let mut fields = serde_json::Map::new();
    if let Some(target) = arguments.get("target").filter(|t| !t.is_null()) {
        fields.insert("target".to_string(), target.clone());
    }
    for (arg_key, field_key) in [
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
        .map_err(|err| format!("invalid find_callers parameter: {err}"))
}

/// Merge the sealed daemon response with the redacted workspace-root echo.
fn render_response(response: &GctxFindCallersResponse, redacted_workspace_root: &str) -> Value {
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
fn unavailable_response() -> GctxFindCallersResponse {
    GctxFindCallersResponse {
        workspace_assurance: WorkspaceAssurance {
            state: AssuranceState::Unavailable,
            reason: Some(StaleReason::DaemonAbsent),
            generation: 0,
            last_full_scan: None,
            scan_coverage: None,
        },
        outcome: anvil_gctx_types::FindCallersOutcome::Unavailable,
    }
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("find_callers payload serialises");
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

    fn target() -> Value {
        json!({ "file": "src/a.ts", "kind": "Function", "name": "handle", "ordinal": 0 })
    }

    #[test]
    fn descriptor_advertises_tool_name() {
        assert_eq!(descriptor()["name"], TOOL_NAME);
        assert_eq!(descriptor()["annotations"]["readOnlyHint"], true);
    }

    #[test]
    fn rejects_missing_workspace_root() {
        let result = call(&json!({ "target": target() }));
        assert_eq!(result["isError"], true);
        assert_eq!(payload_of(&result)["error"], "workspaceRoot is required");
    }

    #[test]
    fn rejects_missing_target() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({ "workspaceRoot": workspace.path() }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            payload_of(&result)["error"],
            "target (with a non-empty name and file) is required"
        );
    }

    #[test]
    fn rejects_target_missing_file() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "target": { "kind": "Function", "name": "handle", "ordinal": 0 },
        }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            payload_of(&result)["error"],
            "target (with a non-empty name and file) is required"
        );
    }

    #[test]
    fn rewarm_fires_only_on_not_ready() {
        use anvil_gctx_types::{FindCallersOutcome, FindCallersProjection, RedactionSummary};

        assert!(should_rewarm(&FindCallersOutcome::NotReady {
            recovery_hint: "warming".into(),
        }));
        assert!(!should_rewarm(&FindCallersOutcome::Ready(
            FindCallersProjection {
                callers: Vec::new(),
                next_cursor: None,
                redaction_summary: RedactionSummary::default(),
                partial: false,
            }
        )));
        assert!(!should_rewarm(&FindCallersOutcome::Unavailable));
        assert!(!should_rewarm(&FindCallersOutcome::Disabled));
        assert!(!should_rewarm(&FindCallersOutcome::InvalidQuery {
            reason: "bad".into(),
        }));
    }

    #[test]
    fn degrades_to_unavailable_without_a_daemon() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "target": target(),
            "maxDepth": 2
        }));

        assert_eq!(result["isError"], false);
        let payload = payload_of(&result);
        assert_eq!(payload["outcome"]["status"], "unavailable");
        assert_eq!(payload["workspace_assurance"]["state"], "unavailable");
        assert!(payload.get("workspaceRoot").is_some());
    }
}
