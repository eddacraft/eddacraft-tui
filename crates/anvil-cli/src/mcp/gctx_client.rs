//! Shared daemon client for GCTX MCP tools and resources.
//!
//! This module keeps the graph-context MCP surfaces graph-free: callers pass a
//! sealed request DTO and receive a sealed daemon response DTO over the
//! read-only `anvil/gctx/*` JSON-RPC methods. Transport failures that mean “no
//! usable daemon surface” classify as [`GctxDaemonError::Unavailable`]; malformed
//! replies and security-relevant failures classify as [`GctxDaemonError::Failure`].

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

/// Why a daemon GCTX request could not complete.
///
/// `Unavailable` (socket absent / `Method not found`) degrades to a structured
/// `unavailable` outcome; `Failure` (a malformed reply, an IO error mid-exchange,
/// or a security-relevant validation failure) is a tool/resource error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) enum GctxDaemonError {
    Unavailable,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(any(target_os = "linux", target_os = "macos"), allow(dead_code))]
enum PeerCredentialPlatform {
    LinuxOrMacos,
    OtherUnix,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
const PEER_CREDENTIAL_PLATFORM: PeerCredentialPlatform = PeerCredentialPlatform::LinuxOrMacos;

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
const PEER_CREDENTIAL_PLATFORM: PeerCredentialPlatform = PeerCredentialPlatform::OtherUnix;

fn classify_peer_validation_failure(platform: PeerCredentialPlatform) -> GctxDaemonError {
    match platform {
        PeerCredentialPlatform::LinuxOrMacos => GctxDaemonError::Failure,
        // CIB-099: non-Linux/macOS Unix builds do not have the same peer-credential
        // implementation. Treat that validation failure as an unavailable daemon
        // transport for GCTX so the sibling tools degrade consistently instead of
        // surfacing a hard tool failure.
        PeerCredentialPlatform::OtherUnix => GctxDaemonError::Unavailable,
    }
}

#[cfg(any(unix, windows))]
#[derive(serde::Deserialize)]
struct GctxRpcEnvelope<R> {
    #[serde(default)]
    id: Option<String>,
    #[serde(default = "Option::default")]
    result: Option<R>,
    #[serde(default)]
    error: Option<GctxRpcError>,
}

#[cfg(any(unix, windows))]
#[derive(serde::Deserialize)]
struct GctxRpcError {
    code: i64,
}

/// Forward a sealed GCTX request to the daemon over the read-only
/// `anvil/gctx/*` surface and deserialise the sealed response.
#[cfg(unix)]
pub(crate) fn gctx_call<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
) -> Result<Resp, GctxDaemonError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const TIMEOUT: Duration = Duration::from_secs(2);
    // Identity-only GCTX pages/reports are small; 4 MiB is a generous malformed-
    // response cap, sized above any honest reply.
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
        classify_peer_validation_failure(PEER_CREDENTIAL_PLATFORM)
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

/// Forward a sealed GCTX request to the daemon over the Windows owner-only named
/// pipe. This mirrors the Unix socket JSON-RPC contract while bounding the whole
/// synchronous pipe exchange on a worker thread because Win32 pipe reads/writes
/// do not expose the same per-stream timeout setters.
#[cfg(windows)]
pub(crate) fn gctx_call<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
) -> Result<Resp, GctxDaemonError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    use std::io::{BufRead, BufReader, Read, Write};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    const RESPONSE_LINE_CAP: u64 = 4 << 20;
    const TIMEOUT: Duration = Duration::from_secs(2);

    let pipe_name = anvil_intercept::ipc::resolve_pipe_name().map_err(|err| {
        eprintln!("anvil-mcp: gctx {method} pipe unavailable: {err}");
        GctxDaemonError::Unavailable
    })?;
    let params = serde_json::to_value(request).map_err(|err| {
        eprintln!("anvil-mcp: gctx {method} request serialise failed: {err}");
        GctxDaemonError::Failure
    })?;
    let method = method.to_owned();
    let method_label = method.clone();
    let request_id = request_id.to_owned();
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let outcome: Result<serde_json::Value, GctxDaemonError> = (|| {
            let mut client = anvil_intercept_win32::connect_owner_only_pipe_client(&pipe_name)
                .map_err(|err| {
                    eprintln!("anvil-mcp: gctx {method} pipe connect failed: {err}");
                    GctxDaemonError::Unavailable
                })?;
            let mut frame = json!({
                "jsonrpc": "2.0",
                "method": method.as_str(),
                "params": params,
                "id": request_id.as_str(),
            });
            crate::usage::attach_principal(&mut frame);
            if let Err(err) = writeln!(client, "{frame}").and_then(|()| client.flush()) {
                eprintln!("anvil-mcp: gctx {method} pipe request write failed: {err}");
                return Err(GctxDaemonError::Failure);
            }

            let mut reader = BufReader::new(client);
            let mut line = Vec::new();
            let read = reader
                .by_ref()
                .take(RESPONSE_LINE_CAP + 1)
                .read_until(b'\n', &mut line)
                .map_err(|err| {
                    eprintln!("anvil-mcp: gctx {method} pipe response read failed: {err}");
                    GctxDaemonError::Failure
                })?;
            if read == 0 || line.len() as u64 > RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
                eprintln!(
                    "anvil-mcp: gctx {method} pipe response was empty, oversized, or unframed"
                );
                return Err(GctxDaemonError::Failure);
            }
            let line = String::from_utf8(line).map_err(|_| {
                eprintln!("anvil-mcp: gctx {method} pipe response was not UTF-8");
                GctxDaemonError::Failure
            })?;
            let envelope: GctxRpcEnvelope<serde_json::Value> = serde_json::from_str(&line)
                .map_err(|err| {
                    eprintln!("anvil-mcp: gctx {method} pipe response parse failed: {err}");
                    GctxDaemonError::Failure
                })?;
            if envelope.id.as_deref() != Some(request_id.as_str()) {
                eprintln!("anvil-mcp: gctx {method} pipe response id mismatch");
                return Err(GctxDaemonError::Failure);
            }
            if let Some(error) = envelope.error {
                return if error.code == -32601 {
                    Err(GctxDaemonError::Unavailable)
                } else {
                    eprintln!("anvil-mcp: gctx {method} pipe daemon error {}", error.code);
                    Err(GctxDaemonError::Failure)
                };
            }
            envelope.result.ok_or_else(|| {
                eprintln!(
                    "anvil-mcp: gctx {method} pipe response carried neither result nor error"
                );
                GctxDaemonError::Failure
            })
        })();
        let _ = tx.send(outcome);
    });

    let value = match rx.recv_timeout(TIMEOUT) {
        Ok(outcome) => outcome?,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            eprintln!("anvil-mcp: gctx {method_label} pipe request timed out");
            return Err(GctxDaemonError::Unavailable);
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => return Err(GctxDaemonError::Unavailable),
    };
    serde_json::from_value(value).map_err(|err| {
        eprintln!("anvil-mcp: gctx {method_label} pipe response decode failed: {err}");
        GctxDaemonError::Failure
    })
}

/// Non-Unix, non-Windows targets have no daemon transport.
#[cfg(all(not(unix), not(windows)))]
pub(crate) fn gctx_call<Req, Resp>(
    method: &str,
    _request: &Req,
    _request_id: &str,
) -> Result<Resp, GctxDaemonError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    tracing::debug!(
        target: "anvil_mcp::gctx",
        method,
        "GCTX daemon client unavailable on this platform"
    );
    Err(GctxDaemonError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_and_macos_peer_validation_failures_remain_hard_failures() {
        assert_eq!(
            classify_peer_validation_failure(PeerCredentialPlatform::LinuxOrMacos),
            GctxDaemonError::Failure
        );
    }

    #[test]
    fn other_unix_peer_validation_failures_degrade_to_unavailable() {
        assert_eq!(
            classify_peer_validation_failure(PeerCredentialPlatform::OtherUnix),
            GctxDaemonError::Unavailable
        );
    }
}
