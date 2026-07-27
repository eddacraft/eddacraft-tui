//! Dual-era request dispatch against the MCP `2026-07-28` RC.

use serde_json::Value;

use super::domain::{self, DomainResult, SERVER_INSTRUCTIONS};
use super::meta::{looks_like_modern_request, parse_modern_meta};
use super::render::{
    CachePolicy, ProtocolEra, discover_body, error_response, error_response_with_data,
    render_success,
};
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
    match method {
        Some("notifications/initialized") => None,
        // Legacy exit notification: process exit is handled by the stdio host.
        Some("exit") => None,
        // Modern clients must not use legacy lifecycle notifications; ignore
        // unknown notifications per JSON-RPC.
        Some(_) | None => None,
    }
}

fn handle_modern(id: &Value, method: Option<&str>, message: &Value, params: Option<&Value>) -> Value {
    // Validate modern meta on every modern request, including discover.
    let meta = match parse_modern_meta(params) {
        Ok(m) => m,
        Err(err) => {
            return match err.data() {
                Some(data) => error_response_with_data(id, err.code(), err.message(), &data),
                None => error_response(id, err.code(), err.message()),
            };
        }
    };
    let _ = meta; // retained for future span/trace correlation (MCP26-009)

    match method {
        Some("server/discover") => {
            // Discovery must not wait on warm-up; warm is one-shot elsewhere.
            render_success(
                id,
                ProtocolEra::Modern,
                discover_body(SERVER_INSTRUCTIONS),
                CachePolicy::None, // already stamped in discover_body
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
        // Modern-only method on the legacy path without modern meta.
        Some("server/discover") => error_response_with_data(
            id,
            ERR_UNSUPPORTED_PROTOCOL_VERSION,
            "Unsupported protocol version",
            &serde_json::json!({
                "supported": [super::versions::MODERN_PROTOCOL_VERSION],
                "requested": null,
                "hint": "server/discover requires modern params._meta"
            }),
        ),
        Some(_) => error_response(id, ERR_METHOD_NOT_FOUND, "Method not found"),
        None => error_response(id, ERR_INVALID_REQUEST, "Invalid Request"),
    }
}

fn finish(id: &Value, era: ProtocolEra, result: DomainResult) -> Value {
    match result {
        DomainResult::Ok { body, cache } => {
            // discover_body already includes modern stamps; skip double-stamp when
            // cache is None and body already has resultType (modern discover path
            // uses render_success with CachePolicy::None after pre-stamped body).
            if era == ProtocolEra::Modern
                && body.get("resultType").and_then(Value::as_str) == Some("complete")
                && matches!(cache, CachePolicy::None)
            {
                return super::render::success_response(id, body);
            }
            render_success(id, era, body, cache)
        }
        DomainResult::Rpc(value) => value,
    }
}

/// Whether this notification should terminate the stdio process (legacy exit).
pub fn is_exit_notification(message: &Value) -> bool {
    message.is_object()
        && message.get("method").and_then(Value::as_str) == Some("exit")
        && message.get("id").is_none()
        // Modern clients may send exit as a notification; dual-era policy:
        // only terminate on legacy-path exit (no modern meta). Modern exit
        // notifications are ignored for process lifetime (MCP26-003).
        && !looks_like_modern_request(message.get("params"))
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
        let message = json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": modern_params()
        });
        assert!(!is_exit_notification(&message));
    }

    #[test]
    fn legacy_exit_notification_signals_process_exit() {
        let message = json!({
            "jsonrpc": "2.0",
            "method": "exit"
        });
        assert!(is_exit_notification(&message));
    }

    #[test]
    fn legacy_initialize_still_works() {
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
    }
}
