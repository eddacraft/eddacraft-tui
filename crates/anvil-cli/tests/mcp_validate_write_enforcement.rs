//! RTAI-006 / ADR-098 AD-3 E2E: drive `anvil mcp serve --stdio` with
//! `.anvil.yaml` enforcement-mode fixtures and assert the `validate_write`
//! tool response honours the unified posture vocabulary
//! (`off` / `warn` / `fence` / `interrupt`, with `block` an alias for
//! `interrupt`). The veto postures record the true decision — `fence`
//! stays `fence`, `interrupt` stays `interrupt` — rather than the pre-AD-3
//! collapse to `block`.
//!
//! Unix-only: this integration test wires up `IpcListener::bind(&Path)`
//! which has a different signature on Windows (`&str` named-pipe form).
//! The end-to-end MCP enforcement path also depends on the daemon-side
//! peer-credential check (`SO_PEERCRED` on Linux, `getpeereid` on macOS)
//! which is not implemented on other Unix targets either. Gate the
//! whole integration test file rather than per-test so the test binary
//! compiles cleanly on Windows Cross.
#![cfg(unix)]
//!
//! These tests spawn the real Rust MCP shim, send a JSON-RPC tools/call
//! frame whose `proposedContent` triggers the secret-detection rule,
//! and check the structured response per the mapping table documented
//! in `crates/anvil-cli/src/mcp/enforcement.rs`.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use anvil_intercept::Shutdown;
use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::runtime::Runtime;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const CHILD_TIMEOUT: Duration = Duration::from_secs(5);
const DAEMON_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);
const SECRET_PROPOSED_CONTENT: &str = "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n";

#[test]
fn enforcement_mode_block_alias_rejects_secret_write_e2e() {
    // ADR-098 AD-3: `block` is an alias for the `interrupt` posture, and
    // the response records the true `interrupt` decision (no parse-time
    // collapse). `isError` is still true — the write is vetoed.
    let workspace = workspace_with_enforcement_mode("block");
    let payload = run_validate_write_against(workspace.path(), SECRET_PROPOSED_CONTENT);

    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["result"]["isError"], true);

    let tool = parse_tool_payload(&payload);
    assert_eq!(tool["decision"], "interrupt");
    assert_eq!(tool["safeDefault"], "do-not-write");
    assert_eq!(tool["correlation"]["enforcementMode"], "interrupt");
    assert_eq!(tool["summary"]["bySeverity"]["error"], 1);
    assert_eq!(tool["diagnostics"][0]["category"], "secret");
    assert_eq!(
        tool["diagnostics"][0]["source"]["rule_id"],
        "secret-detection"
    );
}

#[test]
fn enforcement_mode_fence_rejects_secret_write_with_true_fence_decision_e2e() {
    // ADR-098 AD-3 regression: a `fence` posture records the true `fence`
    // decision end-to-end (fence stays fence, no collapse to `block`) and
    // still reports `isError: true` via the veto projection.
    let workspace = workspace_with_enforcement_mode("fence");
    let payload = run_validate_write_against(workspace.path(), SECRET_PROPOSED_CONTENT);

    assert_eq!(payload["result"]["isError"], true);

    let tool = parse_tool_payload(&payload);
    assert_eq!(tool["decision"], "fence");
    assert_eq!(tool["safeDefault"], "do-not-write");
    assert_eq!(tool["correlation"]["enforcementMode"], "fence");
    assert_eq!(tool["summary"]["bySeverity"]["error"], 1);
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
fn missing_anvil_yaml_defaults_to_interrupt_e2e() {
    // ADR-098 AD-3: the MCP no-config default is the `block` alias —
    // `interrupt` — preserving the veto-on-error default while recording
    // the true decision.
    let workspace = tempfile::tempdir().expect("workspace exists");
    let payload = run_validate_write_against(workspace.path(), SECRET_PROPOSED_CONTENT);

    assert_eq!(payload["result"]["isError"], true);
    let tool = parse_tool_payload(&payload);
    assert_eq!(tool["decision"], "interrupt");
    assert_eq!(tool["correlation"]["enforcementMode"], "interrupt");
}

fn workspace_with_enforcement_mode(mode: &str) -> TempDir {
    let workspace = tempfile::tempdir().expect("workspace exists");
    let yaml = format!("enforcement:\n  mode: {mode}\n");
    fs::write(workspace.path().join(".anvil.yaml"), yaml).expect("write enforcement fixture");
    workspace
}

fn run_validate_write_against(workspace_root: &Path, proposed_content: &str) -> Value {
    let daemon = LiveDaemon::start();
    let mut child = spawn_mcp_server(workspace_root, daemon.xdg_runtime_dir());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    let request = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            },
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

fn spawn_mcp_server(workspace_root: &Path, xdg_runtime_dir: &Path) -> Child {
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
        .env("ANVIL_DEV", "1")
        .env("ANVIL_MCP_PREFERRED", ANVIL_BIN)
        .env("XDG_RUNTIME_DIR", xdg_runtime_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn anvil mcp serve --stdio")
}

struct LiveDaemon {
    runtime: Runtime,
    shutdown: Shutdown,
    server: Option<tokio::task::JoinHandle<Result<(), anvil_intercept::ipc::IpcError>>>,
    runtime_dir: TempDir,
}

impl LiveDaemon {
    fn start() -> Self {
        let runtime = Runtime::new().expect("tokio runtime starts");
        let runtime_dir = tempfile::tempdir().expect("runtime dir exists");
        let socket_path = daemon_socket_path(runtime_dir.path());
        let _runtime_guard = runtime.enter();
        let listener =
            IpcListener::bind(&socket_path, NoopDispatcher).expect("daemon socket binds");
        let (shutdown, token) = Shutdown::new();
        let server = runtime.spawn(listener.serve(token));

        Self {
            runtime,
            shutdown,
            server: Some(server),
            runtime_dir,
        }
    }

    fn xdg_runtime_dir(&self) -> &Path {
        self.runtime_dir.path()
    }
}

impl Drop for LiveDaemon {
    fn drop(&mut self) {
        self.shutdown.trigger();
        if let Some(server) = self.server.take() {
            self.runtime.block_on(async {
                tokio::time::timeout(DAEMON_SHUTDOWN_TIMEOUT, server)
                    .await
                    .expect("daemon task timed out during shutdown")
                    .expect("daemon task join failed")
                    .expect("daemon exited with error");
            });
        }
    }
}

fn daemon_socket_path(xdg_runtime_dir: &Path) -> PathBuf {
    xdg_runtime_dir.join("anvil").join("intercept.sock")
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
