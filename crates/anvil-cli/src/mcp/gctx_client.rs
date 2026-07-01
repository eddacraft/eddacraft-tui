//! Shared daemon JSON-RPC client for the line-framed `anvil/*` socket surface.
//!
//! Callers pass a sealed request DTO and receive a sealed daemon response DTO
//! over a single line-framed JSON-RPC exchange. The GCTX MCP tools/resources use
//! it for the read-only `anvil/gctx/*` methods; the `anvil hook` witness path
//! (MLP2-005 phase 3) reuses the same transport for `anvil/witness/append`.
//! Transport failures that mean “no usable daemon surface” (absent socket /
//! `Method not found`) classify as [`DaemonRpcError::Unavailable`]; malformed
//! replies and security-relevant failures classify as [`DaemonRpcError::Failure`].

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

/// Why a daemon JSON-RPC request could not complete.
///
/// `Unavailable` (socket absent / `Method not found`) degrades to a structured
/// `unavailable` outcome; `Failure` (a malformed reply, an IO error mid-exchange,
/// or a security-relevant validation failure) is a tool/resource error. The
/// witness-append hook path treats *both* as "no durable daemon result" and falls
/// back to the embedded writer (the daemon is a pure optimisation there), so it
/// does not distinguish the two variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) enum DaemonRpcError {
    Unavailable,
    Failure,
}

#[cfg(unix)]
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

#[cfg(unix)]
fn classify_peer_validation_failure(platform: PeerCredentialPlatform) -> DaemonRpcError {
    match platform {
        PeerCredentialPlatform::LinuxOrMacos => DaemonRpcError::Failure,
        // CIB-099: non-Linux/macOS Unix builds do not have the same peer-credential
        // implementation. Treat that validation failure as an unavailable daemon
        // transport so callers degrade consistently instead of surfacing a hard
        // failure — the GCTX tools return `unavailable`; the witness hook falls
        // back to the embedded writer.
        PeerCredentialPlatform::OtherUnix => DaemonRpcError::Unavailable,
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

/// Forward a sealed request to the daemon over a line-framed JSON-RPC exchange
/// on the Unix socket and deserialise the sealed response. Method-agnostic: the
/// `anvil/gctx/*` reads and `anvil/witness/append` share this transport.
#[cfg(unix)]
pub(crate) fn daemon_rpc_call<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
) -> Result<Resp, DaemonRpcError>
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

    let socket_path = ipc::resolve_socket_path().map_err(|_| DaemonRpcError::Unavailable)?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
        return match err {
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(DaemonRpcError::Unavailable)
            }
            _ => {
                eprintln!("anvil-daemon: {method} socket unavailable: {err}");
                Err(DaemonRpcError::Failure)
            }
        };
    }
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        eprintln!("anvil-daemon: {method} connect failed: {err}");
        DaemonRpcError::Unavailable
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        eprintln!("anvil-daemon: {method} peer rejected: {err}");
        classify_peer_validation_failure(PEER_CREDENTIAL_PLATFORM)
    })?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-daemon: {method} read-timeout setup failed: {err}");
        DaemonRpcError::Failure
    })?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(|err| {
        eprintln!("anvil-daemon: {method} write-timeout setup failed: {err}");
        DaemonRpcError::Failure
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
        eprintln!("anvil-daemon: {method} request write failed: {err}");
        return Err(DaemonRpcError::Failure);
    }

    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_CAP + 1)
        .read_until(b'\n', &mut line)
        .map_err(|err| {
            eprintln!("anvil-daemon: {method} response read failed: {err}");
            DaemonRpcError::Failure
        })?;
    if read == 0 || line.len() as u64 > RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
        eprintln!("anvil-daemon: {method} response was empty, oversized, or unframed");
        return Err(DaemonRpcError::Failure);
    }
    let line = String::from_utf8(line).map_err(|_| {
        eprintln!("anvil-daemon: {method} response was not UTF-8");
        DaemonRpcError::Failure
    })?;

    let envelope: GctxRpcEnvelope<Resp> = serde_json::from_str(&line).map_err(|err| {
        eprintln!("anvil-daemon: {method} response parse failed: {err}");
        DaemonRpcError::Failure
    })?;
    if envelope.id.as_deref() != Some(request_id) {
        eprintln!("anvil-daemon: {method} response id mismatch");
        return Err(DaemonRpcError::Failure);
    }
    if let Some(error) = envelope.error {
        return if error.code == -32601 {
            Err(DaemonRpcError::Unavailable)
        } else {
            eprintln!("anvil-daemon: {method} daemon error {}", error.code);
            Err(DaemonRpcError::Failure)
        };
    }
    envelope.result.ok_or_else(|| {
        eprintln!("anvil-daemon: {method} response carried neither result nor error");
        DaemonRpcError::Failure
    })
}

/// Forward a sealed request to the daemon over the Windows owner-only named
/// pipe. This mirrors the Unix socket JSON-RPC contract while bounding the whole
/// synchronous pipe exchange on a worker thread because Win32 pipe reads/writes
/// do not expose the same per-stream timeout setters.
#[cfg(windows)]
pub(crate) fn daemon_rpc_call<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
) -> Result<Resp, DaemonRpcError>
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
        eprintln!("anvil-daemon: {method} pipe unavailable: {err}");
        DaemonRpcError::Unavailable
    })?;
    let params = serde_json::to_value(request).map_err(|err| {
        eprintln!("anvil-daemon: {method} request serialise failed: {err}");
        DaemonRpcError::Failure
    })?;
    let method = method.to_owned();
    let method_label = method.clone();
    let request_id = request_id.to_owned();
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let outcome: Result<serde_json::Value, DaemonRpcError> = (|| {
            let mut client = anvil_intercept_win32::connect_owner_only_pipe_client(&pipe_name)
                .map_err(|err| {
                    eprintln!("anvil-daemon: {method} pipe connect failed: {err}");
                    DaemonRpcError::Unavailable
                })?;
            let mut frame = json!({
                "jsonrpc": "2.0",
                "method": method.as_str(),
                "params": params,
                "id": request_id.as_str(),
            });
            crate::usage::attach_principal(&mut frame);
            if let Err(err) = writeln!(client, "{frame}").and_then(|()| client.flush()) {
                eprintln!("anvil-daemon: {method} pipe request write failed: {err}");
                return Err(DaemonRpcError::Failure);
            }

            let mut reader = BufReader::new(client);
            let mut line = Vec::new();
            let read = reader
                .by_ref()
                .take(RESPONSE_LINE_CAP + 1)
                .read_until(b'\n', &mut line)
                .map_err(|err| {
                    eprintln!("anvil-daemon: {method} pipe response read failed: {err}");
                    DaemonRpcError::Failure
                })?;
            if read == 0 || line.len() as u64 > RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
                eprintln!("anvil-daemon: {method} pipe response was empty, oversized, or unframed");
                return Err(DaemonRpcError::Failure);
            }
            let line = String::from_utf8(line).map_err(|_| {
                eprintln!("anvil-daemon: {method} pipe response was not UTF-8");
                DaemonRpcError::Failure
            })?;
            let envelope: GctxRpcEnvelope<serde_json::Value> = serde_json::from_str(&line)
                .map_err(|err| {
                    eprintln!("anvil-daemon: {method} pipe response parse failed: {err}");
                    DaemonRpcError::Failure
                })?;
            if envelope.id.as_deref() != Some(request_id.as_str()) {
                eprintln!("anvil-daemon: {method} pipe response id mismatch");
                return Err(DaemonRpcError::Failure);
            }
            if let Some(error) = envelope.error {
                return if error.code == -32601 {
                    Err(DaemonRpcError::Unavailable)
                } else {
                    eprintln!("anvil-daemon: {method} pipe daemon error {}", error.code);
                    Err(DaemonRpcError::Failure)
                };
            }
            envelope.result.ok_or_else(|| {
                eprintln!("anvil-daemon: {method} pipe response carried neither result nor error");
                DaemonRpcError::Failure
            })
        })();
        let _ = tx.send(outcome);
    });

    let value = match rx.recv_timeout(TIMEOUT) {
        Ok(outcome) => outcome?,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            eprintln!("anvil-daemon: {method_label} pipe request timed out");
            return Err(DaemonRpcError::Unavailable);
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => return Err(DaemonRpcError::Unavailable),
    };
    serde_json::from_value(value).map_err(|err| {
        eprintln!("anvil-daemon: {method_label} pipe response decode failed: {err}");
        DaemonRpcError::Failure
    })
}

/// Non-Unix, non-Windows targets have no daemon transport.
#[cfg(all(not(unix), not(windows)))]
pub(crate) fn daemon_rpc_call<Req, Resp>(
    method: &str,
    _request: &Req,
    _request_id: &str,
) -> Result<Resp, DaemonRpcError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    tracing::debug!(
        target: "anvil::daemon",
        method,
        "daemon transport unavailable on this platform"
    );
    Err(DaemonRpcError::Unavailable)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn linux_and_macos_peer_validation_failures_remain_hard_failures() {
        assert_eq!(
            classify_peer_validation_failure(PeerCredentialPlatform::LinuxOrMacos),
            DaemonRpcError::Failure
        );
    }

    #[test]
    fn other_unix_peer_validation_failures_degrade_to_unavailable() {
        assert_eq!(
            classify_peer_validation_failure(PeerCredentialPlatform::OtherUnix),
            DaemonRpcError::Unavailable
        );
    }
}
