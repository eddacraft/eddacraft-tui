use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anvil_intercept::enforcement::{EnforcementPipeline, ProposedChange};
use anvil_intercept::ipc;
use anvil_intercept_rules::ChangeKind;
use anvil_kernel_types::{Diagnostic, Mode};
use serde::Deserialize;
use serde_json::{Value, json};

pub(crate) const INPUT_RULE_ID: &str = "mcp-validate-write-input";
pub(crate) const PRE_WRITE_MODE: &str = "pre-write";
const DAEMON_REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const DAEMON_RESPONSE_LINE_BYTES: u64 = 1 << 20;
const DAEMON_REQUEST_ID: &str = "mcp-prewrite-validation";
const SCAN_BUFFER_REQUEST_VERSION: u64 = 1;
const SCAN_BUFFER_RESULT_VERSION: u64 = 1;
const DAEMON_FAILURE: ValidationBackendFailure = ValidationBackendFailure {
    code: "validation-backend-unavailable",
    message: "Anvil could not validate the proposed write.",
    retriable: true,
};
const DAEMON_TRUNCATED_FAILURE: ValidationBackendFailure = ValidationBackendFailure {
    code: "validation-backend-truncated",
    message: "Anvil daemon returned a truncated validation response.",
    retriable: true,
};

pub struct PreWriteValidationRequest<'a> {
    pub relative_path: &'a str,
    pub content: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationBackend {
    Daemon,
    Embedded,
}

impl ValidationBackend {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Daemon => "daemon",
            Self::Embedded => "embedded",
        }
    }
}

/// Observable state of the daemon backend at the moment a request was
/// served. Distinguishes the three demotion paths so the MCP response
/// can carry an explicit signal rather than implying state by absence:
///
/// - `Available`: the daemon answered with structured diagnostics.
/// - `NotWired`: the daemon client reported `Unavailable` (the current
///   stub state — no daemon is wired up yet); the embedded validator
///   served the response.
/// - `Unavailable`: the daemon was expected but failed operationally
///   (e.g. socket timeout, IPC parse error). No diagnostics were
///   produced; the response carries an `error` payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonStatus {
    Available,
    NotWired,
    Unavailable,
}

impl DaemonStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::NotWired => "not-wired",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub backend: ValidationBackend,
    pub daemon_status: DaemonStatus,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationBackendFailure {
    pub code: &'static str,
    pub message: &'static str,
    pub retriable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DaemonValidationOutcome {
    Diagnostics(Vec<Diagnostic>),
    Unavailable,
    OperationalFailure(ValidationBackendFailure),
}

pub trait DaemonValidationClient {
    fn validate_pre_write(
        &self,
        request: &PreWriteValidationRequest<'_>,
    ) -> DaemonValidationOutcome;
}

pub struct LocalDaemonValidationClient;

pub struct SocketDaemonValidationClient {
    socket_path: PathBuf,
}

impl LocalDaemonValidationClient {
    #[cfg(unix)]
    #[cfg_attr(not(test), allow(dead_code))]
    #[must_use]
    pub fn with_socket_path(socket_path: impl Into<PathBuf>) -> SocketDaemonValidationClient {
        SocketDaemonValidationClient {
            socket_path: socket_path.into(),
        }
    }
}

impl DaemonValidationClient for LocalDaemonValidationClient {
    fn validate_pre_write(
        &self,
        request: &PreWriteValidationRequest<'_>,
    ) -> DaemonValidationOutcome {
        #[cfg(unix)]
        {
            let socket_path = match ipc::resolve_socket_path() {
                Ok(path) => path,
                Err(err) => {
                    eprintln!("anvil-mcp: daemon socket path unavailable: {err}");
                    return DaemonValidationOutcome::Unavailable;
                }
            };
            SocketDaemonValidationClient { socket_path }.validate_pre_write(request)
        }
        #[cfg(not(unix))]
        {
            let _ = request;
            eprintln!("anvil-mcp: daemon validation requires a Unix domain socket");
            DaemonValidationOutcome::Unavailable
        }
    }
}

impl DaemonValidationClient for SocketDaemonValidationClient {
    fn validate_pre_write(
        &self,
        request: &PreWriteValidationRequest<'_>,
    ) -> DaemonValidationOutcome {
        match request_daemon_diagnostics(&self.socket_path, request) {
            Ok(diagnostics) => DaemonValidationOutcome::Diagnostics(diagnostics),
            Err(DaemonRequestError::Unavailable) => DaemonValidationOutcome::Unavailable,
            Err(DaemonRequestError::Failure(failure)) => {
                DaemonValidationOutcome::OperationalFailure(failure)
            }
        }
    }
}

#[derive(Debug)]
enum DaemonRequestError {
    Unavailable,
    Failure(ValidationBackendFailure),
}

impl From<ValidationBackendFailure> for DaemonRequestError {
    fn from(failure: ValidationBackendFailure) -> Self {
        Self::Failure(failure)
    }
}

#[cfg(unix)]
fn request_daemon_diagnostics(
    socket_path: &Path,
    request: &PreWriteValidationRequest<'_>,
) -> Result<Vec<Diagnostic>, DaemonRequestError> {
    eprintln!(
        "anvil-mcp: connecting to daemon validation socket {}",
        socket_path.display()
    );
    if let Err(err) = ipc::validate_socket_path_for_client(socket_path) {
        eprintln!("anvil-mcp: daemon validation socket is unavailable: {err}");
        return match err {
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(DaemonRequestError::Unavailable)
            }
            _ => Err(DAEMON_FAILURE.into()),
        };
    }
    let mut stream = std::os::unix::net::UnixStream::connect(socket_path).map_err(|err| {
        eprintln!("anvil-mcp: daemon validation connection failed: {err}");
        DaemonRequestError::Failure(DAEMON_FAILURE)
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        eprintln!("anvil-mcp: daemon validation peer rejected: {err}");
        DaemonRequestError::Failure(DAEMON_FAILURE)
    })?;
    stream
        .set_read_timeout(Some(DAEMON_REQUEST_TIMEOUT))
        .map_err(|err| {
            eprintln!("anvil-mcp: daemon validation read-timeout setup failed: {err}");
            DaemonRequestError::Failure(DAEMON_FAILURE)
        })?;
    stream
        .set_write_timeout(Some(DAEMON_REQUEST_TIMEOUT))
        .map_err(|err| {
            eprintln!("anvil-mcp: daemon validation write-timeout setup failed: {err}");
            DaemonRequestError::Failure(DAEMON_FAILURE)
        })?;

    let frame = json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": request.relative_path,
            "text": request.content,
            "version": SCAN_BUFFER_REQUEST_VERSION,
            "mode": "preWrite"
        },
        "id": DAEMON_REQUEST_ID
    });
    eprintln!("anvil-mcp: sending daemon validation request");
    writeln!(stream, "{frame}").map_err(|err| {
        eprintln!("anvil-mcp: daemon validation request failed: {err}");
        DaemonRequestError::Failure(DAEMON_FAILURE)
    })?;
    stream.flush().map_err(|err| {
        eprintln!("anvil-mcp: daemon validation flush failed: {err}");
        DaemonRequestError::Failure(DAEMON_FAILURE)
    })?;

    let mut reader = BufReader::new(stream);
    let response = read_capped_response_line(&mut reader)?;
    eprintln!("anvil-mcp: received daemon validation response");

    let response: JsonRpcScanBufferResponse = serde_json::from_str(&response).map_err(|err| {
        eprintln!("anvil-mcp: daemon validation response parse failed: {err}");
        DAEMON_FAILURE
    })?;
    validate_jsonrpc_response_shape(&response)?;
    if let Some(error) = response.error {
        eprintln!(
            "anvil-mcp: daemon validation returned JSON-RPC error {}: {}",
            error.code, error.message
        );
        return Err(DAEMON_FAILURE.into());
    }
    let result = response.result.expect("shape validation requires result");
    if result.version != SCAN_BUFFER_RESULT_VERSION {
        eprintln!(
            "anvil-mcp: daemon validation response version mismatch: {}",
            result.version
        );
        return Err(DAEMON_FAILURE.into());
    }
    if result.truncated {
        eprintln!("anvil-mcp: daemon validation response was truncated");
        return Err(DAEMON_TRUNCATED_FAILURE.into());
    }

    Ok(result.diagnostics)
}

fn read_capped_response_line(
    reader: &mut impl BufRead,
) -> Result<String, ValidationBackendFailure> {
    let mut response = Vec::new();
    let read = reader
        .by_ref()
        .take(DAEMON_RESPONSE_LINE_BYTES + 1)
        .read_until(b'\n', &mut response)
        .map_err(|err| {
            eprintln!("anvil-mcp: daemon validation response read failed: {err}");
            DAEMON_FAILURE
        })?;
    if read == 0 {
        eprintln!("anvil-mcp: daemon validation response was empty");
        return Err(DAEMON_FAILURE);
    }
    if response.len() as u64 > DAEMON_RESPONSE_LINE_BYTES {
        eprintln!("anvil-mcp: daemon validation response exceeded line cap");
        return Err(DAEMON_FAILURE);
    }
    if !response.ends_with(b"\n") {
        eprintln!("anvil-mcp: daemon validation response omitted newline frame terminator");
        return Err(DAEMON_FAILURE);
    }
    String::from_utf8(response).map_err(|err| {
        eprintln!("anvil-mcp: daemon validation response was not UTF-8: {err}");
        DAEMON_FAILURE
    })
}

fn validate_jsonrpc_response_shape(
    response: &JsonRpcScanBufferResponse,
) -> Result<(), ValidationBackendFailure> {
    if response.jsonrpc != "2.0" {
        eprintln!(
            "anvil-mcp: daemon validation response used unsupported JSON-RPC version: {}",
            response.jsonrpc
        );
        return Err(DAEMON_FAILURE);
    }
    if response.id.as_ref() != Some(&Value::String(DAEMON_REQUEST_ID.to_string())) {
        eprintln!("anvil-mcp: daemon validation response id did not match request id");
        return Err(DAEMON_FAILURE);
    }
    match (response.result.is_some(), response.error.is_some()) {
        (true, false) | (false, true) => Ok(()),
        (true, true) => {
            eprintln!("anvil-mcp: daemon validation response included both result and error");
            Err(DAEMON_FAILURE)
        }
        (false, false) => {
            eprintln!("anvil-mcp: daemon validation response omitted result and error");
            Err(DAEMON_FAILURE)
        }
    }
}

#[cfg(not(unix))]
fn request_daemon_diagnostics(
    _socket_path: &Path,
    _request: &PreWriteValidationRequest<'_>,
) -> Result<Vec<Diagnostic>, DaemonRequestError> {
    Err(DaemonRequestError::Unavailable)
}

#[derive(Debug, Deserialize)]
struct JsonRpcScanBufferResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<ScanBufferResult>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ScanBufferResult {
    #[allow(dead_code)]
    version: u64,
    diagnostics: Vec<Diagnostic>,
    truncated: bool,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[allow(dead_code)]
    data: Option<Value>,
}

pub fn validate_pre_write(
    request: &PreWriteValidationRequest<'_>,
    daemon: &impl DaemonValidationClient,
) -> Result<ValidationResult, ValidationBackendFailure> {
    match daemon.validate_pre_write(request) {
        DaemonValidationOutcome::Diagnostics(diagnostics) => Ok(ValidationResult {
            backend: ValidationBackend::Daemon,
            daemon_status: DaemonStatus::Available,
            diagnostics,
        }),
        DaemonValidationOutcome::Unavailable => {
            // `Unavailable` is the stub-default path: no daemon is
            // wired in yet, so we silently demote to embedded. The
            // response surfaces this via `daemon_status: NotWired`
            // so the agent can observe the demotion without parsing
            // backend strings.
            let mut result = embedded_validate_pre_write(request);
            result.daemon_status = DaemonStatus::NotWired;
            Ok(result)
        }
        DaemonValidationOutcome::OperationalFailure(failure) => Err(failure),
    }
}

fn embedded_validate_pre_write(request: &PreWriteValidationRequest<'_>) -> ValidationResult {
    let pipeline = EnforcementPipeline::default();
    let change = ProposedChange {
        path: Path::new(request.relative_path),
        change_kind: ChangeKind::Modified,
        content: Some(request.content.as_bytes()),
    };
    let diagnostics = pipeline
        .diagnostics_for_proposed_changes(&[change], &Mode::Unknown(PRE_WRITE_MODE.to_string()));

    ValidationResult {
        backend: ValidationBackend::Embedded,
        // The default for an embedded result is `NotWired` — the
        // function is only reached when the daemon path could not
        // serve the request. Callers that want to express "embedded
        // by design" (rather than "demoted from Unavailable") can
        // override `daemon_status` after construction.
        daemon_status: DaemonStatus::NotWired,
        diagnostics,
    }
}

pub(crate) fn sanitise_id_part(value: &str) -> String {
    let sanitised = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if sanitised.is_empty() {
        "unknown".to_string()
    } else {
        sanitised
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DaemonStatus, DaemonValidationClient, DaemonValidationOutcome, PreWriteValidationRequest,
    };
    use super::{ValidationBackend, ValidationBackendFailure};
    use super::{embedded_validate_pre_write, validate_pre_write};
    #[cfg(target_os = "linux")]
    use anvil_intercept::Shutdown;
    #[cfg(target_os = "linux")]
    use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
    #[cfg(target_os = "linux")]
    use std::io::{BufRead as _, BufReader as StdBufReader, Write as _};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::os::unix::net::UnixListener;
    #[cfg(target_os = "linux")]
    use std::thread;
    #[cfg(unix)]
    use tempfile::tempdir;
    #[cfg(target_os = "linux")]
    use tokio::runtime::Runtime;

    struct FixtureDaemon {
        outcome: DaemonValidationOutcome,
    }

    impl DaemonValidationClient for FixtureDaemon {
        fn validate_pre_write(
            &self,
            _request: &PreWriteValidationRequest<'_>,
        ) -> DaemonValidationOutcome {
            self.outcome.clone()
        }
    }

    #[test]
    fn daemon_result_wins_when_available() {
        let request = secret_request();
        let embedded = embedded_validate_pre_write(&request);
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Diagnostics(embedded.diagnostics.clone()),
        };

        let result = validate_pre_write(&request, &daemon).expect("daemon result is valid");

        assert_eq!(result.backend, ValidationBackend::Daemon);
        assert_eq!(result.daemon_status, DaemonStatus::Available);
        assert_eq!(result.diagnostics, embedded.diagnostics);
    }

    #[test]
    fn unavailable_daemon_falls_back_to_embedded_validation() {
        let request = secret_request();
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Unavailable,
        };

        let result = validate_pre_write(&request, &daemon).expect("embedded fallback succeeds");

        assert_eq!(result.backend, ValidationBackend::Embedded);
        // The demotion is observable via `daemon_status`: callers
        // can distinguish "embedded by design" (would set this to
        // `Available` if they ever wired such a path) from
        // "stub-default not-wired" (the current state).
        assert_eq!(result.daemon_status, DaemonStatus::NotWired);
        assert_eq!(result.diagnostics[0].source.rule_id, "secret-detection");
    }

    #[test]
    fn operational_daemon_failure_does_not_fall_back() {
        let request = secret_request();
        let failure = ValidationBackendFailure {
            code: "validation-backend-unavailable",
            message: "Anvil could not validate the proposed write.",
            retriable: true,
        };
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::OperationalFailure(failure),
        };

        let error = validate_pre_write(&request, &daemon).expect_err("daemon failure blocks");

        assert_eq!(error, failure);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn local_daemon_client_returns_scan_buffer_diagnostics_with_embedded_parity() {
        let runtime = Runtime::new().expect("tokio runtime starts");
        let workspace = tempdir().expect("runtime dir exists");
        std::fs::set_permissions(workspace.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir permissions tightened");
        let socket = workspace.path().join("intercept.sock");
        let _runtime_guard = runtime.enter();
        let listener = IpcListener::bind(&socket, NoopDispatcher).expect("daemon socket binds");
        let (shutdown, token) = Shutdown::new();
        let server = runtime.spawn(listener.serve(token));

        let request = secret_request();
        let embedded = embedded_validate_pre_write(&request);
        let client = super::LocalDaemonValidationClient::with_socket_path(socket);

        let outcome = client.validate_pre_write(&request);

        shutdown.trigger();
        runtime.block_on(async {
            server
                .await
                .expect("daemon task joins")
                .expect("daemon exits cleanly");
        });

        let DaemonValidationOutcome::Diagnostics(diagnostics) = outcome else {
            panic!("daemon should return diagnostics, got {outcome:?}");
        };
        assert_eq!(diagnostics, embedded.diagnostics);
    }

    #[cfg(unix)]
    #[test]
    fn local_daemon_client_demotes_to_embedded_when_socket_is_unavailable() {
        let workspace = tempdir().expect("runtime dir exists");
        std::fs::set_permissions(workspace.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir permissions tightened");
        let client = super::LocalDaemonValidationClient::with_socket_path(
            workspace.path().join("missing-intercept.sock"),
        );

        let outcome = client.validate_pre_write(&secret_request());

        assert_eq!(outcome, DaemonValidationOutcome::Unavailable);
    }

    #[cfg(unix)]
    #[test]
    fn local_daemon_client_fails_closed_when_validated_socket_refuses_connection() {
        let workspace = tempdir().expect("runtime dir exists");
        std::fs::set_permissions(workspace.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir permissions tightened");
        let socket = workspace.path().join("intercept.sock");
        let listener = UnixListener::bind(&socket).expect("stale socket binds");
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .expect("socket permissions tightened");
        drop(listener);
        let client = super::LocalDaemonValidationClient::with_socket_path(socket);

        let outcome = client.validate_pre_write(&secret_request());

        assert!(matches!(
            outcome,
            DaemonValidationOutcome::OperationalFailure(_)
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn local_daemon_client_rejects_mismatched_jsonrpc_response_id() {
        let (socket, daemon) = fake_daemon_response(
            r#"{"jsonrpc":"2.0","id":"stale-response","result":{"version":1,"diagnostics":[],"truncated":false}}
"#,
        );
        let client = super::LocalDaemonValidationClient::with_socket_path(socket);

        let outcome = client.validate_pre_write(&secret_request());

        daemon.join().expect("fake daemon exits");
        assert_eq!(
            outcome,
            DaemonValidationOutcome::OperationalFailure(ValidationBackendFailure {
                code: "validation-backend-unavailable",
                message: "Anvil could not validate the proposed write.",
                retriable: true,
            })
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn local_daemon_client_rejects_response_with_result_and_error() {
        let (socket, daemon) = fake_daemon_response(
            r#"{"jsonrpc":"2.0","id":"mcp-prewrite-validation","result":{"version":1,"diagnostics":[],"truncated":false},"error":{"code":-32603,"message":"boom"}}
"#,
        );
        let client = super::LocalDaemonValidationClient::with_socket_path(socket);

        let outcome = client.validate_pre_write(&secret_request());

        daemon.join().expect("fake daemon exits");
        assert!(matches!(
            outcome,
            DaemonValidationOutcome::OperationalFailure(_)
        ));
    }

    #[test]
    fn capped_daemon_response_reader_rejects_unframed_response() {
        let mut reader = std::io::Cursor::new(b"{}".as_slice());

        let error = super::read_capped_response_line(&mut reader).expect_err("newline is required");

        assert_eq!(error.code, "validation-backend-unavailable");
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    #[test]
    fn local_daemon_client_fails_closed_when_peer_validation_is_unimplemented() {
        let workspace = tempdir().expect("runtime dir exists");
        std::fs::set_permissions(workspace.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir permissions tightened");
        let socket = workspace.path().join("intercept.sock");
        let _listener = UnixListener::bind(&socket).expect("socket binds");
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .expect("socket permissions tightened");
        let client = super::LocalDaemonValidationClient::with_socket_path(socket);

        let outcome = client.validate_pre_write(&secret_request());

        assert!(matches!(
            outcome,
            DaemonValidationOutcome::OperationalFailure(_)
        ));
    }

    #[cfg(target_os = "linux")]
    fn fake_daemon_response(
        response: &'static str,
    ) -> (std::path::PathBuf, thread::JoinHandle<()>) {
        let workspace = tempdir().expect("runtime dir exists").keep();
        std::fs::set_permissions(&workspace, std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir permissions tightened");
        let socket = workspace.join("intercept.sock");
        let listener = UnixListener::bind(&socket).expect("fake daemon binds");
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .expect("socket permissions tightened");
        let daemon = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("client connects");
            let mut request = String::new();
            StdBufReader::new(stream.try_clone().expect("clone stream"))
                .read_line(&mut request)
                .expect("fake daemon reads request");
            assert!(
                request.contains("\"method\":\"scan_buffer\""),
                "unexpected request: {request}"
            );
            stream
                .write_all(response.as_bytes())
                .expect("fake daemon writes response");
        });
        (socket, daemon)
    }

    fn secret_request() -> PreWriteValidationRequest<'static> {
        PreWriteValidationRequest {
            relative_path: "src/secret.ts",
            content: "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n",
        }
    }
}
