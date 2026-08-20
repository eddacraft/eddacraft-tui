use std::path::Path;
use std::time::Instant;

use serde_json::{Value, json};

use anvil_checks::antipattern::{AntipatternCheckConfig, WarningSeverity, run_antipattern_check};

use crate::mcp::tools::shared::{
    build_warnings_array, collect_relative_files, redact_workspace_root, resolve_workspace_files,
    validate_workspace_root,
};
use crate::mcp::validation::DaemonStatus;

pub const TOOL_NAME: &str = "anvil_check";

const SUPPORTED_CHECKS: &[&str] = &["antipattern"];

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Validate files against anvil antipattern rules. Returns warnings with locations, severity, and suggestions. Architecture check parity is deferred to a follow-up slice.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "File paths to check (relative to workspaceRoot)"
                },
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                },
                "checks": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["architecture", "antipattern"]
                    },
                    "description": "Which checks to run. Phase 1 honours \"antipattern\" only; other entries are ignored."
                }
            },
            "required": ["files", "workspaceRoot"],
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true
        }
    })
}

pub fn call(arguments: &Value) -> Value {
    let payload = match check_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn check_payload(arguments: &Value) -> Result<Value, String> {
    let server_root = std::env::current_dir()
        .map_err(|err| format!("MCP server cwd is not accessible: {err}"))?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot is required".to_string())?;
    let files = arguments
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "files is required".to_string())?;

    let (server_root, workspace_path) =
        validate_workspace_root(Path::new(workspace_root), &server_root)?;

    let relative_files = collect_relative_files(files, "files")?;
    let absolute_paths = resolve_workspace_files(&workspace_path, &relative_files, "files")?;
    let workspace_str = workspace_path.to_string_lossy().to_string();
    let file_refs: Vec<&str> = absolute_paths.iter().map(String::as_str).collect();

    let config = AntipatternCheckConfig {
        severity_threshold: WarningSeverity::Error,
        ..AntipatternCheckConfig::default()
    };

    let started = Instant::now();
    let result = run_antipattern_check(&file_refs, &config, Some(&workspace_str));
    let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    let warnings = build_warnings_array(&result.warnings.warnings);

    Ok(json!({
        "warnings": warnings,
        "summary": result.warnings.summary,
        "executionTimeMs": elapsed,
        "checksRun": SUPPORTED_CHECKS,
        "hasBlockingWarnings": !result.passed,
        "workspaceRoot": redact_workspace_root(&workspace_path, &server_root),
        "backend": "local",
        "daemonStatus": DaemonStatus::NotWired.as_str()
    }))
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("check payload serialises");
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": payload.get("error").is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_advertises_files_and_workspace_root() {
        let descriptor = descriptor();
        assert_eq!(descriptor["name"], TOOL_NAME);
        assert_eq!(descriptor["inputSchema"]["type"], "object");
        let required = descriptor["inputSchema"]["required"]
            .as_array()
            .expect("required is an array");
        assert!(required.contains(&json!("files")));
        assert!(required.contains(&json!("workspaceRoot")));
    }

    #[test]
    fn check_rejects_missing_workspace_root() {
        let result = call(&json!({ "files": ["src/a.ts"] }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "workspaceRoot is required");
    }

    #[test]
    fn check_rejects_missing_files() {
        let cwd = std::env::current_dir().unwrap();
        let result = call(&json!({ "workspaceRoot": cwd }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "files is required");
    }

    #[test]
    fn check_rejects_relative_workspace_root() {
        let result = call(&json!({ "workspaceRoot": ".", "files": [] }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "workspaceRoot must be an absolute path");
    }

    #[test]
    fn check_rejects_absolute_file_entries() {
        let cwd = std::env::current_dir().unwrap();
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let abs = workspace.path().join("src/a.ts").display().to_string();

        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "files": [abs]
        }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "files[0] must be a workspace-relative path"
        );
    }

    #[test]
    fn check_rejects_parent_dir_escapes() {
        let cwd = std::env::current_dir().unwrap();
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");

        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "files": ["../etc/passwd"]
        }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "files[0] must not escape the workspace via \"..\""
        );
    }

    #[test]
    fn check_returns_clean_payload_for_clean_files() {
        let cwd = std::env::current_dir().unwrap();
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let src = workspace.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("clean.ts"), "export const value = 1;\n").unwrap();

        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "files": ["src/clean.ts"]
        }));

        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["hasBlockingWarnings"], false);
        assert_eq!(payload["summary"]["total"], 0);
        assert_eq!(payload["warnings"], json!([]));
        assert_eq!(payload["backend"], "local");
        assert_eq!(payload["daemonStatus"], "not-wired");
        assert_eq!(payload["checksRun"], json!(["antipattern"]));
    }

    #[test]
    fn check_rejects_workspace_outside_server_root() {
        let other = tempfile::tempdir().expect("foreign workspace exists");
        let result = call(&json!({
            "workspaceRoot": other.path(),
            "files": []
        }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            crate::mcp::tools::shared::WORKSPACE_ROOT_NOT_ADMITTED
        );
    }

    #[cfg(unix)]
    #[test]
    fn check_rejects_symlink_targets_outside_workspace() {
        let cwd = std::env::current_dir().unwrap();
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let outside_dir = tempfile::tempdir_in(&cwd).expect("outside dir exists");
        let outside_file = outside_dir.path().join("secret.txt");
        std::fs::write(&outside_file, "shh").expect("outside file writes");

        let link = workspace.path().join("escape.ts");
        std::os::unix::fs::symlink(&outside_file, &link).expect("symlink created");

        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "files": ["escape.ts"]
        }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "files[0] resolves outside workspaceRoot");
    }
}
