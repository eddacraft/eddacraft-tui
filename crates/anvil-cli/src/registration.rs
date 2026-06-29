//! ACTMO-014/015: the shared worktree-registration client.
//!
//! Relocated out of `activation/` (ADR-094 decision 2) so `anvil start`
//! (activation), `anvil workspace register/unregister/list` (ACTMO-015), and
//! `anvil workspace register --all` (ACTMO-018) all drive the daemon over one
//! primitive. The daemon classifies registration failures and the client maps
//! them to honest outcomes: a re-register of an already-owned worktree
//! heartbeats the existing owner rather than erroring (ADR-094 decision 3), a
//! fenced/cascaded worktree points at `anvil intercept unblock`, and a cap
//! breach gives a clear message. Paths are canonicalised with `dunce` so
//! identity is stable and display paths are free of the Windows `\\?\` prefix.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anvil_intercept_proto::SessionId;
use anvil_intercept_proto::session::{ACTIVATION_SPINE_CLAIMED_AGENT_ID, AgentTag};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::activation::daemon_evidence::ACTIVATION_DAEMON_QUERY_TIMEOUT;

const REGISTER_METHOD: &str = "session.register";
const UNREGISTER_METHOD: &str = "session.unregister";
const HEARTBEAT_METHOD: &str = "heartbeat";
const RESPONSE_LINE_BYTES: u64 = 1 << 20;
const DRIVER_ID: &str = "anvil-start";
const CLAIMED_AGENT_ID: &str = ACTIVATION_SPINE_CLAIMED_AGENT_ID;

/// Outcome of a worktree registration attempt against the intercept daemon.
/// ACTMO-014 enriches the original four-variant enum with the daemon's
/// refusal classifications so callers can give honest, actionable guidance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorktreeRegistration {
    /// A fresh durable registration was recorded.
    Registered,
    /// The worktree was already registered (deterministic session id, or the
    /// daemon reported `WorktreeAlreadyOwned` for an equivalent path); the
    /// existing owner was heartbeated. Idempotent success.
    Refreshed,
    /// The daemon is not running / not reachable. Non-fatal for activation.
    DaemonUnavailable,
    /// The worktree is fenced or in fence-cascade mode and refuses
    /// registration until cleared. The string is user-facing guidance that
    /// points at `anvil intercept unblock`.
    Fenced(String),
    /// A registration cap was exceeded (per-worktree session cap, or the
    /// distinct registered-worktree membership cap). The string is the
    /// user-facing cap message.
    CapExceeded(String),
    /// Any other rejection. The string is the daemon's message.
    Rejected(String),
}

/// ACTMO-014/015: register a worktree as durable membership with the daemon.
/// `worktree` may be any spelling of the path; it is canonicalised with
/// `dunce` before the deterministic activation session id is derived.
pub(crate) fn register_worktree_with_daemon(worktree: &Path) -> WorktreeRegistration {
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
        // ADR-094 decision 3: a re-register of the same canonical worktree
        // (our deterministic id, or a `WorktreeAlreadyOwned` for an equivalent
        // spelling) heartbeats the existing owner rather than erroring.
        Err(err) if err.is_session_already_registered() || err.is_worktree_already_owned() => {
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
        Err(err) if err.is_fenced() => {
            let message = format!(
                "worktree {} is fenced; clear it with `anvil intercept unblock {}`",
                canonical.display(),
                canonical.display(),
            );
            tracing::warn!(worktree = %canonical.display(), "activation: worktree registration refused — fenced");
            WorktreeRegistration::Fenced(message)
        }
        Err(err) if err.is_cap_exceeded() => {
            let message = err.to_string();
            tracing::warn!(worktree = %canonical.display(), error = %message, "activation: worktree registration refused — cap exceeded");
            WorktreeRegistration::CapExceeded(message)
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

/// Canonicalise with `dunce` so identity is stable and the path keeps a plain
/// form on Windows (no `\\?\` verbatim prefix) for both the wire key and any
/// display. Falls back to the raw path when canonicalisation fails; the daemon
/// is server-authoritative for identity, so a client fallback that diverges is
/// reconciled by the `WorktreeAlreadyOwned`→heartbeat path.
fn canonicalise_for_registration(worktree: &Path) -> PathBuf {
    dunce::canonicalize(worktree).unwrap_or_else(|err| {
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

/// ACTMO-014 markers pinning the daemon's `RegistryError` Display wording the
/// client classifies on. Each is covered by a cross-crate pin test below, so a
/// rephrase in the registry fails CI rather than silently degrading the
/// client's outcome mapping.
///
/// Each marker includes its adjacent **static** prefix word (`worktree …`,
/// `degraded …`) so it cannot be matched by a worktree *path* embedded in an
/// unrelated error's `{worktree:?}` field — e.g. a `SessionCapExceeded` message
/// for a path literally containing "is fenced" must not classify as fenced
/// (covered by `path_substring_does_not_misclassify`).
const WORKTREE_ALREADY_OWNED_MARKER: &str = "worktree already owned";
const WORKTREE_FENCED_MARKER: &str = "worktree is fenced";
const WORKTREE_CASCADED_MARKER: &str = "degraded fence-cascade mode";
const SESSION_CAP_MARKER: &str = "worktree session cap exceeded";
const REGISTERED_CAP_MARKER: &str = "registered worktree cap exceeded";

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
        self.message_contains(SESSION_ALREADY_REGISTERED_MARKER)
    }

    /// ADR-094 decision 3: the same canonical worktree reached via a different
    /// spelling — the client heartbeats the existing owner.
    fn is_worktree_already_owned(&self) -> bool {
        self.message_contains(WORKTREE_ALREADY_OWNED_MARKER)
    }

    /// The worktree is fenced or in fence-cascade mode — the one genuine
    /// registration refusal, pointing the operator at `anvil intercept unblock`.
    fn is_fenced(&self) -> bool {
        self.message_contains(WORKTREE_FENCED_MARKER)
            || self.message_contains(WORKTREE_CASCADED_MARKER)
    }

    /// A per-worktree session cap or the distinct registered-worktree
    /// membership cap was exceeded.
    fn is_cap_exceeded(&self) -> bool {
        self.message_contains(SESSION_CAP_MARKER) || self.message_contains(REGISTERED_CAP_MARKER)
    }

    /// Case-insensitive substring match against the JSON-RPC error message.
    /// Transport / daemon-unavailable / protocol errors never carry a registry
    /// classification, so they always return `false`.
    fn message_contains(&self, marker: &str) -> bool {
        matches!(
            self,
            Self::JsonRpc { message, .. } if message.to_ascii_lowercase().contains(marker)
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

/// ACTMO-015: outcome of an `anvil workspace unregister`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorktreeUnregistration {
    /// The durable registration was removed.
    Unregistered,
    /// The worktree was not registered — `unregister` is idempotent, so this
    /// is a benign success, not an error.
    NotRegistered,
    /// The daemon is not reachable.
    DaemonUnavailable,
    /// Any other rejection.
    Rejected(String),
}

/// ACTMO-015: unregister a worktree's durable membership. Derives the same
/// deterministic activation session id as `register`, so unregistering by path
/// hits the right entry. Idempotent: an unregistered worktree returns
/// [`WorktreeUnregistration::NotRegistered`].
pub(crate) fn unregister_worktree_with_daemon(worktree: &Path) -> WorktreeUnregistration {
    let canonical = canonicalise_for_registration(worktree);
    let session_id = activation_session_id(&canonical);
    let request_id = format!("anvil-workspace-unregister-{}", session_id.as_str());
    match request_jsonrpc(
        UNREGISTER_METHOD,
        &serde_json::json!({ "session_id": session_id.as_str() }),
        &request_id,
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    ) {
        Ok(result) => {
            // The daemon returns `{ "removed": <bool> }`; `false` means the id
            // was not present, which is the idempotent no-op case.
            let removed = result
                .get("removed")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if removed {
                WorktreeUnregistration::Unregistered
            } else {
                WorktreeUnregistration::NotRegistered
            }
        }
        Err(DaemonRegistrationError::DaemonUnavailable(_)) => {
            WorktreeUnregistration::DaemonUnavailable
        }
        Err(err) => WorktreeUnregistration::Rejected(err.to_string()),
    }
}

/// ACTMO-014/016: why a path is not a registerable Git worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NotRegisterable {
    /// A bare repository — no working tree to register.
    BareRepository,
    /// `cwd` is inside the `.git` directory, not the working tree.
    InsideGitDir,
    /// Not a Git worktree at all (or `git` is unavailable).
    NotAWorktree(String),
}

impl std::fmt::Display for NotRegisterable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BareRepository => {
                f.write_str("bare repositories have no working tree to register")
            }
            Self::InsideGitDir => {
                f.write_str("path is inside a .git directory, not a working tree")
            }
            Self::NotAWorktree(detail) => write!(f, "not a Git worktree: {detail}"),
        }
    }
}

/// ACTMO-014/016 (ADR-094 decision 4): resolve the registerable Git worktree
/// containing `start`, or explain why it is not registerable. Rejects bare
/// repositories and the `.git` internal directory; accepts ordinary, linked,
/// and submodule worktrees (where `.git` is a file pointer). Returns the
/// canonical top-level path that should be registered.
pub(crate) fn registerable_worktree(start: &Path) -> Result<PathBuf, NotRegisterable> {
    // The boolean probes do not require a working tree, so they succeed even
    // inside `.git/` — unlike `--show-toplevel`, which *fails* there and would
    // abort a combined `rev-parse` before the booleans could classify it.
    let probe = git_rev_parse(start, &["--is-bare-repository", "--is-inside-git-dir"])?;
    let mut lines = probe.lines();
    let is_bare = lines.next().map(str::trim) == Some("true");
    let is_inside_git_dir = lines.next().map(str::trim) == Some("true");
    if is_bare {
        return Err(NotRegisterable::BareRepository);
    }
    if is_inside_git_dir {
        return Err(NotRegisterable::InsideGitDir);
    }

    // A real (ordinary, linked, or submodule) worktree: resolve its top level.
    let toplevel = git_rev_parse(start, &["--show-toplevel"])?;
    let toplevel = toplevel.trim();
    if toplevel.is_empty() {
        return Err(NotRegisterable::NotAWorktree(
            "git did not report a working-tree top level".to_owned(),
        ));
    }
    Ok(canonicalise_for_registration(Path::new(toplevel)))
}

/// Run `git -C <start> rev-parse <args>` and return stdout, mapping any
/// failure (git missing, not a repo) to [`NotRegisterable::NotAWorktree`].
fn git_rev_parse(start: &Path, args: &[&str]) -> Result<String, NotRegisterable> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(start)
        .arg("rev-parse")
        .args(args)
        .output()
        .map_err(|err| NotRegisterable::NotAWorktree(format!("failed to run git: {err}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(NotRegisterable::NotAWorktree(if stderr.is_empty() {
            format!("git rev-parse failed with status {}", output.status)
        } else {
            stderr
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
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

    /// ACTMO-014 cross-crate guard: every classification marker must remain a
    /// substring of the `RegistryError` Display wording the client classifies
    /// on. A rephrase in the registry fails here rather than silently breaking
    /// the CLI's fenced / cap / already-owned outcome mapping.
    #[test]
    fn markers_pin_registry_error_wording() {
        use anvil_intercept::registry::RegistryError;
        use anvil_intercept_proto::SessionId;

        let cases: Vec<(String, &str)> = vec![
            (
                RegistryError::WorktreeAlreadyOwned {
                    existing: SessionId::new("sess_x"),
                }
                .to_string(),
                WORKTREE_ALREADY_OWNED_MARKER,
            ),
            (
                RegistryError::WorktreeFenced {
                    worktree: "/tmp/wt".into(),
                }
                .to_string(),
                WORKTREE_FENCED_MARKER,
            ),
            (
                RegistryError::WorktreeCascaded {
                    worktree: "/tmp/wt".into(),
                }
                .to_string(),
                WORKTREE_CASCADED_MARKER,
            ),
            (
                RegistryError::SessionCapExceeded {
                    worktree: "/tmp/wt".into(),
                    cap: 16,
                    live: 16,
                }
                .to_string(),
                SESSION_CAP_MARKER,
            ),
            (
                RegistryError::RegisteredWorktreeCapExceeded { cap: 64, live: 64 }.to_string(),
                REGISTERED_CAP_MARKER,
            ),
        ];
        for (display, marker) in cases {
            assert!(
                display.to_ascii_lowercase().contains(marker),
                "registry wording {display:?} drifted from marker {marker:?}",
            );
        }
    }

    /// ACTMO-014: the client maps each pinned marker to the right outcome
    /// classification.
    #[test]
    fn daemon_error_classification_routes_each_refusal() {
        let owned = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: "worktree already owned by session SessionId(\"sess_x\")".to_owned(),
        };
        assert!(owned.is_worktree_already_owned());
        assert!(!owned.is_fenced());

        let fenced = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: "worktree is fenced until explicit unblock: \"/tmp/wt\"".to_owned(),
        };
        assert!(fenced.is_fenced());
        assert!(!fenced.is_cap_exceeded());

        let cascaded = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message:
                "worktree is in degraded fence-cascade mode and refuses new sessions: \"/tmp/wt\""
                    .to_owned(),
        };
        assert!(cascaded.is_fenced());

        let registered_cap = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: "registered worktree cap exceeded: 64 registered at cap=64".to_owned(),
        };
        assert!(registered_cap.is_cap_exceeded());
        assert!(!registered_cap.is_fenced());

        // Transport-class errors never carry a registry classification.
        let unavailable = DaemonRegistrationError::DaemonUnavailable("no socket".to_owned());
        assert!(!unavailable.is_fenced());
        assert!(!unavailable.is_cap_exceeded());
        assert!(!unavailable.is_worktree_already_owned());
    }

    /// ACTMO-014 (adversarial review F3): a worktree PATH containing a marker
    /// substring must not misclassify an unrelated error. A cap-exceeded
    /// message for a path containing "is fenced" / "fence-cascade mode" stays
    /// classified as a cap breach, because the markers are anchored to the
    /// error's static prefix words, not a bare substring.
    fn cap_error_for(path: &str) -> DaemonRegistrationError {
        DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: format!(
                "worktree session cap exceeded for {path:?}: 16 live sessions at cap=16"
            ),
        }
    }

    #[test]
    fn path_substring_does_not_misclassify() {
        let pathological = cap_error_for("/home/alice/is fenced/fence-cascade mode/project");
        assert!(
            pathological.is_cap_exceeded(),
            "the cap error must classify as cap-exceeded",
        );
        assert!(
            !pathological.is_fenced(),
            "a path containing 'is fenced'/'fence-cascade mode' must not classify as fenced",
        );
    }

    /// ACTMO-014/016: a real Git worktree resolves to its canonical top level;
    /// a path inside `.git` is rejected; a non-repo is rejected.
    #[test]
    fn registerable_worktree_resolves_real_worktree_and_rejects_git_internals() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git")
                .status
                .success();
            assert!(ok, "git {args:?} failed");
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "t@t"]);
        run(&["config", "user.name", "t"]);

        let toplevel = registerable_worktree(dir.path()).expect("worktree is registerable");
        assert_eq!(
            toplevel,
            dunce::canonicalize(dir.path()).expect("canonical")
        );

        // Inside the .git directory is rejected.
        let err =
            registerable_worktree(&dir.path().join(".git")).expect_err("inside .git rejected");
        assert_eq!(err, NotRegisterable::InsideGitDir);

        // A non-repo directory is rejected.
        let outside = tempfile::tempdir().expect("non-repo");
        assert!(matches!(
            registerable_worktree(outside.path()),
            Err(NotRegisterable::NotAWorktree(_))
        ));
    }
}
