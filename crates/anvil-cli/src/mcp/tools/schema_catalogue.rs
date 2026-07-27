//! MCP26-008: catalogue-level JSON Schema 2020-12 checks for published tools.
//!
//! Each tool `inputSchema` must:
//! - be a JSON object with root `"type": "object"` (MCP tools contract);
//! - compile as a JSON Schema under Draft 2020-12;
//! - pass meta-schema validation;
//! - not contain external `$ref` values (no network dereference);
//! - stay within a bounded nesting depth.
//!
//! Any future `outputSchema` is checked with the same rules (object root not
//! required for output schemas per MCP SEP-2106).

use serde_json::Value;

use super::registry;

/// Maximum nesting depth for published tool schemas (properties / items / defs).
pub const MAX_SCHEMA_DEPTH: usize = 16;

/// Validate every registered tool descriptor's schema fields.
///
/// Returns `Ok(())` when the whole catalogue is clean, or `Err` with one
/// human-readable line per failing tool field.
pub fn validate_catalogue_schemas() -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for tool in registry::all() {
        let descriptor = tool.descriptor();
        match descriptor.get("inputSchema") {
            None => errors.push(format!("{}: missing inputSchema", tool.name)),
            Some(schema) => {
                if let Err(e) = validate_input_schema(tool.name, schema) {
                    errors.push(e);
                }
            }
        }
        if let Some(output) = descriptor.get("outputSchema")
            && let Err(e) = validate_output_schema(tool.name, output)
        {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_input_schema(tool_name: &str, schema: &Value) -> Result<(), String> {
    let Value::Object(map) = schema else {
        return Err(format!("{tool_name}: inputSchema must be a JSON object"));
    };
    match map.get("type") {
        Some(Value::String(t)) if t == "object" => {}
        Some(other) => {
            return Err(format!(
                "{tool_name}: inputSchema root type must be \"object\", got {other}"
            ));
        }
        None => {
            return Err(format!(
                "{tool_name}: inputSchema root must declare \"type\": \"object\""
            ));
        }
    }
    validate_schema_common(tool_name, "inputSchema", schema)
}

fn validate_output_schema(tool_name: &str, schema: &Value) -> Result<(), String> {
    if !schema.is_object() {
        return Err(format!("{tool_name}: outputSchema must be a JSON object"));
    }
    validate_schema_common(tool_name, "outputSchema", schema)
}

fn validate_schema_common(tool_name: &str, field: &str, schema: &Value) -> Result<(), String> {
    let depth = schema_depth(schema, 0);
    if depth > MAX_SCHEMA_DEPTH {
        return Err(format!(
            "{tool_name}: {field} nesting depth {depth} exceeds limit {MAX_SCHEMA_DEPTH}"
        ));
    }
    if let Some(uri) = first_external_ref(schema) {
        return Err(format!(
            "{tool_name}: {field} must not use external $ref (found {uri})"
        ));
    }

    // Meta-schema validity under Draft 2020-12 (jsonschema crate default meta path).
    if let Err(err) = jsonschema::meta::validate(schema) {
        return Err(format!(
            "{tool_name}: {field} failed JSON Schema 2020-12 meta validation: {err}"
        ));
    }

    // Compilability as a Draft 2020-12 schema (catches draft-specific issues).
    if let Err(err) = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(schema)
    {
        return Err(format!(
            "{tool_name}: {field} does not compile as Draft 2020-12: {err}"
        ));
    }

    Ok(())
}

fn schema_depth(value: &Value, current: usize) -> usize {
    match value {
        Value::Object(map) => map
            .values()
            .map(|v| schema_depth(v, current + 1))
            .max()
            .unwrap_or(current),
        Value::Array(items) => items
            .iter()
            .map(|v| schema_depth(v, current + 1))
            .max()
            .unwrap_or(current),
        _ => current,
    }
}

fn first_external_ref(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(uri)) = map.get("$ref")
                && (uri.starts_with("http://")
                    || uri.starts_with("https://")
                    || uri.starts_with("//"))
            {
                return Some(uri.clone());
            }
            for child in map.values() {
                if let Some(found) = first_external_ref(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => {
            for child in items {
                if let Some(found) = first_external_ref(child) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn entire_published_catalogue_validates_as_draft_2020_12() {
        validate_catalogue_schemas().unwrap_or_else(|errs| {
            panic!(
                "catalogue schema validation failed:\n{}",
                errs.join("\n")
            );
        });
    }

    #[test]
    fn rejects_non_object_root_input_schema() {
        let schema = json!({ "type": "string" });
        let err = validate_input_schema("demo", &schema).unwrap_err();
        assert!(err.contains("object"), "{err}");
    }

    #[test]
    fn rejects_external_ref() {
        let schema = json!({
            "type": "object",
            "properties": {
                "x": { "$ref": "https://example.com/schema.json" }
            }
        });
        let err = validate_input_schema("demo", &schema).unwrap_err();
        assert!(err.contains("external $ref"), "{err}");
    }

    #[test]
    fn accepts_local_defs_ref() {
        let schema = json!({
            "type": "object",
            "$defs": {
                "name": { "type": "string" }
            },
            "properties": {
                "n": { "$ref": "#/$defs/name" }
            }
        });
        validate_input_schema("demo", &schema).expect("local $ref should be fine");
    }
}
