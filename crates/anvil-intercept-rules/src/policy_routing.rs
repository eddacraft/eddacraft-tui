//! OPAE-007 / POLRESET-006: map policy outcomes onto [`ControlDecision`] given
//! [`EnforcementMode`].
//!
//! Contract-only adapter over `anvil-kernel-types`. Does not widen the binary
//! [`crate::InterceptRule`] trait; policy evaluation stays off the resident
//! daemon hot path (ADR-098).

use anvil_kernel_types::EnforcementMode;
use anvil_kernel_types::diagnostics::ControlDecision;

/// The severity class of a policy outcome — one of the two Rego rule
/// families a pack emits.
///
/// This mirrors the pack contract the engine crate extracts from a
/// `data.anvil.policies` result (`violation`/`violations`/`deny`/`denies`
/// → [`PolicySeverityClass::Violation`]; `warn`/`warnings` →
/// [`PolicySeverityClass::Warning`]). The routing contract is deliberately
/// severity-**class** shaped, not free-form severity, so the daemon-side
/// vocabulary stays a closed, engine-free enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PolicySeverityClass {
    /// An advisory finding (a Rego `warn` rule). Warns but never vetoes.
    Warning,
    /// A blocking-intent finding (a Rego `violation`/`deny` rule). Escalates
    /// through the posture, up to a veto under `fence` / `interrupt`.
    Violation,
}

/// A single policy evaluation outcome, in engine-free terms.
///
/// Carries the id of the rule that produced it and its
/// [severity class](PolicySeverityClass). It intentionally holds no
/// engine-specific detail (no `regorus` value, no finding shape) so the
/// contract depends only on `anvil-kernel-types`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyOutcome {
    /// Stable identifier of the rule / policy that produced this outcome.
    pub rule_id: String,
    /// Which Rego rule family the outcome came from.
    pub class: PolicySeverityClass,
}

impl PolicyOutcome {
    /// A violation-class outcome for `rule_id`.
    #[must_use]
    pub fn violation(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            class: PolicySeverityClass::Violation,
        }
    }

    /// A warning-class outcome for `rule_id`.
    #[must_use]
    pub fn warning(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            class: PolicySeverityClass::Warning,
        }
    }
}

/// Route a [`PolicyOutcome`] onto a [`ControlDecision`] under `posture`.
///
/// See the [module docs](self) for the full mapping table. In short:
///
/// - [`PolicySeverityClass::Warning`] → [`ControlDecision::Warn`] under
///   every posture except [`EnforcementMode::Off`] (→
///   [`ControlDecision::Allow`]). Never a veto.
/// - [`PolicySeverityClass::Violation`] →
///   [`EnforcementMode::escalated_decision`] for the posture (`off → allow`,
///   `warn → warn`, `fence → fence`, `interrupt → interrupt`).
///
/// This is a pure function of its two arguments — no I/O, no engine, no
/// state — so a daemon-side consumer can speak the same vocabulary without
/// linking a policy engine (ADR-098 AD-4).
#[must_use]
pub fn route_policy_outcome(outcome: &PolicyOutcome, posture: EnforcementMode) -> ControlDecision {
    match outcome.class {
        // Advisory: warn under every enforcing posture; suppressed under
        // `off`. An advisory finding cannot veto a write.
        PolicySeverityClass::Warning => match posture {
            EnforcementMode::Off => ControlDecision::Allow,
            _ => ControlDecision::Warn,
        },
        // Blocking-intent: escalate through the posture. `warn` caps at
        // `Warn` (ADR-002 warnings-first default), so a violation only
        // vetoes once the operator opts into `fence` / `interrupt`.
        PolicySeverityClass::Violation => posture.escalated_decision(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POSTURES: [EnforcementMode; 4] = [
        EnforcementMode::Off,
        EnforcementMode::Warn,
        EnforcementMode::Fence,
        EnforcementMode::Interrupt,
    ];

    #[test]
    fn policy_routing_full_class_by_posture_matrix() {
        // The complete mapping table pinned end to end.
        let violation = PolicyOutcome::violation("rule-a");
        let warning = PolicyOutcome::warning("rule-b");

        // Warning class: allow under off, warn under every enforcing posture.
        assert_eq!(
            route_policy_outcome(&warning, EnforcementMode::Off),
            ControlDecision::Allow
        );
        assert_eq!(
            route_policy_outcome(&warning, EnforcementMode::Warn),
            ControlDecision::Warn
        );
        assert_eq!(
            route_policy_outcome(&warning, EnforcementMode::Fence),
            ControlDecision::Warn
        );
        assert_eq!(
            route_policy_outcome(&warning, EnforcementMode::Interrupt),
            ControlDecision::Warn
        );

        // Violation class: escalate through the posture ladder.
        assert_eq!(
            route_policy_outcome(&violation, EnforcementMode::Off),
            ControlDecision::Allow
        );
        assert_eq!(
            route_policy_outcome(&violation, EnforcementMode::Warn),
            ControlDecision::Warn
        );
        assert_eq!(
            route_policy_outcome(&violation, EnforcementMode::Fence),
            ControlDecision::Fence
        );
        assert_eq!(
            route_policy_outcome(&violation, EnforcementMode::Interrupt),
            ControlDecision::Interrupt
        );
    }

    #[test]
    fn policy_routing_off_posture_always_allows_never_vetoes() {
        // `off` suppresses every decision to `allow` regardless of class —
        // and `allow` is never a veto.
        for outcome in [PolicyOutcome::violation("v"), PolicyOutcome::warning("w")] {
            let decision = route_policy_outcome(&outcome, EnforcementMode::Off);
            assert_eq!(decision, ControlDecision::Allow);
            assert!(!decision.is_veto());
        }
    }

    #[test]
    fn policy_routing_warnings_never_veto_under_any_posture() {
        // An advisory (warning-class) outcome can never veto a write, no
        // matter how strict the posture — ADR-098 AD-5 warnings-first.
        let warning = PolicyOutcome::warning("advisory");
        for posture in POSTURES {
            let decision = route_policy_outcome(&warning, posture);
            assert!(
                !decision.is_veto(),
                "warning-class must not veto under {posture:?}, got {decision:?}",
            );
        }
    }

    #[test]
    fn policy_routing_violation_matches_posture_escalated_decision() {
        // The violation ladder is exactly the posture's own escalation, so
        // it can never drift from `escalated_decision`.
        let violation = PolicyOutcome::violation("v");
        for posture in POSTURES {
            assert_eq!(
                route_policy_outcome(&violation, posture),
                posture.escalated_decision(),
            );
        }
    }

    #[test]
    fn policy_routing_only_violation_can_veto_and_only_under_strict_postures() {
        // Vetoes come only from a violation under `fence` / `interrupt`.
        assert!(
            route_policy_outcome(&PolicyOutcome::violation("v"), EnforcementMode::Fence).is_veto()
        );
        assert!(
            route_policy_outcome(&PolicyOutcome::violation("v"), EnforcementMode::Interrupt)
                .is_veto()
        );
        assert!(
            !route_policy_outcome(&PolicyOutcome::violation("v"), EnforcementMode::Warn).is_veto()
        );
    }

    #[test]
    fn policy_routing_never_emits_bare_block() {
        // Policy routing speaks only the escalation ladder
        // (allow/warn/fence/interrupt); the bare `block` decision belongs to
        // the input-error / gate surfaces, never to policy routing.
        for posture in POSTURES {
            for outcome in [PolicyOutcome::violation("v"), PolicyOutcome::warning("w")] {
                assert_ne!(
                    route_policy_outcome(&outcome, posture),
                    ControlDecision::Block,
                );
            }
        }
    }
}
