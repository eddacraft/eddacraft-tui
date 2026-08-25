//! Sensitivity classification and fail-closed redaction (SETCON-003).

use serde_json::{Map, Value};

use crate::catalogue::Catalogue;
use crate::types::Sensitivity;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RedactionError {
    #[error("redaction failed on {key}: {reason}")]
    Failed { key: String, reason: String },
}

/// Recursively redact `value` using catalogue classifications.
///
/// A failure aborts the entire payload — callers must emit nothing.
pub fn redact_value(catalogue: &Catalogue, value: &Value) -> Result<Value, RedactionError> {
    redact_walk(catalogue, value, None)
}

fn redact_walk(
    catalogue: &Catalogue,
    value: &Value,
    current_key: Option<&str>,
) -> Result<Value, RedactionError> {
    if let Some(key) = current_key
        && let Some(entry) = catalogue.get(key)
    {
        return Ok(redact_classified(entry.sensitivity, value));
    }
    if let Some(key) = current_key
        && catalogue.get(key).is_none()
    {
        // Unknown keys are unclassified: hide the value.
        return Ok(redact_classified(Sensitivity::Unclassified, value));
    }
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), redact_walk(catalogue, v, Some(k))?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(redact_walk(catalogue, item, current_key)?);
            }
            Ok(Value::Array(out))
        }
        other => Ok(other.clone()),
    }
}

fn redact_classified(sensitivity: Sensitivity, value: &Value) -> Value {
    match sensitivity {
        Sensitivity::Public | Sensitivity::Internal => value.clone(),
        Sensitivity::Secret => secret_presence(value),
        Sensitivity::Unclassified => hidden(),
    }
}

fn secret_presence(value: &Value) -> Value {
    let present = match value {
        Value::Null => false,
        Value::String(s) if s.is_empty() => false,
        Value::Array(items) if items.is_empty() => false,
        _ => true,
    };
    let mut map = Map::new();
    map.insert("present".into(), Value::Bool(present));
    Value::Object(map)
}

fn hidden() -> Value {
    let mut map = Map::new();
    map.insert("redacted".into(), Value::Bool(true));
    map.insert("reason".into(), Value::String("unclassified".into()));
    Value::Object(map)
}

/// Abort helper used when a channel cannot guarantee redaction.
#[must_use]
pub fn fail_closed(key: &str, reason: &str) -> RedactionError {
    RedactionError::Failed {
        key: key.to_owned(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod redaction_tests {
    use super::*;
    use crate::catalogue::{Catalogue, CatalogueEntry, Mutability};
    use crate::runtime_state::EvidenceTrust;
    use crate::types::{
        ConsequenceClass, EvidenceMode, HealthRelevance, MergeSemantics, Scope, SettingGroup,
        SettingKey, ValueType,
    };

    fn cat_with(key: &str, sensitivity: Sensitivity, class: ConsequenceClass) -> Catalogue {
        let mut cat = Catalogue::new();
        cat.register(CatalogueEntry {
            key: SettingKey(key.into()),
            label: key.into(),
            owner: "core".into(),
            group: SettingGroup::Privacy,
            order: 1,
            value_type: ValueType::String,
            default: None,
            supported_scopes: vec![Scope::User],
            precedence: vec![Scope::User],
            merge: MergeSemantics::Replace,
            mutability: Mutability::SettingsService,
            canonical_writer: "settings".into(),
            consequence_class: class,
            sensitivity,
            evidence_mode: EvidenceMode::None,
            health_relevance: HealthRelevance::None,
            activation_owner: None,
            evidence_trust: EvidenceTrust::None,
            docs_ref: None,
            deprecated_aliases: vec![],
            version_compatibility: "1".into(),
        })
        .unwrap();
        cat
    }

    #[test]
    fn redaction_hides_unclassified_and_class_d_presence_only() {
        let cat = cat_with("privacy.token", Sensitivity::Secret, ConsequenceClass::D);
        let mut map = Map::new();
        map.insert("privacy.token".into(), Value::String("s3cret".into()));
        let out = redact_value(&cat, &Value::Object(map)).unwrap();
        assert_eq!(out["privacy.token"]["present"], Value::Bool(true));
        assert!(out["privacy.token"].get("s3cret").is_none());
        assert_ne!(out["privacy.token"], Value::String("s3cret".into()));
    }

    #[test]
    fn redaction_unknown_key_is_hidden_not_emitted() {
        let cat = Catalogue::new();
        let mut map = Map::new();
        map.insert("mystery".into(), Value::String("nope".into()));
        let out = redact_value(&cat, &Value::Object(map)).unwrap();
        assert_eq!(out["mystery"]["redacted"], Value::Bool(true));
        assert_ne!(out["mystery"], Value::String("nope".into()));
    }
}
