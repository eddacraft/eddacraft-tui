//! `anvil_search_symbols` MCP tool (GCTX-010 / ADR-084).
//!
//! Identity-only symbol search for AI assistants. This tool holds **no graph**:
//! it validates the workspace root (CE-8 client-side), forwards a sealed query
//! to the running `anvil-intercept` daemon over `anvil/gctx/search_symbols`, and
//! returns the daemon-projected sealed DTO verbatim. It links only
//! `anvil-gctx-types` (graph-free), so it is structurally incapable of emitting
//! a graph internal (CE-5).
//!
//! Daemon-required, degrade gracefully (CE-7): when the daemon is absent or has
//! no GCTX surface, the tool returns a structured `unavailable` outcome — never
//! a file read and never an empty-looking success.

use std::path::Path;

use serde_json::{Value, json};

use anvil_intercept_proto::protocol::{
    ANVIL_GCTX_SEARCH_SYMBOLS, AssuranceState, GctxSearchSymbolsRequest, GctxSearchSymbolsResponse,
    StaleReason, WorkspaceAssurance,
};

use crate::mcp::gctx_client::{DaemonRpcError, daemon_rpc_call};
use crate::mcp::tools::shared::{redact_workspace_root, validate_workspace_root};

pub const TOOL_NAME: &str = "anvil_search_symbols";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Search the workspace's semantic graph for symbols by name, kind, file, language, or visibility. Returns paginated, deterministic, identity-only summaries (symbol identity + visibility — no source text). Requires the anvil daemon to be running; returns a structured `unavailable`/`not_ready`/`disabled` outcome while the graph is absent, warming, or an operator has switched the surface off (`ANVIL_GCTX_EGRESS=0`).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                },
                "name": {
                    "type": "string",
                    "description": "Case-insensitive substring match on the symbol name"
                },
                "kind": {
                    "type": "string",
                    "description": "Exact structural kind",
                    "enum": ["Function", "Class", "Module", "Export", "Interface", "TypeAlias", "Enum", "Method"]
                },
                "file": {
                    "type": "string",
                    "description": "Case-insensitive substring match on the workspace-root-relative path"
                },
                "language": {
                    "type": "string",
                    "description": "Language token derived from the file extension",
                    "enum": ["typescript", "javascript", "rust"]
                },
                "visibility": {
                    "type": "string",
                    "description": "Exact visibility",
                    "enum": ["Public", "Internal"]
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum summaries to return (clamped server-side)",
                    "minimum": 1
                },
                "cursor": {
                    "type": "string",
                    "description": "Opaque pagination cursor from a previous response's `next_cursor`. Echo it back verbatim to fetch the next page; treat it as an opaque token (never construct one)."
                }
            },
            "required": ["workspaceRoot"],
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
    let payload = match search_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn search_payload(arguments: &Value) -> Result<Value, String> {
    let server_root = std::env::current_dir()
        .map_err(|err| format!("MCP server cwd is not accessible: {err}"))?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot is required".to_string())?;
    let (server_root, workspace_path) =
        validate_workspace_root(Path::new(workspace_root), &server_root)?;
    let redacted_workspace_root = redact_workspace_root(&workspace_path, &server_root);

    let query = parse_query(arguments)?;
    let request = GctxSearchSymbolsRequest {
        workspace_root: workspace_path.to_string_lossy().into_owned(),
        query,
    };

    let response = match daemon_rpc_call(ANVIL_GCTX_SEARCH_SYMBOLS, &request, "mcp-gctx-search") {
        Ok(response) => response,
        Err(DaemonRpcError::Unavailable) => unavailable_response(),
        Err(DaemonRpcError::Failure) => {
            return Err("graph-context daemon request failed".to_string());
        }
    };

    // GCTX-010 C1 (ADR-085) on-demand re-warm: a `NotReady` graph (cold, or
    // warming-but-not-yet-populated) is the one outcome a retry can recover
    // from, so enqueue a full scan to make the *next* query more likely to
    // succeed. Best-effort and fire-and-forget; the daemon-side executor
    // (DSV-045) drives and coalesces the scan, so firing on every miss is safe.
    if should_rewarm(&response.outcome) {
        let _ = crate::commands::watch_save_time::warm_up_root(&workspace_path);
    }

    Ok(render_response(&response, &redacted_workspace_root))
}

/// Whether a search outcome warrants an on-demand re-warm (GCTX-010 C1). Only a
/// `NotReady` graph benefits: `Ready` is already populated, `Unavailable` has no
/// live daemon to enqueue against, `Disabled` is an operator switch we must not
/// fight (`ANVIL_GCTX_EGRESS=0`), and `InvalidQuery` is the caller's bug.
///
/// Written as an exhaustive match (not `matches!`) so a future
/// [`SearchSymbolsOutcome`](anvil_gctx_types::SearchSymbolsOutcome) variant —
/// e.g. a Phase-2 `Bounded`/budget state — forces a compile error here rather
/// than silently defaulting to "do not re-warm".
fn should_rewarm(outcome: &anvil_gctx_types::SearchSymbolsOutcome) -> bool {
    use anvil_gctx_types::SearchSymbolsOutcome as Outcome;
    match outcome {
        Outcome::NotReady { .. } => true,
        Outcome::Ready(_)
        | Outcome::Unavailable
        | Outcome::Disabled
        | Outcome::InvalidQuery { .. } => false,
    }
}

/// Build a [`SearchSymbolsQuery`](anvil_gctx_types::SearchSymbolsQuery) from the
/// MCP arguments by deserialising only the recognised filter fields. An
/// unparseable filter (e.g. an unknown `kind`) is a tool error.
fn parse_query(arguments: &Value) -> Result<anvil_gctx_types::SearchSymbolsQuery, String> {
    let mut fields = serde_json::Map::new();
    for key in [
        "name",
        "kind",
        "file",
        "language",
        "visibility",
        "limit",
        "cursor",
    ] {
        if let Some(value) = arguments.get(key)
            && !value.is_null()
        {
            fields.insert(key.to_string(), value.clone());
        }
    }
    serde_json::from_value(Value::Object(fields))
        .map_err(|err| format!("invalid search parameter: {err}"))
}

/// Merge the sealed daemon response with the redacted workspace-root echo.
fn render_response(response: &GctxSearchSymbolsResponse, redacted_workspace_root: &str) -> Value {
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
fn unavailable_response() -> GctxSearchSymbolsResponse {
    GctxSearchSymbolsResponse {
        workspace_assurance: WorkspaceAssurance {
            state: AssuranceState::Unavailable,
            reason: Some(StaleReason::DaemonAbsent),
            generation: 0,
            last_full_scan: None,
            scan_coverage: None,
        },
        outcome: anvil_gctx_types::SearchSymbolsOutcome::Unavailable,
    }
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("search_symbols payload serialises");
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
        let result = call(&json!({}));
        assert_eq!(result["isError"], true);
        assert_eq!(payload_of(&result)["error"], "workspaceRoot is required");
    }

    #[test]
    fn rejects_relative_workspace_root() {
        let result = call(&json!({ "workspaceRoot": "." }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            payload_of(&result)["error"],
            "workspaceRoot must be an absolute path"
        );
    }

    #[test]
    fn rejects_unknown_kind_filter() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "kind": "NotARealKind"
        }));
        assert_eq!(result["isError"], true);
        assert!(
            payload_of(&result)["error"]
                .as_str()
                .unwrap()
                .contains("invalid search parameter")
        );
    }

    #[test]
    fn rewarm_fires_only_on_not_ready() {
        use anvil_gctx_types::{RedactionSummary, SearchSymbolsOutcome, SearchSymbolsProjection};

        // The one recoverable state: a warming / cold-but-unpopulated graph.
        assert!(should_rewarm(&SearchSymbolsOutcome::NotReady {
            recovery_hint: "warming".into(),
        }));

        // Every other outcome must NOT trigger a re-warm.
        assert!(!should_rewarm(&SearchSymbolsOutcome::Ready(
            SearchSymbolsProjection {
                symbols: Vec::new(),
                next_cursor: None,
                redaction_summary: RedactionSummary::default(),
            }
        )));
        assert!(!should_rewarm(&SearchSymbolsOutcome::Unavailable));
        assert!(!should_rewarm(&SearchSymbolsOutcome::Disabled));
        assert!(!should_rewarm(&SearchSymbolsOutcome::InvalidQuery {
            reason: "bad".into(),
        }));
    }

    #[test]
    fn unavailable_response_is_sealed() {
        let payload = render_response(&unavailable_response(), ".");
        assert_eq!(payload["outcome"]["status"], "unavailable");
        assert_eq!(payload["workspace_assurance"]["state"], "unavailable");
        assert_eq!(payload["workspaceRoot"], ".");
    }

    #[test]
    fn accepts_an_opaque_cursor_argument() {
        let query = parse_query(&json!({ "cursor": "deadbeef" })).expect("query parses");

        assert_eq!(
            query
                .cursor
                .as_ref()
                .map(anvil_gctx_types::OpaqueCursor::as_str),
            Some("deadbeef")
        );
    }
}
