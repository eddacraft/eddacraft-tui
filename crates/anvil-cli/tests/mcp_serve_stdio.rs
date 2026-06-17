//! RMCP-002: `anvil mcp serve --stdio` starts a Rust stdio MCP server.
//!
//! The daemon-backed cases are Unix-only because they wire up
//! `IpcListener::bind(&Path)`, which has a different named-pipe form on
//! Windows. The pure stdio protocol cases stay portable and should keep
//! running on Windows Cross.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::path::PathBuf;

#[cfg(unix)]
use anvil_intercept::Shutdown;
#[cfg(unix)]
use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
use serde_json::{Value, json};
#[cfg(unix)]
use tokio::runtime::Runtime;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const CHILD_TIMEOUT: Duration = Duration::from_secs(3);
#[cfg(unix)]
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
fn mcp_serve_stdio_tools_call_status_rejects_workspace_outside_server_root() {
    let server_root = tempfile::tempdir().expect("server root exists");
    let sibling_workspace = tempfile::tempdir().expect("sibling workspace exists");
    let mut child = spawn_mcp_server_in(server_root.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 13,
                "method": "tools/call",
                "params": {
                    "name": "anvil_status",
                    "arguments": {
                        "workspaceRoot": sibling_workspace.path()
                    }
                }
            })
        )
        .expect("failed to send out-of-root status tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after out-of-root status call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("status error response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["result"]["isError"], true);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(
        payload["error"],
        "workspaceRoot must be inside the MCP server root"
    );
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
#[allow(clippy::too_many_lines)] // end-to-end flow: setup + initialize + tools/list + parse + assertions
fn mcp_serve_stdio_tools_list_returns_registered_tools() {
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
    assert_eq!(tools.len(), 11);
    let validate_write = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_validate_write")
        .expect("tools/list includes anvil_validate_write");
    let search_symbols = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_search_symbols")
        .expect("tools/list includes anvil_search_symbols");
    assert_eq!(search_symbols["inputSchema"]["type"], "object");
    assert_eq!(search_symbols["annotations"]["readOnlyHint"], true);
    let find_dependents = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_find_dependents")
        .expect("tools/list includes anvil_find_dependents");
    assert_eq!(find_dependents["inputSchema"]["type"], "object");
    assert_eq!(find_dependents["annotations"]["readOnlyHint"], true);
    let find_dependents_required = find_dependents["inputSchema"]["required"]
        .as_array()
        .expect("find_dependents inputSchema.required is an array");
    assert!(find_dependents_required.contains(&json!("workspaceRoot")));
    assert!(find_dependents_required.contains(&json!("file")));
    let impact_of_change = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_impact_of_change")
        .expect("tools/list includes anvil_impact_of_change");
    assert_eq!(impact_of_change["inputSchema"]["type"], "object");
    assert_eq!(impact_of_change["annotations"]["readOnlyHint"], true);
    let impact_required = impact_of_change["inputSchema"]["required"]
        .as_array()
        .expect("impact_of_change inputSchema.required is an array");
    assert!(impact_required.contains(&json!("workspaceRoot")));
    assert!(impact_required.contains(&json!("changedFiles")));
    let apply_patch = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_apply_patch")
        .expect("tools/list includes anvil_apply_patch");
    let status = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_status")
        .expect("tools/list includes anvil_status");
    let check = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_check")
        .expect("tools/list includes anvil_check");
    let gate = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_gate")
        .expect("tools/list includes anvil_gate");
    let query_boundary = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_query_boundary")
        .expect("tools/list includes anvil_query_boundary");
    let suppress = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_suppress")
        .expect("tools/list includes anvil_suppress");
    let fix = tools
        .iter()
        .find(|tool| tool["name"] == "anvil_fix")
        .expect("tools/list includes anvil_fix");
    assert_eq!(apply_patch["inputSchema"]["type"], "object");
    assert_eq!(apply_patch["annotations"]["destructiveHint"], true);
    let apply_patch_required = apply_patch["inputSchema"]["required"]
        .as_array()
        .expect("anvil_apply_patch required is an array");
    assert!(apply_patch_required.contains(&json!("path")));
    assert!(apply_patch_required.contains(&json!("unifiedDiff")));
    let description = validate_write["description"]
        .as_str()
        .expect("tool descriptor must include a description");
    assert!(description.contains("before EVERY file write"));
    assert!(description.contains("Honour `block` decisions"));
    assert_eq!(validate_write["inputSchema"]["type"], "object");
    assert_eq!(status["inputSchema"]["type"], "object");
    assert!(
        status["inputSchema"]["properties"]
            .get("workspaceRoot")
            .is_some()
    );
    assert_eq!(check["inputSchema"]["type"], "object");
    let check_required = check["inputSchema"]["required"]
        .as_array()
        .expect("check inputSchema.required is an array");
    assert!(check_required.contains(&json!("files")));
    assert!(check_required.contains(&json!("workspaceRoot")));
    assert_eq!(gate["inputSchema"]["type"], "object");
    assert!(
        gate["inputSchema"]["properties"]
            .get("targetFiles")
            .is_some()
    );
    assert_eq!(query_boundary["inputSchema"]["type"], "object");
    let qb_required = query_boundary["inputSchema"]["required"]
        .as_array()
        .expect("anvil_query_boundary required is an array");
    assert!(qb_required.contains(&json!("sourceFile")));
    assert!(qb_required.contains(&json!("targetFile")));
    assert_eq!(suppress["inputSchema"]["type"], "object");
    let suppress_required = suppress["inputSchema"]["required"]
        .as_array()
        .expect("anvil_suppress required is an array");
    assert!(suppress_required.contains(&json!("reason")));
    assert_eq!(fix["inputSchema"]["type"], "object");
    let fix_required = fix["inputSchema"]["required"]
        .as_array()
        .expect("anvil_fix required is an array");
    assert!(fix_required.contains(&json!("warningId")));
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
fn mcp_serve_stdio_tools_call_known_tool_allows_clean_content_via_embedded_fallback() {
    let runtime_dir = tempfile::tempdir().expect("isolated runtime dir exists");
    let home_dir = tempfile::tempdir().expect("isolated home dir exists");
    let mut child =
        spawn_mcp_server_with_dev_bypass_without_daemon(runtime_dir.path(), home_dir.path());
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
    assert_eq!(payload["decision"], "allow");
    assert_eq!(payload["summary"]["total"], 0);
    assert_eq!(payload["diagnostics"], json!([]));
    assert_eq!(payload["correlation"]["backend"], "embedded");
    assert_eq!(payload["correlation"]["daemonStatus"], "not-wired");
}

#[test]
fn mcp_serve_stdio_tools_call_status_returns_workspace_health_summary() {
    let workspace = tempfile::tempdir().expect("workspace dir exists");
    std::fs::write(
        workspace.path().join(".anvilrc"),
        r#"{"checks":["secret-detection","policy"]}"#,
    )
    .expect("test config is writable");
    std::fs::create_dir_all(workspace.path().join(".anvil")).expect("anvil dir is writable");
    std::fs::write(workspace.path().join(".anvil/architecture.json"), "{}")
        .expect("baseline is writable");

    let mut child = spawn_mcp_server_in(workspace.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 12,
                "method": "tools/call",
                "params": {
                    "name": "anvil_status",
                    "arguments": {
                        "workspaceRoot": workspace.path()
                    }
                }
            })
        )
        .expect("failed to send status tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after status tool call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("status tool response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 12);
    assert_eq!(parsed["result"]["isError"], false);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["workspaceRoot"], ".");
    assert_eq!(payload["hasBaseline"], true);
    assert_eq!(payload["config"]["loaded"], true);
    assert_eq!(payload["config"]["source"], ".anvilrc");
    assert_eq!(
        payload["config"]["checks"],
        json!(["secret-detection", "policy"])
    );
    assert_eq!(payload["backend"], "local");
    assert_eq!(payload["daemonStatus"], "not-wired");
    assert!(
        payload["availableChecks"]
            .as_array()
            .expect("availableChecks is an array")
            .contains(&json!("secret-detection"))
    );
}

#[test]
fn mcp_serve_stdio_tools_call_check_returns_clean_payload_for_clean_files() {
    let workspace = tempfile::tempdir().expect("workspace dir exists");
    let src = workspace.path().join("src");
    std::fs::create_dir_all(&src).expect("src dir exists");
    std::fs::write(src.join("clean.ts"), "export const value = 1;\n")
        .expect("clean fixture is writable");

    let mut child = spawn_mcp_server_in(workspace.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 20,
                "method": "tools/call",
                "params": {
                    "name": "anvil_check",
                    "arguments": {
                        "workspaceRoot": workspace.path(),
                        "files": ["src/clean.ts"]
                    }
                }
            })
        )
        .expect("failed to send check tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after check tool call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("check tool response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 20);
    assert_eq!(parsed["result"]["isError"], false);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(payload["hasBlockingWarnings"], false);
    assert_eq!(payload["summary"]["total"], 0);
    assert_eq!(payload["warnings"], json!([]));
    assert_eq!(payload["workspaceRoot"], ".");
    assert_eq!(payload["backend"], "local");
    assert_eq!(payload["daemonStatus"], "not-wired");
    assert_eq!(payload["checksRun"], json!(["antipattern"]));
}

#[test]
fn mcp_serve_stdio_tools_call_check_rejects_workspace_outside_server_root() {
    let server_root = tempfile::tempdir().expect("server root exists");
    let foreign_workspace = tempfile::tempdir().expect("foreign workspace exists");

    let mut child = spawn_mcp_server_in(server_root.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 21,
                "method": "tools/call",
                "params": {
                    "name": "anvil_check",
                    "arguments": {
                        "workspaceRoot": foreign_workspace.path(),
                        "files": []
                    }
                }
            })
        )
        .expect("failed to send out-of-root check tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after out-of-root check call; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("check tool response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["result"]["isError"], true);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(
        payload["error"],
        "workspaceRoot must be inside the MCP server root"
    );
}

#[test]
fn mcp_serve_stdio_tools_call_gate_planless_mode_scans_target_files() {
    let workspace = tempfile::tempdir().expect("workspace dir exists");
    let src = workspace.path().join("src");
    std::fs::create_dir_all(&src).expect("src dir exists");
    std::fs::write(src.join("clean.ts"), "export const value = 1;\n")
        .expect("clean fixture is writable");

    let mut child = spawn_mcp_server_in(workspace.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 22,
                "method": "tools/call",
                "params": {
                    "name": "anvil_gate",
                    "arguments": {
                        "workspaceRoot": workspace.path(),
                        "targetFiles": ["src/clean.ts"]
                    }
                }
            })
        )
        .expect("failed to send gate planless tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after gate planless call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("gate planless response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 22);
    assert_eq!(parsed["result"]["isError"], false);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(payload["mode"], "planless");
    assert_eq!(payload["hasBlockingWarnings"], false);
    assert_eq!(payload["summary"]["total"], 0);
    assert_eq!(payload["checksRun"], json!(["antipattern"]));
    assert_eq!(payload["backend"], "local");
    assert_eq!(payload["daemonStatus"], "not-wired");
}

#[test]
fn mcp_serve_stdio_tools_call_blocks_secret_content_via_embedded_fallback() {
    let runtime_dir = tempfile::tempdir().expect("isolated runtime dir exists");
    let home_dir = tempfile::tempdir().expect("isolated home dir exists");
    let mut child =
        spawn_mcp_server_with_dev_bypass_without_daemon(runtime_dir.path(), home_dir.path());
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
        .expect("failed to send secret tool call frame");
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
    assert_eq!(payload["diagnostics"][0]["category"], "secret");
    assert_eq!(payload["correlation"]["backend"], "embedded");
    assert_eq!(payload["correlation"]["daemonStatus"], "not-wired");
}

#[cfg(unix)]
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

#[cfg(unix)]
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
fn mcp_serve_stdio_tools_call_query_boundary_returns_no_baseline_for_clean_workspace() {
    let workspace = tempfile::tempdir().expect("workspace exists");

    let mut child = spawn_mcp_server_in(workspace.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 30,
                "method": "tools/call",
                "params": {
                    "name": "anvil_query_boundary",
                    "arguments": {
                        "workspaceRoot": workspace.path(),
                        "sourceFile": "src/controllers/user.ts",
                        "targetFile": "src/domain/user.ts"
                    }
                }
            })
        )
        .expect("failed to send query_boundary tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after query_boundary call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("query_boundary response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 30);
    assert_eq!(parsed["result"]["isError"], false);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(payload["allowed"], true);
    assert_eq!(payload["reason"], "no-baseline");
    assert_eq!(payload["backend"], "local");
    assert_eq!(payload["daemonStatus"], "not-wired");
}

#[test]
fn mcp_serve_stdio_tools_call_suppress_inserts_comment_in_workspace_file() {
    let workspace = tempfile::tempdir().expect("workspace exists");
    let src = workspace.path().join("src");
    std::fs::create_dir_all(&src).expect("src dir exists");
    std::fs::write(src.join("a.ts"), "const x: any = 1;\n").expect("fixture written");

    let mut child = spawn_mcp_server_in(workspace.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 31,
                "method": "tools/call",
                "params": {
                    "name": "anvil_suppress",
                    "arguments": {
                        "workspaceRoot": workspace.path(),
                        "filePath": "src/a.ts",
                        "warningId": "AP-003",
                        "line": 1,
                        "reason": "legacy contract under TICKET-123"
                    }
                }
            })
        )
        .expect("failed to send suppress tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after suppress call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("suppress response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 31);
    assert_eq!(parsed["result"]["isError"], false);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(payload["suppressed"], true);
    assert_eq!(payload["warningId"], "AP-003");
    assert_eq!(payload["backend"], "embedded");
    assert_eq!(payload["daemonStatus"], "not-wired");

    let on_disk =
        std::fs::read_to_string(workspace.path().join("src/a.ts")).expect("file readable");
    assert!(on_disk.contains("@anvil-ignore-until"));
    assert!(on_disk.contains("AP-003: legacy contract under TICKET-123"));
}

// CLAWP-024: `anvil_suppress` mutates files, so workspace-root containment
// must be enforced before any write. The server starts in one tempdir while
// `workspaceRoot` points at a sibling tempdir outside the server root; the
// call must error and the sibling file must remain byte-for-byte unchanged.
#[test]
fn mcp_serve_stdio_tools_call_suppress_rejects_workspace_outside_server_root() {
    const ORIGINAL: &str = "const x: any = 1;\n";
    let server_root = tempfile::tempdir().expect("server root exists");
    let foreign_workspace = tempfile::tempdir().expect("foreign workspace exists");
    let foreign_src = foreign_workspace.path().join("src");
    std::fs::create_dir_all(&foreign_src).expect("foreign src dir exists");
    std::fs::write(foreign_src.join("a.ts"), ORIGINAL).expect("foreign fixture written");

    let mut child = spawn_mcp_server_in(server_root.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 33,
                "method": "tools/call",
                "params": {
                    "name": "anvil_suppress",
                    "arguments": {
                        "workspaceRoot": foreign_workspace.path(),
                        "filePath": "src/a.ts",
                        "warningId": "AP-003",
                        "line": 1,
                        "reason": "legacy contract under TICKET-123"
                    }
                }
            })
        )
        .expect("failed to send out-of-root suppress tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after out-of-root suppress call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("suppress error response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["result"]["isError"], true);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(
        payload["error"],
        "workspaceRoot must be inside the MCP server root"
    );

    // The mutating tool must not have touched the sibling file on disk.
    let on_disk =
        std::fs::read_to_string(foreign_src.join("a.ts")).expect("foreign file still readable");
    assert_eq!(
        on_disk, ORIGINAL,
        "out-of-root suppress must leave the sibling file unchanged"
    );
}

#[test]
fn mcp_serve_stdio_tools_call_fix_replaces_any_with_unknown() {
    let workspace = tempfile::tempdir().expect("workspace exists");
    let src = workspace.path().join("src");
    std::fs::create_dir_all(&src).expect("src dir exists");
    std::fs::write(src.join("a.ts"), "const x: any = 1;\n").expect("fixture written");

    let mut child = spawn_mcp_server_in(workspace.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 32,
                "method": "tools/call",
                "params": {
                    "name": "anvil_fix",
                    "arguments": {
                        "workspaceRoot": workspace.path(),
                        "filePath": "src/a.ts",
                        "warningId": "AP-003",
                        "line": 1
                    }
                }
            })
        )
        .expect("failed to send fix tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after fix call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("fix response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 32);
    assert_eq!(parsed["result"]["isError"], false);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(payload["fixed"], true);
    assert_eq!(payload["before"], "const x: any = 1;");
    assert_eq!(payload["after"], "const x: unknown = 1;");

    let on_disk =
        std::fs::read_to_string(workspace.path().join("src/a.ts")).expect("file readable");
    assert!(on_disk.contains("const x: unknown = 1;"));
}

// CLAWP-024: `anvil_fix` also mutates files, so it must enforce the same
// workspace-root containment guard as `anvil_suppress`. The server starts in
// one tempdir while `workspaceRoot` points at a sibling tempdir outside the
// server root; the call must error and the sibling file must stay unchanged.
#[test]
fn mcp_serve_stdio_tools_call_fix_rejects_workspace_outside_server_root() {
    const ORIGINAL: &str = "const x: any = 1;\n";
    let server_root = tempfile::tempdir().expect("server root exists");
    let foreign_workspace = tempfile::tempdir().expect("foreign workspace exists");
    let foreign_src = foreign_workspace.path().join("src");
    std::fs::create_dir_all(&foreign_src).expect("foreign src dir exists");
    std::fs::write(foreign_src.join("a.ts"), ORIGINAL).expect("foreign fixture written");

    let mut child = spawn_mcp_server_in(server_root.path());
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    {
        let stdin = child.stdin.as_mut().expect("child stdin is piped");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 34,
                "method": "tools/call",
                "params": {
                    "name": "anvil_fix",
                    "arguments": {
                        "workspaceRoot": foreign_workspace.path(),
                        "filePath": "src/a.ts",
                        "warningId": "AP-003",
                        "line": 1
                    }
                }
            })
        )
        .expect("failed to send out-of-root fix tool call frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after out-of-root fix call and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("fix error response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["result"]["isError"], true);

    let payload = parse_tool_payload(&parsed);
    assert_eq!(
        payload["error"],
        "workspaceRoot must be inside the MCP server root"
    );

    // The mutating tool must not have touched the sibling file on disk.
    let on_disk =
        std::fs::read_to_string(foreign_src.join("a.ts")).expect("foreign file still readable");
    assert_eq!(
        on_disk, ORIGINAL,
        "out-of-root fix must leave the sibling file unchanged"
    );
}

#[test]
fn mcp_serve_stdio_initialize_does_not_advertise_prompts_capability() {
    // RMCPF-012: prompts are deferred-then-retired per
    // `plans/specs/rust-mcp-full-port-inventory.md` §Prompts. The Rust MCP
    // server must NOT advertise a `prompts` capability so clients don't
    // try to call `prompts/list` against a surface we don't ship.
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
                "id": 40,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "rmcpf012-test-client", "version": "0.0.0" }
                }
            })
        )
        .expect("failed to send initialize frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after initialize and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("initialize response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 40);
    let capabilities = parsed["result"]["capabilities"]
        .as_object()
        .expect("capabilities is an object");
    assert!(
        capabilities.get("prompts").is_none(),
        "RMCPF-012 retires prompts — initialize must not advertise the capability, got {capabilities:?}",
    );
    assert!(
        capabilities.get("tools").is_some(),
        "tools capability must still be advertised",
    );
}

#[test]
fn mcp_serve_stdio_prompts_list_returns_method_not_found() {
    // RMCPF-012: `prompts/list` is not implemented because no prompts are
    // ported. The server must return JSON-RPC `Method not found` rather
    // than silently succeeding with an empty list, so clients see the
    // retirement decision rather than guess.
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
                "id": 41,
                "method": "prompts/list"
            })
        )
        .expect("failed to send prompts/list frame");
    }
    drop(child.stdin.take());

    let line = recv_stdout_line(&mut child, &stdout_rx);
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "mcp server must exit cleanly after prompts/list and EOF; status: {status:?}",
    );

    let parsed: Value = serde_json::from_str(&line).unwrap_or_else(|err| {
        panic!("prompts/list response must be JSON-RPC JSON, got {line:?}\nerror: {err}")
    });
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 41);
    assert_eq!(parsed["error"]["code"], -32601);
    assert_eq!(parsed["error"]["message"], "Method not found");
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

fn spawn_mcp_server_in(cwd: &Path) -> Child {
    Command::new(ANVIL_BIN)
        .current_dir(cwd)
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

fn spawn_mcp_server_with_dev_bypass_without_daemon(runtime_dir: &Path, home_dir: &Path) -> Child {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("mcp")
        .arg("serve")
        .arg("--stdio")
        .env("ANVIL_DEV", "1")
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .env("HOME", home_dir)
        .env("USERPROFILE", home_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn anvil mcp serve --stdio")
}

#[cfg(unix)]
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

#[cfg(unix)]
struct LiveDaemon {
    runtime: Runtime,
    shutdown: Shutdown,
    server: Option<tokio::task::JoinHandle<Result<(), anvil_intercept::ipc::IpcError>>>,
    runtime_dir: tempfile::TempDir,
}

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
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
