//! Canonical JSON encoding shared by capsule artefacts.
//!
//! Same discipline as `anvil-witness::WitnessLine::to_canonical_bytes`
//! (sorted keys, minimal whitespace — ADR-074 §Schema rules), but
//! **recursive**, because the capsule manifest nests objects where the
//! witness line is flat. Producers write canonical bytes to disk, so a
//! verifier can digest raw file bytes without re-parsing.

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Serialisation adapter that emits objects with sorted keys at every
/// depth. Sorting happens during emission, so canonical order is a
/// type-level guarantee — it does not depend on `serde_json`'s
/// `preserve_order` feature or on any map's internal ordering.
struct Canonical<'a>(&'a Value);

impl Serialize for Canonical<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Object(map) => {
                let mut entries: Vec<(&String, &Value)> = map.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                let mut out = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    out.serialize_entry(key, &Canonical(value))?;
                }
                out.end()
            }
            // Array order is semantic (e.g. an ordered check list);
            // only object key order is presentation noise.
            Value::Array(items) => {
                let mut out = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    out.serialize_element(&Canonical(item))?;
                }
                out.end()
            }
            other => other.serialize(serializer),
        }
    }
}

/// Encode a JSON value as canonical bytes: recursively sorted object
/// keys, minimal whitespace, no trailing newline.
///
/// # Errors
///
/// Returns the underlying `serde_json` error if the value cannot be
/// serialised (practically unreachable for values built from `Value`).
pub fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&Canonical(value))
}

/// SHA-256 hex digest of a byte slice — the digest form every manifest
/// `files` entry uses.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Key order in the input must not change the canonical bytes —
    /// this is what makes manifest digests reproducible across machines.
    #[test]
    fn canonical_bytes_ignore_object_key_order() {
        let a: Value = serde_json::from_str(r#"{"b":1,"a":{"d":2,"c":3}}"#).unwrap();
        let b: Value = serde_json::from_str(r#"{"a":{"c":3,"d":2},"b":1}"#).unwrap();
        assert_eq!(
            canonical_json_bytes(&a).unwrap(),
            canonical_json_bytes(&b).unwrap()
        );
    }

    /// Golden pin: the exact canonical encoding is part of the digest
    /// contract. Any change here is a schema-epoch event, not a refactor.
    #[test]
    fn canonical_bytes_golden() {
        let value = json!({"z": [3, 1], "a": {"k": "v"}, "m": null});
        let bytes = canonical_json_bytes(&value).unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            r#"{"a":{"k":"v"},"m":null,"z":[3,1]}"#
        );
    }

    /// Objects nested inside arrays sort too — emission-time sorting
    /// is depth-blind.
    #[test]
    fn canonical_bytes_sort_objects_inside_arrays() {
        let value: Value = serde_json::from_str(r#"[{"b":1,"a":2}]"#).unwrap();
        let bytes = canonical_json_bytes(&value).unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), r#"[{"a":2,"b":1}]"#);
    }

    /// Array order is semantic and must survive canonicalisation.
    #[test]
    fn canonical_bytes_keep_array_order() {
        let value = json!(["b", "a"]);
        let bytes = canonical_json_bytes(&value).unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), r#"["b","a"]"#);
    }

    /// Golden pin against the well-known SHA-256 empty-input vector.
    #[test]
    fn sha256_hex_golden() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
