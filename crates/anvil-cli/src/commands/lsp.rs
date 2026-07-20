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
                if let Some(id) = id {
                    write_message(&mut stdout, &initialize_response(&id))?;
                }
            }
            Some("initialized") => {}
            Some("textDocument/didChange") => {
                let uri = message
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str);
                let text = message
                    .pointer("/params/contentChanges/0/text")
                    .and_then(Value::as_str);
                if let (Some(uri), Some(text)) = (uri, text) {
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
