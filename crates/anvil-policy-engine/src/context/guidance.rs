//! CPOL-003: map assertion failures to remediation-first guidance text.

use serde::{Deserialize, Serialize};

use anvil_kernel_types::diagnostics::ControlDecision;

use crate::context::adapters::{AssertionContext, AssertionEvaluation, Violation, evaluate};
use crate::context::assertion::{Assertion, AssertionCondition, AssertionError};
use crate::pack::PolicySeverity;
pub use crate::posture::EnforcementPosture;
use crate::posture::decision_for_band;

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
/// Mirrors [`crate::pack::validator::ValidationIssue`]: a stable [`code`], an
/// attribution, a human-readable `message`, and remediation-first
/// `remediation`. Optional attribution fields are skip-serialised when absent.
///
/// Deliberately carries **no** blocking flag: whether it blocks is computed on
/// demand from [`severity`](Self::severity) and a caller-supplied
/// [`EnforcementPosture`] via [`AssertionGuidance::decision_under`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionGuidance {
    /// Stable machine-readable code for the violated condition kind.
    pub code: GuidanceCode,
    /// The assertion's declared severity band (reused pack vocabulary).
    pub severity: PolicySeverity,
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

impl AssertionGuidance {
    /// The control decision for this guidance under a posture. Posture-driven,
    /// never stored (see the [module docs](self)).
    #[must_use]
    pub fn decision_under(&self, posture: EnforcementPosture) -> ControlDecision {
        decision_under(self.severity, posture)
    }

    /// Whether this guidance blocks under a posture.
    #[must_use]
    pub fn blocks_under(&self, posture: EnforcementPosture) -> bool {
        blocks_under(self.severity, posture)
    }
}

/// Map a declared [`PolicySeverity`] band to a [`ControlDecision`] under a
/// posture.
///
/// A thin adapter over the shared [`crate::posture`] rule: the high bands
/// ([`PolicySeverity::High`]/[`PolicySeverity::Critical`]) count as high-signal,
/// so under [`EnforcementPosture::Enforce`] they block and lower bands warn;
/// under [`EnforcementPosture::Warn`] everything warns (ADR-002). The same rule
/// and [`EnforcementPosture`] type back [`crate::io_risk::guidance`].
#[must_use]
pub fn decision_under(severity: PolicySeverity, posture: EnforcementPosture) -> ControlDecision {
    let high_or_critical = matches!(severity, PolicySeverity::High | PolicySeverity::Critical);
    decision_for_band(high_or_critical, posture)
}

/// Whether a violation of this declared band blocks under a posture.
#[must_use]
pub fn blocks_under(severity: PolicySeverity, posture: EnforcementPosture) -> bool {
    matches!(decision_under(severity, posture), ControlDecision::Block)
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
/// call. Validates the assertion first (via [`evaluate`]); a malformed
/// assertion is an [`AssertionError`], distinct from a well-formed assertion
/// that simply was not violated (`Ok(None)`).
pub fn assess(
    assertion: &Assertion,
    context: &AssertionContext,
) -> Result<Option<AssertionGuidance>, AssertionError> {
    Ok(guidance_for(assertion, &evaluate(assertion, context)?))
}

fn from_violation(assertion: &Assertion, violation: &Violation) -> AssertionGuidance {
    let rationale = {
        let trimmed = assertion.rationale.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    };
    AssertionGuidance {
        code: GuidanceCode::for_condition(&violation.condition),
        severity: assertion.outcome,
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
        let guidance = assess(&confined(), &escaping_context())
            .expect("valid assertion")
            .expect("violation → guidance");
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
        assert!(
            assess(&confined(), &satisfied_ctx)
                .expect("valid assertion")
                .is_none()
        );

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
        assert!(
            assess(&scoped, &save_ctx)
                .expect("valid assertion")
                .is_none()
        );
    }

    #[test]
    fn assertion_guidance_blocking_is_posture_driven_not_band_derived() {
        let critical = assess(
            &assertion(
                PolicySeverity::Critical,
                vec![AssertionCondition::ConfigPresent(ConfigKey {
                    key: "k".into(),
                })],
            ),
            &AssertionContext::from_parts(WorkflowPhase::Manual, [], []),
        )
        .expect("valid assertion")
        .expect("violation");
        // The band is carried, but blocking is not stored on the guidance.
        assert_eq!(critical.severity, PolicySeverity::Critical);
        let json = serde_json::to_value(&critical).expect("serialise");
        assert!(
            json.get("blocking").is_none(),
            "blocking must not be stored on the guidance: {json}"
        );

        // Default posture is warnings-first: nothing blocks (ADR-002).
        assert!(!critical.blocks_under(EnforcementPosture::default()));
        assert_eq!(
            critical.decision_under(EnforcementPosture::Warn),
            ControlDecision::Warn
        );
        // Enforce blocks the high bands, warns the lower ones — same guidance,
        // different posture, different decision.
        assert!(critical.blocks_under(EnforcementPosture::Enforce));

        let medium = assess(
            &assertion(
                PolicySeverity::Medium,
                vec![AssertionCondition::ConfigPresent(ConfigKey {
                    key: "k".into(),
                })],
            ),
            &AssertionContext::from_parts(WorkflowPhase::Manual, [], []),
        )
        .expect("valid assertion")
        .expect("violation");
        assert!(!medium.blocks_under(EnforcementPosture::Enforce));
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
            let guidance = assess(&assertion(PolicySeverity::High, vec![condition]), &ctx)
                .expect("valid assertion")
                .expect("violation");
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
        .expect("valid assertion")
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
    fn assertion_guidance_rejects_malformed_assertion_at_boundary() {
        // assess validates first (via evaluate): a malformed assertion — here a
        // blank rationale — is a boundary rejection, not runtime guidance with
        // odd fields.
        let mut a = assertion(
            PolicySeverity::High,
            vec![AssertionCondition::ConfigPresent(ConfigKey {
                key: "k".into(),
            })],
        );
        a.rationale = "   ".into();
        let err = assess(
            &a,
            &AssertionContext::from_parts(WorkflowPhase::Manual, [], []),
        )
        .expect_err("blank rationale must be rejected");
        assert!(matches!(
            err,
            AssertionError::MissingField {
                field: "rationale",
                ..
            }
        ));
    }
}
