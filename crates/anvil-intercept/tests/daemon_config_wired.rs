#![cfg(unix)]

//! Daemon-config wire-up regression. Companion to
//! `spoof_cross_check_wired.rs`.
//!
//! `DeepSec` finding #1671 caught one instance of a wider pattern: a
//! capability builder on a production constructor with zero callers,
//! making the capability dead in production despite green unit tests.
//! The audit grep that followed surfaced three siblings:
//!
//! * `IpcListener::with_limits` (INTD-016 IPC `DoS` budgets).
//! * `SessionRegistry::with_per_worktree_cap` (MLP2-024 session cap
//!   configurability — the cap shipped at `DEFAULT_PER_WORKTREE_MAX`
//!   regardless of `enforcement.session.per_worktree_max` in
//!   `.anvil.yaml`).
//! * `SessionRegistry::with_unregister_hook` (MLP2-057). Now WIRED (DSV):
//!   `run_foreground` installs the hook via the post-construction
//!   `SessionRegistry::set_unregister_hook` so an unregistered session
//!   drops its worktree's warm `SaveTimeState` (graph cache + assurance
//!   machine). `RuleSetCache::invalidate` remains its second intended
//!   consumer — when MLP2-014 constructs that cache it joins the same
//!   composed closure. Pinned below by
//!   `run_foreground_reclaims_warm_state_on_unregister`.
//!
//! These tests pin the wire-up by going through the real socket via
//! `run_foreground`. They would have failed on `main` before the fix,
//! and they trip if a future refactor drops the chained builder call.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anvil_intercept::config::Resolved;
use anvil_intercept::dos::IpcLimits;
use anvil_intercept::save_time::SymbolParser;
use anvil_intercept::{ForegroundOpts, Shutdown, run_foreground};
use anvil_kernel_types::{FileSymbols, SymbolKind, SymbolNode, TrustLevel, Visibility};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::sleep;

/// A deterministic stub parser for the warm-state-reclamation probes: every file
/// parses to one public function, so the DSV-045 executor reaches a warm `Clean`
/// state (the distinguishable marker these tests need now that `request_full_scan`
/// is actually driven to a terminal state rather than left `Pending`).
#[derive(Debug)]
struct StubParser;

impl SymbolParser for StubParser {
    fn parse(&self, path: &Path, _bytes: &[u8]) -> Option<FileSymbols> {
        let file = path.to_string_lossy().into_owned();
        Some(FileSymbols {
            file: file.clone(),
            symbols: vec![SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "f".to_string(),
                visibility: Visibility::Public,
                file,
                trust_level: TrustLevel::Unknown,
                span: None,
            }],
            imports: Vec::new(),
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
        })
    }
}

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

async fn spawn_daemon_with_config(
    tmp: &TempDir,
    config: Resolved,
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
            .with_fence_store_file(&fence_store)
            .with_enforcement_config(config),
        token,
    ));

    wait_for_path(&pid_file).await;
    wait_for_path(&socket).await;

    (shutdown, handle, socket)
}

/// Spawn a daemon with a stub [`SymbolParser`] injected, so the DSV-045
/// full-scan executor can drive `request_full_scan` to a warm `Clean` state —
/// the distinguishable marker the warm-state-reclamation probes rely on.
async fn spawn_daemon_with_parser(
    tmp: &TempDir,
    config: Resolved,
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
            .with_fence_store_file(&fence_store)
            .with_enforcement_config(config)
            .with_symbol_parser(Arc::new(StubParser)),
        token,
    ));

    wait_for_path(&pid_file).await;
    wait_for_path(&socket).await;

    (shutdown, handle, socket)
}

async fn send_register_request(
    socket: &std::path::Path,
    request_id: &str,
    session_id: &str,
    worktree: &std::path::Path,
    claimed_agent_id: &str,
) -> Value {
    let mut stream = UnixStream::connect(socket).await.expect("connect socket");
    // MLP2-023: a distinct `agent_tag` per session is required for
    // multiple sessions to coexist on the same worktree. Without it,
    // the second register hits `WorktreeAlreadyOwned` (legacy single-
    // session path) before the per-worktree cap check fires. The cap
    // test sends three distinct tags so the third register reaches
    // the MLP2-024 cap branch and surfaces SessionCapExceeded.
    let frame = json!({
        "jsonrpc": "2.0",
        "method": "session.register",
        "params": {
            "session_id": session_id,
            "worktree": worktree.to_str().expect("utf-8 worktree path"),
            "agent_tag": {
                "driver_id": "anvil-run",
                "claimed_agent_id": claimed_agent_id,
                "pid_starttime": 1_700_000_000_u64,
            },
        },
        "id": request_id,
    });
    let line = format!("{frame}\n");
    stream
        .write_all(line.as_bytes())
        .await
        .expect("write register frame");

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .expect("response timeout")
        .expect("read response");
    serde_json::from_str(response.trim_end()).expect("response json")
}

/// Send one JSON-RPC frame over a fresh connection and return the parsed
/// response. Each call opens its own socket (admission is per-connection), so
/// the save-time verbs re-admit their root each time.
async fn send_jsonrpc(
    socket: &std::path::Path,
    request_id: &str,
    method: &str,
    params: Value,
) -> Value {
    let mut stream = UnixStream::connect(socket).await.expect("connect socket");
    let frame = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": request_id,
    });
    let line = format!("{frame}\n");
    stream
        .write_all(line.as_bytes())
        .await
        .expect("write jsonrpc frame");

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .expect("response timeout")
        .expect("read response");
    serde_json::from_str(response.trim_end()).expect("response json")
}

/// The `result.workspace_assurance.state` of a save-time verb response.
fn assurance_state(resp: &Value) -> Option<&str> {
    resp.pointer("/result/workspace_assurance/state")
        .and_then(Value::as_str)
}

/// Poll `workspace_status` until the worktree reaches `clean` (the DSV-045
/// executor drives `Pending → Running → Clean` on the background pool, so the
/// warm state is observable only once the scan settles). Panics on timeout.
async fn poll_until_clean(socket: &std::path::Path, wt_str: &str) {
    for i in 0..300 {
        let resp = send_jsonrpc(
            socket,
            &format!("poll-{i}"),
            "anvil/workspace_status",
            json!({ "workspace_root": wt_str }),
        )
        .await;
        if assurance_state(&resp) == Some("clean") {
            return;
        }
        sleep(Duration::from_millis(10)).await;
    }
    panic!("worktree did not reach a warm `clean` state within the timeout");
}

/// MLP2-024 wire-up: a daemon launched with
/// `Resolved.session_per_worktree_max = 2` must refuse the third
/// concurrent registration on the same worktree. Before the fix,
/// `run_foreground` constructed the `SessionRegistry` via
/// `SessionRegistry::new()` and ignored `enforcement_config`, so the
/// cap always ran at the compile-time default (16). A test that asks
/// for cap = 2 and then registers 3 sessions would silently succeed
/// — the bug.
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_applies_session_per_worktree_cap_from_config() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let worktree = tmp.path().join("worktree");
    std::fs::create_dir(&worktree).expect("create worktree");

    let config = Resolved {
        session_per_worktree_max: 2,
        ..Resolved::default()
    };
    let (shutdown, handle, socket) = spawn_daemon_with_config(&tmp, config).await;

    let first = send_register_request(&socket, "1", "sess-cap-1", &worktree, "claude-1").await;
    assert!(
        first.get("result").is_some() && first.get("error").is_none(),
        "first registration must succeed; got {first:?}",
    );

    let second = send_register_request(&socket, "2", "sess-cap-2", &worktree, "claude-2").await;
    assert!(
        second.get("result").is_some() && second.get("error").is_none(),
        "second registration must succeed; got {second:?}",
    );

    let third = send_register_request(&socket, "3", "sess-cap-3", &worktree, "claude-3").await;
    let err = third.get("error").unwrap_or_else(|| {
        panic!(
            "third registration must be refused once the per-worktree cap is hit, \
             but the daemon returned a success. This means \
             `run_foreground` is not applying `Resolved.session_per_worktree_max` — \
             the MLP2-024 builder is inert in production. Response: {third:?}"
        )
    });
    let message = err.get("message").and_then(Value::as_str).unwrap_or("");
    let data_error = err
        .get("data")
        .and_then(|d| d.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let combined = format!("{message} {data_error}");
    assert!(
        combined.contains("session cap exceeded"),
        "third registration must surface a cap-exceeded error; got error={err:?}"
    );
    assert!(
        data_error.contains("cap=2"),
        "rejection must echo the CONFIGURED cap value (2), not the daemon's \
         compile-time default ({}). data.error={data_error:?}",
        anvil_intercept::registry::DEFAULT_PER_WORKTREE_CAP,
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("daemon shutdown timed out")
        .expect("daemon task join failure")
        .expect("daemon run_foreground reported error");
}

/// MLP2-057 / DSV wire-up: a daemon must reclaim a worktree's warm save-time
/// state (graph cache + assurance machine) when its session leaves the
/// registry. Before the fix, `run_foreground` constructed the registry with no
/// unregister hook, so `SaveTimeState::invalidate` had no caller and the warm
/// state outlived the session — the MLP2-057 builder was inert in production
/// (the #1671 pattern).
///
/// The probe is deterministic and needs no symbol parser:
/// `request_full_scan` moves the assurance machine `Stale → Pending`, a
/// distinguishable warm state. After unregister, a fresh `workspace_status`
/// must see the cold-start machine (`Stale(CrossFileResolutionNeeded)`), not
/// the persisted `Pending`. An unwired daemon keeps the machine and still
/// reads `pending` here — failing the test.
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_reclaims_warm_state_on_unregister() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let worktree = tmp.path().join("worktree");
    std::fs::create_dir(&worktree).expect("create worktree");
    std::fs::write(worktree.join("a.ts"), b"export const x = 1;\n").expect("seed source file");
    let wt_str = worktree.to_str().expect("utf-8 worktree path");

    // A stub parser lets the DSV-045 executor reach a warm `Clean` — the
    // distinguishable marker (a no-parser daemon now settles `request_full_scan`
    // to `Stale`, which is indistinguishable from the cold-start state).
    let (shutdown, handle, socket) = spawn_daemon_with_parser(&tmp, Resolved::default()).await;

    // A session to unregister later — its worktree is the one we warm.
    let reg = send_register_request(&socket, "1", "sess-warm", &worktree, "claude-warm").await;
    assert!(
        reg.get("result").is_some() && reg.get("error").is_none(),
        "registration must succeed; got {reg:?}",
    );

    // Warm the worktree to a distinguishable `Clean` via the full-scan executor.
    send_jsonrpc(
        &socket,
        "2",
        "anvil/request_full_scan",
        json!({ "workspace_root": wt_str }),
    )
    .await;
    poll_until_clean(&socket, wt_str).await;

    let before = send_jsonrpc(
        &socket,
        "3",
        "anvil/workspace_status",
        json!({ "workspace_root": wt_str }),
    )
    .await;
    assert_eq!(
        assurance_state(&before),
        Some("clean"),
        "the warm Clean must be observable before unregister; got {before:?}",
    );

    // Unregister → the hook must drop the worktree's warm state.
    let unreg = send_jsonrpc(
        &socket,
        "4",
        "session.unregister",
        json!({ "session_id": "sess-warm" }),
    )
    .await;
    assert!(
        unreg.get("error").is_none(),
        "unregister must succeed; got {unreg:?}",
    );

    let after = send_jsonrpc(
        &socket,
        "5",
        "anvil/workspace_status",
        json!({ "workspace_root": wt_str }),
    )
    .await;
    assert_eq!(
        assurance_state(&after),
        Some("stale"),
        "warm state must be reclaimed on unregister: a fresh cold machine is \
         Stale, not the persisted Pending. A `pending` here means \
         `run_foreground` never installed the unregister hook — \
         warm-state reclamation is inert in production. Response: {after:?}",
    );
    assert_eq!(
        after
            .pointer("/result/workspace_assurance/reason")
            .and_then(Value::as_str),
        Some("cross-file-resolution-needed"),
        "the reclaimed worktree reports the cold-start reason; got {after:?}",
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("daemon shutdown timed out")
        .expect("daemon task join failure")
        .expect("daemon run_foreground reported error");
}

/// DSV-040 last-session semantics: warm state must SURVIVE while a peer session
/// still holds the worktree, and be reclaimed only when the LAST session leaves.
/// Two sub-agent sessions (distinct tags, MLP2-023) share one worktree;
/// unregistering the first must not drop the shared warm assurance machine.
/// A per-session hook would fail the mid-point assertion.
#[tokio::test(flavor = "current_thread")]
async fn warm_state_survives_until_last_session_unregisters() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let worktree = tmp.path().join("worktree");
    std::fs::create_dir(&worktree).expect("create worktree");
    std::fs::write(worktree.join("a.ts"), b"export const x = 1;\n").expect("seed source file");
    let wt_str = worktree.to_str().expect("utf-8 worktree path");

    let (shutdown, handle, socket) = spawn_daemon_with_parser(&tmp, Resolved::default()).await;

    let a = send_register_request(&socket, "1", "sess-a", &worktree, "claude-a").await;
    assert!(
        a.get("result").is_some() && a.get("error").is_none(),
        "register a: {a:?}",
    );
    let b = send_register_request(&socket, "2", "sess-b", &worktree, "claude-b").await;
    assert!(
        b.get("result").is_some() && b.get("error").is_none(),
        "register b: {b:?}",
    );

    // Warm the shared worktree to a distinguishable `Clean` via the executor.
    send_jsonrpc(
        &socket,
        "3",
        "anvil/request_full_scan",
        json!({ "workspace_root": wt_str }),
    )
    .await;
    poll_until_clean(&socket, wt_str).await;

    // First session leaves — the peer keeps the warm state.
    let unreg_a = send_jsonrpc(
        &socket,
        "4",
        "session.unregister",
        json!({ "session_id": "sess-a" }),
    )
    .await;
    assert!(unreg_a.get("error").is_none(), "unregister a: {unreg_a:?}");

    let mid = send_jsonrpc(
        &socket,
        "5",
        "anvil/workspace_status",
        json!({ "workspace_root": wt_str }),
    )
    .await;
    assert_eq!(
        assurance_state(&mid),
        Some("clean"),
        "warm state MUST survive while a peer session still holds the worktree \
         (last-session reclamation); a `stale` here means the hook fired \
         per-session and pulled warm state from under the live sibling. {mid:?}",
    );

    // Last session leaves — now reclaimed.
    let unreg_b = send_jsonrpc(
        &socket,
        "6",
        "session.unregister",
        json!({ "session_id": "sess-b" }),
    )
    .await;
    assert!(unreg_b.get("error").is_none(), "unregister b: {unreg_b:?}");

    let after = send_jsonrpc(
        &socket,
        "7",
        "anvil/workspace_status",
        json!({ "workspace_root": wt_str }),
    )
    .await;
    assert_eq!(
        assurance_state(&after),
        Some("stale"),
        "warm state reclaimed once the LAST session leaves; got {after:?}",
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("daemon shutdown timed out")
        .expect("daemon task join failure")
        .expect("daemon run_foreground reported error");
}

/// INTD-016 wire-up: a daemon launched with a tightened
/// `Resolved.ipc_limits.control_frame_max_bytes` must reject a
/// control-lane frame that exceeds the configured cap with the
/// documented JSON-RPC "Invalid Request" + `control-lane frame
/// exceeds <cap>-byte cap` reason. Before the fix, `run_foreground`
/// never called `.with_limits(...)`, so the listener always ran with
/// `IpcLimits::default()` (64 KiB control cap) and an oversized frame
/// against a tight configured cap would have been accepted.
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_applies_ipc_limits_from_config() {
    let tmp = tempfile::tempdir().expect("tempdir");

    // 256 is the floor enforced by `IpcLimits::from_config`. Choose it
    // so the test traps both halves of the wire-up: a missing
    // `.with_limits` chain leaves the listener at the 64 KiB default
    // and the assertion below fails; a present chain that forwards a
    // different field would also fail because the response carries
    // the configured cap value.
    let tight_cap_bytes: usize = 256;
    let config = Resolved {
        ipc_limits: IpcLimits {
            control_frame_max_bytes: tight_cap_bytes,
            ..IpcLimits::default()
        },
        ..Resolved::default()
    };
    let (shutdown, handle, socket) = spawn_daemon_with_config(&tmp, config).await;

    // Build a frame guaranteed to exceed 256 bytes by padding the
    // session_id with a long-but-still-valid filler. The JSON envelope
    // plus padding lands well past the cap while staying well under
    // the legacy `scan_buffer` cap that handles non-control oversize.
    let padding = "x".repeat(512);
    let oversized_session_id = format!("sess-oversize-{padding}");
    let frame = json!({
        "jsonrpc": "2.0",
        "method": "session.register",
        "params": {
            "session_id": oversized_session_id,
            "worktree": tmp.path().to_str().expect("utf-8 tmp path"),
        },
        "id": "oversized",
    });
    let line = format!("{frame}\n");
    assert!(
        line.len() > tight_cap_bytes,
        "test fixture must exceed the tight cap; got len={} cap={}",
        line.len(),
        tight_cap_bytes
    );

    let mut stream = UnixStream::connect(&socket).await.expect("connect socket");
    stream
        .write_all(line.as_bytes())
        .await
        .expect("write oversized frame");

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut response))
        .await
        .expect("response timeout")
        .expect("read response");
    let response: Value =
        serde_json::from_str(response.trim_end()).expect("oversized rejection json");

    let err = response.get("error").unwrap_or_else(|| {
        panic!(
            "oversized control frame must be refused with a JSON-RPC error once \
             `Resolved.ipc_limits.control_frame_max_bytes` is honoured. \
             The daemon returned a success/no-error payload, meaning \
             `run_foreground` is not applying the configured IPC limits — \
             the INTD-016 builder is inert in production. Response: {response:?}"
        )
    });
    assert_eq!(
        err.get("code").and_then(Value::as_i64),
        Some(-32600),
        "oversize rejection must use the INTD-016 Invalid-Request code; got {err:?}"
    );
    let reason = err
        .get("data")
        .and_then(|d| d.get("reason"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let expected_substring = format!("exceeds {tight_cap_bytes}-byte cap");
    assert!(
        reason.contains(&expected_substring),
        "rejection reason must echo the CONFIGURED cap ({tight_cap_bytes}), \
         not the daemon's compile-time default. reason={reason:?}"
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("daemon shutdown timed out")
        .expect("daemon task join failure")
        .expect("daemon run_foreground reported error");
}

/// MLP2-071 Phase 2 wire-up: `run_foreground` must wire the telemetry
/// broadcaster into the IPC listener so a `subscribe-telemetry` frame
/// over the real daemon socket is accepted — the daemon mints the
/// `SubscriberId` from `SO_PEERCRED` and registers it — rather than
/// rejected with "not available". Before `.with_broadcaster(...)` was
/// added to the `run_foreground` listener builder, the broadcaster was
/// `None` and this returned a dead-capability error (the #1671 class).
///
/// Linux-gated: subscriber minting reads `/proc/<peer_pid>/stat`, which
/// is the only platform where `pid_starttime` is implemented today.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "current_thread")]
async fn run_foreground_wires_telemetry_subscriber_surface() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (shutdown, handle, socket) = spawn_daemon_with_config(&tmp, Resolved::default()).await;

    let resp = send_jsonrpc(&socket, "sub-1", "subscribe-telemetry", json!({})).await;
    assert!(
        resp.get("error").is_none(),
        "subscribe-telemetry must be accepted by a run_foreground daemon \
         (the broadcaster is wired); got {resp:?}",
    );
    assert_eq!(
        resp.pointer("/result/subscribed"),
        Some(&Value::Bool(true)),
        "the daemon must confirm the subscription; got {resp:?}",
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("daemon shutdown timed out")
        .expect("daemon task join failure")
        .expect("daemon run_foreground reported error");
}
