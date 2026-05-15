//! MLP2-041 — typed view over the `serde_json::Value` intermediate that
//! MLP-011's `anvil_config::parse_file` returns, scoped to the
//! `anvil-policy` consumer.
//!
//! The existing [`crate::config::PolicyConfig`] uses `serde_yaml::from_str`
//! directly and is YAML-only. This view sits over the format-neutral
//! `serde_json::Value` so a policy file in `.yaml`, `.yml`, `.json`, or
//! `.toml` reads through the same boundary, with up-front type validation
//! at the field level.
//!
//! Migration is incremental: the existing `load_config` path is unchanged.
//! Consumers ready to move to the multi-format surface call
//! `PolicyConfigView::from_value(&anvil_config::parse_file(path)?)`.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PolicyViewError {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEntryView {
    pub id: String,
    pub name: String,
    pub category: String,
    pub enabled: bool,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyConfigView {
    pub policies: Vec<PolicyEntryView>,
}

impl PolicyConfigView {
    pub fn from_value(value: &Value) -> Result<Self, PolicyViewError> {
        let root = value
            .as_object()
            .ok_or_else(|| PolicyViewError::NotAnObject {
                path: "config".to_string(),
                found: json_kind(value),
            })?;

        let policies = match root.get("policies") {
            None | Some(Value::Null) => Vec::new(),
            Some(v) => {
                let arr = v.as_array().ok_or_else(|| PolicyViewError::WrongType {
                    path: "policies".to_string(),
                    expected: "array of objects",
                    found: json_kind(v),
                })?;
                let mut out = Vec::with_capacity(arr.len());
                for (i, raw) in arr.iter().enumerate() {
                    out.push(parse_entry(raw, i)?);
                }
                out
            }
        };

        Ok(Self { policies })
    }
}

fn parse_entry(value: &Value, index: usize) -> Result<PolicyEntryView, PolicyViewError> {
    let obj = value
        .as_object()
        .ok_or_else(|| PolicyViewError::NotAnObject {
            path: format!("policies[{index}]"),
            found: json_kind(value),
        })?;
    let prefix = format!("policies[{index}].");
    Ok(PolicyEntryView {
        id: read_string(obj, "id", &prefix)?,
        name: read_string(obj, "name", &prefix)?,
        category: read_string(obj, "category", &prefix)?,
        enabled: read_bool(obj, "enabled", &prefix)?,
        description: read_string(obj, "description", &prefix)?,
        severity: read_string(obj, "severity", &prefix)?,
    })
}

fn read_string(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<String, PolicyViewError> {
    let v = obj.get(key).ok_or_else(|| PolicyViewError::MissingField {
        path: format!("{prefix}{key}"),
    })?;
    v.as_str()
        .map(str::to_string)
        .ok_or_else(|| PolicyViewError::WrongType {
            path: format!("{prefix}{key}"),
            expected: "string",
            found: json_kind(v),
        })
}

fn read_bool(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<bool, PolicyViewError> {
    let v = obj.get(key).ok_or_else(|| PolicyViewError::MissingField {
        path: format!("{prefix}{key}"),
    })?;
    v.as_bool().ok_or_else(|| PolicyViewError::WrongType {
        path: format!("{prefix}{key}"),
        expected: "bool",
        found: json_kind(v),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_policies_parses_to_empty_vec() {
        let v = json!({"policies": []});
        let view = PolicyConfigView::from_value(&v).unwrap();
        assert!(view.policies.is_empty());
    }

    #[test]
    fn absent_policies_parses_to_empty_vec() {
        let v = json!({"other": 1});
        let view = PolicyConfigView::from_value(&v).unwrap();
        assert!(view.policies.is_empty());
    }

    #[test]
    fn single_well_formed_entry_parses() {
        let v = json!({
            "policies": [{
                "id": "policy-1",
                "name": "Example",
                "category": "testing",
                "enabled": true,
                "description": "desc",
                "severity": "medium",
            }]
        });
        let view = PolicyConfigView::from_value(&v).unwrap();
        assert_eq!(view.policies.len(), 1);
        assert_eq!(view.policies[0].id, "policy-1");
        assert!(view.policies[0].enabled);
    }

    #[test]
    fn missing_required_field_reports_indexed_path() {
        let v = json!({
            "policies": [{
                "id": "policy-1",
                "category": "testing",
                "enabled": true,
                "description": "desc",
                "severity": "medium",
            }]
        });
        let err = PolicyConfigView::from_value(&v).unwrap_err();
        match err {
            PolicyViewError::MissingField { path } => assert_eq!(path, "policies[0].name"),
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn wrong_type_for_enabled_is_caught() {
        let v = json!({
            "policies": [{
                "id": "policy-1",
                "name": "Example",
                "category": "testing",
                "enabled": "yes",
                "description": "desc",
                "severity": "medium",
            }]
        });
        let err = PolicyConfigView::from_value(&v).unwrap_err();
        match err {
            PolicyViewError::WrongType { path, expected, .. } => {
                assert_eq!(path, "policies[0].enabled");
                assert_eq!(expected, "bool");
            }
            other => panic!("expected WrongType, got {other:?}"),
        }
    }

    #[test]
    fn non_object_policy_entry_errors() {
        let v = json!({"policies": ["not-an-object"]});
        let err = PolicyConfigView::from_value(&v).unwrap_err();
        match err {
            PolicyViewError::NotAnObject { path, .. } => assert_eq!(path, "policies[0]"),
            other => panic!("expected NotAnObject, got {other:?}"),
        }
    }

    #[test]
    fn non_array_policies_errors() {
        let v = json!({"policies": "no"});
        let err = PolicyConfigView::from_value(&v).unwrap_err();
        match err {
            PolicyViewError::WrongType { path, .. } => assert_eq!(path, "policies"),
            other => panic!("expected WrongType, got {other:?}"),
        }
    }

    #[test]
    fn non_object_root_errors() {
        let v = json!(["not", "an", "object"]);
        let err = PolicyConfigView::from_value(&v).unwrap_err();
        assert!(matches!(err, PolicyViewError::NotAnObject { .. }));
    }
}
