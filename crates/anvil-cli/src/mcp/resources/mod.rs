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

use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use crate::mcp::gctx_client::{DaemonRpcError, daemon_rpc_call};

use anvil_intercept_proto::protocol::{
    ANVIL_GCTX_GRAPH_EDGES, ANVIL_GCTX_GRAPH_STATS, ANVIL_GCTX_SEARCH_SYMBOLS,
    GctxGraphEdgesRequest, GctxGraphEdgesResponse, GctxGraphStatsRequest, GctxGraphStatsResponse,
    GctxSearchSymbolsRequest, GctxSearchSymbolsResponse,
};

/// RMCPF-020 — the local-state `anvil://` resources (architecture baseline,
/// suppressions, config, drift, constraints, anti-pattern catalogue). They read
/// workspace files directly rather than forwarding to the daemon, so they live
/// apart from the GCTX `graph://` egress code in this module.
pub mod anvil;

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
    let mut resources = vec![
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
    ];
    // RMCPF-020: the local-state anvil:// resources are advertised alongside the
    // graph:// ones in a single `resources/list`.
    resources.extend(anvil::list());
    resources
}

/// CIB-091d (CE-6): per-session byte ceiling for `graph://` resource reads.
///
/// `resources/read` page sizes are individually capped ([`MAX_PAGE_LIMIT`] = 200),
/// but nothing bounded the *aggregate* bytes a single assistant session could
/// pull, so an assistant could reassemble the whole graph across many
/// round-trips. The stdio MCP server is **one process per client session**
/// (`mcp_serve_stdio` runs a single synchronous loop over the session's stdio),
/// so a process-global credit is naturally per-session: this counter accumulates
/// the serialised byte size of every `graph://` payload served and refuses once
/// the credit is spent.
///
/// 8 MiB comfortably covers any honest interactive use (each identity-only page
/// is small — a few KiB) while bounding bulk reassembly; the assistant can still
/// page until the ceiling, then receives a structured `quota_exceeded`.
const GRAPH_EGRESS_CREDIT_BYTES: u64 = 8 << 20;

/// Bytes of `graph://` payload already served this session (process). Charged
/// with a wrapping `fetch_add` (the only `AtomicU64` add primitive), but the
/// ceiling check uses a `saturating_add` on the returned prior total so a
/// pathological u64 wrap can never read as "under budget"; never reset for the
/// life of the process (= the session).
static GRAPH_EGRESS_SPENT: AtomicU64 = AtomicU64::new(0);

/// CIB-091d: deduct `payload_bytes` from this session's `graph://` egress credit.
/// Returns [`ReadError::QuotaExceeded`] once the cumulative total exceeds
/// [`GRAPH_EGRESS_CREDIT_BYTES`] — the read that crosses the ceiling is refused
/// (the page is not served), so the budget is a hard cap, not a soft one. Only
/// `graph://` reads are charged; the `anvil://` local-state resources are not.
fn charge_graph_egress(payload_bytes: u64) -> Result<(), ReadError> {
    if try_charge_graph_egress(payload_bytes) {
        Ok(())
    } else {
        Err(ReadError::QuotaExceeded(graph_egress_quota_reason()))
    }
}

/// CIB-091d: the same per-session `graph://` egress credit, shared with the GCTX
/// **tool-call** surface (`anvil_search_symbols`, `find_dependents`,
/// `find_callers`, `impact_of_change`, `affected_tests`). The tool handlers carry
/// the same identity data as the `graph://` resources, so without this they would
/// be an unbounded back door past the resource byte ceiling — an assistant could
/// reassemble the graph via `tools/call` instead of `resources/read`. Both paths
/// charge the **same** [`GRAPH_EGRESS_SPENT`] accumulator.
///
/// Returns `true` when the read is within budget (and the bytes have been
/// charged), `false` once the cumulative total would exceed
/// [`GRAPH_EGRESS_CREDIT_BYTES`] — the caller refuses the over-ceiling payload.
#[must_use]
pub fn try_charge_graph_egress(payload_bytes: u64) -> bool {
    let previously_spent = GRAPH_EGRESS_SPENT.fetch_add(payload_bytes, Ordering::Relaxed);
    let total = previously_spent.saturating_add(payload_bytes);
    total <= GRAPH_EGRESS_CREDIT_BYTES
}

/// The structured reason text for an exhausted per-session `graph://` egress
/// credit (shared by the resource and tool-call surfaces).
#[must_use]
pub fn graph_egress_quota_reason() -> String {
    format!(
        "graph:// egress quota exhausted for this session ({GRAPH_EGRESS_CREDIT_BYTES} bytes); \
         reconnect to reset"
    )
}

/// Test-only: serialise + reset the process-global egress credit so the few
/// tests that drive it to exhaustion (here and in the GCTX `tools/call`
/// dispatch) are deterministic regardless of test run order or parallelism. The
/// returned guard holds the lock for the test's duration; the counter is zeroed
/// on acquire so the credit always starts fresh.
#[cfg(test)]
pub(crate) fn lock_and_reset_graph_egress_for_test() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    GRAPH_EGRESS_SPENT.store(0, Ordering::Relaxed);
    guard
}

/// Test-only: drive the process-global egress credit straight past the ceiling
/// (a `store`, so it cannot wrap the counter low the way a `u64::MAX` `fetch_add`
/// would) so the next charge on any surface refuses. Call under the
/// [`lock_and_reset_graph_egress_for_test`] guard.
#[cfg(test)]
pub(crate) fn exhaust_graph_egress_for_test() {
    GRAPH_EGRESS_SPENT.store(GRAPH_EGRESS_CREDIT_BYTES + 1, Ordering::Relaxed);
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
    /// CIB-091d: the session's cumulative `graph://` egress credit is exhausted →
    /// JSON-RPC `-32603` with a `quota_exceeded` reason.
    QuotaExceeded(String),
}

impl ReadError {
    /// The reason text (for the error `data`).
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::BadRequest(reason) | Self::Internal(reason) | Self::QuotaExceeded(reason) => {
                reason
            }
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
    // RMCPF-020: the anvil:// resources read workspace-local state; route them
    // to their own handler before the graph:// daemon-forwarding dispatch.
    if uri.starts_with("anvil://") {
        return anvil::read(uri);
    }
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
    // CIB-091d (CE-6): charge this session's per-session graph:// egress credit by
    // the serialised payload size; refuse the read that crosses the ceiling so a
    // session cannot reassemble the whole graph across many round-trips.
    let payload_bytes = serde_json::to_vec(&payload).map_or(0, |v| v.len() as u64);
    charge_graph_egress(payload_bytes)?;
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
        match daemon_rpc_call(ANVIL_GCTX_GRAPH_STATS, &request, "mcp-gctx-graph-stats") {
            Ok(response) => response,
            Err(DaemonRpcError::Unavailable) => GctxGraphStatsResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: anvil_gctx_types::GraphStatsOutcome::Unavailable,
            },
            Err(DaemonRpcError::Failure) => {
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
    let response: GctxSearchSymbolsResponse = match daemon_rpc_call(
        ANVIL_GCTX_SEARCH_SYMBOLS,
        &request,
        "mcp-gctx-graph-symbols",
    ) {
        Ok(response) => response,
        Err(DaemonRpcError::Unavailable) => GctxSearchSymbolsResponse {
            workspace_assurance: unavailable_assurance(),
            outcome: anvil_gctx_types::SearchSymbolsOutcome::Unavailable,
        },
        Err(DaemonRpcError::Failure) => {
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
        match daemon_rpc_call(ANVIL_GCTX_GRAPH_EDGES, &request, "mcp-gctx-graph-edges") {
            Ok(response) => response,
            Err(DaemonRpcError::Unavailable) => GctxGraphEdgesResponse {
                workspace_assurance: unavailable_assurance(),
                outcome: anvil_gctx_types::GraphEdgesOutcome::Unavailable,
            },
            Err(DaemonRpcError::Failure) => {
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

#[cfg(test)]
mod tests;
