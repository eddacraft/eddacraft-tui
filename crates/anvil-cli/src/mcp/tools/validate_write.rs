use std::fs;
use std::path::{Component, Path, PathBuf};

use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};
use serde_json::{Value, json};

use crate::mcp::validation::{
    DaemonValidationClient, LocalDaemonValidationClient, PreWriteValidationRequest,
    ValidationBackend, ValidationBackendFailure, validate_pre_write,
};

pub const TOOL_NAME: &str = "anvil_validate_write";

const RESPONSE_SCHEMA: &str = "anvil.mcp.validate-write.v1";
const MAX_PROPOSED_CONTENT_BYTES: usize = 1024 * 1024;
const INPUT_RULE_ID: &str = "mcp-validate-write-input";
const PRE_WRITE_MODE: &str = "pre-write";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Validate a proposed file write before the MCP client applies it.",
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
                "contentEncoding": {
                    "type": "string",
                    "enum": ["utf-8", "base64"],
                    "default": "utf-8"
                },
                "client": {
                    "type": "object",
                    "additionalProperties": true
                }
            },
            "required": ["path", "operation"],
            "additionalProperties": true
        }
    })
}

pub fn call(arguments: &Value) -> Value {
    let default_workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    call_with_workspace(arguments, &default_workspace_root)
}

fn call_with_workspace(arguments: &Value, default_workspace_root: &Path) -> Value {
    call_with_validation_client(
        arguments,
        default_workspace_root,
        &LocalDaemonValidationClient,
    )
}

fn call_with_validation_client(
    arguments: &Value,
    default_workspace_root: &Path,
    daemon: &impl DaemonValidationClient,
) -> Value {
    let request = match ValidateWriteRequest::parse(arguments, default_workspace_root) {
        Ok(request) => request,
        Err(problem) => return tool_result(&problem_payload(problem, None)),
    };

    if let Some(problem) = request.input_problem() {
        return tool_result(&problem_payload(problem, Some(&request.relative_path)));
    }

    let mut backend = ValidationBackend::Embedded;
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
                return tool_result(&backend_failure_payload(&request.relative_path, failure));
            }
        };
        backend = validation.backend;
        diagnostics = validation.diagnostics;
    }

    tool_result(&validation_payload(
        &request.relative_path,
        &diagnostics,
        backend,
        None,
    ))
}

fn tool_result(payload: &Value) -> Value {
    let is_error = payload["decision"] == "block";
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

fn problem_payload(problem: ToolProblem, path: Option<&str>) -> Value {
    let path = path.unwrap_or("<unknown>");
    let diagnostic = input_diagnostic(problem, path);
    validation_payload(
        path,
        &[diagnostic],
        ValidationBackend::Embedded,
        Some(problem),
    )
}

fn backend_failure_payload(path: &str, failure: ValidationBackendFailure) -> Value {
    let diagnostic = input_diagnostic(ToolProblem::new(failure.code, failure.message), path);
    let mut payload = validation_payload(path, &[diagnostic], ValidationBackend::Daemon, None);
    payload["error"] = json!({
        "code": failure.code,
        "message": failure.message,
        "retriable": failure.retriable
    });
    payload
}

fn validation_payload(
    path: &str,
    diagnostics: &[Diagnostic],
    backend: ValidationBackend,
    problem: Option<ToolProblem>,
) -> Value {
    let decision = decision_for(diagnostics);
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
            "path": path
        }
    });

    if decision == "block" {
        payload["safeDefault"] = json!("do-not-write");
    }

    if let Some(problem) = problem {
        payload["error"] = json!({
            "code": problem.code,
            "message": problem.message,
            "retriable": false
        });
    }

    payload
}

fn diagnostic_summary(diagnostics: &[Diagnostic]) -> Value {
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

fn decision_for(diagnostics: &[Diagnostic]) -> &'static str {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        "block"
    } else if diagnostics.is_empty() {
        "allow"
    } else {
        "warn"
    }
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

fn correlation_id(path: &str) -> String {
    format!("corr_mcp_{}", sanitise_id_part(path))
}

fn sanitise_id_part(value: &str) -> String {
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
    relative_path: String,
    operation: Operation,
    content: Option<String>,
    patch_only: bool,
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
        let patch_content = optional_string(arguments.get("patch"))?;
        // Per the launch-shim contract, when both fields are supplied
        // `proposedContent` is authoritative and `patch` is correlation
        // metadata. Patch text is never scanned as file content because diff
        // hunks include removed lines and metadata that would mislead the
        // secret/reasoning checks.
        let content = proposed_content.map(str::to_string);
        let patch_only = content.is_none() && patch_content.is_some();

        Ok(Self {
            relative_path,
            operation,
            content,
            patch_only,
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

    use super::{MAX_PROPOSED_CONTENT_BYTES, call_with_validation_client, call_with_workspace};
    use super::{descriptor, parse_payload};
    use crate::mcp::validation::{
        DaemonValidationClient, DaemonValidationOutcome, PreWriteValidationRequest,
        ValidationBackendFailure,
    };
    use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};
    use serde_json::{Value, json};
    use tempfile::tempdir;

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
    fn descriptor_advertises_supported_content_encodings() {
        let descriptor = descriptor();

        assert_eq!(
            descriptor["inputSchema"]["properties"]["contentEncoding"]["enum"],
            json!(["utf-8", "base64"])
        );
    }

    #[test]
    fn clean_content_allows_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            json!({
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["summary"]["total"], 0);
        assert_eq!(payload["diagnostics"], json!([]));
    }

    #[test]
    fn secret_detection_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            json!({
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
            payload["diagnostics"][0]["source"]["rule_id"],
            "secret-detection"
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
        );
        let payload = parse_payload(&result);

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["correlation"]["backend"], "daemon");
        assert_eq!(
            payload["diagnostics"][0]["source"]["rule_id"],
            "secret-detection"
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
        );
        let payload = parse_payload(&result);

        assert_eq!(result["isError"], true);
        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "validation-backend-unavailable");
        assert_eq!(payload["error"]["retriable"], true);
        assert_eq!(payload["correlation"]["backend"], "daemon");
    }

    #[test]
    fn proposed_content_authoritative_when_patch_also_supplied() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
                "workspaceRoot": root_file.to_string_lossy(),
                "path": "src/example.ts",
                "operation": "create",
                "proposedContent": "export const value = 1;\n"
            }),
        );

        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "invalid-workspace-root");
    }

    fn call_payload(workspace_root: &std::path::Path, arguments: Value) -> Value {
        let result = call_with_workspace(&arguments, workspace_root);
        assert_eq!(result["content"][0]["type"], "text");
        parse_payload(&result)
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
}

#[cfg(test)]
fn parse_payload(result: &Value) -> Value {
    let text = result["content"][0]["text"]
        .as_str()
        .expect("tool result contains JSON text");
    serde_json::from_str(text).expect("tool result text is JSON")
}
