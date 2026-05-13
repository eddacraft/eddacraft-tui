use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::genesis::GenesisAnchor;

/// A single witness chain entry. One per line in the active ndjson
/// file. The on-disk representation is **canonical JSON**: object
/// keys sorted lexicographically, no insignificant whitespace, so two
/// machines emitting the same logical record produce byte-identical
/// lines (required for the hash chain to be deterministic).
///
/// Fields that are optional on disk use `Option`; missing fields
/// round-trip as missing rather than as JSON `null`, so the canonical
/// bytes don't depend on whether the writer used a default or
/// declined to set the field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessLine {
    /// Monotonic sequence number within the scope. Starts at 1.
    pub seq: u64,
    /// Scope identifier — "active" for the live file; archive files
    /// can carry their own scope token if a future split is needed.
    pub scope: String,
    /// Stable kind discriminator (e.g. "witness", "rollover",
    /// "baseline-refreshed"). Reserved values are documented in
    /// ADR-037.
    pub kind: String,
    /// Either an [`GenesisAnchor`] string ("GENESIS-FRESH" /
    /// "GENESIS-BASELINED") or the SHA-256 hex of the prior line's
    /// canonical bytes. ADR-037 §D-2.
    pub prev_line_hash: String,
    /// The `project_uuid` from `anvil/project-id`. Cross-machine
    /// identity anchor.
    pub project_uuid: String,
    /// The commit SHA this line attests to. Optional because some
    /// kinds (rollover events, baseline-refreshed) are chain
    /// bookkeeping, not commit attestation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    /// Merge-commit parent SHAs (DAG-aware witness; MLP-005). Empty
    /// for ordinary linear commits; one entry per parent on a merge
    /// commit, in `git rev-list --parents` order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_commits: Vec<String>,
    /// Per-parent `prev_line_hash` for merge-commit DAG joins
    /// (MLP-005). Indexed in lockstep with `parent_commits`; `None`
    /// expresses "this parent had no witnessed history."
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prev_line_hashes: Vec<Option<String>>,
    /// Optional agent tag for multi-agent attribution (MLP-014).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_tag: Option<String>,
    /// Optional rule-set hash (MLP-012). Locks rule version into
    /// the evidence stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules_sha: Option<String>,
    /// ISO-8601 UTC timestamp at write time. Format `YYYY-MM-DDTHH:MM:SSZ`.
    pub ts: String,
    /// Where the validation that produced this line ran — e.g.
    /// "pre-commit", "pre-push", "bootstrap-recovery".
    pub validation_at: String,
}

impl WitnessLine {
    /// Encode the line as canonical JSON bytes (no trailing newline).
    ///
    /// This is the byte sequence that gets SHA-256'd for the next
    /// line's `prev_line_hash`. Sorted keys + minimal whitespace
    /// guarantee determinism.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        // Round-trip through a Value so we can re-emit with sorted
        // keys. `BTreeMap<String, Value>` gives us the ordering for
        // free; nested objects (if any are added later) need their
        // own sort, but the current schema is flat.
        let value = serde_json::to_value(self)?;
        let object = value
            .as_object()
            .expect("WitnessLine serialises to a JSON object");
        let sorted: BTreeMap<&str, &serde_json::Value> =
            object.iter().map(|(k, v)| (k.as_str(), v)).collect();
        serde_json::to_vec(&sorted)
    }

    /// Encode the line for ndjson output: canonical bytes + `\n`.
    pub fn to_ndjson_line(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut bytes = self.to_canonical_bytes()?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// Parse a single ndjson line back into a `WitnessLine`. The
    /// trailing newline (if present) is tolerated.
    pub fn from_ndjson_line(line: &[u8]) -> Result<Self, serde_json::Error> {
        let trimmed = line.strip_suffix(b"\n").unwrap_or(line);
        serde_json::from_slice(trimmed)
    }

    /// Convenience builder for the first line in a fresh chain.
    pub fn genesis(
        anchor: &GenesisAnchor,
        project_uuid: impl Into<String>,
        scope: impl Into<String>,
        ts: impl Into<String>,
        validation_at: impl Into<String>,
    ) -> Self {
        Self {
            seq: 1,
            scope: scope.into(),
            kind: "witness".to_string(),
            prev_line_hash: anchor.anchor_string().to_string(),
            project_uuid: project_uuid.into(),
            commit_sha: None,
            parent_commits: Vec::new(),
            prev_line_hashes: Vec::new(),
            agent_tag: None,
            rules_sha: None,
            ts: ts.into(),
            validation_at: validation_at.into(),
        }
    }
}

/// A SHA-256 hex digest of a canonical witness line.
pub type LineHash = String;

/// Compute the SHA-256 hex digest of `canonical_bytes`. Used by both
/// the writer (to set the next line's `prev_line_hash`) and the
/// verifier (to detect tamper).
pub fn compute_line_hash(canonical_bytes: &[u8]) -> LineHash {
    let digest = Sha256::digest(canonical_bytes);
    hex::encode(digest)
}

/// A loose record alias for callers that want to build `WitnessLine`s
/// without spelling out every default-able field.
pub type WitnessRecord = WitnessLine;

#[cfg(test)]
mod tests {
    use super::*;

    fn line(seq: u64) -> WitnessLine {
        WitnessLine {
            seq,
            scope: "active".to_string(),
            kind: "witness".to_string(),
            prev_line_hash: GenesisAnchor::Fresh.anchor_string().to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
            commit_sha: Some("abcdef".to_string()),
            parent_commits: Vec::new(),
            prev_line_hashes: Vec::new(),
            agent_tag: None,
            rules_sha: None,
            ts: "2026-05-13T00:00:00Z".to_string(),
            validation_at: "pre-commit".to_string(),
        }
    }

    #[test]
    fn canonical_keys_are_sorted() {
        let l = line(1);
        let bytes = l.to_canonical_bytes().unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        // `commit_sha` < `kind` < `prev_line_hash` < `project_uuid`
        // < `scope` < `seq` < `ts` < `validation_at` lexicographically.
        let positions = [
            "commit_sha",
            "kind",
            "prev_line_hash",
            "project_uuid",
            "scope",
            "seq",
            "ts",
            "validation_at",
        ];
        let mut last = 0usize;
        for key in positions {
            let pos = s.find(key).unwrap_or_else(|| panic!("missing key: {key}"));
            assert!(
                pos >= last,
                "key {key} appears at {pos} which is before previous {last}; canonical order broken"
            );
            last = pos;
        }
    }

    #[test]
    fn optional_fields_skipped_when_none() {
        let mut l = line(1);
        l.commit_sha = None;
        l.agent_tag = None;
        l.rules_sha = None;
        let bytes = l.to_canonical_bytes().unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(!s.contains("commit_sha"));
        assert!(!s.contains("agent_tag"));
        assert!(!s.contains("rules_sha"));
    }

    #[test]
    fn round_trip_via_ndjson() {
        let l = line(7);
        let bytes = l.to_ndjson_line().unwrap();
        assert!(bytes.ends_with(b"\n"));
        let parsed = WitnessLine::from_ndjson_line(&bytes).unwrap();
        assert_eq!(parsed, l);
    }

    #[test]
    fn parse_tolerates_missing_trailing_newline() {
        let l = line(7);
        let mut bytes = l.to_canonical_bytes().unwrap();
        // No newline appended.
        let parsed = WitnessLine::from_ndjson_line(&bytes).unwrap();
        assert_eq!(parsed, l);
        // And with newline:
        bytes.push(b'\n');
        let parsed2 = WitnessLine::from_ndjson_line(&bytes).unwrap();
        assert_eq!(parsed2, l);
    }

    #[test]
    fn compute_line_hash_is_deterministic() {
        let l = line(1);
        let bytes = l.to_canonical_bytes().unwrap();
        let h1 = compute_line_hash(&bytes);
        let h2 = compute_line_hash(&bytes);
        assert_eq!(h1, h2);
        // Length is 64 hex chars (256 bits).
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn compute_line_hash_changes_on_payload_change() {
        let a = line(1).to_canonical_bytes().unwrap();
        let b = line(2).to_canonical_bytes().unwrap();
        assert_ne!(compute_line_hash(&a), compute_line_hash(&b));
    }

    #[test]
    fn genesis_helper_sets_anchor_in_prev_field() {
        let l = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "uuid-123",
            "active",
            "2026-05-13T00:00:00Z",
            "pre-commit",
        );
        assert_eq!(l.seq, 1);
        assert_eq!(l.prev_line_hash, "GENESIS-FRESH");
    }
}
