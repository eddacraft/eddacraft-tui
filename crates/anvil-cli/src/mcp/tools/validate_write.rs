use std::fs;
use std::path::{Component, Path, PathBuf};

use anvil_checks::secret::patterns::DEFAULT_COMPILED_PATTERNS;
use anvil_kernel_types::diagnostics::ControlDecision;
use anvil_kernel_types::protection_claim::ProtectionClaim;
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::mcp::enforcement::{self, EnforcementMode};
use crate::mcp::validation::{
    DaemonStatus, DaemonValidationClient, INPUT_RULE_ID, LocalDaemonValidationClient,
    PRE_WRITE_MODE, PreWriteValidationRequest, ValidationBackend, ValidationBackendFailure,
    sanitise_id_part, validate_pre_write,
};

pub const TOOL_NAME: &str = "anvil_validate_write";

const RESPONSE_SCHEMA: &str = "anvil.mcp.validate-write.v1";
const MAX_PROPOSED_CONTENT_BYTES: usize = 1024 * 1024;

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Pre-write validation gate. Call this tool before EVERY file write to verify the proposed content does not introduce secrets, anti-patterns, or boundary violations. Honour `block` decisions; do not write files the tool refuses.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute workspace root. Defaults to the server cwd when omitted."
                },
                "path": {
                    "type": "string",
                    "description": "Workspace-relative path, or an absolute path inside workspaceRoot."
                },
                "operation": {
                    "type": "string",
                    "enum": ["create", "update", "delete", "rename"]
                },
                "proposedContent": {
                    "type": "string",
                    "description": "Full proposed UTF-8 file content after the operation."
                },
                "patch": {
                    "type": ["string", "null"],
                    "description": "Unified diff or client patch payload."
                },
                "contentSha256": {
                    "type": "string",
                    "description": "SHA-256 hex digest of the full proposed content. Pair with preview to send a slim payload without proposedContent."
                },
                "preview": {
                    "type": "string",
                    "description": "First lines of the proposed content. Used for partial validation when proposedContent is omitted."
                },
                "contentEncoding": {
                    "type": "string",
                    "enum": ["utf-8"],
                    "default": "utf-8"
                },
                "client": {
                    "type": "object",
                    "additionalProperties": true
                }
            },
            "required": ["path", "operation"],
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

pub fn call(arguments: &Value) -> Value {
    // The MCP launch contract requires the server cwd to resolve to a
    // real workspace directory: trust checks, `.anvil.yaml` lookups,
    // and symlink-escape rejection all read from this path. If the cwd
    // has been deleted out from under the running process (e.g.
    // `rm -rf` of the project dir while the agent is mid-flight), we
    // must surface a structured error rather than papering over with
    // `PathBuf::from(".")`, which would silently rebind every check
    // against an unrelated relative root and trigger spurious
    // workspace-escape errors with confusing messages.
    let default_workspace_root = match std::env::current_dir() {
        Ok(root) => root,
        Err(err) => {
            let problem = ToolProblem::new(
                "server-cwd-unavailable",
                "MCP server cwd is not accessible.",
            );
            // The cwd is gone, so we have no path context; report the
            // OS error verbatim in the diagnostic message via the
            // structured payload. The enforcement mode defaults to
            // Block (we have no `.anvil.yaml` to read).
            return tool_result(&server_cwd_unavailable_payload(problem, &err));
        }
    };
    call_with_workspace(arguments, &default_workspace_root)
}

fn call_with_workspace(arguments: &Value, default_workspace_root: &Path) -> Value {
    call_with_validation_client(
        arguments,
        default_workspace_root,
        &LocalDaemonValidationClient,
        &WorkspaceEnforcementResolver,
    )
}

/// Resolver for the per-workspace enforcement mode. The default
/// implementation reads `.anvil.yaml` per RTAI-006 / INTD-008 contract;
/// tests substitute a fixed-mode resolver to exercise each branch
/// without writing temp files.
trait EnforcementResolver {
    fn resolve(&self, workspace_root: &Path) -> EnforcementMode;
}

struct WorkspaceEnforcementResolver;

impl EnforcementResolver for WorkspaceEnforcementResolver {
    fn resolve(&self, workspace_root: &Path) -> EnforcementMode {
        enforcement::load_for_workspace(workspace_root)
    }
}

fn call_with_validation_client(
    arguments: &Value,
    default_workspace_root: &Path,
    daemon: &impl DaemonValidationClient,
    enforcement_resolver: &impl EnforcementResolver,
) -> Value {
    let request = match ValidateWriteRequest::parse(arguments, default_workspace_root) {
        Ok(request) => request,
        Err(problem) => {
            // Input problems short-circuit before we have a trusted
            // workspace root to read `.anvil.yaml` from. They always
            // map to `block` regardless of enforcement mode — the tool
            // cannot evaluate a request it cannot parse.
            return tool_result(&problem_payload(problem, None, EnforcementMode::Block));
        }
    };

    let enforcement_mode = enforcement_resolver.resolve(&request.workspace_root);

    if let Some(problem) = request.input_problem() {
        return tool_result(&problem_payload(
            problem,
            Some(&request.relative_path),
            enforcement_mode,
        ));
    }

    let mut backend = ValidationBackend::Embedded;
    // No content path means we never invoked the validation backend;
    // reflect that with `NotWired` (no daemon was consulted) rather
    // than implying "available" by default.
    let mut daemon_status = DaemonStatus::NotWired;
    let mut diagnostics = Vec::new();
    if let Some(content) = request.content.as_deref() {
        let validation = validate_pre_write(
            &PreWriteValidationRequest {
                relative_path: &request.relative_path,
                content,
            },
            daemon,
        );
        let validation = match validation {
            Ok(validation) => validation,
            Err(failure) => {
                return tool_result(&backend_failure_payload(
                    &request.relative_path,
                    failure,
                    enforcement_mode,
                ));
            }
        };
        backend = validation.backend;
        daemon_status = validation.daemon_status;
        diagnostics = validation.diagnostics;
    }

    // MLP2-051b: best-effort claim attached only when the daemon
    // served the validation. By the time we reach here only two
    // `DaemonStatus` values are possible:
    //   - `Available`  — daemon answered scan_buffer.
    //   - `NotWired`   — embedded fallback ran (silent demotion).
    // `Unavailable` (operational failure) short-circuits to the
    // backend-failure payload above before this point, so it is
    // structurally unreachable here; the gate keeps `Available` as
    // the only state that triggers the claim fetch so the embedded
    // path cannot over-claim daemon coverage. The fetch adds one
    // extra IPC round-trip (capped at 2 s in
    // `query_daemon_status_at`, distinct from the scan_buffer
    // 2 s cap, so the cumulative wall-clock ceiling for a
    // healthy-but-hung daemon is 4 s). A future optimisation can
    // fold the claim into the daemon's `scan_buffer` reply so this
    // gate disappears without changing the wire shape.
    debug_assert_ne!(
        daemon_status,
        DaemonStatus::Unavailable,
        "operational failure must short-circuit before the claim gate",
    );
    let protection_claim = if daemon_status == DaemonStatus::Available {
        daemon.query_protection_claim(&request.workspace_root)
    } else {
        None
    };

    let diagnostics = normalise_response_diagnostics(&diagnostics, backend);

    tool_result(&validation_payload(
        &request.relative_path,
        &diagnostics,
        backend,
        daemon_status,
        None,
        enforcement_mode,
        request.partial_scan,
        protection_claim.as_ref(),
    ))
}

fn tool_result(payload: &Value) -> Value {
    let is_error = payload["decision"] == "block" || payload.get("error").is_some();
    let text = serde_json::to_string(&payload).expect("validate-write payload serialises");
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": is_error
    })
}

fn problem_payload(
    problem: ToolProblem,
    path: Option<&str>,
    enforcement_mode: EnforcementMode,
) -> Value {
    let path = path.unwrap_or("<unknown>");
    let diagnostic = input_diagnostic(problem, path);
    // Input problems are tool-level errors, not rule findings. The
    // enforcement mode controls whether *rule* diagnostics block; a
    // malformed request is still rejected so the client cannot proceed
    // with content the server could not parse.
    //
    // Input problems never reach the validation backend, so the
    // daemon status is `NotWired`: we never consulted the daemon for
    // this request.
    validation_payload_with_decision(
        path,
        &[diagnostic],
        ValidationBackend::Embedded,
        DaemonStatus::NotWired,
        Some(problem),
        enforcement_mode,
        ControlDecision::Block,
        false,
        None,
    )
}

fn backend_failure_payload(
    path: &str,
    failure: ValidationBackendFailure,
    enforcement_mode: EnforcementMode,
) -> Value {
    // The `decision` field is set explicitly to `block` so the
    // response shape stays consistent with every other path through
    // the tool: a backend failure is a hard stop regardless of
    // enforcement mode (the operator cannot choose to "warn-through"
    // a failure to validate at all). An `OperationalFailure` here
    // means the daemon was wired and tried to answer but failed —
    // hence `daemonStatus: "unavailable"` (distinct from `not-wired`,
    // which is "no daemon was ever consulted").
    json!({
        "schema": RESPONSE_SCHEMA,
        "decision": ControlDecision::Block,
        "error": {
            "code": failure.code,
            "message": failure.message,
            "retriable": failure.retriable
        },
        "safeDefault": "do-not-write",
        "correlation": {
            "id": correlation_id(path),
            "surface": "mcp",
            "mode": "preWrite",
            "backend": ValidationBackend::Daemon.as_str(),
            "daemonStatus": DaemonStatus::Unavailable.as_str(),
            "path": path,
            "enforcementMode": enforcement_mode.as_str()
        }
    })
}

fn server_cwd_unavailable_payload(problem: ToolProblem, err: &std::io::Error) -> Value {
    // The cwd is gone, so we have no workspace path to anchor the
    // correlation ID against. Use a static fallback path string so
    // the agent can still correlate this response with its request.
    let path = "<server-cwd>";
    json!({
        "schema": RESPONSE_SCHEMA,
        "decision": ControlDecision::Block,
        "error": {
            "code": problem.code,
            "message": format!("{}: {err}", problem.message),
            "retriable": false
        },
        "safeDefault": "do-not-write",
        "correlation": {
            "id": correlation_id(path),
            "surface": "mcp",
            "mode": "preWrite",
            "backend": ValidationBackend::Embedded.as_str(),
            "daemonStatus": DaemonStatus::NotWired.as_str(),
            "path": path,
            "enforcementMode": EnforcementMode::default().as_str()
        }
    })
}

// Same justification as `validation_payload_with_decision` below: the
// eight inputs are individually meaningful fields of the response
// envelope and folding them into a builder/struct only relocates the
// arity. Inherits the override from its sole call site.
#[allow(clippy::too_many_arguments)]
fn validation_payload(
    path: &str,
    diagnostics: &[Diagnostic],
    backend: ValidationBackend,
    daemon_status: DaemonStatus,
    problem: Option<ToolProblem>,
    enforcement_mode: EnforcementMode,
    partial_scan: bool,
    protection_claim: Option<&ProtectionClaim>,
) -> Value {
    let decision = enforcement::decision_for(diagnostics, enforcement_mode);
    validation_payload_with_decision(
        path,
        diagnostics,
        backend,
        daemon_status,
        problem,
        enforcement_mode,
        decision,
        partial_scan,
        protection_claim,
    )
}

// Nine related fields, all part of the `anvil.mcp.validate-write.v1`
// response envelope; folding them into a struct would just move the
// argument count to that struct's constructor without simplifying the
// shape.
#[allow(clippy::too_many_arguments)]
fn validation_payload_with_decision(
    path: &str,
    diagnostics: &[Diagnostic],
    backend: ValidationBackend,
    daemon_status: DaemonStatus,
    problem: Option<ToolProblem>,
    enforcement_mode: EnforcementMode,
    decision: ControlDecision,
    partial_scan: bool,
    protection_claim: Option<&ProtectionClaim>,
) -> Value {
    // The `enforcementMode` and `daemonStatus` correlation fields are
    // part of the `anvil.mcp.validate-write.v1` schema and are
    // currently tool-local. RTAI-007 / DRVR-002 may promote them to
    // the canonical correlation envelope in `anvil-kernel-types`; we
    // leave them here for now so the launch shim can ship without
    // committing the wider envelope to a stable shape.
    let mut payload = json!({
        "schema": RESPONSE_SCHEMA,
        "decision": decision,
        "summary": diagnostic_summary(diagnostics),
        "diagnostics": diagnostics,
        "correlation": {
            "id": correlation_id(path),
            "surface": "mcp",
            "mode": "preWrite",
            "backend": backend.as_str(),
            "daemonStatus": daemon_status.as_str(),
            "path": path,
            "enforcementMode": enforcement_mode.as_str()
        }
    });

    if partial_scan {
        payload["correlation"]["partialScan"] = json!(true);
    }

    if decision == ControlDecision::Block {
        payload["safeDefault"] = json!("do-not-write");
    }

    if let Some(problem) = problem {
        payload["error"] = json!({
            "code": problem.code,
            "message": problem.message,
            "retriable": false
        });
    }

    // MLP2-051b: wire-additive `protection_claim`. Omitted when the
    // daemon could not supply a snapshot (the field is `Option`-shaped
    // for round-trip parity with the producer-side struct). Drivers
    // pinned to a pre-MLP2-051b shape continue to parse this response
    // unchanged because the new field is the only addition and is
    // omitted from the default no-daemon path.
    if let Some(claim) = protection_claim {
        payload["protection_claim"] =
            serde_json::to_value(claim).expect("ProtectionClaim serialises");
    }

    payload
}

pub(crate) fn diagnostic_summary(diagnostics: &[Diagnostic]) -> Value {
    let mut error = 0usize;
    let mut warning = 0usize;
    let mut info = 0usize;

    for diagnostic in diagnostics {
        match diagnostic.severity {
            Severity::Error => error += 1,
            Severity::Warning => warning += 1,
            Severity::Info => info += 1,
        }
    }

    json!({
        "total": diagnostics.len(),
        "bySeverity": {
            "error": error,
            "warning": warning,
            "info": info
        }
    })
}

pub(crate) fn normalise_response_diagnostics(
    diagnostics: &[Diagnostic],
    _backend: ValidationBackend,
) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .cloned()
        .map(|mut diagnostic| {
            if diagnostic.category == Category::Secret {
                diagnostic.id = redact_secret_id(&diagnostic.id, false);
                diagnostic.summary =
                    "Potential secret detected; remove it from the proposed write.".to_string();
                diagnostic.location.file = redact_secret_values(&diagnostic.location.file);
                diagnostic.source.rule_id = redact_secret_values(&diagnostic.source.rule_id);
                diagnostic.source.source_module =
                    redact_secret_values(&diagnostic.source.source_module);
                diagnostic.remediation_hint = Some(
                    "Remove the secret from the proposed write; use a placeholder or environment variable instead."
                        .to_string(),
                );
                if let Mode::Unknown(value) = &mut diagnostic.mode {
                    *value = redact_secret_values(value);
                }
            }
            diagnostic
        })
        .collect()
}

fn redact_secret_id(id: &str, strict: bool) -> String {
    let redacted = redact_secret_values(id);
    if !strict || redacted != id {
        return redacted;
    }

    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    let digest = hasher.finalize();
    format!("diag_mcp_secret_redacted_{}", hex::encode(&digest[..6]))
}

fn redact_secret_values(value: &str) -> String {
    DEFAULT_COMPILED_PATTERNS
        .iter()
        .fold(value.to_string(), |current, pattern| {
            pattern
                .regex
                .replace_all(&current, "[REDACTED]")
                .into_owned()
        })
}

fn input_diagnostic(problem: ToolProblem, path: &str) -> Diagnostic {
    Diagnostic::new(
        format!(
            "diag_prewrite_{}_{}",
            sanitise_id_part(path),
            sanitise_id_part(problem.code)
        ),
        Severity::Error,
        problem.message,
        Location {
            file: path.to_string(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Policy,
        DiagnosticSource {
            rule_id: INPUT_RULE_ID.to_string(),
            source_module: "anvil-cli::mcp".to_string(),
        },
        Mode::Unknown(PRE_WRITE_MODE.to_string()),
    )
}

pub(crate) fn correlation_id(path: &str) -> String {
    format!("corr_mcp_{}", sanitise_id_part(path))
}

#[derive(Debug, Clone, Copy)]
struct ToolProblem {
    code: &'static str,
    message: &'static str,
}

impl ToolProblem {
    const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

struct ValidateWriteRequest {
    workspace_root: PathBuf,
    relative_path: String,
    operation: Operation,
    content: Option<String>,
    patch_only: bool,
    partial_scan: bool,
    preflight_problem: Option<ToolProblem>,
}

impl ValidateWriteRequest {
    fn parse(arguments: &Value, default_workspace_root: &Path) -> Result<Self, ToolProblem> {
        let Some(arguments) = arguments.as_object() else {
            return Err(ToolProblem::new(
                "invalid-tool-arguments",
                "Validate-write arguments must be an object.",
            ));
        };

        let workspace_root =
            workspace_root(arguments.get("workspaceRoot"), default_workspace_root)?;
        let path = required_string(
            arguments.get("path"),
            "missing-path",
            "Validate-write requires a path.",
        )?;
        let operation = Operation::parse(required_string(
            arguments.get("operation"),
            "missing-operation",
            "Validate-write requires an operation.",
        )?)?;
        let relative_path = resolve_workspace_path(&workspace_root, path)?;
        reject_symlink_escape(&workspace_root, &relative_path)?;

        let content_encoding =
            optional_string(arguments.get("contentEncoding"))?.unwrap_or("utf-8");
        let preflight_problem = (content_encoding != "utf-8").then(|| {
            ToolProblem::new(
                "unsupported-encoding",
                "Only UTF-8 proposed content is supported by launch validation.",
            )
        });

        let proposed_content = optional_string(arguments.get("proposedContent"))?;
        let preview_content = optional_string(arguments.get("preview"))?;
        let patch_content = optional_string(arguments.get("patch"))?;
        // When proposedContent is absent but preview is present, use preview
        // for partial validation (the caller sent a slim payload). When both
        // are present, proposedContent is authoritative. Patch text is never
        // scanned as file content because diff hunks include removed lines and
        // metadata that would mislead the secret/reasoning checks.
        let (content, partial_scan) = match proposed_content {
            Some(full) => (Some(full.to_string()), false),
            None => match preview_content {
                Some(preview) => (Some(preview.to_string()), true),
                None => (None, false),
            },
        };
        let patch_only = content.is_none() && patch_content.is_some();

        Ok(Self {
            workspace_root,
            relative_path,
            operation,
            content,
            patch_only,
            partial_scan,
            preflight_problem,
        })
    }

    fn input_problem(&self) -> Option<ToolProblem> {
        if let Some(problem) = self.preflight_problem {
            return Some(problem);
        }

        if self.content.is_none() && self.operation.requires_content() {
            if self.patch_only {
                return Some(ToolProblem::new(
                    "patch-only-unsupported",
                    "Patch-only validation is not supported in this release; supply proposedContent.",
                ));
            }
            return Some(ToolProblem::new(
                "missing-content",
                "Validate-write requires proposedContent for this operation.",
            ));
        }

        let content = self.content.as_deref()?;

        if content.len() > MAX_PROPOSED_CONTENT_BYTES {
            return Some(ToolProblem::new(
                "oversize-content",
                "The proposed content is too large for launch validation.",
            ));
        }

        if content.contains('\0') {
            return Some(ToolProblem::new(
                "binary-content",
                "Binary content is not supported by launch validation.",
            ));
        }

        None
    }
}

#[derive(Clone, Copy)]
enum Operation {
    Create,
    Update,
    Delete,
    Rename,
}

impl Operation {
    fn parse(value: &str) -> Result<Self, ToolProblem> {
        match value {
            "create" => Ok(Self::Create),
            "update" => Ok(Self::Update),
            "delete" => Ok(Self::Delete),
            "rename" => Ok(Self::Rename),
            _ => Err(ToolProblem::new(
                "unsupported-operation",
                "Validate-write operation must be create, update, delete, or rename.",
            )),
        }
    }

    const fn requires_content(self) -> bool {
        matches!(self, Self::Create | Self::Update)
    }
}

fn workspace_root(
    value: Option<&Value>,
    default_workspace_root: &Path,
) -> Result<PathBuf, ToolProblem> {
    let default_workspace_root = canonical_workspace_root(default_workspace_root)?;

    match value {
        Some(Value::String(root)) => {
            let root = PathBuf::from(root);
            if !root.is_absolute() {
                return Err(ToolProblem::new(
                    "invalid-workspace-root",
                    "workspaceRoot must be an absolute path.",
                ));
            }
            let root = canonical_workspace_root(&root)?;
            if root == default_workspace_root {
                Ok(root)
            } else {
                Err(ToolProblem::new(
                    "untrusted-workspace-root",
                    "workspaceRoot must match the MCP server workspace.",
                ))
            }
        }
        Some(_) => Err(ToolProblem::new(
            "invalid-workspace-root",
            "workspaceRoot must be a string when provided.",
        )),
        None => Ok(default_workspace_root),
    }
}

fn canonical_workspace_root(root: &Path) -> Result<PathBuf, ToolProblem> {
    let root = fs::canonicalize(root).map_err(|_| {
        ToolProblem::new(
            "invalid-workspace-root",
            "workspaceRoot must resolve to an existing directory.",
        )
    })?;

    if root.is_dir() {
        Ok(root)
    } else {
        Err(ToolProblem::new(
            "invalid-workspace-root",
            "workspaceRoot must resolve to an existing directory.",
        ))
    }
}

fn required_string<'a>(
    value: Option<&'a Value>,
    code: &'static str,
    message: &'static str,
) -> Result<&'a str, ToolProblem> {
    match value {
        Some(Value::String(value)) if !value.is_empty() => Ok(value),
        Some(Value::String(_)) | None => Err(ToolProblem::new(code, message)),
        Some(_) => Err(ToolProblem::new(
            "invalid-tool-arguments",
            "Validate-write arguments have invalid field types.",
        )),
    }
}

fn optional_string(value: Option<&Value>) -> Result<Option<&str>, ToolProblem> {
    match value {
        Some(Value::String(value)) => Ok(Some(value)),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(ToolProblem::new(
            "invalid-tool-arguments",
            "Validate-write arguments have invalid field types.",
        )),
    }
}

fn resolve_workspace_path(workspace_root: &Path, path: &str) -> Result<String, ToolProblem> {
    let path = Path::new(path);
    let relative = if path.is_absolute() {
        let normalised = normalise_absolute_path(path)?;
        normalised
            .strip_prefix(workspace_root)
            .map_err(|_| workspace_escape_problem())?
            .to_path_buf()
    } else {
        normalise_relative_path(path)?
    };

    if relative.as_os_str().is_empty() {
        return Err(ToolProblem::new(
            "missing-path",
            "Validate-write requires a path.",
        ));
    }

    Ok(path_to_slash_string(&relative))
}

fn normalise_relative_path(path: &Path) -> Result<PathBuf, ToolProblem> {
    let mut normalised = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalised.push(part),
            // Reject `..` outright. Lexically collapsing parent-dir
            // segments before symlink resolution is unsound: a path like
            // `link/../target`, where `link` is a symlink to a location
            // outside the workspace, would normalise to `target` (in
            // workspace) but resolve at write time to the symlink target's
            // parent, escaping `workspaceRoot`. Root and prefix segments are
            // rejected for the same reason — relative paths must stay
            // unambiguously inside the workspace.
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(workspace_escape_problem());
            }
        }
    }
    Ok(normalised)
}

fn normalise_absolute_path(path: &Path) -> Result<PathBuf, ToolProblem> {
    let mut normalised = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalised.push(prefix.as_os_str()),
            Component::RootDir => normalised.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => normalised.push(part),
            // Same rationale as `normalise_relative_path`: lexically
            // collapsing `..` before symlink resolution would let symlinked
            // segments smuggle the resolved path outside `workspaceRoot`.
            Component::ParentDir => return Err(workspace_escape_problem()),
        }
    }
    Ok(normalised)
}

fn reject_symlink_escape(workspace_root: &Path, relative_path: &str) -> Result<(), ToolProblem> {
    let candidate = workspace_root.join(relative_path);
    let mut anchor = candidate.as_path();
    while !anchor.exists() {
        let Some(parent) = anchor.parent() else {
            return Err(workspace_escape_problem());
        };
        anchor = parent;
    }

    let canonical_anchor = fs::canonicalize(anchor).map_err(|_| workspace_escape_problem())?;
    if canonical_anchor.starts_with(workspace_root) {
        Ok(())
    } else {
        Err(workspace_escape_problem())
    }
}

fn workspace_escape_problem() -> ToolProblem {
    ToolProblem::new(
        "workspace-escape",
        "Validate-write path must stay inside workspaceRoot.",
    )
}

fn path_to_slash_string(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::descriptor;
    use super::{
        EnforcementResolver, MAX_PROPOSED_CONTENT_BYTES, call_with_validation_client,
        redact_secret_id,
    };
    use crate::mcp::enforcement::EnforcementMode;
    use crate::mcp::validation::{
        DaemonValidationClient, DaemonValidationOutcome, PreWriteValidationRequest,
        ValidationBackendFailure,
    };
    #[cfg(unix)]
    use anvil_intercept::Shutdown;
    #[cfg(unix)]
    use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
    use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};
    use serde_json::{Value, json};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    #[cfg(unix)]
    use std::sync::Mutex;
    use tempfile::tempdir;
    #[cfg(unix)]
    use tokio::runtime::Runtime;

    /// Global lock for tests that mutate the process cwd. Cargo runs
    /// unit tests in parallel; without serialisation here, the
    /// deleted-cwd test could race against any other test that calls
    /// `current_dir` (e.g. embedded validation paths that resolve
    /// relative file lookups).
    #[cfg(unix)]
    static CWD_GUARD: Mutex<()> = Mutex::new(());

    /// RAII helper for the `deleted_server_cwd_*` test: restores the
    /// captured cwd on drop so the test runner's working directory is
    /// always reinstated, even if the test body panics.
    #[cfg(unix)]
    struct CwdRestore(std::path::PathBuf);
    #[cfg(unix)]
    impl Drop for CwdRestore {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }

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

    /// Test-only resolver that returns a fixed enforcement mode regardless
    /// of workspace contents. Lets unit tests exercise each branch of the
    /// enforcement-mode policy without writing temporary `.anvil.yaml`
    /// fixtures (those live in the dedicated E2E suite).
    struct FixedEnforcement(EnforcementMode);

    impl EnforcementResolver for FixedEnforcement {
        fn resolve(&self, _workspace_root: &Path) -> EnforcementMode {
            self.0
        }
    }

    #[test]
    fn descriptor_advertises_supported_content_encodings() {
        let descriptor = descriptor();

        assert_eq!(
            descriptor["inputSchema"]["properties"]["contentEncoding"]["enum"],
            json!(["utf-8"])
        );
    }

    #[test]
    fn clean_content_allows_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(
            payload,
            json!({
                "schema": "anvil.mcp.validate-write.v1",
                "decision": "allow",
                "summary": {
                    "total": 0,
                    "bySeverity": {
                        "error": 0,
                        "warning": 0,
                        "info": 0,
                    }
                },
                "diagnostics": [],
                "correlation": {
                    "id": "corr_mcp_src_example_ts",
                    "surface": "mcp",
                    "mode": "preWrite",
                    "backend": "embedded",
                    "daemonStatus": "not-wired",
                    "path": "src/example.ts",
                    "enforcementMode": "block",
                }
            })
        );
    }

    #[test]
    fn secret_detection_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["summary"]["bySeverity"]["error"], 1);
        assert_eq!(payload["diagnostics"][0]["category"], "secret");
        assert_eq!(
            payload["diagnostics"][0]["summary"],
            "Potential secret detected; remove it from the proposed write."
        );
        assert_eq!(
            payload["diagnostics"][0]["remediation_hint"],
            "Remove the secret from the proposed write; use a placeholder or environment variable instead."
        );
        assert_eq!(
            payload["diagnostics"][0]["source"]["rule_id"],
            "secret-detection"
        );
        assert!(
            !serde_json::to_string(&payload)
                .expect("payload serialises")
                .contains("ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }

    #[test]
    fn daemon_backend_payload_is_reported_when_available() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Diagnostics(vec![sample_daemon_diagnostic()]),
        };
        let result = call_with_validation_client(
            &json!({
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Block),
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["correlation"]["backend"], "daemon");
        assert_eq!(payload["correlation"]["enforcementMode"], "block");
        assert_eq!(
            payload["diagnostics"][0]["source"]["rule_id"],
            "secret-detection"
        );
        assert_eq!(
            payload["diagnostics"][0]["location"]["file"],
            "src/secret.ts"
        );
    }

    #[test]
    fn daemon_secret_diagnostics_are_redacted_before_mcp_response() {
        let workspace = tempdir().expect("workspace exists");
        let raw_secret = "ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut diagnostic = sample_daemon_diagnostic();
        let daemon_secret_id = "custom-daemon-secret-diagnostic";
        diagnostic.id = daemon_secret_id.to_string();
        diagnostic.summary = format!("Potential secret detected: {raw_secret}");
        diagnostic.location.file = format!("src/{raw_secret}.ts");
        diagnostic.source.rule_id = format!("secret-detection-{raw_secret}");
        diagnostic.source.source_module = format!("daemon::{raw_secret}");
        diagnostic.mode = Mode::Unknown(format!("pre-write-{raw_secret}"));
        diagnostic.remediation_hint = Some(format!("Remove {raw_secret} from the proposed write."));
        let non_secret = Diagnostic::new(
            "diag_reasoning_001",
            Severity::Warning,
            "Reasoning pattern detected",
            Location {
                file: "src/notes.ts".to_string(),
                line: Some(1),
                column: None,
                end_line: None,
                end_column: None,
            },
            Category::Reasoning,
            DiagnosticSource {
                rule_id: "reasoning-pattern".to_string(),
                source_module: "anvil-checks::reasoning".to_string(),
            },
            Mode::Unknown("pre-write".to_string()),
        );
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Diagnostics(vec![non_secret, diagnostic]),
        };
        let result = call_with_validation_client(
            &json!({
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Block),
        );
        let payload = parse_payload(&result);
        let response_text = serde_json::to_string(&payload).expect("payload serialises");
        let expected_redacted_id = redact_secret_id(daemon_secret_id, false);

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["diagnostics"][0]["id"], "diag_reasoning_001");
        assert_eq!(
            payload["diagnostics"][1]["id"].as_str(),
            Some(expected_redacted_id.as_str())
        );
        assert_eq!(
            payload["diagnostics"][1]["schema_version"],
            "anvil.diagnostic.v1"
        );
        assert!(!response_text.contains(raw_secret));
        assert_eq!(
            payload["diagnostics"][1]["summary"],
            "Potential secret detected; remove it from the proposed write."
        );
        assert_eq!(
            payload["diagnostics"][1]["remediation_hint"],
            "Remove the secret from the proposed write; use a placeholder or environment variable instead."
        );
        assert_eq!(
            payload["diagnostics"][1]["source"]["rule_id"],
            "secret-detection-[REDACTED]"
        );
        assert_eq!(
            payload["diagnostics"][1]["source"]["source_module"],
            "daemon::[REDACTED]"
        );
    }

    #[test]
    fn daemon_and_embedded_paths_emit_identical_diagnostic_envelopes() {
        let workspace = tempdir().expect("workspace exists");
        let arguments = json!({
            "path": "src/secret.ts",
            "operation": "create",
            "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
        });
        let embedded = parse_payload(&call_with_validation_client(
            &arguments,
            workspace.path(),
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
            &FixedEnforcement(EnforcementMode::Block),
        ));
        let embedded_diagnostics: Vec<Diagnostic> =
            serde_json::from_value(embedded["diagnostics"].clone())
                .expect("embedded diagnostics deserialize");
        let daemon = parse_payload(&call_with_validation_client(
            &arguments,
            workspace.path(),
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Diagnostics(embedded_diagnostics),
            },
            &FixedEnforcement(EnforcementMode::Block),
        ));

        assert_eq!(daemon["diagnostics"], embedded["diagnostics"]);
    }

    // Match `validate_connected_peer_for_client`'s cfg gate: peer-cred is
    // implemented on Linux (SO_PEERCRED) and macOS (getpeereid) only; on
    // BSD/Solaris the helper still returns "not implemented", which would
    // make this test fail deterministically. Narrow from cfg(unix) so the
    // test only runs where the daemon path is expected to succeed.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn live_daemon_mcp_tool_call_matches_embedded_diagnostic_envelope() {
        let runtime = Runtime::new().expect("tokio runtime starts");
        let runtime_dir = tempdir().expect("runtime dir exists");
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir permissions tightened");
        let socket = runtime_dir.path().join("intercept.sock");
        let _runtime_guard = runtime.enter();
        let listener = IpcListener::bind(&socket, NoopDispatcher).expect("daemon socket binds");
        let (shutdown, token) = Shutdown::new();
        let server = runtime.spawn(listener.serve(token));

        let workspace = tempdir().expect("workspace exists");
        let arguments = json!({
            "path": "src/secret.ts",
            "operation": "create",
            "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
        });
        let embedded = parse_payload(&call_with_validation_client(
            &arguments,
            workspace.path(),
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
            &FixedEnforcement(EnforcementMode::Block),
        ));
        let daemon = parse_payload(&call_with_validation_client(
            &arguments,
            workspace.path(),
            &super::LocalDaemonValidationClient::with_socket_path(socket),
            &FixedEnforcement(EnforcementMode::Block),
        ));

        shutdown.trigger();
        runtime.block_on(async {
            server
                .await
                .expect("daemon task joins")
                .expect("daemon exits cleanly");
        });

        assert_eq!(daemon["correlation"]["backend"], "daemon");
        assert_eq!(daemon["correlation"]["daemonStatus"], "available");
        assert_eq!(daemon["decision"], embedded["decision"]);
        assert_eq!(daemon["diagnostics"], embedded["diagnostics"]);
    }

    #[test]
    fn daemon_operational_failure_blocks_without_fallback() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::OperationalFailure(ValidationBackendFailure {
                code: "validation-backend-unavailable",
                message: "Anvil could not validate the proposed write.",
                retriable: true,
            }),
        };
        let result = call_with_validation_client(
            &json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Block),
        );
        let payload = parse_payload(&result);

        assert_eq!(result["isError"], true);
        assert_eq!(payload["error"]["code"], "validation-backend-unavailable");
        assert_eq!(payload["error"]["retriable"], true);
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["correlation"]["backend"], "daemon");
        // The response shape now carries `decision: "block"` on the
        // backend-failure path so callers can pattern-match the same
        // field across every response branch (council finding 1).
        assert_eq!(payload["decision"], "block");
        // The daemon was wired and failed operationally — distinct
        // from `not-wired`, where no daemon was consulted.
        assert_eq!(payload["correlation"]["daemonStatus"], "unavailable");
        assert!(payload.get("diagnostics").is_none());
        assert!(payload.get("summary").is_none());
    }

    #[test]
    fn proposed_content_authoritative_when_patch_also_supplied() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "proposedContent": "export const value = 1;\n",
                "patch": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        // Patch text is correlation metadata only — its embedded "secret"
        // must not influence validation when proposedContent is supplied.
        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["summary"]["total"], 0);
        assert_eq!(payload["diagnostics"], json!([]));
    }

    #[test]
    fn patch_only_blocks_as_unsupported() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "patch": "--- a/src/example.ts\n+++ b/src/example.ts\n@@\n-old\n+new\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "patch-only-unsupported");
        assert_eq!(payload["safeDefault"], "do-not-write");
    }

    #[test]
    fn reasoning_pattern_warns_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/reasoning.ts",
                "operation": "update",
                "proposedContent": "// the lead said to skip this branch\nexport const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "warn");
        assert_eq!(payload["summary"]["bySeverity"]["info"], 1);
        assert_eq!(payload["diagnostics"][0]["category"], "reasoning");
        assert_eq!(payload["diagnostics"][0]["mode"], "pre-write");
    }

    #[test]
    fn missing_path_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "missing-path");
        assert_eq!(payload["safeDefault"], "do-not-write");
    }

    #[test]
    fn binary_content_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/blob.bin",
                "operation": "create",
                "proposedContent": "abc\0def"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "binary-content");
    }

    #[test]
    fn unsupported_encoding_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/blob.bin",
                "operation": "create",
                "contentEncoding": "base64",
                "proposedContent": "aGVsbG8="
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "unsupported-encoding");
    }

    #[test]
    fn oversize_content_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/huge.ts",
                "operation": "create",
                "proposedContent": "x".repeat(MAX_PROPOSED_CONTENT_BYTES + 1)
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "oversize-content");
    }

    #[test]
    fn workspace_escape_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "../outside.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "workspace-escape");
    }

    #[test]
    fn parent_dir_traversal_blocks_write_even_when_lexically_in_workspace() {
        // A path like `link/../target` would lexically normalise to `target`
        // (inside the workspace), but if `link` were a symlink to an
        // external directory the actual write target would escape the
        // workspace. Reject any `..` segment outright.
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "link/../target.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "workspace-escape");
    }

    #[test]
    fn absolute_parent_dir_traversal_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let mut absolute = workspace.path().to_path_buf();
        absolute.push("link");
        absolute.push("..");
        absolute.push("target.ts");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": absolute.to_string_lossy(),
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "workspace-escape");
    }

    #[test]
    fn client_controlled_workspace_root_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let other_workspace = tempdir().expect("other workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "workspaceRoot": other_workspace.path().to_string_lossy(),
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "untrusted-workspace-root");
    }

    #[test]
    fn file_valued_workspace_root_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let root_file = workspace.path().join("not-a-directory");
        fs::write(&root_file, "not a directory").expect("root fixture file written");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "workspaceRoot": root_file.to_string_lossy(),
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "invalid-workspace-root");
    }

    /// RTAI-006: in `block` mode (default), a secret diagnostic
    /// rejects the write and the response carries `safeDefault`.
    #[test]
    fn enforcement_mode_block_rejects_secret_write() {
        let workspace = tempdir().expect("workspace exists");
        let result = call_with_validation_client(
            &json!({
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
            &FixedEnforcement(EnforcementMode::Block),
        );
        let payload = parse_payload(&result);

        assert_eq!(result["isError"], true);
        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["correlation"]["enforcementMode"], "block");
        assert_eq!(payload["summary"]["bySeverity"]["error"], 1);
        assert_eq!(payload["diagnostics"][0]["category"], "secret");
    }

    /// RTAI-006: in `warn` mode, a secret diagnostic that would
    /// otherwise block becomes a warning. The diagnostic is still
    /// returned verbatim so the agent can show it to the user.
    #[test]
    fn enforcement_mode_warn_downgrades_secret_block_to_warn() {
        let workspace = tempdir().expect("workspace exists");
        let result = call_with_validation_client(
            &json!({
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
            &FixedEnforcement(EnforcementMode::Warn),
        );
        let payload = parse_payload(&result);

        // Diagnostics are still returned, but the decision is now `warn`
        // and the payload carries no `safeDefault` flag.
        assert_eq!(result["isError"], false);
        assert_eq!(payload["decision"], "warn");
        assert!(payload.get("safeDefault").is_none());
        assert_eq!(payload["correlation"]["enforcementMode"], "warn");
        assert_eq!(payload["summary"]["bySeverity"]["error"], 1);
        assert_eq!(payload["diagnostics"][0]["category"], "secret");
    }

    /// RTAI-006: in `off` mode, diagnostics are returned but the
    /// decision is always `allow`. This is the operator-pull-the-handbrake
    /// mode for noisy environments where Anvil should report findings
    /// without ever blocking the agent.
    #[test]
    fn enforcement_mode_off_passes_secret_write_with_diagnostics() {
        let workspace = tempdir().expect("workspace exists");
        let result = call_with_validation_client(
            &json!({
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
            &FixedEnforcement(EnforcementMode::Off),
        );
        let payload = parse_payload(&result);

        assert_eq!(result["isError"], false);
        assert_eq!(payload["decision"], "allow");
        assert!(payload.get("safeDefault").is_none());
        assert_eq!(payload["correlation"]["enforcementMode"], "off");
        // Diagnostics are still surfaced so the agent can see what
        // would have blocked the write under stricter modes.
        assert_eq!(payload["summary"]["bySeverity"]["error"], 1);
        assert_eq!(payload["diagnostics"][0]["category"], "secret");
    }

    /// RTAI-006: enforcement mode does not paper over malformed input.
    /// A missing path is a tool-level error and must always block.
    #[test]
    fn enforcement_mode_off_still_rejects_malformed_input() {
        let workspace = tempdir().expect("workspace exists");
        let result = call_with_validation_client(
            &json!({
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &super::LocalDaemonValidationClient,
            &FixedEnforcement(EnforcementMode::Off),
        );
        let payload = parse_payload(&result);

        assert_eq!(result["isError"], true);
        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "missing-path");
    }

    /// RTAI-006: when `.anvil.yaml` declares `enforcement.mode: warn`,
    /// the workspace resolver picks it up end-to-end without any
    /// in-process override. This is the unit-level proof of the
    /// `.anvil.yaml` -> tool-response wire that the E2E test also
    /// covers via the live binary.
    #[test]
    fn anvil_yaml_warn_mode_is_honoured_end_to_end() {
        let workspace = tempdir().expect("workspace exists");
        std::fs::write(
            workspace.path().join(".anvil.yaml"),
            "enforcement:\n  mode: warn\n",
        )
        .expect("write fixture");

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(payload["decision"], "warn");
        assert_eq!(payload["correlation"]["enforcementMode"], "warn");
        assert!(payload.get("safeDefault").is_none());
        assert_eq!(payload["summary"]["bySeverity"]["error"], 1);
    }

    /// RTAI-006: the `Unavailable` daemon path silently demotes to
    /// the embedded validator. Without an explicit signal in the
    /// response, callers can't tell "embedded by design" from
    /// "daemon was expected and unavailable". `correlation.daemonStatus`
    /// surfaces the demotion regardless of the configured enforcement
    /// mode — `warn` here. Council finding 3.
    #[test]
    fn unavailable_daemon_under_warn_mode_carries_demotion_signal() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Unavailable,
        };
        let result = call_with_validation_client(
            &json!({
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Warn),
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["decision"], "warn");
        assert_eq!(payload["correlation"]["enforcementMode"], "warn");
        assert_eq!(payload["correlation"]["backend"], "embedded");
        assert_eq!(payload["correlation"]["daemonStatus"], "not-wired");
    }

    /// Same demotion-signal contract as the warn-mode test, but
    /// exercised under `off`. Council finding 3.
    #[test]
    fn unavailable_daemon_under_off_mode_carries_demotion_signal() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Unavailable,
        };
        let result = call_with_validation_client(
            &json!({
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Off),
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["correlation"]["enforcementMode"], "off");
        assert_eq!(payload["correlation"]["backend"], "embedded");
        assert_eq!(payload["correlation"]["daemonStatus"], "not-wired");
    }

    /// RTAI-006: a deleted server cwd surfaces a structured
    /// `server-cwd-unavailable` error rather than silently rebinding
    /// to a relative `.` path that would confuse downstream checks.
    /// Council finding 4.
    #[cfg(unix)]
    #[test]
    fn deleted_server_cwd_surfaces_structured_error() {
        // `set_current_dir` is process-global. Hold the cwd mutex for
        // the duration of the test so concurrent unit tests cannot
        // observe each other's directory state. The guard is released
        // when `_lock` drops at the end of the function.
        let _lock = CWD_GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original_cwd = std::env::current_dir().expect("test runner has a cwd");
        // Drop order matters: `_restore` drops before `_lock`, so the
        // cwd is back before any other test grabs the lock.

        let scratch = tempdir().expect("scratch workspace exists");
        let scratch_path = scratch.path().to_path_buf();
        std::env::set_current_dir(&scratch_path).expect("cd into scratch dir");
        let _restore = CwdRestore(original_cwd);
        // Drop the TempDir so the directory is removed while the
        // process cwd still points at it. `std::env::current_dir`
        // will now return an error.
        drop(scratch);

        let result = super::call(&json!({
            "path": "src/example.ts",
            "operation": "create",
            "proposedContent": "export const value = 1;\n"
        }));

        let payload = parse_payload(&result);
        assert_eq!(result["isError"], true);
        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["error"]["code"], "server-cwd-unavailable");
        assert_eq!(payload["correlation"]["daemonStatus"], "not-wired");
    }

    fn parse_payload(result: &Value) -> Value {
        let text = result["content"][0]["text"]
            .as_str()
            .expect("tool result contains JSON text");
        serde_json::from_str(text).expect("tool result text is JSON")
    }

    fn call_payload(workspace_root: &std::path::Path, arguments: &Value) -> Value {
        let result = call_with_validation_client(
            arguments,
            workspace_root,
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
            &super::WorkspaceEnforcementResolver,
        );
        assert_eq!(result["content"][0]["type"], "text");
        parse_payload(&result)
    }

    #[test]
    fn preview_content_allows_write_when_clean() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "preview": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["correlation"]["partialScan"], true);
    }

    #[test]
    fn preview_content_blocks_when_secret_detected() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/secret.ts",
                "operation": "update",
                "preview": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["correlation"]["partialScan"], true);
    }

    #[test]
    fn full_content_does_not_set_partial_scan() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "allow");
        assert!(payload["correlation"]["partialScan"].is_null());
    }

    fn sample_daemon_diagnostic() -> Diagnostic {
        Diagnostic::new(
            "diag_prewrite_src_secret_ts_1_github-token",
            Severity::Error,
            "Potential secret detected (GitHub Token)",
            Location {
                file: "src/secret.ts".to_string(),
                line: Some(1),
                column: None,
                end_line: None,
                end_column: None,
            },
            Category::Secret,
            DiagnosticSource {
                rule_id: "secret-detection".to_string(),
                source_module: "anvil-checks::secret".to_string(),
            },
            Mode::Unknown("pre-write".to_string()),
        )
        .with_remediation_hint("Use a placeholder or environment variable instead.")
    }

    /// Test fixture that mirrors the production [`FixtureDaemon`] but
    /// also surfaces a canned [`ProtectionClaim`] through the new
    /// [`DaemonValidationClient::query_protection_claim`] trait method.
    /// Kept separate so the existing `FixtureDaemon` construction
    /// sites above stay untouched.
    struct FixtureDaemonWithClaim {
        outcome: DaemonValidationOutcome,
        claim: Option<anvil_kernel_types::protection_claim::ProtectionClaim>,
    }

    impl DaemonValidationClient for FixtureDaemonWithClaim {
        fn validate_pre_write(
            &self,
            _request: &PreWriteValidationRequest<'_>,
        ) -> DaemonValidationOutcome {
            self.outcome.clone()
        }

        fn query_protection_claim(
            &self,
            _workspace_root: &Path,
        ) -> Option<anvil_kernel_types::protection_claim::ProtectionClaim> {
            self.claim.clone()
        }
    }

    /// Pinned reference claim used by the MLP2-051b tests below.
    fn sample_protection_claim() -> anvil_kernel_types::protection_claim::ProtectionClaim {
        use anvil_kernel_types::protection_claim::{
            ProtectionClaim, SurfaceClaim, SurfaceClaimState, WorktreeClaimState,
        };
        ProtectionClaim::new(
            WorktreeClaimState::PreWriteDaemon,
            vec![SurfaceClaim {
                identifier: "mcp-shim-claude".to_string(),
                state: SurfaceClaimState::Participating,
            }],
        )
    }

    /// MLP2-051b: when the daemon serves the validation AND surfaces
    /// a protection claim, the `validate_write` response carries the
    /// closed-set claim shape under `protection_claim`.
    #[test]
    fn protection_claim_attached_to_response_when_daemon_supplies_one() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemonWithClaim {
            outcome: DaemonValidationOutcome::Diagnostics(vec![]),
            claim: Some(sample_protection_claim()),
        };
        let result = call_with_validation_client(
            &json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Block),
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["correlation"]["backend"], "daemon");
        assert_eq!(payload["correlation"]["daemonStatus"], "available");
        let claim = &payload["protection_claim"];
        assert_eq!(claim["schema_version"], "anvil.protection-claim.v1");
        assert_eq!(claim["worktree_state"], "pre-write-daemon");
        let surfaces = claim["surfaces"].as_array().expect("surfaces array");
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0]["identifier"], "mcp-shim-claude");
        assert_eq!(surfaces[0]["state"], "participating");
    }

    /// MLP2-051b: when the daemon is wired but cannot supply a claim
    /// (snapshot fetch failed / no worktree match), the field is
    /// omitted entirely rather than synthesised as `unprotected`. The
    /// no-over-claim posture lets drivers distinguish "daemon said
    /// nothing" from "daemon said unprotected".
    #[test]
    fn protection_claim_omitted_when_daemon_returns_none() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemonWithClaim {
            outcome: DaemonValidationOutcome::Diagnostics(vec![]),
            claim: None,
        };
        let result = call_with_validation_client(
            &json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Block),
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["correlation"]["daemonStatus"], "available");
        assert!(
            payload.get("protection_claim").is_none(),
            "claim must be omitted when daemon returns None, got: {payload}",
        );
    }

    /// MLP2-051b: the embedded-only path (no daemon was wired) never
    /// emits a `protection_claim`. The shim does not call
    /// `query_protection_claim` at all here — there is no daemon to
    /// answer — so the field stays absent.
    #[test]
    fn protection_claim_omitted_when_daemon_not_wired() {
        let workspace = tempdir().expect("workspace exists");
        // A claim is "available" on the fixture but the validation
        // outcome demotes to embedded; the shim must NOT attach the
        // claim because the daemon was never consulted for the
        // validation. Pinning this prevents a future regression that
        // calls `query_protection_claim` unconditionally and over-
        // claims daemon coverage on the embedded path.
        let daemon = FixtureDaemonWithClaim {
            outcome: DaemonValidationOutcome::Unavailable,
            claim: Some(sample_protection_claim()),
        };
        let result = call_with_validation_client(
            &json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Block),
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["correlation"]["backend"], "embedded");
        assert_eq!(payload["correlation"]["daemonStatus"], "not-wired");
        assert!(
            payload.get("protection_claim").is_none(),
            "embedded-path responses must not carry a daemon-derived claim, got: {payload}",
        );
    }

    /// MLP2-051b: a backend operational failure short-circuits the
    /// payload, so even a daemon-supplied claim is never attached.
    /// The failure response is a hard stop carrying `error` +
    /// `decision: block`; folding a claim onto it would muddle the
    /// "we could not validate" signal with daemon state.
    #[test]
    fn protection_claim_omitted_on_backend_failure() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemonWithClaim {
            outcome: DaemonValidationOutcome::OperationalFailure(ValidationBackendFailure {
                code: "validation-backend-unavailable",
                message: "Anvil could not validate the proposed write.",
                retriable: true,
            }),
            claim: Some(sample_protection_claim()),
        };
        let result = call_with_validation_client(
            &json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Block),
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "validation-backend-unavailable");
        assert!(
            payload.get("protection_claim").is_none(),
            "backend-failure responses must not carry a protection_claim, got: {payload}",
        );
    }

    /// MLP2-051b driver-compat pin: an MCP response that DOES carry
    /// `protection_claim` deserialises into a struct that pins the
    /// new shape (the additive field is `Option<ProtectionClaim>`),
    /// AND a response without the field deserialises into the same
    /// struct with the field set to `None`. Together these prove the
    /// "wire-additive" contract: pre-MLP2-051b drivers ignore the new
    /// field, post-MLP2-051b drivers read it when present, and an
    /// absent field is never a deserialise error.
    #[test]
    fn protection_claim_field_is_wire_additive_for_driver_clients() {
        use anvil_kernel_types::protection_claim::ProtectionClaim;
        use serde::Deserialize;

        /// Mirror of the subset of fields a driver-side parser cares
        /// about. The `protection_claim` field is `Option`-shaped per
        /// the MLP2-051b spec; `serde(default)` keeps absence
        /// indistinguishable from `null` and `skip_serializing_if`-
        /// omitted producers.
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct DriverViewWithClaim {
            decision: String,
            #[serde(default)]
            protection_claim: Option<ProtectionClaim>,
        }

        let workspace = tempdir().expect("workspace exists");
        let with_claim = parse_payload(&call_with_validation_client(
            &json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &FixtureDaemonWithClaim {
                outcome: DaemonValidationOutcome::Diagnostics(vec![]),
                claim: Some(sample_protection_claim()),
            },
            &FixedEnforcement(EnforcementMode::Block),
        ));
        let parsed_with: DriverViewWithClaim =
            serde_json::from_value(with_claim).expect("driver parses payload with claim");
        let claim = parsed_with
            .protection_claim
            .expect("driver sees Some(claim) when daemon supplies one");
        assert_eq!(
            claim.worktree_state,
            anvil_kernel_types::protection_claim::WorktreeClaimState::PreWriteDaemon,
        );

        let without_claim = parse_payload(&call_with_validation_client(
            &json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &FixtureDaemonWithClaim {
                outcome: DaemonValidationOutcome::Unavailable,
                claim: None,
            },
            &FixedEnforcement(EnforcementMode::Block),
        ));
        let parsed_without: DriverViewWithClaim =
            serde_json::from_value(without_claim).expect("driver parses payload without claim");
        assert!(
            parsed_without.protection_claim.is_none(),
            "absent protection_claim must parse as None, not error",
        );
    }
}
