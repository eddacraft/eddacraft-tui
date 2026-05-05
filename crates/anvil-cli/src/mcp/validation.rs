use std::io::{BufRead, BufReader, Write};
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
                    return DaemonValidationOutcome::OperationalFailure(DAEMON_FAILURE);
                }
            };
            SocketDaemonValidationClient { socket_path }.validate_pre_write(request)
        }
        #[cfg(not(unix))]
        {
            let _ = request;
            eprintln!("anvil-mcp: daemon validation requires a Unix domain socket");
            DaemonValidationOutcome::OperationalFailure(DAEMON_FAILURE)
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
            Err(failure) => DaemonValidationOutcome::OperationalFailure(failure),
        }
    }
}

#[cfg(unix)]
fn request_daemon_diagnostics(
    socket_path: &Path,
    request: &PreWriteValidationRequest<'_>,
) -> Result<Vec<Diagnostic>, ValidationBackendFailure> {
    eprintln!(
        "anvil-mcp: connecting to daemon validation socket {}",
        socket_path.display()
    );
    let mut stream = std::os::unix::net::UnixStream::connect(socket_path).map_err(|err| {
        eprintln!("anvil-mcp: daemon validation connection failed: {err}");
        DAEMON_FAILURE
    })?;
    stream
        .set_read_timeout(Some(DAEMON_REQUEST_TIMEOUT))
        .map_err(|err| {
            eprintln!("anvil-mcp: daemon validation read-timeout setup failed: {err}");
            DAEMON_FAILURE
        })?;
    stream
        .set_write_timeout(Some(DAEMON_REQUEST_TIMEOUT))
        .map_err(|err| {
            eprintln!("anvil-mcp: daemon validation write-timeout setup failed: {err}");
            DAEMON_FAILURE
        })?;

    let frame = json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": request.relative_path,
            "text": request.content,
            "version": 1,
            "mode": "preWrite"
        },
        "id": "mcp-prewrite-validation"
    });
    eprintln!("anvil-mcp: sending daemon validation request");
    writeln!(stream, "{frame}").map_err(|err| {
        eprintln!("anvil-mcp: daemon validation request failed: {err}");
        DAEMON_FAILURE
    })?;
    stream.flush().map_err(|err| {
        eprintln!("anvil-mcp: daemon validation flush failed: {err}");
        DAEMON_FAILURE
    })?;

    let mut response = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut response).map_err(|err| {
        eprintln!("anvil-mcp: daemon validation response read failed: {err}");
        DAEMON_FAILURE
    })?;
    eprintln!("anvil-mcp: received daemon validation response");

    let response: JsonRpcScanBufferResponse = serde_json::from_str(&response).map_err(|err| {
        eprintln!("anvil-mcp: daemon validation response parse failed: {err}");
        DAEMON_FAILURE
    })?;
    if let Some(error) = response.error {
        eprintln!(
            "anvil-mcp: daemon validation returned JSON-RPC error {}: {}",
            error.code, error.message
        );
        return Err(DAEMON_FAILURE);
    }
    let Some(result) = response.result else {
        eprintln!("anvil-mcp: daemon validation response omitted result");
        return Err(DAEMON_FAILURE);
    };
    if result.truncated {
        eprintln!("anvil-mcp: daemon validation response was truncated");
        return Err(DAEMON_TRUNCATED_FAILURE);
    }

    Ok(result.diagnostics)
}

#[cfg(not(unix))]
fn request_daemon_diagnostics(
    _socket_path: &Path,
    _request: &PreWriteValidationRequest<'_>,
) -> Result<Vec<Diagnostic>, ValidationBackendFailure> {
    Err(DAEMON_FAILURE)
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
    #[cfg(unix)]
    use anvil_intercept::Shutdown;
    #[cfg(unix)]
    use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use tempfile::tempdir;
    #[cfg(unix)]
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

    #[cfg(unix)]
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
    fn local_daemon_client_reports_explicit_failure_when_socket_is_unavailable() {
        let workspace = tempdir().expect("runtime dir exists");
        let client = super::LocalDaemonValidationClient::with_socket_path(
            workspace.path().join("missing-intercept.sock"),
        );

        let outcome = client.validate_pre_write(&secret_request());

        assert_eq!(
            outcome,
            DaemonValidationOutcome::OperationalFailure(ValidationBackendFailure {
                code: "validation-backend-unavailable",
                message: "Anvil could not validate the proposed write.",
                retriable: true,
            })
        );
    }

    fn secret_request() -> PreWriteValidationRequest<'static> {
        PreWriteValidationRequest {
            relative_path: "src/secret.ts",
            content: "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n",
        }
    }
}
