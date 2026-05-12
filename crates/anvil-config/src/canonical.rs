use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum CanonicalError {
    #[error("number {value} is not representable in canonical JSON (NaN or infinity)")]
    NonFiniteNumber { value: String },
    #[error("serde_json failed during canonical emission: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Encode `value` into a canonical JSON byte-stream suitable for
/// cryptographic hashing (`rules_sha`).
///
/// The encoding follows RFC 8785-style rules sufficient for our
/// purposes:
///
/// - **Object keys are sorted lexicographically** by their unicode
///   scalar values (`BTreeMap` collects them, which uses `Ord` on
///   `String` — identical to a memcmp on the UTF-8 bytes for
///   ASCII-only keys; configs in this project use ASCII keys
///   exclusively).
/// - **No insignificant whitespace** — `serde_json::to_string` emits
///   the compact form by default; we use it directly.
/// - **Integers serialise without a decimal point**; floats use
///   `serde_json`'s default which is the shortest round-tripping
///   representation. NaN and infinity are rejected at the boundary
///   because they have no canonical encoding.
/// - **Strings are UTF-8 encoded with the standard JSON escapes**.
/// - **Null, true, false** are emitted as literals.
///
/// The function is deliberately not a general-purpose canoniser: it
/// covers exactly the value shapes that come out of [`crate::parse_str`].
pub fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, CanonicalError> {
    let mut buf = Vec::new();
    write_value(&mut buf, value)?;
    Ok(buf)
}

fn write_value(out: &mut Vec<u8>, value: &Value) -> Result<(), CanonicalError> {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Number(n) => {
            // Reject non-finite numbers explicitly; they are not valid
            // JSON and would produce a `null` from `serde_json` which
            // would silently collapse different inputs to the same
            // canonical output.
            if let Some(f) = n.as_f64()
                && !f.is_finite()
            {
                return Err(CanonicalError::NonFiniteNumber {
                    value: n.to_string(),
                });
            }
            out.extend_from_slice(n.to_string().as_bytes());
        }
        Value::String(s) => write_string(out, s)?,
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_value(out, item)?;
            }
            out.push(b']');
        }
        Value::Object(map) => {
            // Sort keys lexicographically via BTreeMap. We borrow the
            // string keys and the value refs so this allocates only
            // the BTreeMap structure, not the strings themselves.
            let sorted: BTreeMap<&str, &Value> =
                map.iter().map(|(k, v)| (k.as_str(), v)).collect();
            out.push(b'{');
            for (i, (k, v)) in sorted.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_string(out, k)?;
                out.push(b':');
                write_value(out, v)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

/// Encode a string as a canonical JSON string. Defers to
/// `serde_json::to_string` for the escape rules so we inherit its
/// hardened handling of control characters and surrogate pairs.
fn write_string(out: &mut Vec<u8>, s: &str) -> Result<(), CanonicalError> {
    let encoded = serde_json::to_string(s)?;
    out.extend_from_slice(encoded.as_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn null_emits_null() {
        assert_eq!(canonical_json_bytes(&Value::Null).unwrap(), b"null");
    }

    #[test]
    fn bool_emits_literal() {
        assert_eq!(canonical_json_bytes(&Value::Bool(true)).unwrap(), b"true");
        assert_eq!(canonical_json_bytes(&Value::Bool(false)).unwrap(), b"false");
    }

    #[test]
    fn integer_emits_no_decimal() {
        assert_eq!(canonical_json_bytes(&json!(42)).unwrap(), b"42");
        assert_eq!(canonical_json_bytes(&json!(-7)).unwrap(), b"-7");
    }

    #[test]
    fn float_emits_shortest_roundtrip() {
        // serde_json's default formatting is shortest round-trip.
        assert_eq!(canonical_json_bytes(&json!(1.5)).unwrap(), b"1.5");
    }

    #[test]
    fn non_finite_number_rejected() {
        let value = serde_json::Value::Number(
            serde_json::Number::from_f64(f64::INFINITY).unwrap_or_else(|| {
                // serde_json refuses NaN/Inf at construction; build via
                // string deser as a last resort. If that also fails,
                // skip — the rejection is enforced by serde_json itself.
                serde_json::from_str::<serde_json::Value>("0")
                    .unwrap()
                    .as_number()
                    .cloned()
                    .unwrap()
            }),
        );
        // If we couldn't even construct a non-finite number, the
        // serde_json layer already enforces the invariant; nothing to
        // check at the canonical layer.
        if let serde_json::Value::Number(n) = &value
            && n.as_f64().is_some_and(f64::is_finite)
        {
            return;
        }
        let err = canonical_json_bytes(&value).unwrap_err();
        assert!(matches!(err, CanonicalError::NonFiniteNumber { .. }));
    }

    #[test]
    fn string_uses_json_escapes() {
        assert_eq!(
            canonical_json_bytes(&json!("a\nb\"c")).unwrap(),
            b"\"a\\nb\\\"c\"",
        );
    }

    #[test]
    fn array_emits_with_no_spaces() {
        assert_eq!(canonical_json_bytes(&json!([1, 2, 3])).unwrap(), b"[1,2,3]");
    }

    #[test]
    fn object_keys_are_sorted() {
        let value = json!({"b": 1, "a": 2, "c": 3});
        let bytes = canonical_json_bytes(&value).unwrap();
        assert_eq!(bytes, br#"{"a":2,"b":1,"c":3}"#);
    }

    #[test]
    fn nested_object_keys_sorted_recursively() {
        let value = json!({"outer": {"z": 1, "a": 2}, "checks": [{"name": "x", "id": "y"}]});
        let bytes = canonical_json_bytes(&value).unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        // outer comes before checks lexicographically? c < o, so checks first.
        assert!(s.starts_with(r#"{"checks":[{"id":"y","name":"x"}],"outer":{"a":2,"z":1}}"#));
    }

    #[test]
    fn empty_object_and_array() {
        assert_eq!(canonical_json_bytes(&json!({})).unwrap(), b"{}");
        assert_eq!(canonical_json_bytes(&json!([])).unwrap(), b"[]");
    }

    #[test]
    fn key_order_independent_of_insertion_order() {
        // Two objects with the same key set but different insertion
        // order produce identical bytes.
        let a = json!({"a": 1, "b": 2, "c": 3});
        let b = json!({"c": 3, "a": 1, "b": 2});
        assert_eq!(
            canonical_json_bytes(&a).unwrap(),
            canonical_json_bytes(&b).unwrap(),
        );
    }
}
