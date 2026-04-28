//! RMCP-002: `anvil mcp serve --stdio` starts a Rust stdio MCP server.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const CHILD_TIMEOUT: Duration = Duration::from_secs(3);

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
fn mcp_serve_stdio_oversize_frame_returns_protocol_error() {
    let mut child = spawn_mcp_server();
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        let oversize_frame = vec![b'a'; 1024 * 1024 + 2];
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
