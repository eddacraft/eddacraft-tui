//! GCTX-023 MCP integration: drive `anvil_symbol_context` over stdio against a
//! live daemon socket with a pre-warmed span-bearing graph. Pins identity-only
//! default, gated text under `ANVIL_GCTX_EGRESS=1` + `includeSource`, and cold
//! graph degradation.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anvil_checks::antipattern::types::AntipatternCheckConfig;
use anvil_gctx_types::GCTX_EGRESS_ENV;
use anvil_intercept::Shutdown;
use anvil_intercept::confinement::Confinement;
use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
use anvil_intercept::save_time::{SaveTimeState, SymbolParser};
use anvil_intercept::workspace_pool::WorkScheduler;

use anvil_kernel_types::{
    ByteRange, FileSymbols, SymbolKind, SymbolNode, TrustLevel, Visibility, content_hash,
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::runtime::Runtime;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const CHILD_TIMEOUT: Duration = Duration::from_secs(10);
const DAEMON_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

const GREET_SOURCE: &[u8] = b"export function greet() { return 1; }\n";

#[derive(Debug)]
struct SnippetStubParser;

impl SymbolParser for SnippetStubParser {
    fn parse(&self, path: &Path, bytes: &[u8]) -> Option<FileSymbols> {
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|name| format!("src/{name}"))
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let end = u32::try_from(bytes.len()).unwrap_or(u32::MAX).saturating_sub(1);
        Some(FileSymbols {
            file: file.clone(),
            symbols: vec![SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "greet".to_string(),
                visibility: Visibility::Public,
                file,
                trust_level: TrustLevel::Unknown,
                span: Some(ByteRange { start: 0, end }),
            }],
            imports: Vec::new(),
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: Some(content_hash(bytes)),
        })
    }
}

fn save_time_state() -> SaveTimeState {
    SaveTimeState::new(
        WorkScheduler::new().expect("scheduler"),
        AntipatternCheckConfig::default(),
        Confinement::open_default(),
    )
    .with_parser(Arc::new(SnippetStubParser))
}

fn prepare_workspace(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().join("workspace");
    std::fs::create_dir_all(root.join("src")).expect("mkdir src");
    std::fs::write(root.join("src/greet.ts"), GREET_SOURCE).expect("write source");
    std::fs::canonicalize(&root).expect("canonicalise")
}

fn daemon_jsonrpc(socket: &Path, method: &str, params: Value) -> Value {
    let mut stream = UnixStream::connect(socket).expect("connect daemon socket");
    let frame = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": "gctx-mcp-warm",
    });
    writeln!(stream, "{frame}").expect("write daemon request");
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read daemon response");
    serde_json::from_str(line.trim()).expect("daemon response json")
}

fn warm_graph_over_daemon_socket(socket: &Path, root: &Path) {
    let root = root.to_string_lossy();
    let validate = daemon_jsonrpc(
        socket,
        "anvil/validate_paths",
        json!({
            "workspace_root": root,
            "paths": [{ "path": "src/greet.ts", "change": "modified" }],
        }),
    );
    assert!(
        validate.get("error").is_none(),
        "validate_paths must warm the graph: {validate}",
    );

    for i in 0..300 {
        let status = daemon_jsonrpc(
            socket,
            "anvil/workspace_status",
            json!({ "workspace_root": root, "id": format!("warm-{i}") }),
        );
        let state = status
            .pointer("/result/workspace_assurance/state")
            .and_then(Value::as_str);
        if matches!(state, Some("stale") | Some("clean") | Some("bounded")) {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("graph never became readable after validate_paths");
}

struct GctxDaemon {
    runtime: Runtime,
    shutdown: Shutdown,
    server: Option<tokio::task::JoinHandle<Result<(), anvil_intercept::ipc::IpcError>>>,
    runtime_dir: TempDir,
    _socket_dir: PathBuf,
}

impl GctxDaemon {
    fn start(state: Arc<SaveTimeState>) -> Self {
        let runtime = Runtime::new().expect("tokio runtime starts");
        let runtime_dir = tempfile::tempdir().expect("runtime dir exists");
        std::fs::create_dir_all(runtime_dir.path().join("anvil")).expect("anvil subdir");
        let socket_path = daemon_socket_path(runtime_dir.path());
        let socket_parent = socket_path.parent().expect("socket parent");
        std::fs::set_permissions(socket_parent, std::fs::Permissions::from_mode(0o700))
            .expect("secure socket parent");

        let _runtime_guard = runtime.enter();
        let listener = IpcListener::bind(&socket_path, NoopDispatcher)
            .expect("daemon socket binds")
            .with_save_time_state(state);
        let (shutdown, token) = Shutdown::new();
        let server = runtime.spawn(listener.serve(token));

        Self {
            runtime,
            shutdown,
            server: Some(server),
            runtime_dir,
            _socket_dir: socket_parent.to_path_buf(),
        }
    }

    fn xdg_runtime_dir(&self) -> &Path {
        self.runtime_dir.path()
    }

    fn socket_path(&self) -> PathBuf {
        daemon_socket_path(self.runtime_dir.path())
    }
}

impl Drop for GctxDaemon {
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

fn spawn_mcp_server(workspace_root: &Path, xdg_runtime_dir: &Path) -> Child {
    Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("mcp")
        .arg("serve")
        .arg("--stdio")
        .current_dir(workspace_root)
        .env("ANVIL_DEV", "1")
        .env("XDG_RUNTIME_DIR", xdg_runtime_dir)
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
            Err(err) => panic!("failed to poll child: {err}"),
        }
    }
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn parse_tool_payload(parsed: &Value) -> Value {
    let text = parsed["result"]["content"][0]["text"]
        .as_str()
        .expect("tool result must contain a JSON text content item");
    serde_json::from_str(text).expect("tool result text must be JSON")
}

fn call_symbol_context(
    workspace_root: &Path,
    xdg_runtime_dir: &Path,
    include_source: bool,
) -> Value {
    let mut child = spawn_mcp_server(workspace_root, xdg_runtime_dir);
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stdout_rx = spawn_stdout_reader(stdout);

    let request = json!({
        "jsonrpc": "2.0",
        "id": 23,
        "method": "tools/call",
        "params": {
            "name": "anvil_symbol_context",
            "arguments": {
                "workspaceRoot": workspace_root,
                "target": {
                    "file": "src/greet.ts",
                    "kind": "Function",
                    "name": "greet",
                    "ordinal": 0
                },
                "includeSource": include_source,
                "tokenBudget": 500
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
        "mcp server must exit cleanly after symbol_context call; status: {status:?}",
    );

    serde_json::from_str(line.trim())
        .unwrap_or_else(|err| panic!("response must be JSON-RPC JSON, got {line:?}\nerror: {err}"))
}

fn snippet_texts(outcome: &Value) -> Vec<Option<String>> {
    outcome["snippets"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| row["snippet"]["text"].as_str().map(str::to_string))
        .collect()
}

/// MCP path: cold graph degrades to `not_ready` (CE-7 warming contract).
#[test]
fn mcp_symbol_context_not_ready_on_cold_graph() {
    temp_env::with_var_unset(GCTX_EGRESS_ENV, || {
        let workspace = tempfile::tempdir().expect("workspace");
        let canonical = prepare_workspace(&workspace);
        let state = Arc::new(save_time_state());
        let daemon = GctxDaemon::start(state);

        let parsed = call_symbol_context(&canonical, daemon.xdg_runtime_dir(), false);
        assert_eq!(parsed["result"]["isError"], false);
        let payload = parse_tool_payload(&parsed);
        assert_eq!(payload["outcome"]["status"], "not_ready");
        assert!(payload.get("workspaceRoot").is_some());
    });
}

/// MCP path: identity-only when snippet egress is not opted in (CE-1 default).
#[test]
fn mcp_symbol_context_identity_only_without_snippet_egress() {
    temp_env::with_var_unset(GCTX_EGRESS_ENV, || {
        let workspace = tempfile::tempdir().expect("workspace");
        let canonical = prepare_workspace(&workspace);
        let state = Arc::new(save_time_state());
        let daemon = GctxDaemon::start(state);
        warm_graph_over_daemon_socket(&daemon.socket_path(), &canonical);

        let parsed = call_symbol_context(&canonical, daemon.xdg_runtime_dir(), true);
        assert_eq!(parsed["result"]["isError"], false);
        let payload = parse_tool_payload(&parsed);
        let outcome = &payload["outcome"];
        assert!(
            matches!(
                outcome["status"].as_str(),
                Some("ready") | Some("bounded") | Some("budget_exceeded")
            ),
            "warm graph must project: {outcome}",
        );
        assert!(
            outcome["snippets"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty()),
            "expected snippet rows: {outcome}",
        );
        for text in snippet_texts(outcome) {
            assert!(
                text.is_none(),
                "CE-1: MCP must not return text without ANVIL_GCTX_EGRESS=1",
            );
        }
        assert!(outcome.get("redaction_summary").is_some());
    });
}

/// MCP path: gated text + redaction summary when egress is on and capability asserted.
#[test]
fn mcp_symbol_context_emits_text_with_egress_and_capability() {
    temp_env::with_var(GCTX_EGRESS_ENV, Some("1"), || {
        let workspace = tempfile::tempdir().expect("workspace");
        let canonical = prepare_workspace(&workspace);
        let state = Arc::new(save_time_state());
        let daemon = GctxDaemon::start(state);
        warm_graph_over_daemon_socket(&daemon.socket_path(), &canonical);

        let parsed = call_symbol_context(&canonical, daemon.xdg_runtime_dir(), true);
        assert_eq!(parsed["result"]["isError"], false);
        let payload = parse_tool_payload(&parsed);
        let outcome = &payload["outcome"];
        assert!(
            matches!(
                outcome["status"].as_str(),
                Some("ready") | Some("bounded") | Some("budget_exceeded")
            ),
            "warm graph must project: {outcome}",
        );
        let texts: Vec<String> = snippet_texts(outcome).into_iter().flatten().collect();
        assert!(
            !texts.is_empty(),
            "gated MCP path must return snippet text: {outcome}",
        );
        assert!(texts.iter().any(|t| t.contains("greet")));
        let summary = &outcome["redaction_summary"];
        assert!(summary.get("estimated_tokens").is_some());
        assert!(summary.get("outcome").is_some());
    });
}

/// Determinism: two consecutive MCP calls observe the same primary outcome label.
#[test]
fn mcp_symbol_context_is_deterministic_across_calls() {
    temp_env::with_var(GCTX_EGRESS_ENV, Some("1"), || {
        let workspace = tempfile::tempdir().expect("workspace");
        let canonical = prepare_workspace(&workspace);
        let state = Arc::new(save_time_state());
        let daemon = GctxDaemon::start(state);
        warm_graph_over_daemon_socket(&daemon.socket_path(), &canonical);

        let first = parse_tool_payload(&call_symbol_context(
            &canonical,
            daemon.xdg_runtime_dir(),
            true,
        ));
        let second = parse_tool_payload(&call_symbol_context(
            &canonical,
            daemon.xdg_runtime_dir(),
            true,
        ));

        assert_eq!(first["outcome"]["status"], second["outcome"]["status"]);
        let first_texts: Vec<String> = snippet_texts(&first["outcome"])
            .into_iter()
            .flatten()
            .collect();
        let second_texts: Vec<String> = snippet_texts(&second["outcome"])
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(first_texts, second_texts);
    });
}