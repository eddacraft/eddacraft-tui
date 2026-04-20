//! FLAGS-008 exemplar + FLAGM-002: CLI licence gating via the shared feature-flag resolver.
//!
//! Routes licence-gated feature access through `anvil_kernel::feature_flags`
//! rather than a bespoke per-command check, so future tier/entitlement
//! changes ship as manifest updates.
//!
//! FLAGM-002 introduces [`CliGateFlag`], a CLI-local wrapper around the
//! shared [`FeatureFlagDefinition`] that carries the per-command gating list
//! as typed metadata. The canonical gated-command names are the single
//! source of truth for `requires_auth()` in `main.rs`; the legacy match
//! remains in place for one release for dual-evaluation parity.

use anvil_kernel::feature_flags::{ResolutionDetails, resolve_flag};
use anvil_kernel_types::{
    AudienceContext, EnvironmentContext, EnvironmentName, EvaluationContext, FeatureFlagDefinition,
    FlagClass, FlagStatus, FlagValue, FlagValueType, FlagVariant,
};

pub const CLI_LICENCE_GATE_KEY: &str = "cli.licence-gate";

/// Canonical names of CLI commands that require a valid licence to run.
///
/// This list is metadata on the `cli.licence-gate` flag (exposed via
/// [`CliGateFlag::metadata`]); it is authoritative for the per-command
/// auth decision in `main::requires_auth`. `main::command_canonical_name`
/// maps `Commands` variants onto these names.
///
/// Keep in sync with `FeatureFlagDefinition::description` and with the
/// auth-bypass list maintained alongside `requires_auth`.
pub const CLI_GATED_COMMANDS: &[&str] = &[
    "architecture",
    "audit",
    "auth-whoami",
    "check",
    "drift",
    "export",
    "gate",
    "gate-config",
    "policy",
    "status",
    "watch",
    "whoami",
];

/// CLI-local metadata attached to the `cli.licence-gate` flag.
///
/// Kept in the CLI crate because the set of gated commands is a property
/// of the CLI host, not the shared flag contract. Other runtimes
/// (docs-site, anvil-api) have their own hosts and own gating lists.
#[derive(Debug, Clone, Copy)]
pub struct CliGateMetadata {
    /// Canonical command names that must evaluate this flag before running.
    pub gated_commands: &'static [&'static str],
}

/// The shared `cli.licence-gate` flag definition plus CLI-local metadata.
///
/// Having the metadata live on the flag means `requires_auth` does not
/// have to maintain its own parallel list; adding or removing a gated
/// command is a one-line edit in [`CLI_GATED_COMMANDS`].
#[derive(Debug, Clone)]
pub struct CliGateFlag {
    pub definition: FeatureFlagDefinition,
    pub metadata: CliGateMetadata,
}

/// Builds the inline exemplar definition for `cli.licence-gate`.
///
/// Follows the shared model exercised in
/// `packages/anvil/runtime/src/feature-flags/exemplars.test.ts` with one
/// intentional compatibility difference: the exemplar's `default_variant`
/// is `disabled` (Entitlement class fails closed), but the CLI keeps its
/// existing `enabled` default here so existing licensed sessions are not
/// regressed by the exemplar wiring. The full cutover to a fail-closed
/// default is scoped to a later FLAGM task.
///
/// The shared snapshot pipeline will supersede this inline definition
/// once the CLI gains a local snapshot loader; keep the shape in sync
/// with the exemplar until then.
pub fn cli_licence_gate_flag() -> CliGateFlag {
    CliGateFlag {
        definition: FeatureFlagDefinition {
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
        },
        metadata: CliGateMetadata {
            gated_commands: CLI_GATED_COMMANDS,
        },
    }
}

/// Build an OpenFeature-aligned evaluation context for the current CLI session.
///
/// `plan` comes from the `/api/v1/auth/verify` response (`WhoamiResponse.plan`).
/// `targeting_key` is supplied by the caller and should be a stable identifier
/// for this CLI session. Callers should prefer a non-PII stable identifier when
/// available, and may fall back to a constant such as `"cli-session"`. Today
/// `commands::auth::whoami` passes the authenticated email; a follow-up
/// will plumb a JWT `sub` through instead.
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
    resolve_flag(&flag.definition, &context, None)
}

/// Convenience wrapper: is the CLI licence gate allowing access for this plan?
///
/// Exposed for future callers (e.g. per-command entitlement checks); the
/// current exemplar wiring in `commands::auth::whoami` uses the lower-level
/// `evaluate_cli_licence_gate` to surface the resolved variant. Access is
/// determined from the resolved variant itself so any resolver-supported
/// path to `"enabled"` (including emergency overrides) is treated as enabled.
#[allow(dead_code)]
pub fn is_cli_licence_enabled(plan: Option<&str>) -> bool {
    let details = evaluate_cli_licence_gate("cli-session", plan);
    details.variant == "enabled"
}

/// Whether a command (by canonical name) sits behind the CLI licence gate.
///
/// This is the flag-driven replacement for the hard-coded match in
/// `main::requires_auth`. Lookup is O(N) over [`CLI_GATED_COMMANDS`],
/// which is a dozen entries — well below any realistic CLI dispatch cost.
pub fn command_needs_licence_gate(command_name: &str) -> bool {
    cli_licence_gate_flag()
        .metadata
        .gated_commands
        .iter()
        .any(|name| *name == command_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel::feature_flags::ResolutionReason;

    #[test]
    fn flag_definition_matches_exemplar_contract() {
        let flag = cli_licence_gate_flag();
        assert_eq!(flag.definition.key, CLI_LICENCE_GATE_KEY);
        assert_eq!(flag.definition.class, FlagClass::Entitlement);
        assert_eq!(flag.definition.created_for, "FLAGS-008");
        assert!(flag.definition.default_variant_exists());
        assert!(flag.definition.has_valid_key());
    }

    #[test]
    fn gate_metadata_exposes_gated_commands() {
        let flag = cli_licence_gate_flag();
        assert_eq!(flag.metadata.gated_commands, CLI_GATED_COMMANDS);
        assert!(!flag.metadata.gated_commands.is_empty());
    }

    #[test]
    fn gated_commands_are_sorted_and_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for name in CLI_GATED_COMMANDS {
            assert!(
                seen.insert(*name),
                "duplicate gated command: {name}"
            );
        }
        let sorted: Vec<&&str> = {
            let mut copy: Vec<&&str> = CLI_GATED_COMMANDS.iter().collect();
            copy.sort();
            copy
        };
        let actual: Vec<&&str> = CLI_GATED_COMMANDS.iter().collect();
        assert_eq!(actual, sorted, "CLI_GATED_COMMANDS must stay sorted");
    }

    #[test]
    fn command_needs_licence_gate_recognises_gated_commands() {
        assert!(command_needs_licence_gate("audit"));
        assert!(command_needs_licence_gate("gate-config"));
        assert!(command_needs_licence_gate("auth-whoami"));
        assert!(command_needs_licence_gate("whoami"));
    }

    #[test]
    fn command_needs_licence_gate_rejects_bypass_commands() {
        assert!(!command_needs_licence_gate("doctor"));
        assert!(!command_needs_licence_gate("admin"));
        assert!(!command_needs_licence_gate("login"));
        assert!(!command_needs_licence_gate("unknown-command"));
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
