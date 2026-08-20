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

    // Drain stdout/stderr on dedicated threads while the child runs. Waiting
    // for exit *before* reading either pipe deadlocks once the child fills an
    // OS pipe buffer (~64 KiB): the child blocks in write(2) and never exits,
    // so the parent only ever reports a timeout. Cap each stream during the
    // drain so a flood cannot OOM the MCP host.
    let mut stdout_handle = child
        .stdout
        .take()
        .expect("stdout is piped above; taken once");
    let mut stderr_handle = child
        .stderr
        .take()
        .expect("stderr is piped above; taken once");
    let stdout_reader =
        std::thread::spawn(move || read_capped_stream(&mut stdout_handle, stream_cap));
    let stderr_reader =
        std::thread::spawn(move || read_capped_stream(&mut stderr_handle, stream_cap));

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            // Child death closes the write ends and unblocks the readers.
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "anvil gate subprocess exceeded the MCP timeout",
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    };

    // Join both drain threads before propagating either stream error so a
    // cap overflow / I/O failure on one side cannot leave the other reader
    // detached (and still holding the pipe open).
    let stdout_result = stdout_reader
        .join()
        .map_err(|_| std::io::Error::other("stdout reader thread panicked"))?;
    let stderr_result = stderr_reader
        .join()
        .map_err(|_| std::io::Error::other("stderr reader thread panicked"))?;
    let stdout = stdout_result?;
    let stderr = stderr_result?;

    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

/// Read up to `cap` bytes from `handle`. If the stream exceeds the cap, keep
/// draining (discard) so a full pipe cannot wedge the writer, then error.
fn read_capped_stream<R: std::io::Read>(handle: &mut R, cap: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;

    let mut buffer = Vec::new();
    // `cap + 1` lets us distinguish "exactly at cap" from "overflowed".
    let cap_plus_one = cap.saturating_add(1) as u64;
    let read = handle
        .by_ref()
        .take(cap_plus_one)
        .read_to_end(&mut buffer)?;
    if read > cap {
        // Discard the remainder so the child is not stuck writing into a
        // full OS pipe after we stop accumulating.
        let mut sink = std::io::sink();
        let _ = std::io::copy(handle, &mut sink);
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
            crate::mcp::tools::shared::WORKSPACE_ROOT_NOT_ADMITTED
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

    /// Regression for the full-gate pipe deadlock: if the parent waited for
    /// exit before draining either pipe, a child that wrote more than an OS
    /// pipe buffer (~64 KiB) would block in write(2) and never exit, so the
    /// parent would only ever report a timeout.
    #[cfg(unix)]
    #[test]
    fn wait_with_timeout_drains_stdout_beyond_pipe_buffer() {
        // 256 KiB >> typical 64 KiB pipe capacity.
        let mut cmd = Command::new("dd");
        cmd.args(["if=/dev/zero", "bs=1024", "count=256", "status=none"]);

        let started = Instant::now();
        let output = wait_with_timeout(cmd, Duration::from_secs(5), FULL_GATE_STREAM_CAP)
            .expect("large stdout must not deadlock or time out");
        assert!(
            output.status.success(),
            "dd should exit 0, got {:?}",
            output.status
        );
        assert_eq!(output.stdout.len(), 256 * 1024);
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "concurrent drain must finish well under the timeout"
        );
    }

    #[cfg(unix)]
    #[test]
    fn wait_with_timeout_drains_stderr_beyond_pipe_buffer() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "dd if=/dev/zero bs=1024 count=256 status=none >&2"]);

        let started = Instant::now();
        let output = wait_with_timeout(cmd, Duration::from_secs(5), FULL_GATE_STREAM_CAP)
            .expect("large stderr must not deadlock or time out");
        assert!(output.status.success());
        assert_eq!(output.stderr.len(), 256 * 1024);
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[cfg(unix)]
    #[test]
    fn wait_with_timeout_drains_both_streams_beyond_pipe_buffer() {
        // Pure sh + dd so the regression does not depend on python3 being
        // present in every Unix CI image. Background a stderr writer, then
        // emit a full stdout buffer on the main shell.
        let mut cmd = Command::new("sh");
        cmd.args([
            "-c",
            "dd if=/dev/zero bs=1024 count=256 status=none >&2 & \
             dd if=/dev/zero bs=1024 count=256 status=none; \
             wait",
        ]);

        let output = wait_with_timeout(cmd, Duration::from_secs(5), FULL_GATE_STREAM_CAP)
            .expect("dual large streams must not deadlock");
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 256 * 1024);
        assert_eq!(output.stderr.len(), 256 * 1024);
    }

    #[cfg(unix)]
    #[test]
    fn wait_with_timeout_enforces_stream_cap_without_deadlock() {
        // Child writes well over a tiny cap; the reader must still drain so
        // the process can exit, then surface InvalidData.
        let mut cmd = Command::new("dd");
        cmd.args(["if=/dev/zero", "bs=1024", "count=128", "status=none"]);

        let err = wait_with_timeout(cmd, Duration::from_secs(5), 1024)
            .expect_err("output over cap must error");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(
            err.to_string().contains("exceeded 1024 byte cap"),
            "unexpected error: {err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn wait_with_timeout_kills_hung_child() {
        let mut cmd = Command::new("sleep");
        cmd.arg("30");

        let started = Instant::now();
        let err = wait_with_timeout(cmd, Duration::from_millis(300), FULL_GATE_STREAM_CAP)
            .expect_err("hung child must time out");
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "timeout path must not hang joining readers"
        );
    }
}
