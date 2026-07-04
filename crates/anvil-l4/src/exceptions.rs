//! EXCEPT-006: exception application semantics for L3/L4 gate
//! verdicts.
//!
//! ADR-073 moved policy exceptions into the tracked
//! `anvil/exceptions/` store; EXCEPT-005 gave each grant a validity
//! verdict (active / unattributed / expired / revoked /
//! invalid-scope). This module owns what an *applying* exception
//! **means** for a gate verdict — the piece the brainstorm's
//! enforcement algorithm (git-native-governance `solution.md` §8.3)
//! calls "apply only to the matching finding instance":
//!
//! - A clean grant suppresses its matching diagnostic entirely.
//! - An **unattributed** grant (v0 shape, no `owner`/`created_by`)
//!   applies but is never silently honoured (ADR-073): the
//!   diagnostic stays visible, downgraded to [`Severity::Warn`] and
//!   annotated with the exception id, so the branch rule's `on_warn`
//!   routing decides admission and the operator sees why.
//! - Anything else (expired / revoked / invalid scope / no matching
//!   grant) leaves the diagnostic untouched — the finding stands.
//!
//! Matching and validity classification stay with the caller: this
//! crate deliberately depends only on `anvil-config` (see the crate
//! docs), so it cannot — and must not — read the exception store or
//! run glob matching. The caller (the CLI's `CommitAntipatternEngine`)
//! verifies each grant with `anvil-policy`'s `verify_exception_at`,
//! matches scope with `PolicyException::covers_finding`, and hands
//! this module one [`ExceptionDisposition`] per diagnostic,
//! positionally aligned — the same positional-correlation contract
//! [`validate_range`](crate::validate_range) uses for verdicts.
//!
//! Fail-safe direction: any ambiguity (a misaligned disposition
//! slice) applies **no** exceptions, so findings stand and the gate
//! blocks rather than silently admitting.

use crate::validate::{Severity, ValidationDiagnostic, ValidationVerdict};

/// Per-diagnostic exception coverage, computed by the caller from the
/// tracked exception store. Positionally aligned with the diagnostic
/// slice handed to [`apply_exception_dispositions`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExceptionDisposition {
    /// No valid exception covers this diagnostic — it stands.
    NotCovered,
    /// A valid, attributed exception covers this diagnostic: suppress
    /// it. `exception_id` is the tracked grant's stable id, recorded
    /// so exception use is attributable.
    Suppressed { exception_id: String },
    /// A valid but **unattributed** exception covers this diagnostic
    /// (`ExceptionVerdict::Unattributed`, the ADR-073 downgrade
    /// signal): keep the diagnostic, downgrade it to
    /// [`Severity::Warn`], and annotate it with the exception id so
    /// it is never silently honoured.
    SuppressedDowngraded { exception_id: String },
}

/// One exercised exception — the "record exception use" half of
/// EXCEPT-006. The caller emits these to its telemetry/evidence
/// surface; witness-envelope and capsule inclusion build on the same
/// record (EXCEPT-009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedException {
    /// Stable id of the tracked grant that applied.
    pub exception_id: String,
    /// Rule id of the diagnostic the grant applied to.
    pub rule_id: String,
    /// True when the grant was unattributed and the diagnostic was
    /// downgraded rather than suppressed.
    pub downgraded: bool,
}

/// Result of applying exception dispositions to a diagnostic set:
/// the recomputed verdict plus the record of every exercised grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptionOutcome {
    /// [`ValidationVerdict::Allow`] when every diagnostic was
    /// suppressed; otherwise [`ValidationVerdict::Block`] carrying
    /// the surviving (possibly downgraded) diagnostics.
    pub verdict: ValidationVerdict,
    /// Every exception that applied, in diagnostic order.
    pub applied: Vec<AppliedException>,
}

/// Apply per-diagnostic exception dispositions and recompute the
/// verdict.
///
/// `dispositions` must be positionally aligned with `diagnostics`.
/// A length mismatch is a caller bug; it fails safe — **no**
/// exception applies and every finding stands — because silently
/// admitting on a bookkeeping error would invert the gate.
#[must_use]
pub fn apply_exception_dispositions(
    diagnostics: Vec<ValidationDiagnostic>,
    dispositions: &[ExceptionDisposition],
) -> ExceptionOutcome {
    if diagnostics.len() != dispositions.len() {
        return ExceptionOutcome {
            verdict: verdict_for(diagnostics),
            applied: Vec::new(),
        };
    }

    let mut kept = Vec::with_capacity(diagnostics.len());
    let mut applied = Vec::new();
    for (mut diagnostic, disposition) in diagnostics.into_iter().zip(dispositions) {
        match disposition {
            ExceptionDisposition::NotCovered => kept.push(diagnostic),
            ExceptionDisposition::Suppressed { exception_id } => {
                applied.push(AppliedException {
                    exception_id: exception_id.clone(),
                    rule_id: diagnostic.rule_id,
                    downgraded: false,
                });
            }
            ExceptionDisposition::SuppressedDowngraded { exception_id } => {
                applied.push(AppliedException {
                    exception_id: exception_id.clone(),
                    rule_id: diagnostic.rule_id.clone(),
                    downgraded: true,
                });
                diagnostic.severity = Severity::Warn;
                diagnostic.message = annotate_downgrade(&diagnostic.message, exception_id);
                kept.push(diagnostic);
            }
        }
    }
    ExceptionOutcome {
        verdict: verdict_for(kept),
        applied,
    }
}

/// Recompute the verdict for a surviving diagnostic set: empty →
/// `Allow`, anything left → `Block`.
fn verdict_for(diagnostics: Vec<ValidationDiagnostic>) -> ValidationVerdict {
    if diagnostics.is_empty() {
        ValidationVerdict::Allow
    } else {
        ValidationVerdict::Block { diagnostics }
    }
}

/// [`ValidationDiagnostic`] messages are ≤200 chars by contract.
const MESSAGE_CHAR_BUDGET: usize = 200;

/// Append the downgrade annotation, truncating the *base* message if
/// the combination would exceed the message contract — the annotation
/// (the "why is this Warn now" signal) is never cut.
fn annotate_downgrade(message: &str, exception_id: &str) -> String {
    let annotation = format!(" [unattributed exception {exception_id}: downgraded to warn]");
    let annotation_len = annotation.chars().count();
    let base_budget = MESSAGE_CHAR_BUDGET.saturating_sub(annotation_len);
    let base_len = message.chars().count();
    if base_len <= base_budget {
        return format!("{message}{annotation}");
    }
    let truncated: String = message
        .chars()
        .take(base_budget.saturating_sub(3))
        .collect();
    format!("{truncated}...{annotation}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(rule_id: &str, severity: Severity, message: &str) -> ValidationDiagnostic {
        ValidationDiagnostic {
            rule_id: rule_id.to_string(),
            severity,
            message: message.to_string(),
        }
    }

    fn suppressed(id: &str) -> ExceptionDisposition {
        ExceptionDisposition::Suppressed {
            exception_id: id.to_string(),
        }
    }

    fn downgraded(id: &str) -> ExceptionDisposition {
        ExceptionDisposition::SuppressedDowngraded {
            exception_id: id.to_string(),
        }
    }

    /// A valid, attributed exception suppresses its diagnostic and the
    /// verdict recomputes to `Allow` — the core "gates suppress only
    /// matching, valid findings" outcome.
    #[test]
    fn active_exception_suppresses_diagnostic_and_allows() {
        let outcome = apply_exception_dispositions(
            vec![diag("AP-001", Severity::Block, "leak")],
            &[suppressed("exc-1")],
        );
        assert_eq!(outcome.verdict, ValidationVerdict::Allow);
        assert_eq!(
            outcome.applied,
            vec![AppliedException {
                exception_id: "exc-1".to_string(),
                rule_id: "AP-001".to_string(),
                downgraded: false,
            }],
        );
    }

    /// An unattributed grant is never silently honoured (ADR-073):
    /// the diagnostic survives at `Warn`, annotated with the grant id,
    /// and the applied record flags the downgrade.
    #[test]
    fn unattributed_exception_downgrades_to_warn_with_annotation() {
        let outcome = apply_exception_dispositions(
            vec![diag("AP-002", Severity::Block, "hardcoded secret")],
            &[downgraded("exc-2")],
        );
        let ValidationVerdict::Block { diagnostics } = &outcome.verdict else {
            panic!("expected Block, got {:?}", outcome.verdict);
        };
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warn);
        assert!(
            diagnostics[0].message.contains("exc-2"),
            "downgrade annotation must name the exception id: {}",
            diagnostics[0].message,
        );
        assert!(
            diagnostics[0].message.contains("unattributed"),
            "downgrade annotation must say why: {}",
            diagnostics[0].message,
        );
        assert_eq!(
            outcome.applied,
            vec![AppliedException {
                exception_id: "exc-2".to_string(),
                rule_id: "AP-002".to_string(),
                downgraded: true,
            }],
        );
    }

    /// A `Warn` diagnostic covered by an unattributed grant stays
    /// `Warn` — the downgrade never upgrades severity.
    #[test]
    fn downgrade_of_warn_diagnostic_stays_warn() {
        let outcome = apply_exception_dispositions(
            vec![diag("AP-003", Severity::Warn, "style")],
            &[downgraded("exc-3")],
        );
        let ValidationVerdict::Block { diagnostics } = &outcome.verdict else {
            panic!("expected Block, got {:?}", outcome.verdict);
        };
        assert_eq!(diagnostics[0].severity, Severity::Warn);
    }

    /// `NotCovered` leaves the diagnostic byte-for-byte untouched —
    /// expired / revoked / invalid-scope / non-matching grants never
    /// alter a finding.
    #[test]
    fn not_covered_leaves_diagnostic_untouched() {
        let input = vec![diag("AP-004", Severity::Block, "finding stands")];
        let outcome =
            apply_exception_dispositions(input.clone(), &[ExceptionDisposition::NotCovered]);
        assert_eq!(
            outcome.verdict,
            ValidationVerdict::Block { diagnostics: input },
        );
        assert!(outcome.applied.is_empty());
    }

    /// Mixed dispositions: only the covered diagnostics change, order
    /// is preserved, and the applied record carries one entry per
    /// exercised grant in diagnostic order.
    #[test]
    fn mixed_dispositions_apply_per_diagnostic() {
        let outcome = apply_exception_dispositions(
            vec![
                diag("AP-001", Severity::Block, "suppress me"),
                diag("AP-002", Severity::Block, "downgrade me"),
                diag("AP-003", Severity::Block, "keep me"),
            ],
            &[
                suppressed("exc-1"),
                downgraded("exc-2"),
                ExceptionDisposition::NotCovered,
            ],
        );
        let ValidationVerdict::Block { diagnostics } = &outcome.verdict else {
            panic!("expected Block, got {:?}", outcome.verdict);
        };
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].rule_id, "AP-002");
        assert_eq!(diagnostics[0].severity, Severity::Warn);
        assert_eq!(diagnostics[1].rule_id, "AP-003");
        assert_eq!(diagnostics[1].severity, Severity::Block);
        assert_eq!(diagnostics[1].message, "keep me");
        assert_eq!(outcome.applied.len(), 2);
        assert_eq!(outcome.applied[0].exception_id, "exc-1");
        assert!(!outcome.applied[0].downgraded);
        assert_eq!(outcome.applied[1].exception_id, "exc-2");
        assert!(outcome.applied[1].downgraded);
    }

    /// Every diagnostic suppressed → the verdict recomputes to
    /// `Allow`, not `Block { diagnostics: [] }`.
    #[test]
    fn all_suppressed_recomputes_allow() {
        let outcome = apply_exception_dispositions(
            vec![
                diag("AP-001", Severity::Block, "a"),
                diag("AP-002", Severity::Warn, "b"),
            ],
            &[suppressed("exc-1"), suppressed("exc-2")],
        );
        assert_eq!(outcome.verdict, ValidationVerdict::Allow);
        assert_eq!(outcome.applied.len(), 2);
    }

    /// A misaligned disposition slice is a caller bug and fails safe:
    /// no exception applies, every finding stands, nothing is
    /// recorded as exercised.
    #[test]
    fn length_mismatch_fails_safe_and_keeps_all_findings() {
        let input = vec![
            diag("AP-001", Severity::Block, "a"),
            diag("AP-002", Severity::Block, "b"),
        ];
        let outcome = apply_exception_dispositions(input.clone(), &[suppressed("exc-1")]);
        assert_eq!(
            outcome.verdict,
            ValidationVerdict::Block { diagnostics: input },
        );
        assert!(outcome.applied.is_empty());
    }

    /// No diagnostics → `Allow` (vacuous), regardless of dispositions.
    #[test]
    fn empty_diagnostics_yield_allow() {
        let outcome = apply_exception_dispositions(vec![], &[]);
        assert_eq!(outcome.verdict, ValidationVerdict::Allow);
        assert!(outcome.applied.is_empty());
    }

    /// The downgrade annotation respects the [`ValidationDiagnostic`]
    /// message contract (≤200 chars): a long base message is truncated
    /// to make room, the annotation itself is never cut.
    #[test]
    fn downgrade_annotation_keeps_message_within_contract() {
        let long = "x".repeat(300);
        let outcome = apply_exception_dispositions(
            vec![diag("AP-001", Severity::Block, &long)],
            &[downgraded("exc-long")],
        );
        let ValidationVerdict::Block { diagnostics } = &outcome.verdict else {
            panic!("expected Block, got {:?}", outcome.verdict);
        };
        let message = &diagnostics[0].message;
        assert!(
            message.chars().count() <= 200,
            "message must stay within the 200-char contract, got {}",
            message.chars().count(),
        );
        assert!(
            message.contains("exc-long"),
            "annotation must survive truncation: {message}",
        );
    }
}
