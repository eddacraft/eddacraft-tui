//! FLAGS-008 exemplar: CLI licence gating via the shared feature-flag resolver.
//!
//! Routes licence-gated feature access through `anvil_kernel::feature_flags`
//! rather than a bespoke per-command check, so future tier/entitlement
//! changes ship as manifest updates.

use anvil_kernel::feature_flags::{ResolutionDetails, ResolutionReason, resolve_flag};
use anvil_kernel_types::{
    AudienceContext, EnvironmentContext, EnvironmentName, EvaluationContext, FeatureFlagDefinition,
    FlagClass, FlagStatus, FlagValue, FlagValueType, FlagVariant,
};

pub const CLI_LICENCE_GATE_KEY: &str = "cli.licence-gate";

/// Builds the inline exemplar definition for `cli.licence-gate`.
///
/// Mirrors `packages/anvil/runtime/src/feature-flags/exemplars.test.ts` so
/// TypeScript and Rust surfaces evaluate the same flag contract. The shared
/// snapshot pipeline will supersede this inline definition once the CLI
/// gains a local snapshot loader; keep the shape in sync until then.
pub fn cli_licence_gate_flag() -> FeatureFlagDefinition {
    FeatureFlagDefinition {
        key: CLI_LICENCE_GATE_KEY.into(),
        owner: "BAUTH".into(),
        intent: "Gate CLI features behind licence validation".into(),
        class: FlagClass::Entitlement,
        value_type: FlagValueType::Boolean,
        variants: vec![
            FlagVariant {
                key: "enabled".into(),
                value: FlagValue::Boolean(true),
            },
            FlagVariant {
                key: "disabled".into(),
                value: FlagValue::Boolean(false),
            },
        ],
        default_variant: "enabled".into(),
        status: FlagStatus::Active,
        created_for: "FLAGS-008".into(),
        expiry_or_review_date: None,
        description: Some("Controls access to licence-gated CLI commands".into()),
        targeting: None,
    }
}

/// Build an OpenFeature-aligned evaluation context for the current CLI session.
///
/// `plan` comes from the `/api/v1/auth/verify` response (`WhoamiResponse.plan`).
/// `targeting_key` should be a stable per-session identifier; the CLI uses the
/// credential `sub` when available and falls back to `"cli-session"`.
pub fn cli_evaluation_context(
    targeting_key: impl Into<String>,
    plan: Option<&str>,
) -> EvaluationContext {
    EvaluationContext {
        targeting_key: targeting_key.into(),
        environment: EnvironmentContext {
            environment: EnvironmentName::Prod,
            channel: None,
            deployment_ring: None,
        },
        audience: Some(AudienceContext {
            licence_plan: plan.map(str::to_string),
            account_tier: plan.map(str::to_string),
            ..AudienceContext::default()
        }),
    }
}

/// Resolve `cli.licence-gate` for the given plan and return the raw details.
///
/// Callers decide policy (block vs warn); this helper only evaluates.
pub fn evaluate_cli_licence_gate(
    targeting_key: impl Into<String>,
    plan: Option<&str>,
) -> ResolutionDetails {
    let flag = cli_licence_gate_flag();
    let context = cli_evaluation_context(targeting_key, plan);
    resolve_flag(&flag, &context, None)
}

/// Convenience wrapper: is the CLI licence gate allowing access for this plan?
///
/// Exposed for future callers (e.g. per-command entitlement checks); the
/// current exemplar wiring in `commands::auth::whoami` uses the lower-level
/// `evaluate_cli_licence_gate` to surface the resolved variant.
#[allow(dead_code)]
pub fn is_cli_licence_enabled(plan: Option<&str>) -> bool {
    let details = evaluate_cli_licence_gate("cli-session", plan);
    details.variant == "enabled"
        && matches!(
            details.reason,
            ResolutionReason::Default
                | ResolutionReason::TargetingMatch
                | ResolutionReason::LocalOverride
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_definition_matches_exemplar_contract() {
        let flag = cli_licence_gate_flag();
        assert_eq!(flag.key, CLI_LICENCE_GATE_KEY);
        assert_eq!(flag.class, FlagClass::Entitlement);
        assert_eq!(flag.created_for, "FLAGS-008");
        assert!(flag.default_variant_exists());
        assert!(flag.has_valid_key());
    }

    #[test]
    fn default_variant_resolves_to_enabled_when_no_plan() {
        let details = evaluate_cli_licence_gate("cli-session", None);
        assert_eq!(details.variant, "enabled");
        assert_eq!(details.reason, ResolutionReason::Default);
    }

    #[test]
    fn plan_is_propagated_onto_evaluation_context() {
        let context = cli_evaluation_context("operator-42", Some("pro"));
        assert_eq!(context.targeting_key, "operator-42");
        let audience = context.audience.expect("audience");
        assert_eq!(audience.licence_plan.as_deref(), Some("pro"));
        assert_eq!(audience.account_tier.as_deref(), Some("pro"));
    }

    #[test]
    fn is_enabled_helper_allows_default() {
        assert!(is_cli_licence_enabled(Some("beta")));
        assert!(is_cli_licence_enabled(None));
    }
}
