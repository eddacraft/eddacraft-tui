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

// ── MLP2-035: adversarial-refresh detection ─────────────────────────

/// Wire-level surface marker emitted when [`analyze_refresh`] returns
/// [`RefreshSuspicion::Suspicious`]. Mirrors the
/// `degraded:identity-mismatch` constant pattern from MLP2-003 — the
/// string is the contract; rename it and downstream renderers (status
/// JSON, fence-state degraded marker — Phase 2) silently miss it.
pub const DEGRADED_REASON: &str = "degraded:baseline-suspicious";

/// Heuristic thresholds for [`analyze_refresh`].
///
/// Two knobs together avoid the most common false-positive shapes:
///
/// - `removed_ratio_threshold` (default 0.75) — a refresh that drops
///   ≥75% of the prior baseline's findings is flagged. Tunable down
///   for high-assurance branches that want every drop questioned, or
///   up for codebases that legitimately churn large fractions of
///   their finding set per release.
/// - `minimum_removed` (default 10) — a baseline of 8 findings going
///   to 0 will hit the ratio (1.0) but is almost certainly a small
///   project legitimately resolving its violations. The minimum
///   prevents the alert from firing on tiny baselines where the
///   ratio is statistically meaningless.
///
/// `Confidence: low (needs threshold tuning)` per the MLP2-035 spec
/// — these defaults are conservative starting points; the operator
/// can override via `anvil baseline --suspicion-ratio` /
/// `--suspicion-min-removed` once they have a feel for their
/// project's baseline cadence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SuspicionThresholds {
    pub removed_ratio_threshold: f64,
    pub minimum_removed: usize,
}

impl SuspicionThresholds {
    pub const DEFAULT_REMOVED_RATIO: f64 = 0.75;
    pub const DEFAULT_MINIMUM_REMOVED: usize = 10;
}

impl Default for SuspicionThresholds {
    fn default() -> Self {
        Self {
            removed_ratio_threshold: Self::DEFAULT_REMOVED_RATIO,
            minimum_removed: Self::DEFAULT_MINIMUM_REMOVED,
        }
    }
}

/// Outcome of comparing the pre-refresh baseline finding set against
/// the post-refresh one. The simplest possible heuristic surface for
/// v1: `Clean` for any drop the operator can defensibly explain by
/// ordinary work, `Suspicious` when the drop crosses both the ratio
/// and the absolute-minimum gates.
///
/// Code-churn correlation (the spec's "without a corresponding
/// code-size reduction" axis) is **not** measured in v1 — that
/// requires recording either the commit SHA or the total scanned
/// `LoC` at refresh time, neither of which the on-disk baseline
/// schema carries today. Filed as Phase 2 follow-up; the threshold
/// knobs here are the v1 operator escape hatch in the meantime.
#[derive(Debug, Clone, PartialEq)]
pub enum RefreshSuspicion {
    /// The drop fell below at least one of the two gates
    /// (`removed_ratio_threshold` or `minimum_removed`), or there
    /// were no removals at all.
    Clean,
    /// Both gates were crossed. Caller surfaces this as
    /// [`DEGRADED_REASON`] (`degraded:baseline-suspicious`).
    Suspicious {
        removed_count: usize,
        old_total: usize,
        removed_ratio: f64,
        threshold: SuspicionThresholds,
    },
}

/// Compare the pre-refresh and post-refresh finding sets and return
/// [`RefreshSuspicion`].
///
/// Inputs are slices of [`BaselineFinding`] rather than full
/// [`crate::Baseline`] objects so the caller can pre-strip whatever
/// fields aren't relevant to the comparison (e.g. compare under a
/// rule allowlist). Pure decision over two finding sets — no I/O.
pub fn analyze_refresh(
    old: &[BaselineFinding],
    new: &[BaselineFinding],
    thresholds: &SuspicionThresholds,
) -> RefreshSuspicion {
    use std::collections::HashSet;

    let old_total = old.len();
    if old_total == 0 {
        // First-create or empty-baseline → nothing to drop, nothing
        // to be suspicious about. Caller shouldn't typically call
        // analyze_refresh on a first-create path, but the boundary
        // check costs nothing and avoids div-by-zero on the ratio.
        return RefreshSuspicion::Clean;
    }

    // Set membership keyed on the same triple `BaselineDiff` uses to
    // partition. If a finding's `(rule_id, file_path, fingerprint)`
    // appears in `old` but not `new`, it's a removal.
    let new_keys: HashSet<(&str, &str, &str)> = new
        .iter()
        .map(|f| {
            (
                f.rule_id.as_str(),
                f.file_path.as_str(),
                f.fingerprint.as_str(),
            )
        })
        .collect();
    let removed_count = old
        .iter()
        .filter(|f| {
            !new_keys.contains(&(
                f.rule_id.as_str(),
                f.file_path.as_str(),
                f.fingerprint.as_str(),
            ))
        })
        .count();

    if removed_count < thresholds.minimum_removed {
        return RefreshSuspicion::Clean;
    }

    // `old_total > 0` guaranteed above.
    #[allow(clippy::cast_precision_loss)]
    let removed_ratio = removed_count as f64 / old_total as f64;
    if removed_ratio < thresholds.removed_ratio_threshold {
        return RefreshSuspicion::Clean;
    }

    RefreshSuspicion::Suspicious {
        removed_count,
        old_total,
        removed_ratio,
        threshold: *thresholds,
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
    fn diff_is_not_clean_when_both_added_and_removed() {
        let d = BaselineDiff {
            unchanged: vec![],
            added: vec![entry("a", "x", "0".repeat(16).as_str())],
            removed: vec![entry("b", "y", "1".repeat(16).as_str())],
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

    // ── MLP2-035: analyze_refresh ───────────────────────────────────

    fn finding(rule: &str, file: &str, fp_seed: u8) -> BaselineFinding {
        BaselineFinding {
            rule_id: rule.to_string(),
            file_path: file.to_string(),
            // Each finding gets a distinct 16-hex digest so set
            // membership keys never accidentally collide.
            fingerprint: format!("{fp_seed:02x}{:0>14}", "0"),
        }
    }

    fn many_findings(n: usize) -> Vec<BaselineFinding> {
        (0..n)
            .map(|i| {
                #[allow(clippy::cast_possible_truncation)]
                finding("rule-a", &format!("src/file_{i}.rs"), (i % 256) as u8)
            })
            .collect()
    }

    #[test]
    fn degraded_reason_constant_is_pinned() {
        // The wire-level marker is contract — downstream renderers
        // (Phase 2 fence wiring + status JSON) match on this exact
        // string.
        assert_eq!(DEGRADED_REASON, "degraded:baseline-suspicious");
    }

    #[test]
    fn default_thresholds_match_documented_values() {
        let t = SuspicionThresholds::default();
        assert!((t.removed_ratio_threshold - 0.75).abs() < f64::EPSILON);
        assert_eq!(t.minimum_removed, 10);
    }

    #[test]
    fn analyze_refresh_clean_when_old_is_empty() {
        // First-create boundary: nothing to drop.
        let new = many_findings(50);
        let r = analyze_refresh(&[], &new, &SuspicionThresholds::default());
        assert_eq!(r, RefreshSuspicion::Clean);
    }

    #[test]
    fn analyze_refresh_clean_when_no_removals() {
        let same = many_findings(50);
        let r = analyze_refresh(&same, &same, &SuspicionThresholds::default());
        assert_eq!(r, RefreshSuspicion::Clean);
    }

    #[test]
    fn analyze_refresh_clean_when_drop_below_minimum_removed() {
        // 8 of 12 dropped → 0.66 ratio (below 0.75) AND 8 removed
        // (below 10 minimum). Both gates favour Clean; this
        // exercises the absolute-minimum guard explicitly: even at
        // ratio = 1.0, a baseline that started with 8 findings and
        // ended with 0 must not fire (small projects legitimately
        // resolve everything).
        let old = many_findings(8);
        let new: Vec<_> = vec![]; // 8 removed, ratio 1.0
        let r = analyze_refresh(&old, &new, &SuspicionThresholds::default());
        assert_eq!(
            r,
            RefreshSuspicion::Clean,
            "minimum_removed=10 must veto on a tiny baseline even at 100% drop"
        );
    }

    #[test]
    fn analyze_refresh_clean_when_drop_below_ratio_threshold() {
        // 50 → 25 = 50% drop. Below 75% threshold → Clean.
        let old = many_findings(50);
        let new = many_findings(50)[25..].to_vec();
        let r = analyze_refresh(&old, &new, &SuspicionThresholds::default());
        assert_eq!(r, RefreshSuspicion::Clean);
    }

    #[test]
    fn analyze_refresh_suspicious_when_both_gates_crossed() {
        // MLP2-035 validation fixture: 100 findings → 10 (90% drop,
        // 90 removed) clearly clears both gates.
        let old = many_findings(100);
        let new = many_findings(100)[..10].to_vec();
        let r = analyze_refresh(&old, &new, &SuspicionThresholds::default());
        match r {
            RefreshSuspicion::Suspicious {
                removed_count,
                old_total,
                removed_ratio,
                threshold,
            } => {
                assert_eq!(removed_count, 90);
                assert_eq!(old_total, 100);
                assert!((removed_ratio - 0.9).abs() < f64::EPSILON);
                assert_eq!(threshold, SuspicionThresholds::default());
            }
            other @ RefreshSuspicion::Clean => panic!("expected Suspicious, got {other:?}"),
        }
    }

    #[test]
    fn analyze_refresh_honours_overridden_thresholds() {
        // Operator escape hatch: a noisier project sets the ratio
        // higher (0.95) so a 90% drop is no longer flagged.
        let old = many_findings(100);
        let new = many_findings(100)[..10].to_vec();
        let strict = SuspicionThresholds {
            removed_ratio_threshold: 0.95,
            minimum_removed: 10,
        };
        assert_eq!(
            analyze_refresh(&old, &new, &strict),
            RefreshSuspicion::Clean
        );

        // Going the other way: a stricter operator sets ratio 0.5,
        // so a 50% drop now fires.
        let old = many_findings(100);
        let new = many_findings(100)[..50].to_vec();
        let strict = SuspicionThresholds {
            removed_ratio_threshold: 0.5,
            minimum_removed: 10,
        };
        assert!(matches!(
            analyze_refresh(&old, &new, &strict),
            RefreshSuspicion::Suspicious { .. }
        ));
    }

    #[test]
    fn analyze_refresh_set_membership_uses_full_triple() {
        // Two findings sharing rule_id + file_path but distinct
        // fingerprints are independent. Removing one of them
        // shouldn't masquerade as keeping it.
        //
        // Old has 12 findings (just over the minimum_removed=10
        // gate); new keeps exactly one src/lib.rs entry → 11
        // removed, ratio 11/12 ≈ 0.917, both gates cross.
        let old = vec![
            finding("rule-a", "src/lib.rs", 0x01),
            finding("rule-a", "src/lib.rs", 0x02),
            finding("rule-a", "src/a.rs", 0x03),
            finding("rule-a", "src/b.rs", 0x04),
            finding("rule-a", "src/c.rs", 0x05),
            finding("rule-a", "src/d.rs", 0x06),
            finding("rule-a", "src/e.rs", 0x07),
            finding("rule-a", "src/f.rs", 0x08),
            finding("rule-a", "src/g.rs", 0x09),
            finding("rule-a", "src/h.rs", 0x0a),
            finding("rule-a", "src/i.rs", 0x0b),
            finding("rule-a", "src/j.rs", 0x0c),
        ];
        let new = vec![finding("rule-a", "src/lib.rs", 0x01)];
        match analyze_refresh(&old, &new, &SuspicionThresholds::default()) {
            RefreshSuspicion::Suspicious { removed_count, .. } => {
                assert_eq!(
                    removed_count, 11,
                    "fingerprint must distinguish findings sharing (rule_id, file_path)"
                );
            }
            other @ RefreshSuspicion::Clean => panic!("expected Suspicious, got {other:?}"),
        }
    }

    #[test]
    fn analyze_refresh_suspicious_at_exact_minimum_removed() {
        // `removed_count < minimum_removed` (not `<=`): 10 of 10
        // with default min=10 and ratio 1.0 must fire. A `<=`
        // mutation would let a full wipe of a 10-finding baseline
        // through as Clean.
        let old = many_findings(10);
        let r = analyze_refresh(&old, &[], &SuspicionThresholds::default());
        match r {
            RefreshSuspicion::Suspicious {
                removed_count,
                old_total,
                removed_ratio,
                ..
            } => {
                assert_eq!(removed_count, 10);
                assert_eq!(old_total, 10);
                assert!((removed_ratio - 1.0).abs() < f64::EPSILON);
            }
            other @ RefreshSuspicion::Clean => {
                panic!("10-of-10 drop must be Suspicious at the default minimum, got {other:?}")
            }
        }
    }

    #[test]
    fn analyze_refresh_clean_just_below_minimum_removed() {
        // 9 of 9 is a 100% drop but below default minimum_removed=10.
        let old = many_findings(9);
        let r = analyze_refresh(&old, &[], &SuspicionThresholds::default());
        assert_eq!(r, RefreshSuspicion::Clean);
    }

    #[test]
    fn analyze_refresh_suspicious_at_exact_ratio_threshold() {
        // 15 of 20 = 0.75, removed_count=15 >= 10. Docs say ≥75%
        // is flagged; `<` (not `<=`) on the ratio gate is what
        // implements that. A `<=` mutation would miss an exact-75%
        // wipe.
        let old = many_findings(20);
        let new = many_findings(20)[..5].to_vec();
        let r = analyze_refresh(&old, &new, &SuspicionThresholds::default());
        match r {
            RefreshSuspicion::Suspicious {
                removed_count,
                old_total,
                removed_ratio,
                ..
            } => {
                assert_eq!(removed_count, 15);
                assert_eq!(old_total, 20);
                assert!((removed_ratio - 0.75).abs() < f64::EPSILON);
            }
            other @ RefreshSuspicion::Clean => {
                panic!("exact 75% drop must be Suspicious, got {other:?}")
            }
        }
    }

    #[test]
    fn analyze_refresh_clean_just_below_ratio_threshold() {
        // 14 of 20 = 0.70, removed_count=14 >= 10, but below 0.75.
        let old = many_findings(20);
        let new = many_findings(20)[..6].to_vec();
        let r = analyze_refresh(&old, &new, &SuspicionThresholds::default());
        assert_eq!(r, RefreshSuspicion::Clean);
    }

    #[test]
    fn analyze_refresh_empty_old_is_clean_even_with_zero_thresholds() {
        // First-create must not divide by zero or fire just because
        // the operator set both gates to zero. The empty-old guard
        // is the contract, not an incidental skip of the min-removed
        // check.
        let new = many_findings(5);
        let t = SuspicionThresholds {
            removed_ratio_threshold: 0.0,
            minimum_removed: 0,
        };
        assert_eq!(analyze_refresh(&[], &new, &t), RefreshSuspicion::Clean);
    }

    #[test]
    fn analyze_refresh_additions_are_not_counted_as_removals() {
        // A refresh that only adds findings must stay Clean even
        // when the added set is large. The heuristic is a drop
        // detector, not a churn detector.
        let old = many_findings(20);
        let mut new = many_findings(20);
        new.extend(many_findings(80)[20..].iter().cloned());
        let r = analyze_refresh(&old, &new, &SuspicionThresholds::default());
        assert_eq!(r, RefreshSuspicion::Clean);
    }
}
