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

    /// MLP2-036: `true` when this baseline is a partial-scan
    /// snapshot — the scan budget was exhausted before every file
    /// was processed. The companion [`Self::continuation`] field
    /// names the next file to scan. A consumer that sees
    /// `partial=true` should treat the findings list as incomplete
    /// (`anvil status` renders this as a degraded state) and may
    /// trigger a resume scan to make progress toward a complete
    /// baseline.
    #[serde(default, skip_serializing_if = "is_false")]
    pub partial: bool,

    /// MLP2-036: when [`Self::partial`] is `true`, the next
    /// file-path the resume scan should pick up at (lexicographic
    /// order — files before this cursor are guaranteed to have
    /// been scanned in the current `findings` list). `None` when
    /// the baseline is complete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,

    /// MLP2-065: fingerprint of the *pre-cursor* file list at the
    /// time this partial baseline was saved. Hex SHA-256 over the
    /// canonical-form relative paths (forward-slash, sorted, one
    /// per NDJSON-style `\n`-terminated line) of every scannable
    /// file lexicographically `< continuation`. The resume path
    /// recomputes the same hash against the current tree state and
    /// refuses to skip pre-cursor files (forcing a restart) when
    /// the hashes diverge — pre-fix a new file inserted before the
    /// cursor between resume passes was silently skipped, letting
    /// the baseline be marked complete without ever scanning the
    /// inserted file. `None` when the baseline is complete or
    /// produced by a pre-MLP2-065 anvil; consumers MUST treat
    /// `None` on a partial baseline as "drift-detection unavailable
    /// — restart to be safe" (the resume CLI does so today).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_cursor_fingerprint: Option<String>,
}

/// `serde(skip_serializing_if = "is_false")` helper so the
/// `partial` flag stays out of the on-disk JSON when the baseline
/// is complete. Lets older anvil reads of a complete baseline
/// behave byte-identically to pre-MLP2-036.
///
/// `&bool` (not `bool`) is mandated by serde's
/// `skip_serializing_if` contract: the predicate receives a
/// reference to the field. The clippy
/// `trivially_copy_pass_by_ref` lint flags it as a style issue;
/// suppress here because the alternative would be a wrapper struct
/// or a closure, both heavier than the lint warning.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
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
    /// MLP2-036: `partial=true` without a `continuation` cursor —
    /// the resume path has nowhere to pick up. Indicates a half-
    /// written baseline or a hand-edited one.
    #[error("baseline.json is partial but missing `continuation` cursor")]
    PartialMissingContinuation,
    /// MLP2-036: `continuation` set on a non-partial baseline. Would
    /// silently restart the next `--refresh` mid-tree; refuse at the
    /// boundary.
    #[error("baseline.json has `continuation` set but is not marked partial")]
    ContinuationOnCompleteBaseline,
}

impl Baseline {
    /// Build a fresh baseline at adoption time.
    pub fn new(metadata: BaselineMetadata, findings: Vec<BaselineFinding>) -> Self {
        let mut b = Self {
            format_version: FORMAT_VERSION,
            metadata,
            cutoff_commit: None,
            findings,
            partial: false,
            continuation: None,
            pre_cursor_fingerprint: None,
        };
        b.canonicalise();
        b
    }

    /// MLP2-036: merge a continuation-scan's findings into this
    /// baseline. Deduplicates against existing findings on the
    /// `(rule_id, file_path, fingerprint)` triple so repeated runs
    /// of the resume path can never inflate the list. Re-runs
    /// `canonicalise` so the on-disk shape stays deterministic.
    ///
    /// Caller is responsible for updating [`Self::partial`] /
    /// [`Self::continuation`] — this method only extends the
    /// findings vector.
    pub fn merge_partial_findings(&mut self, additional: Vec<BaselineFinding>) {
        self.findings.extend(additional);
        self.canonicalise();
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
        // Refuse `format_version == 0` as well as `> FORMAT_VERSION`.
        // Zero isn't a published shape — accepting it would mean any
        // baseline produced before the format_version field existed
        // (or by a defaulting bug) silently loads as v1, hiding a
        // real corruption signal. Today the only valid value is 1.
        if self.format_version == 0 || self.format_version > FORMAT_VERSION {
            return Err(FormatError::UnsupportedFormat {
                got: self.format_version,
                supported: FORMAT_VERSION,
            });
        }
        // MLP2-036: a partial baseline MUST carry a continuation
        // cursor — without it, the resume path has nowhere to pick
        // up. The inverse (continuation set, partial=false) is also
        // refused because a complete baseline shouldn't claim a
        // resume cursor; that would mislead `--refresh` into
        // restarting mid-tree on the next run.
        match (self.partial, self.continuation.as_ref()) {
            (true, None) => return Err(FormatError::PartialMissingContinuation),
            (false, Some(_)) => return Err(FormatError::ContinuationOnCompleteBaseline),
            _ => {}
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
        // MLP2-036: only emit `continuation` when present so a
        // complete baseline's bytes stay byte-identical to
        // pre-MLP2-036 output (older anvil reads + diff tools both
        // see the same shape).
        if let Some(c) = &self.continuation {
            map.insert("continuation", Value::String(c.clone()));
        }
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
        // MLP2-036: same skip-when-default reasoning as continuation.
        if self.partial {
            map.insert("partial", Value::Bool(true));
        }
        // MLP2-065: drift fingerprint travels alongside the cursor.
        // Skip-when-None preserves the byte-exact pre-MLP2-065 shape
        // for complete baselines (the field is irrelevant there).
        if let Some(fp) = &self.pre_cursor_fingerprint {
            map.insert("pre_cursor_fingerprint", Value::String(fp.clone()));
        }
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
    fn validate_rejects_zero_format_version() {
        // `format_version: 0` is not a published shape. Accepting it
        // would hide either a defaulting bug or a pre-versioned file.
        let mut b = Baseline::new(metadata(), vec![]);
        b.format_version = 0;
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

    // ── MLP2-036: partial / continuation schema ──────────────────

    #[test]
    fn complete_baseline_omits_partial_and_continuation_keys() {
        // Wire-shape stability: a baseline produced by the default
        // path must serialise to bytes that look identical to
        // pre-MLP2-036 (no `partial` key, no `continuation` key)
        // so older anvil reads keep working byte-for-byte.
        let b = Baseline::new(metadata(), vec![finding("rule-a", "a.rs", "x")]);
        let bytes = b.to_canonical_bytes().unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(
            !text.contains("\"partial\""),
            "complete baseline must not emit `partial` key; got {text}"
        );
        assert!(
            !text.contains("\"continuation\""),
            "complete baseline must not emit `continuation` key; got {text}"
        );
    }

    #[test]
    fn partial_baseline_round_trips_through_canonical_bytes() {
        let mut b = Baseline::new(metadata(), vec![finding("rule-a", "a.rs", "x")]);
        b.partial = true;
        b.continuation = Some("src/middle.rs".to_string());
        let bytes = b.to_canonical_bytes().unwrap();
        let parsed = Baseline::from_bytes(&bytes).unwrap();
        assert!(parsed.partial);
        assert_eq!(parsed.continuation.as_deref(), Some("src/middle.rs"));
        // Sanity-check the on-disk shape includes the new keys when
        // they're set.
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"partial\":true"));
        assert!(text.contains("\"continuation\":\"src/middle.rs\""));
    }

    #[test]
    fn validate_refuses_partial_without_continuation() {
        let mut b = Baseline::new(metadata(), vec![finding("rule-a", "a.rs", "x")]);
        b.partial = true;
        // continuation deliberately left None
        let bytes = b.to_canonical_bytes().unwrap();
        let err = Baseline::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, FormatError::PartialMissingContinuation));
    }

    #[test]
    fn validate_refuses_continuation_on_complete_baseline() {
        let mut b = Baseline::new(metadata(), vec![finding("rule-a", "a.rs", "x")]);
        b.continuation = Some("src/x.rs".to_string());
        // partial deliberately left false
        let bytes = b.to_canonical_bytes().unwrap();
        let err = Baseline::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, FormatError::ContinuationOnCompleteBaseline));
    }

    #[test]
    fn merge_partial_findings_dedupes_against_existing() {
        // Resume-scan idempotence: re-applying the same findings
        // must not inflate the list. Critical for the v1 resume
        // path, which sorts files lexicographically and continues
        // from the cursor — if the cursor file ever gets re-scanned
        // (operator manually re-runs without bumping cursor), the
        // dedup keeps the baseline byte-stable.
        let mut b = Baseline::new(
            metadata(),
            vec![
                finding("rule-a", "a.rs", "snippet-1"),
                finding("rule-a", "b.rs", "snippet-2"),
            ],
        );
        let additional = vec![
            finding("rule-a", "b.rs", "snippet-2"), // duplicate
            finding("rule-a", "c.rs", "snippet-3"), // new
        ];
        b.merge_partial_findings(additional);
        assert_eq!(
            b.findings.len(),
            3,
            "duplicate finding must collapse; got {:?}",
            b.findings
        );
        let files: Vec<_> = b.findings.iter().map(|f| f.file_path.as_str()).collect();
        assert_eq!(files, vec!["a.rs", "b.rs", "c.rs"]);
    }

    #[test]
    fn merge_partial_findings_preserves_canonical_sort_order() {
        let mut b = Baseline::new(metadata(), vec![finding("rule-z", "z.rs", "snippet-z")]);
        b.merge_partial_findings(vec![
            finding("rule-a", "a.rs", "snippet-a"),
            finding("rule-m", "m.rs", "snippet-m"),
        ]);
        let rules: Vec<_> = b.findings.iter().map(|f| f.rule_id.as_str()).collect();
        assert_eq!(
            rules,
            vec!["rule-a", "rule-m", "rule-z"],
            "merge must re-canonicalise so on-disk shape stays stable"
        );
    }

    #[test]
    fn validate_rejects_uppercase_fingerprint() {
        // 16-char uppercase hex is the length-passing cousin of the
        // existing "BAD" fixture. Dropping the `!is_ascii_uppercase`
        // conjunct would accept this and let mixed-case fingerprints
        // fork the identity space.
        let mut b = Baseline::new(metadata(), vec![]);
        b.findings.push(BaselineFinding {
            rule_id: "rule-a".to_string(),
            file_path: "x.rs".to_string(),
            fingerprint: "0123456789ABCDEF".to_string(),
        });
        let err = b.validate().unwrap_err();
        assert!(matches!(err, FormatError::MalformedFingerprint { .. }));
    }

    #[test]
    fn validate_rejects_fingerprint_length_other_than_sixteen() {
        for raw in ["0123456789abcde", "0123456789abcdef0"] {
            let mut b = Baseline::new(metadata(), vec![]);
            b.findings.push(BaselineFinding {
                rule_id: "rule-a".to_string(),
                file_path: "x.rs".to_string(),
                fingerprint: raw.to_string(),
            });
            let err = b.validate().unwrap_err();
            assert!(
                matches!(err, FormatError::MalformedFingerprint { .. }),
                "expected malformed fingerprint for {raw:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn validate_rejects_empty_rule_id() {
        let mut b = Baseline::new(metadata(), vec![]);
        b.findings.push(BaselineFinding {
            rule_id: String::new(),
            file_path: "x.rs".to_string(),
            fingerprint: "0".repeat(16),
        });
        let err = b.validate().unwrap_err();
        assert!(matches!(err, FormatError::InvalidRuleId { .. }));
    }

    #[test]
    fn validate_accepts_current_format_version() {
        let b = Baseline::new(metadata(), vec![finding("rule-a", "a.rs", "x")]);
        assert_eq!(b.format_version, FORMAT_VERSION);
        b.validate().unwrap();
    }

    #[test]
    fn from_bytes_rejects_malformed_json() {
        let err = Baseline::from_bytes(b"{not json").unwrap_err();
        assert!(matches!(err, FormatError::Json(_)));
    }

    #[test]
    fn from_bytes_accepts_json_without_trailing_newline() {
        let original = Baseline::new(metadata(), vec![finding("rule-a", "a.rs", "x")]);
        let bytes = original.to_canonical_bytes().unwrap();
        let without_nl = bytes
            .strip_suffix(b"\n")
            .expect("canonical bytes end in newline");
        let parsed = Baseline::from_bytes(without_nl).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn diff_identity_scan_is_clean_and_preserves_entries() {
        let f1 = finding("rule-a", "a.rs", "snippet-1");
        let f2 = finding("rule-b", "b.rs", "snippet-2");
        let recorded = Baseline::new(metadata(), vec![f1.clone(), f2.clone()]);
        let diff = recorded.diff(&[f2.clone(), f1.clone()]);
        assert!(diff.is_clean());
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert_eq!(diff.unchanged.len(), 2);
        assert_eq!(diff.unchanged[0].rule_id, "rule-a");
        assert_eq!(diff.unchanged[1].rule_id, "rule-b");
        assert_eq!(diff.unchanged[0].fingerprint, f1.fingerprint);
    }

    #[test]
    fn diff_treats_same_path_and_rule_with_different_fingerprint_as_distinct() {
        // A snippet edit is a remove+add, not an in-place keep.
        // Identity that dropped fingerprint would treat this as
        // unchanged and a gate would miss the new finding.
        let recorded = Baseline::new(
            metadata(),
            vec![finding("rule-a", "src/lib.rs", "let x = 1;")],
        );
        let new_scan = vec![finding("rule-a", "src/lib.rs", "let x = 2;")];
        let diff = recorded.diff(&new_scan);
        assert!(!diff.is_clean());
        assert!(diff.unchanged.is_empty());
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 1);
        assert_ne!(diff.added[0].fingerprint, diff.removed[0].fingerprint);
    }

    #[test]
    fn diff_treats_same_path_and_fingerprint_with_different_rule_as_distinct() {
        let snippet = "let x = 1;";
        let recorded = Baseline::new(metadata(), vec![finding("rule-a", "src/lib.rs", snippet)]);
        let new_scan = vec![finding("rule-b", "src/lib.rs", snippet)];
        let diff = recorded.diff(&new_scan);
        assert_eq!(diff.unchanged.len(), 0);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].rule_id, "rule-a");
        assert_eq!(diff.added[0].rule_id, "rule-b");
    }

    #[test]
    fn diff_empty_scan_marks_every_recorded_finding_removed() {
        let recorded = Baseline::new(
            metadata(),
            vec![
                finding("rule-a", "a.rs", "s1"),
                finding("rule-b", "b.rs", "s2"),
            ],
        );
        let diff = recorded.diff(&[]);
        assert!(!diff.is_clean());
        assert!(diff.unchanged.is_empty());
        assert!(diff.added.is_empty());
        assert_eq!(diff.removed.len(), 2);
        assert_eq!(diff.removed[0].rule_id, "rule-a");
        assert_eq!(diff.removed[1].rule_id, "rule-b");
    }

    #[test]
    fn diff_against_empty_recorded_marks_every_scan_finding_added() {
        let recorded = Baseline::new(metadata(), vec![]);
        let new_scan = vec![
            finding("rule-z", "z.rs", "s-z"),
            finding("rule-a", "a.rs", "s-a"),
        ];
        let diff = recorded.diff(&new_scan);
        assert!(!diff.is_clean());
        assert!(diff.unchanged.is_empty());
        assert!(diff.removed.is_empty());
        assert_eq!(diff.added.len(), 2);
        // Output is sorted by (rule_id, file_path, fingerprint)
        // regardless of scan order.
        assert_eq!(diff.added[0].rule_id, "rule-a");
        assert_eq!(diff.added[1].rule_id, "rule-z");
    }

    #[test]
    fn pre_cursor_fingerprint_round_trips_on_partial_baseline() {
        let mut b = Baseline::new(metadata(), vec![finding("rule-a", "a.rs", "x")]);
        b.partial = true;
        b.continuation = Some("src/middle.rs".to_string());
        b.pre_cursor_fingerprint = Some("ab".repeat(32));
        let bytes = b.to_canonical_bytes().unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(
            text.contains("\"pre_cursor_fingerprint\""),
            "partial baseline with a cursor hash must emit the key; got {text}"
        );
        let parsed = Baseline::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.pre_cursor_fingerprint, b.pre_cursor_fingerprint);
        assert!(parsed.partial);
        assert_eq!(parsed.continuation.as_deref(), Some("src/middle.rs"));
    }

    #[test]
    fn complete_baseline_omits_pre_cursor_fingerprint_key() {
        let b = Baseline::new(metadata(), vec![finding("rule-a", "a.rs", "x")]);
        let text = String::from_utf8(b.to_canonical_bytes().unwrap()).unwrap();
        assert!(
            !text.contains("\"pre_cursor_fingerprint\""),
            "complete baseline must not emit pre_cursor_fingerprint; got {text}"
        );
    }
}
