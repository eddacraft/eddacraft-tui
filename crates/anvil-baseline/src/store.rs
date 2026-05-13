use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::diff::{BaselineDiff, BaselineDiffEntry};
use crate::finding::BaselineFinding;

/// Current `format_version` written by this crate. Bump on any
/// schema-breaking change; an older anvil reading a newer file should
/// refuse rather than silently re-interpret the bytes.
pub const FORMAT_VERSION: u16 = 1;

/// Adoption-time metadata for the baseline.
///
/// Pinned at the moment `anvil baseline` runs. The `project_uuid`
/// must match `anvil/project-id` (validated by the CLI consumer, not
/// by this library — the file path lives in MLP-001's identity
/// module).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineMetadata {
    /// ISO-8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`). Format
    /// matches `WitnessLine.ts` for consistency across the protection
    /// surface.
    pub created_at: String,

    /// Anvil version string at adoption time, e.g. `"0.7.0-beta"`.
    pub created_by_version: String,

    /// Project UUID from `anvil/project-id`. The consumer is expected
    /// to cross-check this matches the on-disk identity before
    /// committing a `baseline.json`.
    pub project_uuid: String,
}

/// On-disk representation of `anvil/baseline.json`.
///
/// The shape is small and explicit; future fields go at the end of
/// the struct so older parsers stay forward-compatible (serde drops
/// unknown keys by default — preserving them is a follow-up if needed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    /// Schema version. Compared against [`FORMAT_VERSION`] on load.
    pub format_version: u16,

    /// Adoption metadata.
    pub metadata: BaselineMetadata,

    /// Optional commit SHA that L4 server-side validation treats as
    /// the cut-over point: commits at or before this SHA are exempt
    /// from witness-presence checks. Used by `validate_at_l4` when
    /// MLP-006 materialises.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cutoff_commit: Option<String>,

    /// Recorded findings, sorted by `(rule_id, file_path,
    /// fingerprint)` after `canonicalise()` so file output is
    /// deterministic.
    pub findings: Vec<BaselineFinding>,
}

#[derive(Debug, Error)]
pub enum FormatError {
    /// JSON decode error.
    #[error("baseline.json is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// `format_version` outside the range this crate understands.
    /// Older anvil cannot read newer baselines.
    #[error("baseline.json format_version {got} is newer than supported {supported}")]
    UnsupportedFormat { got: u16, supported: u16 },
    /// A finding's `fingerprint` was malformed.
    #[error("baseline.json finding has malformed fingerprint {raw:?}")]
    MalformedFingerprint { raw: String },
    /// A finding's `rule_id` was empty or non-ASCII.
    #[error("baseline.json finding has invalid rule_id {raw:?}")]
    InvalidRuleId { raw: String },
    /// A finding's `file_path` was empty.
    #[error("baseline.json finding has empty file_path")]
    EmptyFilePath,
}

impl Baseline {
    /// Build a fresh baseline at adoption time.
    pub fn new(metadata: BaselineMetadata, findings: Vec<BaselineFinding>) -> Self {
        let mut b = Self {
            format_version: FORMAT_VERSION,
            metadata,
            cutoff_commit: None,
            findings,
        };
        b.canonicalise();
        b
    }

    /// Sort findings deterministically + dedup exact duplicates so
    /// the on-disk shape is byte-stable across machines.
    pub fn canonicalise(&mut self) {
        self.findings
            .sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        self.findings.dedup_by(|a, b| a.sort_key() == b.sort_key());
    }

    /// Validate invariants. Called after deserialisation.
    pub fn validate(&self) -> Result<(), FormatError> {
        if self.format_version > FORMAT_VERSION {
            return Err(FormatError::UnsupportedFormat {
                got: self.format_version,
                supported: FORMAT_VERSION,
            });
        }
        for f in &self.findings {
            if f.file_path.is_empty() {
                return Err(FormatError::EmptyFilePath);
            }
            if f.rule_id.is_empty() || !f.rule_id.is_ascii() {
                return Err(FormatError::InvalidRuleId {
                    raw: f.rule_id.clone(),
                });
            }
            if f.fingerprint.len() != 16
                || !f
                    .fingerprint
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            {
                return Err(FormatError::MalformedFingerprint {
                    raw: f.fingerprint.clone(),
                });
            }
        }
        Ok(())
    }

    /// Encode the baseline as canonical JSON bytes (sorted top-level
    /// keys, no insignificant whitespace, trailing newline) suitable
    /// for direct file write.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, FormatError> {
        let mut map: BTreeMap<&'static str, Value> = BTreeMap::new();
        map.insert(
            "cutoff_commit",
            match &self.cutoff_commit {
                Some(s) => Value::String(s.clone()),
                None => Value::Null,
            },
        );
        map.insert(
            "findings",
            Value::Array(
                self.findings
                    .iter()
                    .map(|f| {
                        let mut entry: BTreeMap<&'static str, Value> = BTreeMap::new();
                        entry.insert("file_path", Value::String(f.file_path.clone()));
                        entry.insert("fingerprint", Value::String(f.fingerprint.clone()));
                        entry.insert("rule_id", Value::String(f.rule_id.clone()));
                        serde_json::to_value(&entry).expect("BTreeMap<&str, Value> serialises")
                    })
                    .collect(),
            ),
        );
        map.insert("format_version", Value::Number(self.format_version.into()));
        let mut meta: BTreeMap<&'static str, Value> = BTreeMap::new();
        meta.insert(
            "created_at",
            Value::String(self.metadata.created_at.clone()),
        );
        meta.insert(
            "created_by_version",
            Value::String(self.metadata.created_by_version.clone()),
        );
        meta.insert(
            "project_uuid",
            Value::String(self.metadata.project_uuid.clone()),
        );
        map.insert("metadata", serde_json::to_value(&meta)?);
        let mut bytes = serde_json::to_vec(&map)?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// Parse from canonical bytes (or any valid JSON the schema
    /// understands). Validates invariants before returning.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FormatError> {
        let trimmed = bytes.strip_suffix(b"\n").unwrap_or(bytes);
        let mut b: Self = serde_json::from_slice(trimmed)?;
        b.canonicalise();
        b.validate()?;
        Ok(b)
    }

    /// Compare this baseline (treated as "recorded") against a
    /// freshly-scanned set of findings. Returns the partition of
    /// `(unchanged, added, removed)` for the diff.
    pub fn diff(&self, new_scan: &[BaselineFinding]) -> BaselineDiff {
        use std::collections::HashSet;
        let recorded: HashSet<(&str, &str, &str)> = self
            .findings
            .iter()
            .map(|f| {
                (
                    f.rule_id.as_str(),
                    f.file_path.as_str(),
                    f.fingerprint.as_str(),
                )
            })
            .collect();
        let scanned: HashSet<(&str, &str, &str)> = new_scan
            .iter()
            .map(|f| {
                (
                    f.rule_id.as_str(),
                    f.file_path.as_str(),
                    f.fingerprint.as_str(),
                )
            })
            .collect();
        let mut unchanged: Vec<BaselineDiffEntry> = Vec::new();
        let mut added: Vec<BaselineDiffEntry> = Vec::new();
        let mut removed: Vec<BaselineDiffEntry> = Vec::new();
        for f in &self.findings {
            let key = (
                f.rule_id.as_str(),
                f.file_path.as_str(),
                f.fingerprint.as_str(),
            );
            if scanned.contains(&key) {
                unchanged.push(BaselineDiffEntry::from(f.clone()));
            } else {
                removed.push(BaselineDiffEntry::from(f.clone()));
            }
        }
        for f in new_scan {
            let key = (
                f.rule_id.as_str(),
                f.file_path.as_str(),
                f.fingerprint.as_str(),
            );
            if !recorded.contains(&key) {
                added.push(BaselineDiffEntry::from(f.clone()));
            }
        }
        unchanged.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        added.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        removed.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        BaselineDiff {
            unchanged,
            added,
            removed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute_fingerprint;

    fn finding(rule: &str, file: &str, snippet: &str) -> BaselineFinding {
        BaselineFinding {
            rule_id: rule.to_string(),
            file_path: file.to_string(),
            fingerprint: compute_fingerprint(rule, snippet).unwrap(),
        }
    }

    fn metadata() -> BaselineMetadata {
        BaselineMetadata {
            created_at: "2026-05-13T00:00:00Z".to_string(),
            created_by_version: "0.7.0-beta".to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
        }
    }

    #[test]
    fn new_canonicalises_findings_order() {
        let b = Baseline::new(
            metadata(),
            vec![
                finding("rule-z", "b.rs", "snippet-1"),
                finding("rule-a", "a.rs", "snippet-2"),
                finding("rule-a", "a.rs", "snippet-1"),
            ],
        );
        // Sorted by (rule_id, file_path, fingerprint).
        assert_eq!(b.findings[0].rule_id, "rule-a");
        assert_eq!(b.findings[1].rule_id, "rule-a");
        assert_eq!(b.findings[2].rule_id, "rule-z");
    }

    #[test]
    fn new_dedups_exact_duplicates() {
        let dup = finding("rule-a", "a.rs", "snippet-1");
        let b = Baseline::new(metadata(), vec![dup.clone(), dup.clone(), dup]);
        assert_eq!(b.findings.len(), 1);
    }

    #[test]
    fn canonical_bytes_are_deterministic() {
        let b1 = Baseline::new(
            metadata(),
            vec![
                finding("rule-z", "z.rs", "snippet-1"),
                finding("rule-a", "a.rs", "snippet-2"),
            ],
        );
        let b2 = Baseline::new(
            metadata(),
            vec![
                finding("rule-a", "a.rs", "snippet-2"),
                finding("rule-z", "z.rs", "snippet-1"),
            ],
        );
        assert_eq!(
            b1.to_canonical_bytes().unwrap(),
            b2.to_canonical_bytes().unwrap()
        );
    }

    #[test]
    fn round_trip_canonical_bytes() {
        let original = Baseline::new(
            metadata(),
            vec![finding("rule-a", "src/lib.rs", "let x = 1;")],
        );
        let bytes = original.to_canonical_bytes().unwrap();
        let parsed = Baseline::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn round_trip_preserves_cutoff_commit() {
        let mut b = Baseline::new(metadata(), vec![]);
        b.cutoff_commit = Some("abc123".to_string());
        let bytes = b.to_canonical_bytes().unwrap();
        let parsed = Baseline::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.cutoff_commit.as_deref(), Some("abc123"));
    }

    #[test]
    fn cutoff_commit_omitted_when_none() {
        let b = Baseline::new(metadata(), vec![]);
        let bytes = b.to_canonical_bytes().unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        // cutoff_commit serialises as `null`, which is fine — the
        // canonical encoder emits it explicitly so the byte shape is
        // deterministic. The struct's `skip_serializing_if` only
        // applies when round-tripping through the derive, not when
        // we hand-build the BTreeMap. Pin the current behaviour so
        // future changes are explicit.
        assert!(s.contains("\"cutoff_commit\":null"));
    }

    #[test]
    fn validate_rejects_newer_format_version() {
        let mut b = Baseline::new(metadata(), vec![]);
        b.format_version = FORMAT_VERSION + 1;
        let err = b.validate().unwrap_err();
        assert!(matches!(err, FormatError::UnsupportedFormat { .. }));
    }

    #[test]
    fn validate_rejects_empty_file_path() {
        let mut b = Baseline::new(metadata(), vec![]);
        b.findings.push(BaselineFinding {
            rule_id: "rule-a".to_string(),
            file_path: String::new(),
            fingerprint: "0".repeat(16),
        });
        let err = b.validate().unwrap_err();
        assert!(matches!(err, FormatError::EmptyFilePath));
    }

    #[test]
    fn validate_rejects_bad_fingerprint() {
        let mut b = Baseline::new(metadata(), vec![]);
        b.findings.push(BaselineFinding {
            rule_id: "rule-a".to_string(),
            file_path: "x".to_string(),
            fingerprint: "BAD".to_string(),
        });
        let err = b.validate().unwrap_err();
        assert!(matches!(err, FormatError::MalformedFingerprint { .. }));
    }

    #[test]
    fn validate_rejects_non_ascii_rule_id() {
        let mut b = Baseline::new(metadata(), vec![]);
        b.findings.push(BaselineFinding {
            rule_id: "règle".to_string(),
            file_path: "x".to_string(),
            fingerprint: "0".repeat(16),
        });
        let err = b.validate().unwrap_err();
        assert!(matches!(err, FormatError::InvalidRuleId { .. }));
    }

    #[test]
    fn diff_partitions_unchanged_added_removed() {
        let recorded = Baseline::new(
            metadata(),
            vec![
                finding("rule-a", "a.rs", "snippet-1"),
                finding("rule-a", "b.rs", "snippet-1"),
                finding("rule-b", "c.rs", "snippet-1"),
            ],
        );
        // New scan: keep first two, remove the third, and add one
        // new finding.
        let new_scan = vec![
            finding("rule-a", "a.rs", "snippet-1"),
            finding("rule-a", "b.rs", "snippet-1"),
            finding("rule-c", "d.rs", "snippet-1"),
        ];
        let diff = recorded.diff(&new_scan);
        assert_eq!(diff.unchanged.len(), 2);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.added[0].rule_id, "rule-c");
        assert_eq!(diff.removed[0].rule_id, "rule-b");
    }

    #[test]
    fn diff_handles_empty_recorded_and_scan() {
        let recorded = Baseline::new(metadata(), vec![]);
        let diff = recorded.diff(&[]);
        assert!(diff.unchanged.is_empty());
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn diff_treats_same_fingerprint_in_different_path_as_distinct() {
        // Move-resistance is at the *fingerprint* level (same
        // snippet content), but file_path is part of the identity
        // key — moving the file changes the diff entry.
        let recorded = Baseline::new(metadata(), vec![finding("rule-a", "old.rs", "let x = 1;")]);
        let new_scan = vec![finding("rule-a", "new.rs", "let x = 1;")];
        let diff = recorded.diff(&new_scan);
        assert_eq!(diff.unchanged.len(), 0);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 1);
    }
}
