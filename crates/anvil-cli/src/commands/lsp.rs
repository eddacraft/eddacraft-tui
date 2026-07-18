//! RTAI-005 throwaway spike (ADR-109, Accepted 2026-07-16).
//!
//! Proves the loop `textDocument/didChange` -> daemon `scan_buffer`
//! (`mode = "midEdit"`) -> `textDocument/publishDiagnostics` end to end,
//! one rule / one fixture, per the RTAI-001 spike precedent
//! (`plans/modules/realtime-ai-validation.aps.md`). This settles the
//! connection-lifecycle question (per-call connect, mirroring
//! `crate::mcp::validation`'s `SocketDaemonValidationClient`) before any
//! full build — it is not production-hardened and is not wired into the
//! MCP validation client's fail-closed/security posture.

use clap::Args;
#[cfg(unix)]
use serde_json::{Value, json};
#[cfg(unix)]
use std::collections::HashMap;
#[cfg(unix)]
use std::io::{self, BufReader, Write};

#[derive(Debug, Args)]
pub struct LspArgs {
    /// Serve LSP over stdin/stdout.
    #[arg(long)]
    stdio: bool,
}

pub fn run(args: &LspArgs) -> anyhow::Result<()> {
    if !args.stdio {
        anyhow::bail!("`anvil lsp` currently requires --stdio");
    }
    run_stdio_server()
}

#[cfg(unix)]
const MAX_LSP_FRAME_BYTES: u64 = 4 * 1024 * 1024;

#[cfg(not(unix))]
fn run_stdio_server() -> anyhow::Result<()> {
    anyhow::bail!(
        "anvil lsp (spike) requires a Unix domain socket; not yet supported on this platform"
    );
}

#[cfg(unix)]
fn run_stdio_server() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = io::stdout().lock();
    // Open-document text, tracked so `textDocument/references` can convert
    // an LSP (line, character) position to a byte offset itself — the
    // request carries only a position, and the daemon's `symbol_at` verb
    // takes a byte offset (GCTX-provider convention), not a line/character
    // pair.
    let mut documents: HashMap<String, String> = HashMap::new();
    // Prefer workspace roots advertised by the LSP client. Process CWD is
    // only a compatibility fallback for clients that omit both modern
    // workspace fields; it must not override an explicit root because the
    // server is commonly launched outside the project directory.
    let fallback_workspace_root = std::env::current_dir().ok();
    let mut workspace_roots = fallback_workspace_root.iter().cloned().collect::<Vec<_>>();

    while let Some(body) = read_lsp_frame(&mut reader)? {
        if body.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let Ok(message) = serde_json::from_slice::<Value>(&body) else {
            continue;
        };

        let method = message.get("method").and_then(Value::as_str);
        let id = message.get("id").cloned();

        match method {
            Some("initialize") => {
                let advertised_roots = workspace_roots_from_initialize(&message);
                if !advertised_roots.is_empty() {
                    workspace_roots = advertised_roots;
                }
                if let Some(id) = id {
                    write_message(&mut stdout, &initialize_response(&id))?;
                }
            }
            Some("initialized") => {}
            Some("textDocument/didOpen") => {
                let uri = message
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str);
                let text = message
                    .pointer("/params/textDocument/text")
                    .and_then(Value::as_str);
                if let (Some(uri), Some(text)) = (uri, text) {
                    documents.insert(uri.to_string(), text.to_string());
                }
            }
            Some("textDocument/didChange") => {
                let uri = message
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str);
                let text = message
                    .pointer("/params/contentChanges/0/text")
                    .and_then(Value::as_str);
                if let (Some(uri), Some(text)) = (uri, text) {
                    documents.insert(uri.to_string(), text.to_string());
                    let path = uri_to_path(uri);
                    // Always publish — `didChange` is a notification, so a
                    // client (including the benchmark harness) waiting on
                    // `publishDiagnostics` to clear/update state would hang
                    // if a scan failure produced no reply. An empty
                    // diagnostics set on failure degrades to "no in-flight
                    // findings," matching RTAI-005's daemon-down posture.
                    let diagnostics =
                        daemon::scan_buffer_mid_edit(&path, text).unwrap_or_else(|err| {
                            eprintln!("anvil-lsp: mid-edit scan failed: {err}");
                            Vec::new()
                        });
                    let notification = publish_diagnostics_notification(uri, &diagnostics);
                    write_message(&mut stdout, &notification)?;
                }
            }
            Some("textDocument/didClose") => {
                if let Some(uri) = message
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                {
                    documents.remove(uri);
                }
            }
            Some("textDocument/references") => {
                if let Some(id) = id {
                    let response = references_response(&id, &message, &documents, &workspace_roots);
                    write_message(&mut stdout, &response)?;
                }
            }
            // Custom extension methods (not LSP-native — no standard verb
            // covers "blast radius" / "likely tests" per the 2026-07-16
            // spike report's Bucket 2). Wired to the already-shipped
            // `anvil_impact_of_change`/`anvil_affected_tests` daemon RPCs
            // via the same shared production client `references` uses.
            Some("anvil/impactOfChange") => {
                if let Some(id) = id {
                    let response = impact_of_change_response(&id, &message, &workspace_roots);
                    write_message(&mut stdout, &response)?;
                }
            }
            Some("anvil/affectedTests") => {
                if let Some(id) = id {
                    let response = affected_tests_response(&id, &message, &workspace_roots);
                    write_message(&mut stdout, &response)?;
                }
            }
            Some("shutdown") => {
                if let Some(id) = id {
                    write_message(&mut stdout, &success_response(&id, &Value::Null))?;
                }
            }
            Some("exit") => break,
            _ => {
                if let Some(id) = id {
                    write_message(
                        &mut stdout,
                        &error_response(&id, -32601, "Method not found"),
                    )?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(unix)]
fn read_lsp_frame(reader: &mut impl io::BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let Some(len) = content_length else {
        return Ok(Some(Vec::new()));
    };
    if len as u64 > MAX_LSP_FRAME_BYTES {
        let mut remaining = len;
        let mut sink = [0u8; 8192];
        while remaining > 0 {
            let chunk = remaining.min(sink.len());
            reader.read_exact(&mut sink[..chunk])?;
            remaining -= chunk;
        }
        return Ok(Some(Vec::new()));
    }
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

#[cfg(unix)]
fn write_message(stdout: &mut impl Write, message: &Value) -> anyhow::Result<()> {
    let body = serde_json::to_vec(message)?;
    write!(stdout, "Content-Length: {}\r\n\r\n", body.len())?;
    stdout.write_all(&body)?;
    stdout.flush()?;
    Ok(())
}

#[cfg(unix)]
fn success_response(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

#[cfg(unix)]
fn error_response(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(unix)]
fn initialize_response(id: &Value) -> Value {
    success_response(
        id,
        &json!({
            "capabilities": {
                // Full sync: `contentChanges[0].text` carries the whole
                // buffer, no incremental-range reconciliation needed for
                // this spike.
                "textDocumentSync": 1
            },
            "serverInfo": {
                "name": "anvil-lsp-spike",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

#[cfg(unix)]
fn uri_to_path(uri: &str) -> String {
    uri.strip_prefix("file://").unwrap_or(uri).to_string()
}

/// Workspace roots advertised by the client during `initialize`. Modern
/// `workspaceFolders` takes precedence over the deprecated `rootUri`; the
/// server keeps every folder so a document in a nested/multi-root workspace
/// can be routed to the most specific admitted root.
#[cfg(unix)]
fn workspace_roots_from_initialize(message: &Value) -> Vec<std::path::PathBuf> {
    let workspace_folders = message
        .pointer("/params/workspaceFolders")
        .and_then(Value::as_array)
        .map(|folders| {
            folders
                .iter()
                .filter_map(|folder| folder.get("uri").and_then(Value::as_str))
                .filter_map(|uri| uri.strip_prefix("file://"))
                .map(std::path::PathBuf::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !workspace_folders.is_empty() {
        return workspace_folders;
    }

    message
        .pointer("/params/rootUri")
        .and_then(Value::as_str)
        .and_then(|uri| uri.strip_prefix("file://"))
        .map(std::path::PathBuf::from)
        .into_iter()
        .collect()
}

/// Select the most specific advertised workspace containing `path`.
#[cfg(unix)]
fn workspace_root_for_path<'a>(
    path: &std::path::Path,
    workspace_roots: &'a [std::path::PathBuf],
) -> Option<&'a std::path::Path> {
    workspace_roots
        .iter()
        .filter(|root| path.strip_prefix(root).is_ok())
        .max_by_key(|root| root.components().count())
        .map(std::path::PathBuf::as_path)
}

#[cfg(unix)]
fn publish_diagnostics_notification(
    uri: &str,
    diagnostics: &[anvil_kernel_types::Diagnostic],
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": diagnostics.iter().map(to_lsp_diagnostic).collect::<Vec<_>>()
        }
    })
}

#[cfg(unix)]
fn to_lsp_diagnostic(diagnostic: &anvil_kernel_types::Diagnostic) -> Value {
    let start_line = diagnostic.location.line.unwrap_or(1).saturating_sub(1);
    let start_col = diagnostic.location.column.unwrap_or(1).saturating_sub(1);
    let end_line = diagnostic
        .location
        .end_line
        .unwrap_or(diagnostic.location.line.unwrap_or(1))
        .saturating_sub(1);
    let end_col = diagnostic
        .location
        .end_column
        .unwrap_or(diagnostic.location.column.unwrap_or(1))
        .saturating_sub(1);

    let severity = match diagnostic.severity {
        anvil_kernel_types::Severity::Error => 1,
        anvil_kernel_types::Severity::Warning | anvil_kernel_types::Severity::Unknown => 2,
        anvil_kernel_types::Severity::Info => 3,
    };

    json!({
        "range": {
            "start": { "line": start_line, "character": start_col },
            "end": { "line": end_line, "character": end_col }
        },
        "severity": severity,
        "code": diagnostic.source.rule_id,
        "source": "anvil",
        "message": diagnostic.summary,
        // Marker distinguishing in-flight (mid-edit) from on-disk
        // (save-time) diagnostics, per RTAI-005's Expected Outcome.
        "data": { "phase": "midEdit" }
    })
}

/// Convert an LSP `Position` (0-based line, 0-based UTF-16-code-unit
/// character) to a byte offset into `text`. Out-of-range input (a line or
/// character past the end of the document) clamps to `text.len()` rather
/// than failing — LSP clients occasionally send a stale position against a
/// buffer that has since shrunk, and a clamp degrades to "nothing found"
/// downstream rather than an error.
#[cfg(unix)]
fn lsp_position_to_byte_offset(text: &str, line: u32, character: u32) -> usize {
    let mut lines_seen = 0u32;
    let mut line_start = 0usize;
    let mut chars = text.char_indices();
    while lines_seen < line {
        match chars.next() {
            Some((idx, '\n')) => {
                lines_seen += 1;
                line_start = idx + 1;
            }
            Some(_) => {}
            None => return text.len(),
        }
    }

    let mut utf16_units = 0u32;
    for (idx, ch) in text[line_start..].char_indices() {
        if ch == '\n' || utf16_units >= character {
            return line_start + idx;
        }
        utf16_units += u32::try_from(ch.len_utf16()).unwrap_or(1);
    }
    text.len()
}

/// The reverse of [`lsp_position_to_byte_offset`]: a byte offset into `text`
/// to an LSP `(line, character)` pair. `byte_offset` clamps to `text.len()`.
#[cfg(unix)]
fn byte_offset_to_lsp_position(text: &str, byte_offset: usize) -> (u32, u32) {
    let mut clamped = byte_offset.min(text.len());
    // Daemon spans describe the resident on-disk graph while an open LSP
    // buffer may have moved since that graph was built. Never let a stale
    // byte offset land inside a UTF-8 code point and panic the stdio server.
    while !text.is_char_boundary(clamped) {
        clamped = clamped.saturating_sub(1);
    }
    let mut line = 0u32;
    let mut line_start = 0usize;
    for (idx, ch) in text.char_indices() {
        if idx >= clamped {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = idx + 1;
        }
    }
    let character = text[line_start..clamped]
        .chars()
        .map(|c| u32::try_from(c.len_utf16()).unwrap_or(1))
        .sum();
    (line, character)
}

#[cfg(unix)]
fn path_to_uri(path: &std::path::Path) -> String {
    format!("file://{}", path.display())
}

/// `path` (an absolute filesystem path, from [`uri_to_path`]) relative to
/// `workspace_root` — the convention every `SymbolIdentity.file` uses
/// (`crates/anvil-cli/src/mcp/tools/find_callers.rs`'s `target.file`, for
/// example). Returns `None` outside `workspace_root`: CE-6 requires a relative
/// path, so forwarding an absolute path would turn the intended clean miss
/// into an `InvalidQuery` outcome.
#[cfg(unix)]
fn relative_to_workspace(path: &str, workspace_root: &std::path::Path) -> Option<String> {
    std::path::Path::new(path)
        .strip_prefix(workspace_root)
        .ok()
        .map(|rel| rel.to_string_lossy().into_owned())
}

/// Build the `textDocument/references` JSON-RPC response: resolve the
/// queried position to a symbol (`symbol_at`), find its callers
/// (`find_callers`, auto-paged), then resolve each caller's location
/// (`get_snippet`, identity-only) into an LSP `Location`. A daemon-down or
/// no-symbol-here outcome degrades to an empty result array, never an
/// error — matches `textDocument/references`' "nothing found" convention.
#[cfg(unix)]
fn references_response(
    id: &Value,
    message: &Value,
    documents: &HashMap<String, String>,
    workspace_roots: &[std::path::PathBuf],
) -> Value {
    let uri = message
        .pointer("/params/textDocument/uri")
        .and_then(Value::as_str);
    let line = message
        .pointer("/params/position/line")
        .and_then(Value::as_u64);
    let character = message
        .pointer("/params/position/character")
        .and_then(Value::as_u64);
    let (Some(uri), Some(line), Some(character)) = (uri, line, character) else {
        return error_response(id, -32602, "Invalid params");
    };

    let Some(text) = documents.get(uri) else {
        // No tracked buffer (no `didOpen`, or already closed) — a clean
        // empty result, not an error.
        return success_response(id, &Value::Array(Vec::new()));
    };
    let line = u32::try_from(line).unwrap_or(u32::MAX);
    let character = u32::try_from(character).unwrap_or(u32::MAX);
    let byte_offset =
        u32::try_from(lsp_position_to_byte_offset(text, line, character)).unwrap_or(u32::MAX);
    let path = std::path::PathBuf::from(uri_to_path(uri));
    let Some(workspace_root) = workspace_root_for_path(&path, workspace_roots) else {
        return success_response(id, &Value::Array(Vec::new()));
    };
    let Some(relative_file) = relative_to_workspace(&path.to_string_lossy(), workspace_root) else {
        return success_response(id, &Value::Array(Vec::new()));
    };

    let locations = daemon::references_at(workspace_root, &relative_file, byte_offset)
        .unwrap_or_else(|err| {
            eprintln!("anvil-lsp: references lookup failed: {err}");
            Vec::new()
        });

    let results: Vec<Value> = locations
        .into_iter()
        .map(|(file, span)| {
            let absolute = workspace_root.join(&file);
            // The caller's own file text if it happens to be open too
            // (accurate UTF-16 conversion); otherwise read from disk. A
            // read failure degrades to a start-of-file location rather
            // than dropping the result — the file/symbol identity is
            // still useful even if the exact line cannot be resolved.
            let caller_uri = path_to_uri(&absolute);
            let caller_text = documents
                .get(&caller_uri)
                .cloned()
                .or_else(|| std::fs::read_to_string(&absolute).ok());
            let (start_line, start_char) = caller_text.as_deref().map_or((0, 0), |t| {
                byte_offset_to_lsp_position(t, span.start as usize)
            });
            let (end_line, end_char) = caller_text
                .as_deref()
                .map_or((start_line, start_char), |t| {
                    byte_offset_to_lsp_position(t, span.end as usize)
                });
            json!({
                "uri": caller_uri,
                "range": {
                    "start": { "line": start_line, "character": start_char },
                    "end": { "line": end_line, "character": end_char }
                }
            })
        })
        .collect();

    success_response(id, &Value::Array(results))
}

/// Extract a custom-method `changed_files` array (workspace-relative or
/// absolute — absolute entries are relativized the same way
/// `textDocument/references` relativizes a URI-derived path) and an
/// optional `max_depth` from `anvil/impactOfChange` /
/// `anvil/affectedTests`' shared params shape:
/// `{ "changedFiles": [...], "maxDepth": <int>? }`.
#[cfg(unix)]
fn parse_changed_files_params(
    message: &Value,
    workspace_root: &std::path::Path,
) -> Option<(Vec<String>, Option<u32>)> {
    let changed_files = message
        .pointer("/params/changedFiles")
        .and_then(Value::as_array)
        .map_or_else(
            || Some(Vec::new()),
            |files| {
                files
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|f| {
                        if std::path::Path::new(f).is_absolute() {
                            relative_to_workspace(f, workspace_root)
                        } else {
                            Some(f.to_string())
                        }
                    })
                    .collect::<Option<Vec<_>>>()
            },
        )?;
    let max_depth = message
        .pointer("/params/maxDepth")
        .and_then(Value::as_u64)
        .and_then(|d| u32::try_from(d).ok());
    Some((changed_files, max_depth))
}

#[cfg(unix)]
fn workspace_root_for_changed_files<'a>(
    message: &Value,
    workspace_roots: &'a [std::path::PathBuf],
) -> Option<&'a std::path::Path> {
    let absolute_path = message
        .pointer("/params/changedFiles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(std::path::Path::new)
        .find(|path| path.is_absolute());

    absolute_path
        .and_then(|path| workspace_root_for_path(path, workspace_roots))
        .or_else(|| workspace_roots.first().map(std::path::PathBuf::as_path))
}

#[cfg(unix)]
fn impact_of_change_response(
    id: &Value,
    message: &Value,
    workspace_roots: &[std::path::PathBuf],
) -> Value {
    let Some(workspace_root) = workspace_root_for_changed_files(message, workspace_roots) else {
        return error_response(id, -32602, "Invalid params");
    };
    let Some((changed_files, max_depth)) = parse_changed_files_params(message, workspace_root)
    else {
        return error_response(id, -32602, "Invalid params");
    };
    if changed_files.is_empty() {
        return error_response(id, -32602, "Invalid params");
    }
    match daemon::impact_of_change(workspace_root, changed_files, max_depth) {
        Ok(report) => success_response(id, &report),
        Err(err) => {
            eprintln!("anvil-lsp: impactOfChange lookup failed: {err}");
            success_response(id, &json!({ "status": "unavailable" }))
        }
    }
}

#[cfg(unix)]
fn affected_tests_response(
    id: &Value,
    message: &Value,
    workspace_roots: &[std::path::PathBuf],
) -> Value {
    let Some(workspace_root) = workspace_root_for_changed_files(message, workspace_roots) else {
        return error_response(id, -32602, "Invalid params");
    };
    let Some((changed_files, max_depth)) = parse_changed_files_params(message, workspace_root)
    else {
        return error_response(id, -32602, "Invalid params");
    };
    if changed_files.is_empty() {
        return error_response(id, -32602, "Invalid params");
    }
    match daemon::affected_tests(workspace_root, changed_files, max_depth) {
        Ok(report) => success_response(id, &report),
        Err(err) => {
            eprintln!("anvil-lsp: affectedTests lookup failed: {err}");
            success_response(id, &json!({ "status": "unavailable" }))
        }
    }
}

#[cfg(unix)]
mod daemon {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;
    use anvil_kernel_types::Diagnostic;
    use anyhow::{Context, Result, bail};
    use serde::Deserialize;
    use serde_json::json;

    const DAEMON_REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
    const DAEMON_RESPONSE_LINE_BYTES: u64 = 1 << 20;
    const DAEMON_REQUEST_ID: &str = "lsp-mid-edit-validation";
    const SCAN_BUFFER_REQUEST_VERSION: u64 = 1;

    /// Thin frontend over the shipped `scan_buffer` RPC in
    /// `mode = "midEdit"` — connects fresh per call, mirroring
    /// `crate::mcp::validation::request_daemon_diagnostics`'s
    /// connection-lifecycle choice (RTAI-005's readiness-note open
    /// question). Deliberately not merged with that function: this is
    /// spike code, not wired into the MCP client's fail-closed/security
    /// posture or its test suite.
    pub(super) fn scan_buffer_mid_edit(path: &str, text: &str) -> Result<Vec<Diagnostic>> {
        let socket_path = ipc::resolve_socket_path().context("resolve daemon socket path")?;
        ipc::validate_socket_path_for_client(&socket_path).context("daemon socket unavailable")?;
        let mut stream = UnixStream::connect(&socket_path).context("connect to daemon")?;
        ipc::validate_connected_peer_for_client(&stream).context("daemon peer rejected")?;
        stream
            .set_read_timeout(Some(DAEMON_REQUEST_TIMEOUT))
            .context("set read timeout")?;
        stream
            .set_write_timeout(Some(DAEMON_REQUEST_TIMEOUT))
            .context("set write timeout")?;

        let frame = json!({
            "jsonrpc": "2.0",
            "method": "scan_buffer",
            "params": {
                "path": path,
                "text": text,
                "version": SCAN_BUFFER_REQUEST_VERSION,
                "mode": "midEdit"
            },
            "id": DAEMON_REQUEST_ID
        });
        writeln!(stream, "{frame}").context("send scan_buffer request")?;
        stream.flush().context("flush scan_buffer request")?;

        let mut reader = BufReader::new(stream);
        let line = read_capped_response_line(&mut reader)?;
        parse_scan_buffer_response(&line)
    }

    /// Defensive guard against a daemon that never terminates a
    /// `find_callers` cursor (mirrors `crates/anvil-cli/src/commands/
    /// kindling.rs`'s `collect_daemon_rows` `MAX_PAGES` pattern, ADR-095).
    const MAX_REFERENCE_PAGES: u32 = 50;

    /// Resolve `textDocument/references` for a position: `symbol_at`
    /// (position -> [`SymbolIdentity`]) -> `find_callers` (identity ->
    /// caller identities, auto-paged) -> `get_snippet` per caller
    /// (identity -> location, identity-only). Unlike
    /// [`scan_buffer_mid_edit`], this reuses the shared, production
    /// `crate::mcp::gctx_client::daemon_rpc_call` client every GCTX MCP
    /// tool already uses — full parity with those tools, not a fourth
    /// bespoke socket client.
    ///
    /// An on-demand request (not per-keystroke), per the 2026-06-03
    /// graph-backed-navigation brainstorm's two-timing-class design: the
    /// `get_snippet` fan-out (one call per caller) is affordable here in a
    /// way it would not be on the `didChange` hot path.
    pub(super) fn references_at(
        workspace_root: &std::path::Path,
        file: &str,
        byte_offset: u32,
    ) -> Result<Vec<(String, anvil_kernel_types::ByteRange)>> {
        use anvil_gctx_types::{FindCallersOutcome, SnippetOutcome, SymbolAtOutcome};
        use anvil_gctx_types::{FindCallersQuery, SnippetQuery, SymbolAtQuery};
        use anvil_intercept_proto::protocol::{
            ANVIL_GCTX_FIND_CALLERS, ANVIL_GCTX_GET_SNIPPET, ANVIL_GCTX_SYMBOL_AT,
            GctxFindCallersRequest, GctxFindCallersResponse, GctxGetSnippetRequest,
            GctxGetSnippetResponse, GctxSymbolAtRequest, GctxSymbolAtResponse,
        };

        use crate::mcp::gctx_client::daemon_rpc_call;

        let workspace_root_str = workspace_root.to_string_lossy().into_owned();

        let symbol_at_request = GctxSymbolAtRequest {
            workspace_root: workspace_root_str.clone(),
            query: SymbolAtQuery {
                file: Some(file.to_string()),
                byte_offset: Some(byte_offset),
            },
        };
        let symbol_at_response: GctxSymbolAtResponse =
            daemon_rpc_call(ANVIL_GCTX_SYMBOL_AT, &symbol_at_request, "lsp-symbol-at")
                .map_err(|err| anyhow::anyhow!("symbol_at daemon call failed: {err:?}"))?;
        let SymbolAtOutcome::Ready(projection) = symbol_at_response.outcome else {
            // Warming / unavailable / disabled / invalid — no target to walk.
            return Ok(Vec::new());
        };
        let Some(target) = projection.symbol else {
            // Clean miss: no symbol at this position.
            return Ok(Vec::new());
        };

        let mut callers = Vec::new();
        let mut cursor = None;
        for _ in 0..MAX_REFERENCE_PAGES {
            let find_callers_request = GctxFindCallersRequest {
                workspace_root: workspace_root_str.clone(),
                query: FindCallersQuery {
                    target: Some(target.clone()),
                    max_depth: Some(1),
                    cursor: cursor.take(),
                    ..Default::default()
                },
            };
            let response: GctxFindCallersResponse = daemon_rpc_call(
                ANVIL_GCTX_FIND_CALLERS,
                &find_callers_request,
                "lsp-find-callers",
            )
            .map_err(|err| anyhow::anyhow!("find_callers daemon call failed: {err:?}"))?;
            let FindCallersOutcome::Ready(page) = response.outcome else {
                break;
            };
            callers.extend(page.callers.into_iter().map(|c| c.caller));
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        let mut locations = Vec::with_capacity(callers.len());
        for caller in callers {
            let snippet_request = GctxGetSnippetRequest {
                workspace_root: workspace_root_str.clone(),
                query: SnippetQuery {
                    target: caller,
                    include_source: false,
                },
            };
            let response: Result<GctxGetSnippetResponse, _> =
                daemon_rpc_call(ANVIL_GCTX_GET_SNIPPET, &snippet_request, "lsp-get-snippet");
            if let Ok(response) = response
                && let SnippetOutcome::Ready(result) = response.outcome
            {
                locations.push((result.file, result.span));
            }
        }
        Ok(locations)
    }

    /// Thin frontend over the already-shipped `anvil_impact_of_change`
    /// daemon RPC — the whole point of the `anvil/impactOfChange` custom
    /// method (per the 2026-07-16 spike report's Decision (c)): reuse the
    /// existing capability at MCP tool-call parity, not reimplement it.
    /// Returns the outcome serialised close to its sealed wire shape rather
    /// than translated into an LSP-native structure (there is no LSP-native
    /// shape for "blast radius" to translate into).
    pub(super) fn impact_of_change(
        workspace_root: &std::path::Path,
        changed_files: Vec<String>,
        max_depth: Option<u32>,
    ) -> Result<serde_json::Value> {
        use anvil_gctx_types::ImpactQuery;
        use anvil_intercept_proto::protocol::{
            ANVIL_GCTX_IMPACT_OF_CHANGE, GctxImpactOfChangeRequest, GctxImpactOfChangeResponse,
        };

        use crate::mcp::gctx_client::daemon_rpc_call;

        let request = GctxImpactOfChangeRequest {
            workspace_root: workspace_root.to_string_lossy().into_owned(),
            query: ImpactQuery {
                changed_files,
                max_depth,
            },
        };
        let response: GctxImpactOfChangeResponse = daemon_rpc_call(
            ANVIL_GCTX_IMPACT_OF_CHANGE,
            &request,
            "lsp-impact-of-change",
        )
        .map_err(|err| anyhow::anyhow!("impact_of_change daemon call failed: {err:?}"))?;
        Ok(serde_json::to_value(response.outcome)?)
    }

    /// Thin frontend over the already-shipped `anvil_affected_tests` daemon
    /// RPC. Mirrors [`impact_of_change`].
    pub(super) fn affected_tests(
        workspace_root: &std::path::Path,
        changed_files: Vec<String>,
        max_depth: Option<u32>,
    ) -> Result<serde_json::Value> {
        use anvil_gctx_types::AffectedTestsQuery;
        use anvil_intercept_proto::protocol::{
            ANVIL_GCTX_AFFECTED_TESTS, GctxAffectedTestsRequest, GctxAffectedTestsResponse,
        };

        use crate::mcp::gctx_client::daemon_rpc_call;

        let request = GctxAffectedTestsRequest {
            workspace_root: workspace_root.to_string_lossy().into_owned(),
            query: AffectedTestsQuery {
                changed_files,
                max_depth,
            },
        };
        let response: GctxAffectedTestsResponse =
            daemon_rpc_call(ANVIL_GCTX_AFFECTED_TESTS, &request, "lsp-affected-tests")
                .map_err(|err| anyhow::anyhow!("affected_tests daemon call failed: {err:?}"))?;
        Ok(serde_json::to_value(response.outcome)?)
    }

    fn read_capped_response_line(reader: &mut impl BufRead) -> Result<String> {
        let mut response = Vec::new();
        let read = reader
            .by_ref()
            .take(DAEMON_RESPONSE_LINE_BYTES + 1)
            .read_until(b'\n', &mut response)?;
        if read == 0 {
            bail!("daemon closed connection without a response");
        }
        if response.len() as u64 > DAEMON_RESPONSE_LINE_BYTES {
            bail!("daemon response exceeded line cap");
        }
        if !response.ends_with(b"\n") {
            bail!("daemon response omitted newline frame terminator");
        }
        Ok(String::from_utf8(response)?)
    }

    fn parse_scan_buffer_response(line: &str) -> Result<Vec<Diagnostic>> {
        let response: JsonRpcScanBufferResponse =
            serde_json::from_str(line).context("parse daemon response")?;
        if let Some(error) = response.error {
            bail!("daemon scan_buffer error {}: {}", error.code, error.message);
        }
        let result = response.result.context("daemon response missing result")?;
        if result.truncated {
            bail!("daemon scan_buffer response was truncated");
        }
        Ok(result.diagnostics)
    }

    // B3-style local mirror (see `mcp::validation`): deliberately
    // decoupled from the daemon's own `ScanBufferResponse` struct.
    #[derive(Debug, Deserialize)]
    struct JsonRpcScanBufferResponse {
        result: Option<ScanBufferResult>,
        error: Option<JsonRpcErrorBody>,
    }

    #[derive(Debug, Deserialize)]
    struct ScanBufferResult {
        diagnostics: Vec<Diagnostic>,
        truncated: bool,
    }

    #[derive(Debug, Deserialize)]
    struct JsonRpcErrorBody {
        code: i64,
        message: String,
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn position_to_offset_resolves_first_line() {
        let text = "hello world\nsecond line\n";
        assert_eq!(lsp_position_to_byte_offset(text, 0, 0), 0);
        assert_eq!(lsp_position_to_byte_offset(text, 0, 6), 6);
    }

    #[test]
    fn position_to_offset_resolves_later_lines() {
        let text = "hello world\nsecond line\nthird\n";
        // "second line" starts right after the first "\n" (byte 12).
        assert_eq!(lsp_position_to_byte_offset(text, 1, 0), 12);
        assert_eq!(lsp_position_to_byte_offset(text, 1, 6), 18);
        // "third" starts after the second "\n" (byte 24).
        assert_eq!(lsp_position_to_byte_offset(text, 2, 0), 24);
    }

    #[test]
    fn position_to_offset_clamps_past_end_of_line_and_document() {
        let text = "abc\ndef\n";
        // character far past "abc"'s length clamps to the line's actual end
        // (the "\n"), not somewhere into the next line.
        assert_eq!(lsp_position_to_byte_offset(text, 0, 999), 3);
        // a line number past the document clamps to the document's end.
        assert_eq!(lsp_position_to_byte_offset(text, 999, 0), text.len());
    }

    #[test]
    fn position_to_offset_counts_utf16_units_not_bytes_or_chars() {
        // "é" is 1 UTF-16 unit but 2 UTF-8 bytes; "𝕊" (U+1D54A) is a
        // surrogate pair — 2 UTF-16 units but 4 UTF-8 bytes and 1 Rust
        // `char`. A byte- or char-counting implementation would
        // mis-resolve either case.
        let text = "café";
        // c(0) a(1) f(2) é(2 utf16 units at char index 3, byte offset 3..5)
        assert_eq!(lsp_position_to_byte_offset(text, 0, 3), 3);
        assert_eq!(lsp_position_to_byte_offset(text, 0, 4), text.len());

        let text = "a\u{1D54A}b"; // 'a', astral char (2 UTF-16 units), 'b'
        assert_eq!(lsp_position_to_byte_offset(text, 0, 0), 0);
        assert_eq!(lsp_position_to_byte_offset(text, 0, 1), 1); // mid-surrogate-pair offset: byte offset of the astral char's start
        assert_eq!(
            lsp_position_to_byte_offset(text, 0, 3),
            1 + '\u{1D54A}'.len_utf8()
        ); // past the astral char, at 'b'
    }

    #[test]
    fn offset_to_position_round_trips_with_position_to_offset() {
        let text = "hello world\nsecond line\nthird\n";
        for (line, character) in [(0, 0), (0, 6), (1, 0), (1, 6), (2, 0)] {
            let offset = lsp_position_to_byte_offset(text, line, character);
            assert_eq!(
                byte_offset_to_lsp_position(text, offset),
                (line, character),
                "round trip failed for ({line}, {character})"
            );
        }
    }

    #[test]
    fn offset_to_position_clamps_past_end_of_document() {
        let text = "abc\ndef";
        assert_eq!(byte_offset_to_lsp_position(text, 9999), (1, 3));
    }

    #[test]
    fn offset_to_position_clamps_stale_offsets_to_a_utf8_boundary() {
        let text = "aéz";
        // Byte offset 2 is inside the two-byte `é`; a stale graph span must
        // degrade to the preceding character boundary rather than panic.
        assert_eq!(byte_offset_to_lsp_position(text, 2), (0, 1));
    }

    #[test]
    fn relative_to_workspace_strips_the_prefix() {
        let root = std::path::Path::new("/home/user/project");
        assert_eq!(
            relative_to_workspace("/home/user/project/src/a.ts", root),
            Some("src/a.ts".to_string())
        );
    }

    #[test]
    fn relative_to_workspace_rejects_paths_outside_the_root() {
        let root = std::path::Path::new("/home/user/project");
        assert_eq!(relative_to_workspace("/somewhere/else/a.ts", root), None);
    }

    #[test]
    fn initialize_workspace_roots_prefers_workspace_folders() {
        let message = json!({
            "params": {
                "rootUri": "file:///fallback/project",
                "workspaceFolders": [
                    { "uri": "file:///home/user/project", "name": "project" },
                    { "uri": "file:///home/user/project/packages/a", "name": "a" }
                ]
            }
        });

        assert_eq!(
            workspace_roots_from_initialize(&message),
            vec![
                std::path::PathBuf::from("/home/user/project"),
                std::path::PathBuf::from("/home/user/project/packages/a"),
            ]
        );
    }

    #[test]
    fn initialize_workspace_roots_falls_back_to_root_uri() {
        let message = json!({
            "params": { "rootUri": "file:///home/user/project" }
        });

        assert_eq!(
            workspace_roots_from_initialize(&message),
            vec![std::path::PathBuf::from("/home/user/project")]
        );
    }

    #[test]
    fn workspace_root_for_path_selects_the_most_specific_folder() {
        let roots = vec![
            std::path::PathBuf::from("/home/user/project"),
            std::path::PathBuf::from("/home/user/project/packages/a"),
        ];

        assert_eq!(
            workspace_root_for_path(
                std::path::Path::new("/home/user/project/packages/a/src/lib.rs"),
                &roots,
            ),
            Some(std::path::Path::new("/home/user/project/packages/a"))
        );
    }

    #[test]
    fn workspace_root_for_path_returns_none_outside_advertised_folders() {
        let roots = vec![std::path::PathBuf::from("/home/user/project")];

        assert_eq!(
            workspace_root_for_path(std::path::Path::new("/elsewhere/src/lib.rs"), &roots),
            None
        );
    }

    #[test]
    fn uri_and_path_round_trip() {
        let path = std::path::Path::new("/home/user/project/src/a.ts");
        let uri = path_to_uri(path);
        assert_eq!(uri, "file:///home/user/project/src/a.ts");
        assert_eq!(uri_to_path(&uri), "/home/user/project/src/a.ts");
    }

    #[test]
    fn references_response_on_untracked_document_is_a_clean_empty_result_not_an_error() {
        let documents: HashMap<String, String> = HashMap::new();
        let message = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": "file:///never/opened.ts" },
                "position": { "line": 0, "character": 0 }
            }
        });
        let roots = vec![std::path::PathBuf::from("/workspace")];
        let response = references_response(&json!(1), &message, &documents, &roots);
        assert_eq!(response["result"], json!([]));
        assert!(response.get("error").is_none());
    }

    #[test]
    fn references_response_rejects_a_malformed_request() {
        let documents: HashMap<String, String> = HashMap::new();
        let message = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/references",
            "params": { "textDocument": { "uri": "file:///a.ts" } }
        });
        let roots = vec![std::path::PathBuf::from("/")];
        let response = references_response(&json!(1), &message, &documents, &roots);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn references_response_returns_a_clean_miss_outside_advertised_workspaces() {
        let uri = "file:///elsewhere/src/lib.rs";
        let documents = HashMap::from([(uri.to_string(), "fn example() {}".to_string())]);
        let message = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 3 }
            }
        });
        let roots = vec![std::path::PathBuf::from("/workspace")];

        let response = references_response(&json!(1), &message, &documents, &roots);

        assert_eq!(response["result"], json!([]));
        assert!(response.get("error").is_none());
    }

    #[test]
    fn parse_changed_files_relativizes_absolute_entries_and_keeps_relative_ones() {
        let root = std::path::Path::new("/home/user/project");
        let message = json!({
            "params": {
                "changedFiles": ["/home/user/project/src/a.ts", "src/b.ts"],
                "maxDepth": 2
            }
        });
        let (changed_files, max_depth) = parse_changed_files_params(&message, root)
            .expect("all changed files should be under the workspace root");
        assert_eq!(
            changed_files,
            vec!["src/a.ts".to_string(), "src/b.ts".to_string(),]
        );
        assert_eq!(max_depth, Some(2));
    }

    #[test]
    fn parse_changed_files_rejects_absolute_entries_outside_the_workspace() {
        let root = std::path::Path::new("/home/user/project");
        let message = json!({
            "params": {
                "changedFiles": ["/home/user/project/src/a.ts", "/elsewhere/c.ts"]
            }
        });

        assert_eq!(parse_changed_files_params(&message, root), None);
    }

    #[test]
    fn parse_changed_files_defaults_are_empty_and_no_depth() {
        let root = std::path::Path::new("/home/user/project");
        let message = json!({ "params": {} });
        let (changed_files, max_depth) =
            parse_changed_files_params(&message, root).expect("empty params are valid to parse");
        assert!(changed_files.is_empty());
        assert_eq!(max_depth, None);
    }

    #[test]
    fn impact_of_change_response_rejects_empty_changed_files() {
        let message = json!({ "params": { "changedFiles": [] } });
        let roots = vec![std::path::PathBuf::from("/workspace")];
        let response = impact_of_change_response(&json!(1), &message, &roots);
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn affected_tests_response_rejects_empty_changed_files() {
        let message = json!({ "params": { "changedFiles": [] } });
        let roots = vec![std::path::PathBuf::from("/workspace")];
        let response = affected_tests_response(&json!(1), &message, &roots);
        assert_eq!(response["error"]["code"], -32602);
    }
}
