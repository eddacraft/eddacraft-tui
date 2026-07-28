//! Era-specific MCP result envelope rendering (RC baseline).

use serde_json::{Value, json};

use super::meta::META_SERVER_INFO;
use super::versions::MODERN_PROTOCOL_VERSION;

/// Cache policy approved for RC dual-era support.
pub const TTL_STABLE_MS: u64 = 3_600_000;
pub const TTL_STALE_MS: u64 = 0;
pub const CACHE_SCOPE_PRIVATE: &str = "private";

#[derive(Debug, Clone, Copy)]
pub enum CachePolicy {
    StablePrivate,
    ImmediatePrivate,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolEra {
    Modern,
    Legacy,
}

pub fn server_info() -> Value {
    json!({
        "name": "anvil",
        "version": env!("CARGO_PKG_VERSION")
    })
}

pub fn success_response(id: &Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

pub fn error_response(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

pub fn error_response_with_data(id: &Value, code: i64, message: &str, data: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
            "data": data
        }
    })
}

pub fn parse_error_response() -> Value {
    error_response(&Value::Null, super::versions::ERR_PARSE, "Parse error")
}

/// Render an era-neutral domain success body into a JSON-RPC response.
pub fn render_success(id: &Value, era: ProtocolEra, mut body: Value, cache: CachePolicy) -> Value {
    match era {
        ProtocolEra::Legacy => success_response(id, body),
        ProtocolEra::Modern => {
            stamp_modern(&mut body, cache);
            success_response(id, body)
        }
    }
}

fn stamp_modern(body: &mut Value, cache: CachePolicy) {
    let Value::Object(map) = body else {
        // Domain handlers always return objects for success bodies.
        *body = json!({
            "resultType": "complete",
            "value": body.clone(),
            "_meta": {
                "io.modelcontextprotocol/serverInfo": server_info()
            }
        });
        apply_cache(body, cache);
        return;
    };

    map.insert("resultType".into(), json!("complete"));

    let meta = map.entry("_meta".to_string()).or_insert_with(|| json!({}));
    if let Value::Object(meta_map) = meta {
        meta_map
            .entry(META_SERVER_INFO.to_string())
            .or_insert_with(server_info);
    }

    apply_cache_map(map, cache);
}

fn apply_cache(body: &mut Value, cache: CachePolicy) {
    if let Value::Object(map) = body {
        apply_cache_map(map, cache);
    }
}

fn apply_cache_map(map: &mut serde_json::Map<String, Value>, cache: CachePolicy) {
    match cache {
        CachePolicy::StablePrivate => {
            map.insert("ttlMs".into(), json!(TTL_STABLE_MS));
            map.insert("cacheScope".into(), json!(CACHE_SCOPE_PRIVATE));
        }
        CachePolicy::ImmediatePrivate => {
            map.insert("ttlMs".into(), json!(TTL_STALE_MS));
            map.insert("cacheScope".into(), json!(CACHE_SCOPE_PRIVATE));
        }
        CachePolicy::None => {}
    }
}

/// Build the modern `server/discover` result body (RC shape).
pub fn discover_body(instructions: &str) -> Value {
    json!({
        "resultType": "complete",
        "supportedVersions": [MODERN_PROTOCOL_VERSION],
        "capabilities": {
            "tools": {},
            "resources": {}
        },
        "instructions": instructions,
        "ttlMs": TTL_STABLE_MS,
        "cacheScope": CACHE_SCOPE_PRIVATE,
        "_meta": {
            "io.modelcontextprotocol/serverInfo": server_info()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_success_stamps_result_type_and_server_info() {
        let body = json!({ "tools": [] });
        let response = render_success(
            &json!(1),
            ProtocolEra::Modern,
            body,
            CachePolicy::StablePrivate,
        );
        let result = &response["result"];
        assert_eq!(result["resultType"], "complete");
        assert_eq!(result["ttlMs"], TTL_STABLE_MS);
        assert_eq!(result["cacheScope"], "private");
        assert_eq!(result["_meta"][META_SERVER_INFO]["name"], "anvil");
    }

    #[test]
    fn legacy_success_does_not_add_modern_fields() {
        let body = json!({ "tools": [] });
        let response = render_success(
            &json!(1),
            ProtocolEra::Legacy,
            body,
            CachePolicy::StablePrivate,
        );
        let result = &response["result"];
        assert!(result.get("resultType").is_none());
        assert!(result.get("ttlMs").is_none());
        assert!(result.get("_meta").is_none());
    }
}
