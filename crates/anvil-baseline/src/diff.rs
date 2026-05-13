use crate::finding::BaselineFinding;

/// One entry in a [`BaselineDiff`]. Identical shape to
/// [`BaselineFinding`] but distinct type so the diff API can evolve
/// (e.g. carry a `reason` field) without touching the on-disk
/// schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineDiffEntry {
    pub rule_id: String,
    pub file_path: String,
    pub fingerprint: String,
}

impl From<BaselineFinding> for BaselineDiffEntry {
    fn from(f: BaselineFinding) -> Self {
        Self {
            rule_id: f.rule_id,
            file_path: f.file_path,
            fingerprint: f.fingerprint,
        }
    }
}

impl BaselineDiffEntry {
    pub(crate) fn sort_key(&self) -> (&str, &str, &str) {
        (&self.rule_id, &self.file_path, &self.fingerprint)
    }
}

/// Result of comparing a recorded [`crate::Baseline`] to a fresh
/// scan. Three deterministic partitions:
///
/// - `unchanged` — present in both
/// - `added` — in the new scan but not the recorded baseline
///   (these are the findings a gate should escalate on)
/// - `removed` — in the recorded baseline but not the new scan
///   (these are the findings that got resolved between adoption and
///   now)
///
/// Each list is sorted by `(rule_id, file_path, fingerprint)`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BaselineDiff {
    pub unchanged: Vec<BaselineDiffEntry>,
    pub added: Vec<BaselineDiffEntry>,
    pub removed: Vec<BaselineDiffEntry>,
}

impl BaselineDiff {
    /// True when the new scan produced no additions and no removals.
    pub fn is_clean(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rule: &str, file: &str, fp: &str) -> BaselineDiffEntry {
        BaselineDiffEntry {
            rule_id: rule.to_string(),
            file_path: file.to_string(),
            fingerprint: fp.to_string(),
        }
    }

    #[test]
    fn diff_is_clean_when_added_and_removed_empty() {
        let d = BaselineDiff {
            unchanged: vec![entry("a", "x", "0".repeat(16).as_str())],
            added: vec![],
            removed: vec![],
        };
        assert!(d.is_clean());
    }

    #[test]
    fn diff_is_not_clean_when_added_present() {
        let d = BaselineDiff {
            unchanged: vec![],
            added: vec![entry("a", "x", "0".repeat(16).as_str())],
            removed: vec![],
        };
        assert!(!d.is_clean());
    }

    #[test]
    fn diff_is_not_clean_when_removed_present() {
        let d = BaselineDiff {
            unchanged: vec![],
            added: vec![],
            removed: vec![entry("a", "x", "0".repeat(16).as_str())],
        };
        assert!(!d.is_clean());
    }

    #[test]
    fn from_baseline_finding_preserves_fields() {
        let f = BaselineFinding {
            rule_id: "rule-a".to_string(),
            file_path: "src/lib.rs".to_string(),
            fingerprint: "0".repeat(16),
        };
        let e: BaselineDiffEntry = f.clone().into();
        assert_eq!(e.rule_id, f.rule_id);
        assert_eq!(e.file_path, f.file_path);
        assert_eq!(e.fingerprint, f.fingerprint);
    }
}
