//! Era-neutral MCP domain handlers (MCP26-002).
//!
//! Handlers return domain payloads and structured errors without knowing the
//! negotiated protocol version. The protocol adapter owns envelopes.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::auth::credentials;
use crate::feature_flags;
use crate::mcp::tools::{registry, validate_write};

use super::render::{CachePolicy, error_response, error_response_with_data, server_info};
use super::versions::{
    DEFAULT_LEGACY_PROTOCOL_VERSION, ERR_INTERNAL, ERR_INVALID_PARAMS,
    is_legacy_version,
};

/// Shared server instructions text (initialize + discover).
pub const SERVER_INSTRUCTIONS: &str = "This server provides two write-validation tools: anvil_validate_write and anvil_apply_patch. Before applying any file write - Write, Edit, MultiEdit, fs.write, apply_edit, or equivalent - call anvil_validate_write with the proposed content (or a preview of the first lines) and respect the response decision. When applying a unified diff to an existing file, prefer anvil_apply_patch instead; it accepts a unifiedDiff and scans only the added lines, producing a smaller, more readable approval prompt. Decision vocabulary: `block` is authoritative — do not write, do not bypass via alternate tools (the response carries either a `diagnostics` array of findings or an `error` describing why the gate refused). `warn` means findings were detected but the workspace enforcement mode lets the write proceed — surface the diagnostics and continue. `gateUnavailable` is informational — the gate could not run (e.g. credentials missing or backend offline); surface the warning to the user and proceed with the write. `allow` means the proposed content passed validation.";

/// Domain-level outcome before protocol envelope rendering.
#[derive(Debug)]
pub enum DomainResult {
    /// Success body (era-neutral object).
    Ok {
        body: Value,
        cache: CachePolicy,
    },
    /// JSON-RPC error already fully shaped (rare paths that bypass render).
    Rpc(Value),
}

impl DomainResult {
    pub fn ok_body(body: Value, cache: CachePolicy) -> Self {
        Self::Ok { body, cache }
    }

    pub fn invalid_params(id: &Value) -> Self {
        Self::Rpc(error_response(id, ERR_INVALID_PARAMS, "Invalid params"))
    }

    pub fn invalid_params_data(id: &Value, data: Value) -> Self {
        Self::Rpc(error_response_with_data(
            id,
            ERR_INVALID_PARAMS,
            "Invalid params",
            &data,
        ))
    }

    pub fn internal_data(id: &Value, data: Value) -> Self {
        Self::Rpc(error_response_with_data(
            id,
            ERR_INTERNAL,
            "Internal error",
            &data,
        ))
    }
}

/// Best-effort graph warm-up. Safe to call from modern or legacy paths; does
/// not block discovery (fire-and-forget daemon warm).
pub fn warm_up_workspace() {
    if let Ok(cwd) = std::env::current_dir() {
        let _ = crate::commands::watch_save_time::warm_up_root(&cwd);
    }
}

/// One-time process warm-up for modern clients that never call `initialize`.
pub fn ensure_warmed_once() {
    static WARMED: OnceLock<()> = OnceLock::new();
    WARMED.get_or_init(|| {
        warm_up_workspace();
    });
}

pub fn legacy_initialize(id: &Value, message: &Value) -> DomainResult {
    warm_up_workspace();

    let Some(params) = message.get("params").and_then(Value::as_object) else {
        return DomainResult::invalid_params(id);
    };

    let protocol_version = match params.get("protocolVersion") {
        Some(Value::String(version)) => {
            // Prefer negotiated legacy versions when known; still echo unknown
            // only if it is a string (pre-MCP26 behaviour was echo-any). Dual-era
            // seals the modern path; legacy keeps echo for compatibility of
            // in-flight clients while fixtures pin known versions.
            version.as_str()
        }
        Some(_) => return DomainResult::invalid_params(id),
        None => DEFAULT_LEGACY_PROTOCOL_VERSION,
    };

    // Record that this process completed a legacy handshake (informational).
    let _ = is_legacy_version(protocol_version);

    DomainResult::ok_body(
        json!({
            "protocolVersion": protocol_version,
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "instructions": SERVER_INSTRUCTIONS,
            "serverInfo": server_info()
        }),
        CachePolicy::None,
    )
}

pub fn tools_list(_id: &Value) -> DomainResult {
    ensure_warmed_once();
    let tools = registry::all()
        .iter()
        .map(registry::ToolDefinition::descriptor)
        .collect::<Vec<_>>();

    DomainResult::ok_body(json!({ "tools": tools }), CachePolicy::StablePrivate)
}

pub fn resources_list(_id: &Value) -> DomainResult {
    ensure_warmed_once();
    DomainResult::ok_body(
        json!({ "resources": crate::mcp::resources::list() }),
        CachePolicy::StablePrivate,
    )
}

pub fn resources_read(id: &Value, message: &Value) -> DomainResult {
    ensure_warmed_once();
    let Some(params) = message.get("params").and_then(Value::as_object) else {
        return DomainResult::invalid_params(id);
    };
    let Some(uri) = params.get("uri").and_then(Value::as_str) else {
        return DomainResult::invalid_params(id);
    };
    match crate::mcp::resources::read(uri) {
        Ok(result) => DomainResult::ok_body(result, CachePolicy::ImmediatePrivate),
        Err(err @ crate::mcp::resources::ReadError::BadRequest(_)) => {
            DomainResult::invalid_params_data(
                id,
                json!({ "reason": err.reason(), "uri": uri }),
            )
        }
        Err(err @ crate::mcp::resources::ReadError::Internal(_)) => DomainResult::internal_data(
            id,
            json!({ "reason": err.reason(), "uri": uri }),
        ),
        Err(err @ crate::mcp::resources::ReadError::QuotaExceeded(_)) => {
            DomainResult::internal_data(
                id,
                json!({ "reason": err.reason(), "uri": uri, "kind": "quota_exceeded" }),
            )
        }
    }
}

pub fn tools_call(id: &Value, message: &Value) -> DomainResult {
    ensure_warmed_once();
    let Some(params) = message.get("params").and_then(Value::as_object) else {
        return DomainResult::invalid_params(id);
    };

    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return DomainResult::invalid_params(id);
    };

    let Some(tool) = registry::find(name) else {
        return DomainResult::invalid_params_data(
            id,
            json!({
                "reason": "unknown-tool",
                "tool": name
            }),
        );
    };

    let empty_arguments = json!({});
    let arguments = params.get("arguments").unwrap_or(&empty_arguments);

    if tool.requires_auth && !mcp_tool_auth_ok() {
        return DomainResult::ok_body(
            mcp_tool_auth_required_result(tool, arguments),
            CachePolicy::None,
        );
    }

    let result = tool.call(arguments);

    if tool.charges_graph_egress && !gctx_tool_result_is_error(&result) {
        let payload_bytes = serde_json::to_vec(&result).map_or(0, |v| v.len() as u64);
        if !crate::mcp::resources::try_charge_graph_egress(payload_bytes) {
            return DomainResult::ok_body(gctx_quota_exceeded_result(tool.name), CachePolicy::None);
        }
    }

    DomainResult::ok_body(result, CachePolicy::None)
}

pub fn legacy_ping() -> DomainResult {
    DomainResult::ok_body(json!({}), CachePolicy::None)
}

pub fn legacy_shutdown() -> DomainResult {
    DomainResult::ok_body(Value::Null, CachePolicy::None)
}


/// A GCTX tool result is an error when its MCP envelope carries `isError: true`.
pub fn gctx_tool_result_is_error(result: &Value) -> bool {
    result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub fn gctx_quota_exceeded_result(tool_name: &str) -> Value {
    let reason = crate::mcp::resources::graph_egress_quota_reason();
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "error": reason,
                    "kind": "quota_exceeded",
                    "tool": tool_name,
                })).expect("quota-exceeded payload serialises")
            }
        ],
        "isError": true
    })
}

fn mcp_tool_auth_required_result(tool: &registry::ToolDefinition, arguments: &Value) -> Value {
    if tool.name == validate_write::TOOL_NAME {
        return mcp_auth_required_result(arguments);
    }

    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "schemaVersion": "anvil.mcp.auth-required.v1",
                    "decision": "gateUnavailable",
                    "safeDefault": "allow-with-warning",
                    "reason": "anvil MCP credentials are required for this tool. Run `anvil auth login` or `anvil auth login --edict`.",
                    "tool": tool.name,
                    "correlation": {
                        "daemonStatus": crate::mcp::validation::DaemonStatus::NotWired.as_str(),
                        "enforcementMode": "block",
                        "gateState": "unavailable"
                    }
                })).expect("auth-required payload serialises")
            }
        ],
        "isError": false
    })
}

pub fn mcp_tool_auth_ok() -> bool {
    if feature_flags::cli_dev_bypass_active().is_some() {
        return true;
    }

    let Ok(Some(creds)) = credentials::load() else {
        return false;
    };

    if credentials::is_expired(&creds) {
        return false;
    }

    if credentials::is_edict(&creds) {
        return cached_edict_auth_ok(&creds);
    }

    true
}

const EDICT_VERIFY_CACHE_TTL: Duration = Duration::from_mins(1);

#[derive(Clone)]
pub struct EdictAuthCacheEntry {
    pub license: String,
    pub checked_at: Instant,
    pub ok: bool,
}

pub fn edict_auth_cache() -> &'static Mutex<Option<EdictAuthCacheEntry>> {
    static CACHE: OnceLock<Mutex<Option<EdictAuthCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub fn edict_verify_cache_ttl() -> Duration {
    EDICT_VERIFY_CACHE_TTL
}

fn edict_verify_runtime() -> Option<&'static tokio::runtime::Runtime> {
    static RT: OnceLock<Option<tokio::runtime::Runtime>> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
    })
    .as_ref()
}

fn cached_edict_auth_ok(creds: &credentials::Credentials) -> bool {
    if let Ok(guard) = edict_auth_cache().lock()
        && let Some(entry) = guard.as_ref()
        && entry.license == creds.license
        && entry.checked_at.elapsed() < EDICT_VERIFY_CACHE_TTL
    {
        return entry.ok;
    }

    let ok = verify_mcp_edict_auth(creds);

    if let Ok(mut guard) = edict_auth_cache().lock() {
        *guard = Some(EdictAuthCacheEntry {
            license: creds.license.clone(),
            checked_at: Instant::now(),
            ok,
        });
    }
    ok
}

fn verify_mcp_edict_auth(creds: &credentials::Credentials) -> bool {
    let Some(rt) = edict_verify_runtime() else {
        return false;
    };

    let Ok(client) = crate::auth::client::AnvilClient::with_token(creds.license.clone()) else {
        return false;
    };

    rt.block_on(client.verify_edict()).is_ok()
}

pub fn mcp_auth_required_result(arguments: &Value) -> Value {
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("<unknown>");
    let payload = json!({
        "schema": "anvil.mcp.validate-write.v1",
        "decision": "gateUnavailable",
        "error": {
            "code": "authentication-required",
            "message": "Pre-write gate unavailable: authentication required. Run `anvil auth login` or `anvil auth login --edict`. The write may proceed; the gate could not validate it.",
            "retriable": true
        },
        "safeDefault": "allow-with-warning",
        "correlation": {
            "id": "corr_mcp_auth_required",
            "surface": "mcp",
            "mode": "preWrite",
            "backend": "embedded",
            "daemonStatus": "not-wired",
            "path": path,
            "enforcementMode": "block",
            "gateState": "unavailable"
        }
    });
    let text = serde_json::to_string(&payload).expect("auth-required payload serialises");
    json!({
        "content": [{"type": "text", "text": text}],
        "isError": false
    })
}
