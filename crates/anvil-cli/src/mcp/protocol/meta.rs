//! Modern per-request `_meta` parsing (MCP `2026-07-28` RC).

use serde_json::Value;

use super::versions::{ERR_INVALID_PARAMS, ERR_UNSUPPORTED_PROTOCOL_VERSION, is_modern_version};

/// Keys under `params._meta` for the modern era (RC).
pub const META_PROTOCOL_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
pub const META_CLIENT_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
pub const META_CLIENT_INFO: &str = "io.modelcontextprotocol/clientInfo";
pub const META_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";

/// Bounds for metadata nesting inside the existing frame limit.
const MAX_META_DEPTH: usize = 8;

#[derive(Debug, Clone)]
pub struct ModernRequestMeta {
    /// Negotiated modern protocol version (MCP26-009 records this on spans).
    pub protocol_version: String,
    /// Present client info object is accepted but never trusted for auth.
    #[allow(dead_code)]
    pub client_info: Option<Value>,
    #[allow(dead_code)]
    pub client_capabilities: Value,
}

#[derive(Debug)]
pub enum MetaError {
    MissingMeta,
    MissingProtocolVersion,
    InvalidProtocolVersionType,
    MissingClientCapabilities,
    InvalidClientCapabilitiesType,
    UnsupportedProtocolVersion { requested: String },
    MetadataTooDeep,
    InvalidClientInfoType,
}

impl MetaError {
    pub fn code(&self) -> i64 {
        match self {
            Self::UnsupportedProtocolVersion { .. } => ERR_UNSUPPORTED_PROTOCOL_VERSION,
            _ => ERR_INVALID_PARAMS,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::UnsupportedProtocolVersion { .. } => "Unsupported protocol version",
            Self::MissingMeta
            | Self::MissingProtocolVersion
            | Self::InvalidProtocolVersionType
            | Self::MissingClientCapabilities
            | Self::InvalidClientCapabilitiesType
            | Self::MetadataTooDeep
            | Self::InvalidClientInfoType => "Invalid params",
        }
    }

    pub fn data(&self) -> Option<Value> {
        match self {
            Self::UnsupportedProtocolVersion { requested } => Some(serde_json::json!({
                "supported": [super::versions::MODERN_PROTOCOL_VERSION],
                "requested": requested,
            })),
            _ => None,
        }
    }
}

/// True when `params` carries a `_meta` field (modern intent).
///
/// Presence alone selects the modern era so malformed metadata is rejected by
/// [`parse_modern_meta`] rather than falling through to the legacy dispatcher.
pub fn looks_like_modern_request(params: Option<&Value>) -> bool {
    params.is_some_and(|p| p.get("_meta").is_some())
}

pub fn parse_modern_meta(params: Option<&Value>) -> Result<ModernRequestMeta, MetaError> {
    let Some(params) = params else {
        return Err(MetaError::MissingMeta);
    };
    let Some(meta) = params.get("_meta") else {
        return Err(MetaError::MissingMeta);
    };
    if depth(meta, 0) > MAX_META_DEPTH {
        return Err(MetaError::MetadataTooDeep);
    }
    let Some(meta_obj) = meta.as_object() else {
        return Err(MetaError::MissingMeta);
    };

    let protocol_version = match meta_obj.get(META_PROTOCOL_VERSION) {
        None => return Err(MetaError::MissingProtocolVersion),
        Some(Value::String(s)) => s.clone(),
        Some(_) => return Err(MetaError::InvalidProtocolVersionType),
    };

    if !is_modern_version(&protocol_version) {
        return Err(MetaError::UnsupportedProtocolVersion {
            requested: protocol_version,
        });
    }

    let client_capabilities = match meta_obj.get(META_CLIENT_CAPABILITIES) {
        None => return Err(MetaError::MissingClientCapabilities),
        Some(Value::Object(_)) => meta_obj
            .get(META_CLIENT_CAPABILITIES)
            .cloned()
            .expect("checked"),
        Some(_) => return Err(MetaError::InvalidClientCapabilitiesType),
    };

    let client_info = match meta_obj.get(META_CLIENT_INFO) {
        None => None,
        Some(Value::Object(_)) => meta_obj.get(META_CLIENT_INFO).cloned(),
        Some(_) => return Err(MetaError::InvalidClientInfoType),
    };

    Ok(ModernRequestMeta {
        protocol_version,
        client_info,
        client_capabilities,
    })
}

fn depth(value: &Value, current: usize) -> usize {
    if current > MAX_META_DEPTH {
        return current;
    }
    match value {
        Value::Object(map) => map
            .values()
            .map(|v| depth(v, current + 1))
            .max()
            .unwrap_or(current),
        Value::Array(items) => items
            .iter()
            .map(|v| depth(v, current + 1))
            .max()
            .unwrap_or(current),
        _ => current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_requires_protocol_version_and_capabilities() {
        let err = parse_modern_meta(Some(&json!({}))).unwrap_err();
        assert!(matches!(err, MetaError::MissingMeta));

        let err = parse_modern_meta(Some(&json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28"
            }
        })))
        .unwrap_err();
        assert!(matches!(err, MetaError::MissingClientCapabilities));
    }

    #[test]
    fn unsupported_version_returns_supported_list() {
        let err = parse_modern_meta(Some(&json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2099-01-01",
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        })))
        .unwrap_err();
        assert_eq!(err.code(), ERR_UNSUPPORTED_PROTOCOL_VERSION);
        let data = err.data().expect("data");
        assert_eq!(data["requested"], "2099-01-01");
        assert_eq!(data["supported"], json!(["2026-07-28"]));
    }

    #[test]
    fn accepts_absent_client_info() {
        let meta = parse_modern_meta(Some(&json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": { "roots": {} }
            }
        })))
        .expect("valid modern meta");
        assert_eq!(meta.protocol_version, "2026-07-28");
        assert!(meta.client_info.is_none());
    }

    #[test]
    fn presence_of_meta_is_modern_intent() {
        assert!(looks_like_modern_request(Some(&json!({
            "_meta": {}
        }))));
        assert!(looks_like_modern_request(Some(&json!({
            "_meta": "not-an-object"
        }))));
        assert!(!looks_like_modern_request(Some(&json!({
            "protocolVersion": "2024-11-05"
        }))));
        assert!(!looks_like_modern_request(None));
    }
}
