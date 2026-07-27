//! Dual-era request dispatch against the MCP `2026-07-28` RC.

use serde_json::Value;

use super::domain::{self, DomainResult, SERVER_INSTRUCTIONS};
use super::meta::{looks_like_modern_request, parse_modern_meta};
use super::render::{
    CachePolicy, ProtocolEra, discover_body, error_response, error_response_with_data,
    render_success,
};
use super::trace::enter_request_span;
use super::versions::{
    ERR_INVALID_REQUEST, ERR_METHOD_NOT_FOUND, ERR_UNSUPPORTED_PROTOCOL_VERSION,
};

/// Handle one JSON-RPC message. Returns `None` for notifications that need no
/// response. Returns `Some` for requests (including JSON-RPC errors).
pub fn handle_message(message: &Value) -> Option<Value> {
    if !message.is_object() {
        return Some(error_response(
            &Value::Null,
            ERR_INVALID_REQUEST,
            "Invalid Request",
        ));
    }

    let id = message.get("id");
    let method = message.get("method").and_then(Value::as_str);
    let params = message.get("params");

    // Notifications (no id)
    if id.is_none() {
        return handle_notification(method);
    }

    let id = id.expect("checked");

    // Era selection: modern meta or server/discover → modern; else legacy.
    let modern_shape = method == Some("server/discover") || looks_like_modern_request(params);

    if modern_shape {
        return Some(handle_modern(id, method, message, params));
    }

    Some(handle_legacy(id, method, message))
}

fn handle_notification(method: Option<&str>) -> Option<Value> {
    // Notifications never produce a JSON-RPC response body. Process exit for
    // legacy `exit` is decided by [`is_exit_notification`] in the stdio host.
    match method {
        Some("notifications/initialized" | "exit") => None,
        // Ignore unknown notifications per JSON-RPC.
        Some(_) | None => None,
    }
}

fn handle_modern(id: &Value, method: Option<&str>, message: &Value, params: Option<&Value>) -> Value {
    // Validate modern meta on every modern request, including discover.
    let meta = match parse_modern_meta(params) {
        Ok(m) => m,
        Err(err) => {
            // Still enter a span so malformed meta is observable; version unknown.
            let _span = enter_request_span(method, ProtocolEra::Modern, None, params);
            return match err.data() {
                Some(data) => error_response_with_data(id, err.code(), err.message(), &data),
                None => error_response(id, err.code(), err.message()),
            };
        }
    };

    let _span = enter_request_span(
        method,
        ProtocolEra::Modern,
        Some(meta.protocol_version.as_str()),
        params,
    );

    match method {
        Some("server/discover") => {
            // Discovery must not wait on warm-up; warm is one-shot elsewhere.
            // Always stamp via render_success (idempotent for pre-filled fields).
            render_success(
                id,
                ProtocolEra::Modern,
                discover_body(SERVER_INSTRUCTIONS),
                CachePolicy::StablePrivate,
            )
        }
        // Modern lifecycle: do not honour ping/shutdown/exit as successes.
        Some("ping" | "shutdown" | "exit" | "initialize" | "notifications/initialized") => {
            error_response(id, ERR_METHOD_NOT_FOUND, "Method not found")
        }
        Some("tools/list") => finish(id, ProtocolEra::Modern, domain::tools_list(id)),
        Some("tools/call") => finish(id, ProtocolEra::Modern, domain::tools_call(id, message)),
        Some("resources/list") => finish(id, ProtocolEra::Modern, domain::resources_list(id)),
        Some("resources/read") => {
            finish(id, ProtocolEra::Modern, domain::resources_read(id, message))
        }
        Some(_) => error_response(id, ERR_METHOD_NOT_FOUND, "Method not found"),
        None => error_response(id, ERR_INVALID_REQUEST, "Invalid Request"),
    }
}

fn handle_legacy(id: &Value, method: Option<&str>, message: &Value) -> Value {
    let params = message.get("params");
    let protocol_version = params
        .and_then(|p| p.get("protocolVersion"))
        .and_then(Value::as_str);
    let _span = enter_request_span(method, ProtocolEra::Legacy, protocol_version, params);

    match method {
        Some("initialize") => finish(id, ProtocolEra::Legacy, domain::legacy_initialize(id, message)),
        Some("exit") => error_response(id, ERR_INVALID_REQUEST, "Invalid Request"),
        Some("shutdown") => finish(id, ProtocolEra::Legacy, domain::legacy_shutdown()),
        Some("ping") => finish(id, ProtocolEra::Legacy, domain::legacy_ping()),
        Some("tools/list") => finish(id, ProtocolEra::Legacy, domain::tools_list(id)),
        Some("tools/call") => finish(id, ProtocolEra::Legacy, domain::tools_call(id, message)),
        Some("resources/list") => finish(id, ProtocolEra::Legacy, domain::resources_list(id)),
        Some("resources/read") => {
            finish(id, ProtocolEra::Legacy, domain::resources_read(id, message))
        }
        // server/discover always takes the modern branch in handle_message.
        Some(_) => error_response(id, ERR_METHOD_NOT_FOUND, "Method not found"),
        None => error_response(id, ERR_INVALID_REQUEST, "Invalid Request"),
    }
}

fn finish(id: &Value, era: ProtocolEra, result: DomainResult) -> Value {
    match result {
        // Always era-render: stamp_modern is idempotent for discover-shaped bodies.
        DomainResult::Ok { body, cache } => render_success(id, era, body, cache),
        DomainResult::Rpc(value) => value,
    }
}

/// Whether this notification should terminate the stdio process.
///
/// Dual-era policy (Council / MCP26-003): only honour bare `exit` after a
/// successful sealed legacy `initialize` in this process. Modern clients stop
/// on EOF; modern `_meta` on exit never terminates; bare exit without prior
/// legacy init is ignored (no process kill).
pub fn is_exit_notification(message: &Value) -> bool {
    message.is_object()
        && message.get("method").and_then(Value::as_str) == Some("exit")
        && message.get("id").is_none()
        && !looks_like_modern_request(message.get("params"))
        && domain::legacy_process_initialized()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn modern_params() -> Value {
        json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        })
    }

    #[test]
    fn modern_discover_returns_supported_versions_and_cache() {
        let response = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "server/discover",
            "params": modern_params()
        }))
        .expect("response");
        let result = &response["result"];
        assert_eq!(result["resultType"], "complete");
        assert_eq!(result["supportedVersions"], json!(["2026-07-28"]));
        assert_eq!(result["capabilities"]["tools"], json!({}));
        assert_eq!(result["ttlMs"], 3_600_000);
        assert_eq!(result["cacheScope"], "private");
        assert_eq!(
            result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
            "anvil"
        );
    }

    #[test]
    fn modern_tools_list_without_initialize() {
        let response = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": modern_params()
        }))
        .expect("response");
        let result = &response["result"];
        assert_eq!(result["resultType"], "complete");
        assert!(result["tools"].as_array().is_some_and(|t| !t.is_empty()));
        assert_eq!(result["ttlMs"], 3_600_000);
    }

    #[test]
    fn modern_unsupported_version_is_32022() {
        let response = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2099-01-01",
                    "io.modelcontextprotocol/clientCapabilities": {}
                }
            }
        }))
        .expect("response");
        assert_eq!(response["error"]["code"], -32022);
        assert_eq!(response["error"]["data"]["requested"], "2099-01-01");
    }

    #[test]
    fn modern_exit_request_is_method_not_found() {
        let response = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "exit",
            "params": modern_params()
        }))
        .expect("response");
        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn modern_exit_notification_does_not_signal_process_exit() {
        let _guard = domain::lock_legacy_init_for_test();
        domain::reset_legacy_initialized_for_test();
        let message = json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": modern_params()
        });
        assert!(!is_exit_notification(&message));
    }

    #[test]
    fn bare_exit_without_legacy_init_does_not_terminate() {
        let _guard = domain::lock_legacy_init_for_test();
        domain::reset_legacy_initialized_for_test();
        let message = json!({
            "jsonrpc": "2.0",
            "method": "exit"
        });
        assert!(!is_exit_notification(&message));
    }

    #[test]
    fn legacy_exit_after_initialize_signals_process_exit() {
        let _guard = domain::lock_legacy_init_for_test();
        domain::reset_legacy_initialized_for_test();
        let _ = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }
        }));
        let message = json!({
            "jsonrpc": "2.0",
            "method": "exit"
        });
        assert!(is_exit_notification(&message));
        domain::reset_legacy_initialized_for_test();
    }

    #[test]
    fn legacy_initialize_still_works() {
        let _guard = domain::lock_legacy_init_for_test();
        domain::reset_legacy_initialized_for_test();
        let response = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }
        }))
        .expect("response");
        assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(response["result"]["serverInfo"]["name"], "anvil");
        assert!(response["result"].get("resultType").is_none());
        domain::reset_legacy_initialized_for_test();
    }

    #[test]
    fn legacy_initialize_rejects_unknown_protocol_version() {
        let _guard = domain::lock_legacy_init_for_test();
        domain::reset_legacy_initialized_for_test();
        let response = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2099-01-01",
                "capabilities": {}
            }
        }))
        .expect("response");
        assert_eq!(response["error"]["code"], -32602);
        assert_eq!(
            response["error"]["data"]["reason"],
            "unsupported-legacy-protocol-version"
        );
        assert!(!domain::legacy_process_initialized());
    }

    #[test]
    fn legacy_initialize_accepts_all_sealed_versions() {
        let _guard = domain::lock_legacy_init_for_test();
        domain::reset_legacy_initialized_for_test();
        for version in ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"] {
            domain::reset_legacy_initialized_for_test();
            let response = handle_message(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": version,
                    "capabilities": {}
                }
            }))
            .expect("response");
            assert_eq!(response["result"]["protocolVersion"], version);
            assert!(domain::legacy_process_initialized());
        }
        domain::reset_legacy_initialized_for_test();
    }
}
