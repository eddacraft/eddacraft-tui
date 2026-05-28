use std::fs;
use std::io::Read as _;
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
                    "description": "Unified diff. When supplied without proposedContent, the validator reads the on-disk file at workspaceRoot+path, applies the patch in memory, and validates the resulting post-image. The disk file is never written. When proposedContent is also supplied, the patch is correlation metadata only."
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
        Err(ParseError::Problem(problem)) => {
            // Input problems short-circuit before we have a trusted
            // workspace root to read `.anvil.yaml` from. They always
            // map to `block` regardless of enforcement mode — the tool
            // cannot evaluate a request it cannot parse.
            return tool_result(&problem_payload(problem, None, EnforcementMode::Block));
        }
        Err(ParseError::UntrustedWorkspaceRoot { expected }) => {
            // CIB-007: same `block` outcome as any other input
            // problem, plus a recoverable `expectedWorkspaceRoot`
            // field so the caller can retry with the right value.
            return tool_result(&untrusted_workspace_root_payload(
                &expected,
                EnforcementMode::Block,
            ));
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

    // CIB-005: when the caller sent a patch but no `proposedContent`,
    // read the on-disk file and apply the patch in memory to produce
    // the post-image, then feed it through the same pipeline as a
    // full-content payload. The validator never writes to disk.
    //
    // Race-window note: the read is point-in-time. If the file
    // changes between this validation call and the agent's
    // subsequent write, the patch validated here may differ from the
    // patch the agent actually applies. The agent is responsible for
    // re-reading its base before writing; the validator's contract
    // is "if you write the same patch against the same base, this is
    // the result we validated".
    let mut request = request;
    // Materialise only when content is missing, a patch was supplied,
    // AND the operation actually consumes content. Delete and rename
    // do not require post-image content, so a patch field on those
    // operations is correlation metadata only — reading the on-disk
    // file there would scan content that is about to disappear and
    // could block the operation on findings in soon-to-be-removed
    // bytes (Copilot review, 2026-05-18).
    if request.content.is_none()
        && request.patch_text.is_some()
        && request.operation.requires_content()
    {
        match materialise_patch_content(
            &request.workspace_root,
            &request.relative_path,
            request.patch_text.as_deref().expect("checked above"),
        ) {
            Ok(post_image) => {
                request.content = Some(post_image);
            }
            Err(problem) => {
                return tool_result(&problem_payload(
                    problem,
                    Some(&request.relative_path),
                    enforcement_mode,
                ));
            }
        }
        // Re-run the post-content input checks (size, NUL) now that
        // we have materialised content.
        if let Some(problem) = request.input_problem() {
            return tool_result(&problem_payload(
                problem,
                Some(&request.relative_path),
                enforcement_mode,
            ));
        }
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

/// CIB-007: dedicated payload for `untrusted-workspace-root` that
/// includes the shim's expected `workspaceRoot`. Mirrors
/// `problem_payload` for everything else, so the response envelope
/// stays consistent with the other input-problem paths.
fn untrusted_workspace_root_payload(expected: &Path, enforcement_mode: EnforcementMode) -> Value {
    let problem = ToolProblem::new(
        "untrusted-workspace-root",
        "workspaceRoot must match the MCP server workspace.",
    );
    let mut payload = problem_payload(problem, None, enforcement_mode);
    payload["error"]["expectedWorkspaceRoot"] = json!(expected.to_string_lossy());
    payload
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
    let normalised: Vec<Diagnostic> = diagnostics
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
        .collect();

    // MLP2-073 / #1799 — dedupe BEFORE the caller builds `summary` so a
    // single planted finding never reports `summary.total: 2`. Dedupe
    // is keyed primarily on `id` (the canonical-identity field that
    // every producer is expected to make unique per finding); the
    // defensive secondary key `(rule_id, file, line, column)` catches
    // producers that share rule_id and location but accidentally
    // assign distinct ids. Order is preserved — the first occurrence
    // wins so the wire ordering stays deterministic.
    dedupe_diagnostics(normalised)
}

fn dedupe_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    // Key shapes (both must be NEW for a diagnostic to survive):
    //   * `(id, file, line, column)` — primary; the audit case at
    //     #1799 sees this whole tuple identical across the duplicated
    //     emissions. Keying on `(id, location)` rather than `id` alone
    //     avoids suppressing two *distinct* findings that share a
    //     hashed id after secret redaction
    //     (`diag_mcp_secret_redacted_<hex6>` prefix collisions are
    //     rare at 2^48 buckets but real for a security surface —
    //     losing a finding is worse than rendering the same hash twice
    //     for distinct locations).
    //   * `(rule_id, file, line, column)` — defensive secondary;
    //     catches producers that share rule+location but accidentally
    //     drift their `id` per scan invocation (e.g. a UUID suffix).
    //     For whole-file rules `line: None` matches every other
    //     `line: None` entry from the same `(rule_id, file)` — that
    //     is intentional: a single missing-license-header rule firing
    //     on a file is one logical finding regardless of how many
    //     times the producer emits it.
    let mut seen_id_loc =
        std::collections::HashSet::<(String, String, Option<u32>, Option<u32>)>::with_capacity(
            diagnostics.len(),
        );
    let mut seen_rule_loc =
        std::collections::HashSet::<(String, String, Option<u32>, Option<u32>)>::with_capacity(
            diagnostics.len(),
        );
    let mut out = Vec::with_capacity(diagnostics.len());
    for diagnostic in diagnostics {
        let id_loc = (
            diagnostic.id.clone(),
            diagnostic.location.file.clone(),
            diagnostic.location.line,
            diagnostic.location.column,
        );
        if !seen_id_loc.insert(id_loc) {
            continue;
        }
        let rule_loc = (
            diagnostic.source.rule_id.clone(),
            diagnostic.location.file.clone(),
            diagnostic.location.line,
            diagnostic.location.column,
        );
        if !seen_rule_loc.insert(rule_loc) {
            continue;
        }
        out.push(diagnostic);
    }
    out
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

/// CIB-007: the `untrusted-workspace-root` case carries a dynamic
/// `expectedWorkspaceRoot` in its error payload so the caller can
/// retry with the right value on the next call. `ToolProblem` is
/// `Copy` with `&'static` strings to keep the common path cheap, so
/// the dynamic case is modelled as a separate enum variant rather
/// than bloating every `ToolProblem` site with an owned-string
/// detail map.
#[derive(Debug)]
enum ParseError {
    Problem(ToolProblem),
    UntrustedWorkspaceRoot { expected: PathBuf },
}

impl From<ToolProblem> for ParseError {
    fn from(problem: ToolProblem) -> Self {
        Self::Problem(problem)
    }
}

struct ValidateWriteRequest {
    workspace_root: PathBuf,
    relative_path: String,
    operation: Operation,
    content: Option<String>,
    /// CIB-005: raw patch text retained so we can materialise the
    /// post-image from the on-disk file when no `proposedContent` was
    /// supplied. `None` when the caller sent full content or a
    /// preview.
    patch_text: Option<String>,
    partial_scan: bool,
    preflight_problem: Option<ToolProblem>,
}

impl ValidateWriteRequest {
    fn parse(arguments: &Value, default_workspace_root: &Path) -> Result<Self, ParseError> {
        let Some(arguments) = arguments.as_object() else {
            return Err(ToolProblem::new(
                "invalid-tool-arguments",
                "Validate-write arguments must be an object.",
            )
            .into());
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
        // Precedence (Copilot review, 2026-05-18):
        // 1. `proposedContent` is authoritative when present (full post-image).
        // 2. Otherwise `patch` is authoritative — the post-image is
        //    materialised from on-disk content and matches what the agent
        //    will actually write. `preview` is ignored in this branch so
        //    we do not run partial validation on a stale slice while the
        //    actual change comes from the patch.
        // 3. Otherwise `preview` triggers partial validation on the slim
        //    payload.
        // Patch text is never scanned as file content directly because
        // diff hunks include removed lines and metadata that would mislead
        // the secret/reasoning checks; CIB-005 materialises the post-image
        // from on-disk content before scanning.
        let (content, partial_scan) = match (proposed_content, patch_content, preview_content) {
            (Some(full), _, _) => (Some(full.to_string()), false),
            (None, None, Some(preview)) => (Some(preview.to_string()), true),
            // (None, Some(_), _) — patch authoritative, defer to
            //   materialisation; preview is ignored.
            // (None, None, None) — no content path.
            _ => (None, false),
        };
        let patch_text = patch_content.map(str::to_string);

        Ok(Self {
            workspace_root,
            relative_path,
            operation,
            content,
            patch_text,
            partial_scan,
            preflight_problem,
        })
    }

    fn input_problem(&self) -> Option<ToolProblem> {
        if let Some(problem) = self.preflight_problem {
            return Some(problem);
        }

        // CIB-005: patch-only is a valid input shape — content is
        // materialised from disk later, in `materialise_patch_content`.
        // Only flag "missing-content" when there is neither content
        // nor a patch to derive it from.
        if self.content.is_none() && self.patch_text.is_none() && self.operation.requires_content()
        {
            return Some(ToolProblem::new(
                "missing-content",
                "Validate-write requires proposedContent or patch for this operation.",
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
) -> Result<PathBuf, ParseError> {
    let default_workspace_root = canonical_workspace_root(default_workspace_root)?;

    match value {
        Some(Value::String(root)) => {
            let root = PathBuf::from(root);
            if !root.is_absolute() {
                return Err(ToolProblem::new(
                    "invalid-workspace-root",
                    "workspaceRoot must be an absolute path.",
                )
                .into());
            }
            let root = canonical_workspace_root(&root)?;
            if root == default_workspace_root {
                Ok(root)
            } else {
                // CIB-007: surface the shim's expected workspace root
                // so the caller can self-correct on the next call.
                // Trust boundary is unchanged — the mismatch still
                // blocks the write.
                Err(ParseError::UntrustedWorkspaceRoot {
                    expected: default_workspace_root,
                })
            }
        }
        Some(_) => Err(ToolProblem::new(
            "invalid-workspace-root",
            "workspaceRoot must be a string when provided.",
        )
        .into()),
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

/// CIB-005: materialise the post-image of a patch against the
/// on-disk file. Returns the post-image content if the patch applies
/// cleanly; otherwise a structured `ToolProblem` whose code
/// distinguishes "file missing/unreadable" from "patch context did
/// not match disk". The on-disk file is never written.
fn materialise_patch_content(
    workspace_root: &Path,
    relative_path: &str,
    patch_text: &str,
) -> Result<String, ToolProblem> {
    let absolute = workspace_root.join(relative_path);

    // Cap the on-disk read at `MAX_PROPOSED_CONTENT_BYTES` (the same
    // 1 MiB ceiling that gates `proposedContent`) before we touch the
    // file. Without this, an attacker or malfunctioning agent could
    // point patch-mode at a multi-GiB blob or a `/proc`-style
    // pseudo-file and OOM or hang the shim. The metadata check is
    // also our first cheap filter — pseudo-files and special devices
    // typically report `len() == 0`, so we still rely on the
    // post-read size check below to catch streaming pseudo-files.
    //
    // Residual TOCTOU note: a symlink or file swap between this
    // metadata stat and the read below would not bypass the
    // workspace-escape gate (`reject_symlink_escape` ran at parse
    // time, anchored at the first existing ancestor), but it could
    // race a different file's content into the read. The response
    // never echoes raw file content (only structural error codes
    // and redacted diagnostics), so this is not an exfiltration
    // path. Hardening to O_NOFOLLOW-style reads is tracked for a
    // follow-up if the current contract proves insufficient.
    let metadata = fs::metadata(&absolute).map_err(|_| {
        ToolProblem::new(
            "patch-target-unreadable",
            "Validate-write could not read the patch target file at workspaceRoot+path.",
        )
    })?;
    if !metadata.is_file() || metadata.len() > MAX_PROPOSED_CONTENT_BYTES as u64 {
        return Err(ToolProblem::new(
            "patch-target-unreadable",
            "Validate-write could not read the patch target file at workspaceRoot+path.",
        ));
    }

    // Capped read (Copilot review, 2026-05-18): `fs::read_to_string`
    // pre-allocates and reads to EOF, so a pseudo-file that reports a
    // small `len()` and then streams more data, or a real file that
    // grew between the stat and the read, would still trigger an
    // unbounded allocation before any post-read length check. Use
    // `Read::take` with a one-byte tolerance over the ceiling so an
    // attempt to exceed the cap surfaces as `patch-target-unreadable`
    // mid-stream rather than after `MAX_PROPOSED_CONTENT_BYTES` are
    // already in memory.
    let file = fs::File::open(&absolute).map_err(|_| {
        ToolProblem::new(
            "patch-target-unreadable",
            "Validate-write could not read the patch target file at workspaceRoot+path.",
        )
    })?;
    let mut original = String::new();
    file.take(MAX_PROPOSED_CONTENT_BYTES as u64 + 1)
        .read_to_string(&mut original)
        .map_err(|_| {
            ToolProblem::new(
                "patch-target-unreadable",
                "Validate-write could not read the patch target file at workspaceRoot+path.",
            )
        })?;
    if original.len() > MAX_PROPOSED_CONTENT_BYTES {
        return Err(ToolProblem::new(
            "patch-target-unreadable",
            "Validate-write could not read the patch target file at workspaceRoot+path.",
        ));
    }

    apply_unified_diff(&original, patch_text).map_err(|_| {
        ToolProblem::new(
            "patch-apply-failed",
            "Validate-write could not apply the supplied patch to the current on-disk content (context mismatch or unsupported diff shape).",
        )
    })
}

/// Minimal unified-diff applier covering the shapes agents commonly
/// produce: optional `--- a/...` / `+++ b/...` headers, hunk headers
/// of the form `@@ -<old_start>[,<old_count>] +<new_start>[,<new_count>] @@`,
/// and body lines prefixed with ` ` (context), `-` (removed), or `+`
/// (added). The `\ No newline at end of file` marker is recognised
/// and adjusts the trailing-newline policy of the corresponding
/// side. Anything else returns `Err`; the caller maps that to a
/// `patch-apply-failed` block so the agent gets a clear signal
/// rather than a silent partial validation.
fn apply_unified_diff(original: &str, patch: &str) -> Result<String, ApplyError> {
    let mut original_lines: Vec<&str> = original.split('\n').collect();
    let original_trailing_newline = original.ends_with('\n');
    if original_trailing_newline {
        // The split produces a trailing empty element for the
        // newline-terminated case; drop it so `lines` holds one
        // entry per logical source line.
        original_lines.pop();
    }

    let mut output_lines: Vec<String> = Vec::new();
    let mut new_trailing_newline = original_trailing_newline;
    let mut cursor: usize = 0;

    let mut iter = patch.split('\n').peekable();
    while let Some(raw_line) = iter.next() {
        if raw_line.starts_with("---") || raw_line.starts_with("+++") {
            continue;
        }
        if raw_line.starts_with("diff --git") || raw_line.starts_with("index ") {
            continue;
        }
        if let Some(rest) = raw_line.strip_prefix("@@") {
            let (old_start, old_count, new_count) = parse_hunk_header(rest)?;
            // Unified-diff convention: `-N,M` with M > 0 means "starting
            // at 1-indexed line N, take M lines" → 0-indexed start
            // N-1. `-N,0` (pure insertion) means "insert AFTER 1-indexed
            // line N" → 0-indexed insertion point is N. The two cases
            // need different cursor placement; collapsing them into a
            // single `saturating_sub(1)` would insert at the wrong
            // position for pure-insertion hunks (Copilot review,
            // 2026-05-18).
            let old_start_idx = if old_count == 0 {
                old_start
            } else {
                old_start.saturating_sub(1)
            };
            if old_start_idx > original_lines.len() || cursor > old_start_idx {
                return Err(ApplyError::HunkOutOfRange);
            }
            while cursor < old_start_idx {
                output_lines.push(original_lines[cursor].to_string());
                cursor += 1;
            }

            let mut old_consumed: usize = 0;
            let mut new_consumed: usize = 0;
            let mut last_side_was_addition = false;
            while old_consumed < old_count || new_consumed < new_count {
                let body = iter.next().ok_or(ApplyError::TruncatedHunk)?;
                if let Some(marker) = body.strip_prefix('\\') {
                    // `\ No newline at end of file`. Affects the side
                    // owned by the previous body line.
                    if !marker.trim_start().starts_with("No newline") {
                        return Err(ApplyError::UnsupportedMarker);
                    }
                    if last_side_was_addition {
                        new_trailing_newline = false;
                    } else {
                        // Old/context-side marker asserts the original
                        // lacked a trailing newline. The patch may
                        // restore one on the new side (Case C, Copilot
                        // review 2026-05-18): if a subsequent `+` line
                        // in this hunk has no following new-side
                        // marker, the new side gets a trailing newline.
                        // Default to upgrading; a new-side marker below
                        // will downgrade if present.
                        new_trailing_newline = true;
                    }
                    continue;
                }
                let (prefix, content) =
                    body.split_at(body.chars().next().map_or(0, char::len_utf8));
                match prefix {
                    " " => {
                        if cursor >= original_lines.len() || original_lines[cursor] != content {
                            return Err(ApplyError::ContextMismatch);
                        }
                        output_lines.push(content.to_string());
                        cursor += 1;
                        old_consumed += 1;
                        new_consumed += 1;
                        last_side_was_addition = false;
                    }
                    "-" => {
                        if cursor >= original_lines.len() || original_lines[cursor] != content {
                            return Err(ApplyError::ContextMismatch);
                        }
                        cursor += 1;
                        old_consumed += 1;
                        last_side_was_addition = false;
                    }
                    "+" => {
                        output_lines.push(content.to_string());
                        new_consumed += 1;
                        last_side_was_addition = true;
                    }
                    _ => return Err(ApplyError::InvalidLinePrefix),
                }
            }
            // A trailing-newline marker for the new side can follow
            // the final `+` line of a hunk. Peek without consuming
            // any non-marker line that belongs to the next hunk.
            if let Some(next) = iter.peek()
                && let Some(marker) = next.strip_prefix('\\')
                && marker.trim_start().starts_with("No newline")
            {
                iter.next();
                if last_side_was_addition {
                    new_trailing_newline = false;
                }
            }
        } else if !raw_line.is_empty() {
            return Err(ApplyError::UnexpectedContent);
        }
        // Blank lines between hunks fall through to the next loop
        // iteration; some patch producers emit them as visual
        // padding.
    }

    while cursor < original_lines.len() {
        output_lines.push(original_lines[cursor].to_string());
        cursor += 1;
    }

    let mut result = output_lines.join("\n");
    if new_trailing_newline && !result.is_empty() {
        result.push('\n');
    }
    Ok(result)
}

#[derive(Debug)]
enum ApplyError {
    ContextMismatch,
    HunkOutOfRange,
    InvalidLinePrefix,
    InvalidHunkHeader,
    TruncatedHunk,
    UnexpectedContent,
    UnsupportedMarker,
}

fn parse_hunk_header(rest: &str) -> Result<(usize, usize, usize), ApplyError> {
    // Expected shape: ` -<old_start>[,<old_count>] +<new_start>[,<new_count>] @@[ optional section header]`.
    // Both side counts are needed so we know when the hunk body is
    // exhausted (added-only or removed-only hunks would never satisfy
    // a single-side terminator).
    let mut tokens = rest.split_whitespace();
    let old_tok = tokens
        .find(|t| t.starts_with('-'))
        .ok_or(ApplyError::InvalidHunkHeader)?;
    let new_tok = tokens
        .find(|t| t.starts_with('+'))
        .ok_or(ApplyError::InvalidHunkHeader)?;
    let (old_start, old_count) = parse_range(old_tok.strip_prefix('-').unwrap_or(""))?;
    let (_new_start, new_count) = parse_range(new_tok.strip_prefix('+').unwrap_or(""))?;
    Ok((old_start, old_count, new_count))
}

fn parse_range(body: &str) -> Result<(usize, usize), ApplyError> {
    let (start_str, count_str) = match body.split_once(',') {
        Some((s, c)) => (s, c),
        None => (body, "1"),
    };
    let start: usize = start_str
        .parse()
        .map_err(|_| ApplyError::InvalidHunkHeader)?;
    let count: usize = count_str
        .parse()
        .map_err(|_| ApplyError::InvalidHunkHeader)?;
    Ok((start, count))
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
        diagnostic_summary, normalise_response_diagnostics, redact_secret_id,
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
        // MLP2-051b: `FixtureDaemon` does not override
        // `query_protection_claim` (the trait default returns
        // `None`). Pin the absence here so any future regression
        // that conjures a claim out of an unaware fixture is
        // caught at the base test rather than only at the
        // dedicated `FixtureDaemonWithClaim` tests below.
        assert!(
            payload.get("protection_claim").is_none(),
            "FixtureDaemon returns None; claim must not appear, got: {payload}",
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
        // MLP2-051b: the second IPC round-trip ran against the live
        // listener's default `NoopStatusProvider`, which returns an
        // empty snapshot. The workspace path is not in `worktrees`,
        // so `build_protection_claim_from_wire` yields the
        // `unprotected` state with no surfaces. The point is to
        // prove the field is *present* on the daemon-served
        // response — the dedicated `mcp_protection_claim.rs`
        // integration test exercises richer state combinations.
        let claim = &daemon["protection_claim"];
        assert!(
            !claim.is_null(),
            "daemon-served response must carry protection_claim, got: {daemon}",
        );
        assert_eq!(claim["worktree_state"], "unprotected");
        assert!(
            claim["surfaces"]
                .as_array()
                .expect("surfaces array")
                .is_empty(),
        );
        // Embedded path must NOT carry the claim — pin the negative
        // here too so a future regression that calls the daemon
        // unconditionally is caught at the same surface.
        assert!(
            embedded.get("protection_claim").is_none(),
            "embedded response must not carry protection_claim, got: {embedded}",
        );
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

    /// CIB-005: a patch-only payload against an existing file is
    /// applied to the on-disk content in memory, and the resulting
    /// post-image goes through the full pre-write pipeline. The disk
    /// file is never written by this tool.
    #[test]
    fn patch_only_validates_after_applying_to_on_disk_file() {
        let workspace = tempdir().expect("workspace exists");
        let target = workspace.path().join("src/example.ts");
        fs::create_dir_all(target.parent().expect("parent dir")).expect("src dir created");
        fs::write(&target, "export const value = 1;\n").expect("seed target");

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "patch": "--- a/src/example.ts\n+++ b/src/example.ts\n@@ -1 +1 @@\n-export const value = 1;\n+export const value = 2;\n"
            }),
        );

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["summary"]["total"], 0);
        // Validator is read-only — on-disk file must be untouched.
        assert_eq!(
            fs::read_to_string(&target).expect("target readable"),
            "export const value = 1;\n",
            "validate_write must never write to disk",
        );
    }

    /// CIB-005: post-image of a patch is fed through the same secret
    /// detection pipeline as a full `proposedContent` payload. A patch
    /// that introduces a secret blocks the write.
    #[test]
    fn patch_only_post_image_secret_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let target = workspace.path().join("src/secret.ts");
        fs::create_dir_all(target.parent().expect("parent dir")).expect("src dir created");
        fs::write(&target, "const placeholder = 'redacted';\n").expect("seed target");

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/secret.ts",
                "operation": "update",
                "patch": "--- a/src/secret.ts\n+++ b/src/secret.ts\n@@ -1 +1 @@\n-const placeholder = 'redacted';\n+const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["summary"]["bySeverity"]["error"], 1);
        assert_eq!(payload["diagnostics"][0]["category"], "secret");
    }

    /// CIB-005 acceptance: a one-string rename inside a large file
    /// (the 2026-05-18 beta tester case) succeeds without a full
    /// `proposedContent` payload.
    #[test]
    fn patch_only_one_string_rename_in_large_file_validates_cleanly() {
        let workspace = tempdir().expect("workspace exists");
        let target = workspace.path().join("meta/tags.json");
        fs::create_dir_all(target.parent().expect("parent dir")).expect("meta dir created");

        // Synthesise a large JSON-shaped fixture so the one-line rename
        // case mirrors the original screenshot scale (~2700 lines) but
        // keeps the test deterministic and fast. Final two body lines
        // are the renamed entry and the closing bracket.
        let mut body = String::from("[\n");
        for idx in 0..1500 {
            std::fmt::Write::write_fmt(
                &mut body,
                format_args!("  {{\"id\": {idx}, \"tag\": \"tag-{idx}\"}},\n"),
            )
            .expect("string write");
        }
        body.push_str("  {\"id\": 1500, \"tag\": \"old-name\"}\n]\n");
        fs::write(&target, &body).expect("seed large fixture");

        // Build the patch precisely to avoid string-literal
        // line-continuation whitespace surprises.
        // File layout (1-indexed):
        //   line 1     "["
        //   lines 2..  body entries for idx 0..1499 (line 1501 = idx 1499)
        //   line 1502  "  {\"id\": 1500, \"tag\": \"old-name\"}"
        //   line 1503  "]"
        // The hunk renames line 1502 with the surrounding two lines
        // as context.
        let patch = String::from("--- a/meta/tags.json\n")
            + "+++ b/meta/tags.json\n"
            + "@@ -1501,3 +1501,3 @@\n"
            + "   {\"id\": 1499, \"tag\": \"tag-1499\"},\n"
            + "-  {\"id\": 1500, \"tag\": \"old-name\"}\n"
            + "+  {\"id\": 1500, \"tag\": \"new-name\"}\n"
            + " ]\n";

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "meta/tags.json",
                "operation": "update",
                "patch": patch
            }),
        );

        assert_eq!(
            payload["decision"], "allow",
            "expected allow, got payload: {payload}"
        );
        assert_eq!(payload["summary"]["total"], 0);
    }

    /// CIB-005 / adversarial review: a pure-insertion hunk of the
    /// form `@@ -N,0 +N,M @@` (no removed lines, only additions) is
    /// the standard unified-diff shape models emit for append-only
    /// or insert-before edits. Must apply cleanly AND insert at the
    /// correct position (after line N, per the unified-diff
    /// convention) so the validator scans the same post-image the
    /// caller would write.
    #[test]
    fn patch_only_pure_insertion_hunk_applies_at_correct_position() {
        // Direct unit-test of the patch applier so we can assert the
        // post-image bytes, not just the validator decision.
        let original = "line a\nline b\nline c\n";
        let patch = "--- a/x\n+++ b/x\n@@ -2,0 +3,2 @@\n+inserted-1\n+inserted-2\n";
        let post_image =
            super::apply_unified_diff(original, patch).expect("pure-insertion patch applies");
        assert_eq!(
            post_image, "line a\nline b\ninserted-1\ninserted-2\nline c\n",
            "pure-insertion `-N,0` inserts AFTER line N",
        );

        // End-to-end smoke through the validator: pure-insertion hunk
        // must produce decision=allow and not block as
        // patch-apply-failed.
        let workspace = tempdir().expect("workspace exists");
        let target = workspace.path().join("src/example.ts");
        fs::create_dir_all(target.parent().expect("parent dir")).expect("src dir created");
        fs::write(&target, "line a\nline b\n").expect("seed target");

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "patch": "--- a/src/example.ts\n+++ b/src/example.ts\n@@ -2,0 +3,2 @@\n+inserted-1\n+inserted-2\n"
            }),
        );

        assert_eq!(
            payload["decision"], "allow",
            "pure-insertion hunks must apply cleanly, got: {payload}"
        );
        assert_eq!(payload["summary"]["total"], 0);
    }

    /// CIB-005 / Copilot review: a patch that adds a trailing newline
    /// to a file that previously lacked one (Case C: `\ No newline`
    /// marker on the removed side only) must materialise a post-image
    /// that ends with `\n`. The original validator inherited the
    /// original's trailing-newline state and could not upgrade.
    #[test]
    fn patch_can_add_trailing_newline_to_unterminated_file() {
        let original = "alpha\nbeta";
        let patch = String::from("--- a/x\n+++ b/x\n@@ -2 +2 @@\n")
            + "-beta\n"
            + "\\ No newline at end of file\n"
            + "+beta-prime\n";
        let post_image = super::apply_unified_diff(original, &patch)
            .expect("trailing-newline-add patch applies");
        assert_eq!(
            post_image, "alpha\nbeta-prime\n",
            "patch with old-side `\\ No newline` and unmarked new side adds a trailing newline",
        );
    }

    /// CIB-005 / Copilot review: a patch that strips the trailing
    /// newline from a previously-terminated file (Case D: marker on
    /// new side) materialises a post-image without `\n`.
    #[test]
    fn patch_can_strip_trailing_newline_from_terminated_file() {
        let original = "alpha\nbeta\n";
        let patch = String::from("--- a/x\n+++ b/x\n@@ -2 +2 @@\n")
            + "-beta\n"
            + "+beta-prime\n"
            + "\\ No newline at end of file\n";
        let post_image =
            super::apply_unified_diff(original, &patch).expect("trailing-newline-strip applies");
        assert_eq!(
            post_image, "alpha\nbeta-prime",
            "patch with new-side `\\ No newline` strips the trailing newline",
        );
    }

    /// CIB-005 / Copilot review: when `patch` is supplied alongside
    /// `preview` (without `proposedContent`), the patch is
    /// authoritative — preview is not used for partial validation,
    /// because doing so would scan a stale slice while the post-image
    /// the caller actually writes comes from the patch.
    #[test]
    fn patch_with_preview_uses_patch_post_image_not_preview_slice() {
        let workspace = tempdir().expect("workspace exists");
        let target = workspace.path().join("src/example.ts");
        fs::create_dir_all(target.parent().expect("parent dir")).expect("src dir created");
        fs::write(&target, "const placeholder = 'redacted';\n").expect("seed target");

        // Preview is clean. Patch inserts a real secret into the
        // post-image. The validator must scan the post-image (block)
        // rather than the preview (allow).
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "preview": "const placeholder = 'redacted';\n",
                "patch": "--- a/src/example.ts\n+++ b/src/example.ts\n@@ -1 +1 @@\n-const placeholder = 'redacted';\n+const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(
            payload["decision"], "block",
            "preview+patch must scan the patch post-image, not the preview slice; got: {payload}"
        );
        assert_eq!(payload["summary"]["bySeverity"]["error"], 1);
        // The presence of `patch` should not flip partial-scan mode on.
        assert!(
            payload["correlation"].get("partialScan").is_none(),
            "patch-mode validation is full, not partial",
        );
    }

    /// CIB-005 / Copilot review: `delete` and `rename` operations do
    /// not require post-image content. A `patch` field on those
    /// operations must be treated as correlation metadata only —
    /// materialisation must not read the about-to-be-removed file
    /// and produce findings on its contents.
    #[test]
    fn patch_on_delete_operation_does_not_materialise_or_scan() {
        let workspace = tempdir().expect("workspace exists");
        let target = workspace.path().join("src/old.ts");
        fs::create_dir_all(target.parent().expect("parent dir")).expect("src dir created");
        // Seed the file with content that would normally trigger a
        // secret block. If materialisation ran, this would surface as
        // an error in the response.
        fs::write(
            &target,
            "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n",
        )
        .expect("seed target");

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/old.ts",
                "operation": "delete",
                "patch": "--- a/src/old.ts\n+++ /dev/null\n@@ -1 +0,0 @@\n-const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(
            payload["decision"], "allow",
            "delete with patch metadata must not scan the file being removed; got: {payload}"
        );
        assert_eq!(payload["summary"]["total"], 0);
    }

    /// CIB-005 / adversarial review: a pure-deletion hunk of the
    /// form `@@ -N,M +N,0 @@` (lines removed, no additions). Exercises
    /// the `new_count = 0` branch of the body-loop guard.
    #[test]
    fn patch_only_pure_deletion_hunk_applies_cleanly() {
        let workspace = tempdir().expect("workspace exists");
        let target = workspace.path().join("src/example.ts");
        fs::create_dir_all(target.parent().expect("parent dir")).expect("src dir created");
        fs::write(&target, "keep me\ndelete me\nkeep me too\n").expect("seed target");

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "patch": "--- a/src/example.ts\n+++ b/src/example.ts\n@@ -2,1 +1,0 @@\n-delete me\n"
            }),
        );

        assert_eq!(
            payload["decision"], "allow",
            "pure-deletion hunks must apply cleanly, got: {payload}"
        );
        assert_eq!(payload["summary"]["total"], 0);
    }

    /// CIB-005 / adversarial review: an on-disk file larger than
    /// `MAX_PROPOSED_CONTENT_BYTES` in patch mode must be rejected
    /// before the read can OOM the process. The same 1 MiB ceiling
    /// that applies to `proposedContent` applies to the patch-mode
    /// read target.
    #[test]
    fn patch_only_oversize_target_blocks_before_read() {
        let workspace = tempdir().expect("workspace exists");
        let target = workspace.path().join("src/huge.ts");
        fs::create_dir_all(target.parent().expect("parent dir")).expect("src dir created");
        let oversize = "x".repeat(MAX_PROPOSED_CONTENT_BYTES + 1);
        fs::write(&target, &oversize).expect("seed oversize target");

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/huge.ts",
                "operation": "update",
                "patch": "--- a/src/huge.ts\n+++ b/src/huge.ts\n@@ -1 +1 @@\n-old\n+new\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(
            payload["error"]["code"], "patch-target-unreadable",
            "oversize target must be rejected before the read, got: {payload}"
        );
    }

    /// CIB-005: a patch-only payload whose context does not match the
    /// current on-disk file is rejected with a clear, structured
    /// error rather than a silent wrong-content validation.
    #[test]
    fn patch_only_context_mismatch_blocks_with_clear_code() {
        let workspace = tempdir().expect("workspace exists");
        let target = workspace.path().join("src/example.ts");
        fs::create_dir_all(target.parent().expect("parent dir")).expect("src dir created");
        fs::write(&target, "export const value = 1;\n").expect("seed target");

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "patch": "--- a/src/example.ts\n+++ b/src/example.ts\n@@ -1 +1 @@\n-export const VALUE = 1;\n+export const VALUE = 2;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "patch-apply-failed");
    }

    /// CIB-005: a patch-only payload that targets a non-existent file
    /// is rejected with a clear, structured error so the caller can
    /// distinguish "patch is wrong" from "file is missing".
    #[test]
    fn patch_only_missing_target_blocks_with_clear_code() {
        let workspace = tempdir().expect("workspace exists");

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/nope.ts",
                "operation": "update",
                "patch": "--- a/src/nope.ts\n+++ b/src/nope.ts\n@@ -1 +1 @@\n-old\n+new\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "patch-target-unreadable");
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

        // CIB-007: the error payload must include the expected
        // `workspaceRoot` so callers can self-correct on the next call
        // without operator intervention. The value is the shim's
        // canonicalised cwd.
        let expected = fs::canonicalize(workspace.path()).expect("workspace canonicalises");
        assert_eq!(
            payload["error"]["expectedWorkspaceRoot"],
            json!(expected.to_string_lossy()),
            "untrusted-workspace-root must carry expectedWorkspaceRoot",
        );
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
        // `set_current_dir` is process-global. The workspace-wide cwd
        // guard (CIB-026) serialises this test against every other
        // cwd-mutating test in the crate and restores the original cwd on
        // exit — even though this test deliberately deletes the dir it
        // `cd`s into, the guard captured the original beforehand.
        let scratch = tempdir().expect("scratch workspace exists");
        let scratch_path = scratch.path().to_path_buf();

        let payload = crate::test_support::cwd::with_cwd_in(&scratch_path, || {
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
            payload
        });

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

    // ── MLP2-073 / #1799: pre-write summary dedupe ────────────────

    fn fixture_diag(id: &str, rule_id: &str, file: &str, line: Option<u32>) -> Diagnostic {
        Diagnostic::new(
            id.to_string(),
            Severity::Error,
            "fixture".to_string(),
            Location {
                file: file.to_string(),
                line,
                column: None,
                end_line: None,
                end_column: None,
            },
            Category::Secret,
            DiagnosticSource {
                rule_id: rule_id.to_string(),
                source_module: "anvil-cli::test".to_string(),
            },
            Mode::Unknown("pre-write".to_string()),
        )
    }

    #[test]
    fn normalise_response_dedupes_identical_id() {
        // The audit (#1799) planted one secret on src/smelly.ts:1 and
        // got back `summary.total: 2` with two diagnostics sharing the
        // same id, location, and summary. The wire shape must dedupe.
        let dup_a = fixture_diag(
            "diag_secret_pre_write_src_smelly_ts_1_high_entropy_string",
            "secret-detection-high-entropy",
            "src/smelly.ts",
            Some(1),
        );
        let dup_b = dup_a.clone();
        let normalised = normalise_response_diagnostics(
            &[dup_a, dup_b],
            crate::mcp::validation::ValidationBackend::Embedded,
        );
        assert_eq!(
            normalised.len(),
            1,
            "two diagnostics with the same id must dedupe; got: {normalised:?}"
        );

        let summary = diagnostic_summary(&normalised);
        assert_eq!(
            summary["total"], 1,
            "summary.total must reflect deduped count (the MLP2-073 contract)"
        );
        assert_eq!(summary["bySeverity"]["error"], 1);
    }

    #[test]
    fn normalise_response_dedupes_distinct_ids_with_same_rule_and_location() {
        // Defensive secondary key: producers that share rule_id and
        // location but accidentally assign distinct ids (e.g. a UUID
        // suffix per scan invocation) must still dedupe so a single
        // logical finding never doubles the count.
        let a = fixture_diag(
            "diag_run_001_secret",
            "secret-detection-high-entropy",
            "src/smelly.ts",
            Some(1),
        );
        let b = fixture_diag(
            "diag_run_002_secret",
            "secret-detection-high-entropy",
            "src/smelly.ts",
            Some(1),
        );
        let normalised = normalise_response_diagnostics(
            &[a, b],
            crate::mcp::validation::ValidationBackend::Embedded,
        );
        assert_eq!(
            normalised.len(),
            1,
            "same (rule_id, location) must dedupe even when ids differ"
        );
    }

    #[test]
    fn normalise_response_keeps_distinct_diagnostics() {
        // Two truly-distinct findings (different rule_id OR different
        // location) must NOT collapse — the dedupe is for accidental
        // duplication, not for compressing legitimate signal.
        let a = fixture_diag(
            "diag_a",
            "secret-detection-high-entropy",
            "src/a.ts",
            Some(1),
        );
        let b = fixture_diag("diag_b", "antipattern-AP-008", "src/a.ts", Some(1));
        let c = fixture_diag(
            "diag_c",
            "secret-detection-high-entropy",
            "src/a.ts",
            Some(7),
        );
        let normalised = normalise_response_diagnostics(
            &[a, b, c],
            crate::mcp::validation::ValidationBackend::Embedded,
        );
        assert_eq!(
            normalised.len(),
            3,
            "distinct (rule_id, location) tuples must survive dedupe"
        );
    }

    #[test]
    fn dedupe_does_not_suppress_distinct_location_when_ids_collide() {
        // Council follow-up on MLP2-073: secret-redaction shrinks the
        // id namespace to a 6-byte hash prefix (`diag_mcp_secret_redacted_<hex6>`).
        // Collisions are rare (2^48 buckets) but real. Two genuinely
        // distinct findings — different files / lines — that hash to
        // the same redacted id must NOT collapse: losing a finding on
        // a security surface is worse than rendering the same hash
        // twice for distinct locations. Keying on (id, location)
        // rather than id alone is what makes this safe.
        let a = fixture_diag(
            "diag_mcp_secret_redacted_aabbcc",
            "secret-detection-high-entropy",
            "src/leak_a.ts",
            Some(1),
        );
        let b = fixture_diag(
            "diag_mcp_secret_redacted_aabbcc", // collision: same hash prefix
            "secret-detection-high-entropy",
            "src/leak_b.ts", // but different file
            Some(7),
        );
        let normalised = normalise_response_diagnostics(
            &[a, b],
            crate::mcp::validation::ValidationBackend::Embedded,
        );
        assert_eq!(
            normalised.len(),
            2,
            "id collision at distinct locations must NOT collapse — both findings survive"
        );
    }

    #[test]
    fn dedupe_preserves_first_occurrence_order() {
        // Order on the wire matters for deterministic snapshots and
        // for consumers that surface diagnostics top-down. The first
        // occurrence of each unique id must win.
        let a1 = fixture_diag("diag_a", "rule-a", "src/x.ts", Some(1));
        let a2 = fixture_diag("diag_a", "rule-a", "src/x.ts", Some(1));
        let b = fixture_diag("diag_b", "rule-b", "src/x.ts", Some(2));
        let normalised = normalise_response_diagnostics(
            &[a1, b, a2],
            crate::mcp::validation::ValidationBackend::Embedded,
        );
        assert_eq!(normalised.len(), 2);
        assert_eq!(normalised[0].id, "diag_a");
        assert_eq!(normalised[1].id, "diag_b");
    }
}
