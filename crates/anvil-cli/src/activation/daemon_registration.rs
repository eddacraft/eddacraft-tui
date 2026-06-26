use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anvil_intercept_proto::SessionId;
use anvil_intercept_proto::session::AgentTag;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::daemon_evidence::ACTIVATION_DAEMON_QUERY_TIMEOUT;

const REGISTER_METHOD: &str = "session.register";
const HEARTBEAT_METHOD: &str = "heartbeat";
const RESPONSE_LINE_BYTES: u64 = 1 << 20;
const DRIVER_ID: &str = "anvil-start";
const CLAIMED_AGENT_ID: &str = "activation-spine";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WorktreeRegistration {
    Registered,
    Refreshed,
    DaemonUnavailable,
    Rejected(String),
}

pub(super) fn register_worktree_with_daemon(worktree: &Path) -> WorktreeRegistration {
    let canonical = canonicalise_for_registration(worktree);
    let session_id = activation_session_id(&canonical);
    let request_id = format!("anvil-start-register-{}", session_id.as_str());

    match request_jsonrpc(
        REGISTER_METHOD,
        &session_register_params(&session_id, &canonical),
        &request_id,
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    ) {
        Ok(_) => {
            tracing::info!(
                worktree = %canonical.display(),
                session_id = session_id.as_str(),
                "activation: registered worktree with intercept daemon",
            );
            WorktreeRegistration::Registered
        }
        Err(err) if err.is_session_already_registered() => {
            refresh_existing_activation_session(&session_id, &canonical)
        }
        Err(DaemonRegistrationError::DaemonUnavailable(message)) => {
            tracing::debug!(
                worktree = %canonical.display(),
                error = %message,
                "activation: intercept daemon unavailable for worktree registration",
            );
            WorktreeRegistration::DaemonUnavailable
        }
        Err(err) => {
            let message = err.to_string();
            tracing::warn!(
                worktree = %canonical.display(),
                error = %message,
                "activation: worktree registration with intercept daemon failed",
            );
            WorktreeRegistration::Rejected(message)
        }
    }
}

fn refresh_existing_activation_session(
    session_id: &SessionId,
    canonical: &Path,
) -> WorktreeRegistration {
    let request_id = format!("anvil-start-heartbeat-{}", session_id.as_str());
    match request_jsonrpc(
        HEARTBEAT_METHOD,
        &serde_json::json!({ "session_id": session_id.as_str() }),
        &request_id,
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    ) {
        Ok(_) => {
            tracing::info!(
                worktree = %canonical.display(),
                session_id = session_id.as_str(),
                "activation: refreshed existing daemon worktree registration",
            );
            WorktreeRegistration::Refreshed
        }
        Err(err) => {
            let message = err.to_string();
            tracing::warn!(
                worktree = %canonical.display(),
                session_id = session_id.as_str(),
                error = %message,
                "activation: existing worktree registration could not be refreshed",
            );
            WorktreeRegistration::Rejected(message)
        }
    }
}

fn canonicalise_for_registration(worktree: &Path) -> PathBuf {
    std::fs::canonicalize(worktree).unwrap_or_else(|err| {
        tracing::warn!(
            error = %err,
            worktree = %worktree.display(),
            "activation: worktree canonicalisation failed before daemon registration",
        );
        worktree.to_path_buf()
    })
}

fn activation_session_id(worktree: &Path) -> SessionId {
    let mut hasher = Sha256::new();
    hasher.update(worktree.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    SessionId::new(format!("sess_activation_{}", hex::encode(&digest[..8])))
}

fn activation_agent_tag() -> AgentTag {
    AgentTag::new(DRIVER_ID, CLAIMED_AGENT_ID, 0)
}

fn session_register_params(session_id: &SessionId, worktree: &Path) -> Value {
    serde_json::json!({
        "session_id": session_id.as_str(),
        "worktree": worktree.to_string_lossy(),
        "agent_tag": activation_agent_tag(),
    })
}

/// Stable marker the daemon's registry uses when a `session.register`
/// reuses a live session id (`RegistryError::SessionAlreadyExists`,
/// `#[error("session already registered: …")]`). Activation derives a
/// deterministic session id from the worktree path, so a re-run of
/// `anvil start` against an already-registered worktree is the expected
/// way to hit this — we detect it and downgrade to a heartbeat instead
/// of treating it as a rejection.
///
/// Matched against the structured `error.data.error` field (not a blob of
/// the whole envelope) and case-insensitively, so reordering or casing
/// changes do not silently break the heartbeat fall-through. The
/// `marker_pins_registry_session_already_exists_display` test fails CI if
/// the daemon ever rephrases the wording out from under this constant.
const SESSION_ALREADY_REGISTERED_MARKER: &str = "session already registered";

#[derive(Debug)]
enum DaemonRegistrationError {
    DaemonUnavailable(String),
    /// A JSON-RPC `error` response. `code` is the numeric error code when
    /// present; `message` is the most specific human string available
    /// (the daemon nests the registry detail under `error.data.error`).
    JsonRpc {
        code: Option<i64>,
        message: String,
    },
    Transport(String),
    Protocol(String),
}

impl DaemonRegistrationError {
    /// True when this error signals the worktree's activation session is
    /// already registered, i.e. a re-run that should heartbeat rather than
    /// re-register.
    fn is_session_already_registered(&self) -> bool {
        matches!(
            self,
            Self::JsonRpc { message, .. }
                if message
                    .to_ascii_lowercase()
                    .contains(SESSION_ALREADY_REGISTERED_MARKER)
        )
    }
}

impl std::fmt::Display for DaemonRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DaemonUnavailable(message)
            | Self::Transport(message)
            | Self::Protocol(message) => f.write_str(message),
            Self::JsonRpc { code, message } => match code {
                Some(code) => write!(f, "daemon error {code}: {message}"),
                None => f.write_str(message),
            },
        }
    }
}

impl std::error::Error for DaemonRegistrationError {}

fn request_jsonrpc(
    method: &str,
    params: &Value,
    id: &str,
    timeout: Duration,
) -> Result<Value, DaemonRegistrationError> {
    let body = jsonrpc_request_line(method, params, id);
    let response = round_trip(&body, timeout)?;
    parse_jsonrpc_response(&response, id)
}

fn jsonrpc_request_line(method: &str, params: &Value, id: &str) -> Vec<u8> {
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

#[cfg(unix)]
fn round_trip(body: &[u8], timeout: Duration) -> Result<String, DaemonRegistrationError> {
    // `Write` is only needed for the Unix-socket transport; the Windows
    // named-pipe client (below) writes through an inherent method, so importing
    // it at module scope is an unused import on Windows (`-D warnings`).
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    use anvil_intercept::ipc;

    let socket_path = ipc::resolve_socket_path()
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
        return match err {
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(DaemonRegistrationError::DaemonUnavailable(format!(
                    "anvil intercept daemon is not running (no socket at {})",
                    socket_path.display()
                )))
            }
            other => Err(DaemonRegistrationError::Transport(format!(
                "anvil intercept daemon socket is unavailable: {other}"
            ))),
        };
    }

    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        DaemonRegistrationError::Transport(format!(
            "failed to connect to intercept daemon socket {}: {err}",
            socket_path.display()
        ))
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        DaemonRegistrationError::Transport(format!("daemon peer credentials rejected: {err}"))
    })?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    stream
        .write_all(body)
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    stream
        .flush()
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    read_one_line(stream)
}

#[cfg(windows)]
fn round_trip(body: &[u8], timeout: Duration) -> Result<String, DaemonRegistrationError> {
    use std::sync::mpsc;
    use std::thread;

    let pipe_name = anvil_intercept_win32::pipe_name_for_current_user()
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    let pipe_name_clone = pipe_name.clone();
    let body = body.to_vec();
    let (tx, rx) = mpsc::sync_channel::<Result<String, DaemonRegistrationError>>(1);
    thread::spawn(move || {
        let result = (|| {
            let mut client = anvil_intercept_win32::connect_owner_only_pipe_client(
                &pipe_name_clone,
            )
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    DaemonRegistrationError::DaemonUnavailable(format!(
                        "anvil intercept daemon is not running (no pipe at {pipe_name_clone})"
                    ))
                } else {
                    DaemonRegistrationError::Transport(format!(
                        "failed to connect to intercept daemon pipe {pipe_name_clone}: {err}"
                    ))
                }
            })?;
            client
                .write_all(&body)
                .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
            read_one_line(client)
        })();
        let _ = tx.send(result);
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => Err(DaemonRegistrationError::Transport(format!(
            "timed out talking to the daemon on pipe {pipe_name}"
        ))),
    }
}

#[cfg(all(not(unix), not(windows)))]
fn round_trip(_body: &[u8], _timeout: Duration) -> Result<String, DaemonRegistrationError> {
    Err(DaemonRegistrationError::DaemonUnavailable(
        "intercept daemon IPC is not supported on this platform".to_owned(),
    ))
}

fn read_one_line<R: Read>(stream: R) -> Result<String, DaemonRegistrationError> {
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_BYTES + 1)
        .read_until(b'\n', &mut buf)
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    if read == 0 {
        return Err(DaemonRegistrationError::Transport(
            "daemon closed the connection before responding".to_owned(),
        ));
    }
    if (buf.len() as u64) > RESPONSE_LINE_BYTES {
        return Err(DaemonRegistrationError::Protocol(format!(
            "daemon response exceeded {RESPONSE_LINE_BYTES} byte cap"
        )));
    }
    let line = std::str::from_utf8(buf.trim_ascii_end())
        .map_err(|err| DaemonRegistrationError::Protocol(err.to_string()))?;
    Ok(line.to_owned())
}

fn parse_jsonrpc_response(
    response: &str,
    request_id: &str,
) -> Result<Value, DaemonRegistrationError> {
    let value: Value = serde_json::from_str(response)
        .map_err(|err| DaemonRegistrationError::Protocol(err.to_string()))?;
    if value.get("jsonrpc") != Some(&Value::String("2.0".to_owned())) {
        return Err(DaemonRegistrationError::Protocol(format!(
            "daemon response missing or wrong jsonrpc version: {value}"
        )));
    }
    if value.get("id") != Some(&Value::String(request_id.to_owned())) {
        return Err(DaemonRegistrationError::Protocol(format!(
            "daemon response id does not match request {request_id:?}: {value}"
        )));
    }
    if let Some(error) = value.get("error") {
        return Err(jsonrpc_error_from_value(error));
    }
    value.get("result").cloned().ok_or_else(|| {
        DaemonRegistrationError::Protocol(format!("daemon response missing result: {value}"))
    })
}

/// Map a JSON-RPC `error` object to a structured [`DaemonRegistrationError`].
///
/// The daemon flattens internal failures to a generic `-32603` envelope and
/// nests the specific registry detail under `error.data.error`, so we read
/// that field first (most specific), then `error.message`, and only fall back
/// to the whole object when neither is a string. Pulling out the focused field
/// is what lets [`DaemonRegistrationError::is_session_already_registered`]
/// match a stable marker rather than grepping the entire serialised envelope.
fn jsonrpc_error_from_value(error: &Value) -> DaemonRegistrationError {
    let code = error.get("code").and_then(Value::as_i64);
    let message = error
        .get("data")
        .and_then(|data| data.get("error"))
        .and_then(Value::as_str)
        .or_else(|| error.get("message").and_then(Value::as_str))
        .map_or_else(|| error.to_string(), str::to_owned);
    DaemonRegistrationError::JsonRpc { code, message }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn activation_session_id_is_stable_and_path_derived() {
        let first = activation_session_id(Path::new("/tmp/repo"));
        let second = activation_session_id(Path::new("/tmp/repo"));
        let different = activation_session_id(Path::new("/tmp/other"));

        assert_eq!(first, second);
        assert_ne!(first, different);
        assert!(first.as_str().starts_with("sess_activation_"));
    }

    #[test]
    fn register_params_use_activation_identity_without_lineage() {
        let session_id = activation_session_id(Path::new("/tmp/repo"));
        let params = session_register_params(&session_id, Path::new("/tmp/repo"));

        assert_eq!(params["worktree"], "/tmp/repo");
        assert_eq!(params["agent_tag"]["driver_id"], "anvil-start");
        assert_eq!(params["agent_tag"]["claimed_agent_id"], "activation-spine");
        assert!(
            params.get("lineage").is_none(),
            "activation registration must not require peer lineage support"
        );
    }

    #[test]
    fn parse_detects_session_already_registered_from_nested_daemon_envelope() {
        // The shape the daemon actually emits: a generic -32603 with the
        // registry detail nested under error.data.error (ipc.rs).
        let response = r#"{"jsonrpc":"2.0","id":"req-1","error":{"code":-32603,"message":"Internal error","data":{"error":"session already registered: SessionId(\"sess_activation_abcd\")"}}}"#;
        let err = parse_jsonrpc_response(response, "req-1").unwrap_err();
        assert!(
            err.is_session_already_registered(),
            "nested registry detail must be detected: {err}"
        );
    }

    #[test]
    fn session_already_registered_detection_is_case_insensitive_and_field_scoped() {
        // Casing/whitespace drift in the message must still match.
        let upper = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: "  SESSION Already Registered: sess_x".to_owned(),
        };
        assert!(upper.is_session_already_registered());

        // An unrelated error must not be mistaken for it.
        let other = DaemonRegistrationError::JsonRpc {
            code: Some(-32602),
            message: "invalid params: worktree".to_owned(),
        };
        assert!(!other.is_session_already_registered());

        // Transport/daemon-unavailable errors are never this signal.
        assert!(
            !DaemonRegistrationError::DaemonUnavailable("no socket".to_owned())
                .is_session_already_registered()
        );
    }

    #[test]
    fn marker_pins_registry_session_already_exists_display() {
        // Cross-crate guard (Council S1): the heartbeat fall-through depends on
        // the daemon's registry wording. If RegistryError::SessionAlreadyExists
        // is ever rephrased, this fails CI instead of silently breaking the
        // re-run heartbeat (which would let the session lapse on its TTL).
        let display = anvil_intercept::registry::RegistryError::SessionAlreadyExists(
            anvil_intercept_proto::SessionId::new("sess_activation_abcd"),
        )
        .to_string();
        assert!(
            display
                .to_ascii_lowercase()
                .contains(SESSION_ALREADY_REGISTERED_MARKER),
            "registry wording drifted from the activation marker: {display:?}"
        );
    }
}
