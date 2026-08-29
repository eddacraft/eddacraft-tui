//! Canonical JSON normalisation for revision and classified-value digests.

use serde_json::{Map, Value};

/// Sort object keys recursively while preserving array order and scalar values.
pub(crate) fn canonicalise(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonicalise).collect()),
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            let mut canonical = Map::new();
            for key in keys {
                canonical.insert(key.clone(), canonicalise(&object[key]));
            }
            Value::Object(canonical)
        }
        scalar => scalar.clone(),
    }
}
