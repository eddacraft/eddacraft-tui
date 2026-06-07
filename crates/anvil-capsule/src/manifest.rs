//! The `anvil.capsule.v1` manifest — the digest-protected root of a
//! review capsule directory (ADR-074).
//!
//! The manifest lists a SHA-256 digest for every other file in the
//! capsule. Digests are over the file's bytes; producers write
//! canonical JSON (see [`crate::canonical`]), so byte digests are
//! reproducible across machines without the verifier re-parsing
//! evidence it has not yet trusted.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::canonical::{canonical_json_bytes, sha256_hex};
use crate::errors::CapsuleError;

/// The manifest schema identifier this crate produces and accepts.
pub const CAPSULE_SCHEMA: &str = "anvil.capsule.v1";

/// Evidence files every capsule must list (ADR-074 §Layout). Files are
/// present-but-empty rather than omitted when there is nothing to
/// report, so a missing file is unambiguously a tamper/corruption
/// signal, not "no findings".
pub const REQUIRED_FILES: [&str; 9] = [
    "commits.json",
    "policy.json",
    "baseline.json",
    "rules.json",
    "witness.ndjson",
    "diagnostics.sarif",
    "exceptions.json",
    "edda-context.json",
    "README.md",
];

/// Root manifest of a capsule directory (`manifest.json`).
///
/// Closed schema: unknown fields are a parse error, because a field the
/// parser ignored would be content the digest discipline cannot vouch
/// for round-tripping. Evolution is a new schema version, never a
/// silent extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapsuleManifest {
    /// Always [`CAPSULE_SCHEMA`]; gated on parse.
    pub schema: String,
    /// The commit range and witness coverage the capsule packages.
    pub range: CapsuleRange,
    /// Tool identity that produced the capsule.
    pub producer: Producer,
    /// Capsule file name → SHA-256 hex digest of that file's bytes.
    /// `BTreeMap` keeps emission order deterministic.
    pub files: BTreeMap<String, String>,
}

/// The commit range a capsule covers, plus pointers into the embedded
/// full witness chain (ADR-074: `verify_chain_dag` is genesis-anchored,
/// so `witness.ndjson` carries the complete chain and the range is
/// marked by sequence pointers, never by subsetting).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapsuleRange {
    /// Base commit SHA (exclusive end of `base..head`).
    pub base: String,
    /// Head commit SHA.
    pub head: String,
    /// First witness `seq` relevant to the range. Missing (not `null`)
    /// when the range has no witnessed lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness_seq_start: Option<u64>,
    /// Last witness `seq` relevant to the range. Missing (not `null`)
    /// when the range has no witnessed lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness_seq_end: Option<u64>,
}

/// Identity of the producing tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    /// The Anvil version that wrote the capsule.
    pub anvil_version: String,
}

impl CapsuleManifest {
    /// Build a manifest with the current schema version and no file
    /// digests yet; collectors fill [`Self::files`] as they write.
    #[must_use]
    pub fn new(range: CapsuleRange, producer: Producer) -> Self {
        Self {
            schema: CAPSULE_SCHEMA.to_string(),
            range,
            producer,
            files: BTreeMap::new(),
        }
    }

    /// Record `name`'s digest from the bytes that were written for it.
    pub fn record_file(&mut self, name: &str, bytes: &[u8]) {
        self.files.insert(name.to_string(), sha256_hex(bytes));
    }

    /// Required files (ADR-074 §Layout) not yet listed in
    /// [`Self::files`]. Non-empty means the capsule is incomplete —
    /// a `degraded` signal for the verifier, never `pass`.
    #[must_use]
    pub fn missing_required(&self) -> Vec<&'static str> {
        REQUIRED_FILES
            .into_iter()
            .filter(|name| !self.files.contains_key(*name))
            .collect()
    }

    /// Encode as canonical JSON bytes (sorted keys, minimal
    /// whitespace) — the byte form written to `manifest.json`.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::Serialise`] if encoding fails (practically
    /// unreachable).
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CapsuleError> {
        let value =
            serde_json::to_value(self).map_err(|e| CapsuleError::Serialise(e.to_string()))?;
        canonical_json_bytes(&value).map_err(|e| CapsuleError::Serialise(e.to_string()))
    }

    /// Parse and schema-gate a manifest from file bytes.
    ///
    /// # Errors
    ///
    /// [`CapsuleError::Parse`] for malformed JSON or unknown fields;
    /// [`CapsuleError::SchemaMismatch`] when the document declares a
    /// schema other than [`CAPSULE_SCHEMA`].
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, CapsuleError> {
        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|e| CapsuleError::Parse(e.to_string()))?;
        if manifest.schema != CAPSULE_SCHEMA {
            return Err(CapsuleError::SchemaMismatch {
                expected: CAPSULE_SCHEMA,
                found: manifest.schema,
            });
        }
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CapsuleManifest {
        let mut manifest = CapsuleManifest::new(
            CapsuleRange {
                base: "1111111111111111111111111111111111111111".to_string(),
                head: "2222222222222222222222222222222222222222".to_string(),
                witness_seq_start: Some(3),
                witness_seq_end: Some(9),
            },
            Producer {
                anvil_version: "0.7.4-beta".to_string(),
            },
        );
        manifest.record_file("commits.json", b"{}");
        manifest
    }

    #[test]
    fn manifest_round_trips_through_canonical_bytes() {
        let manifest = sample();
        let bytes = manifest.to_canonical_bytes().unwrap();
        let parsed = CapsuleManifest::from_json_bytes(&bytes).unwrap();
        assert_eq!(parsed, manifest);
        // Idempotent: re-encoding the parse yields identical bytes.
        assert_eq!(parsed.to_canonical_bytes().unwrap(), bytes);
    }

    #[test]
    fn manifest_rejects_unknown_schema_version() {
        let mut manifest = sample();
        manifest.schema = "anvil.capsule.v999".to_string();
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let err = CapsuleManifest::from_json_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CapsuleError::SchemaMismatch { .. }));
    }

    /// Closed schema: a field the parser would ignore is content the
    /// digest discipline cannot vouch for. Unknown fields are an error.
    #[test]
    fn manifest_rejects_unknown_fields() {
        let bytes = sample().to_canonical_bytes().unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["smuggled"] = serde_json::json!(true);
        let raw = serde_json::to_vec(&value).unwrap();
        let err = CapsuleManifest::from_json_bytes(&raw).unwrap_err();
        assert!(matches!(err, CapsuleError::Parse(_)));
    }

    /// Absent witness pointers serialise as missing keys, never `null`
    /// — the same canonical discipline `WitnessLine` uses.
    #[test]
    fn manifest_witness_pointers_absent_not_null() {
        let mut manifest = sample();
        manifest.range.witness_seq_start = None;
        manifest.range.witness_seq_end = None;
        let bytes = manifest.to_canonical_bytes().unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(!text.contains("witness_seq_start"));
        assert!(!text.contains("null"));
        let parsed = CapsuleManifest::from_json_bytes(&bytes).unwrap();
        assert_eq!(parsed.range.witness_seq_start, None);
    }

    /// Golden pin: the exact canonical encoding is the digest contract.
    /// A diff here is a schema-epoch event, not a refactor.
    #[test]
    fn manifest_canonical_bytes_golden() {
        let bytes = sample().to_canonical_bytes().unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            concat!(
                r#"{"files":{"commits.json":"44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"},"#,
                r#""producer":{"anvil_version":"0.7.4-beta"},"#,
                r#""range":{"base":"1111111111111111111111111111111111111111","#,
                r#""head":"2222222222222222222222222222222222222222","#,
                r#""witness_seq_end":9,"witness_seq_start":3},"#,
                r#""schema":"anvil.capsule.v1"}"#
            )
        );
    }

    #[test]
    fn manifest_missing_required_lists_unrecorded_files() {
        let manifest = sample();
        let missing = manifest.missing_required();
        assert!(!missing.contains(&"commits.json"));
        assert!(missing.contains(&"witness.ndjson"));
        assert_eq!(missing.len(), REQUIRED_FILES.len() - 1);
    }

    #[test]
    fn manifest_record_file_digests_bytes() {
        let mut manifest = sample();
        manifest.record_file("rules.json", b"");
        assert_eq!(
            manifest.files["rules.json"],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
