//! `anvil_symbol_context` MCP tool (GCTX-023 / ADR-084).
//!
//! Bounded symbol-context slice for AI assistants: search + local impact +
//! snippet extraction under a token budget. This tool holds **no graph** — it
//! validates the workspace root (CE-8 client-side), forwards a sealed query to
//! the running `anvil-intercept` daemon over `anvil/gctx/symbol_context`, and
//! returns the daemon-projected sealed DTO verbatim. Source text rides only when
//! the operator has opted in for the workspace (`anvil gctx egress enable`, or
//! `ANVIL_GCTX_EGRESS=1` as a process override) **and** the request asserts
//! `includeSource` (CE-1); otherwise span-as-location only.

use std::path::Path;

use serde_json::{Value, json};

use anvil_gctx_types::{EgressSource, GCTX_EGRESS_ENV, SnippetEgress, resolve_snippet_egress};
use anvil_intercept::egress_consent::read_snippet_consent;
use anvil_intercept_proto::protocol::{
    AssuranceState, GctxSymbolContextRequest, GctxSymbolContextResponse, StaleReason,
    WorkspaceAssurance,
};

use crate::mcp::tools::shared::{redact_workspace_root, validate_workspace_root};

pub const TOOL_NAME: &str = "anvil_symbol_context";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Build bounded symbol context around a seed symbol or file: neighbourhood symbols, one-hop importers, and (for symbol seeds) direct callers, each with span-as-location and optional source snippets under a token budget. Source text is returned only when an operator has enabled snippet egress for the workspace (run `anvil gctx egress enable`; or set `ANVIL_GCTX_EGRESS=1` as a process override) AND the request sets `includeSource: true`; otherwise identity-only locations. When source is requested while egress is off, the response carries a `snippetEgressHint` string describing how to enable it. Requires the anvil daemon; returns a structured `unavailable`/`not_ready`/`disabled`/`bounded` outcome while the graph is absent, warming, budget-limited, or switched off (`ANVIL_GCTX_EGRESS=0`).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                },
                "target": {
                    "type": "object",
                    "description": "Seed symbol identity (workspace-root-relative file, kind, name, optional ordinal).",
                    "properties": {
                        "file": { "type": "string" },
                        "kind": {
                            "type": "string",
                            "enum": ["Function", "Class", "Module", "Export", "Interface", "TypeAlias", "Enum", "Method"]
                        },
                        "name": { "type": "string" },
                        "ordinal": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["file", "kind", "name"]
                },
                "file": {
                    "type": "string",
                    "description": "Alternative seed: workspace-root-relative file path (all span-bearing symbols in the file)."
                },
                "tokenBudget": {
                    "type": "integer",
                    "description": "Maximum estimated snippet tokens to return (clamped server-side).",
                    "minimum": 1
                },
                "includeSource": {
                    "type": "boolean",
                    "description": "Capability assertion — request source text. Honoured only when an operator has enabled snippet egress for the workspace (`anvil gctx egress enable`, or `ANVIL_GCTX_EGRESS=1`); otherwise the response is identity-only with a `snippetEgressHint`."
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
    let payload = match symbol_context_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn symbol_context_payload(arguments: &Value) -> Result<Value, String> {
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
    let request = GctxSymbolContextRequest {
        workspace_root: workspace_path.to_string_lossy().into_owned(),
        query,
    };

    let response = match daemon_symbol_context(&request) {
        Ok(response) => response,
        Err(GctxDaemonError::Unavailable) => unavailable_response(),
        Err(GctxDaemonError::Failure) => {
            return Err("graph-context daemon request failed".to_string());
        }
    };

    if should_rewarm(&response.outcome) {
        let _ = crate::commands::watch_save_time::warm_up_root(&workspace_path);
    }

    let mut payload = render_response(&response, &redacted_workspace_root);
    // GCTX-024 discoverable degradation: if the assistant asked for source text
    // but egress is off, tell it (and the operator) exactly how to enable it —
    // rather than silently returning identity-only locations.
    if let Some(hint) = egress_hint_for_request(arguments, &workspace_path)
        && let Some(object) = payload.as_object_mut()
    {
        object.insert("snippetEgressHint".to_string(), Value::String(hint));
    }

    Ok(payload)
}

/// Build the discoverable-degradation hint for a `symbol_context` call, if one is
/// warranted: only when `includeSource` was requested AND snippet egress resolves
/// off for this workspace. Reuses the same env+consent resolver the daemon uses,
/// so the hint never contradicts the actual decision (and a kill-switched `0`
/// gets a different, accurate message from a never-enabled default).
fn egress_hint_for_request(arguments: &Value, workspace_path: &Path) -> Option<String> {
    let include_source = arguments
        .get("includeSource")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let env_raw = std::env::var(GCTX_EGRESS_ENV).ok();
    // A consent-read error fails safe to "no persisted consent" for the hint —
    // the daemon already logged it; here we only choose advisory wording.
    let persisted = read_snippet_consent(workspace_path).unwrap_or(None);
    let (decision, source) = resolve_snippet_egress(env_raw.as_deref(), persisted);
    snippet_egress_hint(include_source, decision, source)
}

/// Pure hint selector. `None` when no hint is warranted (egress enabled, or the
/// caller did not ask for source).
fn snippet_egress_hint(
    include_source: bool,
    decision: SnippetEgress,
    source: EgressSource,
) -> Option<String> {
    if !include_source || matches!(decision, SnippetEgress::Enabled) {
        return None;
    }
    Some(match source {
        EgressSource::Env => "Graph context is disabled by ANVIL_GCTX_EGRESS=0 \
            (kill-switch). Unset that variable, then an operator can enable source-text snippets \
            with `anvil gctx egress enable`."
            .to_string(),
        EgressSource::Config | EgressSource::Default => "Source-text snippets are off for this \
            workspace (identity-only). An operator can enable them with \
            `anvil gctx egress enable`."
            .to_string(),
    })
}

fn should_rewarm(outcome: &anvil_gctx_types::SymbolContextOutcome) -> bool {
    use anvil_gctx_types::SymbolContextOutcome as Outcome;
    match outcome {
        Outcome::NotReady { .. } => true,
        Outcome::Ready(_)
        | Outcome::Bounded(_)
        | Outcome::BudgetExceeded(_)
        | Outcome::Unavailable
        | Outcome::Disabled
        | Outcome::InvalidQuery { .. } => false,
    }
}

fn parse_query(arguments: &Value) -> Result<anvil_gctx_types::SymbolContextQuery, String> {
    let has_target = arguments.get("target").is_some();
    let has_file = arguments
        .get("file")
        .and_then(Value::as_str)
        .is_some_and(|f| !f.is_empty());
    if has_target == has_file {
        return Err(
            "exactly one of `target` (symbol seed) or `file` (file seed) is required".to_string(),
        );
    }

    let mut fields = serde_json::Map::new();
    if let Some(target) = arguments.get("target") {
        let mut target = target.clone();
        if let Some(object) = target.as_object_mut() {
            object
                .entry("ordinal".to_string())
                .or_insert_with(|| json!(0));
        }
        fields.insert("selector".to_string(), json!({ "symbol": target }));
    } else if let Some(file) = arguments.get("file").and_then(Value::as_str) {
        fields.insert("selector".to_string(), json!({ "file": { "file": file } }));
    }
    for key in ["tokenBudget", "includeSource"] {
        if let Some(value) = arguments.get(key)
            && !value.is_null()
        {
            let wire_key = if key == "tokenBudget" {
                "token_budget"
            } else {
                "include_source"
            };
            fields.insert(wire_key.to_string(), value.clone());
        }
    }
    serde_json::from_value(Value::Object(fields))
        .map_err(|err| format!("invalid symbol_context parameter: {err}"))
}

fn render_response(response: &GctxSymbolContextResponse, redacted_workspace_root: &str) -> Value {
    let mut value = serde_json::to_value(response).expect("gctx response serialises");
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "workspaceRoot".to_string(),
            Value::String(redacted_workspace_root.to_string()),
        );
    }
    value
}

fn unavailable_response() -> GctxSymbolContextResponse {
    GctxSymbolContextResponse {
        workspace_assurance: WorkspaceAssurance {
            state: AssuranceState::Unavailable,
            reason: Some(StaleReason::DaemonAbsent),
            generation: 0,
            last_full_scan: None,
            scan_coverage: None,
        },
        outcome: anvil_gctx_types::SymbolContextOutcome::Unavailable,
    }
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("symbol_context payload serialises");
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

#[cfg_attr(not(unix), allow(dead_code))]
enum GctxDaemonError {
    Unavailable,
    Failure,
}

#[cfg(unix)]
fn daemon_symbol_context(
    request: &GctxSymbolContextRequest,
) -> Result<GctxSymbolContextResponse, GctxDaemonError> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const TIMEOUT: Duration = Duration::from_secs(2);
    const RESPONSE_LINE_CAP: u64 = 4 << 20;
    const REQUEST_ID: &str = "mcp-gctx-symbol-context";

    let socket_path = ipc::resolve_socket_path().map_err(|_| GctxDaemonError::Unavailable)?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
        eprintln!("anvil-mcp: gctx symbol_context socket unavailable: {err}");
        return match err {
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(GctxDaemonError::Unavailable)
            }
            _ => Err(GctxDaemonError::Failure),
        };
    }
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        eprintln!("anvil-mcp: gctx symbol_context connect failed: {err}");
        GctxDaemonError::Unavailable
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        eprintln!("anvil-mcp: gctx symbol_context peer rejected: {err}");
        GctxDaemonError::Failure
    })?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx symbol_context read-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx symbol_context write-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;

    let mut frame = json!({
        "jsonrpc": "2.0",
        "method": anvil_intercept_proto::protocol::ANVIL_GCTX_SYMBOL_CONTEXT,
        "params": request,
        "id": REQUEST_ID,
    });
    crate::usage::attach_principal(&mut frame);
    if let Err(err) = writeln!(stream, "{frame}").and_then(|()| stream.flush()) {
        eprintln!("anvil-mcp: gctx symbol_context request write failed: {err}");
        return Err(GctxDaemonError::Failure);
    }

    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_CAP + 1)
        .read_until(b'\n', &mut line)
        .map_err(|err| {
            eprintln!("anvil-mcp: gctx symbol_context response read failed: {err}");
            GctxDaemonError::Failure
        })?;
    if read == 0 || line.len() as u64 > RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
        eprintln!("anvil-mcp: gctx symbol_context response was empty, oversized, or unframed");
        return Err(GctxDaemonError::Failure);
    }
    let line = String::from_utf8(line).map_err(|_| {
        eprintln!("anvil-mcp: gctx symbol_context response was not UTF-8");
        GctxDaemonError::Failure
    })?;

    let envelope: GctxRpcEnvelope = serde_json::from_str(&line).map_err(|err| {
        eprintln!("anvil-mcp: gctx symbol_context response parse failed: {err}");
        GctxDaemonError::Failure
    })?;
    if envelope.id.as_deref() != Some(REQUEST_ID) {
        eprintln!("anvil-mcp: gctx symbol_context response id mismatch");
        return Err(GctxDaemonError::Failure);
    }
    if let Some(error) = envelope.error {
        return if error.code == -32601 {
            Err(GctxDaemonError::Unavailable)
        } else {
            eprintln!("anvil-mcp: gctx symbol_context daemon error {}", error.code);
            Err(GctxDaemonError::Failure)
        };
    }
    envelope.result.ok_or(GctxDaemonError::Failure)
}

#[cfg(not(unix))]
fn daemon_symbol_context(
    _request: &GctxSymbolContextRequest,
) -> Result<GctxSymbolContextResponse, GctxDaemonError> {
    Err(GctxDaemonError::Unavailable)
}

#[cfg(unix)]
#[derive(serde::Deserialize)]
struct GctxRpcEnvelope {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    result: Option<GctxSymbolContextResponse>,
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
        let result = call(&json!({ "file": "src/a.ts" }));
        assert_eq!(result["isError"], true);
        assert_eq!(payload_of(&result)["error"], "workspaceRoot is required");
    }

    #[test]
    fn rejects_neither_nor_both_seeds() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let missing = call(&json!({ "workspaceRoot": workspace.path() }));
        assert_eq!(missing["isError"], true);
        assert!(
            payload_of(&missing)["error"]
                .as_str()
                .unwrap()
                .contains("exactly one of")
        );

        let both = call(&json!({
            "workspaceRoot": workspace.path(),
            "file": "src/a.ts",
            "target": { "file": "src/a.ts", "kind": "Function", "name": "f" }
        }));
        assert_eq!(both["isError"], true);
    }

    #[test]
    fn rewarm_fires_only_on_not_ready() {
        use anvil_gctx_types::{
            GctxOutcome, SymbolContextOutcome, SymbolContextProjection,
            SymbolContextRedactionSummary,
        };

        assert!(should_rewarm(&SymbolContextOutcome::NotReady {
            recovery_hint: "warming".into(),
        }));
        let summary = SymbolContextRedactionSummary {
            estimated_tokens: 0,
            redacted_secrets: 0,
            snippets_truncated: 0,
            fully_suppressed_symbols: 0,
            omitted_sensitive_paths: 0,
            outcome: GctxOutcome::Hit,
        };
        let projection = SymbolContextProjection {
            snippets: Vec::new(),
            omitted_context: Vec::new(),
            redaction_summary: summary,
        };
        assert!(!should_rewarm(&SymbolContextOutcome::Ready(
            projection.clone()
        )));
        assert!(!should_rewarm(&SymbolContextOutcome::Bounded(
            projection.clone()
        )));
        assert!(!should_rewarm(&SymbolContextOutcome::BudgetExceeded(
            projection
        )));
        assert!(!should_rewarm(&SymbolContextOutcome::Unavailable));
        assert!(!should_rewarm(&SymbolContextOutcome::Disabled));
        assert!(!should_rewarm(&SymbolContextOutcome::InvalidQuery {
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
    fn accepts_symbol_seed_and_include_source_flag() {
        let cwd = std::env::current_dir().expect("cwd");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "target": {
                "file": "src/a.ts",
                "kind": "Function",
                "name": "greet",
                "ordinal": 0
            },
            "includeSource": true,
            "tokenBudget": 500
        }));
        assert_eq!(result["isError"], false);
        assert_eq!(payload_of(&result)["outcome"]["status"], "unavailable");
    }

    #[test]
    fn call_injects_snippet_egress_hint_when_source_requested_and_off() {
        // includeSource requested, no env, no consent → the payload must carry a
        // discoverable hint naming the enable command (end-to-end through `call`,
        // independent of daemon availability).
        temp_env::with_var_unset("ANVIL_GCTX_EGRESS", || {
            let cwd = std::env::current_dir().expect("cwd");
            let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
            let result = call(&json!({
                "workspaceRoot": workspace.path(),
                "file": "src/a.ts",
                "includeSource": true
            }));
            let payload = payload_of(&result);
            let hint = payload
                .get("snippetEgressHint")
                .and_then(Value::as_str)
                .expect("snippetEgressHint present when source requested but egress off");
            assert!(hint.contains("anvil gctx egress enable"), "hint: {hint}");
        });
    }

    #[test]
    fn call_omits_hint_when_source_not_requested() {
        temp_env::with_var_unset("ANVIL_GCTX_EGRESS", || {
            let cwd = std::env::current_dir().expect("cwd");
            let workspace = tempfile::tempdir_in(&cwd).expect("workspace");
            let result = call(&json!({
                "workspaceRoot": workspace.path(),
                "file": "src/a.ts"
            }));
            assert!(payload_of(&result).get("snippetEgressHint").is_none());
        });
    }

    #[test]
    fn egress_hint_only_when_source_requested_and_off() {
        // No hint when the caller did not ask for source.
        assert!(
            snippet_egress_hint(false, SnippetEgress::IdentityOnly, EgressSource::Default)
                .is_none()
        );
        // No hint when egress is enabled.
        assert!(snippet_egress_hint(true, SnippetEgress::Enabled, EgressSource::Config).is_none());
        // Default/config off → enable hint.
        let hint =
            snippet_egress_hint(true, SnippetEgress::IdentityOnly, EgressSource::Default).unwrap();
        assert!(hint.contains("anvil gctx egress enable"));
        assert!(!hint.contains("kill-switch"));
        // Kill-switch off → distinct message that names the env var.
        let killed =
            snippet_egress_hint(true, SnippetEgress::IdentityOnly, EgressSource::Env).unwrap();
        assert!(killed.contains("ANVIL_GCTX_EGRESS=0"));
        assert!(killed.contains("anvil gctx egress enable"));
    }
}
