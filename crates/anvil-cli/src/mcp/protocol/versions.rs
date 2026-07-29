//! Supported protocol versions for the dual-era host.

/// Modern protocol version targeted by MCP26.
pub const MODERN_PROTOCOL_VERSION: &str = "2026-07-28";

/// Default version when a legacy `initialize` omits `protocolVersion`.
pub const DEFAULT_LEGACY_PROTOCOL_VERSION: &str = "2024-11-05";

/// Sealed legacy initialise-era set (operator default: keep all).
pub const LEGACY_PROTOCOL_VERSIONS: &[&str] =
    &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// JSON-RPC / MCP error: unsupported modern protocol version.
pub const ERR_UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;

/// JSON-RPC invalid params.
pub const ERR_INVALID_PARAMS: i64 = -32602;

/// JSON-RPC method not found.
pub const ERR_METHOD_NOT_FOUND: i64 = -32601;

/// JSON-RPC invalid request.
pub const ERR_INVALID_REQUEST: i64 = -32600;

/// JSON-RPC internal error.
pub const ERR_INTERNAL: i64 = -32603;

/// JSON-RPC parse error.
pub const ERR_PARSE: i64 = -32700;

pub fn is_legacy_version(version: &str) -> bool {
    LEGACY_PROTOCOL_VERSIONS.contains(&version)
}

pub fn is_modern_version(version: &str) -> bool {
    version == MODERN_PROTOCOL_VERSION
}
