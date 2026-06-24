#![cfg(all(unix, target_os = "linux"))]

//! GCTX-023 wire-up regression: `anvil/gctx/symbol_context` and
//! `anvil/gctx/get_snippet` are served end-to-end over a real socket with a
//! warm, span-bearing graph. Pins CE-1 identity-only vs gated text, CE-7 cold
//! degradation, and dispatch routing (the #1671 builder-never-called pattern).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anvil_checks::antipattern::types::AntipatternCheckConfig;
use anvil_gctx_types::GCTX_EGRESS_ENV;
use anvil_intercept::Shutdown;
use anvil_intercept::confinement::Confinement;
use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
use anvil_intercept::save_time::{SaveTimeState, SymbolParser};
use anvil_intercept::workspace_pool::WorkScheduler;
use anvil_intercept_proto::protocol::{ANVIL_GCTX_GET_SNIPPET, ANVIL_GCTX_SYMBOL_CONTEXT};
use anvil_kernel_types::{
    ByteRange, FileSymbols, SymbolKind, SymbolNode, TrustLevel, Visibility, content_hash,
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::sleep;

const GREET_SOURCE: &[u8] = b"export function greet() { return 1; }\n";

/// Parser that stamps span + content hash so GCTX-021/023 can extract snippets.
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

fn workspace(tmp: &TempDir) -> String {
    let root = tmp.path().join("wt");
    std::fs::create_dir_all(root.join("src")).expect("mkdir");
    std::fs::write(root.join("src/greet.ts"), GREET_SOURCE).expect("write");
    std::fs::canonicalize(&root)
        .expect("canonicalise")
        .to_string_lossy()
        .into_owned()
}

async fn warm_graph_over_socket(socket: &Path, root: &str) {
    let validate = request(
        socket,
        "anvil/validate_paths",
        json!({
            "workspace_root": root,
            "paths": [{ "path": "src/greet.ts", "change": "modified" }],
        }),
    )
    .await;
    assert!(
        validate.get("error").is_none(),
        "validate_paths must warm the graph: {validate}",
    );

    for i in 0..300 {
        let status = request(
            socket,
            "anvil/workspace_status",
            json!({ "workspace_root": root, "id": format!("warm-{i}") }),
        )
        .await;
        let state = status
            .pointer("/result/workspace_assurance/state")
            .and_then(Value::as_str);
        if matches!(state, Some("stale") | Some("clean") | Some("bounded")) {
            return;
        }
        sleep(Duration::from_millis(10)).await;
    }
    panic!("graph never became readable after validate_paths");
}

fn symbol_context_params(root: &str, include_source: bool) -> Value {
    json!({
        "workspace_root": root,
        "query": {
            "selector": {
                "symbol": {
                    "file": "src/greet.ts",
                    "kind": "Function",
                    "name": "greet",
                    "ordinal": 0
                }
            },
            "include_source": include_source,
            "token_budget": 500
        }
    })
}

fn get_snippet_params(root: &str, include_source: bool) -> Value {
    json!({
        "workspace_root": root,
        "query": {
            "target": {
                "file": "src/greet.ts",
                "kind": "Function",
                "name": "greet",
                "ordinal": 0
            },
            "include_source": include_source
        }
    })
}

fn snippet_texts(outcome: &Value) -> Vec<Option<String>> {
    outcome["snippets"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| row["snippet"]["text"].as_str().map(str::to_string))
        .collect()
}

struct GctxListener {
    shutdown: Shutdown,
    handle: tokio::task::JoinHandle<Result<(), anvil_intercept::ipc::IpcError>>,
    socket: PathBuf,
    _socket_dir: PathBuf,
}

impl GctxListener {
    async fn start(state: Arc<SaveTimeState>) -> Self {
        let dir = tempfile::tempdir().expect("socket tempdir");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("secure tempdir permissions");
        let socket = dir.path().join("intercept.sock");
        let _socket_dir = dir.keep();

        let listener = IpcListener::bind(&socket, NoopDispatcher)
            .expect("bind gctx listener")
            .with_save_time_state(state);
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(listener.serve(token));
        for _ in 0..200 {
            if socket.exists() {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
        Self {
            shutdown,
            handle,
            socket,
            _socket_dir,
        }
    }

    async fn shutdown(self) {
        self.shutdown.trigger();
        let _ = tokio::time::timeout(Duration::from_secs(5), self.handle).await;
    }
}

async fn request(socket: &Path, method: &str, params: Value) -> Value {
    let mut stream = UnixStream::connect(socket).await.expect("connect socket");
    let frame = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": "gctx-symbol-context-wired",
    });
    stream
        .write_all(format!("{frame}\n").as_bytes())
        .await
        .expect("write request");

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .expect("response timeout")
        .expect("read response");
    serde_json::from_str(response.trim_end()).expect("response json")
}

/// CE-7: a cold worktree degrades in-band to `not_ready` over the real socket.
#[test]
fn symbol_context_not_ready_on_cold_worktree() {
    temp_env::with_var_unset(GCTX_EGRESS_ENV, || {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
        let tmp = TempDir::new().expect("tempdir");
        let root = workspace(&tmp);
        let state = Arc::new(save_time_state());
        let listener = GctxListener::start(state).await;

        let response = request(
            &listener.socket,
            ANVIL_GCTX_SYMBOL_CONTEXT,
            symbol_context_params(&root, false),
        )
        .await;

        assert!(
            response.get("error").is_none(),
            "symbol_context must route to the gctx arm, got error: {response}",
        );
        assert_eq!(response["result"]["outcome"]["status"], "not_ready");
        listener.shutdown().await;
            });
    });
}

/// GCTX-021: `get_snippet` routes over the socket and degrades cold → `not_ready`.
#[test]
fn get_snippet_not_ready_on_cold_worktree() {
    temp_env::with_var_unset(GCTX_EGRESS_ENV, || {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
        let tmp = TempDir::new().expect("tempdir");
        let root = workspace(&tmp);
        let state = Arc::new(save_time_state());
        let listener = GctxListener::start(state).await;

        let response = request(
            &listener.socket,
            ANVIL_GCTX_GET_SNIPPET,
            get_snippet_params(&root, false),
        )
        .await;

        assert!(
            response.get("error").is_none(),
            "get_snippet must route to the gctx arm, got error: {response}",
        );
        assert_eq!(response["result"]["outcome"]["status"], "not_ready");
        listener.shutdown().await;
            });
    });
}

/// CE-1: with a warm graph but snippet egress off (unset `ANVIL_GCTX_EGRESS`),
/// `include_source` does not emit text — span-as-location only.
#[test]
fn symbol_context_identity_only_without_snippet_egress() {
    temp_env::with_var_unset(GCTX_EGRESS_ENV, || {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
        let tmp = TempDir::new().expect("tempdir");
        let root = workspace(&tmp);
        let state = Arc::new(save_time_state());
        let listener = GctxListener::start(Arc::clone(&state)).await;
        warm_graph_over_socket(&listener.socket, &root).await;

        let response = request(
            &listener.socket,
            ANVIL_GCTX_SYMBOL_CONTEXT,
            symbol_context_params(&root, true),
        )
        .await;

        assert!(
            response.get("error").is_none(),
            "symbol_context must succeed in-band: {response}",
        );
        let outcome = &response["result"]["outcome"];
        assert!(
            matches!(
                outcome["status"].as_str(),
                Some("ready") | Some("bounded") | Some("budget_exceeded")
            ),
            "warm graph must project context, got: {outcome}",
        );
        assert!(
            outcome["snippets"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty()),
            "span-bearing seed must produce snippet rows: {outcome}",
        );
        for text in snippet_texts(outcome) {
            assert!(
                text.is_none(),
                "CE-1: text must be absent without ANVIL_GCTX_EGRESS=1, got: {text:?}",
            );
        }
        assert!(
            outcome.get("redaction_summary").is_some(),
            "sealed DTO must carry counts-only redaction_summary: {outcome}",
        );
        listener.shutdown().await;
            });
    });
}

/// CE-1 + CE-11: with `ANVIL_GCTX_EGRESS=1` and `include_source`, snippets carry
/// fresh source text and the redaction summary is populated.
#[test]
fn symbol_context_emits_text_with_egress_and_capability() {
    temp_env::with_var(GCTX_EGRESS_ENV, Some("1"), || {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
        let tmp = TempDir::new().expect("tempdir");
        let root = workspace(&tmp);
        let state = Arc::new(save_time_state());
        let listener = GctxListener::start(Arc::clone(&state)).await;
        warm_graph_over_socket(&listener.socket, &root).await;

        let response = request(
            &listener.socket,
            ANVIL_GCTX_SYMBOL_CONTEXT,
            symbol_context_params(&root, true),
        )
        .await;

        assert!(
            response.get("error").is_none(),
            "symbol_context must succeed in-band: {response}",
        );
        let outcome = &response["result"]["outcome"];
        assert!(
            matches!(
                outcome["status"].as_str(),
                Some("ready") | Some("bounded") | Some("budget_exceeded")
            ),
            "warm graph must project context, got: {outcome}",
        );
        let texts: Vec<String> = snippet_texts(outcome).into_iter().flatten().collect();
        assert!(
            !texts.is_empty(),
            "gated path must return at least one snippet with text: {outcome}",
        );
        assert!(
            texts.iter().any(|t| t.contains("greet")),
            "snippet text must include the symbol body: {texts:?}",
        );
        let summary = &outcome["redaction_summary"];
        assert!(summary.get("estimated_tokens").is_some());
        assert!(summary.get("outcome").is_some());
        listener.shutdown().await;
            });
    });
}

/// `get_snippet` mirrors the same CE-1 gate on the dedicated RPC.
#[test]
fn get_snippet_emits_text_with_egress_and_capability() {
    temp_env::with_var(GCTX_EGRESS_ENV, Some("1"), || {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
        let tmp = TempDir::new().expect("tempdir");
        let root = workspace(&tmp);
        let state = Arc::new(save_time_state());
        let listener = GctxListener::start(Arc::clone(&state)).await;
        warm_graph_over_socket(&listener.socket, &root).await;

        let response = request(
            &listener.socket,
            ANVIL_GCTX_GET_SNIPPET,
            get_snippet_params(&root, true),
        )
        .await;

        assert!(
            response.get("error").is_none(),
            "get_snippet must succeed in-band: {response}",
        );
        let snippet = &response["result"]["outcome"];
        assert_eq!(snippet["status"], "ready");
        let text = snippet["text"]
            .as_str()
            .expect("gated get_snippet must carry text");
        assert!(text.contains("greet"));
        listener.shutdown().await;
            });
    });
}