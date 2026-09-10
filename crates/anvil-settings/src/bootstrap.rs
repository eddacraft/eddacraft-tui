//! Bounded, syntax-preserving project-config bootstrap mutation.
//!
//! This is deliberately not a general settings writer. It may only add a
//! missing project-scoped target registered in the settings catalogue, and it
//! returns `NeedsInput` whenever a safe insertion span cannot be proven.

use std::path::Path;

use anvil_config::ConfigFormat;
use serde_json::Value;

use crate::Catalogue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapSetting {
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapMutation {
    Unchanged,
    Updated(String),
    NeedsInput { patch: String, reason: String },
}

#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("unknown or non-bootstrap project setting {0}")]
    UnknownSetting(String),
    #[error("invalid project config: {0}")]
    InvalidConfig(String),
    #[error("duplicate bootstrap setting {0}")]
    DuplicateSetting(String),
    #[error("bootstrap value does not match the catalogue type for {0}")]
    InvalidValue(String),
}

impl Catalogue {
    /// Plan additions to an existing project config without normalising any
    /// unrelated byte. Existing values are always operator-owned.
    pub fn plan_project_config_bootstrap(
        &self,
        existing: &str,
        format: ConfigFormat,
        additions: &[BootstrapSetting],
    ) -> Result<BootstrapMutation, BootstrapError> {
        let parsed = anvil_config::parse_str(existing, format, Path::new("project config"))
            .map_err(|error| BootstrapError::InvalidConfig(error.to_string()))?;
        if !parsed.is_object() {
            return Ok(BootstrapMutation::NeedsInput {
                patch: render_patch(format, additions),
                reason: "project config root is not an object".to_owned(),
            });
        }

        let mut seen = std::collections::BTreeSet::new();
        let mut missing = Vec::new();
        for addition in additions {
            if !seen.insert(addition.key.as_str()) {
                return Err(BootstrapError::DuplicateSetting(addition.key.clone()));
            }
            let target = self
                .project_config_target(&addition.key)
                .ok_or_else(|| BootstrapError::UnknownSetting(addition.key.clone()))?;
            let entry = self
                .get(&addition.key)
                .ok_or_else(|| BootstrapError::UnknownSetting(addition.key.clone()))?;
            if !value_matches_type(&addition.value, &entry.value_type) {
                return Err(BootstrapError::InvalidValue(addition.key.clone()));
            }
            let Some(root_key) = target
                .pointer
                .strip_prefix('/')
                .filter(|key| !key.contains('/'))
            else {
                return Ok(BootstrapMutation::NeedsInput {
                    patch: render_patch(format, additions),
                    reason: format!(
                        "{} requires a nested insertion that cannot yet preserve syntax",
                        addition.key
                    ),
                });
            };
            if parsed.pointer(&target.pointer).is_none() {
                missing.push((root_key, &addition.value));
            }
        }
        if missing.is_empty() {
            return Ok(BootstrapMutation::Unchanged);
        }

        let updated = match format {
            ConfigFormat::Yaml | ConfigFormat::Yml => append_yaml(existing, &missing),
            ConfigFormat::Toml => append_toml(existing, &missing),
            ConfigFormat::Json => insert_json(existing, &missing),
        };
        let Some(updated) = updated else {
            return Ok(BootstrapMutation::NeedsInput {
                patch: render_patch(format, additions),
                reason: "a syntax-preserving insertion point could not be proven".to_owned(),
            });
        };
        if anvil_config::parse_str(&updated, format, Path::new("project config")).is_err() {
            return Ok(BootstrapMutation::NeedsInput {
                patch: render_patch(format, additions),
                reason: "the candidate insertion does not preserve a valid document".to_owned(),
            });
        }
        Ok(BootstrapMutation::Updated(updated))
    }
}

fn value_matches_type(value: &Value, value_type: &crate::ValueType) -> bool {
    match value_type {
        crate::ValueType::Boolean => value.is_boolean(),
        crate::ValueType::String => value.is_string(),
        crate::ValueType::Integer => value.is_i64() || value.is_u64(),
        crate::ValueType::Enum { allowed } => value
            .as_str()
            .is_some_and(|candidate| allowed.iter().any(|item| item == candidate)),
        crate::ValueType::List | crate::ValueType::Set => value.is_array(),
        crate::ValueType::Map => value.is_object(),
    }
}

fn append_yaml(existing: &str, missing: &[(&str, &Value)]) -> Option<String> {
    // Explicit document terminators make an append ambiguous. Root mappings
    // with a final stream are safe because parsing already proved the shape.
    if existing.lines().any(|line| line.trim() == "...") {
        return None;
    }
    let mut out = existing.to_owned();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    for (key, value) in missing {
        out.push_str(key);
        out.push_str(": ");
        out.push_str(&serde_json::to_string(value).ok()?);
        out.push('\n');
    }
    Some(out)
}

fn append_toml(existing: &str, missing: &[(&str, &Value)]) -> Option<String> {
    let first_table = existing
        .lines()
        .position(|line| line.trim_start().starts_with('['));
    let mut additions = String::new();
    for (key, value) in missing {
        additions.push_str(key);
        additions.push_str(" = ");
        additions.push_str(&serde_json::to_string(value).ok()?);
        additions.push('\n');
    }
    if first_table.is_none() {
        let mut out = existing.to_owned();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&additions);
        return Some(out);
    }
    let line_index = first_table?;
    let byte_index = existing
        .split_inclusive('\n')
        .take(line_index)
        .map(str::len)
        .sum::<usize>();
    let mut out = existing.to_owned();
    out.insert_str(byte_index, &additions);
    Some(out)
}

fn insert_json(existing: &str, missing: &[(&str, &Value)]) -> Option<String> {
    let closing = existing.trim_end().strip_suffix('}')?.len();
    let object = serde_json::from_str::<Value>(existing).ok()?;
    let non_empty = object.as_object().is_some_and(|map| !map.is_empty());
    let mut insertion = String::new();
    if non_empty {
        insertion.push(',');
    }
    insertion.push('\n');
    for (index, (key, value)) in missing.iter().enumerate() {
        insertion.push_str("  ");
        insertion.push_str(&serde_json::to_string(key).ok()?);
        insertion.push_str(": ");
        insertion.push_str(&serde_json::to_string(value).ok()?);
        if index + 1 != missing.len() {
            insertion.push(',');
        }
        insertion.push('\n');
    }
    let mut out = existing.to_owned();
    out.insert_str(closing, &insertion);
    Some(out)
}

fn render_patch(format: ConfigFormat, additions: &[BootstrapSetting]) -> String {
    let missing = additions
        .iter()
        .map(|addition| (addition.key.as_str(), &addition.value))
        .collect::<Vec<_>>();
    match format {
        ConfigFormat::Yaml | ConfigFormat::Yml => append_yaml("", &missing).unwrap_or_default(),
        ConfigFormat::Toml => append_toml("", &missing).unwrap_or_default(),
        ConfigFormat::Json => insert_json("{}", &missing).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn foundation() -> Vec<BootstrapSetting> {
        vec![
            BootstrapSetting {
                key: "project.schema_version".into(),
                value: Value::String("1.0.0".into()),
            },
            BootstrapSetting {
                key: "project.format".into(),
                value: Value::String("yaml".into()),
            },
        ]
    }

    #[test]
    fn yaml_addition_preserves_unrelated_bytes() {
        let cat = crate::first_release_catalogue().unwrap();
        let input = "# keep me\nenforcement:\n  mode: block\n";
        let BootstrapMutation::Updated(output) = cat
            .plan_project_config_bootstrap(input, ConfigFormat::Yaml, &foundation())
            .unwrap()
        else {
            panic!("expected update");
        };
        assert!(output.starts_with(input));
        assert!(output.contains("schema_version: \"1.0.0\"\n"));
    }

    #[test]
    fn json_addition_preserves_existing_member_bytes() {
        let cat = crate::first_release_catalogue().unwrap();
        let input = "{\n    \"checks\" : [\"fmt\"]\n}\n";
        let BootstrapMutation::Updated(output) = cat
            .plan_project_config_bootstrap(input, ConfigFormat::Json, &foundation())
            .unwrap()
        else {
            panic!("expected update");
        };
        assert!(output.contains("    \"checks\" : [\"fmt\"]"));
        assert!(serde_json::from_str::<Value>(&output).is_ok());
    }

    #[test]
    fn existing_values_are_never_replaced() {
        let cat = crate::first_release_catalogue().unwrap();
        let input = "schema_version: custom\nformat: custom\n";
        assert_eq!(
            cat.plan_project_config_bootstrap(input, ConfigFormat::Yaml, &foundation())
                .unwrap(),
            BootstrapMutation::Unchanged
        );
    }

    #[test]
    fn bootstrap_rejects_values_outside_the_canonical_setting_type() {
        let cat = crate::first_release_catalogue().unwrap();
        let invalid = [BootstrapSetting {
            key: "project.format".into(),
            value: Value::String("xml".into()),
        }];

        assert!(matches!(
            cat.plan_project_config_bootstrap("{}", ConfigFormat::Json, &invalid),
            Err(BootstrapError::InvalidValue(key)) if key == "project.format"
        ));
    }
}
