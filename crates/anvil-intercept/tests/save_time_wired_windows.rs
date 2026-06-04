#![cfg(windows)]

//! DSV-010b wire-up regression (Windows): prove the three save-time verbs are
//! served end to end by the production `run_foreground` listener over a real
//! named pipe — the Windows parity of the Linux `save_time_wired.rs`.
//!
//! `run_foreground` builds the `SaveTimeState` and calls `.with_save_time_state`
//! on the Windows listener too (ADR-070 Stage 2); a refactor that drops that
//! builder call or the dispatch arm would trip a regression here, the same way
//! the Linux test guards the Unix path (the #1671 lesson — a builder defined but
//! never called ships an inert feature).
//!
//! The reads go through the `WorkspaceAnchor` Windows arm (the ADR-068
//! `NtCreateFile`/`OBJ_DONT_REPARSE` guard), so this exercises the full
//! pipe → dispatch → admission → guarded-read → verdict → wire path. No
//! `SymbolParser` is injected on Windows yet, so verdicts are the safe
//! `Partial` — the documented degraded mode Unix also uses without a parser.
//!
//! Per-PID pipe name so parallel test cases never collide on a shared per-user
//! daemon pipe (the MLP2-075 rationale).

use std::path::PathBuf;
use std::time::Duration;

use anvil_intercept::{ForegroundOpts, Shutdown, run_foreground};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::time::sleep;

/// `ERROR_PIPE_BUSY` (231): all pipe instances are momentarily busy between the
/// server accepting one client and creating the next instance — retry.
const ERROR_PIPE_BUSY: i32 = 231;

fn test_pid_file(tmp: &TempDir) -> PathBuf {
    tmp.path().join("anvil").join("intercept.pid")
}

/// Per-case pipe name. The harness runs the two tests in this binary in parallel
/// (same PID), so the PID alone is not unique — a distinct `case` suffix per test
/// keeps each daemon's `PipeInstance::First` bind from colliding (the Unix
/// counterpart gets uniqueness for free from its per-test `TempDir` socket path).
fn test_pipe_name(case: &str) -> String {
    format!(
        r"\\.\pipe\anvil-intercept-save-time-wired-{}-{case}",
        std::process::id(),
    )
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
    pipe_name: &str,
) -> (Shutdown, tokio::task::JoinHandle<anyhow::Result<()>>) {
    let pid_file = test_pid_file(tmp);
    let fence_store = tmp.path().join("state/intercept-fences.json");

    let (shutdown, token) = Shutdown::new();
    let handle = tokio::spawn(run_foreground(
        ForegroundOpts::with_pid_file_and_ipc_pipe_name(&pid_file, pipe_name)
            .with_fence_store_file(&fence_store),
        token,
    ));

    wait_for_path(&pid_file).await;
    (shutdown, handle)
}

/// Connect to the daemon pipe, retrying while the daemon is still binding the
/// first instance or all instances are momentarily busy.
async fn connect(pipe_name: &str) -> NamedPipeClient {
    for _ in 0..200 {
        match ClientOptions::new().open(pipe_name) {
            Ok(client) => return client,
            // Not yet bound, or all instances busy: wait and retry.
            Err(err)
                if err.raw_os_error() == Some(ERROR_PIPE_BUSY)
                    || err.kind() == std::io::ErrorKind::NotFound =>
            {
                sleep(Duration::from_millis(10)).await;
            }
            Err(err) => panic!("unexpected pipe connect error: {err}"),
        }
    }
    panic!("named pipe never became connectable: {pipe_name}");
}

/// Send one JSON-RPC request over a fresh pipe connection and return the parsed
/// response.
async fn request(pipe_name: &str, method: &str, params: Value) -> Value {
    let client = connect(pipe_name).await;
    let frame = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": "save-time-wired-win",
    });

    let mut reader = BufReader::new(client);
    reader
        .get_mut()
        .write_all(format!("{frame}\n").as_bytes())
        .await
        .expect("write request");

    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .expect("response timeout")
        .expect("read response");
    serde_json::from_str(response.trim_end()).expect("response json")
}

/// A real workspace with one source file; returns the canonical root (the daemon
/// canonicalises the admitted root and keys the verdict on it).
fn workspace(tmp: &TempDir) -> String {
    let root = tmp.path().join("wt");
    std::fs::create_dir_all(root.join("src")).expect("mkdir");
    std::fs::write(root.join("src/a.ts"), b"export const value = 1;\n").expect("write");
    std::fs::canonicalize(&root)
        .expect("canonicalise")
        .to_string_lossy()
        .into_owned()
}

/// `anvil/validate_paths` over the real named pipe returns a verdict-shaped
/// response: a coalesced `evaluated[]` echoing the daemon-computed hash (proof
/// the Windows guard read the guarded bytes), the frozen `check_families`, and
/// (with no parser wired) a safe `Partial` over a `stale` workspace.
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_serves_validate_paths_over_named_pipe() {
    let tmp = TempDir::new().expect("tempdir");
    let pipe_name = test_pipe_name("validate-paths");
    let (shutdown, handle) = spawn_daemon(&tmp, &pipe_name).await;
    let root = workspace(&tmp);

    let response = request(
        &pipe_name,
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
        "the daemon read the guarded bytes through the Windows anchor and echoed \
         ITS hash: {result}",
    );
    assert_eq!(result["workspace_assurance"]["state"], "stale");

    shutdown.trigger();
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
}

/// `anvil/workspace_status` reports the cold workspace as `stale` (B6 — never
/// `clean` on first contact), and `anvil/request_full_scan` queues a scan
/// (`pending`). Both over the real named pipe.
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_serves_status_and_full_scan_over_named_pipe() {
    let tmp = TempDir::new().expect("tempdir");
    let pipe_name = test_pipe_name("status-full-scan");
    let (shutdown, handle) = spawn_daemon(&tmp, &pipe_name).await;
    let root = workspace(&tmp);

    let status = request(
        &pipe_name,
        "anvil/workspace_status",
        json!({ "workspace_root": root }),
    )
    .await;
    assert!(status.get("error").is_none(), "status routes: {status}");
    assert_eq!(status["result"]["workspace_assurance"]["state"], "stale");

    let scan = request(
        &pipe_name,
        "anvil/request_full_scan",
        json!({ "workspace_root": root }),
    )
    .await;
    assert!(scan.get("error").is_none(), "full_scan routes: {scan}");
    assert_eq!(scan["result"]["workspace_assurance"]["state"], "pending");

    shutdown.trigger();
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
}
