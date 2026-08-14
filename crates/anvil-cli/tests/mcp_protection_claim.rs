//! MLP2-051b E2E: drive `anvil mcp serve --stdio` against a live daemon
//! socket and assert that the `validate_write` response carries the
//! typed `protection_claim` envelope when (and only when) the daemon
//! served the validation.
//!
//! Unix-only for the same reason as
//! `mcp_validate_write_enforcement.rs`: `IpcListener::bind(&Path)` is
//! a different signature on Windows, and the daemon-side peer-cred
//! check (`SO_PEERCRED` on Linux, `getpeereid` on macOS) is not
//! implemented on BSD/Solaris targets. Gate the file rather than each
//! test so the binary still compiles cleanly on Windows Cross.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use anvil_intercept::Shutdown;
use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
use anvil_kernel_types::protection_claim::{
    PROTECTION_CLAIM_SCHEMA_VERSION, ProtectionClaim, WorktreeClaimState,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::runtime::Runtime;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const CHILD_TIMEOUT: Duration = Duration::from_secs(5);
const DAEMON_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);
const CLEAN_PROPOSED_CONTENT: &str = "export const value = 1;\n";

/// MLP2-051b: when a daemon is wired and serves the validation, the
/// `validate_write` response carries `protection_claim` with the
/// closed-set schema. With a `NoopStatusProvider` the workspace is
/// not in the snapshot, so the typed state is `unprotected` and the
/// surfaces array is empty — but the field is *present*, which is
/// the wire-additive contract this test pins.
#[test]
fn validate_write_response_carries_protection_claim_when_daemon_serves() {
    let daemon = LiveDaemon::start();
    let workspace = tempfile::tempdir().expect("workspace exists");
    let payload = run_validate_write_with_daemon(
        workspace.path(),
        daemon.xdg_runtime_dir(),
        CLEAN_PROPOSED_CONTENT,
    );

    let tool = parse_tool_payload(&payload);
    assert_eq!(
        tool["correlation"]["backend"], "daemon",
        "must use the daemon backend for this test, got: {tool}",
    );
    assert_eq!(tool["correlation"]["daemonStatus"], "available");

    let claim = &tool["protection_claim"];
    assert!(
        !claim.is_null(),
        "protection_claim must be present when daemon serves the validation, got: {tool}",
    );
    assert_eq!(claim["schema_version"], PROTECTION_CLAIM_SCHEMA_VERSION);
    assert_eq!(
        claim["worktree_state"], "unprotected",
        "NoopStatusProvider snapshot has no worktrees → claim is unprotected, got: {claim}",
    );
    assert!(
        claim["surfaces"]
            .as_array()
            .expect("surfaces array")
            .is_empty(),
        "surfaces must be empty when no worktree matches the workspace, got: {claim}",
    );

    // Parity gate: the response must also round-trip into a
    // closed-set `ProtectionClaim` struct (proves the wire shape
    // matches the kernel type byte-for-byte, not just key-by-key).
    let parsed: ProtectionClaim =
        serde_json::from_value(claim.clone()).expect("protection_claim parses into kernel type");
    assert_eq!(parsed.worktree_state, WorktreeClaimState::Unprotected);
    assert!(parsed.surfaces.is_empty());
}

/// MLP2-051b: when the daemon socket cannot be reached, the MCP shim
/// emits the backend-failure envelope (decision = block, error =
/// validation-backend-unavailable) and must NOT carry a synthesised
/// `protection_claim`. The absent-field posture is what lets a
/// driver pin the claim shape only to live daemon snapshots.
///
/// This path is the operationally important one to pin in
/// integration: an `XDG_RUNTIME_DIR` that points at an empty
/// `anvil/` directory (no `intercept.sock`) is exactly what an
/// agent sees when the daemon has not been started yet. The pure
/// `not-wired` path (no XDG, embedded validator only) is covered
/// by the inline `protection_claim_omitted_when_daemon_not_wired`
/// unit test in `validate_write.rs`.
#[test]
fn validate_write_response_omits_protection_claim_when_daemon_unreachable() {
    let workspace = tempfile::tempdir().expect("workspace exists");
    let xdg = tempfile::tempdir().expect("xdg dir exists");
    std::fs::create_dir_all(xdg.path().join("anvil")).expect("anvil/ subdir");

    let payload =
        run_validate_write_with_daemon(workspace.path(), xdg.path(), CLEAN_PROPOSED_CONTENT);
    let tool = parse_tool_payload(&payload);

    assert_eq!(
        tool["decision"], "block",
        "backend-failure path must emit decision: block, got: {tool}",
    );
    assert_eq!(tool["error"]["code"], "validation-backend-unavailable");
    assert_eq!(tool["correlation"]["daemonStatus"], "unavailable");
    assert!(
        tool.get("protection_claim").is_none(),
        "backend-failure responses must not carry a protection_claim, got: {tool}",
    );
}

/// MLP2-051b: wire-additive contract. A driver pinned to a struct
/// that DOES NOT carry `protection_claim` still deserialises a
/// response with the field (serde drops unknown keys by default).
/// A driver pinned to a struct that DOES carry the field as
/// `Option<ProtectionClaim>` with `#[serde(default)]` parses both
/// the absent and present shapes without error.
/// A driver that has not yet adopted MLP2-051b only knows about
/// `decision`. The lack of `deny_unknown_fields` on the response
/// means the new `protection_claim` field is silently dropped during
/// deserialisation rather than causing an error.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PreClaimDriverView {
    decision: String,
}

/// A driver that has adopted MLP2-051b carries the optional field.
/// `#[serde(default)]` makes the absent shape parse as `None` rather
/// than failing — pairs with the producer-side
/// `skip_serializing_if = "Option::is_none"`.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PostClaimDriverView {
    decision: String,
    #[serde(default)]
    protection_claim: Option<ProtectionClaim>,
}

#[test]
fn pre_and_post_mlp2_051b_drivers_both_round_trip_the_response() {
    let daemon = LiveDaemon::start();
    let workspace = tempfile::tempdir().expect("workspace exists");
    let payload = run_validate_write_with_daemon(
        workspace.path(),
        daemon.xdg_runtime_dir(),
        CLEAN_PROPOSED_CONTENT,
    );
    let tool = parse_tool_payload(&payload);

    let pre: PreClaimDriverView =
        serde_json::from_value(tool.clone()).expect("pre-MLP2-051b driver parses response");
    assert_eq!(pre.decision, "allow", "clean content must be allowed");

    let post: PostClaimDriverView =
        serde_json::from_value(tool).expect("post-MLP2-051b driver parses response");
    assert_eq!(post.decision, "allow");
    let claim = post
        .protection_claim
        .expect("post-MLP2-051b driver sees Some(claim) when daemon is wired");
    assert_eq!(claim.worktree_state, WorktreeClaimState::Unprotected);
}

fn run_validate_write_with_daemon(
    workspace_root: &Path,
    xdg_runtime_dir: &Path,
    proposed_content: &str,
) -> Value {
    let mut child = spawn_mcp_server(workspace_root, xdg_runtime_dir);
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    let request = json!({
        "jsonrpc": "2.0",
        "id": 51,
        "method": "tools/call",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            },
            "name": "anvil_validate_write",
            "arguments": {
                "path": "src/clean.ts",
                "operation": "create",
                "proposedContent": proposed_content,
                // RMCPF-043: default detail is minimal (schema+decision only
                // on clean allow). Claim-bearing envelope tests require full.
                "detail": "full"
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
