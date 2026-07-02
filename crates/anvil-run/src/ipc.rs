//! Synchronous JSON-RPC client for the launcher.
//!
//! The launcher is a single-shot process and the daemon's wire format
//! is line-delimited JSON-RPC 2.0 (see
//! `crates/anvil-intercept/src/ipc.rs`). Pulling in a tokio runtime
//! for what amounts to four request/response round-trips would be
//! wasted weight, so this module wraps the existing daemon
//! discovery helpers with a blocking client.
//!
//! Wire framing matches the existing CLI status command (see
//! `crates/anvil-cli/src/commands/intercept.rs`): one JSON-RPC frame
//! per line, terminated with `\n`. The daemon caps a response line at
//! 1 MiB; this client mirrors that cap so a misbehaving peer cannot
//! exhaust launcher memory.

#[cfg(unix)]
use std::io::Write;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Wall-clock deadline applied to a single JSON-RPC round-trip. The
/// launcher does not retry on timeout — daemon contention or a
/// hung handler should surface as a clear refusal, not a hang.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

/// Cap on a single response line. Anything larger trips
/// [`ClientError::ResponseTooLarge`] — same posture as the daemon's
/// own listener cap.
pub const RESPONSE_LINE_BYTES: u64 = 1 << 20;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("the anvil intercept daemon is not running (no socket at {path})")]
    DaemonNotRunning { path: PathBuf },
    #[error("the anvil intercept daemon refused the connection: {reason}")]
    DaemonRefused { reason: String },
    #[error("IO error while talking to the daemon: {0}")]
    Io(#[from] std::io::Error),
    #[error("daemon response was not valid JSON: {0}")]
    BadJson(String),
    #[error("daemon response exceeded the {RESPONSE_LINE_BYTES}-byte response cap")]
    ResponseTooLarge,
    #[error("daemon returned a JSON-RPC error: {0}")]
    JsonRpc(String),
    #[error("daemon response had no `result` field: {0}")]
    MissingResult(String),
    #[error("daemon endpoint (socket path or pipe name) could not be resolved: {0}")]
    SocketPath(String),
}

/// Resolve the per-user daemon socket path (Unix) or pipe name
/// (Windows). Mirrors the canonical algorithm used by the daemon
/// itself so the launcher cannot drift onto a different rendezvous
/// when only `$XDG_RUNTIME_DIR` is set.
#[cfg(unix)]
pub fn resolve_endpoint() -> Result<PathBuf, ClientError> {
    anvil_intercept::ipc::resolve_socket_path()
        .map_err(|err| ClientError::SocketPath(err.to_string()))
}

#[cfg(windows)]
pub fn resolve_endpoint() -> Result<String, ClientError> {
    anvil_intercept::ipc::resolve_pipe_name()
        .map_err(|err| ClientError::SocketPath(err.to_string()))
}

/// Connect to the daemon, send a single JSON-RPC frame, and return
/// the parsed `result` value. The caller decides how to interpret
/// the result; helpers below specialise this for `query_status`,
/// `session.register`, etc.
///
/// On Unix the socket path is validated for owner/mode before
/// connecting — same posture as the daemon's listener, and same
/// posture as `anvil intercept status`. On Windows the helper opens
/// the per-user named pipe synchronously.
pub fn request<R: DeserializeOwned>(method: &str, params: &Value, id: &str) -> Result<R> {
    let body = jsonrpc_request_line(method, params, id);
    let response = round_trip(&body)?;
    let value: Value = serde_json::from_str(&response).map_err(|err| {
        anyhow!(ClientError::BadJson(format!(
            "method={method}: {err} ({response})"
        )))
    })?;
    validate_jsonrpc_envelope(&value, id)
        .with_context(|| format!("daemon response for {method} failed validation"))?;
    let result = value
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow!(ClientError::MissingResult(value.to_string())))?;
    serde_json::from_value(result)
        .with_context(|| format!("daemon response for {method} did not match the expected shape"))
}

/// Send a JSON-RPC notification (no `id`). Used by the heartbeat
/// loop, where we explicitly do not want a response.
pub fn notify(method: &str, params: &Value) -> Result<()> {
    let body = jsonrpc_notification_line(method, params);
    send_one_way(&body)
}

/// Build a JSON-RPC request line (including the trailing `\n`).
/// Public for tests; otherwise call [`request`] which uses it.
#[must_use]
pub fn jsonrpc_request_line(method: &str, params: &Value, id: &str) -> Vec<u8> {
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id,
    });
    let mut bytes = frame.to_string().into_bytes();
    bytes.push(b'\n');
    bytes
}

/// Build a JSON-RPC notification line — same shape minus the `id`.
#[must_use]
pub fn jsonrpc_notification_line(method: &str, params: &Value) -> Vec<u8> {
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    let mut bytes = frame.to_string().into_bytes();
    bytes.push(b'\n');
    bytes
}

/// Validate the framing fields of a JSON-RPC response. Pulled out so
/// tests can exercise the validation path without a live daemon.
pub fn validate_jsonrpc_envelope(value: &Value, request_id: &str) -> Result<()> {
    if value.get("jsonrpc") != Some(&Value::String("2.0".to_string())) {
        bail!("missing or wrong jsonrpc version: {value}");
    }
    if value.get("id") != Some(&Value::String(request_id.to_string())) {
        bail!("response id does not match request (expected {request_id:?}): {value}");
    }
    if let Some(error) = value.get("error") {
        return Err(anyhow!(ClientError::JsonRpc(error.to_string())));
    }
    Ok(())
}

#[cfg(unix)]
fn round_trip(body: &[u8]) -> Result<String> {
    use std::os::unix::net::UnixStream;

    let path = resolve_endpoint()?;
    if let Err(err) = anvil_intercept::ipc::validate_socket_path_for_client(&path) {
        return match err {
            anvil_intercept::ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(anyhow!(ClientError::DaemonNotRunning { path }))
            }
            other => Err(anyhow!(ClientError::DaemonRefused {
                reason: other.to_string(),
            })),
        };
    }
    let mut stream = UnixStream::connect(&path)
        .map_err(ClientError::Io)
        .with_context(|| format!("failed to connect to {}", path.display()))?;
    anvil_intercept::ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        anyhow!(ClientError::DaemonRefused {
            reason: err.to_string(),
        })
    })?;
    stream.set_read_timeout(Some(REQUEST_TIMEOUT))?;
    stream.set_write_timeout(Some(REQUEST_TIMEOUT))?;
    stream.write_all(body)?;
    stream.flush()?;
    read_one_line(stream)
}

#[cfg(windows)]
fn round_trip(body: &[u8]) -> Result<String> {
    use std::sync::mpsc;
    use std::thread;

    let pipe_name = resolve_endpoint()?;
    let pipe_name_clone = pipe_name.clone();
    let (tx, rx) = mpsc::sync_channel::<Result<String>>(1);
    let body_owned = body.to_vec();
    thread::spawn(move || {
        let result = (|| {
            let mut client = anvil_intercept_win32::connect_owner_only_pipe_client(
                &pipe_name_clone,
            )
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    anyhow!(ClientError::DaemonNotRunning {
                        path: PathBuf::from(&pipe_name_clone),
                    })
                } else {
                    anyhow!(ClientError::DaemonRefused {
                        reason: err.to_string(),
                    })
                }
            })?;
            client.write_all(&body_owned)?;
            read_one_line(client)
        })();
        let _ = tx.send(result);
    });
    match rx.recv_timeout(REQUEST_TIMEOUT) {
        Ok(result) => result,
        Err(_) => Err(anyhow!(
            "timed out talking to the daemon on pipe {pipe_name}"
        )),
    }
}

#[cfg(unix)]
fn send_one_way(body: &[u8]) -> Result<()> {
    use std::os::unix::net::UnixStream;

    let path = resolve_endpoint()?;
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("failed to connect to {}", path.display()))?;
    stream.set_write_timeout(Some(REQUEST_TIMEOUT))?;
    stream.write_all(body)?;
    stream.flush()?;
    // The daemon may write nothing back for a notification; drop
    // the stream so the connection closes cleanly.
    Ok(())
}

#[cfg(windows)]
fn send_one_way(body: &[u8]) -> Result<()> {
    let pipe_name = resolve_endpoint()?;
    let mut client = anvil_intercept_win32::connect_owner_only_pipe_client(&pipe_name)
        .with_context(|| format!("failed to connect to {pipe_name}"))?;
    client.write_all(body)?;
    Ok(())
}

fn read_one_line<R: Read>(stream: R) -> Result<String> {
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_BYTES + 1)
        .read_until(b'\n', &mut buf)?;
    if read == 0 {
        bail!("daemon closed the connection before responding");
    }
    if (buf.len() as u64) > RESPONSE_LINE_BYTES {
        return Err(anyhow!(ClientError::ResponseTooLarge));
    }
    let line = std::str::from_utf8(trim_trailing_newline(&buf))
        .map_err(|_| anyhow!(ClientError::BadJson("response was not valid UTF-8".into())))?;
    Ok(line.to_owned())
}

fn trim_trailing_newline(buf: &[u8]) -> &[u8] {
    let mut end = buf.len();
    while end > 0 && (buf[end - 1] == b'\n' || buf[end - 1] == b'\r') {
        end -= 1;
    }
    &buf[..end]
}

/// Wall-clock-free helper: build the bytes the launcher would send
/// to register a session. Pulled out so tests can pin the wire
/// shape without a live daemon.
///
/// **MLP2-025c (2026-05-16):** the daemon's MLP2-023+ parser
/// expects a nested `agent_tag` object and an optional nested
/// `lineage` object; until this revision the launcher only sent
/// flat `driver_id` / `claimed_agent_id` / `pid_starttime` keys
/// the daemon silently ignored — meaning no production session
/// has had a tag or lineage anchor since MLP2-023 shipped. The
/// flat keys are kept for backward visibility (they harm
/// nothing) and the nested objects are added alongside.
#[allow(clippy::too_many_arguments)]
// MLP2-025c added launcher_pid for the lineage anchor; the surface mirrors the wire shape and is not naturally bundleable.
#[must_use]
pub fn session_register_params(
    session_id: &str,
    worktree: &Path,
    cwd: &Path,
    driver_id: &str,
    claimed_agent_id: &str,
    pid_starttime: u64,
    tmux_pane: Option<&str>,
    // MLP2-025c: launcher's own PID for the lineage anchor. The
    // launcher reports `std::process::id()` at register time;
    // the daemon trusts this claim per the MLP2-025b spec §7.
    launcher_pid: u32,
) -> Value {
    let mut params = serde_json::Map::new();
    params.insert("session_id".into(), Value::String(session_id.into()));
    params.insert(
        "worktree".into(),
        Value::String(worktree.to_string_lossy().into_owned()),
    );
    params.insert(
        "cwd".into(),
        Value::String(cwd.to_string_lossy().into_owned()),
    );
    params.insert("driver_id".into(), Value::String(driver_id.into()));
    params.insert(
        "claimed_agent_id".into(),
        Value::String(claimed_agent_id.into()),
    );
    params.insert(
        "pid_starttime".into(),
        Value::Number(serde_json::Number::from(pid_starttime)),
    );
    if let Some(pane) = tmux_pane {
        params.insert("tmux_pane".into(), Value::String(pane.into()));
    }

    // MLP2-025c: nested `agent_tag` object the daemon's MLP2-023
    // parser actually reads. Same `(driver_id, claimed_agent_id,
    // pid_starttime)` triple as the flat fields above — duplication
    // is intentional and bounded; both shapes will coexist until a
    // future cleanup confirms the flat fields are unused.
    let mut agent_tag = serde_json::Map::new();
    agent_tag.insert("driver_id".into(), Value::String(driver_id.into()));
    agent_tag.insert(
        "claimed_agent_id".into(),
        Value::String(claimed_agent_id.into()),
    );
    agent_tag.insert(
        "pid_starttime".into(),
        Value::Number(serde_json::Number::from(pid_starttime)),
    );
    params.insert("agent_tag".into(), Value::Object(agent_tag));

    // MLP2-025c: nested `lineage` anchor for the daemon's
    // (pid, pid_starttime) lineage index. The launcher's own PID
    // here is what the daemon walks back to from the writer's
    // PID at write time — match means trusted attribution.
    let mut lineage = serde_json::Map::new();
    lineage.insert(
        "pid".into(),
        Value::Number(serde_json::Number::from(launcher_pid)),
    );
    lineage.insert(
        "pid_starttime".into(),
        Value::Number(serde_json::Number::from(pid_starttime)),
    );
    params.insert("lineage".into(), Value::Object(lineage));

    Value::Object(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_line_is_a_complete_jsonrpc_2_0_object() {
        let bytes = jsonrpc_request_line(
            "session.register",
            &serde_json::json!({"session_id": "s1"}),
            "req-1",
        );
        assert!(bytes.ends_with(b"\n"), "must be newline-framed");
        let line = std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap();
        let value: Value = serde_json::from_str(line).unwrap();
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["method"], "session.register");
        assert_eq!(value["id"], "req-1");
        assert_eq!(value["params"]["session_id"], "s1");
    }

    #[test]
    fn notification_line_has_no_id_field() {
        let bytes = jsonrpc_notification_line("heartbeat", &serde_json::json!({"session_id": "s"}));
        let line = std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap();
        assert!(
            !line.contains("\"id\""),
            "notification must not have id: {line}"
        );
    }

    #[test]
    fn envelope_validation_rejects_id_mismatch() {
        let resp = serde_json::json!({"jsonrpc":"2.0","id":"other","result":{}});
        let err = validate_jsonrpc_envelope(&resp, "ours").expect_err("id mismatch");
        assert!(err.to_string().contains("response id does not match"));
    }

    #[test]
    fn envelope_validation_rejects_wrong_jsonrpc_version() {
        let resp = serde_json::json!({"jsonrpc":"1.0","id":"r","result":{}});
        let err = validate_jsonrpc_envelope(&resp, "r").expect_err("wrong version");
        assert!(err.to_string().contains("jsonrpc"));
    }

    #[test]
    fn envelope_validation_surfaces_jsonrpc_error_payload() {
        let resp = serde_json::json!({"jsonrpc":"2.0","id":"r","error":{"code":-32601,"message":"Method not found"}});
        let err = validate_jsonrpc_envelope(&resp, "r").expect_err("rpc error must surface");
        assert!(err.to_string().contains("Method not found"));
    }

    #[test]
    fn session_register_params_carry_the_full_intl_003_payload() {
        let params = session_register_params(
            "sess_abc",
            Path::new("/tmp/wt"),
            Path::new("/tmp/wt/sub"),
            "anvil-run",
            "claude-1",
            1_700_000_000,
            Some("%5"),
            42_424, // launcher_pid (new)
        );
        // INTL-003 / pre-MLP2-025c flat metadata.
        assert_eq!(params["session_id"], "sess_abc");
        assert_eq!(params["worktree"], "/tmp/wt");
        assert_eq!(params["cwd"], "/tmp/wt/sub");
        assert_eq!(params["driver_id"], "anvil-run");
        assert_eq!(params["claimed_agent_id"], "claude-1");
        assert_eq!(params["pid_starttime"], 1_700_000_000);
        assert_eq!(params["tmux_pane"], "%5");

        // MLP2-025c: nested `agent_tag` so the daemon's MLP2-023+
        // parser actually honours the composite identity (previously
        // the flat fields were silently dropped by the daemon).
        assert_eq!(params["agent_tag"]["driver_id"], "anvil-run");
        assert_eq!(params["agent_tag"]["claimed_agent_id"], "claude-1");
        assert_eq!(params["agent_tag"]["pid_starttime"], 1_700_000_000);

        // MLP2-025c: nested `lineage` anchor so the daemon's
        // (pid, pid_starttime) lineage index gets seeded for the
        // spoof cross-check.
        assert_eq!(params["lineage"]["pid"], 42_424);
        assert_eq!(params["lineage"]["pid_starttime"], 1_700_000_000);
    }

    #[test]
    fn session_register_params_omits_tmux_pane_when_absent() {
        let params = session_register_params(
            "sess_abc",
            Path::new("/tmp/wt"),
            Path::new("/tmp/wt"),
            "anvil-run",
            "claude-1",
            42,
            None,
            7,
        );
        assert!(params.get("tmux_pane").is_none(), "no tmux_pane key");
        assert_eq!(params["cwd"], "/tmp/wt");
        // MLP2-025c: agent_tag + lineage still present.
        assert_eq!(params["agent_tag"]["claimed_agent_id"], "claude-1");
        assert_eq!(params["lineage"]["pid"], 7);
        assert_eq!(params["lineage"]["pid_starttime"], 42);
    }

    /// MLP2-025c: the lineage object's `pid` is the **launcher's**
    /// own PID, and the `pid_starttime` is shared with the
    /// `agent_tag` (both derive from the same launcher process).
    /// Pins the spec §7 trust model: the launcher's register-time
    /// claim about itself is trusted, and the cross-check matches
    /// the launcher's lineage when child processes write under
    /// `ANVIL_AGENT_TAG`.
    #[test]
    fn session_register_params_lineage_pid_starttime_matches_agent_tag() {
        let params = session_register_params(
            "sess_lineage",
            Path::new("/tmp/wt"),
            Path::new("/tmp/wt"),
            "anvil-run",
            "claude-2",
            1_700_000_500,
            None,
            9999,
        );
        assert_eq!(
            params["agent_tag"]["pid_starttime"], params["lineage"]["pid_starttime"],
            "agent_tag.pid_starttime and lineage.pid_starttime are the same value",
        );
        assert_eq!(params["lineage"]["pid"], 9999);
    }

    #[test]
    fn trim_newline_eats_both_lf_and_crlf() {
        assert_eq!(trim_trailing_newline(b"hello\n"), b"hello");
        assert_eq!(trim_trailing_newline(b"hello\r\n"), b"hello");
        assert_eq!(trim_trailing_newline(b"hello"), b"hello");
    }
}
