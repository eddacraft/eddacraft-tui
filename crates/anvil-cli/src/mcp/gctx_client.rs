//! Shared daemon JSON-RPC client for the line-framed `anvil/*` socket surface.
//!
//! Callers pass a sealed request DTO and receive a sealed daemon response DTO
//! over a single line-framed JSON-RPC exchange. The GCTX MCP tools/resources use
//! it for the read-only `anvil/gctx/*` methods; the `anvil hook` witness path
//! (MLP2-005 phase 3) reuses the same transport for `anvil/witness/append`.
//! Transport failures that mean “no usable daemon surface” (an absent socket,
//! a stale refused socket for a non-validation method, or `Method not found` for a non-validation method) classify as
//! [`DaemonRpcError::Unavailable`]; malformed replies and security-relevant
//! failures classify as [`DaemonRpcError::Failure`].

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
#[cfg(unix)]
use std::sync::atomic::Ordering;

/// Why a daemon JSON-RPC request could not complete.
///
/// `Unavailable` (socket absent, or a non-validation method that the daemon does
/// not implement) degrades to a structured `unavailable` outcome; `Failure` (a
/// malformed reply, an IO error mid-exchange, or a security-relevant validation
/// failure) is a tool/resource error. The witness-append hook path treats *both*
/// as "no durable daemon result" and falls back to the embedded writer (the
/// daemon is a pure optimisation there), so it does not distinguish the two
/// variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) enum DaemonRpcError {
    Unavailable,
    Failure,
}

/// How transport failures are surfaced to the operator.
///
/// GCTX MCP tools keep [`Self::Surface`] so a dead daemon stays visible.
/// The hook witness path uses [`Self::Quiet`] because it always falls
/// back to the embedded writer (CIB-345).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DaemonRpcNoise {
    Surface,
    Quiet,
}

fn emit_daemon_rpc_line(noise: DaemonRpcNoise, method: &str, detail: &str) {
    match noise {
        DaemonRpcNoise::Quiet => {
            tracing::debug!(target: "anvil::daemon", method, "{detail}");
        }
        DaemonRpcNoise::Surface => {
            #[cfg(test)]
            if capture_surface_line(method, detail) {
                return;
            }
            eprintln!("anvil-daemon: {method} {detail}");
        }
    }
}

#[cfg(test)]
thread_local! {
    static RPC_SURFACE_CAPTURE: std::cell::RefCell<Option<Vec<String>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn capture_surface_line(method: &str, detail: &str) -> bool {
    RPC_SURFACE_CAPTURE.with(|slot| {
        slot.borrow_mut().as_mut().is_some_and(|buf| {
            buf.push(format!("anvil-daemon: {method} {detail}"));
            true
        })
    })
}

/// Record surface RPC lines emitted on this thread while `f` runs.
///
/// Used by hook tests to prove the quiet witness path does not dump
/// pipe/connect failures when the embedded fallback will run.
#[cfg(test)]
pub(crate) fn capture_rpc_surface_lines<F, T>(f: F) -> (T, Vec<String>)
where
    F: FnOnce() -> T,
{
    RPC_SURFACE_CAPTURE.with(|slot| {
        *slot.borrow_mut() = Some(Vec::new());
    });
    let out = f();
    let lines = RPC_SURFACE_CAPTURE.with(|slot| slot.borrow_mut().take().unwrap_or_default());
    (out, lines)
}

#[cfg(any(unix, windows))]
fn classify_connect_error(method: &str, kind: std::io::ErrorKind) -> DaemonRpcError {
    if kind == std::io::ErrorKind::NotFound
        || (kind == std::io::ErrorKind::ConnectionRefused && method != "scan_buffer")
    {
        DaemonRpcError::Unavailable
    } else {
        DaemonRpcError::Failure
    }
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
        // An unimplemented or rejected identity check is never equivalent to an
        // absent transport. Callers that enforce writes must fail closed.
        PeerCredentialPlatform::LinuxOrMacos | PeerCredentialPlatform::OtherUnix => {
            DaemonRpcError::Failure
        }
    }
}

#[cfg(any(unix, windows))]
#[derive(serde::Deserialize)]
struct GctxRpcEnvelope<R> {
    #[serde(default)]
    jsonrpc: Option<String>,
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

#[cfg(any(unix, windows))]
fn decode_rpc_response<R>(
    line: &str,
    method: &str,
    request_id: &str,
    noise: DaemonRpcNoise,
) -> Result<R, DaemonRpcError>
where
    R: DeserializeOwned,
{
    let envelope: GctxRpcEnvelope<R> = serde_json::from_str(line).map_err(|err| {
        emit_daemon_rpc_line(noise, method, &format!("response parse failed: {err}"));
        DaemonRpcError::Failure
    })?;
    if envelope.jsonrpc.as_deref() != Some("2.0")
        || envelope.id.as_deref() != Some(request_id)
        || envelope.result.is_some() == envelope.error.is_some()
    {
        emit_daemon_rpc_line(noise, method, "response envelope was invalid");
        return Err(DaemonRpcError::Failure);
    }
    if let Some(error) = envelope.error {
        emit_daemon_rpc_line(noise, method, &format!("daemon error {}", error.code));
        if error.code == -32601 && method != "scan_buffer" {
            return Err(DaemonRpcError::Unavailable);
        }
        return Err(DaemonRpcError::Failure);
    }
    envelope.result.ok_or(DaemonRpcError::Failure)
}

/// Forward a sealed request to the daemon over a line-framed JSON-RPC exchange
/// on the Unix socket and deserialise the sealed response. Method-agnostic: the
/// `anvil/gctx/*` reads and `anvil/witness/append` share this transport.
pub(crate) fn daemon_rpc_call<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
) -> Result<Resp, DaemonRpcError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    daemon_rpc_call_cancellable(method, request, request_id, None)
}

/// Same transport as [`daemon_rpc_call`], but transport failures stay off
/// stderr. Only the hook witness-append path uses this: that caller always
/// falls back to the embedded writer (CIB-345).
pub(crate) fn daemon_rpc_call_quiet<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
) -> Result<Resp, DaemonRpcError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    daemon_rpc_call_cancellable_with_noise(method, request, request_id, None, DaemonRpcNoise::Quiet)
}

#[cfg(unix)]
pub(crate) fn daemon_rpc_call_cancellable<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
    cancellation: Option<Arc<AtomicBool>>,
) -> Result<Resp, DaemonRpcError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    daemon_rpc_call_cancellable_with_noise(
        method,
        request,
        request_id,
        cancellation,
        DaemonRpcNoise::Surface,
    )
}

#[cfg(unix)]
// Call sites pass `Arc` so the cancel flag can outlive the RPC; taking by
// value keeps the public surface simple (same as the Windows path).
#[allow(
    clippy::needless_pass_by_value,
    reason = "callers own Arc; shared with Windows RPC surface — GH #3371"
)]
fn daemon_rpc_call_cancellable_with_noise<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
    cancellation: Option<Arc<AtomicBool>>,
    noise: DaemonRpcNoise,
) -> Result<Resp, DaemonRpcError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    use std::io::BufReader;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const TIMEOUT: Duration = Duration::from_secs(2);
    // GH #3371: short poll so cancel checks stay responsive without a watcher thread.
    const POLL: Duration = Duration::from_millis(25);
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
                emit_daemon_rpc_line(noise, method, &format!("socket unavailable: {err}"));
                Err(DaemonRpcError::Failure)
            }
        };
    }
    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        emit_daemon_rpc_line(noise, method, &format!("connect failed: {err}"));
        classify_connect_error(method, err.kind())
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        emit_daemon_rpc_line(noise, method, &format!("peer rejected: {err}"));
        classify_peer_validation_failure(PEER_CREDENTIAL_PLATFORM)
    })?;
    let deadline = std::time::Instant::now() + TIMEOUT;
    stream.set_read_timeout(Some(POLL)).map_err(|err| {
        emit_daemon_rpc_line(noise, method, &format!("read-timeout setup failed: {err}"));
        DaemonRpcError::Failure
    })?;
    stream.set_write_timeout(Some(POLL)).map_err(|err| {
        emit_daemon_rpc_line(noise, method, &format!("write-timeout setup failed: {err}"));
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
    // The legacy `scan_buffer` parser is intentionally a sealed JSON-RPC
    // surface and rejects the usage extension field. Its caller attribution is
    // handled by the daemon's authenticated transport boundary.
    if method != "scan_buffer" {
        crate::usage::attach_principal(&mut frame);
    }
    let request_bytes = format!("{frame}\n");
    if write_all_until_deadline(
        &mut stream,
        request_bytes.as_bytes(),
        deadline,
        cancellation.as_ref(),
    )
    .is_err()
    {
        emit_daemon_rpc_line(noise, method, "request write failed or cancelled");
        return Err(DaemonRpcError::Failure);
    }

    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    let read = read_until_newline_deadline(
        &mut reader,
        &mut line,
        RESPONSE_LINE_CAP + 1,
        deadline,
        cancellation.as_ref(),
    )
    .map_err(|err| {
        emit_daemon_rpc_line(noise, method, &format!("response read failed: {err}"));
        DaemonRpcError::Failure
    })?;
    if read == 0 || line.len() as u64 > RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
        emit_daemon_rpc_line(noise, method, "response was empty, oversized, or unframed");
        return Err(DaemonRpcError::Failure);
    }
    let line = String::from_utf8(line).map_err(|_| {
        emit_daemon_rpc_line(noise, method, "response was not UTF-8");
        DaemonRpcError::Failure
    })?;

    decode_rpc_response(&line, method, request_id, noise)
}

/// Write `bytes` honouring short write timeouts and optional cancellation.
#[cfg(unix)]
fn write_all_until_deadline(
    stream: &mut std::os::unix::net::UnixStream,
    mut bytes: &[u8],
    deadline: std::time::Instant,
    cancellation: Option<&Arc<AtomicBool>>,
) -> std::io::Result<()> {
    use std::io::Write;
    while !bytes.is_empty() {
        if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "daemon RPC write cancelled",
            ));
        }
        if std::time::Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "daemon RPC write deadline exceeded",
            ));
        }
        match stream.write(bytes) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "daemon RPC write returned zero",
                ));
            }
            Ok(n) => bytes = &bytes[n..],
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error),
        }
    }
    stream.flush()
}

/// Read one newline-terminated frame with short timeouts + cancel polls.
#[cfg(unix)]
fn read_until_newline_deadline(
    reader: &mut impl std::io::BufRead,
    line: &mut Vec<u8>,
    cap: u64,
    deadline: std::time::Instant,
    cancellation: Option<&Arc<AtomicBool>>,
) -> std::io::Result<usize> {
    use std::io::{BufRead as _, Read as _};
    line.clear();
    loop {
        if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "daemon RPC read cancelled",
            ));
        }
        if std::time::Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "daemon RPC read deadline exceeded",
            ));
        }
        let before = line.len();
        let remaining = cap.saturating_sub(before as u64);
        match reader.take(remaining).read_until(b'\n', line) {
            Ok(0) if line.is_empty() => return Ok(0),
            Ok(_) if line.ends_with(b"\n") || line.len() as u64 >= cap => return Ok(line.len()),
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error),
        }
    }
}

#[cfg(windows)]
const WINDOWS_RESPONSE_LINE_CAP: u64 = 4 << 20;
#[cfg(windows)]
const WINDOWS_RPC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[cfg(windows)]
// Taking the name by value keeps the three call sites (and the two tests)
// simple; the body borrows it. Changing the signature would cascade through
// call sites and the inner `connect_owner_only_overlapped_pipe_client(&..)`
// borrow, none of which a Linux host can type-check. CIB-204.
#[allow(
    clippy::needless_pass_by_value,
    reason = "Windows-only RPC helper; signature change is unverifiable from a Linux host — CIB-204"
)]
fn daemon_rpc_call_windows_at<Resp>(
    pipe_name: String,
    method: &str,
    params: serde_json::Value,
    request_id: &str,
    cancellation: Option<Arc<AtomicBool>>,
    noise: DaemonRpcNoise,
) -> Result<Resp, DaemonRpcError>
where
    Resp: DeserializeOwned,
{
    let deadline = std::time::Instant::now() + WINDOWS_RPC_TIMEOUT;
    let cancellation = cancellation.as_deref();
    let mut client = anvil_intercept_win32::connect_owner_only_overlapped_pipe_client(&pipe_name)
        .map_err(|error| {
        emit_daemon_rpc_line(noise, method, &format!("pipe connect failed: {error}"));
        classify_connect_error(method, error.kind())
    })?;
    let mut frame = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": request_id,
    });
    if method != "scan_buffer" {
        crate::usage::attach_principal(&mut frame);
    }
    let mut request_bytes = serde_json::to_vec(&frame).map_err(|error| {
        emit_daemon_rpc_line(noise, method, &format!("request serialise failed: {error}"));
        DaemonRpcError::Failure
    })?;
    request_bytes.push(b'\n');
    client
        .write_all_cancellable(&request_bytes, deadline, cancellation)
        .map_err(|error| {
            emit_daemon_rpc_line(
                noise,
                method,
                &format!("pipe request write failed: {error}"),
            );
            DaemonRpcError::Failure
        })?;

    let mut line = Vec::new();
    let mut chunk = [0_u8; 4096];
    while line.len() as u64 <= WINDOWS_RESPONSE_LINE_CAP {
        let read = client
            .read_cancellable(&mut chunk, deadline, cancellation)
            .map_err(|error| {
                emit_daemon_rpc_line(
                    noise,
                    method,
                    &format!("pipe response read failed: {error}"),
                );
                DaemonRpcError::Failure
            })?;
        if read == 0 {
            break;
        }
        line.extend_from_slice(&chunk[..read]);
        if let Some(newline) = line.iter().position(|byte| *byte == b'\n') {
            line.truncate(newline + 1);
            break;
        }
    }
    if line.is_empty() || line.len() as u64 > WINDOWS_RESPONSE_LINE_CAP || !line.ends_with(b"\n") {
        emit_daemon_rpc_line(
            noise,
            method,
            "pipe response was empty, oversized, or unframed",
        );
        return Err(DaemonRpcError::Failure);
    }
    let line = String::from_utf8(line).map_err(|_| {
        emit_daemon_rpc_line(noise, method, "pipe response was not UTF-8");
        DaemonRpcError::Failure
    })?;
    let value = decode_rpc_response(&line, method, request_id, noise)?;
    serde_json::from_value(value).map_err(|error| {
        emit_daemon_rpc_line(
            noise,
            method,
            &format!("pipe response decode failed: {error}"),
        );
        DaemonRpcError::Failure
    })
}

/// Forward a sealed request to the daemon over a Windows owner-authenticated,
/// overlapped named pipe. Each pending read/write is explicitly cancelled and
/// completion-drained at the shared absolute deadline or caller cancellation,
/// so no synchronous worker or kernel-owned request buffer can be stranded.
#[cfg(windows)]
pub(crate) fn daemon_rpc_call_cancellable<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
    cancellation: Option<Arc<AtomicBool>>,
) -> Result<Resp, DaemonRpcError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    daemon_rpc_call_cancellable_with_noise(
        method,
        request,
        request_id,
        cancellation,
        DaemonRpcNoise::Surface,
    )
}

#[cfg(windows)]
fn daemon_rpc_call_cancellable_with_noise<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
    cancellation: Option<Arc<AtomicBool>>,
    noise: DaemonRpcNoise,
) -> Result<Resp, DaemonRpcError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    let pipe_name = anvil_intercept::ipc::resolve_pipe_name().map_err(|err| {
        emit_daemon_rpc_line(noise, method, &format!("pipe unavailable: {err}"));
        DaemonRpcError::Unavailable
    })?;
    let params = serde_json::to_value(request).map_err(|err| {
        emit_daemon_rpc_line(noise, method, &format!("request serialise failed: {err}"));
        DaemonRpcError::Failure
    })?;
    daemon_rpc_call_windows_at(pipe_name, method, params, request_id, cancellation, noise)
}

/// Non-Unix, non-Windows targets have no daemon transport.
#[cfg(all(not(unix), not(windows)))]
pub(crate) fn daemon_rpc_call_cancellable<Req, Resp>(
    method: &str,
    request: &Req,
    request_id: &str,
    cancellation: Option<Arc<AtomicBool>>,
) -> Result<Resp, DaemonRpcError>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    daemon_rpc_call_cancellable_with_noise(
        method,
        request,
        request_id,
        cancellation,
        DaemonRpcNoise::Surface,
    )
}

#[cfg(all(not(unix), not(windows)))]
fn daemon_rpc_call_cancellable_with_noise<Req, Resp>(
    method: &str,
    _request: &Req,
    _request_id: &str,
    _cancellation: Option<Arc<AtomicBool>>,
    _noise: DaemonRpcNoise,
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
    fn stale_socket_degrades_only_for_non_validation_methods() {
        assert_eq!(
            classify_connect_error(
                "anvil/gctx/searchSymbols",
                std::io::ErrorKind::ConnectionRefused
            ),
            DaemonRpcError::Unavailable
        );
        assert_eq!(
            classify_connect_error("scan_buffer", std::io::ErrorKind::ConnectionRefused),
            DaemonRpcError::Failure
        );
    }

    #[test]
    fn linux_and_macos_peer_validation_failures_remain_hard_failures() {
        assert_eq!(
            classify_peer_validation_failure(PeerCredentialPlatform::LinuxOrMacos),
            DaemonRpcError::Failure
        );
    }

    #[test]
    fn other_unix_peer_validation_failures_remain_hard_failures() {
        assert_eq!(
            classify_peer_validation_failure(PeerCredentialPlatform::OtherUnix),
            DaemonRpcError::Failure
        );
    }

    #[test]
    fn received_daemon_errors_and_malformed_envelopes_fail_closed() {
        let method_not_found = r#"{"jsonrpc":"2.0","id":"scan-1","error":{"code":-32601}}"#;
        let missing_version = r#"{"id":"scan-1","result":{}}"#;
        let ambiguous = r#"{"jsonrpc":"2.0","id":"scan-1","result":{},"error":{"code":-1}}"#;

        for response in [method_not_found, missing_version, ambiguous] {
            assert_eq!(
                decode_rpc_response::<serde_json::Value>(
                    response,
                    "scan_buffer",
                    "scan-1",
                    DaemonRpcNoise::Surface,
                ),
                Err(DaemonRpcError::Failure)
            );
        }
    }

    #[test]
    fn legacy_method_not_found_remains_unavailable_for_non_validation_calls() {
        let method_not_found = r#"{"jsonrpc":"2.0","id":"gctx-1","error":{"code":-32601}}"#;

        assert_eq!(
            decode_rpc_response::<serde_json::Value>(
                method_not_found,
                "anvil/gctx/search_symbols",
                "gctx-1",
                DaemonRpcNoise::Surface,
            ),
            Err(DaemonRpcError::Unavailable)
        );
    }

    #[test]
    fn quiet_noise_does_not_record_a_surface_line() {
        let ((), lines) = capture_rpc_surface_lines(|| {
            emit_daemon_rpc_line(
                DaemonRpcNoise::Quiet,
                "anvil/witness/append",
                "connect failed: Connection refused",
            );
        });
        assert!(
            lines.is_empty(),
            "quiet witness transport must not dump pipe/connect lines: {lines:?}"
        );
    }

    #[test]
    fn surface_noise_records_the_raw_transport_line() {
        let ((), lines) = capture_rpc_surface_lines(|| {
            emit_daemon_rpc_line(
                DaemonRpcNoise::Surface,
                "anvil/witness/append",
                "connect failed: Connection refused",
            );
        });
        assert_eq!(
            lines,
            vec![
                "anvil-daemon: anvil/witness/append connect failed: Connection refused".to_string()
            ]
        );
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    use serde_json::json;

    use super::*;

    static WINDOWS_TRANSPORT_TEST_LOCK: Mutex<()> = Mutex::new(());
    const LEGACY_WORKER_LIMIT: usize = 8;

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("tokio runtime")
    }

    fn non_responding_server(
        runtime: &tokio::runtime::Runtime,
        pipe_name: &str,
    ) -> (tokio::task::JoinHandle<()>, mpsc::Receiver<()>) {
        let server = {
            let _guard = runtime.enter();
            anvil_intercept_win32::create_owner_only_pipe_server(
                pipe_name,
                anvil_intercept_win32::PipeInstance::First,
            )
            .expect("bind owner-only test pipe")
        };
        let (accepted_tx, accepted_rx) = mpsc::sync_channel(1);
        let task = runtime.spawn(async move {
            server.connect().await.expect("accept test client");
            let _ = accepted_tx.send(());
            std::future::pending::<()>().await;
        });
        (task, accepted_rx)
    }

    #[test]
    fn windows_gctx_round_trip_deserialises_typed_result() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let _test_guard = WINDOWS_TRANSPORT_TEST_LOCK
            .lock()
            .expect("transport test lock");
        let runtime = runtime();
        let pipe_name = format!(r"\\.\pipe\anvil-gctx-success-test-{}", std::process::id());
        let server = {
            let _guard = runtime.enter();
            anvil_intercept_win32::create_owner_only_pipe_server(
                &pipe_name,
                anvil_intercept_win32::PipeInstance::First,
            )
            .expect("bind owner-only test pipe")
        };

        let server_task = runtime.spawn(async move {
            let server = server;
            server.connect().await.expect("accept test client");
            let mut reader = BufReader::new(server);
            let mut request_line = String::new();
            reader
                .read_line(&mut request_line)
                .await
                .expect("read newline-framed request");
            assert!(
                request_line.ends_with('\n'),
                "request must be newline framed"
            );
            let request: serde_json::Value =
                serde_json::from_str(&request_line).expect("parse JSON-RPC request");
            assert_eq!(
                request["method"],
                anvil_intercept_proto::protocol::ANVIL_GCTX_GRAPH_STATS
            );
            assert_eq!(request["id"], "windows-gctx-success");

            let response = json!({
                "jsonrpc": "2.0",
                "id": "windows-gctx-success",
                "result": {
                    "workspace_assurance": {
                        "state": "clean",
                        "generation": 0,
                    },
                    "outcome": {
                        "status": "ready",
                        "symbol_count": 7,
                        "symbol_edge_count": 0,
                        "file_count": 0,
                        "dependency_edge_count": 0,
                    },
                },
            });
            reader
                .get_mut()
                .write_all(format!("{response}\n").as_bytes())
                .await
                .expect("write newline-framed response");
            reader.get_mut().shutdown().await.expect("server shutdown");
        });

        let result: Result<
            anvil_intercept_proto::protocol::GctxGraphStatsResponse,
            DaemonRpcError,
        > = daemon_rpc_call_windows_at(
            pipe_name,
            anvil_intercept_proto::protocol::ANVIL_GCTX_GRAPH_STATS,
            json!({"workspace_root": r"C:\gctx-test"}),
            "windows-gctx-success",
            None,
            DaemonRpcNoise::Surface,
        );
        runtime
            .block_on(async { tokio::time::timeout(Duration::from_secs(1), server_task).await })
            .expect("server task joins within deadline")
            .expect("server task joins");
        let response = result.expect("typed graph-stats response");
        assert_eq!(
            response.workspace_assurance.state,
            anvil_intercept_proto::protocol::AssuranceState::Clean
        );
        assert_eq!(
            response.outcome,
            anvil_gctx_types::GraphStatsOutcome::Ready(anvil_gctx_types::GraphStatsProjection {
                symbol_count: 7,
                symbol_edge_count: 0,
                file_count: 0,
                dependency_edge_count: 0,
            })
        );
    }

    #[test]
    fn repeated_windows_cancellation_drains_beyond_the_legacy_worker_limit() {
        let _test_guard = WINDOWS_TRANSPORT_TEST_LOCK
            .lock()
            .expect("transport test lock");
        let runtime = runtime();

        for attempt in 0..(LEGACY_WORKER_LIMIT + 2) {
            let pipe_name = format!(
                r"\\.\pipe\anvil-gctx-cancel-test-{}-{attempt}",
                std::process::id(),
            );
            let (server_task, accepted_rx) = non_responding_server(&runtime, &pipe_name);
            let cancellation = Arc::new(AtomicBool::new(false));
            let cancellation_signal = Arc::clone(&cancellation);
            let canceller = thread::spawn(move || {
                accepted_rx
                    .recv_timeout(Duration::from_secs(1))
                    .expect("every call must acquire a permit and connect");
                cancellation_signal.store(true, Ordering::Release);
            });

            let started = Instant::now();
            let result: Result<serde_json::Value, DaemonRpcError> = daemon_rpc_call_windows_at(
                pipe_name,
                "scan_buffer",
                json!({"mode": "midEdit", "path": "main.rs", "text": "x".repeat(1024 * 1024)}),
                &format!("cancel-{attempt}"),
                Some(cancellation),
                DaemonRpcNoise::Surface,
            );
            assert_eq!(result, Err(DaemonRpcError::Failure));
            assert!(
                started.elapsed() < Duration::from_secs(1),
                "caller cancellation must return promptly"
            );
            canceller.join().expect("canceller joins");
            server_task.abort();
        }
    }

    #[test]
    fn windows_timeout_is_bounded_and_drains_the_pending_operation() {
        let _test_guard = WINDOWS_TRANSPORT_TEST_LOCK
            .lock()
            .expect("transport test lock");
        let runtime = runtime();
        let pipe_name = format!(r"\\.\pipe\anvil-gctx-timeout-test-{}", std::process::id());
        let (server_task, accepted_rx) = non_responding_server(&runtime, &pipe_name);

        let started = Instant::now();
        let result: Result<serde_json::Value, DaemonRpcError> = daemon_rpc_call_windows_at(
            pipe_name,
            "scan_buffer",
            json!({"mode": "midEdit", "path": "main.rs", "text": "fn main() {}"}),
            "timeout",
            None,
            DaemonRpcNoise::Surface,
        );
        let elapsed = started.elapsed();
        assert_eq!(result, Err(DaemonRpcError::Failure));
        accepted_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("timed request connects to the server");
        assert!(
            elapsed <= WINDOWS_RPC_TIMEOUT + Duration::from_millis(100),
            "two-second RPC deadline must remain bounded; elapsed {elapsed:?}"
        );
        server_task.abort();
    }
}
