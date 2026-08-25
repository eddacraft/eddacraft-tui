//! Sensitivity classification and fail-closed redaction (SETCON-003).

use serde_json::{Map, Value};

use crate::catalogue::Catalogue;
use crate::types::Sensitivity;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RedactionError {
    #[error("redaction failed on {key}: {reason}")]
    Failed { key: String, reason: String },
}

/// Abort helper: a redaction failure emits no settings payload.
#[must_use]
pub fn fail_closed(key: &str, reason: &str) -> RedactionError {
    RedactionError::Failed {
        key: key.to_owned(),
        reason: reason.to_owned(),
    }
}

/// Redact one setting's value using that key's catalogue classification.
///
/// Class D / Secret values become presence-only. Unclassified values are
/// hidden. After transformation, a leak of the original secret string fails
/// closed rather than emitting a degraded payload.
pub fn redact_setting_value(
    catalogue: &Catalogue,
    key: &str,
    value: &Value,
) -> Result<Value, RedactionError> {
    let sensitivity = catalogue
        .get(key)
        .map_or(Sensitivity::Unclassified, |e| e.sensitivity);
    let redacted = redact_classified(sensitivity, value);
    if sensitivity == Sensitivity::Secret {
        ensure_not_leaked(key, value, &redacted)?;
    }
    Ok(redacted)
}

/// Recursively redact a JSON object whose keys are canonical setting keys.
pub fn redact_value(catalogue: &Catalogue, value: &Value) -> Result<Value, RedactionError> {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), redact_setting_or_row(catalogue, k, v)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(redact_value(catalogue, item)?);
            }
            Ok(Value::Array(out))
        }
        other => Ok(other.clone()),
    }
}

/// Envelope `data` rows are `{requested, resolved, runtime}`. Redact the
/// value fields using the parent setting key; do not treat the wrapper as
/// the classified value (that would mark every Secret object `present`).
fn redact_setting_or_row(
    catalogue: &Catalogue,
    key: &str,
    value: &Value,
) -> Result<Value, RedactionError> {
    if let Value::Object(map) = value
        && (map.contains_key("requested") || map.contains_key("resolved"))
    {
        let mut out = Map::new();
        for (field, inner) in map {
            if field == "requested" || field == "resolved" {
                out.insert(field.clone(), redact_setting_value(catalogue, key, inner)?);
            } else {
                out.insert(field.clone(), inner.clone());
            }
        }
        return Ok(Value::Object(out));
    }
    redact_setting_value(catalogue, key, value)
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

fn ensure_not_leaked(key: &str, original: &Value, redacted: &Value) -> Result<(), RedactionError> {
    if let Some(secret) = original.as_str()
        && !secret.is_empty()
        && redacted_contains_string(redacted, secret)
    {
        return Err(fail_closed(key, "secret value leaked after redaction"));
    }
    Ok(())
}

fn redacted_contains_string(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(s) => s.contains(needle),
        Value::Array(items) => items
            .iter()
            .any(|item| redacted_contains_string(item, needle)),
        Value::Object(map) => map
            .values()
            .any(|item| redacted_contains_string(item, needle)),
        _ => false,
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

    #[test]
    fn redaction_fail_closed_when_secret_would_leak() {
        let err = ensure_not_leaked(
            "privacy.token",
            &Value::String("s3cret".into()),
            &Value::String("s3cret-still-here".into()),
        )
        .unwrap_err();
        assert_eq!(
            err,
            fail_closed("privacy.token", "secret value leaked after redaction")
        );
    }

    #[test]
    fn redaction_envelope_row_does_not_treat_wrapper_as_present() {
        let cat = cat_with(
            "privacy.license_token",
            Sensitivity::Secret,
            ConsequenceClass::D,
        );
        let mut row = Map::new();
        row.insert("requested".into(), Value::Null);
        row.insert("resolved".into(), Value::Null);
        row.insert("runtime".into(), Value::String("unknown".into()));
        let mut data = Map::new();
        data.insert("privacy.license_token".into(), Value::Object(row));
        let out = redact_value(&cat, &Value::Object(data)).unwrap();
        assert_eq!(
            out["privacy.license_token"]["requested"]["present"],
            Value::Bool(false)
        );
        assert_eq!(
            out["privacy.license_token"]["resolved"]["present"],
            Value::Bool(false)
        );
        assert_eq!(
            out["privacy.license_token"]["runtime"],
            Value::String("unknown".into())
        );
    }
}
