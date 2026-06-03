#![cfg(all(unix, target_os = "linux"))]

//! DSV-005 wire-up regression: prove the three save-time verbs are served
//! end to end by the production `run_foreground` listener over a real socket.
//!
//! The unit / component tests exercise `SaveTimeConn` and the JSON-RPC dispatch
//! arm in-process. They prove the orchestration is correct; they do not prove
//! the production entry point actually routes the verbs. This pins that: a
//! refactor that drops the `.with_save_time_state` builder call or the dispatch
//! arm would trip a regression here (the lesson of #1671 — a builder defined but
//! never called shipped an inert feature).
//!
//! No `SymbolParser` is injected (the kernel-backed impl lives in `anvil-cli`,
//! which this crate cannot depend on), so verdicts are the safe `Partial` — the
//! full socket → dispatch → admission → guarded-read → verdict → wire path is
//! still exercised; only the parse step is absent.

use std::path::PathBuf;
use std::time::Duration;

use anvil_intercept::{ForegroundOpts, Shutdown, run_foreground};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::sleep;

fn test_pid_file(tmp: &TempDir) -> PathBuf {
    tmp.path().join("anvil").join("intercept.pid")
}

fn test_ipc_socket(tmp: &TempDir) -> PathBuf {
    tmp.path().join("ipc").join("intercept.sock")
}

async fn wait_for_path(path: &std::path::Path) {
    for _ in 0..200 {
        if path.exists() {
            return;
        }
        sleep(Duration::from_millis(10)).await;
    }
    panic!("path did not appear: {}", path.display());
}

async fn spawn_daemon(
    tmp: &TempDir,
) -> (
    Shutdown,
    tokio::task::JoinHandle<anyhow::Result<()>>,
    PathBuf,
) {
    let pid_file = test_pid_file(tmp);
    let socket = test_ipc_socket(tmp);
    let fence_store = tmp.path().join("state/intercept-fences.json");

    let (shutdown, token) = Shutdown::new();
    let handle = tokio::spawn(run_foreground(
        ForegroundOpts::with_pid_file_and_ipc_socket(&pid_file, &socket)
            .with_fence_store_file(&fence_store),
        token,
    ));

    wait_for_path(&pid_file).await;
    wait_for_path(&socket).await;

    (shutdown, handle, socket)
}

/// Send one JSON-RPC request over the socket and return the parsed response.
async fn request(socket: &std::path::Path, method: &str, params: Value) -> Value {
    let mut stream = UnixStream::connect(socket).await.expect("connect socket");
    let frame = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": "save-time-wired",
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

/// A real workspace with one source file; returns the canonical root (the
/// daemon canonicalises the admitted root and keys the verdict on it).
fn workspace(tmp: &TempDir) -> String {
    let root = tmp.path().join("wt");
    std::fs::create_dir_all(root.join("src")).expect("mkdir");
    std::fs::write(root.join("src/a.ts"), b"export const value = 1;\n").expect("write");
    std::fs::canonicalize(&root)
        .expect("canonicalise")
        .to_string_lossy()
        .into_owned()
}

/// `anvil/validate_paths` over the real socket returns a verdict-shaped
/// response: a coalesced `evaluated[]` echoing the daemon-computed hash, the
/// frozen `check_families`, and (with no parser wired) a safe `Partial` over a
/// `stale` workspace.
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_serves_validate_paths() {
    let tmp = TempDir::new().expect("tempdir");
    let (shutdown, handle, socket) = spawn_daemon(&tmp).await;
    let root = workspace(&tmp);

    let response = request(
        &socket,
        "anvil/validate_paths",
        json!({
            "workspace_root": root,
            "paths": [{"path": "src/a.ts", "change": "modified"}],
        }),
    )
    .await;

    assert!(
        response.get("error").is_none(),
        "validate_paths must route to the save-time arm, got error: {response}",
    );
    let result = &response["result"];
    assert_eq!(result["coverage"], "partial", "no parser ⇒ safe Partial");
    assert_eq!(result["check_families"][0], "antipattern");
    assert_eq!(result["evaluated"][0]["path"], "src/a.ts");
    assert!(
        result["evaluated"][0]["content_hash"].is_string(),
        "the daemon read the guarded bytes and echoed ITS hash: {result}",
    );
    assert_eq!(result["workspace_assurance"]["state"], "stale");

    shutdown.trigger();
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
}

/// `anvil/workspace_status` reports the cold workspace as `stale`
/// (B6 — never `clean` on first contact), and `anvil/request_full_scan`
/// queues a scan (`pending`). Both over the real socket.
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_serves_status_and_full_scan() {
    let tmp = TempDir::new().expect("tempdir");
    let (shutdown, handle, socket) = spawn_daemon(&tmp).await;
    let root = workspace(&tmp);

    let status = request(
        &socket,
        "anvil/workspace_status",
        json!({ "workspace_root": root }),
    )
    .await;
    assert!(status.get("error").is_none(), "status routes: {status}");
    assert_eq!(status["result"]["workspace_assurance"]["state"], "stale");

    let scan = request(
        &socket,
        "anvil/request_full_scan",
        json!({ "workspace_root": root }),
    )
    .await;
    assert!(scan.get("error").is_none(), "full_scan routes: {scan}");
    assert_eq!(scan["result"]["workspace_assurance"]["state"], "pending");

    shutdown.trigger();
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
}
