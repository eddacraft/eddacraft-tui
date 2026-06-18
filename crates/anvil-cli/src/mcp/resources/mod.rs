//! GCTX-030 — read-only `graph://` MCP resources.
//!
//! Exposes safe, identity-only graph summaries as MCP **resources** (distinct
//! from the GCTX tools): `graph://symbols`, `graph://edges`, and `graph://stats`.
//! Like the GCTX tools, this layer holds **no graph**: it forwards a sealed
//! request to the running `anvil-intercept` daemon over the read-only
//! `anvil/gctx/*` surface and returns the daemon-projected sealed DTO. It links
//! only `anvil-gctx-types` (graph-free), so it is structurally incapable of
//! emitting a graph internal (CE-5).
//!
//! The workspace root is the **server's own cwd** — the session-pinned,
//! stdio-only root (GCTX-002 CE-8). There is no client-supplied root: a resource
//! read names only a `uri`, and pagination/filter ride in the URI query string
//! (`graph://edges?file=src/a.ts&cursor=…`). Daemon-required; degrades gracefully
//! to a structured `unavailable`/`not_ready`/`disabled` outcome (CE-7).

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use anvil_intercept_proto::protocol::{
    ANVIL_GCTX_GRAPH_EDGES, ANVIL_GCTX_GRAPH_STATS, ANVIL_GCTX_SEARCH_SYMBOLS,
    GctxGraphEdgesRequest, GctxGraphEdgesResponse, GctxGraphStatsRequest, GctxGraphStatsResponse,
    GctxSearchSymbolsRequest, GctxSearchSymbolsResponse,
};

/// `graph://symbols` — all resident symbols, identity-only (reuses the
/// `search_symbols` surface with no filters).
pub const URI_SYMBOLS: &str = "graph://symbols";
/// `graph://edges` — all resident symbol-graph edges, identity-only.
pub const URI_EDGES: &str = "graph://edges";
/// `graph://stats` — workspace-wide graph counts.
pub const URI_STATS: &str = "graph://stats";

const MIME_JSON: &str = "application/json";

/// The `resources/list` descriptors for the three `graph://` resources.
#[must_use]
pub fn list() -> Vec<Value> {
    vec![
        descriptor(
            URI_SYMBOLS,
            "Workspace symbols",
            "All resident symbols in the workspace as identity-only summaries \
             (file, kind, name, ordinal, visibility), paginated. Append \
             `?cursor=…` (echo a prior `next_cursor`) to page, or `?file=…` to \
             scope to one file. Backed by the anvil daemon's graph; returns a \
             structured `unavailable`/`not_ready`/`disabled` outcome while the \
             graph is absent, warming, or switched off (`ANVIL_GCTX_EGRESS=0`).",
        ),
        descriptor(
            URI_EDGES,
            "Workspace graph edges",
            "All resident symbol-graph edges as identity-only `(from, to, \
             edge_type)` summaries, paginated. Append `?cursor=…` to page or \
             `?file=…` to scope to edges whose source symbol is in one file. \
             Identity-only; same daemon/degradation contract as `graph://symbols`.",
        ),
        descriptor(
            URI_STATS,
            "Workspace graph statistics",
            "Counts-only summary of the workspace graph: resident symbols, \
             symbol-graph edges, files, and dependency edges. No content, no \
             names — just totals. Same daemon/degradation contract as the other \
             `graph://` resources.",
        ),
    ]
}

/// A [`read`] failure, classified for the JSON-RPC error code the dispatcher
/// returns (council CR-2): a client mistake (unknown URI / malformed query) vs a
/// server-side fault (daemon transport failure). Daemon **degradation**
/// (unavailable/warming/disabled) is NOT an error — it rides in-band in the
/// sealed outcome (CE-7).
#[derive(Debug)]
pub enum ReadError {
    /// Unknown resource URI or malformed query string → JSON-RPC `-32602`.
    BadRequest(String),
    /// Daemon transport/protocol failure or an inaccessible server cwd →
    /// JSON-RPC `-32603`.
    Internal(String),
}

impl ReadError {
    /// The reason text (for the error `data`).
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::BadRequest(reason) | Self::Internal(reason) => reason,
        }
    }
}

/// Read one `graph://` resource by `uri`, returning the MCP `resources/read`
/// result (`{ contents: [{ uri, mimeType, text }] }`).
///
/// # Errors
///
/// [`ReadError::BadRequest`] for an unknown URI or malformed query;
/// [`ReadError::Internal`] for a daemon transport failure. Daemon degradation
/// (unavailable/warming/disabled) is **not** an error — it rides in the sealed
/// outcome the resource content carries (CE-7).
pub fn read(uri: &str) -> Result<Value, ReadError> {
    let (base, query) = split_uri(uri);
    let payload = match base {
        URI_SYMBOLS => read_symbols(&query)?,
        URI_EDGES => read_edges(&query)?,
        URI_STATS => read_stats(&query)?,
        other => {
            return Err(ReadError::BadRequest(format!(
                "unknown resource uri: {other}"
            )));
        }
    };
    Ok(contents(uri, &payload))
}

fn descriptor(uri: &str, name: &str, description: &str) -> Value {
    json!({
        "uri": uri,
        "name": name,
        "description": description,
        "mimeType": MIME_JSON,
        "annotations": { "readOnlyHint": true }
    })
}

/// Wrap a sealed response payload in the MCP `resources/read` envelope.
fn contents(uri: &str, payload: &Value) -> Value {
    json!({
        "contents": [
            {
                "uri": uri,
                "mimeType": MIME_JSON,
                "text": serde_json::to_string(payload).expect("resource payload serialises"),
            }
        ]
    })
}

fn read_stats(query: &[(String, String)]) -> Result<Value, ReadError> {
    ensure_known_query_keys(query, &[])?;
    let root = workspace_root()?;
    let request = GctxGraphStatsRequest {
        workspace_root: root.clone(),
    };
    let response: GctxGraphStatsResponse =
        match gctx_call(ANVIL_GCTX_GRAPH_STATS, &request, "mcp-gctx-graph-stats") {
            Ok(response) => response,
            Err(GctxDaemonError::Unavailable) => GctxGraphStatsResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: anvil_gctx_types::GraphStatsOutcome::Unavailable,
            },
            Err(GctxDaemonError::Failure) => {
                return Err(ReadError::Internal(
                    "graph-context daemon request failed".to_string(),
                ));
            }
        };
    // ADR-085 C1 on-demand re-warm: a NotReady graph is the one outcome a retry
    // can recover from (council CR-1, mirroring the GCTX tools).
    if matches!(
        response.outcome,
        anvil_gctx_types::GraphStatsOutcome::NotReady { .. }
    ) {
        rewarm(&root);
    }
    Ok(serde_json::to_value(response).expect("gctx response serialises"))
}

fn read_symbols(query: &[(String, String)]) -> Result<Value, ReadError> {
    ensure_known_query_keys(query, &["file", "cursor", "limit"])?;
    let root = workspace_root()?;
    let mut search = anvil_gctx_types::SearchSymbolsQuery {
        file: validated_file_filter(query)?,
        ..Default::default()
    };
    if let Some(cursor) = query_value(query, "cursor") {
        search.cursor = Some(anvil_gctx_types::OpaqueCursor::new(cursor.to_string()));
    }
    if let Some(limit) = query_value(query, "limit") {
        search.limit = Some(parse_limit(limit)?);
    }
    let request = GctxSearchSymbolsRequest {
        workspace_root: root.clone(),
        query: search,
    };
    let response: GctxSearchSymbolsResponse = match gctx_call(
        ANVIL_GCTX_SEARCH_SYMBOLS,
        &request,
        "mcp-gctx-graph-symbols",
    ) {
        Ok(response) => response,
        Err(GctxDaemonError::Unavailable) => GctxSearchSymbolsResponse {
            workspace_assurance: unavailable_assurance(),
            outcome: anvil_gctx_types::SearchSymbolsOutcome::Unavailable,
        },
        Err(GctxDaemonError::Failure) => {
            return Err(ReadError::Internal(
                "graph-context daemon request failed".to_string(),
            ));
        }
    };
    if matches!(
        response.outcome,
        anvil_gctx_types::SearchSymbolsOutcome::NotReady { .. }
    ) {
        rewarm(&root);
    }
    Ok(serde_json::to_value(response).expect("gctx response serialises"))
}

fn read_edges(query: &[(String, String)]) -> Result<Value, ReadError> {
    ensure_known_query_keys(query, &["file", "cursor", "limit"])?;
    let root = workspace_root()?;
    let mut edges = anvil_gctx_types::GraphEdgesQuery {
        file: validated_file_filter(query)?,
        ..Default::default()
    };
    if let Some(cursor) = query_value(query, "cursor") {
        edges.cursor = Some(anvil_gctx_types::OpaqueCursor::new(cursor.to_string()));
    }
    if let Some(limit) = query_value(query, "limit") {
        edges.limit = Some(parse_limit(limit)?);
    }
    let request = GctxGraphEdgesRequest {
        workspace_root: root.clone(),
        query: edges,
    };
    let response: GctxGraphEdgesResponse =
        match gctx_call(ANVIL_GCTX_GRAPH_EDGES, &request, "mcp-gctx-graph-edges") {
            Ok(response) => response,
            Err(GctxDaemonError::Unavailable) => GctxGraphEdgesResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: anvil_gctx_types::GraphEdgesOutcome::Unavailable,
            },
            Err(GctxDaemonError::Failure) => {
                return Err(ReadError::Internal(
                    "graph-context daemon request failed".to_string(),
                ));
            }
        };
    if matches!(
        response.outcome,
        anvil_gctx_types::GraphEdgesOutcome::NotReady { .. }
    ) {
        rewarm(&root);
    }
    Ok(serde_json::to_value(response).expect("gctx response serialises"))
}

/// ADR-085 C1 best-effort, fire-and-forget on-demand re-warm of a cold/warming
/// graph (council CR-1 — the resources mirror the GCTX tools' rewarm).
fn rewarm(root: &str) {
    let _ = crate::commands::watch_save_time::warm_up_root(std::path::Path::new(root));
}

/// Reject a query string that carries any key outside `allowed`. This is the
/// load-bearing guard for a `&` in a value (council follow-up): `split_uri`
/// splits on `&` *first*, so `?file=src/a&b.ts` arrives as `file=src/a` plus a
/// stray `b.ts` key — silently truncating the path. Catching the unexpected key
/// turns that into a loud `BadRequest` instead of a wrong-path read.
fn ensure_known_query_keys(query: &[(String, String)], allowed: &[&str]) -> Result<(), ReadError> {
    if let Some((key, _)) = query.iter().find(|(k, _)| !allowed.contains(&k.as_str())) {
        return Err(ReadError::BadRequest(format!(
            "unexpected query parameter `{key}` (allowed: {}); note `&` and `%` \
             in a file path are unsupported — pass a raw workspace-relative path",
            if allowed.is_empty() {
                "none".to_string()
            } else {
                allowed.join(", ")
            }
        )));
    }
    Ok(())
}

/// Validate the optional `file` query filter. The URI query parser does no
/// percent-decoding, so a value containing `%` would silently mis-map to a
/// different path (council ADV-5/CR-5); reject it loudly. (A `&` cannot reach
/// here — `split_uri` already consumed it as a separator; the stray key it
/// produces is rejected by [`ensure_known_query_keys`].) A raw workspace-relative
/// path is otherwise forwarded verbatim (the daemon does the CE-6 path hygiene).
fn validated_file_filter(query: &[(String, String)]) -> Result<Option<String>, ReadError> {
    match query_value(query, "file") {
        None => Ok(None),
        Some(file) if file.contains('%') => Err(ReadError::BadRequest(
            "the `file` query value must be a raw (un-percent-encoded) \
             workspace-relative path without `%`"
                .to_string(),
        )),
        Some(file) => Ok(Some(file.to_string())),
    }
}

/// The session-pinned workspace root: the MCP server's own canonicalised cwd
/// (GCTX-002 CE-8 — stdio-only, no client-supplied root).
fn workspace_root() -> Result<String, ReadError> {
    let cwd = std::env::current_dir()
        .map_err(|err| ReadError::Internal(format!("MCP server cwd is not accessible: {err}")))?;
    let canonical = cwd.canonicalize().unwrap_or(cwd);
    Ok(canonical.to_string_lossy().into_owned())
}

fn parse_limit(raw: &str) -> Result<u32, ReadError> {
    raw.parse::<u32>()
        .map_err(|_| ReadError::BadRequest(format!("invalid `limit` query value: {raw}")))
}

/// Split `graph://edges?file=a&cursor=b` into the base URI and decoded query
/// pairs. Minimal `k=v&k=v` parsing — the only values are simple relative paths,
/// opaque hex cursors, and integers (no URL-encoding to undo).
fn split_uri(uri: &str) -> (&str, Vec<(String, String)>) {
    match uri.split_once('?') {
        None => (uri, Vec::new()),
        Some((base, raw_query)) => {
            let pairs = raw_query
                .split('&')
                .filter(|segment| !segment.is_empty())
                .map(|segment| match segment.split_once('=') {
                    Some((key, value)) => (key.to_string(), value.to_string()),
                    None => (segment.to_string(), String::new()),
                })
                .collect();
            (base, pairs)
        }
    }
}

fn query_value<'a>(query: &'a [(String, String)], key: &str) -> Option<&'a str> {
    query
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .filter(|v| !v.is_empty())
}

fn unavailable_assurance() -> anvil_intercept_proto::protocol::WorkspaceAssurance {
    anvil_intercept_proto::protocol::WorkspaceAssurance {
        state: anvil_intercept_proto::protocol::AssuranceState::Unavailable,
        reason: Some(anvil_intercept_proto::protocol::StaleReason::DaemonAbsent),
        generation: 0,
        last_full_scan: None,
        scan_coverage: None,
    }
}

/// Why a daemon GCTX request could not complete (mirrors the GCTX tools).
/// `Unavailable` (socket absent / `Method not found`) degrades to a structured
/// `unavailable` outcome; `Failure` (a malformed reply, an IO error) is an error.
#[cfg_attr(not(unix), allow(dead_code))]
enum GctxDaemonError {
    Unavailable,
    Failure,
}

/// A generic JSON-RPC envelope for a sealed GCTX response of type `R`. One
/// helper serves all three resources (and would serve the tools too), unlike the
/// per-tool envelopes in `mcp/tools/`.
#[cfg(unix)]
#[derive(serde::Deserialize)]
struct GctxRpcEnvelope<R> {
    #[serde(default)]
    id: Option<String>,
    #[serde(default = "Option::default")]
    result: Option<R>,
    #[serde(default)]
    error: Option<GctxRpcError>,
}

#[cfg(unix)]
#[derive(serde::Deserialize)]
struct GctxRpcError {
    code: i64,
}

/// Forward a sealed GCTX request to the daemon over the read-only `anvil/gctx/*`
/// surface and deserialise the sealed response. Generic over the request and
/// response types so all three resources share one socket exchange.
#[cfg(unix)]
fn gctx_call<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
) -> Result<Resp, GctxDaemonError>
where
    Req: serde::Serialize,
    Resp: DeserializeOwned,
{
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const TIMEOUT: Duration = Duration::from_secs(2);
    // Identity-only pages are small; 4 MiB is a generous malformed-response cap.
    const RESPONSE_LINE_CAP: u64 = 4 << 20;

    let socket_path = ipc::resolve_socket_path().map_err(|_| GctxDaemonError::Unavailable)?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
        return match err {
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(GctxDaemonError::Unavailable)
            }
            _ => {
                eprintln!("anvil-mcp: gctx {method} socket unavailable: {err}");
                Err(GctxDaemonError::Failure)
            }
        };
    }
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        eprintln!("anvil-mcp: gctx {method} connect failed: {err}");
        GctxDaemonError::Unavailable
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        eprintln!("anvil-mcp: gctx {method} peer rejected: {err}");
        GctxDaemonError::Failure
    })?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx {method} read-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-mcp: gctx {method} write-timeout setup failed: {err}");
        GctxDaemonError::Failure
    })?;

    let mut frame = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": request,
        "id": request_id,
    });
    // USAGE-004: attach the caller's salted-hash principal so the daemon records
    // an attributable `command.invoked` row.
    crate::usage::attach_principal(&mut frame);
    if let Err(err) = writeln!(stream, "{frame}").and_then(|()| stream.flush()) {
        eprintln!("anvil-mcp: gctx {method} request write failed: {err}");
        return Err(GctxDaemonError::Failure);
    }

    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_CAP + 1)
        .read_until(b'\n', &mut line)
        .map_err(|err| {
            eprintln!("anvil-mcp: gctx {method} response read failed: {err}");
            GctxDaemonError::Failure
        })?;
    if read == 0 || line.len() as u64 > RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
        eprintln!("anvil-mcp: gctx {method} response was empty, oversized, or unframed");
        return Err(GctxDaemonError::Failure);
    }
    let line = String::from_utf8(line).map_err(|_| {
        eprintln!("anvil-mcp: gctx {method} response was not UTF-8");
        GctxDaemonError::Failure
    })?;

    let envelope: GctxRpcEnvelope<Resp> = serde_json::from_str(&line).map_err(|err| {
        eprintln!("anvil-mcp: gctx {method} response parse failed: {err}");
        GctxDaemonError::Failure
    })?;
    if envelope.id.as_deref() != Some(request_id) {
        eprintln!("anvil-mcp: gctx {method} response id mismatch");
        return Err(GctxDaemonError::Failure);
    }
    if let Some(error) = envelope.error {
        return if error.code == -32601 {
            Err(GctxDaemonError::Unavailable)
        } else {
            eprintln!("anvil-mcp: gctx {method} daemon error {}", error.code);
            Err(GctxDaemonError::Failure)
        };
    }
    envelope.result.ok_or_else(|| {
        eprintln!("anvil-mcp: gctx {method} response carried neither result nor error");
        GctxDaemonError::Failure
    })
}

/// The Windows named-pipe GCTX client is a future item (shared with the GCTX
/// tool suite). Until it lands, degrade to a structured `unavailable` outcome.
#[cfg(not(unix))]
fn gctx_call<Req, Resp>(
    method: &str,
    _request: &Req,
    _request_id: &str,
) -> Result<Resp, GctxDaemonError>
where
    Req: serde::Serialize,
    Resp: DeserializeOwned,
{
    tracing::debug!(
        target: "anvil_mcp::gctx",
        method,
        "GCTX daemon client unavailable on non-unix (named-pipe transport pending)"
    );
    Err(GctxDaemonError::Unavailable)
}

#[cfg(test)]
mod tests;
