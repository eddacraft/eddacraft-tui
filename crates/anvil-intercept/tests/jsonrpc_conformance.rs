#![cfg(unix)]

// No published JSON-RPC fixture set is present in this workspace, so
// INTD-014 pins the daemon boundary with local fixture-style cases.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use anvil_intercept::Shutdown;
use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
use anvil_intercept::registry::{RegistryError, SessionDispatcher};
use anvil_intercept_proto::{SessionId, SessionRecord};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Clone, Copy)]
struct RejectingDispatcher;

impl SessionDispatcher for RejectingDispatcher {
    fn register(&self, _id: &SessionId, _worktree: &Path) -> Result<(), RegistryError> {
        Err(RegistryError::UnknownSession(SessionId::new("internal")))
    }

    fn heartbeat(&self, _id: &SessionId) -> Result<(), RegistryError> {
        Err(RegistryError::UnknownSession(SessionId::new("internal")))
    }

    fn unregister(&self, _id: &SessionId) -> Result<bool, RegistryError> {
        Err(RegistryError::UnknownSession(SessionId::new("internal")))
    }

    fn list(&self) -> Vec<SessionRecord> {
        Vec::new()
    }
}

async fn with_dispatcher<D: SessionDispatcher + Send + Sync + 'static>(
    dispatcher: D,
) -> (
    Shutdown,
    tokio::task::JoinHandle<Result<(), anvil_intercept::ipc::IpcError>>,
    UnixStream,
    TempDir,
) {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700))
        .expect("secure tempdir permissions");
    let socket = tmp.path().join("intercept.sock");
    let listener = IpcListener::bind(&socket, dispatcher).expect("bind listener");
    let (shutdown, token) = Shutdown::new();
    let handle = tokio::spawn(async move { listener.serve(token).await });
    let client = UnixStream::connect(&socket).await.expect("connect client");
    (shutdown, handle, client, tmp)
}

async fn with_client() -> (
    Shutdown,
    tokio::task::JoinHandle<Result<(), anvil_intercept::ipc::IpcError>>,
    UnixStream,
    TempDir,
) {
    with_dispatcher(NoopDispatcher).await
}

async fn request(frame: Value) -> Value {
    request_with_dispatcher(frame, NoopDispatcher).await
}

async fn request_raw(frame: &str) -> Value {
    let (shutdown, handle, mut client, _tmp) = with_client().await;
    client
        .write_all(format!("{frame}\n").as_bytes())
        .await
        .expect("write request");
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(1), reader.read_line(&mut line))
        .await
        .expect("response timeout")
        .expect("read response");
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
    serde_json::from_str(line.trim_end()).expect("response json")
}

async fn request_with_dispatcher<D: SessionDispatcher + Send + Sync + 'static>(
    frame: Value,
    dispatcher: D,
) -> Value {
    let (shutdown, handle, mut client, _tmp) = with_dispatcher(dispatcher).await;
    client
        .write_all(format!("{frame}\n").as_bytes())
        .await
        .expect("write request");
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(1), reader.read_line(&mut line))
        .await
        .expect("response timeout")
        .expect("read response");
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
    serde_json::from_str(line.trim_end()).expect("response json")
}

#[tokio::test]
async fn request_returns_jsonrpc_result_with_same_id() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "session.list",
        "id": "req-1"
    }))
    .await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "req-1");
    assert_eq!(response["result"], json!([]));
    assert!(response.get("error").is_none());
}

#[tokio::test]
async fn parse_error_returns_reserved_code_with_null_id() {
    let response = request_raw("{not json").await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_null());
    assert_eq!(response["error"]["code"], -32700);
    assert_eq!(response["error"]["message"], "Parse error");
    assert!(response["error"].get("data").is_some());
}

#[tokio::test]
async fn notification_does_not_emit_response() {
    let (shutdown, handle, mut client, _tmp) = with_client().await;
    client
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"list-sessions\"}\n")
        .await
        .expect("write notification");

    let mut reader = BufReader::new(client);
    let mut line = String::new();
    let read = tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut line)).await;
    assert!(
        read.is_err(),
        "notification must not produce response: {line}"
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
}

#[tokio::test]
async fn valid_notification_errors_do_not_emit_response() {
    let (shutdown, handle, mut client, _tmp) = with_client().await;
    client
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"missing\"}\n")
        .await
        .expect("write notification");

    let mut reader = BufReader::new(client);
    let mut line = String::new();
    let read = tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut line)).await;
    assert!(
        read.is_err(),
        "notification error must not produce response: {line}"
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
}

#[tokio::test]
async fn null_id_is_a_request_not_a_notification() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "list-sessions",
        "id": null
    }))
    .await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("id").is_some());
    assert!(response["id"].is_null());
    assert_eq!(response["result"], json!([]));
}

#[tokio::test]
async fn batch_returns_responses_for_requests_only() {
    let response = request(json!([
        {"jsonrpc": "2.0", "method": "list-sessions", "id": "one"},
        {"jsonrpc": "2.0", "method": "list-sessions"},
        {"jsonrpc": "2.0", "method": "missing", "id": "two"}
    ]))
    .await;

    let responses = response.as_array().expect("batch response array");
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["id"], "one");
    assert_eq!(responses[0]["result"], json!([]));
    assert_eq!(responses[1]["id"], "two");
    assert_eq!(responses[1]["error"]["code"], -32601);
}

#[tokio::test]
async fn all_notification_batch_does_not_emit_response() {
    let (shutdown, handle, mut client, _tmp) = with_client().await;
    client
        .write_all(
            b"[{\"jsonrpc\":\"2.0\",\"method\":\"list-sessions\"},{\"jsonrpc\":\"2.0\",\"method\":\"missing\"}]\n",
        )
        .await
        .expect("write batch");

    let mut reader = BufReader::new(client);
    let mut line = String::new();
    let read = tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut line)).await;
    assert!(
        read.is_err(),
        "all-notification batch must not produce response: {line}"
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
}

#[tokio::test]
async fn invalid_no_id_object_is_not_treated_as_notification() {
    let response = request(json!({"jsonrpc": "2.0"})).await;

    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32600);
}

#[tokio::test]
async fn empty_batch_returns_invalid_request() {
    let response = request(json!([])).await;

    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32600);
}

#[tokio::test]
async fn error_object_contains_code_message_and_data() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "missing",
        "id": "bad-method"
    }))
    .await;

    let error = &response["error"];
    assert_eq!(error["code"], -32601);
    assert_eq!(error["message"], "Method not found");
    assert_eq!(error["data"], json!({"method": "missing"}));
    assert!(response.get("result").is_none());
}

#[tokio::test]
async fn reserved_error_codes_are_used_for_protocol_failures() {
    let invalid_request = request(json!({"jsonrpc": "2.0", "id": "no-method"})).await;
    assert_eq!(invalid_request["error"]["code"], -32600);

    let parse_error = request_raw("{").await;
    assert_eq!(parse_error["error"]["code"], -32700);

    let method_not_found = request(json!({
        "jsonrpc": "2.0",
        "method": "missing",
        "id": "missing"
    }))
    .await;
    assert_eq!(method_not_found["error"]["code"], -32601);

    let invalid_params = request(json!({
        "jsonrpc": "2.0",
        "method": "heartbeat",
        "params": {},
        "id": "bad-params"
    }))
    .await;
    assert_eq!(invalid_params["error"]["code"], -32602);

    let internal_error = request_with_dispatcher(
        json!({
            "jsonrpc": "2.0",
            "method": "heartbeat",
            "params": {"session_id": "missing"},
            "id": "internal"
        }),
        RejectingDispatcher,
    )
    .await;
    assert_eq!(internal_error["error"]["code"], -32603);
}
