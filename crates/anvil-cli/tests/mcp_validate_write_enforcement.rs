//! RTAI-006 E2E: drive `anvil mcp serve --stdio` with `.anvil.yaml`
//! enforcement-mode fixtures and assert the `validate_write` tool
//! response honours `block` / `warn` / `off` semantics.
//!
//! These tests spawn the real Rust MCP shim, send a JSON-RPC tools/call
//! frame whose `proposedContent` triggers the secret-detection rule,
//! and check the structured response per the mapping table documented
//! in `crates/anvil-cli/src/mcp/enforcement.rs`.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tempfile::TempDir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const CHILD_TIMEOUT: Duration = Duration::from_secs(5);
const SECRET_PROPOSED_CONTENT: &str = "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n";

#[test]
fn enforcement_mode_block_rejects_secret_write_e2e() {
    let workspace = workspace_with_enforcement_mode("block");
    let payload = run_validate_write_against(workspace.path(), SECRET_PROPOSED_CONTENT);

    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["result"]["isError"], true);

    let tool = parse_tool_payload(&payload);
    assert_eq!(tool["decision"], "block");
    assert_eq!(tool["safeDefault"], "do-not-write");
    assert_eq!(tool["correlation"]["enforcementMode"], "block");
    assert_eq!(tool["summary"]["bySeverity"]["error"], 1);
    assert_eq!(tool["diagnostics"][0]["category"], "secret");
    assert_eq!(
        tool["diagnostics"][0]["source"]["rule_id"],
        "secret-detection"
    );
}

#[test]
fn enforcement_mode_warn_returns_warn_decision_with_diagnostics_e2e() {
    let workspace = workspace_with_enforcement_mode("warn");
    let payload = run_validate_write_against(workspace.path(), SECRET_PROPOSED_CONTENT);

    assert_eq!(payload["jsonrpc"], "2.0");
    // `warn` is a non-error decision; the JSON-RPC result is not an error.
    assert_eq!(payload["result"]["isError"], false);

    let tool = parse_tool_payload(&payload);
    assert_eq!(tool["decision"], "warn");
    assert!(tool.get("safeDefault").is_none());
    assert_eq!(tool["correlation"]["enforcementMode"], "warn");
    // The diagnostic still rides through so the agent can render it.
    assert_eq!(tool["summary"]["bySeverity"]["error"], 1);
    assert_eq!(tool["diagnostics"][0]["category"], "secret");
    assert_eq!(
        tool["diagnostics"][0]["source"]["rule_id"],
        "secret-detection"
    );
}

#[test]
fn enforcement_mode_off_returns_allow_decision_with_diagnostics_e2e() {
    let workspace = workspace_with_enforcement_mode("off");
    let payload = run_validate_write_against(workspace.path(), SECRET_PROPOSED_CONTENT);

    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["result"]["isError"], false);

    let tool = parse_tool_payload(&payload);
    assert_eq!(tool["decision"], "allow");
    assert!(tool.get("safeDefault").is_none());
    assert_eq!(tool["correlation"]["enforcementMode"], "off");
    // Even in `off` mode the agent gets to see the would-have-blocked
    // diagnostics — RTAI-006 is "decision off", not "diagnostics off".
    assert_eq!(tool["summary"]["bySeverity"]["error"], 1);
    assert_eq!(tool["diagnostics"][0]["category"], "secret");
    assert_eq!(
        tool["diagnostics"][0]["source"]["rule_id"],
        "secret-detection"
    );
}

#[test]
fn missing_anvil_yaml_defaults_to_block_e2e() {
    let workspace = tempfile::tempdir().expect("workspace exists");
    let payload = run_validate_write_against(workspace.path(), SECRET_PROPOSED_CONTENT);

    let tool = parse_tool_payload(&payload);
    assert_eq!(tool["decision"], "block");
    assert_eq!(tool["correlation"]["enforcementMode"], "block");
}

fn workspace_with_enforcement_mode(mode: &str) -> TempDir {
    let workspace = tempfile::tempdir().expect("workspace exists");
    let yaml = format!("enforcement:\n  mode: {mode}\n");
    fs::write(workspace.path().join(".anvil.yaml"), yaml).expect("write enforcement fixture");
    workspace
}

fn run_validate_write_against(workspace_root: &Path, proposed_content: &str) -> Value {
    let mut child = spawn_mcp_server(workspace_root);
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    let request = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": {
            "name": "anvil_validate_write",
            "arguments": {
                "path": "src/secret.ts",
                "operation": "create",
                "proposedContent": proposed_content
            }
        }
    });

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(stdin, "{request}").expect("send tools/call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after tools/call and EOF; status: {status:?}",
    );

    serde_json::from_str(line.trim())
        .unwrap_or_else(|err| panic!("response must be JSON-RPC JSON, got {line:?}\nerror: {err}"))
}

fn spawn_mcp_server(workspace_root: &Path) -> Child {
    Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("mcp")
        .arg("serve")
        .arg("--stdio")
        // The MCP shim resolves `.anvil.yaml` from the server cwd when
        // `workspaceRoot` is omitted — see
        // `validate_write::call::default_workspace_root`. Anchoring the
        // child cwd in the temp workspace is the cleanest way to drive
        // each enforcement mode without a per-call workspaceRoot field
        // (which the trust check rejects unless it matches the cwd).
        .current_dir(workspace_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn anvil mcp serve --stdio")
}

fn spawn_stdout_reader(stdout: ChildStdout) -> Receiver<std::io::Result<String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let result = reader.read_line(&mut line).map(|_| line);
        let _ = tx.send(result);
    });
    rx
}

fn recv_stdout_line(child: &mut Child, rx: &Receiver<std::io::Result<String>>) -> String {
    match rx.recv_timeout(CHILD_TIMEOUT) {
        Ok(Ok(line)) => line,
        Ok(Err(err)) => panic!("failed to read child stdout: {err}"),
        Err(err) => {
            kill_child(child);
            panic!("timed out waiting for child stdout: {err}");
        }
    }
}

fn parse_tool_payload(parsed: &Value) -> Value {
    let text = parsed["result"]["content"][0]["text"]
        .as_str()
        .expect("tool result must contain a JSON text content item");
    serde_json::from_str(text).expect("tool result text must be JSON")
}

fn wait_for_exit(child: &mut Child) -> ExitStatus {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) if started.elapsed() <= CHILD_TIMEOUT => {
                thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                kill_child(child);
                panic!("timed out waiting for child exit");
            }
            Err(err) => {
                kill_child(child);
                panic!("failed to wait for child exit: {err}");
            }
        }
    }
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
