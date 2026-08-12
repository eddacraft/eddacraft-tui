//! MLP2-041 — per-consumer typed views over the `serde_json::Value`
//! intermediate that MLP-011's `parse_file` / `parse_str` returns.
//!
//! The `anvil-config` crate parses any of `.anvil.{yaml,yml,json,toml}` (and
//! the legacy `.anvilrc`) into a `serde_json::Value`. Consumers historically
//! either read the `Value` directly (ad-hoc field probing) or built their own
//! ad-hoc structs that hand-parsed each format. Both routes lose
//! type-boundary validation and duplicate parsing logic.
//!
//! This module defines per-consumer `*ConfigView` structs with a
//! `from_value(&Value)` constructor that validates field shape and types up
//! front, so consumers downstream only deal with strongly typed data. The
//! pattern is intentionally minimal — consumers migrate at their own pace
//! (see `plans/modules/multilayer-protection-v2.aps.md` MLP2-041 for the
//! incremental-migration contract).
//!
//! Pattern reference: [`anvil_config::RuleModes::from_value`].
//!
//! `#![allow(dead_code)]` is intentional: the spec is explicit that
//! migration is incremental. The views ship now as a known-good foundation;
//! `commands/gate.rs`, `commands/init.rs`, and `crates/anvil-policy` adopt
//! them in their own follow-up commits without coupling those changes to
//! this one. Tests in this module exercise every public item, so the
//! foundation is regression-protected even before consumers wire it up.
#![allow(dead_code)]

use serde_json::Value;

/// Errors produced by `*ConfigView::from_value` constructors. Path strings
/// use dotted notation matching the original config key (e.g. `checks[2]`
/// or `enforcement.checks`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigViewError {
    #[error("invalid {path}: expected an object, found {found}")]
    NotAnObject { path: String, found: &'static str },

    #[error("invalid {path}: expected {expected}, found {found}")]
    WrongType {
        path: String,
        expected: &'static str,
        found: &'static str,
    },

    #[error("invalid {path}: missing required field")]
    MissingField { path: String },
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn require_object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, Value>, ConfigViewError> {
    value
        .as_object()
        .ok_or_else(|| ConfigViewError::NotAnObject {
            path: path.to_string(),
            found: json_kind(value),
        })
}

/// Read a `Vec<String>` from a `checks`-shaped key. Treats absent / null as
/// empty (matches the legacy `extract_checks_from_*` helpers in
/// `commands/gate.rs`). Returns an error for a present-but-wrong-type
/// `checks` value (e.g. `checks: "secrets"`) so the type boundary catches
/// fat-finger configs that the legacy `filter_map(as_str)` quietly swallowed.
fn read_string_array(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    path_prefix: &str,
) -> Result<Vec<String>, ConfigViewError> {
    let Some(v) = obj.get(key) else {
        return Ok(Vec::new());
    };
    if v.is_null() {
        return Ok(Vec::new());
    }
    let arr = v.as_array().ok_or_else(|| ConfigViewError::WrongType {
        path: format!("{path_prefix}{key}"),
        expected: "array of string",
        found: json_kind(v),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let s = item.as_str().ok_or_else(|| ConfigViewError::WrongType {
            path: format!("{path_prefix}{key}[{i}]"),
            expected: "string",
            found: json_kind(item),
        })?;
        out.push(s.to_string());
    }
    Ok(out)
}

fn read_required_string(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    path_prefix: &str,
) -> Result<String, ConfigViewError> {
    let v = obj.get(key).ok_or_else(|| ConfigViewError::MissingField {
        path: format!("{path_prefix}{key}"),
    })?;
    v.as_str()
        .map(str::to_string)
        .ok_or_else(|| ConfigViewError::WrongType {
            path: format!("{path_prefix}{key}"),
            expected: "string",
            found: json_kind(v),
        })
}

/// Typed view for `commands/gate.rs` over a parsed `.anvil.<ext>` /
/// `.anvilrc` value. Replaces the trio of `extract_checks_from_{json,yaml,
/// toml}` helpers with a single boundary-validated read.
///
/// The legacy helpers `filter_map`-ed non-string array members away; this
/// view rejects them. The intent is the same downstream
/// (`validate_check_names` would have raised on the empty/wrong names
/// anyway) but the failure surface is moved up to the parser where the
/// path is still meaningful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateConfigView {
    pub checks: Vec<String>,
}

impl GateConfigView {
    pub fn from_value(value: &Value) -> Result<Self, ConfigViewError> {
        let obj = require_object(value, "config")?;
        let checks = read_string_array(obj, "checks", "")?;
        Ok(Self { checks })
    }
}

/// Typed view for `commands/init.rs` over a parsed `.anvil.<ext>` /
/// `.anvilrc` value. Mirrors the fields the existing private
/// `init::AnvilConfig` struct serializes.
///
/// Key casing: every writer emits canonical `schema_version` /
/// `planning_dir` since ADR-120 / UCFG-003; files written by pre-flip
/// YAML/JSON producers carry `schemaVersion` / `planningDir`. The view
/// accepts `camelCase` first and falls back to `snake_case` so it reads
/// both eras without forcing a config-format migration (mixed-key
/// precedence reconciliation is UCFG-002's owned-write scope).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitConfigView {
    pub schema_version: String,
    pub planning_dir: String,
    pub format: String,
    pub checks: Vec<String>,
}

impl InitConfigView {
    pub fn from_value(value: &Value) -> Result<Self, ConfigViewError> {
        let obj = require_object(value, "config")?;
        let schema_version = read_camel_or_snake(obj, "schemaVersion", "schema_version", "")?;
        let planning_dir = read_camel_or_snake(obj, "planningDir", "planning_dir", "")?;
        let format = read_required_string(obj, "format", "")?;
        let checks = read_string_array(obj, "checks", "")?;
        Ok(Self {
            schema_version,
            planning_dir,
            format,
            checks,
        })
    }
}

fn read_camel_or_snake(
    obj: &serde_json::Map<String, Value>,
    camel: &str,
    snake: &str,
    path_prefix: &str,
) -> Result<String, ConfigViewError> {
    if obj.contains_key(camel) {
        read_required_string(obj, camel, path_prefix)
    } else if obj.contains_key(snake) {
        read_required_string(obj, snake, path_prefix)
    } else {
        Err(ConfigViewError::MissingField {
            path: format!("{path_prefix}{camel}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- GateConfigView ----

    #[test]
    fn gate_view_parses_check_list() {
        let v = json!({"checks": ["secret-detection", "import-boundaries"]});
        let view = GateConfigView::from_value(&v).unwrap();
        assert_eq!(view.checks, vec!["secret-detection", "import-boundaries"]);
    }

    #[test]
    fn gate_view_absent_checks_is_empty() {
        let v = json!({"other": 1});
        let view = GateConfigView::from_value(&v).unwrap();
        assert!(view.checks.is_empty());
    }

    #[test]
    fn gate_view_null_checks_is_empty() {
        let v = json!({"checks": null});
        let view = GateConfigView::from_value(&v).unwrap();
        assert!(view.checks.is_empty());
    }

    #[test]
    fn gate_view_non_array_checks_errors_with_path() {
        let v = json!({"checks": "secret-detection"});
        let err = GateConfigView::from_value(&v).unwrap_err();
        assert!(
            matches!(err, ConfigViewError::WrongType { ref path, .. } if path == "checks"),
            "got: {err:?}"
        );
    }

    #[test]
    fn gate_view_non_string_element_errors_with_indexed_path() {
        let v = json!({"checks": ["secret-detection", 42, "import-boundaries"]});
        let err = GateConfigView::from_value(&v).unwrap_err();
        match err {
            ConfigViewError::WrongType { path, .. } => assert_eq!(path, "checks[1]"),
            other => panic!("expected WrongType, got {other:?}"),
        }
    }

    #[test]
    fn gate_view_non_object_root_errors() {
        let v = json!([1, 2, 3]);
        let err = GateConfigView::from_value(&v).unwrap_err();
        assert!(
            matches!(err, ConfigViewError::NotAnObject { .. }),
            "{err:?}"
        );
    }

    // ---- InitConfigView ----

    #[test]
    fn init_view_parses_camelcase_keys() {
        let v = json!({
            "schemaVersion": "1.0.0",
            "planningDir": "plans",
            "format": "yaml",
            "checks": ["a", "b"],
        });
        let view = InitConfigView::from_value(&v).unwrap();
        assert_eq!(view.schema_version, "1.0.0");
        assert_eq!(view.planning_dir, "plans");
        assert_eq!(view.format, "yaml");
        assert_eq!(view.checks, vec!["a", "b"]);
    }

    #[test]
    fn init_view_parses_snake_case_keys_from_toml_writer() {
        // The legacy `toml_serialise` in `commands/init.rs` emits
        // `schema_version` / `planning_dir` (snake_case). The view must
        // accept both so a TOML-format `.anvilrc` round-trips through
        // MLP-011's `discover` + `parse_file`.
        let v = json!({
            "schema_version": "1.0.0",
            "planning_dir": "plans",
            "format": "toml",
            "checks": [],
        });
        let view = InitConfigView::from_value(&v).unwrap();
        assert_eq!(view.schema_version, "1.0.0");
        assert_eq!(view.planning_dir, "plans");
    }

    #[test]
    fn init_view_camelcase_wins_when_both_present() {
        // Mixed-key configs should not silently prefer one over the
        // other in an unprincipled way — pin the precedence so a future
        // refactor doesn't quietly flip it.
        let v = json!({
            "schemaVersion": "from-camel",
            "schema_version": "from-snake",
            "planningDir": "plans",
            "format": "yaml",
            "checks": [],
        });
        let view = InitConfigView::from_value(&v).unwrap();
        assert_eq!(view.schema_version, "from-camel");
    }

    #[test]
    fn init_view_missing_required_field_errors() {
        let v = json!({
            "planningDir": "plans",
            "format": "yaml",
            "checks": [],
        });
        let err = InitConfigView::from_value(&v).unwrap_err();
        match err {
            ConfigViewError::MissingField { path } => assert_eq!(path, "schemaVersion"),
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn init_view_wrong_type_errors_with_kind() {
        let v = json!({
            "schemaVersion": 1,
            "planningDir": "plans",
            "format": "yaml",
            "checks": [],
        });
        let err = InitConfigView::from_value(&v).unwrap_err();
        match err {
            ConfigViewError::WrongType {
                path,
                expected,
                found,
            } => {
                assert_eq!(path, "schemaVersion");
                assert_eq!(expected, "string");
                assert_eq!(found, "number");
            }
            other => panic!("expected WrongType, got {other:?}"),
        }
    }
}
