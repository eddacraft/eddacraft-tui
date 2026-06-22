use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use anvil_checks::antipattern::{AntipatternCheckConfig, WarningSeverity, run_antipattern_check};

use crate::mcp::tools::shared::{
    build_warnings_array, collect_relative_files, redact_workspace_root, resolve_workspace_files,
    validate_workspace_root,
};
use crate::mcp::validation::DaemonStatus;

pub const TOOL_NAME: &str = "anvil_gate";

/// Hard cap on the `anvil gate --json` subprocess. Full-mode gate runs
/// trigger npm audit, OPA evaluation, and coverage reads, which can take
/// several seconds on a real workspace. The cap stops a hung subprocess
/// from blocking the MCP server indefinitely while still allowing a
/// realistic gate pass to complete.
const FULL_GATE_TIMEOUT: Duration = Duration::from_mins(2);

/// Per-stream cap on subprocess stdout/stderr capture. The gate output is
/// structured JSON whose realistic upper bound is well under a megabyte;
/// the 16 MiB cap defends against a pathological or compromised plugin
/// flooding the MCP server with output before the timeout fires.
const FULL_GATE_STREAM_CAP: usize = 16 * 1024 * 1024;

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Run the anvil quality gate. Supply targetFiles for a planless antipattern scan, or omit for a full config-driven gate run.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                },
                "targetFiles": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Specific files to analyse (planless mode). Omit for a full gate run."
                },
                "skipChecks": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Names of checks to skip during a full gate run"
                },
                "failFast": {
                    "type": "boolean",
                    "description": "Stop on the first failing check (full mode only)"
                }
            },
            "required": ["workspaceRoot"],
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": false
        }
    })
}

pub fn call(arguments: &Value) -> Value {
    let payload = match gate_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn gate_payload(arguments: &Value) -> Result<Value, String> {
    let server_root = std::env::current_dir()
        .map_err(|err| format!("MCP server cwd is not accessible: {err}"))?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot is required".to_string())?;

    let (server_root, workspace_path) =
        validate_workspace_root(Path::new(workspace_root), &server_root)?;

    let target_files = arguments
        .get("targetFiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if target_files.is_empty() {
        let skip_checks = arguments
            .get("skipChecks")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let fail_fast = arguments
            .get("failFast")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        run_full_gate(&workspace_path, &server_root, &skip_checks, fail_fast)
    } else {
        run_planless_gate(&workspace_path, &server_root, &target_files)
    }
}

fn run_planless_gate(
    workspace_path: &Path,
    server_root: &Path,
    target_files: &[Value],
) -> Result<Value, String> {
    let relative_files = collect_relative_files(target_files, "targetFiles")?;
    let absolute_paths = resolve_workspace_files(workspace_path, &relative_files, "targetFiles")?;
    let file_refs: Vec<&str> = absolute_paths.iter().map(String::as_str).collect();
    let workspace_str = workspace_path.to_string_lossy().to_string();

    let config = AntipatternCheckConfig {
        severity_threshold: WarningSeverity::Error,
        ..AntipatternCheckConfig::default()
    };

    let started = Instant::now();
    let result = run_antipattern_check(&file_refs, &config, Some(&workspace_str));
    let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    let warnings = build_warnings_array(&result.warnings.warnings);

    Ok(json!({
        "mode": "planless",
        "checksRun": ["antipattern"],
        "hasBlockingWarnings": !result.passed,
        "executionTimeMs": elapsed,
        "warnings": warnings,
        "summary": result.warnings.summary,
        "workspaceRoot": redact_workspace_root(workspace_path, server_root),
        "backend": "local",
        "daemonStatus": DaemonStatus::NotWired.as_str()
    }))
}

fn run_full_gate(
    workspace_path: &Path,
    server_root: &Path,
    skip_checks: &[Value],
    fail_fast: bool,
) -> Result<Value, String> {
    let exe = std::env::current_exe()
        .map_err(|err| format!("MCP host binary is not accessible: {err}"))?;

    let mut cmd = Command::new(&exe);
    cmd.current_dir(workspace_path)
        .arg("--no-tui")
        .arg("--json")
        .arg("gate");

    let mut skip_names = Vec::with_capacity(skip_checks.len());
    for (index, entry) in skip_checks.iter().enumerate() {
        let name = entry
            .as_str()
            .ok_or_else(|| format!("skipChecks[{index}] must be a string"))?;
        if name.is_empty() {
            return Err(format!("skipChecks[{index}] must not be empty"));
        }
        if name.contains(',') {
            return Err(format!(
                "skipChecks[{index}] must not contain commas; pass entries as separate items"
            ));
        }
        if name.contains('\0') {
            return Err(format!("skipChecks[{index}] must not contain NUL bytes"));
        }
        skip_names.push(name.to_string());
    }
    if !skip_names.is_empty() {
        cmd.arg("--skip-checks").arg(skip_names.join(","));
    }
    if fail_fast {
        cmd.arg("--fail-fast");
    }

    let started = Instant::now();
    let output = wait_with_timeout(cmd, FULL_GATE_TIMEOUT, FULL_GATE_STREAM_CAP)
        .map_err(|err| format!("anvil gate subprocess failed: {err}"))?;
    let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| format!("anvil gate stdout was not UTF-8: {err}"))?;
    let exit_status = output.status;

    let gate_result: Value = match serde_json::from_str(stdout.trim()) {
        Ok(value) => value,
        Err(err) => {
            return Err(format!(
                "anvil gate did not emit JSON (exit {:?}): {err}",
                exit_status.code()
            ));
        }
    };

    let overall = gate_result
        .get("overall")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Ok(json!({
        "mode": "full",
        "overall": overall,
        "score": gate_result.get("score").cloned().unwrap_or(Value::Null),
        "checks": gate_result.get("checks").cloned().unwrap_or(json!([])),
        "executionTimeMs": elapsed,
        "exitCode": exit_status.code(),
        "workspaceRoot": redact_workspace_root(workspace_path, server_root),
        "backend": "local",
        "daemonStatus": DaemonStatus::NotWired.as_str()
    }))
}

fn wait_with_timeout(
    mut cmd: Command,
    timeout: Duration,
    stream_cap: usize,
) -> std::io::Result<std::process::Output> {
    use std::process::Stdio;

    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let stdout = read_capped(child.stdout.take(), stream_cap)?;
            let stderr = read_capped(child.stderr.take(), stream_cap)?;
            return Ok(std::process::Output {
                status,
                stdout,
                stderr,
            });
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "anvil gate subprocess exceeded the MCP timeout",
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn read_capped<R: std::io::Read>(handle: Option<R>, cap: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;

    let Some(mut handle) = handle else {
        return Ok(Vec::new());
    };
    let mut buffer = Vec::new();
    // `cap + 1` lets us distinguish "exactly at cap" from "overflowed".
    let cap_plus_one = cap.saturating_add(1) as u64;
    let read = handle
        .by_ref()
        .take(cap_plus_one)
        .read_to_end(&mut buffer)?;
    if read > cap {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("anvil gate output exceeded {cap} byte cap"),
        ));
    }
    Ok(buffer)
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("gate payload serialises");
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
    fn descriptor_advertises_workspace_root_and_optional_target_files() {
        let descriptor = descriptor();
        assert_eq!(descriptor["name"], TOOL_NAME);
        let required = descriptor["inputSchema"]["required"]
            .as_array()
            .expect("required is an array");
        assert!(required.contains(&json!("workspaceRoot")));
        assert!(!required.contains(&json!("targetFiles")));
    }

    #[test]
    fn gate_rejects_missing_workspace_root() {
        let result = call(&json!({}));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "workspaceRoot is required");
    }

    #[test]
    fn gate_rejects_relative_workspace_root() {
        let result = call(&json!({ "workspaceRoot": "." }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "workspaceRoot must be an absolute path");
    }

    #[test]
    fn gate_rejects_workspace_outside_server_root() {
        let other = tempfile::tempdir().expect("foreign workspace exists");
        let result = call(&json!({
            "workspaceRoot": other.path(),
            "targetFiles": ["src/a.ts"]
        }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "workspaceRoot must be inside the MCP server root"
        );
    }

    #[test]
    fn gate_planless_mode_returns_parity_shape_for_clean_files() {
        let cwd = std::env::current_dir().unwrap();
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let src = workspace.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("clean.ts"), "export const value = 1;\n").unwrap();

        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "targetFiles": ["src/clean.ts"]
        }));

        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["mode"], "planless");
        assert_eq!(payload["hasBlockingWarnings"], false);
        assert_eq!(payload["summary"]["total"], 0);
        assert_eq!(payload["checksRun"], json!(["antipattern"]));
        assert_eq!(payload["backend"], "local");
        assert_eq!(payload["daemonStatus"], "not-wired");
    }

    #[test]
    fn gate_planless_rejects_absolute_target_file_entries() {
        let cwd = std::env::current_dir().unwrap();
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let abs = workspace.path().join("src/a.ts").display().to_string();

        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "targetFiles": [abs]
        }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "targetFiles[0] must be a workspace-relative path"
        );
    }

    #[test]
    fn gate_planless_rejects_skip_check_entries_with_commas() {
        let cwd = std::env::current_dir().unwrap();
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");

        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "skipChecks": ["lint,test"]
        }));

        // skipChecks is only consulted in full mode (no targetFiles). The
        // payload should bubble up the validation error rather than running
        // the subprocess with an injected flag.
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "skipChecks[0] must not contain commas; pass entries as separate items"
        );
    }

    #[test]
    fn gate_planless_rejects_skip_check_entries_with_nul_bytes() {
        let cwd = std::env::current_dir().unwrap();
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");

        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "skipChecks": ["lint\u{0}test"]
        }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "skipChecks[0] must not contain NUL bytes");
    }

    #[cfg(unix)]
    #[test]
    fn gate_planless_rejects_symlink_targets_outside_workspace() {
        let cwd = std::env::current_dir().unwrap();
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let outside_dir = tempfile::tempdir_in(&cwd).expect("outside dir exists");
        let outside_file = outside_dir.path().join("secret.txt");
        std::fs::write(&outside_file, "shh").expect("outside file writes");

        let link = workspace.path().join("escape.ts");
        std::os::unix::fs::symlink(&outside_file, &link).expect("symlink created");

        let result = call(&json!({
            "workspaceRoot": workspace.path(),
            "targetFiles": ["escape.ts"]
        }));

        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "targetFiles[0] resolves outside workspaceRoot"
        );
    }
}
