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
    AssuranceState, GctxFindCallersRequest, GctxFindCallersResponse, StaleReason,
    WorkspaceAssurance,
};

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

    let response = match daemon_find_callers(&request) {
        Ok(response) => response,
        Err(GctxDaemonError::Unavailable) => unavailable_response(),
        Err(GctxDaemonError::Failure) => {
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

/// Why a daemon GCTX request could not complete. `Unavailable` (socket absent /
/// `Method not found`) degrades to a structured `unavailable` outcome; `Failure`
/// (a malformed reply, an IO error mid-exchange) is a tool error.
#[cfg_attr(not(unix), allow(dead_code))]
enum GctxDaemonError {
    Unavailable,
    Failure,
}

#[cfg(unix)]
fn daemon_find_callers(
    request: &GctxFindCallersRequest,
) -> Result<GctxFindCallersResponse, GctxDaemonError> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const TIMEOUT: Duration = Duration::from_secs(2);
    // An identity-only caller page is small (identities + distances). 4 MiB is a
    // generous malformed-response cap, above any honest reply.
    const RESPONSE_LINE_CAP: u64 = 4 << 20;
    const REQUEST_ID: &str = "mcp-gctx-find-callers";

    let socket_path = ipc::resolve_socket_path().map_err(|_| GctxDaemonError::Unavailable)?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
        return match err {
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(GctxDaemonError::Unavailable)
            }
            _ => {
                eprintln!("anvil-mcp: gctx find_callers socket unavailable: {err}");
                Err(GctxDaemonError::Failure)
            }
        };
    }
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        eprintln!("anvil-mcp: gctx find_callers connect failed: {err}");
        GctxDaemonError::Unavailable
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        eprintln!("anvil-mcp: gctx find_callers peer rejected: {err}");
        GctxDaemonError::Failure
    })?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx find_callers read-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx find_callers write-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;

    let mut frame = json!({
        "jsonrpc": "2.0",
        "method": anvil_intercept_proto::protocol::ANVIL_GCTX_FIND_CALLERS,
        "params": request,
        "id": REQUEST_ID,
    });
    // USAGE-004: attach the caller's salted-hash principal so the daemon
    // records an attributable `command.invoked` row.
    crate::usage::attach_principal(&mut frame);
    if let Err(err) = writeln!(stream, "{frame}").and_then(|()| stream.flush()) {
        eprintln!("anvil-mcp: gctx find_callers request write failed: {err}");
        return Err(GctxDaemonError::Failure);
    }

    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_CAP + 1)
        .read_until(b'\n', &mut line)
        .map_err(|err| {
            eprintln!("anvil-mcp: gctx find_callers response read failed: {err}");
            GctxDaemonError::Failure
        })?;
    if read == 0 || line.len() as u64 > RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
        eprintln!("anvil-mcp: gctx find_callers response was empty, oversized, or unframed");
        return Err(GctxDaemonError::Failure);
    }
    let line = String::from_utf8(line).map_err(|_| {
        eprintln!("anvil-mcp: gctx find_callers response was not UTF-8");
        GctxDaemonError::Failure
    })?;

    let envelope: GctxRpcEnvelope = serde_json::from_str(&line).map_err(|err| {
        eprintln!("anvil-mcp: gctx find_callers response parse failed: {err}");
        GctxDaemonError::Failure
    })?;
    if envelope.id.as_deref() != Some(REQUEST_ID) {
        eprintln!("anvil-mcp: gctx find_callers response id mismatch");
        return Err(GctxDaemonError::Failure);
    }
    if let Some(error) = envelope.error {
        return if error.code == -32601 {
            Err(GctxDaemonError::Unavailable)
        } else {
            eprintln!("anvil-mcp: gctx find_callers daemon error {}", error.code);
            Err(GctxDaemonError::Failure)
        };
    }
    envelope.result.ok_or_else(|| {
        eprintln!("anvil-mcp: gctx find_callers response carried neither result nor error");
        GctxDaemonError::Failure
    })
}

#[cfg(not(unix))]
fn daemon_find_callers(
    _request: &GctxFindCallersRequest,
) -> Result<GctxFindCallersResponse, GctxDaemonError> {
    // The Windows named-pipe GCTX client is a future item (shared with the rest
    // of the GCTX tool suite — find_dependents, impact_of_change, …). Until it
    // lands, degrade to a structured `unavailable` outcome. Logged at debug so the
    // Windows cross-matrix run shows an actionable line instead of a silent
    // unavailable (council PL-5); PII-free — no request fields.
    tracing::debug!(
        target: "anvil_mcp::gctx",
        tool = "find_callers",
        "GCTX daemon client unavailable on non-unix (named-pipe transport pending)"
    );
    Err(GctxDaemonError::Unavailable)
}

#[cfg(unix)]
#[derive(serde::Deserialize)]
struct GctxRpcEnvelope {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    result: Option<GctxFindCallersResponse>,
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
