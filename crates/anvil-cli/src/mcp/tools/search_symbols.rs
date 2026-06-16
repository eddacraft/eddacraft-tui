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
    AssuranceState, GctxSearchSymbolsRequest, GctxSearchSymbolsResponse, StaleReason,
    WorkspaceAssurance,
};

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

    let response = match daemon_search(&request) {
        Ok(response) => response,
        Err(GctxDaemonError::Unavailable) => unavailable_response(),
        Err(GctxDaemonError::Failure) => {
            return Err("graph-context daemon request failed".to_string());
        }
    };

    Ok(render_response(&response, &redacted_workspace_root))
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

/// Why a daemon GCTX request could not complete. `Unavailable` (socket absent /
/// `Method not found`) degrades to a structured `unavailable` outcome; `Failure`
/// (a malformed reply, an IO error mid-exchange) is a tool error.
#[cfg_attr(not(unix), allow(dead_code))]
enum GctxDaemonError {
    Unavailable,
    // Constructed by the Unix socket client; non-Unix builds currently degrade
    // before opening a daemon transport.
    Failure,
}

#[cfg(unix)]
fn daemon_search(
    request: &GctxSearchSymbolsRequest,
) -> Result<GctxSearchSymbolsResponse, GctxDaemonError> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const TIMEOUT: Duration = Duration::from_secs(2);
    // The response is the sealed projection: at MAX_PAGE_LIMIT (200) identity
    // summaries it is ~tens of KiB. 4 MiB is a generous malformed-response cap,
    // sized above any honest reply rather than to a precise bound.
    const RESPONSE_LINE_CAP: u64 = 4 << 20;
    const REQUEST_ID: &str = "mcp-gctx-search";

    let socket_path = ipc::resolve_socket_path().map_err(|_| GctxDaemonError::Unavailable)?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
        eprintln!("anvil-mcp: gctx search socket unavailable: {err}");
        return match err {
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(GctxDaemonError::Unavailable)
            }
            _ => Err(GctxDaemonError::Failure),
        };
    }
    // A refused connection on an existing socket (daemon crashed / restarting)
    // is treated as `Unavailable` — GCTX is daemon-required and degrades, so any
    // absence of a live daemon is a graceful state, not a tool error.
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        eprintln!("anvil-mcp: gctx search connect failed: {err}");
        GctxDaemonError::Unavailable
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        eprintln!("anvil-mcp: gctx search peer rejected: {err}");
        GctxDaemonError::Failure
    })?;
    // A failed timeout setup must NOT proceed to an unbounded blocking read.
    stream.set_read_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx search read-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx search write-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;

    let frame = json!({
        "jsonrpc": "2.0",
        "method": anvil_intercept_proto::protocol::ANVIL_GCTX_SEARCH_SYMBOLS,
        "params": request,
        "id": REQUEST_ID,
    });
    if let Err(err) = writeln!(stream, "{frame}").and_then(|()| stream.flush()) {
        eprintln!("anvil-mcp: gctx search request write failed: {err}");
        return Err(GctxDaemonError::Failure);
    }

    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_CAP + 1)
        .read_until(b'\n', &mut line)
        .map_err(|err| {
            eprintln!("anvil-mcp: gctx search response read failed: {err}");
            GctxDaemonError::Failure
        })?;
    if read == 0 || line.len() as u64 > RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
        eprintln!("anvil-mcp: gctx search response was empty, oversized, or unframed");
        return Err(GctxDaemonError::Failure);
    }
    let line = String::from_utf8(line).map_err(|_| {
        eprintln!("anvil-mcp: gctx search response was not UTF-8");
        GctxDaemonError::Failure
    })?;

    let envelope: GctxRpcEnvelope = serde_json::from_str(&line).map_err(|err| {
        eprintln!("anvil-mcp: gctx search response parse failed: {err}");
        GctxDaemonError::Failure
    })?;
    // Correlate the reply to our request (the socket is one-shot per call today,
    // but verifying the id keeps the contract explicit if that ever changes).
    if envelope.id.as_deref() != Some(REQUEST_ID) {
        eprintln!("anvil-mcp: gctx search response id mismatch");
        return Err(GctxDaemonError::Failure);
    }
    if let Some(error) = envelope.error {
        // `Method not found` ⇒ the daemon has no GCTX surface (older build, or
        // no save-time state) ⇒ degrade to Unavailable, not a hard failure.
        return if error.code == -32601 {
            Err(GctxDaemonError::Unavailable)
        } else {
            eprintln!("anvil-mcp: gctx search daemon error {}", error.code);
            Err(GctxDaemonError::Failure)
        };
    }
    envelope.result.ok_or(GctxDaemonError::Failure)
}

#[cfg(not(unix))]
fn daemon_search(
    _request: &GctxSearchSymbolsRequest,
) -> Result<GctxSearchSymbolsResponse, GctxDaemonError> {
    // The Windows named-pipe GCTX client is a future item (mirrors the DSV
    // save-time Windows gap). Until it lands, degrade to `unavailable` rather
    // than fabricate results.
    Err(GctxDaemonError::Unavailable)
}

#[cfg(unix)]
#[derive(serde::Deserialize)]
struct GctxRpcEnvelope {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    result: Option<GctxSearchSymbolsResponse>,
    #[serde(default)]
    error: Option<GctxRpcError>,
}

#[cfg(unix)]
#[derive(serde::Deserialize)]
struct GctxRpcError {
    code: i64,
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
    fn degrades_to_unavailable_without_a_daemon() {
        // No daemon is running in the test environment, so the tool must return
        // a structured `unavailable` outcome — never a file read, never an error.
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({ "workspaceRoot": workspace.path() }));

        assert_eq!(result["isError"], false);
        let payload = payload_of(&result);
        assert_eq!(payload["outcome"]["status"], "unavailable");
        assert_eq!(payload["workspace_assurance"]["state"], "unavailable");
        // The redacted relative workspace root is echoed.
        assert!(payload.get("workspaceRoot").is_some());
    }

    #[test]
    fn accepts_an_opaque_cursor_argument() {
        // A `cursor` string must parse through to the query (not be dropped or
        // rejected). Without a daemon it still degrades to `unavailable`, but the
        // key assertion is that it is NOT a parse error.
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "cursor": "deadbeef"
        }));
        assert_eq!(result["isError"], false);
        assert_eq!(payload_of(&result)["outcome"]["status"], "unavailable");
    }
}
