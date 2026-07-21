use std::path::Path;

#[cfg(any(unix, test))]
use std::io::{BufRead, Read};
#[cfg(unix)]
use std::io::{BufReader, Write};
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::time::Duration;

use anvil_intercept::enforcement::{EnforcementPipeline, ProposedChange};
// MLP2-075: the Windows branch of `query_protection_claim` resolves the
// named pipe via `ipc::resolve_pipe_name`, so this import must cover
// windows too — a unix-only gate fails E0433 on the msvc cross legs
// (which Linux CI cannot see).
#[cfg(any(unix, windows))]
use anvil_intercept::ipc;
// MLP2-075: also consumed by the Windows pipe client below.
#[cfg(any(unix, windows))]
use anvil_intercept::status::build_protection_claim_from_wire;
use anvil_intercept_rules::ChangeKind;
use anvil_kernel_types::protection_claim::ProtectionClaim;
use anvil_kernel_types::{Diagnostic, Mode};
#[cfg(unix)]
use serde::Deserialize;
#[cfg(unix)]
use serde_json::{Value, json};

#[cfg(any(unix, windows))]
use crate::daemon_validation::{ScanBufferError, ScanMode, scan_buffer};

pub(crate) const INPUT_RULE_ID: &str = "mcp-validate-write-input";
pub(crate) const PRE_WRITE_MODE: &str = "pre-write";
#[cfg(unix)]
const DAEMON_REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
/// MLP2-051i: wall-clock budget for the MCP shim's protection-claim
/// snapshot fetch. Pinned to the activation diagnostic's 500 ms cap
/// (`crate::activation::daemon_evidence::ACTIVATION_DAEMON_QUERY_TIMEOUT`)
/// by the runtime test `mcp_protection_claim_timeout_matches_activation_budget`
/// so a wedged daemon cannot stall `validate_write` for the 2 s default
/// carried by `query_daemon_status_at`.
///
/// Scope is intentionally narrow: the pre-write `scan_buffer` path
/// (`request_daemon_diagnostics`) keeps the longer `DAEMON_REQUEST_TIMEOUT`
/// as its own budget. That path's drip-attack resistance is tracked
/// separately — closing this MCP claim fetch is not equivalent to
/// closing the `scan_buffer` read loop.
///
/// `#[cfg(any(unix, windows))]` matches the union of the two consumer
/// `query_protection_claim` impls below; if a non-unix non-windows
/// target is ever added, prune the gate (or the constant moves with
/// the impls).
#[cfg(any(unix, windows))]
pub(crate) const MCP_PROTECTION_CLAIM_QUERY_TIMEOUT: std::time::Duration =
    std::time::Duration::from_millis(500);
#[cfg(any(unix, test))]
const DAEMON_RESPONSE_LINE_BYTES: u64 = 1 << 20;
#[cfg(unix)]
const DAEMON_REQUEST_ID: &str = "mcp-prewrite-validation";
#[cfg(unix)]
const SCAN_BUFFER_REQUEST_VERSION: u64 = 1;
#[cfg(unix)]
const SCAN_BUFFER_RESULT_VERSION: u64 = 1;
#[cfg(any(unix, windows, test))]
const DAEMON_FAILURE: ValidationBackendFailure = ValidationBackendFailure {
    code: "validation-backend-unavailable",
    message: "anvil could not validate the proposed write.",
    retriable: true,
};
#[cfg(any(unix, windows))]
const DAEMON_TRUNCATED_FAILURE: ValidationBackendFailure = ValidationBackendFailure {
    code: "validation-backend-truncated",
    message: "anvil daemon returned a truncated validation response.",
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
/// - `NotWired`: the daemon client reported `Unavailable` — no reachable
///   daemon answered (absent socket / dead daemon); the embedded validator
///   served the response. (DSV-007: the daemon `scan_buffer` path IS wired,
///   so this is "no daemon running", not "feature unimplemented". `validate_write`
///   stays on `scan_buffer`, not `validate_paths`, because it is a pre-write
///   gate over proposed content the daemon has not read — see the tool's
///   reconciliation note.)
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

    /// MLP2-051b: best-effort fetch of the per-workspace
    /// [`ProtectionClaim`] for inclusion on the `validate_write`
    /// response. The default returns `None`, which causes the MCP
    /// shim to omit the `protection_claim` field entirely (the
    /// spec's no-over-claim posture when no daemon snapshot is
    /// available). Live daemon clients override to query the
    /// daemon and return `Some(...)` on success, `None` on any
    /// failure (timeout, parse, daemon-down) — the claim is
    /// advisory, never blocking.
    fn query_protection_claim(&self, _workspace_root: &Path) -> Option<ProtectionClaim> {
        None
    }
}

pub struct LocalDaemonValidationClient;

#[cfg(unix)]
pub struct SocketDaemonValidationClient {
    socket_path: PathBuf,
}

/// MLP2-075 Windows analogue of [`SocketDaemonValidationClient`].
///
/// Carries a per-instance named-pipe name so tests can bind a unique
/// pipe (per-PID) and avoid colliding with the canonical per-user
/// pipe a real daemon would own on the same Windows runner —
/// mirroring the rationale documented on
/// [`crate::commands::intercept::query_daemon_status_windows_at`].
#[cfg(windows)]
pub struct WindowsPipeDaemonValidationClient {
    pipe_name: String,
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

    /// MLP2-075: Windows equivalent of [`Self::with_socket_path`].
    /// Lets tests construct a validation client bound to a custom
    /// pipe name so the fixture daemon and the production daemon
    /// never collide on the same Windows runner.
    #[cfg(windows)]
    #[cfg_attr(not(test), allow(dead_code))]
    #[must_use]
    pub fn with_pipe_name(pipe_name: impl Into<String>) -> WindowsPipeDaemonValidationClient {
        WindowsPipeDaemonValidationClient {
            pipe_name: pipe_name.into(),
        }
    }
}

impl DaemonValidationClient for LocalDaemonValidationClient {
    fn validate_pre_write(
        &self,
        request: &PreWriteValidationRequest<'_>,
    ) -> DaemonValidationOutcome {
        #[cfg(any(unix, windows))]
        {
            let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            match scan_buffer(
                ScanMode::PreWrite,
                request.relative_path,
                request.content,
                &cancellation,
            ) {
                Ok(diagnostics) => DaemonValidationOutcome::Diagnostics(diagnostics),
                Err(ScanBufferError::Unavailable) => DaemonValidationOutcome::Unavailable,
                Err(ScanBufferError::Truncated) => {
                    DaemonValidationOutcome::OperationalFailure(DAEMON_TRUNCATED_FAILURE)
                }
                Err(
                    ScanBufferError::Failed
                    | ScanBufferError::VersionMismatch
                    | ScanBufferError::Cancelled,
                ) => DaemonValidationOutcome::OperationalFailure(DAEMON_FAILURE),
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = request;
            eprintln!("anvil-mcp: daemon validation requires a Unix domain socket");
            DaemonValidationOutcome::Unavailable
        }
    }

    fn query_protection_claim(&self, workspace_root: &Path) -> Option<ProtectionClaim> {
        #[cfg(unix)]
        {
            let socket_path = ipc::resolve_socket_path().ok()?;
            SocketDaemonValidationClient { socket_path }.query_protection_claim(workspace_root)
        }
        // MLP2-075: resolve the canonical per-user pipe (install-root
        // aware since CIB-106) and delegate to the Windows pipe client.
        // Mirrors the Unix branch's resolve-then-delegate shape.
        // Pipe-resolution failure emits a warn line and returns `None` —
        // preserves the no-over-claim posture (omit the field rather
        // than synthesise a misleading "unprotected" state). Slightly
        // more verbose than the Unix branch's silent `.ok()?` because
        // pipe-resolution failures are rarer and warrant
        // operator-visible context.
        #[cfg(windows)]
        {
            let pipe_name = match ipc::resolve_pipe_name() {
                Ok(name) => name,
                Err(err) => {
                    eprintln!(
                        "anvil-mcp: protection_claim pipe resolution failed (omitting field): {err}",
                    );
                    return None;
                }
            };
            // Route through the canonical constructor so production and
            // test paths share the same shape — if `with_pipe_name`
            // ever gains validation logic, production picks it up
            // automatically.
            Self::with_pipe_name(pipe_name).query_protection_claim(workspace_root)
        }
        // Non-Unix, non-Windows targets (e.g. wasm): no daemon
        // attestation surface available. Match the existing
        // no-over-claim posture by omitting the field.
        #[cfg(all(not(unix), not(windows)))]
        {
            let _ = workspace_root;
            None
        }
    }
}

#[cfg(windows)]
impl DaemonValidationClient for WindowsPipeDaemonValidationClient {
    fn validate_pre_write(
        &self,
        request: &PreWriteValidationRequest<'_>,
    ) -> DaemonValidationOutcome {
        // MLP2-075 only lifts `query_protection_claim` parity. Pre-write
        // IPC over Windows named pipes is a separate scope (the MCP
        // shim's Windows pre-write story is tracked elsewhere). Match
        // the no-non-Unix-validation posture of
        // `LocalDaemonValidationClient::validate_pre_write` and return
        // `Unavailable` so the MCP shim falls back to local-only
        // validation.
        let _ = request;
        DaemonValidationOutcome::Unavailable
    }

    fn query_protection_claim(&self, workspace_root: &Path) -> Option<ProtectionClaim> {
        // Parity with the Unix `SocketDaemonValidationClient` body —
        // the claim is advisory metadata; never block on a fetch
        // failure. A stale / missing snapshot maps to `None`, which
        // causes the MCP shim to omit the field instead of
        // synthesising a misleading "unprotected" claim.
        //
        // MLP2-051i: the explicit `_with_timeout` form pins the
        // 500 ms `MCP_PROTECTION_CLAIM_QUERY_TIMEOUT` budget so a
        // wedged daemon cannot stretch `validate_write` to the 2 s
        // default carried by the parameterless `query_daemon_status_windows_at`.
        let snapshot = crate::commands::intercept::query_daemon_status_windows_at_with_timeout(
            &self.pipe_name,
            MCP_PROTECTION_CLAIM_QUERY_TIMEOUT,
        )
        .map_err(|err| {
            eprintln!("anvil-mcp: protection_claim query_status failed (omitting field): {err}");
            err
        })
        .ok()?;
        Some(build_protection_claim_from_wire(&snapshot, workspace_root))
    }
}

#[cfg(unix)]
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

    fn query_protection_claim(&self, workspace_root: &Path) -> Option<ProtectionClaim> {
        // The claim is advisory metadata — never block on a fetch
        // failure. A stale / missing snapshot maps to `None`, which
        // causes the MCP shim to omit the field instead of synthesising
        // a misleading "unprotected" claim.
        //
        // MLP2-051i: the explicit `_with_timeout` form pins the
        // 500 ms `MCP_PROTECTION_CLAIM_QUERY_TIMEOUT` budget so a
        // wedged daemon cannot stretch `validate_write` to the 2 s
        // default carried by the parameterless `query_daemon_status_at`.
        let snapshot = crate::commands::intercept::query_daemon_status_at_with_timeout(
            &self.socket_path,
            MCP_PROTECTION_CLAIM_QUERY_TIMEOUT,
        )
        .map_err(|err| {
            eprintln!("anvil-mcp: protection_claim query_status failed (omitting field): {err}");
            err
        })
        .ok()?;
        Some(build_protection_claim_from_wire(&snapshot, workspace_root))
    }
}

#[cfg(unix)]
#[derive(Debug)]
enum DaemonRequestError {
    Unavailable,
    Failure(ValidationBackendFailure),
}

#[cfg(unix)]
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

    parse_scan_buffer_response(&response)
}

/// Parse and translate a daemon `scan_buffer` reply line into diagnostics, or a
/// **fail-closed** failure.
///
/// This is the MCP client's own translation layer, deliberately decoupled from
/// the daemon's response struct (B3). Unlike the daemon's fine-grained mid-edit
/// contract (`crates/anvil-intercept/tests/midedit_contract.rs`: distinct
/// `-32602` / `-32001` / `-32000` / `-32002` variants), it collapses **every**
/// JSON-RPC error — and every shape violation — into a single failure: the MCP
/// pre-write surface never needs to distinguish daemon error variants, only to
/// fail **closed** (upstream → `block`) rather than silent-pass. A success
/// reply is returned verbatim; a truncated one is a distinct (also-blocking)
/// failure. Pinned by tests so this daemon-error → block translation can never
/// silently regress to a pass (#1737 / RTAI-006 fitness).
#[cfg(unix)]
fn parse_scan_buffer_response(line: &str) -> Result<Vec<Diagnostic>, DaemonRequestError> {
    let response: JsonRpcScanBufferResponse = serde_json::from_str(line).map_err(|err| {
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

#[cfg(any(unix, test))]
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

#[cfg(unix)]
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

#[cfg(unix)]
#[derive(Debug, Deserialize)]
struct JsonRpcScanBufferResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<ScanBufferResult>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: Option<Value>,
}

// B3: intentionally a local `Deserialize`-only mirror of the daemon's
// scan-buffer reply, decoupled from the proto `ScanBufferResponse` so the
// MCP client parses the raw socket JSON without importing the daemon's
// response struct. `diagnostics` is element-compatible with the proto
// `DiagnosticEnvelope` (both `Vec<anvil_kernel_types::Diagnostic>`); it is
// deliberately not re-typed against the alias here.
#[cfg(unix)]
#[derive(Debug, Deserialize)]
struct ScanBufferResult {
    #[allow(dead_code)]
    version: u64,
    diagnostics: Vec<Diagnostic>,
    truncated: bool,
}

#[cfg(unix)]
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
            // `Unavailable` = no reachable daemon answered (absent socket /
            // dead daemon). We silently demote to the byte-identical embedded
            // validator; the response surfaces this via `daemon_status: NotWired`
            // so the agent can observe the demotion without parsing backend
            // strings. (DSV-007: the daemon `scan_buffer` path is wired — this is
            // the daemon-absent fallback, not an unimplemented stub.)
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
            message: "anvil could not validate the proposed write.",
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
                message: "anvil could not validate the proposed write.",
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

    /// MLP2-051i: pin the MCP timeout to the activation budget so a
    /// future edit to one constant cannot silently diverge from the
    /// other. Compile-time pinning would require `Duration::as_millis`
    /// in `const` context (stabilised after this crate's MSRV), so the
    /// guard is a runtime equality test rather than `const _: () =
    /// assert!(...)`.
    #[cfg(any(unix, windows))]
    #[test]
    fn mcp_protection_claim_timeout_matches_activation_budget() {
        use super::MCP_PROTECTION_CLAIM_QUERY_TIMEOUT;
        use crate::activation::daemon_evidence::ACTIVATION_DAEMON_QUERY_TIMEOUT;
        assert_eq!(
            MCP_PROTECTION_CLAIM_QUERY_TIMEOUT, ACTIVATION_DAEMON_QUERY_TIMEOUT,
            "MLP2-051i: MCP protection-claim query budget must mirror activation budget; a divergence \
             permits the MCP shim to inherit a 2 s stall the activation lane no longer allows.",
        );
    }

    /// MLP2-051i: the MCP `query_protection_claim` Unix path must
    /// inherit the same 500 ms budget the activation diagnostic uses
    /// (MLP2-051f) so a wedged daemon cannot stall MCP `validate_write`
    /// for the full 2 s `query_daemon_status_at` default. Mirrors
    /// `activation_query_aborts_within_budget_against_hung_daemon` in
    /// `crate::activation::daemon_evidence`, but exercises the MCP
    /// client surface so a regression in the `_with_timeout` wire-up
    /// is caught here, not just on the activation lane.
    ///
    /// The accepted server stream is held alive (via the `stop_rx`
    /// channel) until after the client times out, so the client never
    /// observes an EOF — the read path must exit on its own budget.
    ///
    /// `#[cfg(target_os = "linux")]` mirrors the gate on the activation
    /// equivalent: macOS CI is not gated and `UnixStream` timeout
    /// reliability on Darwin under load is not part of the coverage
    /// contract here. The production code at `SocketDaemonValidationClient::query_protection_claim`
    /// still ships on all `cfg(unix)` targets; a macOS-specific
    /// regression would surface via the runtime parity test above
    /// (constant-equality, not timing) plus operator report rather
    /// than this timing assertion.
    #[cfg(target_os = "linux")]
    #[test]
    fn mcp_query_protection_claim_aborts_within_budget_against_hung_daemon() {
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        use super::{
            DaemonValidationClient as _, LocalDaemonValidationClient,
            MCP_PROTECTION_CLAIM_QUERY_TIMEOUT,
        };

        let runtime_dir = tempdir().expect("runtime tempdir");
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir perms");
        let socket = runtime_dir.path().join("intercept.sock");
        let listener = UnixListener::bind(&socket).expect("hung-daemon listener binds");
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .expect("socket perms");

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            // Accept exactly one connection and hold the stream in
            // scope until `stop_rx` recv returns. This keeps the
            // accepted side open so the client read blocks until its
            // own timeout fires (rather than seeing EOF from a closed
            // peer, which would return None for the wrong reason).
            let (_stream, _) = listener.accept().expect("hung-daemon accepts");
            let _ = stop_rx.recv();
        });

        let workspace_root = runtime_dir.path().join("workspace");
        let client = LocalDaemonValidationClient::with_socket_path(socket);

        let started = Instant::now();
        let result = client.query_protection_claim(&workspace_root);
        let elapsed = started.elapsed();

        // No-over-claim posture preserved: a timed-out fetch omits
        // the field rather than synthesising a misleading state.
        assert!(
            result.is_none(),
            "hung daemon must surface as None, got {result:?}",
        );
        // 200 ms slack tolerates loaded CI workers; the spec's strict
        // bound is `timeout + 100 ms`, but a CI runner under load can
        // burn 100 ms on scheduling alone. The contract under test is
        // "no 2 s blow-up", not "exactly 500 ms".
        let slack = Duration::from_millis(200);
        assert!(
            elapsed <= MCP_PROTECTION_CLAIM_QUERY_TIMEOUT + slack,
            "MCP query_protection_claim exceeded budget: elapsed = {elapsed:?}, budget = {MCP_PROTECTION_CLAIM_QUERY_TIMEOUT:?}",
        );

        // Release the held server stream so the listener thread can
        // exit cleanly.
        let _ = stop_tx.send(());
        handle
            .join()
            .expect("hung-daemon fixture thread should not panic");
    }

    /// MLP2-075: end-to-end proof that the Windows path returns
    /// `Some(claim)` when the daemon is reachable and attests the
    /// queried worktree. Mirrors `windows_query_daemon_status_round_trips_against_local_pipe`
    /// in `commands::intercept` but exercises the
    /// `query_protection_claim` surface so the wire-up at
    /// `LocalDaemonValidationClient::query_protection_claim` Windows
    /// branch cannot regress to the pre-MLP2-075 `None` short-circuit.
    ///
    /// The pipe name is per-PID (rather than the canonical
    /// `ipc::resolve_pipe_name()` value) so the test never
    /// collides with a real daemon that might be bound on the same
    /// Windows runner.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_query_protection_claim_returns_some_when_daemon_attests_worktree() {
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        use anvil_intercept::Shutdown;
        use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
        // Local import of `DaemonStatus` shadows the `tests` module's
        // `use super::DaemonStatus` (the local reporting enum at
        // validation.rs:83) so the `StatusProvider::query_status`
        // return type resolves correctly. Mirrors the pattern in
        // intercept.rs:1247.
        use anvil_intercept::status::{DaemonStatus, IpcState, StatusProvider, build_status};
        use anvil_kernel_types::protection_claim::WorktreeClaimState;

        use super::{DaemonValidationClient as _, LocalDaemonValidationClient};

        let worktree = std::path::PathBuf::from(r"C:\tmp\mlp2-075-test-wt");

        struct Fixture {
            worktree: std::path::PathBuf,
        }
        impl StatusProvider for Fixture {
            fn query_status(&self) -> DaemonStatus {
                let session = anvil_intercept_proto::SessionRecord {
                    id: anvil_intercept_proto::SessionId::new("sess-mlp2-075"),
                    worktree: self.worktree.clone(),
                    pid: Some(4242),
                    pgid: Some(4242),
                    started_at_unix: 1_700_000_000,
                    last_heartbeat_unix: 1_700_000_010,
                    status: anvil_intercept_proto::SessionStatus::Active,
                    agent_tag: None,
                    daemon_issued_tag: None,
                };
                let started = Instant::now();
                build_status(
                    vec![session],
                    &[],
                    &[],
                    None,
                    started,
                    started + Duration::from_secs(1),
                    "0.0.0-windows-test",
                    IpcState::Serving,
                    None,
                    None,
                    0,
                )
            }
        }

        let pipe_name = format!(
            r"\\.\pipe\anvil-validation-protection-claim-test-{}",
            std::process::id(),
        );

        // Multi-thread runtime: a single worker drives the server task
        // while the main thread runs the synchronous client. A
        // current_thread runtime would never poll the spawned server
        // because the only thread that could is blocked on the client
        // call below — same rationale as
        // `windows_query_daemon_status_round_trips_against_local_pipe`.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let _runtime_guard = runtime.enter();
        let listener = IpcListener::bind(&pipe_name, NoopDispatcher)
            .expect("daemon pipe binds")
            .with_status_provider(Arc::new(Fixture {
                worktree: worktree.clone(),
            }));
        let (shutdown, token) = Shutdown::new();
        let server = runtime.spawn(listener.serve(token));

        let client = LocalDaemonValidationClient::with_pipe_name(pipe_name);
        let claim = client
            .query_protection_claim(&worktree)
            .expect("MLP2-075: daemon-attested worktree must produce Some(claim)");
        assert_eq!(
            claim.worktree_state,
            WorktreeClaimState::PreWriteDaemon,
            "single active unfenced session on Serving daemon must map to PreWriteDaemon",
        );
        assert_eq!(
            claim.surfaces.len(),
            1,
            "exactly one surface (the registered session) must be reported",
        );

        shutdown.trigger();
        runtime.block_on(async {
            server
                .await
                .expect("daemon task joins")
                .expect("daemon exits cleanly");
        });
    }

    /// MLP2-075: with no daemon bound on the pipe name, the Windows
    /// path returns `None` — honest fallback matching the Unix
    /// `resolve_socket_path` failure / unreachable-daemon path. The
    /// MCP shim then omits the `protection_claim` field rather than
    /// synthesising a misleading "unprotected" claim.
    ///
    /// Coverage gap (intentional, MLP2-075 scope): the
    /// `anvil_intercept::ipc::resolve_pipe_name()` failure
    /// branch inside `LocalDaemonValidationClient::query_protection_claim`'s
    /// `cfg(not(unix))` arm is not exercised here (would require
    /// mocking the win32 SID helper). The branch follows the same
    /// no-over-claim posture: warn + return `None`.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_query_protection_claim_returns_none_when_pipe_absent() {
        use super::{DaemonValidationClient as _, LocalDaemonValidationClient};

        let pipe_name = format!(
            r"\\.\pipe\anvil-validation-protection-claim-missing-{}",
            std::process::id(),
        );
        let workspace_root = std::path::Path::new(r"C:\tmp\does-not-matter");
        let client = LocalDaemonValidationClient::with_pipe_name(pipe_name);
        let result = client.query_protection_claim(workspace_root);
        assert!(
            result.is_none(),
            "MLP2-075: no daemon bound must produce None (honest fallback), got {result:?}",
        );
    }

    // --- #1737 / RTAI-006: MCP pre-write fail-closed translation ------------
    //
    // `crates/anvil-intercept/tests/midedit_contract.rs` pins the daemon's
    // fine-grained mid-edit contract (distinct -32602/-32001/-32000/-32002
    // error variants). The MCP pre-write client has its own translation layer
    // that intentionally collapses every daemon error into a single
    // block-inducing failure — an agent needs "could not validate → block", not
    // the specific code. These tests pin that translation directly on the wire
    // reply so a daemon-error → block path can never silently regress to a pass
    // (the coverage the never-built `anvil-rmcp` consumer was meant to give).
    // `Err(DaemonRequestError::Failure(_))` is the fail-closed outcome the
    // upstream `validate_pre_write` maps to `decision: block`.

    #[cfg(unix)]
    fn success_reply(diagnostics_json: &str, truncated: bool) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","result":{{"version":{ver},"diagnostics":{diags},"truncated":{trunc}}}}}"#,
            id = super::DAEMON_REQUEST_ID,
            ver = super::SCAN_BUFFER_RESULT_VERSION,
            diags = diagnostics_json,
            trunc = truncated,
        )
    }

    #[cfg(unix)]
    fn error_reply(code: i64, message: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","error":{{"code":{code},"message":"{message}"}}}}"#,
            id = super::DAEMON_REQUEST_ID,
        )
    }

    #[cfg(unix)]
    fn is_fail_closed(result: &Result<Vec<super::Diagnostic>, super::DaemonRequestError>) -> bool {
        matches!(result, Err(super::DaemonRequestError::Failure(_)))
    }

    #[cfg(unix)]
    #[test]
    fn every_daemon_jsonrpc_error_variant_fails_closed_to_block() {
        // The exact error codes the daemon mid-edit contract pins. Each must
        // collapse to a blocking failure on the MCP path — never Ok (a pass).
        for (code, message) in [
            (-32602, "Invalid params"),
            (-32001, "Scan timed out"),
            (-32000, "Server busy"),
            (-32002, "Cross-session rejection"),
            (-32603, "Internal error"),
        ] {
            let reply = error_reply(code, message);
            let result = super::parse_scan_buffer_response(&reply);
            assert!(
                is_fail_closed(&result),
                "daemon JSON-RPC error {code} must fail closed (block), not pass",
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn clean_daemon_success_reply_parses_to_diagnostics() {
        let reply = success_reply("[]", false);
        let diagnostics =
            super::parse_scan_buffer_response(&reply).expect("a clean success reply must parse");
        assert!(
            diagnostics.is_empty(),
            "an empty-diagnostics success reply yields no findings"
        );
    }

    #[cfg(unix)]
    #[test]
    fn truncated_success_reply_fails_closed() {
        // A truncated scan is not a clean pass — it blocks (distinct failure).
        let reply = success_reply("[]", true);
        assert!(
            is_fail_closed(&super::parse_scan_buffer_response(&reply)),
            "a truncated daemon reply must fail closed, not pass as clean",
        );
    }

    #[cfg(unix)]
    #[test]
    fn malformed_and_shape_violating_replies_fail_closed() {
        // Non-JSON, both-result-and-error, neither, and a mismatched id all
        // fail closed rather than being interpreted as a pass.
        let both = format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","result":{{"version":{ver},"diagnostics":[],"truncated":false}},"error":{{"code":-32000,"message":"x"}}}}"#,
            id = super::DAEMON_REQUEST_ID,
            ver = super::SCAN_BUFFER_RESULT_VERSION,
        );
        let neither = format!(r#"{{"jsonrpc":"2.0","id":"{}"}}"#, super::DAEMON_REQUEST_ID);
        let wrong_id = r#"{"jsonrpc":"2.0","id":"someone-else","result":{"version":1,"diagnostics":[],"truncated":false}}"#;
        for reply in [
            "not json at all".to_string(),
            both,
            neither,
            wrong_id.to_string(),
        ] {
            assert!(
                is_fail_closed(&super::parse_scan_buffer_response(&reply)),
                "malformed/shape-violating reply must fail closed: {reply}",
            );
        }
    }
}
