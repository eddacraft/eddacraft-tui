use std::path::{Path, PathBuf};

use anvil_kernel_types::diagnostics::ControlDecision;
use serde_json::{Value, json};

use crate::mcp::enforcement::{self, EnforcementMode, MCP_DEFAULT_ENFORCEMENT};
use crate::mcp::tools::shared::{
    WorkspacePathKind, normalise_workspace_relative_path, validate_workspace_root,
};
use crate::mcp::tools::validate_write::{
    correlation_id, diagnostic_summary, normalise_response_diagnostics,
};
use crate::mcp::validation::{
    DaemonStatus, DaemonValidationClient, LocalDaemonValidationClient, PreWriteValidationRequest,
    ValidationBackend, ValidationBackendFailure, validate_pre_write,
};

pub const TOOL_NAME: &str = "anvil_apply_patch";
const RESPONSE_SCHEMA: &str = "anvil.mcp.validate-write.v1";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Validate a unified diff before applying it. Scans added lines for secrets and policy violations. Honour `block` decisions; do not apply patches the tool refuses.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute workspace root. Defaults to the server cwd when omitted."
                },
                "path": {
                    "type": "string",
                    "description": "Workspace-relative path of the file being patched."
                },
                "unifiedDiff": {
                    "type": "string",
                    "description": "Unified diff to validate (--- / +++ / @@ format). Only added lines are scanned; removed lines are ignored."
                },
                "expectedSha256": {
                    "type": ["string", "null"],
                    "description": "Optional SHA-256 hex digest of the current file for integrity verification by the caller."
                }
            },
            "required": ["path", "unifiedDiff"],
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": false,
            "openWorldHint": false
        }
    })
}

pub fn call(arguments: &Value) -> Value {
    let default_workspace_root = match std::env::current_dir() {
        Ok(root) => root,
        Err(err) => {
            return tool_result(&json!({
                "schema": RESPONSE_SCHEMA,
                "decision": "block",
                "error": {
                    "code": "server-cwd-unavailable",
                    "message": format!("MCP server cwd is not accessible: {err}"),
                    "retriable": false
                },
                "safeDefault": "do-not-write",
                "correlation": {
                    "id": "corr_mcp_apply_patch",
                    "surface": "mcp",
                    "mode": "preWrite",
                    "backend": ValidationBackend::Embedded.as_str(),
                    "daemonStatus": DaemonStatus::NotWired.as_str(),
                    "path": "<server-cwd>",
                    "enforcementMode": MCP_DEFAULT_ENFORCEMENT.as_str()
                }
            }));
        }
    };
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
    let request = match ApplyPatchRequest::parse(arguments, default_workspace_root) {
        Ok(request) => request,
        Err(message) => {
            return tool_result(&input_error_payload(
                &message,
                "<unknown>",
                MCP_DEFAULT_ENFORCEMENT,
            ));
        }
    };

    let enforcement_mode = enforcement::load_for_workspace(&request.workspace_root);
    let added_lines = extract_added_lines(&request.diff);

    let mut backend = ValidationBackend::Embedded;
    let mut daemon_status = DaemonStatus::NotWired;
    let mut diagnostics = Vec::new();

    if !added_lines.is_empty() {
        let validation = validate_pre_write(
            &PreWriteValidationRequest {
                relative_path: &request.relative_path,
                content: &added_lines,
            },
            daemon,
        );
        match validation {
            Ok(result) => {
                backend = result.backend;
                daemon_status = result.daemon_status;
                diagnostics = result.diagnostics;
            }
            Err(failure) => {
                return tool_result(&backend_failure_payload(
                    &request.relative_path,
                    failure,
                    enforcement_mode,
                ));
            }
        }
    }

    let diagnostics = normalise_response_diagnostics(&diagnostics, backend);
    let decision = enforcement::decision_for(&diagnostics, enforcement_mode);

    let mut payload = json!({
        "schema": RESPONSE_SCHEMA,
        "decision": decision,
        "summary": diagnostic_summary(&diagnostics),
        "diagnostics": diagnostics,
        "correlation": {
            "id": correlation_id(&request.relative_path),
            "surface": "mcp",
            "mode": "preWrite",
            "backend": backend.as_str(),
            "daemonStatus": daemon_status.as_str(),
            "path": &request.relative_path,
            "enforcementMode": enforcement_mode.as_str()
        }
    });

    // ADR-098 AD-3 amendment 1: any veto (block / fence / interrupt), not
    // just `block`, sets the do-not-write safe default.
    if decision.is_veto() {
        payload["safeDefault"] = json!("do-not-write");
    }

    tool_result(&payload)
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
    let text = serde_json::to_string(payload).expect("apply-patch payload serialises");
    json!({
        "content": [{"type": "text", "text": text}],
        "isError": is_error
    })
}

fn input_error_payload(message: &str, path: &str, enforcement_mode: EnforcementMode) -> Value {
    json!({
        "schema": RESPONSE_SCHEMA,
        "decision": "block",
        "error": {
            "code": "invalid-tool-arguments",
            "message": message,
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
            "enforcementMode": enforcement_mode.as_str()
        }
    })
}

fn backend_failure_payload(
    path: &str,
    failure: ValidationBackendFailure,
    enforcement_mode: EnforcementMode,
) -> Value {
    json!({
        "schema": RESPONSE_SCHEMA,
        "decision": "block",
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

/// Extract the content of added lines from a unified diff. Lines beginning
/// with `+` (excluding the `+++` file header) are the additions; the leading
/// `+` is stripped so the result is scannable as file content.
fn extract_added_lines(diff: &str) -> String {
    diff.lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .map(|line| &line[1..])
        .collect::<Vec<_>>()
        .join("\n")
}

struct ApplyPatchRequest {
    workspace_root: PathBuf,
    relative_path: String,
    diff: String,
}

impl ApplyPatchRequest {
    fn parse(arguments: &Value, default_workspace_root: &Path) -> Result<Self, String> {
        let Some(arguments) = arguments.as_object() else {
            return Err("Apply-patch arguments must be an object.".to_string());
        };

        let workspace_root = resolve_workspace_root(
            arguments.get("workspaceRoot").and_then(Value::as_str),
            default_workspace_root,
        )?;

        let path = arguments
            .get("path")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Apply-patch requires a non-empty path.".to_string())?;

        let diff = arguments
            .get("unifiedDiff")
            .and_then(Value::as_str)
            .ok_or_else(|| "Apply-patch requires unifiedDiff.".to_string())?;

        let relative_path =
            normalise_workspace_relative_path("Path", path, WorkspacePathKind::HostFilesystem)?;

        Ok(Self {
            workspace_root,
            relative_path,
            diff: diff.to_string(),
        })
    }
}

fn resolve_workspace_root(
    workspace_root_arg: Option<&str>,
    default_workspace_root: &Path,
) -> Result<PathBuf, String> {
    match workspace_root_arg {
        None => default_workspace_root
            .canonicalize()
            .map_err(|err| format!("MCP server cwd is not accessible: {err}")),
        Some(root) => validate_workspace_root(Path::new(root), default_workspace_root)
            .map(|(_, workspace)| workspace),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::mcp::validation::{
        DaemonValidationClient, DaemonValidationOutcome, PreWriteValidationRequest,
    };
    use serde_json::json;
    use tempfile::tempdir;

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

    fn call_payload(workspace_root: &Path, arguments: &Value) -> Value {
        let result = call_with_validation_client(
            arguments,
            workspace_root,
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
        );
        assert_eq!(result["content"][0]["type"], "text");
        let text = result["content"][0]["text"]
            .as_str()
            .expect("tool result text");
        serde_json::from_str(text).expect("tool result JSON")
    }

    #[test]
    fn clean_diff_allows_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "unifiedDiff": "@@ -1,3 +1,4 @@\n const a = 1;\n+const b = 2;\n const c = 3;\n"
            }),
        );

        assert_eq!(payload["decision"], "allow");
        assert_eq!(payload["schema"], "anvil.mcp.validate-write.v1");
        assert_eq!(payload["summary"]["total"], 0);
    }

    #[test]
    fn host_normalised_path_reaches_correlation_and_daemon() {
        let workspace = tempdir().expect("workspace exists");
        let daemon = RecordingDaemon {
            seen_path: RefCell::new(None),
        };
        let result = call_with_validation_client(
            &json!({
                "path": "./src//x.ts",
                "unifiedDiff": "+const x = 1;\n"
            }),
            workspace.path(),
            &daemon,
        );
        let text = result["content"][0]["text"].as_str().expect("text");
        let payload: Value = serde_json::from_str(text).expect("JSON");

        assert_eq!(payload["correlation"]["path"], "src/x.ts");
        assert_eq!(daemon.seen_path.borrow().as_deref(), Some("src/x.ts"));
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
                "path": r"src/a\b.ts",
                "unifiedDiff": "+const x = 1;\n"
            }),
            workspace.path(),
            &daemon,
        );
        let text = result["content"][0]["text"].as_str().expect("text");
        let payload: Value = serde_json::from_str(text).expect("JSON");

        assert_eq!(payload["correlation"]["path"], r"src/a\b.ts");
        assert_eq!(daemon.seen_path.borrow().as_deref(), Some(r"src/a\b.ts"));
    }

    #[test]
    fn secret_in_added_lines_blocks_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/secret.ts",
                "unifiedDiff": "@@ -1 +1 @@\n-const old = 1;\n+const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(payload["decision"], "interrupt");
        assert_eq!(payload["safeDefault"], "do-not-write");
        assert_eq!(payload["summary"]["bySeverity"]["error"], 1);
    }

    #[test]
    fn secret_in_removed_lines_only_allows_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({
                "path": "src/example.ts",
                "unifiedDiff": "@@ -1 +0,0 @@\n-const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
            }),
        );

        assert_eq!(payload["decision"], "allow");
    }

    #[test]
    fn empty_diff_allows_write() {
        let workspace = tempdir().expect("workspace exists");
        let payload = call_payload(
            workspace.path(),
            &json!({"path": "src/example.ts", "unifiedDiff": ""}),
        );

        assert_eq!(payload["decision"], "allow");
    }

    #[test]
    fn missing_path_is_rejected() {
        let workspace = tempdir().expect("workspace exists");
        let result = call_with_validation_client(
            &json!({"unifiedDiff": "@@ -1 +1 @@\n+const x = 1;\n"}),
            workspace.path(),
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
        );
        let text = result["content"][0]["text"].as_str().expect("text");
        let payload: Value = serde_json::from_str(text).expect("JSON");
        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "invalid-tool-arguments");
        assert_eq!(
            payload["error"]["message"],
            "Apply-patch requires a non-empty path."
        );
    }

    #[test]
    fn path_escape_is_rejected() {
        let workspace = tempdir().expect("workspace exists");
        let result = call_with_validation_client(
            &json!({"path": "../escape.ts", "unifiedDiff": "+const x = 1;\n"}),
            workspace.path(),
            &FixtureDaemon {
                outcome: DaemonValidationOutcome::Unavailable,
            },
        );
        let text = result["content"][0]["text"].as_str().expect("text");
        let payload: Value = serde_json::from_str(text).expect("JSON");
        assert_eq!(payload["decision"], "block");
        assert_eq!(payload["error"]["code"], "invalid-tool-arguments");
    }

    #[test]
    fn portable_path_hazards_are_invalid_tool_arguments() {
        let workspace = tempdir().expect("workspace exists");

        for path in [
            r"C:\outside.ts",
            "./C:/outside.ts",
            "./C:relative",
            r"\\server\share\outside.ts",
            r"src\..\outside.ts",
            "src/evil\0name.ts",
        ] {
            let result = call_with_validation_client(
                &json!({"path": path, "unifiedDiff": "+const x = 1;\n"}),
                workspace.path(),
                &FixtureDaemon {
                    outcome: DaemonValidationOutcome::Unavailable,
                },
            );
            let text = result["content"][0]["text"].as_str().expect("text");
            let payload: Value = serde_json::from_str(text).expect("JSON");

            assert_eq!(
                payload["error"]["code"], "invalid-tool-arguments",
                "path {path:?} should be rejected"
            );
        }
    }

    #[test]
    fn descriptor_has_destructive_annotation() {
        let d = descriptor();
        assert_eq!(d["annotations"]["destructiveHint"], true);
        assert_eq!(d["annotations"]["readOnlyHint"], false);
    }

    #[test]
    fn extract_added_lines_ignores_removed_and_context() {
        let diff =
            "--- a/f.ts\n+++ b/f.ts\n@@ -1,3 +1,3 @@\n context\n-removed\n+added\n context2\n";
        assert_eq!(super::extract_added_lines(diff), "added");
    }

    #[test]
    fn extract_added_lines_strips_leading_plus() {
        let diff = "+const x = 1;\n++double_plus;\n";
        let result = super::extract_added_lines(diff);
        assert!(result.contains("const x = 1;"));
        assert!(result.contains("+double_plus;"));
    }
}
