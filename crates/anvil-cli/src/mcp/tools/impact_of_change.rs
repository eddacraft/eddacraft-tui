//! `anvil_impact_of_change` MCP tool (GCTX-012 / ADR-084).
//!
//! Given a set of **changed file paths**, return the local blast radius as one
//! structured, identity-only `ImpactReport`: the symbols defined in the changed
//! files (affected surface), the depth-bounded dependent-file closure (what
//! imports them), and a best-effort heuristic list of known test files — so an
//! assistant reasons about impact without chaining many `find_dependents` calls.
//!
//! Like the sibling GCTX tools, this holds **no graph**: it validates the
//! workspace root (CE-8 client-side), forwards a sealed query to the running
//! `anvil-intercept` daemon over `anvil/gctx/impact_of_change`, and returns the
//! daemon-projected sealed DTO verbatim. It links only `anvil-gctx-types`
//! (graph-free), so it is structurally incapable of emitting a graph internal
//! (CE-5). Input is **paths only** — never diff content (CE-6). Daemon-required
//! and degrades gracefully (CE-7).

use std::path::Path;

use serde_json::{Value, json};

use anvil_intercept_proto::protocol::{
    AssuranceState, GctxImpactOfChangeRequest, GctxImpactOfChangeResponse, StaleReason,
    WorkspaceAssurance,
};

use crate::mcp::tools::shared::{redact_workspace_root, validate_workspace_root};

pub const TOOL_NAME: &str = "anvil_impact_of_change";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Report the blast radius of a change set over the workspace's dependency graph. Given changed file PATHS (never diff content; ≤200 files), returns a deterministic, identity-only ImpactReport: affected_symbols (symbols defined in the changed files), dependent_files (the depth-bounded set of files that import them, with hop distance), and known_tests (a best-effort heuristic subset of the dependents — use anvil_affected_tests for rigorous coverage). Requires the anvil daemon to be running; returns a structured `unavailable`/`not_ready`/`disabled` outcome while the graph is absent, warming, or an operator has switched the surface off (`ANVIL_GCTX_EGRESS=0`).",
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
                    "description": "Reverse-impact traversal depth for the dependent closure: 1 (direct importers) or 2 (transitive). Clamped server-side; absent defaults to 1.",
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
    let payload = match impact_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn impact_payload(arguments: &Value) -> Result<Value, String> {
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

    let request = GctxImpactOfChangeRequest {
        workspace_root: workspace_path.to_string_lossy().into_owned(),
        query: anvil_gctx_types::ImpactQuery {
            changed_files,
            // An out-of-`u32`-range depth saturates to `u32::MAX`, which the
            // daemon clamps down to the GV2-026 ceiling — never a wrap.
            max_depth: arguments
                .get("maxDepth")
                .and_then(Value::as_u64)
                .map(|d| u32::try_from(d).unwrap_or(u32::MAX)),
        },
    };

    let response = match daemon_impact(&request) {
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

/// Whether an impact outcome warrants an on-demand re-warm (GCTX-010 C1). Only a
/// `NotReady` graph benefits. Exhaustive match so a future
/// [`ImpactOutcome`](anvil_gctx_types::ImpactOutcome) variant forces a compile
/// error here rather than silently defaulting.
fn should_rewarm(outcome: &anvil_gctx_types::ImpactOutcome) -> bool {
    use anvil_gctx_types::ImpactOutcome as Outcome;
    match outcome {
        Outcome::NotReady { .. } => true,
        Outcome::Ready(_)
        | Outcome::Unavailable
        | Outcome::Disabled
        | Outcome::InvalidQuery { .. } => false,
    }
}

/// Merge the sealed daemon response with the redacted workspace-root echo.
fn render_response(response: &GctxImpactOfChangeResponse, redacted_workspace_root: &str) -> Value {
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
fn unavailable_response() -> GctxImpactOfChangeResponse {
    GctxImpactOfChangeResponse {
        workspace_assurance: WorkspaceAssurance {
            state: AssuranceState::Unavailable,
            reason: Some(StaleReason::DaemonAbsent),
            generation: 0,
            last_full_scan: None,
            scan_coverage: None,
        },
        outcome: anvil_gctx_types::ImpactOutcome::Unavailable,
    }
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("impact_of_change payload serialises");
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
fn daemon_impact(
    request: &GctxImpactOfChangeRequest,
) -> Result<GctxImpactOfChangeResponse, GctxDaemonError> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const TIMEOUT: Duration = Duration::from_secs(2);
    // The report is identity-only and bounded (≤200 input × depth-capped
    // closure). 4 MiB is a generous malformed-response cap, above any honest reply.
    const RESPONSE_LINE_CAP: u64 = 4 << 20;
    const REQUEST_ID: &str = "mcp-gctx-impact";

    let socket_path = ipc::resolve_socket_path().map_err(|_| GctxDaemonError::Unavailable)?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
        return match err {
            // The socket simply not existing is the routine "daemon not running"
            // case — stay silent so a daemon-less MCP session does not spam stderr.
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(GctxDaemonError::Unavailable)
            }
            _ => {
                eprintln!("anvil-mcp: gctx impact socket unavailable: {err}");
                Err(GctxDaemonError::Failure)
            }
        };
    }
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        eprintln!("anvil-mcp: gctx impact connect failed: {err}");
        GctxDaemonError::Unavailable
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        eprintln!("anvil-mcp: gctx impact peer rejected: {err}");
        GctxDaemonError::Failure
    })?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx impact read-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx impact write-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;

    let frame = json!({
        "jsonrpc": "2.0",
        "method": anvil_intercept_proto::protocol::ANVIL_GCTX_IMPACT_OF_CHANGE,
        "params": request,
        "id": REQUEST_ID,
    });
    if let Err(err) = writeln!(stream, "{frame}").and_then(|()| stream.flush()) {
        eprintln!("anvil-mcp: gctx impact request write failed: {err}");
        return Err(GctxDaemonError::Failure);
    }

    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_CAP + 1)
        .read_until(b'\n', &mut line)
        .map_err(|err| {
            eprintln!("anvil-mcp: gctx impact response read failed: {err}");
            GctxDaemonError::Failure
        })?;
    if read == 0 || line.len() as u64 > RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
        eprintln!("anvil-mcp: gctx impact response was empty, oversized, or unframed");
        return Err(GctxDaemonError::Failure);
    }
    let line = String::from_utf8(line).map_err(|_| {
        eprintln!("anvil-mcp: gctx impact response was not UTF-8");
        GctxDaemonError::Failure
    })?;

    let envelope: GctxRpcEnvelope = serde_json::from_str(&line).map_err(|err| {
        eprintln!("anvil-mcp: gctx impact response parse failed: {err}");
        GctxDaemonError::Failure
    })?;
    if envelope.id.as_deref() != Some(REQUEST_ID) {
        eprintln!("anvil-mcp: gctx impact response id mismatch");
        return Err(GctxDaemonError::Failure);
    }
    if let Some(error) = envelope.error {
        return if error.code == -32601 {
            Err(GctxDaemonError::Unavailable)
        } else {
            eprintln!("anvil-mcp: gctx impact daemon error {}", error.code);
            Err(GctxDaemonError::Failure)
        };
    }
    envelope.result.ok_or_else(|| {
        eprintln!("anvil-mcp: gctx impact response carried neither result nor error");
        GctxDaemonError::Failure
    })
}

#[cfg(not(unix))]
fn daemon_impact(
    _request: &GctxImpactOfChangeRequest,
) -> Result<GctxImpactOfChangeResponse, GctxDaemonError> {
    // The Windows named-pipe GCTX client is a future item. Until it lands,
    // degrade to `unavailable`.
    Err(GctxDaemonError::Unavailable)
}

#[cfg(unix)]
#[derive(serde::Deserialize)]
struct GctxRpcEnvelope {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    result: Option<GctxImpactOfChangeResponse>,
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
        use anvil_gctx_types::{ImpactOutcome, ImpactReport, ImpactSummary};

        assert!(should_rewarm(&ImpactOutcome::NotReady {
            recovery_hint: "warming".into(),
        }));
        assert!(!should_rewarm(&ImpactOutcome::Ready(ImpactReport {
            affected_symbols: Vec::new(),
            dependent_files: Vec::new(),
            known_tests: Vec::new(),
            summary: ImpactSummary::default(),
        })));
        assert!(!should_rewarm(&ImpactOutcome::Unavailable));
        assert!(!should_rewarm(&ImpactOutcome::Disabled));
        assert!(!should_rewarm(&ImpactOutcome::InvalidQuery {
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
