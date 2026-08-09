use std::fs;
use std::io::Read as _;
use std::num::NonZeroU32;
use std::path::{Component, Path, PathBuf};

use anvil_checks::secret::patterns::DEFAULT_COMPILED_PATTERNS;
use anvil_intercept::enforcement::default_rule_registry;
use anvil_intercept_rules::{ChangeKind, RuleInput, ScopedEvaluation};
use anvil_kernel_types::diagnostics::ControlDecision;
use anvil_kernel_types::protection_claim::ProtectionClaim;
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::mcp::enforcement::{self, EnforcementMode, MCP_DEFAULT_ENFORCEMENT};
use crate::mcp::tools::shared::{WorkspacePathKind, normalise_workspace_relative_path};
use crate::mcp::validation::{
    DaemonStatus, DaemonValidationClient, INPUT_RULE_ID, LocalDaemonValidationClient,
    PRE_WRITE_MODE, PreWriteValidationRequest, ValidationBackend, ValidationBackendFailure,
    sanitise_id_part, validate_pre_write,
};

pub const TOOL_NAME: &str = "anvil_validate_write";

const RESPONSE_SCHEMA: &str = "anvil.mcp.validate-write.v1";
const MAX_PROPOSED_CONTENT_BYTES: usize = 1024 * 1024;
/// Process env override for response detail (RMCPF-040/043). Request `detail`
/// wins when both are set. Default is **minimal** (RMCPF-043).
const VALIDATE_DETAIL_ENV: &str = "ANVIL_MCP_VALIDATE_DETAIL";

/// How much of the validate-write envelope to return (RMCPF-040 / design
/// `2026-08-09-agent-facing-validate-write-ergonomics`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResponseDetail {
    /// Pre-ergonomics envelope (summary, diagnostics, correlation, …).
    Full,
    /// Clean allow only: `{ schema, decision }`. Non-allow stays full.
    Minimal,
}

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Pre-write validation gate. Call before EVERY file write. Prefer anvil_apply_patch or patch-only payloads for edits; use full proposedContent for creates. Honour block; on allow, decision alone is authoritative (detail=minimal may omit empty fields). preview+contentSha256 is partial validation only. Honour `block` decisions; do not write files the tool refuses.",
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
                "detail": {
                    "type": "string",
                    "enum": ["full", "minimal"],
                    "description": "Response envelope detail. Default `minimal`: clean allow returns only schema+decision. `full` returns the complete envelope (summary, diagnostics, correlation, claim, tier). Request overrides ANVIL_MCP_VALIDATE_DETAIL."
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
            // workspace root to read `.anvil.yaml` from. The decision is
            // always an outright `block` regardless of enforcement mode —
            // the tool cannot evaluate a request it cannot parse — while
            // the reported `enforcementMode` is the MCP default posture
            // (unresolved here).
            return tool_result(&problem_payload(problem, None, MCP_DEFAULT_ENFORCEMENT));
        }
        Err(ParseError::UntrustedWorkspaceRoot { expected }) => {
            // CIB-007: same `block` outcome as any other input
            // problem, plus a recoverable `expectedWorkspaceRoot`
            // field so the caller can retry with the right value.
            return tool_result(&untrusted_workspace_root_payload(
                &expected,
                MCP_DEFAULT_ENFORCEMENT,
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
    // CIB-006: pre-image retained when the post-image was materialised
    // from a patch, so the risk-tier safelist can structurally diff the
    // change. `None` on every other content path (full content,
    // preview, no content) — those always take the full tier.
    let materialised_original = match materialise_patch_if_needed(&mut request, enforcement_mode) {
        Ok(original) => original,
        Err(payload) => return payload,
    };

    // CIB-006: risk-tiered validation. A patch-materialised update is
    // matched against the documented safelist of trivial change shapes
    // BEFORE the full pipeline runs; a hit is served by the scoped
    // embedded evaluation and never reaches the daemon. Everything
    // else falls through to the unchanged full pipeline below.
    if let Some(original) = materialised_original.as_deref()
        && matches!(request.operation, Operation::Update)
        && let Some(hit) = safelist_match(
            &request.relative_path,
            original,
            request.content.as_deref().expect("materialised above"),
        )
    {
        let mut payload = tiered_validation_payload(&request, &hit, enforcement_mode);
        apply_response_detail(&mut payload, request.detail);
        return tool_result(&payload);
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

    let mut diagnostics = normalise_response_diagnostics(&diagnostics, backend);

    // POLRESET-006 / OPAE-007: additive pre-write policy evaluation, run AFTER
    // the intercept-rules scan and never replacing it (see
    // `merge_prewrite_policy`). Appends policy diagnostics and merges the routed
    // policy decision strictest-wins with the scan decision.
    let decision = merge_prewrite_policy(&request, &mut diagnostics, enforcement_mode);

    let mut payload = validation_payload_with_decision(
        &request.relative_path,
        &diagnostics,
        backend,
        daemon_status,
        None,
        enforcement_mode,
        decision,
        request.partial_scan,
        protection_claim.as_ref(),
    );
    // CIB-006: surface the tier taken so callers can audit. Every path
    // through this arm ran (or intentionally skipped, for content-free
    // operations) the FULL pipeline; the recorded reason says why the
    // safelist did not serve the request.
    let full_tier_reason = if materialised_original.is_some() {
        // A patch was materialised but the safelist matcher declined —
        // multi-node change, structural change, non-JSON target, or a
        // value the conservative screens refused.
        "patch-not-safelisted"
    } else if request.partial_scan {
        "preview"
    } else if request.content.is_some() {
        "full-content"
    } else {
        // Delete/rename: no post-image content exists to tier.
        "no-content"
    };
    payload["tier"] = json!({
        "decision": "full",
        "reason": full_tier_reason,
    });
    apply_response_detail(&mut payload, request.detail);
    tool_result(&payload)
}

/// RMCPF-040: shrink clean-allow responses when the caller asked for
/// `detail: minimal`. Non-allow decisions keep the full action payload
/// (diagnostics, safeDefault, errors). Validation quality is unchanged —
/// only serialisation of the result is gated.
pub(crate) fn apply_response_detail(payload: &mut Value, detail: ResponseDetail) {
    if detail != ResponseDetail::Minimal {
        return;
    }
    let decision = payload.get("decision").and_then(|value| value.as_str());
    if decision != Some("allow") {
        return;
    }
    let schema = payload
        .get("schema")
        .cloned()
        .unwrap_or_else(|| json!(RESPONSE_SCHEMA));
    let decision = payload
        .get("decision")
        .cloned()
        .unwrap_or_else(|| json!("allow"));
    *payload = json!({
        "schema": schema,
        "decision": decision,
    });
}

/// Resolve response detail: request `detail` wins, then
/// `ANVIL_MCP_VALIDATE_DETAIL`, then **minimal** (RMCPF-043).
pub(crate) fn resolve_response_detail(
    arguments: &serde_json::Map<String, Value>,
) -> ResponseDetail {
    let env_value = std::env::var(VALIDATE_DETAIL_ENV).ok();
    resolve_response_detail_with(arguments, env_value.as_deref())
}

/// Pure resolver for unit tests (no process-env mutation required).
pub(crate) fn resolve_response_detail_with(
    arguments: &serde_json::Map<String, Value>,
    env_value: Option<&str>,
) -> ResponseDetail {
    if let Some(raw) = arguments.get("detail").and_then(|value| value.as_str()) {
        match raw {
            "minimal" => return ResponseDetail::Minimal,
            "full" => return ResponseDetail::Full,
            _ => {
                // Unknown values fall through to env/default rather than
                // failing the whole write gate.
            }
        }
    }
    if env_value == Some("full") {
        ResponseDetail::Full
    } else {
        // Explicit "minimal", unset, or any other value → minimal (A4 default).
        ResponseDetail::Minimal
    }
}

/// POLRESET-006 / OPAE-007: run kill-switch-gated, fail-open pre-write policy
/// evaluation AFTER the intercept-rules scan, appending its diagnostics to
/// `diagnostics` and returning the strictest of the scan decision and the
/// routed policy decision (strictest-wins). Additive — it never suppresses a
/// scan finding, and a broken pack or an eval failure warns rather than blocks
/// (ADR-098 AD-5).
fn merge_prewrite_policy(
    request: &ValidateWriteRequest,
    diagnostics: &mut Vec<Diagnostic>,
    enforcement_mode: EnforcementMode,
) -> ControlDecision {
    // Base decision from the intercept-rules scan, over the scan diagnostics
    // only (before the policy diagnostics are appended).
    let scan_decision = enforcement::decision_for(diagnostics, enforcement_mode);
    let policy = crate::mcp::policy_prewrite::evaluate(
        &request.workspace_root,
        &request.relative_path,
        request.operation.policy_change_kind(),
        enforcement_mode,
    );
    diagnostics.extend(policy.diagnostics);
    crate::mcp::policy_prewrite::strictest_decision(scan_decision, policy.decision)
}

/// CIB-005: apply the caller's patch to the on-disk file in memory
/// when — and only when — the request has no content, carries a patch,
/// AND the operation actually consumes content. Delete and rename do
/// not require post-image content, so a patch field on those
/// operations is correlation metadata only — reading the on-disk file
/// there would scan content that is about to disappear and could block
/// the operation on findings in soon-to-be-removed bytes (Copilot
/// review, 2026-05-18).
///
/// On success the post-image is stored on `request.content` and the
/// retained pre-image is returned (`None` when no materialisation was
/// needed) for the CIB-006 safelist diff. Failures — unreadable
/// target, patch mismatch, or a post-image that fails the input checks
/// (size, NUL) — return the ready-to-send error result.
fn materialise_patch_if_needed(
    request: &mut ValidateWriteRequest,
    enforcement_mode: EnforcementMode,
) -> Result<Option<String>, Value> {
    if request.content.is_some()
        || request.patch_text.is_none()
        || !request.operation.requires_content()
    {
        return Ok(None);
    }

    match materialise_patch_content(
        &request.workspace_root,
        &request.relative_path,
        request.patch_text.as_deref().expect("checked above"),
    ) {
        Ok(materialised) => {
            request.content = Some(materialised.post_image);
            // Re-run the post-content input checks (size, NUL) now
            // that we have materialised content.
            if let Some(problem) = request.input_problem() {
                return Err(tool_result(&problem_payload(
                    problem,
                    Some(&request.relative_path),
                    enforcement_mode,
                )));
            }
            Ok(Some(materialised.original))
        }
        Err(problem) => Err(tool_result(&problem_payload(
            problem,
            Some(&request.relative_path),
            enforcement_mode,
        ))),
    }
}

fn tool_result(payload: &Value) -> Value {
    // ADR-098 AD-3 amendment 1: gate `isError` on the true decision via
    // `ControlDecision::is_veto` (block / fence / interrupt), not a
    // `== "block"` string compare — a fence-vetoed write must not report
    // `isError: false`. An unrecognised decision string deserialises to
    // `Unknown` (not a veto), matching the safe `warn` default.
    let vetoed = serde_json::from_value::<ControlDecision>(payload["decision"].clone())
        .is_ok_and(ControlDecision::is_veto);
    let is_error = vetoed || payload.get("error").is_some();
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
            "enforcementMode": MCP_DEFAULT_ENFORCEMENT.as_str()
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

    // ADR-098 AD-3 amendment 1: any veto (block / fence / interrupt) sets
    // the do-not-write safe default, not just an outright `block`.
    if decision.is_veto() {
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
            // Unknown severity (newer producer) counts as a warning per
            // the envelope-spec forward-compat rule (ADR-096).
            Severity::Warning | Severity::Unknown => warning += 1,
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
    /// RMCPF-040: envelope detail for the response.
    detail: ResponseDetail,
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
        let detail = resolve_response_detail(arguments);

        Ok(Self {
            workspace_root,
            relative_path,
            operation,
            content,
            patch_text,
            partial_scan,
            preflight_problem,
            detail,
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

    /// POLRESET-006: the policy-engine change kind for this operation, so the
    /// pre-write policy input reflects what the write does to the path.
    const fn policy_change_kind(self) -> anvil_policy_engine::context::assertion::ChangeKind {
        use anvil_policy_engine::context::assertion::ChangeKind;
        match self {
            Self::Create => ChangeKind::Added,
            Self::Update => ChangeKind::Modified,
            Self::Delete => ChangeKind::Removed,
            Self::Rename => ChangeKind::Renamed,
        }
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
    // Prefer `dunce` so Windows NT-extended (`\\?\`) prefixes do not
    // disagree with caller-supplied absolute paths from tempfile or
    // agent tooling. On Unix this is identical to `std::fs::canonicalize`.
    let root = dunce::canonicalize(root).map_err(|_| {
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

fn resolve_workspace_path(workspace_root: &Path, raw_path: &str) -> Result<String, ToolProblem> {
    let path = Path::new(raw_path);
    let relative = if path.is_absolute() {
        let normalised = normalise_absolute_path(path)?;
        // Align the absolute spelling onto the same form as
        // `workspace_root` (already `dunce::canonicalize`d) before
        // prefix-stripping. Without this, macOS tempfile paths under
        // `/var` (symlink to `/private/var`) and Windows paths that
        // disagree about the `\\?\` prefix falsely fail as workspace
        // escapes even when they resolve inside the root.
        let aligned = align_absolute_path_to_canonical_root(&normalised)?;
        aligned
            .strip_prefix(workspace_root)
            .map_err(|_| workspace_escape_problem())?
            .to_path_buf()
    } else {
        if raw_path.is_empty() || is_empty_after_relative_normalisation(raw_path) {
            return Err(ToolProblem::new(
                "missing-path",
                "Validate-write requires a path.",
            ));
        }
        PathBuf::from(
            normalise_workspace_relative_path(
                "Validate-write path",
                raw_path,
                WorkspacePathKind::HostFilesystem,
            )
            .map_err(|_| workspace_escape_problem())?,
        )
    };

    if relative.as_os_str().is_empty() {
        return Err(ToolProblem::new(
            "missing-path",
            "Validate-write requires a path.",
        ));
    }

    Ok(path_to_slash_string(&relative))
}

/// Resolve an absolute path onto the same canonical spelling used for
/// `workspace_root`, without inventing missing parents.
///
/// Walks up to the longest existing ancestor, canonicalises that
/// ancestor (via `dunce`), and re-appends any non-existent suffix. This
/// keeps create-of-new-file paths working while still letting a
/// symlink in an existing segment pull the reconstructed path outside
/// the workspace (so the subsequent `strip_prefix` rejects it).
fn align_absolute_path_to_canonical_root(absolute: &Path) -> Result<PathBuf, ToolProblem> {
    let mut anchor = absolute;
    while !anchor.exists() {
        let Some(parent) = anchor.parent() else {
            return Err(workspace_escape_problem());
        };
        if parent == anchor {
            return Err(workspace_escape_problem());
        }
        anchor = parent;
    }

    let canonical_anchor = dunce::canonicalize(anchor).map_err(|_| workspace_escape_problem())?;
    let suffix = absolute
        .strip_prefix(anchor)
        .map_err(|_| workspace_escape_problem())?;
    Ok(canonical_anchor.join(suffix))
}

fn is_empty_after_relative_normalisation(raw_path: &str) -> bool {
    let portable = raw_path.replace('\\', "/");
    !portable.starts_with('/')
        && portable
            .split('/')
            .all(|segment| segment.is_empty() || segment == ".")
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

    // Match `canonical_workspace_root` spelling (no Windows `\\?\`).
    let canonical_anchor = dunce::canonicalize(anchor).map_err(|_| workspace_escape_problem())?;
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

/// CIB-005: the in-memory result of applying a patch to the on-disk
/// file. CIB-006 keeps the pre-image alongside the post-image so the
/// risk-tier safelist can structurally diff the change.
struct MaterialisedPatch {
    /// The on-disk content the patch was applied against.
    original: String,
    /// The post-image produced by applying the patch in memory.
    post_image: String,
}

/// CIB-005: materialise the post-image of a patch against the
/// on-disk file. Returns the pre- and post-image content if the patch
/// applies cleanly; otherwise a structured `ToolProblem` whose code
/// distinguishes "file missing/unreadable" from "patch context did
/// not match disk". The on-disk file is never written.
fn materialise_patch_content(
    workspace_root: &Path,
    relative_path: &str,
    patch_text: &str,
) -> Result<MaterialisedPatch, ToolProblem> {
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

    let post_image = apply_unified_diff(&original, patch_text).map_err(|_| {
        ToolProblem::new(
            "patch-apply-failed",
            "Validate-write could not apply the supplied patch to the current on-disk content (context mismatch or unsupported diff shape).",
        )
    })?;
    Ok(MaterialisedPatch {
        original,
        post_image,
    })
}

// ---------------------------------------------------------------------
// CIB-006: risk-tiered validation for trivial edits.
// ---------------------------------------------------------------------

/// CIB-006: the documented safelist of trivial change shapes.
///
/// # What the safelist does
///
/// Even with patch-mode (CIB-005) in place, full pipeline validation is
/// overkill for genuinely trivial changes. When a patch-materialised
/// update matches a safelist entry, the validator serves it embedded —
/// the daemon round-trip is skipped (that is the speedup). What is
/// NEVER skipped is coverage: the whole-file secret scan still runs
/// over the complete post-image, and the remaining rules run scoped to
/// the touched node (rules whose declared inputs do not overlap the
/// node are skipped with a recorded reason). The tier taken (`full`
/// vs `safelist`) is always surfaced in the response `tier` object so
/// callers can audit the decision.
///
/// # Safelist criteria (initial entries)
///
/// - **`json-single-string-value`** — a single string-value rename
///   inside a JSON file at a stable path. All of the following must
///   hold, checked deterministically against the pre-/post-images:
///   1. the target path has a `.json` extension;
///   2. pre- and post-image both parse as JSON;
///   3. structurally, exactly ONE string leaf differs — no key
///      add/remove/rename, no array length change, no type change, no
///      change to any non-string leaf;
///   4. textually, the images have the same line count and exactly ONE
///      line differs (this pins the touched node to a concrete line
///      and rejects reformat-plus-edit patches).
///
///   A new value that looks like a secret still matches the shape —
///   and is then caught by the fast path's whole-file secret scan and
///   blocked with the same redaction as the full path (adversarial
///   review 2026-07-02: the earlier raw regex pre-screen here was
///   redundant with, and weaker than, that scan).
///
/// # Out-of-safelist behaviour
///
/// Everything else — full-content payloads, previews, create (the
/// safelist is update-only), delete/rename, non-JSON targets,
/// multi-node or structural changes, unparsable JSON — runs the full
/// pipeline unchanged, and the response records why (`tier.reason`).
///
/// # Growth policy
///
/// The safelist is data-driven (add a [`SafelistEntry`] here), but the
/// policy decision of which shapes are "safe enough" to skim carries
/// real under-validation risk: new entries need explicit sign-off (see
/// CIB-006's confidence note) before the list grows beyond what this
/// documentation describes. A safelist hit must NEVER skip a check
/// that could catch a real risk on a non-trivial edit — when in doubt,
/// a matcher must decline and let the full pipeline run.
const RISK_TIER_SAFELIST: &[SafelistEntry] = &[SafelistEntry {
    id: JSON_SINGLE_STRING_VALUE_ENTRY_ID,
    matcher: match_json_single_string_value,
}];

const JSON_SINGLE_STRING_VALUE_ENTRY_ID: &str = "json-single-string-value";

/// One documented safelist entry: a stable id (surfaced in the
/// response `tier.safelistEntry`) plus a deterministic matcher.
struct SafelistEntry {
    id: &'static str,
    matcher: fn(relative_path: &str, original: &str, post_image: &str) -> Option<SafelistHit>,
}

/// A successful safelist match: which entry matched, where the touched
/// node lives, and the node-scoped content the overlapping rules run
/// against.
struct SafelistHit {
    entry_id: &'static str,
    /// RFC 6901 JSON Pointer of the touched node.
    json_pointer: String,
    /// 1-based line of the touched node in the post-image.
    line: NonZeroU32,
    /// The touched post-image line — the only content the edit
    /// introduced (the matcher guarantees every other byte of the
    /// post-image is already on disk unchanged).
    scoped_content: String,
}

/// CIB-006: match the change against [`RISK_TIER_SAFELIST`] in order;
/// first hit wins. `None` means the full pipeline must run. The table
/// is authoritative for the surfaced entry id — a matcher cannot
/// misreport which entry it implements.
fn safelist_match(relative_path: &str, original: &str, post_image: &str) -> Option<SafelistHit> {
    RISK_TIER_SAFELIST.iter().find_map(|entry| {
        (entry.matcher)(relative_path, original, post_image).map(|mut hit| {
            hit.entry_id = entry.id;
            hit
        })
    })
}

/// Matcher for [`JSON_SINGLE_STRING_VALUE_ENTRY_ID`] — see the
/// criteria documented on [`RISK_TIER_SAFELIST`]. Every check is
/// deterministic; any doubt declines the match.
fn match_json_single_string_value(
    relative_path: &str,
    original: &str,
    post_image: &str,
) -> Option<SafelistHit> {
    // 1. JSON files only.
    if !Path::new(relative_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
    {
        return None;
    }

    // 4 (checked early because it is the cheapest disqualifier):
    // textual discipline — same line count, exactly one differing line.
    let old_lines: Vec<&str> = original.split('\n').collect();
    let new_lines: Vec<&str> = post_image.split('\n').collect();
    if old_lines.len() != new_lines.len() {
        return None;
    }
    let mut touched_idx: Option<usize> = None;
    for (idx, (old_line, new_line)) in old_lines.iter().zip(&new_lines).enumerate() {
        if old_line != new_line {
            if touched_idx.is_some() {
                // Multi-line change — not a single-node edit.
                return None;
            }
            touched_idx = Some(idx);
        }
    }
    // A byte-identical post-image is not a rename; nothing to tier.
    let touched_idx = touched_idx?;

    // 2. Both sides must parse as JSON.
    let old_value: Value = serde_json::from_str(original).ok()?;
    let new_value: Value = serde_json::from_str(post_image).ok()?;

    // 3. Structural diff: exactly one string leaf differs, everything
    // else (keys, array lengths, types, non-string leaves) identical.
    let mut pointer = String::new();
    let mut touched_pointer: Option<String> = None;
    single_string_value_diff(&old_value, &new_value, &mut pointer, &mut touched_pointer).ok()?;
    let json_pointer = touched_pointer?;

    // No secret pre-screen here (adversarial review 2026-07-02): a raw
    // regex pass over the touched line duplicated — without the shared
    // SCAN-002 line-length guard — the whole-file secret scan the
    // tiered path runs over the complete post-image anyway. Secret
    // handling lives in `tiered_validation_payload`, not the matcher.
    let touched_line = new_lines[touched_idx];
    let line = u32::try_from(touched_idx)
        .ok()
        .and_then(|idx| idx.checked_add(1))
        .and_then(NonZeroU32::new)?;
    Some(SafelistHit {
        entry_id: JSON_SINGLE_STRING_VALUE_ENTRY_ID,
        json_pointer,
        line,
        scoped_content: touched_line.to_string(),
    })
}

/// Recursive structural diff for the `json-single-string-value`
/// matcher. Records the RFC 6901 pointer of the single differing
/// string leaf in `found`; returns `Err(())` the moment the change is
/// disqualified (key add/remove, array length change, type change,
/// non-string leaf change, or a second differing leaf).
fn single_string_value_diff(
    old: &Value,
    new: &Value,
    pointer: &mut String,
    found: &mut Option<String>,
) -> Result<(), ()> {
    match (old, new) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            if old_map.len() != new_map.len() {
                return Err(());
            }
            for (key, old_child) in old_map {
                // A missing key here is a key rename/remove (lengths
                // already matched, so an addition pairs with it).
                let new_child = new_map.get(key).ok_or(())?;
                let saved_len = pointer.len();
                pointer.push('/');
                pointer.push_str(&escape_json_pointer_token(key));
                single_string_value_diff(old_child, new_child, pointer, found)?;
                pointer.truncate(saved_len);
            }
            Ok(())
        }
        (Value::Array(old_items), Value::Array(new_items)) => {
            if old_items.len() != new_items.len() {
                return Err(());
            }
            for (idx, (old_child, new_child)) in old_items.iter().zip(new_items).enumerate() {
                let saved_len = pointer.len();
                pointer.push('/');
                pointer.push_str(&idx.to_string());
                single_string_value_diff(old_child, new_child, pointer, found)?;
                pointer.truncate(saved_len);
            }
            Ok(())
        }
        (Value::String(old_str), Value::String(new_str)) => {
            if old_str != new_str {
                if found.is_some() {
                    // Second differing leaf — multi-node change.
                    return Err(());
                }
                *found = Some(pointer.clone());
            }
            Ok(())
        }
        // Any other pairing: identical is fine, different (including a
        // type change) disqualifies.
        (old_other, new_other) => {
            if old_other == new_other {
                Ok(())
            } else {
                Err(())
            }
        }
    }
}

/// RFC 6901 token escaping: `~` -> `~0`, `/` -> `~1`.
fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// CIB-006: the rule registry the safelist fast path evaluates.
///
/// This MUST stay identical to the registry the embedded full path
/// uses (`EnforcementPipeline::default()`, which wraps
/// `default_rule_registry()`); the
/// `safelist_registry_matches_enforcement_default` test trips if the
/// seams drift. INTR-007: workspace-configured registries
/// (`registry_from_workspace`) are wired nowhere on the MCP pre-write
/// path today — when they are wired into the enforcement seam, this
/// fast path must resolve the SAME workspace registry, not stay pinned
/// to the static default.
fn tiered_rule_registry() -> anvil_intercept_rules::RuleRegistry {
    default_rule_registry()
}

/// CIB-006: build the response for a safelist hit.
///
/// The actual contract (corrected per adversarial review 2026-07-02):
/// a safelist hit skips the DAEMON round-trip — it does NOT skip
/// whole-file secret coverage. The MCP pre-write path validates the
/// entire post-image on the full tier, so the fast tier must match
/// that coverage for secrets:
///
/// 1. **Whole-file secret scan** — the same embedded
///    `SecretDetectionRule` the full path's enforcement pipeline
///    registers (with its shared SCAN-002 line-length guard) runs over
///    the COMPLETE post-image, so a secret on an untouched line is
///    caught exactly as on the full path, at its real line.
/// 2. **Touched-node-scoped evaluation** — the remaining registry
///    rules (the launch reasoning pattern today) run against the
///    touched node's content only; rules whose declared inputs do not
///    overlap the node are recorded in `tier.skippedRules` with their
///    reason.
///
/// The registry seam is pinned by
/// `safelist_registry_matches_enforcement_default`; see
/// [`tiered_rule_registry`] for the INTR-007 coupling note.
fn tiered_validation_payload(
    request: &ValidateWriteRequest,
    hit: &SafelistHit,
    enforcement_mode: EnforcementMode,
) -> Value {
    let mode = Mode::Unknown(PRE_WRITE_MODE.to_string());
    let registry = tiered_rule_registry();
    let path = Path::new(&request.relative_path);

    // (1) Whole-file secret coverage over the complete post-image.
    // Diagnostics keep their real line numbers — no remap.
    let post_image = request
        .content
        .as_deref()
        .expect("safelist hits require materialised content");
    let full_input = RuleInput {
        path,
        change_kind: ChangeKind::Modified,
        content: Some(post_image.as_bytes()),
    };
    let mut diagnostics =
        anvil_intercept_rules::SecretDetectionRule::default().diagnostics(&full_input, &mode);

    // (2) Scoped evaluation of the registry against the touched node.
    let scoped_input = RuleInput {
        path,
        change_kind: ChangeKind::Modified,
        content: Some(hit.scoped_content.as_bytes()),
    };
    let scoped: ScopedEvaluation =
        registry.diagnostics_scoped_to_touched_node(&scoped_input, &mode);

    // The scoped buffer is a single post-image line, so rules report
    // line 1 (or none); remap onto the touched node's real line so the
    // diagnostics point at the file location the caller will write.
    // (A scoped secret finding duplicates the whole-file scan's; the
    // dedupe inside `normalise_response_diagnostics` collapses it.)
    let mut scoped_diagnostics = scoped.diagnostics;
    for diagnostic in &mut scoped_diagnostics {
        if diagnostic.location.line.is_some() {
            diagnostic.location.line = Some(hit.line.get());
        }
    }
    diagnostics.extend(scoped_diagnostics);
    let diagnostics = normalise_response_diagnostics(&diagnostics, ValidationBackend::Embedded);

    // The decision still flows through the enforcement-mode policy —
    // a scoped finding warns or blocks exactly as it would on the full
    // path (warnings over blocks; the tier changes scope, not policy).
    let mut payload = validation_payload(
        &request.relative_path,
        &diagnostics,
        ValidationBackend::Embedded,
        DaemonStatus::NotWired,
        None,
        enforcement_mode,
        false,
        None,
    );
    payload["tier"] = json!({
        "decision": "safelist",
        "safelistEntry": hit.entry_id,
        "touchedNode": {
            "jsonPointer": hit.json_pointer,
            "line": hit.line.get(),
        },
        "firedRules": scoped.fired_rule_ids,
        "skippedRules": scoped
            .skipped
            .iter()
            .map(|skip| json!({ "ruleId": skip.rule_id, "reason": skip.reason }))
            .collect::<Vec<_>>(),
    });
    payload
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
    use std::cell::RefCell;
    use std::fs;

    use super::descriptor;
    use super::{
        EnforcementResolver, MAX_PROPOSED_CONTENT_BYTES, call_with_validation_client,
        diagnostic_summary, normalise_response_diagnostics, redact_secret_id,
    };
    use crate::mcp::enforcement::EnforcementMode;
    use crate::mcp::validation::{
        DaemonValidationClient, DaemonValidationOutcome, LocalDaemonValidationClient,
        PreWriteValidationRequest, ValidationBackendFailure,
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

    struct RecordingDaemon {
        seen_path: RefCell<Option<String>>,
    }

    impl DaemonValidationClient for FixtureDaemon {
        fn validate_pre_write(
            &self,
            _request: &PreWriteValidationRequest<'_>,
        ) -> DaemonValidationOutcome {
            self.outcome.clone()
        }
    }

    impl DaemonValidationClient for RecordingDaemon {
        fn validate_pre_write(
            &self,
            request: &PreWriteValidationRequest<'_>,
        ) -> DaemonValidationOutcome {
            self.seen_path
                .replace(Some(request.relative_path.to_string()));
            DaemonValidationOutcome::Diagnostics(Vec::new())
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
                "detail": "full",
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
                    "enforcementMode": "interrupt",
                },
                "tier": {
                    "decision": "full",
                    "reason": "full-content",
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
                "detail": "full",
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(payload["decision"], "interrupt");
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
    fn fence_posture_vetoes_with_true_fence_decision_and_is_error() {
        // ADR-098 AD-3 regression: under a `fence` posture a secret error
        // records the true `fence` decision (no collapse to `block`), the
        // response is `isError: true` via `is_veto`, and the do-not-write
        // safe default is set. The pre-AD-3 shim would have reported
        // `decision: "block"`; a `== "block"` isError gate would also have
        // let a fence-vetoed write slip through as `isError: false`.
        let workspace = tempdir().expect("workspace exists");
        let result = call_with_validation_client(
            &json!({
                "detail": "full",
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &LocalDaemonValidationClient,
            &FixedEnforcement(EnforcementMode::Fence),
        );

        assert_eq!(result["isError"], true, "a fence veto must report isError");
        let payload = parse_payload(&result);
        assert_eq!(payload["decision"], "fence", "fence stays fence");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["correlation"]["enforcementMode"], "fence");
    }

    #[test]
    fn daemon_backend_payload_is_reported_when_available() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Diagnostics(vec![sample_daemon_diagnostic()]),
        };
        let result = call_with_validation_client(
            &json!({
                "detail": "full",
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["decision"], "interrupt");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["correlation"]["backend"], "daemon");
        assert_eq!(payload["correlation"]["enforcementMode"], "interrupt");
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
                "detail": "full",
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
        );
        let payload = parse_payload(&result);
        let response_text = serde_json::to_string(&payload).expect("payload serialises");
        let expected_redacted_id = redact_secret_id(daemon_secret_id, false);

        assert_eq!(payload["decision"], "interrupt");
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
            "detail": "full",
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
            &FixedEnforcement(EnforcementMode::Interrupt),
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
            &FixedEnforcement(EnforcementMode::Interrupt),
        ));

        assert_eq!(daemon["diagnostics"], embedded["diagnostics"]);
    }

    // DSV-007 Task 13: the two named tests for the MCP daemon re-point.
    //
    // Reconciliation note (council-confirmed 2026-06-03): the execution-plan
    // wording "re-point the in-process scan to daemon `validate_paths`" is an
    // error. `anvil_validate_write` is a *pre-write* gate over *proposed content
    // not yet on disk*; the daemon `validate_paths` verb has a frozen
    // content-free wire and certifies the *on-disk* bytes it reads itself
    // (ADR-061 §2/§7), so routing proposed content through it would attest the
    // stale on-disk file. The correct daemon verb for buffer content is
    // `scan_buffer` (preWrite mode) — already wired here via
    // `LocalDaemonValidationClient`. ADR-061 §3 only says MCP "re-points… to the
    // daemon" (not `validate_paths`). Consequence for the Task 15 / DSV-009
    // four-path parity gate: the "MCP+daemon" leg exercises `scan_buffer`, not
    // `validate_paths` (both route to the same `run_antipattern_check`).

    #[test]
    fn validate_write_uses_daemon_when_present() {
        // A reachable daemon serves the verdict (backend=daemon); the MCP tool
        // does not re-scan in-process.
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Diagnostics(vec![sample_daemon_diagnostic()]),
        };
        let payload = parse_payload(&call_with_validation_client(
            &json!({
                "detail": "full",
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
        ));

        assert_eq!(
            payload["correlation"]["backend"], "daemon",
            "a present daemon must serve the verdict, not the embedded scanner",
        );
        assert_eq!(payload["correlation"]["daemonStatus"], "available");
        assert_eq!(
            payload["diagnostics"][0]["source"]["rule_id"],
            "secret-detection",
        );
    }

    #[test]
    fn validate_write_in_process_fallback() {
        // A daemon-absent call falls back to the in-process scanner, still
        // catching the secret. The byte-identical guarantee against the daemon
        // envelope is pinned by `daemon_and_embedded_paths_emit_identical_diagnostic_envelopes`
        // (in-process) and `live_daemon_mcp_tool_call_matches_embedded_diagnostic_envelope`
        // (over a real socket); this test pins the standalone fallback path.
        let workspace = tempdir().expect("workspace exists");
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::Unavailable,
        };
        let payload = parse_payload(&call_with_validation_client(
            &json!({
                "detail": "full",
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
        ));

        assert_eq!(
            payload["correlation"]["backend"], "embedded",
            "a daemon-absent call must fall back to the in-process scanner",
        );
        assert_eq!(payload["correlation"]["daemonStatus"], "not-wired");
        assert_eq!(
            payload["decision"], "interrupt",
            "the in-process fallback must still catch the secret",
        );
        assert_eq!(
            payload["diagnostics"][0]["source"]["rule_id"],
            "secret-detection",
        );
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
            "detail": "full",
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
            &FixedEnforcement(EnforcementMode::Interrupt),
        ));
        let daemon = parse_payload(&call_with_validation_client(
            &arguments,
            workspace.path(),
            &super::LocalDaemonValidationClient::with_socket_path(socket),
            &FixedEnforcement(EnforcementMode::Interrupt),
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
                message: "anvil could not validate the proposed write.",
                retriable: true,
            }),
        };
        let result = call_with_validation_client(
            &json!({
                "detail": "full",
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
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
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
                "path": "src/secret.ts",
                "operation": "update",
                "patch": "--- a/src/secret.ts\n+++ b/src/secret.ts\n@@ -1 +1 @@\n-const placeholder = 'redacted';\n+const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(payload["decision"], "interrupt");
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
                "detail": "full",
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
        // CIB-006: the original beta-tester shape (one-string JSON
        // rename at a stable path) is exactly the seeded safelist
        // entry, so the tiered path serves it.
        assert_eq!(
            payload["tier"]["decision"], "safelist",
            "the incident shape must take the risk-tiered path, got: {payload}"
        );
    }

    // ----- CIB-006: risk-tiered validation for trivial edits. ---------

    /// Standard four-line JSON fixture used by the CIB-006 tests.
    /// Layout (1-indexed): line 2 holds the `name` value.
    const CIB006_FIXTURE: &str = "{\n  \"name\": \"old-name\",\n  \"version\": \"1.0.0\"\n}\n";

    fn seed_cib006_fixture(workspace: &std::path::Path, file_name: &str, body: &str) {
        let target = workspace.join(file_name);
        fs::create_dir_all(target.parent().expect("parent dir")).expect("fixture dir created");
        fs::write(&target, body).expect("seed fixture");
    }

    /// CIB-006: a single-string JSON value rename at a stable path
    /// matches the seeded safelist entry, short-circuits the full
    /// pipeline, and surfaces the tier taken (plus the touched node
    /// and the fired/skipped rule audit trail) in the response.
    ///
    /// The fixture daemon is wired to `OperationalFailure`, which the
    /// full path escalates to a hard `block`. An `allow` here is
    /// therefore proof the daemon was never consulted on the tiered
    /// path — not merely that the scan came back clean.
    #[test]
    fn safelist_hit_json_single_string_value_fast_paths() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(workspace.path(), "config.json", CIB006_FIXTURE);
        let daemon = FixtureDaemon {
            outcome: DaemonValidationOutcome::OperationalFailure(ValidationBackendFailure {
                code: "validation-backend-unavailable",
                message: "anvil could not validate the proposed write.",
                retriable: true,
            }),
        };

        let payload = parse_payload(&call_with_validation_client(
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -2 +2 @@\n-  \"name\": \"old-name\",\n+  \"name\": \"new-name\",\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
        ));

        assert_eq!(
            payload["decision"], "allow",
            "safelist hit must not consult the daemon, got: {payload}"
        );
        assert_eq!(payload["tier"]["decision"], "safelist");
        assert_eq!(payload["tier"]["safelistEntry"], "json-single-string-value");
        assert_eq!(payload["tier"]["touchedNode"]["jsonPointer"], "/name");
        assert_eq!(payload["tier"]["touchedNode"]["line"], 2);
        assert_eq!(
            payload["tier"]["firedRules"],
            json!(["secret-detection", "AI-001"]),
            "every default rule overlaps a content node and must keep firing"
        );
        assert_eq!(payload["tier"]["skippedRules"], json!([]));
        assert_eq!(payload["correlation"]["backend"], "embedded");
        assert_eq!(payload["correlation"]["daemonStatus"], "not-wired");
    }

    /// CIB-006 adversarial review (coverage narrowing): the safelist
    /// path must NOT narrow whole-file secret coverage. The full path
    /// validates the entire reconstructed post-image, so a secret
    /// sitting on an UNTOUCHED line blocks there — the fast path must
    /// surface the same diagnostic (at its real line) and block per
    /// enforcement mode, while still serving the safelist tier.
    #[test]
    fn safelist_hit_still_scans_whole_file_for_secrets() {
        let workspace = tempdir().expect("workspace exists");
        // Pre-existing secret on line 3; the edit touches line 2 only.
        seed_cib006_fixture(
            workspace.path(),
            "config.json",
            "{\n  \"name\": \"old-name\",\n  \"token\": \"ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n}\n",
        );

        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -2 +2 @@\n-  \"name\": \"old-name\",\n+  \"name\": \"new-name\",\n"
            }),
        );

        assert_eq!(
            payload["tier"]["decision"], "safelist",
            "the edit shape itself is safelisted, got: {payload}"
        );
        assert_eq!(
            payload["decision"], "interrupt",
            "an untouched-line secret must still block on the fast path, got: {payload}"
        );
        assert_eq!(payload["diagnostics"][0]["category"], "secret");
        assert_eq!(
            payload["diagnostics"][0]["location"]["line"], 3,
            "whole-file diagnostics keep their real line (no touched-node remap)"
        );
        assert!(
            !serde_json::to_string(&payload)
                .expect("payload serialises")
                .contains("ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            "secret redaction applies on the tiered path too"
        );
    }

    /// CIB-006 adversarial review (registry coupling): the safelist
    /// fast path must evaluate the SAME registry as the embedded
    /// enforcement default. This trap fails loudly if either seam
    /// drifts — including when the default registry legitimately gains
    /// a rule (forcing a decision about its tiered coverage). INTR-007:
    /// when workspace-configured registries are wired into enforcement,
    /// `tiered_rule_registry` must resolve the same workspace registry.
    #[test]
    fn safelist_registry_matches_enforcement_default() {
        let tiered = super::tiered_rule_registry();
        let enforcement = anvil_intercept::enforcement::default_rule_registry();
        assert_eq!(
            tiered.rule_ids(),
            enforcement.rule_ids(),
            "CIB-006: the safelist fast path and the embedded full path must never \
             evaluate different rule sets",
        );
        // The whole-file coverage rule on the tiered path must be one
        // of the enforcement rules, or coverage silently forks.
        let secret_rule = anvil_intercept_rules::SecretDetectionRule::default();
        let secret_id = anvil_intercept_rules::InterceptRule::rule_id(&secret_rule).to_string();
        assert!(
            enforcement.rule_ids().contains(&secret_id.as_str()),
            "whole-file secret coverage must use a rule the enforcement default registers",
        );
    }

    /// CIB-006 review: create/delete/rename operations never
    /// safelist-match, even with a `.json` patch attached. The
    /// safelist is update-only by construction; delete/rename never
    /// materialise content at all.
    #[test]
    fn near_miss_non_update_operations_take_full_path() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(workspace.path(), "config.json", CIB006_FIXTURE);
        let rename_patch = "--- a/config.json\n+++ b/config.json\n@@ -2 +2 @@\n-  \"name\": \"old-name\",\n+  \"name\": \"new-name\",\n";

        // create + patch on an existing file materialises, but the
        // safelist is gated to update.
        let create = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "create",
                "patch": rename_patch
            }),
        );
        assert_eq!(
            create["tier"],
            json!({ "decision": "full", "reason": "patch-not-safelisted" }),
            "create must not safelist-match, got: {create}"
        );

        // delete/rename + patch never materialise (patch is
        // correlation metadata only) and cannot safelist-match.
        for operation in ["delete", "rename"] {
            let payload = call_payload(
                workspace.path(),
                &json!({
                    "detail": "full",
                    "path": "config.json",
                    "operation": operation,
                    "patch": rename_patch
                }),
            );
            assert_eq!(
                payload["tier"],
                json!({ "decision": "full", "reason": "no-content" }),
                "{operation} must not safelist-match, got: {payload}"
            );
        }
    }

    /// CIB-006 review: the matcher judges the materialised images, not
    /// the patch text, so a MULTI-HUNK patch whose net effect is one
    /// differing line is still a single-node edit and safelists. The
    /// second hunk here removes and re-adds an identical line (net
    /// zero); the whole-file secret scan still covers every byte.
    #[test]
    fn multi_hunk_patch_with_single_net_line_change_safelist_matches() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(
            workspace.path(),
            "config.json",
            "{\n  \"name\": \"old-name\",\n  \"version\": \"1.0.0\",\n  \"kind\": \"demo\"\n}\n",
        );

        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -2 +2 @@\n-  \"name\": \"old-name\",\n+  \"name\": \"new-name\",\n@@ -4 +4 @@\n-  \"kind\": \"demo\"\n+  \"kind\": \"demo\"\n"
            }),
        );

        assert_eq!(
            payload["tier"]["decision"], "safelist",
            "net single-line change must safelist regardless of hunk count, got: {payload}"
        );
        assert_eq!(payload["tier"]["touchedNode"]["jsonPointer"], "/name");
        assert_eq!(payload["tier"]["touchedNode"]["line"], 2);
        assert_eq!(payload["decision"], "allow");
    }

    /// CIB-006 review: a CRLF-terminated JSON file behaves like its LF
    /// twin — the line-wise textual check compares `\r`-suffixed lines
    /// consistently on both sides, serde treats `\r` as whitespace,
    /// and the rename still pins to the right node and line.
    #[test]
    fn crlf_file_single_value_rename_safelist_matches() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(
            workspace.path(),
            "config.json",
            "{\r\n  \"name\": \"old-name\",\r\n  \"version\": \"1.0.0\"\r\n}\r\n",
        );

        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -2 +2 @@\n-  \"name\": \"old-name\",\r\n+  \"name\": \"new-name\",\r\n"
            }),
        );

        assert_eq!(
            payload["tier"]["decision"], "safelist",
            "CRLF endings must not break the matcher, got: {payload}"
        );
        assert_eq!(payload["tier"]["touchedNode"]["jsonPointer"], "/name");
        assert_eq!(payload["tier"]["touchedNode"]["line"], 2);
        assert_eq!(payload["decision"], "allow");
    }

    /// CIB-006: the tier decision is surfaced on the full path too, so
    /// callers can audit which tier served every response.
    #[test]
    fn full_path_surfaces_tier_decision_for_full_content() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(
            payload["tier"],
            json!({ "decision": "full", "reason": "full-content" })
        );
    }

    /// CIB-006 near-miss: a patch that changes TWO string values is a
    /// multi-node change and must take the full pipeline.
    #[test]
    fn near_miss_multi_node_change_takes_full_path() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(workspace.path(), "config.json", CIB006_FIXTURE);

        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -2,2 +2,2 @@\n-  \"name\": \"old-name\",\n-  \"version\": \"1.0.0\"\n+  \"name\": \"new-name\",\n+  \"version\": \"2.0.0\"\n"
            }),
        );

        assert_eq!(
            payload["tier"],
            json!({ "decision": "full", "reason": "patch-not-safelisted" }),
            "a multi-node change must fall through, got: {payload}"
        );
        assert_eq!(payload["decision"], "allow");
    }

    /// CIB-006 near-miss: renaming a KEY (even with the value kept)
    /// is a structural change and must take the full pipeline.
    #[test]
    fn near_miss_key_rename_takes_full_path() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(workspace.path(), "config.json", CIB006_FIXTURE);

        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -2 +2 @@\n-  \"name\": \"old-name\",\n+  \"title\": \"old-name\",\n"
            }),
        );

        assert_eq!(payload["tier"]["decision"], "full");
        assert_eq!(payload["tier"]["reason"], "patch-not-safelisted");
    }

    /// CIB-006 near-miss: the safelist entry is JSON-only. A one-line
    /// value change in a non-JSON file must take the full pipeline.
    #[test]
    fn near_miss_non_json_file_takes_full_path() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(
            workspace.path(),
            "config.yaml",
            "name: old-name\nversion: 1.0.0\n",
        );

        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.yaml",
                "operation": "update",
                "patch": "--- a/config.yaml\n+++ b/config.yaml\n@@ -1 +1 @@\n-name: old-name\n+name: new-name\n"
            }),
        );

        assert_eq!(payload["tier"]["decision"], "full");
        assert_eq!(payload["tier"]["reason"], "patch-not-safelisted");
    }

    /// CIB-006 guard: a new value that looks like a secret is still a
    /// safelist-shaped edit, but the fast path's whole-file secret
    /// scan catches it — the safelist never masks the secret scan.
    /// The diagnostic is redacted exactly as on the full path.
    ///
    /// (Adversarial review 2026-07-02: the earlier raw regex
    /// pre-screen in the matcher was dropped as redundant with — and
    /// weaker than — the whole-file scan, which carries the shared
    /// SCAN-002 line-length guard.)
    #[test]
    fn secret_looking_value_still_blocks_on_safelist_path() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(workspace.path(), "config.json", CIB006_FIXTURE);

        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -2 +2 @@\n-  \"name\": \"old-name\",\n+  \"name\": \"ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\n"
            }),
        );

        assert_eq!(payload["tier"]["decision"], "safelist");
        assert_eq!(
            payload["decision"], "interrupt",
            "the fast path must still block a secret-looking value, got: {payload}"
        );
        assert_eq!(payload["diagnostics"][0]["category"], "secret");
        assert!(
            !serde_json::to_string(&payload)
                .expect("payload serialises")
                .contains("ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            "the raw secret must never appear in the response"
        );
    }

    /// CIB-006 near-miss: a value changing TYPE (string to number) is
    /// a structural change and must take the full pipeline.
    #[test]
    fn near_miss_value_type_change_takes_full_path() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(workspace.path(), "config.json", CIB006_FIXTURE);

        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -2 +2 @@\n-  \"name\": \"old-name\",\n+  \"name\": 42,\n"
            }),
        );

        assert_eq!(payload["tier"]["decision"], "full");
        assert_eq!(payload["tier"]["reason"], "patch-not-safelisted");
    }

    /// CIB-006 near-miss: adding a key is a structural change and must
    /// take the full pipeline.
    #[test]
    fn near_miss_key_addition_takes_full_path() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(workspace.path(), "config.json", CIB006_FIXTURE);

        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -2 +2,2 @@\n-  \"name\": \"old-name\",\n+  \"name\": \"old-name\",\n+  \"extra\": \"added\",\n"
            }),
        );

        assert_eq!(payload["tier"]["decision"], "full");
        assert_eq!(payload["tier"]["reason"], "patch-not-safelisted");
    }

    /// CIB-006 near-miss: a `.json` file whose on-disk content is not
    /// valid JSON cannot be structurally diffed and must take the full
    /// pipeline.
    #[test]
    fn near_miss_invalid_json_takes_full_path() {
        let workspace = tempdir().expect("workspace exists");
        seed_cib006_fixture(
            workspace.path(),
            "config.json",
            "{ not valid json: old-name\n",
        );

        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "config.json",
                "operation": "update",
                "patch": "--- a/config.json\n+++ b/config.json\n@@ -1 +1 @@\n-{ not valid json: old-name\n+{ not valid json: new-name\n"
            }),
        );

        assert_eq!(payload["tier"]["decision"], "full");
        assert_eq!(payload["tier"]["reason"], "patch-not-safelisted");
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
                "detail": "full",
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
                "detail": "full",
                "path": "src/example.ts",
                "operation": "update",
                "preview": "const placeholder = 'redacted';\n",
                "patch": "--- a/src/example.ts\n+++ b/src/example.ts\n@@ -1 +1 @@\n-const placeholder = 'redacted';\n+const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(
            payload["decision"], "interrupt",
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
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
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
    fn empty_path_stays_missing_path() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "missing-path");
    }

    #[test]
    fn dot_only_relative_paths_stay_missing_path() {
        let workspace = tempdir().expect("workspace exists");

        for path in [".", "./", "././", ".//./"] {
            let payload = call_payload(
                workspace.path(),
                &json!({
                    "detail": "full",
                    "path": path,
                    "operation": "create",
                    "proposedContent": "export const value = 1;\n"
                }),
            );

            assert_eq!(
                payload["error"]["code"], "missing-path",
                "path {path:?} should stay missing-path"
            );
        }
    }

    #[test]
    fn host_normalised_relative_path_reaches_correlation_and_daemon() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = RecordingDaemon {
            seen_path: RefCell::new(None),
        };
        let result = call_with_validation_client(
            &json!({
                "detail": "full",
                "path": "./src//x.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &super::WorkspaceEnforcementResolver,
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["correlation"]["path"], "src/x.ts");
        assert_eq!(daemon.seen_path.borrow().as_deref(), Some("src/x.ts"));
    }

    #[test]
    fn redundant_forward_separators_reach_validate_write_boundary() {
        let workspace = tempdir().expect("workspace exists");

        for path in [".//src/x.ts", ".//./src/x.ts"] {
            let daemon = RecordingDaemon {
                seen_path: RefCell::new(None),
            };
            let result = call_with_validation_client(
                &json!({
                    "detail": "full",
                    "path": path,
                    "operation": "create",
                    "proposedContent": "export const value = 1;\n"
                }),
                workspace.path(),
                &daemon,
                &super::WorkspaceEnforcementResolver,
            );
            let payload = parse_payload(&result);

            assert_eq!(
                payload["decision"], "allow",
                "path {path:?} should be accepted"
            );
            assert_eq!(payload["correlation"]["path"], "src/x.ts");
            assert_eq!(daemon.seen_path.borrow().as_deref(), Some("src/x.ts"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn literal_unix_backslash_reaches_correlation_and_daemon_unchanged() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = RecordingDaemon {
            seen_path: RefCell::new(None),
        };
        let result = call_with_validation_client(
            &json!({
                "detail": "full",
                "path": r"src/a\b.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &super::WorkspaceEnforcementResolver,
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["correlation"]["path"], r"src/a\b.ts");
        assert_eq!(daemon.seen_path.borrow().as_deref(), Some(r"src/a\b.ts"));
    }

    #[test]
    fn binary_content_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
                "path": "../outside.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "workspace-escape");
    }

    #[test]
    fn portable_relative_path_hazards_are_workspace_escape() {
        let workspace = tempdir().expect("workspace exists");

        for path in [
            r"C:\outside.ts",
            r".\\server\share\outside.ts",
            r".\\\server\share\outside.ts",
            r".\\outside.ts",
            r".\\?\C:\outside.ts",
            r".\\\?\C:\outside.ts",
            r".\\.\pipe\name",
            r".\\\.\pipe\name",
            r".\C:\outside.ts",
            r".\C:relative",
            "./C:/outside.ts",
            "./C:relative",
            r"./\\server\share\outside.ts",
            r"./\outside.ts",
            r"./\\?\C:\outside.ts",
            r"./\\.\pipe\name",
            r"\\server\share\outside.ts",
            r"src\..\outside.ts",
            "src/evil\0name.ts",
        ] {
            let payload = call_payload(
                workspace.path(),
                &json!({
                    "detail": "full",
                    "path": path,
                    "operation": "create",
                    "proposedContent": "export const value = 1;\n"
                }),
            );

            assert_eq!(
                payload["error"]["code"], "workspace-escape",
                "path {path:?} should be rejected"
            );
        }
    }

    #[test]
    fn absolute_path_inside_workspace_stays_accepted_and_relative() {
        let workspace = tempdir().expect("workspace exists");
        let absolute = workspace.path().join("src/absolute.ts");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": absolute.to_string_lossy(),
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["correlation"]["path"], "src/absolute.ts");
    }

    /// Regression for Cross macOS/Windows: an absolute path whose
    /// spelling differs from the canonical workspace root (macOS
    /// `/var` → `/private/var`, or a plain alias symlink) must still
    /// allow when it resolves inside the workspace. Before the fix,
    /// only a lexical `strip_prefix` against the canonical root ran,
    /// so the non-canonical absolute form was reported as a
    /// workspace escape.
    #[cfg(unix)]
    #[test]
    fn absolute_path_via_symlink_workspace_alias_stays_accepted() {
        let base = tempdir().expect("base exists");
        let real = base.path().join("real");
        fs::create_dir(&real).expect("real workspace created");
        let alias = base.path().join("alias");
        std::os::unix::fs::symlink(&real, &alias).expect("alias symlink created");

        // Drive the tool with the non-canonical alias as cwd/root, and
        // pass an absolute path also spelled through the alias — the
        // same shape tempfile + `Path::join` produce on macOS when
        // `/var` is a symlink to `/private/var`.
        let absolute = alias.join("src/absolute.ts");
        let payload = call_payload(
            &alias,
            &json!({
                "detail": "full",
                "path": absolute.to_string_lossy(),
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(
            payload["decision"], "allow",
            "absolute path through workspace alias should allow; payload={payload}"
        );
        assert_eq!(payload["correlation"]["path"], "src/absolute.ts");
    }

    /// Companion to the alias-allow case: an absolute path that walks
    /// through a symlink *out* of the workspace must still block.
    #[cfg(unix)]
    #[test]
    fn absolute_path_through_out_of_workspace_symlink_blocks() {
        let workspace = tempdir().expect("workspace exists");
        let outside = tempdir().expect("outside exists");
        let link = workspace.path().join("escape");
        std::os::unix::fs::symlink(outside.path(), &link).expect("escape symlink created");

        let absolute = link.join("secret.ts");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": absolute.to_string_lossy(),
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
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
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
        // canonicalised cwd (via `dunce`, so no Windows `\\?\` prefix).
        let expected = dunce::canonicalize(workspace.path()).expect("workspace canonicalises");
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
                "detail": "full",
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
                "detail": "full",
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
            workspace.path(),
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
            &FixedEnforcement(EnforcementMode::Interrupt),
        );
        let payload = parse_payload(&result);

        assert_eq!(result["isError"], true);
        assert_eq!(payload["decision"], "interrupt");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["correlation"]["enforcementMode"], "interrupt");
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
                "detail": "full",
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
    /// mode for noisy environments where anvil should report findings
    /// without ever blocking the agent.
    #[test]
    fn enforcement_mode_off_passes_secret_write_with_diagnostics() {
        let workspace = tempdir().expect("workspace exists");
        let result = call_with_validation_client(
            &json!({
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
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
                "detail": "full",
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

    /// Test helper: injects `detail: "full"` when absent so envelope-field
    /// assertions stay readable after RMCPF-043's minimal default.
    fn call_payload(workspace_root: &std::path::Path, arguments: &Value) -> Value {
        call_payload_preserving_detail(workspace_root, arguments, true)
    }

    fn call_payload_preserving_detail(
        workspace_root: &std::path::Path,
        arguments: &Value,
        inject_full_detail: bool,
    ) -> Value {
        let mut arguments = arguments.clone();
        if inject_full_detail && let Some(object) = arguments.as_object_mut() {
            object.entry("detail").or_insert_with(|| json!("full"));
        }
        let result = call_with_validation_client(
            &arguments,
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
                "detail": "full",
                "path": "src/example.ts",
                "operation": "update",
                "preview": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["correlation"]["partialScan"], true);
        // CIB-006: preview-based partial validation is a full-tier
        // response with its own recorded reason.
        assert_eq!(
            payload["tier"],
            json!({ "decision": "full", "reason": "preview" })
        );
    }

    #[test]
    fn preview_content_blocks_when_secret_detected() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
                "path": "src/secret.ts",
                "operation": "update",
                "preview": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(payload["decision"], "interrupt");
        assert_eq!(payload["correlation"]["partialScan"], true);
    }

    #[test]
    fn full_content_does_not_set_partial_scan() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "detail": "full",
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
                "detail": "full",
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
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
                "detail": "full",
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
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
                "detail": "full",
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
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
                message: "anvil could not validate the proposed write.",
                retriable: true,
            }),
            claim: Some(sample_protection_claim()),
        };
        let result = call_with_validation_client(
            &json!({
                "detail": "full",
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &daemon,
            &FixedEnforcement(EnforcementMode::Interrupt),
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
                "detail": "full",
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &FixtureDaemonWithClaim {
                outcome: DaemonValidationOutcome::Diagnostics(vec![]),
                claim: Some(sample_protection_claim()),
            },
            &FixedEnforcement(EnforcementMode::Interrupt),
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
                "detail": "full",
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
            workspace.path(),
            &FixtureDaemonWithClaim {
                outcome: DaemonValidationOutcome::Unavailable,
                claim: None,
            },
            &FixedEnforcement(EnforcementMode::Interrupt),
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

    /// RMCPF-040: `detail: full` returns the complete pre-ergonomics envelope.
    /// Default selection is covered by `resolve_response_detail_with` to avoid
    /// inheriting `ANVIL_MCP_VALIDATE_DETAIL` from the process environment.
    #[test]
    fn clean_allow_full_detail_is_full_envelope() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "proposedContent": "export const value = 1;\n",
                "detail": "full"
            }),
        );

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["schema"], super::RESPONSE_SCHEMA);
        assert_eq!(payload["summary"]["total"], 0);
        assert_eq!(payload["diagnostics"], json!([]));
        assert!(payload.get("correlation").is_some());
        assert!(payload.get("tier").is_some());
    }

    /// RMCPF-040: `detail: minimal` on clean allow returns only schema
    /// and decision — no empty diagnostics, summary, correlation, or tier.
    #[test]
    fn clean_allow_minimal_detail_omits_empty_envelope_fields() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "proposedContent": "export const value = 1;\n",
                "detail": "minimal"
            }),
        );

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["schema"], super::RESPONSE_SCHEMA);
        let keys: std::collections::BTreeSet<&str> = payload
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            ["decision", "schema"].into_iter().collect(),
            "minimal allow must be schema+decision only, got {payload}"
        );
    }

    /// RMCPF-040: veto paths ignore minimal detail — agents still need
    /// diagnostics and safeDefault.
    #[test]
    fn block_keeps_full_payload_under_minimal_detail() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/secret.ts",
                "operation": "update",
                "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n",
                "detail": "minimal"
            }),
        );

        assert_ne!(payload["decision"], "allow");
        assert!(payload.get("diagnostics").is_some());
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert!(payload.get("correlation").is_some());
    }

    #[test]
    fn apply_response_detail_leaves_non_allow_unchanged() {
        let mut payload = json!({
            "schema": super::RESPONSE_SCHEMA,
            "decision": "warn",
            "diagnostics": [{"id": "x"}],
            "summary": { "total": 1 }
        });
        super::apply_response_detail(&mut payload, super::ResponseDetail::Minimal);
        assert_eq!(payload["decision"], "warn");
        assert_eq!(payload["diagnostics"][0]["id"], "x");
    }

    #[test]
    fn resolve_response_detail_request_beats_env() {
        let mut map = serde_json::Map::new();
        map.insert("detail".into(), json!("full"));
        assert_eq!(
            super::resolve_response_detail_with(&map, Some("minimal")),
            super::ResponseDetail::Full
        );
        map.insert("detail".into(), json!("minimal"));
        assert_eq!(
            super::resolve_response_detail_with(&map, Some("full")),
            super::ResponseDetail::Minimal
        );
        map.clear();
        assert_eq!(
            super::resolve_response_detail_with(&map, Some("minimal")),
            super::ResponseDetail::Minimal
        );
        assert_eq!(
            super::resolve_response_detail_with(&map, None),
            super::ResponseDetail::Minimal,
            "RMCPF-043: default detail is minimal"
        );
        assert_eq!(
            super::resolve_response_detail_with(&map, Some("full")),
            super::ResponseDetail::Full
        );
    }

    /// RMCPF-043: omitting `detail` yields the lean allow envelope.
    #[test]
    fn clean_allow_default_detail_is_minimal_envelope() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload_preserving_detail(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "operation": "update",
                "proposedContent": "export const value = 1;\n"
            }),
            false,
        );

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["schema"], super::RESPONSE_SCHEMA);
        let keys: std::collections::BTreeSet<&str> = payload
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            ["decision", "schema"].into_iter().collect(),
            "default allow must be schema+decision only, got {payload}"
        );
    }
}
