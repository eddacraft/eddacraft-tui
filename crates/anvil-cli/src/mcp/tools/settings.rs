//! Read-only settings MCP tools (SETINS-007).

use serde_json::{Value, json};

use crate::commands::settings as settings_cmd;
use crate::mcp::tools::shared::{redact_workspace_root, validate_workspace_root};

pub const SHOW: &str = "anvil_settings_show";
pub const EXPLAIN: &str = "anvil_settings_explain";
pub const STATUS: &str = "anvil_settings_status";
pub const SOURCES: &str = "anvil_settings_sources";

fn workspace_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "workspaceRoot": {
                "type": "string",
                "description": "Absolute path to the project root directory"
            }
        },
        "required": ["workspaceRoot"],
        "additionalProperties": true
    })
}

pub fn show_descriptor() -> Value {
    json!({
        "name": SHOW,
        "description": "Read-only settings show envelope (anvil.settings.v1).",
        "inputSchema": workspace_schema(),
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true
        }
    })
}

pub fn explain_descriptor() -> Value {
    json!({
        "name": EXPLAIN,
        "description": "Read-only explain of one settings key from the same read model.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": { "type": "string" },
                "key": { "type": "string", "description": "Canonical setting key" }
            },
            "required": ["workspaceRoot", "key"],
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true
        }
    })
}

pub fn status_descriptor() -> Value {
    json!({
        "name": STATUS,
        "description": "Read-only settings status envelope.",
        "inputSchema": workspace_schema(),
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true
        }
    })
}

pub fn sources_descriptor() -> Value {
    json!({
        "name": SOURCES,
        "description": "Read-only settings sources envelope.",
        "inputSchema": workspace_schema(),
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true
        }
    })
}

fn admit(arguments: &Value) -> Result<(), String> {
    let server_root = crate::mcp::tools::shared::mcp_server_root()?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot is required".to_string())?;
    let workspace_path = std::path::Path::new(workspace_root);
    let _ = validate_workspace_root(workspace_path, &server_root)?;
    let _ = redact_workspace_root(workspace_path, &server_root);
    Ok(())
}

fn wrap(result: anyhow::Result<Value>) -> Value {
    match result {
        Ok(payload) => json!({ "content": [{ "type": "text", "text": payload.to_string() }] }),
        Err(error) => json!({ "error": error.to_string(), "isError": true }),
    }
}

pub fn show_call(arguments: &Value) -> Value {
    if let Err(error) = admit(arguments) {
        return json!({ "error": error, "isError": true });
    }
    wrap(settings_cmd::mcp_show())
}

pub fn explain_call(arguments: &Value) -> Value {
    if let Err(error) = admit(arguments) {
        return json!({ "error": error, "isError": true });
    }
    let key = arguments.get("key").and_then(Value::as_str).unwrap_or("");
    wrap(settings_cmd::mcp_explain(key))
}

pub fn status_call(arguments: &Value) -> Value {
    if let Err(error) = admit(arguments) {
        return json!({ "error": error, "isError": true });
    }
    wrap(settings_cmd::mcp_status())
}

pub fn sources_call(arguments: &Value) -> Value {
    if let Err(error) = admit(arguments) {
        return json!({ "error": error, "isError": true });
    }
    wrap(settings_cmd::mcp_sources())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_mcp_descriptors_are_read_only() {
        for descriptor in [
            show_descriptor(),
            explain_descriptor(),
            status_descriptor(),
            sources_descriptor(),
        ] {
            assert_eq!(descriptor["annotations"]["readOnlyHint"], true);
            assert_eq!(descriptor["annotations"]["destructiveHint"], false);
            let name = descriptor["name"].as_str().unwrap_or_default();
            assert!(name.starts_with("anvil_settings_"));
            assert!(!name.ends_with("_set"));
            assert!(!name.contains("unset"));
        }
    }
}
