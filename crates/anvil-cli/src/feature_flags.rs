//! FLAGS-008 + FLAGM-002/003 + FLAGCAT-005: CLI licence gating via the shared feature-flag resolver.
//! The `cli.licence-gate` definition is catalogue-sourced (generated from
//! `flags/manifest.json`); this module is the CLI host + gating metadata.
//!
//! Routes licence-gated feature access through `anvil_kernel::feature_flags`
//! rather than a bespoke per-command check, so future tier/entitlement
//! changes ship as manifest updates.
//!
//! [`CliGateFlag`] is a CLI-local wrapper around the shared
//! [`FeatureFlagDefinition`] that carries the per-command gating list as
//! typed metadata. The canonical gated-command names in
//! [`CLI_GATED_COMMANDS`] are the sole source of truth for
//! `requires_auth()` in `main.rs`; FLAGM-006 retired the legacy match.
//!
//! `ANVIL_DEV=1` is a documented local-override shortcut on
//! `cli.licence-gate` (see [`cli_dev_bypass_active`] and
//! [`local_overrides_from_env`]). `main::check_auth` asks the resolver
//! whether a local override forces the gate to `"enabled"` for this
//! session; no direct env-var branching remains.

use anvil_kernel::feature_flags::{
    FlagOverrides, ResolutionDetails, ResolutionReason, resolve_flag,
};
use anvil_kernel_types::feature_flags_catalogue::{cli_licence_gate, tui_dashboard_aps_dashboard};
use anvil_kernel_types::{
    AudienceContext, EnvironmentContext, EnvironmentName, EvaluationContext, FeatureFlagDefinition,
};

/// The `cli.licence-gate` flag key, sourced from the generated catalogue
/// (FLAGCAT-005) so it cannot drift from `flags/manifest.json`.
pub const CLI_LICENCE_GATE_KEY: &str = cli_licence_gate::KEY;

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
    "init",
    "mcp-config",
    "mcp-install",
    "new",
    "policy",
    "start",
    "status",
    "watch",
    "welcome",
    "whoami",
    "wizard",
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

/// Builds the `cli.licence-gate` flag plus CLI-local gating metadata.
///
/// FLAGCAT-005: the flag *definition* is now the build-time-generated
/// `cli_licence_gate::definition()`, sourced from `flags/manifest.json` —
/// no hand-rolled literal. The manifest preserves the CLI's `enabled`
/// default (Entitlement class), so behaviour is unchanged. Only the
/// CLI-host gating list ([`CLI_GATED_COMMANDS`]) lives here.
pub fn cli_licence_gate_flag() -> CliGateFlag {
    CliGateFlag {
        definition: cli_licence_gate::definition(),
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
            environment: EnvironmentName::Production,
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
        .contains(&command_name)
}

/// Developer-override env variable that forces `cli.licence-gate` to
/// `"enabled"` for the current session. Retained as a compatibility shim
/// during the FLAGM-003 dual-evaluation window; FLAGM-006 will decide
/// whether to promote it to a documented local-override contract or
/// retire it entirely.
pub const DEV_BYPASS_ENV_VAR: &str = "ANVIL_DEV";

/// Build the [`FlagOverrides`] map implied by environment variables.
///
/// `ANVIL_DEV=1` is the single recognised source. It inserts local
/// overrides forcing the developer-bypassable flags to their `"enabled"`
/// variant: `cli.licence-gate` (FLAGM-003) and, since CIB-046,
/// `tui-dashboard.aps-dashboard` (the internal-developer APS dashboard
/// gate). Returning an owned value (rather than `Option`) keeps callers
/// that always pass overrides simple; an empty map is a no-op inside the
/// resolver, and the resolver only applies the override whose key matches
/// the flag under evaluation, so carrying both keys is harmless.
pub fn local_overrides_from_env() -> FlagOverrides {
    let mut overrides = FlagOverrides::default();
    if std::env::var(DEV_BYPASS_ENV_VAR).as_deref() == Ok("1") {
        overrides
            .local
            .insert(CLI_LICENCE_GATE_KEY.into(), "enabled".into());
        overrides
            .local
            .insert(APS_DASHBOARD_GATE_KEY.into(), "enabled".into());
    }
    overrides
}

/// If a local override forces `cli.licence-gate` to `"enabled"` for the
/// current session, returns the [`ResolutionDetails`] that proved it.
/// Otherwise returns `None`.
///
/// Replaces the raw `ANVIL_DEV=1` branch previously in `main::check_auth`
/// so that the resolver's local-override precedence (not a bespoke
/// env-var read) is the single source of truth for developer bypass.
pub fn cli_dev_bypass_active() -> Option<ResolutionDetails> {
    let overrides = local_overrides_from_env();
    if overrides.local.is_empty() {
        return None;
    }
    let flag = cli_licence_gate_flag();
    let context = cli_evaluation_context("cli-session", None);
    let details = resolve_flag(&flag.definition, &context, Some(&overrides));
    if details.reason == ResolutionReason::LocalOverride && details.variant == "enabled" {
        Some(details)
    } else {
        None
    }
}

// ── CIB-046: internal-developer gate for `anvil plan dashboard` ──────

/// The `tui-dashboard.aps-dashboard` flag key, sourced from the generated
/// catalogue so it cannot drift from `flags/manifest.json`.
pub const APS_DASHBOARD_GATE_KEY: &str = tui_dashboard_aps_dashboard::KEY;

/// Operator escape-hatch env var: a non-empty value grants access to the
/// internal-developer APS dashboard without a personal credential, the same
/// way `anvil admin` authenticates (see `bypass_auth_admin`). A presence
/// check is sufficient here — the dashboard is read-only and local, so the
/// gate does not validate the key against the server.
pub const ADMIN_KEY_ENV_VAR: &str = "ANVIL_ADMIN_KEY";

/// Whether the caller may open the read-only APS plan dashboard
/// (`anvil plan dashboard`).
///
/// CIB-046 brings this surface — previously always-on and unauthenticated
/// because `"plan"` is absent from [`CLI_GATED_COMMANDS`] — under the
/// FLAGCAT catalogue as an internal-developer feature. The flag is
/// default-disabled; the two runtime paths that open it are `ANVIL_DEV=1`
/// (the developer local override, via [`local_overrides_from_env`]) and a
/// non-empty `ANVIL_ADMIN_KEY`. Plumbing a staff-axis audience signal from
/// `/auth/verify` so the flag can target `staff-internal-developer` for a
/// real authenticated caller is a deferred follow-up.
pub fn aps_dashboard_access_allowed() -> bool {
    // `trim().is_empty()` rejects a whitespace-only value (a common shell
    // accident, e.g. `export ANVIL_ADMIN_KEY=" "`) so it cannot open the gate.
    let admin_key_present =
        std::env::var(ADMIN_KEY_ENV_VAR).is_ok_and(|value| !value.trim().is_empty());
    aps_dashboard_access_allowed_with(admin_key_present, &local_overrides_from_env())
}

/// Pure gate decision, separated from env I/O so it can be unit-tested with
/// synthetic inputs. Access is granted when a non-empty admin key is present
/// or the flag resolves to its `"enabled"` variant for the supplied
/// overrides (any resolver path to `"enabled"`, including the dev override).
fn aps_dashboard_access_allowed_with(admin_key_present: bool, overrides: &FlagOverrides) -> bool {
    if admin_key_present {
        return true;
    }
    let definition = tui_dashboard_aps_dashboard::definition();
    let context = cli_evaluation_context("cli-session", None);
    let details = resolve_flag(&definition, &context, Some(overrides));
    details.variant == tui_dashboard_aps_dashboard::variants::ENABLED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_definition_matches_exemplar_contract() {
        let flag = cli_licence_gate_flag();
        assert_eq!(flag.definition.key, CLI_LICENCE_GATE_KEY);
        assert_eq!(
            flag.definition.class,
            anvil_kernel_types::FlagClass::Entitlement
        );
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
            assert!(seen.insert(*name), "duplicate gated command: {name}");
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

    // ── FLAGM-003: dev-bypass as local override ─────────────────────

    #[test]
    fn local_overrides_from_env_has_no_override_when_anvil_dev_unset() {
        temp_env::with_var(DEV_BYPASS_ENV_VAR, None::<&str>, || {
            let overrides = local_overrides_from_env();
            assert!(overrides.local.is_empty());
            assert!(overrides.emergency.is_empty());
        });
    }

    #[test]
    fn local_overrides_from_env_sets_gate_enabled_when_anvil_dev_one() {
        temp_env::with_var(DEV_BYPASS_ENV_VAR, Some("1"), || {
            let overrides = local_overrides_from_env();
            assert_eq!(
                overrides
                    .local
                    .get(CLI_LICENCE_GATE_KEY)
                    .map(String::as_str),
                Some("enabled"),
                "ANVIL_DEV=1 must insert a local override on {CLI_LICENCE_GATE_KEY}"
            );
        });
    }

    #[test]
    fn local_overrides_from_env_ignores_non_one_values() {
        // Only the literal "1" enables dev bypass; any other value (including
        // "true", "0", empty, or a whitespace-decorated "1") is a no-op. The
        // decorated cases guard against a future `.trim()` "helpfully" widening
        // the contract.
        for value in ["true", "0", "", "yes", "1\n", " 1", "1 "] {
            temp_env::with_var(DEV_BYPASS_ENV_VAR, Some(value), || {
                let overrides = local_overrides_from_env();
                assert!(
                    overrides.local.is_empty(),
                    "ANVIL_DEV={value:?} must not enable bypass"
                );
            });
        }
    }

    #[test]
    fn cli_dev_bypass_active_returns_some_when_anvil_dev_one() {
        temp_env::with_var(DEV_BYPASS_ENV_VAR, Some("1"), || {
            let details = cli_dev_bypass_active().expect("override must be active");
            assert_eq!(details.flag_key, CLI_LICENCE_GATE_KEY);
            assert_eq!(details.variant, "enabled");
            assert_eq!(details.reason, ResolutionReason::LocalOverride);
        });
    }

    #[test]
    fn cli_dev_bypass_active_returns_none_when_anvil_dev_unset() {
        temp_env::with_var(DEV_BYPASS_ENV_VAR, None::<&str>, || {
            assert!(cli_dev_bypass_active().is_none());
        });
    }

    // ── CIB-046: APS dashboard internal-developer gate ──────────────

    #[test]
    fn aps_dashboard_gate_key_matches_catalogue() {
        assert_eq!(APS_DASHBOARD_GATE_KEY, "tui-dashboard.aps-dashboard");
        let definition = tui_dashboard_aps_dashboard::definition();
        assert_eq!(definition.key, APS_DASHBOARD_GATE_KEY);
        // Default-disabled: the surface is closed unless explicitly opened.
        assert_eq!(definition.default_variant, "disabled");
    }

    #[test]
    fn aps_dashboard_denied_by_default() {
        // No admin key, no overrides → the default "disabled" variant wins.
        assert!(!aps_dashboard_access_allowed_with(
            false,
            &FlagOverrides::default()
        ));
        // Pin *why* it is denied: the resolver lands on the manifest default,
        // not a targeting match. This makes a `defaultVariant` flip to
        // "enabled" fail here immediately rather than silently opening the gate.
        let definition = tui_dashboard_aps_dashboard::definition();
        let context = cli_evaluation_context("cli-session", None);
        let details = resolve_flag(&definition, &context, Some(&FlagOverrides::default()));
        assert_eq!(details.variant, "disabled");
        assert_eq!(details.reason, ResolutionReason::Default);
    }

    #[test]
    fn aps_dashboard_audience_is_inert_without_staff_axis_plumbing() {
        // MVP deferral guard: the `staff-internal-developer` audience is
        // declared on the `tui-dashboard` group, but the flag carries no
        // targeting rule and the CLI evaluation context has no staff-axis
        // signal. So even a caller whose context names that audience (here via
        // account_tier) still resolves to the default "disabled" variant. When
        // the staff-axis follow-up lands, this test should be updated alongside
        // the new targeting rule — it exists to flag that the gap is intentional.
        let definition = tui_dashboard_aps_dashboard::definition();
        assert!(
            definition.targeting.is_none(),
            "MVP flag must carry no targeting; the only open paths are the escape hatches"
        );
        let context = cli_evaluation_context("cli-session", Some("staff-internal-developer"));
        let details = resolve_flag(&definition, &context, Some(&FlagOverrides::default()));
        assert_eq!(details.variant, "disabled");
        assert_eq!(details.reason, ResolutionReason::Default);
    }

    #[test]
    fn aps_dashboard_allowed_with_admin_key() {
        // A present admin key opens the surface regardless of flag state.
        assert!(aps_dashboard_access_allowed_with(
            true,
            &FlagOverrides::default()
        ));
    }

    #[test]
    fn aps_dashboard_allowed_with_dev_override() {
        // The ANVIL_DEV=1 local override forces the flag to "enabled".
        temp_env::with_var(DEV_BYPASS_ENV_VAR, Some("1"), || {
            let overrides = local_overrides_from_env();
            assert_eq!(
                overrides
                    .local
                    .get(APS_DASHBOARD_GATE_KEY)
                    .map(String::as_str),
                Some("enabled"),
                "ANVIL_DEV=1 must insert a local override on {APS_DASHBOARD_GATE_KEY}"
            );
            assert!(aps_dashboard_access_allowed_with(false, &overrides));
        });
    }

    #[test]
    fn aps_dashboard_env_gate_denies_when_no_env_set() {
        // End-to-end through the env-reading entry point: with neither
        // ANVIL_ADMIN_KEY nor ANVIL_DEV set, the dashboard is refused.
        temp_env::with_vars(
            [
                (ADMIN_KEY_ENV_VAR, None::<&str>),
                (DEV_BYPASS_ENV_VAR, None::<&str>),
            ],
            || {
                assert!(!aps_dashboard_access_allowed());
            },
        );
    }

    #[test]
    fn aps_dashboard_env_gate_allows_with_admin_key() {
        temp_env::with_vars(
            [
                (ADMIN_KEY_ENV_VAR, Some("admin-token")),
                (DEV_BYPASS_ENV_VAR, None::<&str>),
            ],
            || {
                assert!(aps_dashboard_access_allowed());
            },
        );
    }

    #[test]
    fn aps_dashboard_env_gate_denies_with_empty_admin_key() {
        // An empty ANVIL_ADMIN_KEY is not a credential.
        temp_env::with_vars(
            [
                (ADMIN_KEY_ENV_VAR, Some("")),
                (DEV_BYPASS_ENV_VAR, None::<&str>),
            ],
            || {
                assert!(!aps_dashboard_access_allowed());
            },
        );
    }

    #[test]
    fn aps_dashboard_env_gate_denies_with_whitespace_only_admin_key() {
        // A whitespace-only ANVIL_ADMIN_KEY (a common shell accident) is not a
        // credential and must not open the gate.
        for value in ["   ", "\t", " \n"] {
            temp_env::with_vars(
                [
                    (ADMIN_KEY_ENV_VAR, Some(value)),
                    (DEV_BYPASS_ENV_VAR, None::<&str>),
                ],
                || {
                    assert!(
                        !aps_dashboard_access_allowed(),
                        "whitespace-only admin key {value:?} must not open the gate"
                    );
                },
            );
        }
    }
}
