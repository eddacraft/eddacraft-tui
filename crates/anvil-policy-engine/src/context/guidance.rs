//! Assertion failure → remediation-first output (CPOL-003).
//!
//! When an assertion is [violated](AssertionEvaluation::Violated), this module
//! turns the [`Violation`] into an [`AssertionGuidance`]: a structured,
//! machine-readable explanation that leads with *how to fix it*.
//!
//! ## Alignment with pack validation (reuse, not parallel invention)
//!
//! Guidance follows the [`crate::pack::validator::ValidationIssue`] conventions
//! — a stable kebab-case code, an [`IssueSeverity`] Error/Warning axis, clean
//! serde round-trip, and optional fields skip-serialised. It **reuses**
//! [`IssueSeverity`] (the "does this block" axis) and
//! [`PolicySeverity`] (the declared band) directly rather than cloning them.
//!
//! It does **not** reuse [`crate::pack::validator::ValidationIssue`] wholesale,
//! nor its [`crate::pack::IssueCode`] enum: `IssueCode` is a closed set of *pack
//! structure* problems (missing policy file, duplicate id, …). Folding
//! assertion-violation codes into it would conflate two domains — the same
//! reason `validator.rs` keeps a dedicated `IssueSeverity` rather than reusing
//! `PolicySeverity`. So [`GuidanceCode`] is a sibling enum in the shared style,
//! and the blocking axis is derived from the assertion's declared band.

use serde::{Deserialize, Serialize};

use crate::context::adapters::{AssertionContext, AssertionEvaluation, Violation, evaluate};
use crate::context::assertion::{Assertion, AssertionCondition};
use crate::pack::PolicySeverity;
use crate::pack::validator::IssueSeverity;

/// Stable, machine-readable classification of an assertion violation.
///
/// The wire form is kebab-case and is part of the guidance contract consumed by
/// CI and tooling, so variants are added, never renamed. Each maps from the
/// condition kind that was violated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GuidanceCode {
    /// A changed path fell outside a `changed-paths-confined-to` area.
    ChangedPathNotConfined,
    /// A changed path matched a `changed-paths-exclude` protected pattern.
    ProtectedPathModified,
    /// The changed-path count breached a `changed-path-count` threshold.
    ChangedPathCountOutOfBounds,
    /// A `config-equals` value was absent or did not match.
    ConfigValueMismatch,
    /// A `config-present` value was absent or blank.
    ConfigValueMissing,
}

impl GuidanceCode {
    /// The guidance code for the condition kind that was violated.
    #[must_use]
    pub fn for_condition(condition: &AssertionCondition) -> Self {
        match condition {
            AssertionCondition::ChangedPathsConfinedTo(_) => Self::ChangedPathNotConfined,
            AssertionCondition::ChangedPathsExclude(_) => Self::ProtectedPathModified,
            AssertionCondition::ChangedPathCount(_) => Self::ChangedPathCountOutOfBounds,
            AssertionCondition::ConfigEquals(_) => Self::ConfigValueMismatch,
            AssertionCondition::ConfigPresent(_) => Self::ConfigValueMissing,
        }
    }
}

/// A remediation-first explanation of a single assertion violation.
///
/// Mirrors [`crate::pack::validator::ValidationIssue`]: a stable [`code`], a
/// blocking axis, an attribution, a human-readable `message`, and
/// remediation-first `remediation`. Optional attribution fields are
/// skip-serialised when absent.
///
/// [`severity`](Self::severity) carries the assertion's declared band;
/// [`blocking`](Self::blocking) is the derived Error/Warning axis (see
/// [`blocking_for`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionGuidance {
    /// Stable machine-readable code for the violated condition kind.
    pub code: GuidanceCode,
    /// The assertion's declared severity band (reused pack vocabulary).
    pub severity: PolicySeverity,
    /// Whether this violation blocks, on the pack Error/Warning axis (ADR-002).
    pub blocking: IssueSeverity,
    /// The id of the violated assertion.
    pub assertion_id: String,
    /// The offending changed path, when the condition names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Human-readable description of what failed. UK spelling.
    pub message: String,
    /// How to fix it — remediation-first guidance from the assertion.
    pub remediation: String,
    /// Why the assertion exists, when the author supplied a rationale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// Map a declared [`PolicySeverity`] band onto the Error/Warning blocking axis.
///
/// Per ADR-002 (warnings over blocks, exit 0 by default) only the top band
/// blocks: [`PolicySeverity::Critical`] → [`IssueSeverity::Error`]; every lower
/// band is advisory ([`IssueSeverity::Warning`]).
#[must_use]
pub fn blocking_for(severity: PolicySeverity) -> IssueSeverity {
    match severity {
        PolicySeverity::Critical => IssueSeverity::Error,
        PolicySeverity::Low | PolicySeverity::Medium | PolicySeverity::High => {
            IssueSeverity::Warning
        }
    }
}

/// Build guidance for a violation of `assertion`.
///
/// Returns `Some` only when `evaluation` is [`AssertionEvaluation::Violated`];
/// a satisfied or out-of-scope evaluation yields `None` (nothing to remediate).
/// Pure over its arguments.
#[must_use]
pub fn guidance_for(
    assertion: &Assertion,
    evaluation: &AssertionEvaluation,
) -> Option<AssertionGuidance> {
    let AssertionEvaluation::Violated(violation) = evaluation else {
        return None;
    };
    Some(from_violation(assertion, violation))
}

/// Evaluate `assertion` against `context` and, if violated, build its guidance.
///
/// The convenience that ties CPOL-002 evaluation to CPOL-003 guidance in one
/// call. Pure over its arguments.
#[must_use]
pub fn assess(assertion: &Assertion, context: &AssertionContext) -> Option<AssertionGuidance> {
    guidance_for(assertion, &evaluate(assertion, context))
}

fn from_violation(assertion: &Assertion, violation: &Violation) -> AssertionGuidance {
    let rationale = {
        let trimmed = assertion.rationale.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    };
    AssertionGuidance {
        code: GuidanceCode::for_condition(&violation.condition),
        severity: assertion.outcome,
        blocking: blocking_for(assertion.outcome),
        assertion_id: assertion.id.clone(),
        path: violation.offending_path.clone(),
        message: format!("assertion `{}` failed: {}", assertion.id, violation.detail),
        remediation: assertion.remediation.clone(),
        rationale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::adapters::ChangedPath;
    use crate::context::assertion::{
        AssertionScope, ChangeKind, ChangedPathCountSpec, Comparison, ConfigKey, ConfigMatch,
        PathGlob, WorkflowPhase,
    };

    fn assertion(outcome: PolicySeverity, conditions: Vec<AssertionCondition>) -> Assertion {
        Assertion {
            id: "confine-to-src".into(),
            title: "Confine changes".into(),
            scope: AssertionScope::default(),
            conditions,
            outcome,
            rationale: "Small scoped changes keep the blast radius small.".into(),
            remediation: "Move the change under crates/ or split it out.".into(),
        }
    }

    fn confined() -> Assertion {
        assertion(
            PolicySeverity::High,
            vec![AssertionCondition::ChangedPathsConfinedTo(PathGlob {
                glob: "crates/**".into(),
            })],
        )
    }

    fn escaping_context() -> AssertionContext {
        AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [ChangedPath::new("scripts/deploy.sh", ChangeKind::Added)],
            [],
        )
    }

    #[test]
    fn assertion_guidance_violation_produces_remediation_first_output() {
        let guidance = assess(&confined(), &escaping_context()).expect("violation → guidance");
        assert_eq!(guidance.code, GuidanceCode::ChangedPathNotConfined);
        assert_eq!(guidance.assertion_id, "confine-to-src");
        assert_eq!(guidance.path.as_deref(), Some("scripts/deploy.sh"));
        assert_eq!(
            guidance.remediation,
            "Move the change under crates/ or split it out."
        );
        assert!(
            guidance.message.contains("scripts/deploy.sh"),
            "{}",
            guidance.message
        );
    }

    #[test]
    fn assertion_guidance_satisfied_and_out_of_scope_yield_none() {
        // Satisfied: change is confined, no violation.
        let satisfied_ctx = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [ChangedPath::new(
                "crates/x/src/lib.rs",
                ChangeKind::Modified,
            )],
            [],
        );
        assert!(assess(&confined(), &satisfied_ctx).is_none());

        // Out of scope by phase.
        let scoped = assertion(
            PolicySeverity::High,
            vec![AssertionCondition::ConfigPresent(ConfigKey {
                key: "k".into(),
            })],
        );
        let scoped = Assertion {
            scope: AssertionScope {
                paths: vec![],
                phases: vec![WorkflowPhase::Push],
            },
            ..scoped
        };
        let save_ctx = AssertionContext::from_parts(WorkflowPhase::Save, [], []);
        assert!(assess(&scoped, &save_ctx).is_none());
    }

    #[test]
    fn assertion_guidance_severity_band_is_carried_and_blocking_axis_derived() {
        // Critical blocks (Error); every lower band is advisory (Warning).
        let critical = assess(
            &assertion(
                PolicySeverity::Critical,
                vec![AssertionCondition::ConfigPresent(ConfigKey {
                    key: "k".into(),
                })],
            ),
            &AssertionContext::from_parts(WorkflowPhase::Manual, [], []),
        )
        .expect("violation");
        assert_eq!(critical.severity, PolicySeverity::Critical);
        assert_eq!(critical.blocking, IssueSeverity::Error);

        let medium = assess(
            &assertion(
                PolicySeverity::Medium,
                vec![AssertionCondition::ConfigPresent(ConfigKey {
                    key: "k".into(),
                })],
            ),
            &AssertionContext::from_parts(WorkflowPhase::Manual, [], []),
        )
        .expect("violation");
        assert_eq!(medium.severity, PolicySeverity::Medium);
        assert_eq!(medium.blocking, IssueSeverity::Warning);
    }

    #[test]
    fn assertion_guidance_codes_map_from_each_condition_kind() {
        let cases = [
            (
                AssertionCondition::ChangedPathsExclude(PathGlob {
                    glob: "**/Cargo.lock".into(),
                }),
                GuidanceCode::ProtectedPathModified,
            ),
            (
                AssertionCondition::ChangedPathCount(ChangedPathCountSpec {
                    op: Comparison::AtMost,
                    value: 0,
                    change_kind: None,
                }),
                GuidanceCode::ChangedPathCountOutOfBounds,
            ),
            (
                AssertionCondition::ConfigEquals(ConfigMatch {
                    key: "signed".into(),
                    value: "true".into(),
                }),
                GuidanceCode::ConfigValueMismatch,
            ),
            (
                AssertionCondition::ConfigPresent(ConfigKey {
                    key: "owner".into(),
                }),
                GuidanceCode::ConfigValueMissing,
            ),
        ];
        let ctx = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [ChangedPath::new(
                "crates/x/Cargo.lock",
                ChangeKind::Modified,
            )],
            [("signed".to_string(), "false".to_string())],
        );
        for (condition, expected) in cases {
            let guidance =
                assess(&assertion(PolicySeverity::High, vec![condition]), &ctx).expect("violation");
            assert_eq!(guidance.code, expected);
        }
    }

    #[test]
    fn assertion_guidance_round_trips_and_skips_absent_optionals() {
        // A config violation names no path, so `path` is skip-serialised.
        let guidance = assess(
            &assertion(
                PolicySeverity::Medium,
                vec![AssertionCondition::ConfigPresent(ConfigKey {
                    key: "owner".into(),
                })],
            ),
            &AssertionContext::from_parts(WorkflowPhase::Manual, [], []),
        )
        .expect("violation");

        let json = serde_json::to_string(&guidance).expect("serialise");
        assert!(
            !json.contains("\"path\""),
            "absent path must be skip-serialised: {json}"
        );
        assert!(json.contains("config-value-missing"), "{json}");
        let restored: AssertionGuidance = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored, guidance);
    }

    #[test]
    fn assertion_guidance_omits_blank_rationale() {
        let mut a = assertion(
            PolicySeverity::High,
            vec![AssertionCondition::ConfigPresent(ConfigKey {
                key: "k".into(),
            })],
        );
        a.rationale = "   ".into();
        let guidance = assess(
            &a,
            &AssertionContext::from_parts(WorkflowPhase::Manual, [], []),
        )
        .expect("violation");
        assert!(guidance.rationale.is_none());
    }
}
