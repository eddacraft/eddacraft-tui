#![cfg(all(unix, target_os = "linux"))]

//! MLP2-025b regression: prove that `run_foreground` installs the
//! env-tag spoof cross-check on the production listener.
//!
//! The unit tests in `ipc.rs` exercise `run_spoof_cross_check` directly
//! with a synthetic `CrossCheckContext`. That proves the function is
//! correct in isolation; it does not prove the production entry point
//! actually wires it up. `DeepSec` finding #1671 caught exactly that
//! gap: `IpcListener::with_cross_check_context` was defined but had
//! zero call sites, so `run_foreground` shipped with
//! `cross_check: None` and the scan-buffer handler silently skipped
//! the cross-check, falling through to the rule engine.
//!
//! These tests pin the wire-up. They go through the real socket via
//! `run_foreground` so a future refactor that drops the
//! `.with_cross_check_context` call from the builder chain trips a
//! regression.
//!
//! Linux-only because `worktree_for_lineage` walks `/proc` and the
//! spoof verdict on "registered ancestor" depends on that walk. The
//! "no env tag → fall through" case would work on macOS too, but
//! keeping both cases in one file under the same cfg keeps the harness
//! simple.

use std::path::PathBuf;
use std::time::Duration;

use anvil_intercept::{ForegroundOpts, Shutdown, run_foreground};
use anvil_intercept_proto::session::AgentTag;
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

/// Spawn `run_foreground` against a private socket + pid file + fence
/// store under `tmp`, wait for both side-effects to be observable, and
/// return `(shutdown, task handle, socket path, fence-store path)` so
/// the test can talk to the daemon over the socket and inspect
/// persisted fences after shutdown.
async fn spawn_daemon(
    tmp: &TempDir,
) -> (
    Shutdown,
    tokio::task::JoinHandle<anyhow::Result<()>>,
    PathBuf,
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

    (shutdown, handle, socket, fence_store)
}

async fn scan_buffer_with_env_tag(
    socket: &std::path::Path,
    file_path: &str,
    env_agent_tag: Option<&str>,
) -> Value {
    let mut stream = UnixStream::connect(socket).await.expect("connect socket");
    let mut params = serde_json::Map::new();
    params.insert("path".into(), json!(file_path));
    params.insert("text".into(), json!("const value = 1;\n"));
    params.insert("version".into(), json!(1));
    params.insert("mode".into(), json!("midEdit"));
    if let Some(tag) = env_agent_tag {
        params.insert("env_agent_tag".into(), json!(tag));
    }
    let frame = json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": Value::Object(params),
        "id": "wired-spoof",
    });
    let line = format!("{frame}\n");
    stream
        .write_all(line.as_bytes())
        .await
        .expect("write scan request");

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .expect("response timeout")
        .expect("read response");
    serde_json::from_str(response.trim_end()).expect("response json")
}

/// MLP2-025b wire-up: a `scan_buffer` request carrying a spoofed
/// `env_agent_tag` against a daemon with no registered ancestor for
/// the writer's PID must be blocked with `spoof_block` populated.
///
/// The writer's PID is this test process. No session is registered, so
/// the lineage walk reaches root without matching → `Cross::Spoofed`.
/// If `run_foreground` failed to install the cross-check, this request
/// would fall straight through to the rule engine and the response
/// would be a normal `scan_buffer` success without `spoof_block`.
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_blocks_spoofed_env_tag() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let writer_workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&writer_workspace).expect("create workspace");

    let (shutdown, handle, socket, fence_store) = spawn_daemon(&tmp).await;

    // A forged env-tag claiming the writer's PID has a daemon-issued
    // lineage anchor. No session was registered, so the registry's
    // lineage walk will not find a registered ancestor and the
    // cross-check returns `Cross::Spoofed`.
    let forged = AgentTag::new("anvil-run", "forged-session", 1_700_000_000);
    let encoded = anvil_attribution::env::agent_tag_to_env_value(&forged);

    let file = writer_workspace.join("x.rs");
    let response = scan_buffer_with_env_tag(&socket, file.to_str().unwrap(), Some(&encoded)).await;

    let result = response
        .get("result")
        .unwrap_or_else(|| panic!("expected result, got {response:?}"));

    let spoof_block = result.get("spoof_block").unwrap_or_else(|| {
        panic!(
            "scan_buffer with spoofed env_agent_tag returned no spoof_block — \
             the cross-check is not wired into run_foreground. response={result:?}"
        )
    });

    assert!(
        !spoof_block.is_null(),
        "spoof_block must be populated, not null"
    );
    assert_eq!(
        spoof_block["reason"],
        json!("degraded:spoofed-attribution"),
        "spoof_block.reason must surface the documented degraded reason"
    );
    assert!(
        spoof_block.get("fenced_worktree").is_some(),
        "spoof_block must name the fenced worktree"
    );

    // Confirm the side-effect fence was recorded — the fail-closed
    // verdict is "block AND fence", not "block alone".
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("daemon shutdown timed out")
        .expect("daemon task join failure")
        .expect("daemon run_foreground reported error");

    let fence_json = std::fs::read_to_string(&fence_store)
        .unwrap_or_else(|e| panic!("read fence store {}: {e}", fence_store.display()));
    assert!(
        fence_json.contains("degraded:spoofed-attribution"),
        "fence store must record the spoof-attribution fence; got: {fence_json}"
    );
}

/// MLP2-025b wire-up: a `scan_buffer` request with no `env_agent_tag`
/// follows the pre-MLP2-025 path — `Cross::Untagged` falls straight
/// through to the rule engine, no `spoof_block` in the response.
///
/// Pin both halves of the wire contract: the cross-check IS installed
/// (proved by the other test) AND the untagged-write fast path still
/// works.
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_passes_through_untagged_writes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (shutdown, handle, socket, _fence_store) = spawn_daemon(&tmp).await;

    let response = scan_buffer_with_env_tag(&socket, "src/x.rs", None).await;

    let result = response
        .get("result")
        .unwrap_or_else(|| panic!("expected result, got {response:?}"));
    assert!(
        result.get("spoof_block").is_none_or(Value::is_null),
        "untagged writes must not trip the spoof cross-check; got result={result:?}"
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("daemon shutdown timed out")
        .expect("daemon task join failure")
        .expect("daemon run_foreground reported error");
}
