#![cfg(unix)]

// No published JSON-RPC fixture set is present in this workspace, so
// INTD-014 pins the daemon boundary with local fixture-style cases.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Duration;

use anvil_intercept::Shutdown;
use anvil_intercept::enforcement::{CONTENT_SIZE_CAP_BYTES_USIZE, EnforcementPipeline};
use anvil_intercept::ipc::{IpcListener, LEGACY_MAX_LINE_BYTES, NoopDispatcher};
use anvil_intercept::midedit::{
    MAX_CONCURRENT_SCAN_BUFFERS, MAX_SCAN_BUFFER_PATH_BYTES, ScanBufferService,
};
use anvil_intercept::registry::{RegistryError, SessionDispatcher};
use anvil_intercept_proto::{SessionId, SessionRecord};
use anvil_intercept_rules::{InterceptRule, RuleDecision, RuleInput, RuleRegistry};
use anvil_kernel_types::{Diagnostic, Mode};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Clone, Copy)]
struct RejectingDispatcher;

impl SessionDispatcher for RejectingDispatcher {
    fn register(
        &self,
        _id: &SessionId,
        _worktree: &Path,
        _agent_tag: Option<&anvil_intercept_proto::session::AgentTag>,
        _lineage: Option<&anvil_intercept_proto::session::LineageAnchor>,
    ) -> Result<(), RegistryError> {
        Err(RegistryError::UnknownSession(SessionId::new("internal")))
    }

    fn heartbeat(&self, _id: &SessionId, _peer_pid: Option<u32>) -> Result<(), RegistryError> {
        Err(RegistryError::UnknownSession(SessionId::new("internal")))
    }

    fn unregister(&self, _id: &SessionId, _peer_pid: Option<u32>) -> Result<bool, RegistryError> {
        Err(RegistryError::UnknownSession(SessionId::new("internal")))
    }

    fn list(&self) -> Vec<SessionRecord> {
        Vec::new()
    }

    fn report_process(
        &self,
        _id: &SessionId,
        _child_pid: u32,
        _child_pid_starttime: u64,
        _peer_pid: u32,
    ) -> Result<(), RegistryError> {
        Err(RegistryError::UnknownSession(SessionId::new("internal")))
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
    with_dispatcher_and_scan_buffer(dispatcher, ScanBufferService::default()).await
}

async fn with_dispatcher_and_scan_buffer<D: SessionDispatcher + Send + Sync + 'static>(
    dispatcher: D,
    scan_buffer: ScanBufferService,
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
    let listener = IpcListener::bind_with_scan_buffer_service(&socket, dispatcher, scan_buffer)
        .expect("bind listener");
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
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
        .await
        .expect("response timeout")
        .expect("read response");
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
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
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
        .await
        .expect("response timeout")
        .expect("read response");
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
    serde_json::from_str(line.trim_end()).expect("response json")
}

async fn request_with_scan_buffer_service(frame: Value, scan_buffer: ScanBufferService) -> Value {
    let (shutdown, handle, mut client, _tmp) =
        with_dispatcher_and_scan_buffer(NoopDispatcher, scan_buffer).await;
    client
        .write_all(format!("{frame}\n").as_bytes())
        .await
        .expect("write request");
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
        .await
        .expect("response timeout")
        .expect("read response");
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
    serde_json::from_str(line.trim_end()).expect("response json")
}

async fn scan_buffer_request(mut client: UnixStream, id: &str) -> Value {
    let frame = json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/busy.ts",
            "text": "const value = 1;\n",
            "version": 1,
            "mode": "midEdit"
        },
        "id": id
    });
    client
        .write_all(format!("{frame}\n").as_bytes())
        .await
        .expect("write scan request");
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
        .await
        .expect("response timeout")
        .expect("read response");
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
async fn oversized_all_notification_batch_does_not_emit_response() {
    let (shutdown, handle, mut client, _tmp) = with_client().await;
    let batch = (0..=32)
        .map(|_| json!({"jsonrpc": "2.0", "method": "list-sessions"}))
        .collect::<Vec<_>>();
    client
        .write_all(format!("{}\n", Value::Array(batch)).as_bytes())
        .await
        .expect("write batch");

    let mut reader = BufReader::new(client);
    let mut line = String::new();
    let read = tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut line)).await;
    assert!(
        read.is_err(),
        "oversized all-notification batch must not produce response: {line}"
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
async fn oversized_batch_returns_invalid_request() {
    let batch = (0..=32)
        .map(|id| json!({"jsonrpc": "2.0", "method": "list-sessions", "id": id}))
        .collect::<Vec<_>>();
    let response = request(Value::Array(batch)).await;

    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32600);
    assert!(
        response["error"]["data"]["reason"]
            .as_str()
            .expect("reason")
            .contains("batch must not contain more than")
    );
}

#[tokio::test]
async fn oversized_non_scan_jsonrpc_frame_is_rejected_before_full_parse() {
    let padding = "a".repeat(LEGACY_MAX_LINE_BYTES);
    let response = request_raw(&format!(
        r#"{{"jsonrpc":"2.0","method":"list-sessions","params":{{"padding":"{padding}"}},"id":"large-list"}}"#
    ))
    .await;

    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32600);
    assert!(
        response["error"]["data"]["reason"]
            .as_str()
            .expect("reason")
            .contains("non-scan_buffer")
    );
}

#[tokio::test]
async fn oversized_scan_buffer_batch_is_rejected_before_full_parse() {
    let text = "a".repeat(LEGACY_MAX_LINE_BYTES);
    let response = request_raw(&format!(
        r#"[{{"jsonrpc":"2.0","method":"scan_buffer","params":{{"path":"src/a.ts","text":"{text}","version":1,"mode":"midEdit"}},"id":"batched-scan"}}]"#
    ))
    .await;

    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32600);
    assert!(
        response["error"]["data"]["reason"]
            .as_str()
            .expect("reason")
            .contains("single scan_buffer request")
    );
}

/// Per JSON-RPC 2.0, notifications never receive a response — including
/// for oversized frames that the daemon's fast path would otherwise
/// reject as Invalid Request. A `scan_buffer` frame above the legacy
/// cap with no `id` field MUST be dropped silently; the connection
/// stays open and a subsequent regular request still gets served.
#[tokio::test]
async fn oversized_scan_buffer_notification_is_dropped_silently() {
    let (shutdown, handle, mut client, _tmp) = with_client().await;
    let text = "a".repeat(LEGACY_MAX_LINE_BYTES);
    // No `id` field — this is a notification by JSON-RPC 2.0 definition.
    let frame = format!(
        r#"{{"jsonrpc":"2.0","method":"scan_buffer","params":{{"path":"src/a.ts","text":"{text}","version":1,"mode":"midEdit"}}}}"#
    );
    client
        .write_all(format!("{frame}\n").as_bytes())
        .await
        .expect("write notification");

    // Drain attempt for any response — must time out (no response).
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    let read = tokio::time::timeout(Duration::from_millis(150), reader.read_line(&mut line)).await;
    assert!(
        read.is_err(),
        "oversized scan_buffer notification must not produce a response: {line}"
    );

    // Connection should still be alive — send a regular id-bearing request
    // on the same recovered stream and verify the listener is still serving
    // by reading back a well-formed JSON-RPC result with the matching id.
    let mut client = reader.into_inner();
    client
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"session.list\",\"id\":\"liveness-1\"}\n")
        .await
        .expect("write follow-up request");

    let mut reader = BufReader::new(client);
    let mut response_line = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut response_line))
        .await
        .expect("follow-up response timeout")
        .expect("read follow-up response");
    let response: Value =
        serde_json::from_str(response_line.trim_end()).expect("follow-up response json");
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "liveness-1");
    assert_eq!(response["result"], json!([]));
    assert!(response.get("error").is_none());

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
}

#[tokio::test]
async fn oversized_scan_buffer_with_duplicate_method_is_rejected_before_parse() {
    let text = "a".repeat(LEGACY_MAX_LINE_BYTES);
    let response = request_raw(&format!(
        r#"{{"jsonrpc":"2.0","method":"scan_buffer","params":{{"path":"src/a.ts","text":"{text}","version":1,"mode":"midEdit"}},"method":"list-sessions","id":"dup-method"}}"#
    ))
    .await;

    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32600);
    assert!(
        response["error"]["data"]["reason"]
            .as_str()
            .expect("reason")
            .contains("duplicate method")
    );
}

#[tokio::test]
async fn oversized_scan_buffer_with_unrelated_payload_is_rejected_before_parse() {
    let padding = "a".repeat(LEGACY_MAX_LINE_BYTES);
    let response = request_raw(&format!(
        r#"{{"jsonrpc":"2.0","method":"scan_buffer","params":{{"path":"src/a.ts","text":"ok","version":1,"mode":"midEdit"}},"padding":"{padding}","id":"scan-padding"}}"#
    ))
    .await;

    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32600);
    let response_text = response.to_string();
    assert!(response_text.contains("unsupported top-level fields"));
    assert!(
        !response_text.contains(&"a".repeat(256)),
        "error response must not echo attacker-controlled padding"
    );
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

#[tokio::test]
async fn scan_buffer_returns_mid_edit_diagnostics_without_disk_read() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/auth/client.ts",
            "text": "import { sdk } from './client';\nconst config = { api_key: 'abcdEFGH1234567890' };\nsdk.connect(config);\n",
            "version": 7,
            "mode": "midEdit"
        },
        "id": "scan-secret"
    }))
    .await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "scan-secret");
    assert_eq!(response["result"]["version"], 7);
    assert_eq!(response["result"]["truncated"], false);
    let diagnostics = response["result"]["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["source"]["rule_id"], "secret-detection");
    assert_eq!(diagnostics[0]["mode"], "mid-edit");
    assert!(response.get("error").is_none());
}

#[tokio::test]
async fn scan_buffer_routes_namespaced_alias() {
    // DRVR-002 dual-routing: drivers that import the canonical
    // `anvil_intercept_proto::protocol::ANVIL_SCAN_BUFFER` constant
    // (`"anvil/scan_buffer"`) must hit the same handler as the bare
    // `scan_buffer` form RTAI-002 originally pinned. The proto
    // crate's doc-comment promises both names route together; this
    // fixture pins that promise on the wire.
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "anvil/scan_buffer",
        "params": {
            "path": "src/auth/client.ts",
            "text": "import { sdk } from './client';\nconst config = { api_key: 'abcdEFGH1234567890' };\nsdk.connect(config);\n",
            "version": 7,
            "mode": "midEdit"
        },
        "id": "scan-namespaced"
    }))
    .await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "scan-namespaced");
    assert_eq!(response["result"]["version"], 7);
    assert_eq!(response["result"]["truncated"], false);
    let diagnostics = response["result"]["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["source"]["rule_id"], "secret-detection");
    assert_eq!(diagnostics[0]["mode"], "mid-edit");
    assert!(response.get("error").is_none());
}

#[tokio::test]
async fn scan_buffer_in_batch_is_rejected_without_scanning() {
    let response = request(json!([{
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/auth/client.ts",
            "text": "const config = { api_key: 'abcdEFGH1234567890' };\n",
            "version": 1,
            "mode": "midEdit"
        },
        "id": "scan-batch"
    }]))
    .await;

    let responses = response.as_array().expect("batch response array");
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["id"], "scan-batch");
    assert_eq!(responses[0]["error"]["code"], -32600);
    assert!(
        responses[0]["error"]["data"]["reason"]
            .as_str()
            .expect("reason")
            .contains("not supported in JSON-RPC batches")
    );
}

#[tokio::test]
async fn scan_buffer_rejects_unknown_top_level_fields() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/auth/client.ts",
            "text": "const value = 1;\n",
            "version": 1,
            "mode": "midEdit"
        },
        "padding": "not allowed",
        "id": "scan-extra-top"
    }))
    .await;

    assert_eq!(response["id"], "scan-extra-top");
    assert_eq!(response["error"]["code"], -32600);
    assert!(
        response["error"]["data"]["reason"]
            .as_str()
            .expect("reason")
            .contains("only allow jsonrpc")
    );
}

#[tokio::test]
async fn scan_buffer_rejects_unknown_param_fields() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/auth/client.ts",
            "text": "const value = 1;\n",
            "version": 1,
            "mode": "midEdit",
            "padding": "not allowed"
        },
        "id": "scan-extra-param"
    }))
    .await;

    assert_eq!(response["id"], "scan-extra-param");
    assert_eq!(response["error"]["code"], -32602);
    assert!(
        response["error"]["data"]["reason"]
            .as_str()
            .expect("reason")
            .contains("params only allow")
    );
}

#[tokio::test]
async fn oversized_jsonrpc_id_is_rejected_without_echo() {
    let large_id = "a".repeat(257);
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "session.list",
        "id": large_id
    }))
    .await;

    assert!(response["id"].is_null());
    assert_eq!(response["error"]["code"], -32600);
    assert!(
        !response.to_string().contains(&"a".repeat(128)),
        "response must not echo oversized id"
    );
}

#[tokio::test]
async fn scan_buffer_rejects_over_cap_content_as_invalid_params() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/large.ts",
            "text": "a".repeat(CONTENT_SIZE_CAP_BYTES_USIZE + 1),
            "version": 1,
            "mode": "midEdit"
        },
        "id": "scan-large"
    }))
    .await;

    assert_eq!(response["error"]["code"], -32602);
    assert!(
        response["error"]["data"]["reason"]
            .as_str()
            .expect("reason")
            .contains("content exceeds")
    );
}

#[tokio::test]
async fn scan_buffer_rejects_over_cap_path_as_invalid_params() {
    let long_path = "a".repeat(MAX_SCAN_BUFFER_PATH_BYTES + 1);
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": long_path,
            "text": "const value = 1;\n",
            "version": 1,
            "mode": "midEdit"
        },
        "id": "scan-long-path"
    }))
    .await;

    assert_eq!(response["id"], "scan-long-path");
    assert_eq!(response["error"]["code"], -32602);
    let response_text = response.to_string();
    assert!(response_text.contains("path exceeds"));
    assert!(
        !response_text.contains(&"a".repeat(256)),
        "error response must not echo attacker-controlled path"
    );
}

#[tokio::test]
async fn scan_buffer_busy_returns_structured_server_error() {
    struct BlockingRule {
        started: Arc<AtomicUsize>,
        barrier: Arc<Barrier>,
    }

    impl InterceptRule for BlockingRule {
        fn rule_id(&self) -> &'static str {
            "blocking-rule"
        }

        fn needs_content(&self) -> bool {
            true
        }

        fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
            RuleDecision::Allow
        }

        fn diagnostics_with_limit(
            &self,
            _input: &RuleInput<'_>,
            _mode: &Mode,
            _limit: usize,
        ) -> Vec<Diagnostic> {
            self.started.fetch_add(1, Ordering::SeqCst);
            self.barrier.wait();
            Vec::new()
        }
    }

    let started = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(MAX_CONCURRENT_SCAN_BUFFERS + 1));
    let registry = RuleRegistry::with_rules(vec![Box::new(BlockingRule {
        started: Arc::clone(&started),
        barrier: Arc::clone(&barrier),
    })])
    .expect("unique rule");
    let scan_buffer = ScanBufferService::new(EnforcementPipeline::new(registry));
    let (shutdown, handle, first_client, tmp) =
        with_dispatcher_and_scan_buffer(NoopDispatcher, scan_buffer).await;
    let socket = tmp.path().join("intercept.sock");
    let second_client = UnixStream::connect(&socket).await.expect("second connect");

    let first = tokio::spawn(async move { scan_buffer_request(first_client, "scan-first").await });
    let second =
        tokio::spawn(async move { scan_buffer_request(second_client, "scan-second").await });

    for _ in 0..50 {
        if started.load(Ordering::SeqCst) == MAX_CONCURRENT_SCAN_BUFFERS {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(started.load(Ordering::SeqCst), MAX_CONCURRENT_SCAN_BUFFERS);

    let busy_client = UnixStream::connect(&socket).await.expect("busy connect");
    let busy = scan_buffer_request(busy_client, "scan-busy").await;
    assert_eq!(busy["id"], "scan-busy");
    assert_eq!(busy["error"]["code"], -32000);
    assert_eq!(busy["error"]["message"], "Server busy");

    barrier.wait();
    assert_eq!(
        first.await.expect("first join")["result"]["diagnostics"],
        json!([])
    );
    assert_eq!(
        second.await.expect("second join")["result"]["diagnostics"],
        json!([])
    );
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
}

#[tokio::test]
async fn scan_buffer_accepts_worst_case_escaped_content_under_cap() {
    let scan_buffer = ScanBufferService::new(EnforcementPipeline::new(RuleRegistry::new()));
    let response = request_with_scan_buffer_service(
        json!({
            "jsonrpc": "2.0",
            "method": "scan_buffer",
            "params": {
                "path": "src/escaped.ts",
                "text": "\u{0001}".repeat(CONTENT_SIZE_CAP_BYTES_USIZE),
                "version": 3,
                "mode": "midEdit"
            },
            "id": "scan-escaped"
        }),
        scan_buffer,
    )
    .await;

    assert_eq!(response["id"], "scan-escaped");
    assert!(
        response.get("error").is_none(),
        "unexpected response: {response}"
    );
    assert_eq!(response["result"]["diagnostics"], json!([]));
}

#[tokio::test]
async fn scan_buffer_uses_listener_configured_rule_set() {
    let scan_buffer = ScanBufferService::new(EnforcementPipeline::new(RuleRegistry::new()));
    let response = request_with_scan_buffer_service(
        json!({
            "jsonrpc": "2.0",
            "method": "scan_buffer",
            "params": {
                "path": "src/auth/client.ts",
                "text": "const config = { api_key: 'abcdEFGH1234567890' };\n",
                "version": 8,
                "mode": "midEdit"
            },
            "id": "scan-empty-registry"
        }),
        scan_buffer,
    )
    .await;

    assert_eq!(response["id"], "scan-empty-registry");
    assert_eq!(response["result"]["diagnostics"], json!([]));
    assert_eq!(response["result"]["truncated"], false);
}

#[tokio::test]
async fn scan_buffer_rejects_binary_content_with_nul() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "assets/generated.bin",
            "text": "api_key='abcdEFGH1234567890'\u{0000}",
            "version": 2,
            "mode": "midEdit"
        },
        "id": "scan-binary"
    }))
    .await;

    // NUL content is invalid params, not a clean success with empty diagnostics.
    assert_eq!(response["id"], "scan-binary");
    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["message"], "Invalid params");
    let reason = response["error"]["data"]["reason"]
        .as_str()
        .expect("binary-content reason string");
    assert!(
        reason.contains("binary content") || reason.contains("NUL"),
        "reason must identify binary/NUL rejection, got: {reason}"
    );
    assert!(
        response.get("result").is_none(),
        "NUL content must not silently pass with a result: {response}"
    );
}

/// TRACE-001 / ADR-035: a valid W3C `traceparent` placed on the
/// JSON-RPC envelope must round-trip onto the matching response
/// unchanged, so a downstream consumer can correlate a notification or
/// error back to its originating span without re-deriving the trace
/// context. The producer is the source-of-truth for the header bytes;
/// the daemon's job is to validate (reject malformed values) and echo.
#[tokio::test]
async fn traceparent_round_trips_through_jsonrpc_envelope_unchanged() {
    let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "session.list",
        "id": "trace-1",
        "traceparent": traceparent,
    }))
    .await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "trace-1");
    assert_eq!(response["result"], json!([]));
    assert_eq!(
        response["traceparent"], traceparent,
        "traceparent must round-trip byte-for-byte through the envelope"
    );
}

/// JSON-RPC notifications never receive a response. A notification
/// carrying a malformed `traceparent` is still a notification, and the
/// daemon must silently drop it rather than emit a rejection — emitting
/// would violate the JSON-RPC notification contract.
#[tokio::test]
async fn invalid_traceparent_on_notification_is_dropped_silently() {
    let (shutdown, handle, mut client, _tmp) = with_client().await;
    client
        .write_all(
            b"{\"jsonrpc\":\"2.0\",\"method\":\"list-sessions\",\"traceparent\":\"00-not-a-real-traceparent\"}\n",
        )
        .await
        .expect("write notification");

    let mut reader = BufReader::new(client);
    let mut line = String::new();
    let read = tokio::time::timeout(Duration::from_millis(150), reader.read_line(&mut line)).await;
    assert!(
        read.is_err(),
        "notification with invalid traceparent must not produce a response: {line}"
    );

    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
}

#[tokio::test]
async fn invalid_traceparent_is_rejected_as_invalid_request() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "session.list",
        "id": "trace-bad",
        "traceparent": "00-not-a-real-trace-context-value",
    }))
    .await;

    assert_eq!(response["id"], "trace-bad");
    assert_eq!(response["error"]["code"], -32600);
    assert!(
        response["error"]["data"]["reason"]
            .as_str()
            .expect("reason")
            .contains("traceparent is invalid"),
        "expected traceparent rejection, got {response}"
    );
    // Round-trip is only contracted for valid headers; reject responses
    // must not echo an invalid value back.
    assert!(response.get("traceparent").is_none());
}

#[tokio::test]
async fn scan_buffer_malformed_request_returns_structured_error() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/auth/client.ts",
            "version": 1,
            "mode": "midEdit"
        },
        "id": "scan-malformed"
    }))
    .await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "scan-malformed");
    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["message"], "Invalid params");
    assert!(response["error"].get("data").is_some());
}

// ----- INTD-011 query_status fixtures ---------------------------------

#[tokio::test]
async fn query_status_returns_no_traffic_when_aggregator_empty() {
    // Default IPC listener gets a NoopStatusProvider — empty
    // snapshot. The wire shape MUST carry latency.mid_edit = null
    // (NOT zero) so consumers can distinguish "no traffic yet" from
    // "0ms".
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "query_status",
        "id": "status-empty"
    }))
    .await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "status-empty");
    assert!(response.get("error").is_none(), "error: {response}");
    let result = &response["result"];
    assert_eq!(result["sessions"], json!([]));
    assert_eq!(result["worktrees"], json!([]));
    assert_eq!(result["fences"], json!([]));
    assert!(result["health"]["uptime_seconds"].is_u64());
    assert!(result["health"]["version"].is_string());
    assert_eq!(result["health"]["ipc_state"], "serving");
    assert!(
        result["latency"]["mid_edit"].is_null(),
        "no traffic must wire as null, got {result}",
    );
}

#[tokio::test]
async fn query_status_with_traffic_carries_p50_and_p95() {
    use anvil_intercept::status::{DaemonStatus, IpcState, build_status};
    use anvil_intercept_proto::status::DaemonStatusV1;

    // Stub provider that pretends a few mid-edit calls were observed —
    // we test the wire shape carries p50/p95 milliseconds without
    // wiring a real ScanBufferService through.
    struct WithRollup;
    impl anvil_intercept::status::StatusProvider for WithRollup {
        fn query_status(&self) -> DaemonStatus {
            // Build a synthetic rollup. The aggregator's own tests
            // pin the percentile maths; this fixture just needs the
            // wire shape to carry the numbers through.
            let started = std::time::Instant::now();
            let now = started;
            let rollup = anvil_intercept::latency::LatencyRollup {
                p50_ms: 12.5,
                p95_ms: 47.3,
                sample_count: 17,
                window_seconds: 22.4,
            };
            build_status(
                vec![],
                &[],
                &[],
                Some(rollup),
                started,
                now,
                "test-version",
                IpcState::Serving,
                None,
                None,
                0,
            )
        }
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700))
        .expect("secure tempdir permissions");
    let socket = tmp.path().join("intercept.sock");
    let listener = anvil_intercept::ipc::IpcListener::bind_with_scan_buffer_service(
        &socket,
        anvil_intercept::ipc::NoopDispatcher,
        anvil_intercept::midedit::ScanBufferService::default(),
    )
    .expect("bind listener")
    .with_status_provider(std::sync::Arc::new(WithRollup));
    let (shutdown, token) = anvil_intercept::Shutdown::new();
    let handle = tokio::spawn(async move { listener.serve(token).await });

    let mut client = tokio::net::UnixStream::connect(&socket)
        .await
        .expect("connect");
    client
        .write_all(
            br#"{"jsonrpc":"2.0","method":"query_status","id":"status-traffic"}
"#,
        )
        .await
        .expect("write request");
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
        .await
        .expect("response timeout")
        .expect("read response");
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");

    let response: Value = serde_json::from_str(line.trim_end()).expect("response json");
    assert_eq!(response["id"], "status-traffic");
    let mid_edit = &response["result"]["latency"]["mid_edit"];
    assert!(
        mid_edit.is_object(),
        "mid_edit must be an object: {response}"
    );
    assert!((mid_edit["p50_ms"].as_f64().unwrap() - 12.5).abs() < 1e-9);
    assert!((mid_edit["p95_ms"].as_f64().unwrap() - 47.3).abs() < 1e-9);
    assert_eq!(mid_edit["sample_count"], 17);

    // Sanity: the wire shape parses back into the proto type. Driver
    // consumers parse this directly per ADR-031 vocabulary.
    let parsed: DaemonStatusV1 =
        serde_json::from_value(response["result"].clone()).expect("parse via proto");
    let mid = parsed.latency.mid_edit.expect("mid_edit Some");
    assert!((mid.p50_ms - 12.5).abs() < 1e-9);
    assert!((mid.p95_ms - 47.3).abs() < 1e-9);
}

#[tokio::test]
async fn query_status_rejects_params() {
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "query_status",
        "params": {"unexpected": true},
        "id": "status-bad-params"
    }))
    .await;

    assert_eq!(response["id"], "status-bad-params");
    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["message"], "Invalid params");
}

#[tokio::test]
async fn query_status_routes_namespaced_alias() {
    // DRVR-002 + INTD-011 dual-routing: drivers that import the
    // canonical `anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY`
    // constant must hit a live route. The bare `query_status` form is
    // preserved for legacy CLI consumers; both names share the same
    // handler and produce the same response shape.
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "anvil/status/query",
        "id": "status-namespaced"
    }))
    .await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "status-namespaced");
    assert!(response.get("error").is_none(), "error: {response}");
    let result = &response["result"];
    assert_eq!(result["sessions"], json!([]));
    assert_eq!(result["worktrees"], json!([]));
    assert_eq!(result["fences"], json!([]));
    assert!(result["health"]["uptime_seconds"].is_u64());
    assert_eq!(result["health"]["ipc_state"], "serving");
    assert!(
        result["latency"]["mid_edit"].is_null(),
        "no traffic must wire as null, got {result}",
    );
}

#[tokio::test]
async fn query_status_namespaced_alias_rejects_params() {
    // Symmetric with `query_status_rejects_params` — the canonical
    // form shares the same parameter contract (none allowed).
    let response = request(json!({
        "jsonrpc": "2.0",
        "method": "anvil/status/query",
        "params": {"unexpected": true},
        "id": "status-namespaced-bad-params"
    }))
    .await;

    assert_eq!(response["id"], "status-namespaced-bad-params");
    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["message"], "Invalid params");
}

#[tokio::test]
async fn query_status_notification_is_silently_dropped() {
    // JSON-RPC 2.0: notifications never get a response. Status query
    // is request-shaped — a notification form is treated as a no-op.
    let (shutdown, handle, mut client, _tmp) = with_client().await;
    client
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"query_status\"}\n")
        .await
        .expect("write notification");
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    let read = tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut line)).await;
    assert!(
        read.is_err(),
        "query_status notification must not produce response: {line}",
    );
    shutdown.trigger();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("listener timeout")
        .expect("listener join")
        .expect("listener ok");
}
