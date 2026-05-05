//! RMCP-002: `anvil mcp serve --stdio` starts a Rust stdio MCP server.

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
const CHILD_TIMEOUT: Duration = Duration::from_secs(3);
const DAEMON_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

#[test]
fn mcp_serve_stdio_initialise_returns_json_rpc_response() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "rmcp-test-client",
                "version": "0.0.0"
            }
        }
    });

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(stdin, "{request}").expect("failed to send initialize frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);

    assert!(
        status.success(),
        "mcp server must exit cleanly on EOF; status: {status:?}",
    );
    assert!(
        !line.trim().is_empty(),
        "expected one JSON-RPC response on stdout"
    );

    let parsed: Value = serde_json::from_str(line.trim()).unwrap_or_else(|err| {
        panic!("stdout response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });

    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 1);
    assert_eq!(parsed["result"]["serverInfo"]["name"], "anvil");
    assert_eq!(parsed["result"]["capabilities"]["tools"], json!({}));
    let instructions = parsed["result"]["instructions"]
        .as_str()
        .expect("initialize result must include server instructions");
    assert!(instructions.contains("anvil_validate_write"));
    assert!(instructions.contains("Before applying any file write"));
    assert!(instructions.contains("block"));
}

#[test]
fn mcp_serve_stdio_ready_notification_does_not_emit_response() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            })
        )
        .expect("failed to send ready notification");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "ping"
            })
        )
        .expect("failed to send ping frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after ready notification and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("ping response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 5);
    assert_eq!(parsed["result"], json!({}));
}

#[test]
fn mcp_serve_stdio_tools_list_returns_validate_write_tool() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/list"
            })
        )
        .expect("failed to send tools/list frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after tools/list and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("tools/list response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 6);
    let tools = parsed["result"]["tools"]
        .as_array()
        .expect("tools/list result must include a tools array");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "anvil_validate_write");
    let description = tools[0]["description"]
        .as_str()
        .expect("tool descriptor must include a description");
    assert!(description.contains("before EVERY file write"));
    assert!(description.contains("Honour `block` decisions"));
    assert_eq!(tools[0]["inputSchema"]["type"], "object");
}

#[test]
fn mcp_serve_stdio_shutdown_flushes_response_before_exit_notification() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "shutdown"
            })
        )
        .expect("failed to send shutdown frame");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "method": "exit"
            })
        )
        .expect("failed to send exit frame");
    }

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after shutdown/exit; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("shutdown response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 2);
    assert!(parsed["result"].is_null());
}

#[test]
fn mcp_serve_stdio_malformed_json_returns_protocol_error() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(stdin, "{{not json}}").expect("failed to send malformed frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after malformed frame and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("malformed response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], Value::Null);
    assert_eq!(parsed["error"]["code"], -32700);
}

#[test]
fn mcp_serve_stdio_initialize_invalid_params_returns_invalid_params_error() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "initialize",
                "params": "invalid"
            })
        )
        .expect("failed to send invalid initialize frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after invalid initialize params and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("invalid params response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 4);
    assert_eq!(parsed["error"]["code"], -32602);
}

#[test]
fn mcp_serve_stdio_unsupported_method_returns_method_not_found() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "resources/list"
            })
        )
        .expect("failed to send unsupported method frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after unsupported method and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("unsupported method response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 7);
    assert_eq!(parsed["error"]["code"], -32601);
}

#[test]
fn mcp_serve_stdio_tools_call_rejects_unknown_tool() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "tools/call",
                "params": {
                    "name": "not_anvil_validate_write",
                    "arguments": {}
                }
            })
        )
        .expect("failed to send unknown tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after unknown tool call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("unknown tool response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 8);
    assert_eq!(parsed["error"]["code"], -32602);
}

#[test]
fn mcp_serve_stdio_tools_call_missing_arguments_blocks_write() {
    let mut child = spawn_mcp_server_with_dev_bypass();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": {
                    "name": "anvil_validate_write"
                }
            })
        )
        .expect("failed to send missing-arguments tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after missing-arguments tool call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("missing-arguments tool response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 9);
    assert_eq!(parsed["result"]["isError"], true);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(payload["decision"], "block");
    assert_eq!(payload["safeDefault"], "do-not-write");
    assert_eq!(payload["error"]["code"], "missing-path");
}

#[test]
fn mcp_serve_stdio_tools_call_known_tool_allows_clean_content() {
    let daemon = LiveDaemon::start();
    let mut child = spawn_mcp_server_with_dev_bypass_and_daemon(daemon.xdg_runtime_dir());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 10,
                "method": "tools/call",
                "params": {
                    "name": "anvil_validate_write",
                    "arguments": {
                        "path": "src/example.ts",
                        "operation": "create",
                        "proposedContent": "export const value = 1;\n"
                    }
                }
            })
        )
        .expect("failed to send known tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after known tool call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("known tool response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 10);
    assert_eq!(parsed["result"]["isError"], false);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(payload["schema"], "anvil.mcp.validate-write.v1");
    assert_eq!(payload["decision"], "allow");
    assert_eq!(payload["summary"]["total"], 0);
    assert_eq!(payload["diagnostics"], json!([]));
    assert_eq!(payload["correlation"]["surface"], "mcp");
    assert_eq!(payload["correlation"]["mode"], "preWrite");
    assert_eq!(payload["correlation"]["backend"], "daemon");
    assert_eq!(payload["correlation"]["daemonStatus"], "available");
}

#[test]
fn mcp_serve_stdio_tools_call_blocks_secret_content() {
    let daemon = LiveDaemon::start();
    let mut child = spawn_mcp_server_with_dev_bypass_and_daemon(daemon.xdg_runtime_dir());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 11,
                "method": "tools/call",
                "params": {
                    "name": "anvil_validate_write",
                    "arguments": {
                        "path": "src/secret.ts",
                        "operation": "create",
                        "proposedContent": "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n"
                    }
                }
            })
        )
        .expect("failed to send known tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after secret tool call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("secret tool response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 11);
    assert_eq!(parsed["result"]["isError"], true);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(payload["decision"], "block");
    assert_eq!(payload["safeDefault"], "do-not-write");
    assert_eq!(payload["summary"]["bySeverity"]["error"], 1);
    assert_eq!(
        payload["diagnostics"][0]["schema_version"],
        "anvil.diagnostic.v1"
    );
    assert_eq!(payload["diagnostics"][0]["category"], "secret");
    assert_eq!(
        payload["diagnostics"][0]["source"]["rule_id"],
        "secret-detection"
    );
    assert_eq!(payload["correlation"]["backend"], "daemon");
    assert_eq!(payload["correlation"]["daemonStatus"], "available");
}

#[test]
fn mcp_serve_stdio_oversize_frame_returns_protocol_error() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        // Send a frame larger than the server's stdio frame ceiling
        // (currently 4 MiB) without a trailing newline.
        let oversize_frame = vec![b'a'; 4 * 1024 * 1024 + 2];
        stdin
            .write_all(&oversize_frame)
            .expect("failed to send oversize frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after oversize frame and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("oversize response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], Value::Null);
    assert_eq!(parsed["error"]["code"], -32600);
}

fn spawn_mcp_server() -> Child {
    Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("mcp")
        .arg("serve")
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn anvil mcp serve --stdio")
}

fn spawn_mcp_server_with_dev_bypass() -> Child {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("mcp")
        .arg("serve")
        .arg("--stdio")
        .env("ANVIL_DEV", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn anvil mcp serve --stdio")
}

fn spawn_mcp_server_with_dev_bypass_and_daemon(xdg_runtime_dir: &Path) -> Child {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("mcp")
        .arg("serve")
        .arg("--stdio")
        .env("ANVIL_DEV", "1")
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
