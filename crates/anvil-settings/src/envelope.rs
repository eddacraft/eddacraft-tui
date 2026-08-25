//! Versioned `anvil.settings.v1` JSON envelope (SETCON-008).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::catalogue::Catalogue;
use crate::health::Health;
use crate::redaction::{RedactionError, redact_value};

pub const SCHEMA_VERSION: &str = "anvil.settings.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeCommand {
    Show,
    Explain,
    Status,
    Sources,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub schema_version: String,
    pub command: EnvelopeCommand,
    pub generated_at: String,
    pub model_revision: String,
    pub context: Value,
    pub health: Health,
    pub data: Value,
    pub diagnostics: Vec<Value>,
}

impl Envelope {
    /// Build an envelope. Display labels are excluded: `data` is keyed by
    /// canonical setting keys only.
    #[must_use]
    pub fn new(
        command: EnvelopeCommand,
        generated_at: String,
        model_revision: String,
        context: Value,
        health: Health,
        data: Value,
        diagnostics: Vec<Value>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_owned(),
            command,
            generated_at,
            model_revision,
            context,
            health,
            data,
            diagnostics,
        }
    }

    /// Recursively redact setting values in `data` and `context`. Envelope
    /// metadata (`schema_version`, `command`, …) is not a setting key space.
    /// On failure the caller must emit no payload.
    pub fn redacted(&self, catalogue: &Catalogue) -> Result<Value, RedactionError> {
        let mut raw = serde_json::to_value(self).map_err(|err| RedactionError::Failed {
            key: "envelope".into(),
            reason: err.to_string(),
        })?;
        let obj = raw.as_object_mut().ok_or_else(|| RedactionError::Failed {
            key: "envelope".into(),
            reason: "envelope is not an object".into(),
        })?;
        for field in ["data", "context"] {
            if let Some(value) = obj.get(field).cloned() {
                obj.insert(field.to_owned(), redact_value(catalogue, &value)?);
            }
        }
        Ok(raw)
    }
}

/// Canonical empty context object.
#[must_use]
pub fn empty_object() -> Value {
    Value::Object(Map::new())
}

#[cfg(test)]
mod envelope_tests {
    use super::*;
    use crate::catalogue::Catalogue;
    use crate::health::HealthStatus;
    use serde_json::json;

    #[test]
    fn envelope_schema_and_canonical_keys() {
        let env = Envelope::new(
            EnvelopeCommand::Show,
            "2026-08-25T00:00:00Z".into(),
            "rev-1".into(),
            empty_object(),
            Health {
                status: HealthStatus::Healthy,
                reasons: vec![],
            },
            json!({"protection.checks": ["secret-detection"]}),
            vec![],
        );
        let value = serde_json::to_value(&env).unwrap();
        assert_eq!(value["schema_version"], SCHEMA_VERSION);
        assert!(value.get("label").is_none());
        assert!(value["data"].get("protection.checks").is_some());
        let encoded = serde_json::to_string(&env).unwrap();
        assert!(encoded.contains("anvil.settings.v1"));
        assert!(!encoded.contains("\"label\""));
    }

    #[test]
    fn envelope_redaction_applies_to_whole_document() {
        let env = Envelope::new(
            EnvelopeCommand::Show,
            "2026-08-25T00:00:00Z".into(),
            "rev-1".into(),
            empty_object(),
            Health {
                status: HealthStatus::Healthy,
                reasons: vec![],
            },
            json!({"mystery": "nope"}),
            vec![],
        );
        let redacted = env.redacted(&Catalogue::new()).unwrap();
        assert_eq!(redacted["data"]["mystery"]["redacted"], json!(true));
    }
}
