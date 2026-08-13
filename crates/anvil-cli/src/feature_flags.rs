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
#[cfg(feature = "kindling-embedded-runtime")]
use anvil_kernel_types::feature_flags_catalogue::kindling_embedded_runtime;
use anvil_kernel_types::feature_flags_catalogue::{
    cli_licence_gate, dashboard_web, tui_dashboard_aps_dashboard,
};
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
///
/// `welcome` is deliberately absent (ADR-080 / UJ-004): the read-mostly
/// discovery surface is the beta demo and sits in front of the licence
/// wall; durable surfaces (`init`, `start`, `watch`) stay gated.
pub const CLI_GATED_COMMANDS: &[&str] = &[
    "architecture",
    "audit",
    "auth-whoami",
    "check",
    "drift",
    "ensure",
    "export",
    "gate",
    "gate-config",
    "init",
    "mcp-config",
    "mcp-install",
    "new",
    "policy",
    "skill-install",
    "start",
    "status",
    "watch",
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

/// Plan-axis audience ids from `flags/audiences.json`. Keep in lock-step with
/// the catalogue inventory so we never invent a `plan-*` id that is not there.
const PLAN_AUDIENCE_IDS: &[&str] = &["plan-free", "plan-beta", "plan-pro", "plan-enterprise"];

/// Map a raw account plan (`beta`) to its catalogue audience id (`plan-beta`).
///
/// Mirrors `canonicalAccountTier` in `@eddacraft/anvil-flags-catalogue`: an
/// already-canonical id passes through; a bare name is prefixed only when
/// `plan-<name>` exists in the inventory; anything else is returned unchanged
/// so unknown plans fail closed.
#[must_use]
pub fn canonical_account_tier(tier: &str) -> String {
    if tier.is_empty() || PLAN_AUDIENCE_IDS.contains(&tier) {
        return tier.to_string();
    }
    let candidate = format!("plan-{tier}");
    if PLAN_AUDIENCE_IDS.contains(&candidate.as_str()) {
        candidate
    } else {
        tier.to_string()
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
///
/// `licence_plan` is the account column name (`beta`). `account_tier` is the
/// catalogue audience id (`plan-beta`) — BACT-013 / ADR-121 decision 3.
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
            account_tier: plan.map(canonical_account_tier),
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
/// variant: `cli.licence-gate` (FLAGM-003), since CIB-046
/// `tui-dashboard.aps-dashboard` (the internal-developer APS dashboard
/// gate), and since DASH-012 `dashboard.web` (the local browser dashboard).
/// Returning an owned value (rather than `Option`) keeps callers that always
/// pass overrides simple; an empty map is a no-op inside the resolver, and
/// the resolver only applies the override whose key matches the flag under
/// evaluation, so carrying several keys is harmless.
pub fn local_overrides_from_env() -> FlagOverrides {
    let mut overrides = FlagOverrides::default();
    if std::env::var(DEV_BYPASS_ENV_VAR).as_deref() == Ok("1") {
        overrides
            .local
            .insert(CLI_LICENCE_GATE_KEY.into(), "enabled".into());
        overrides
            .local
            .insert(APS_DASHBOARD_GATE_KEY.into(), "enabled".into());
        overrides
            .local
            .insert(DASHBOARD_WEB_GATE_KEY.into(), "enabled".into());
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
    let details = resolve_cli_licence_gate();
    is_dev_bypass(&details).then_some(details)
}

/// Resolve `cli.licence-gate` for the current session, honouring any
/// env-derived overrides.
///
/// Unlike [`cli_dev_bypass_active`], this always calls the resolver (even
/// with no override present), so the gating policy is consulted — and, via
/// the capture sink, recorded as USAGE-002 auth context — on every gated
/// invocation, in production as well as under `ANVIL_DEV`. The auth
/// decision is unchanged by this resolution today; making enforcement
/// flag-driven (so a `disabled` gate skips the credential check) is
/// tracked as USAGE-005.
#[must_use]
pub fn resolve_cli_licence_gate() -> ResolutionDetails {
    let overrides = local_overrides_from_env();
    let flag = cli_licence_gate_flag();
    let context = cli_evaluation_context("cli-session", None);
    resolve_flag(&flag.definition, &context, Some(&overrides))
}

/// Whether a resolved `cli.licence-gate` represents an active local-override
/// developer bypass (`ANVIL_DEV=1` forcing `enabled`).
#[must_use]
pub fn is_dev_bypass(details: &ResolutionDetails) -> bool {
    details.reason == ResolutionReason::LocalOverride && details.variant == "enabled"
}

/// Env var that overrides the SURFSQL governance surface for the session
/// (SURFSQL-005). `=1` forces `track.surface.sql` to `"enabled"` and `=0`
/// forces it to `"disabled"`, via a local override routed through the resolver —
/// not a bespoke env read — so the FLAGCAT flag stays the single source of
/// truth (OPSUP-005).
pub const TRACK_SURFACE_SQL_ENV_VAR: &str = "ANVIL_TRACK_SURFACE_SQL";

/// Whether the SURFSQL gate check is active this session.
///
/// Default-on: graduated after the v0.8.1-beta clean release (OPSUP-005).
/// `ANVIL_TRACK_SURFACE_SQL=0` forces it off for the session; `=1` forces it
/// on. Resolution goes through the shared resolver against the generated
/// `track.surface.sql` definition so the manifest default stays authoritative.
#[must_use]
pub fn track_surface_sql_enabled() -> bool {
    use anvil_kernel_types::feature_flags_catalogue::track_surface_sql;

    let mut overrides = FlagOverrides::default();
    match std::env::var(TRACK_SURFACE_SQL_ENV_VAR).as_deref() {
        Ok("1") => {
            overrides
                .local
                .insert(track_surface_sql::KEY.into(), "enabled".into());
        }
        Ok("0") => {
            overrides
                .local
                .insert(track_surface_sql::KEY.into(), "disabled".into());
        }
        _ => {}
    }
    let definition = track_surface_sql::definition();
    let context = cli_evaluation_context("gate-session", None);
    resolve_flag(&definition, &context, Some(&overrides)).variant == "enabled"
}

/// Env var that overrides the SURFGHA governance surface for the session
/// (SURFGHA-006); mirrors [`TRACK_SURFACE_SQL_ENV_VAR`] (`=1` on, `=0` off).
pub const TRACK_SURFACE_GHA_ENV_VAR: &str = "ANVIL_TRACK_SURFACE_GHA";

/// Whether the SURFGHA gate check is active this session. Default-on
/// (graduated after the v0.8.1-beta clean release per OPSUP-005);
/// `ANVIL_TRACK_SURFACE_GHA=0` forces it off and `=1` forces it on via the
/// shared resolver against the generated `track.surface.gha` definition.
#[must_use]
pub fn track_surface_gha_enabled() -> bool {
    use anvil_kernel_types::feature_flags_catalogue::track_surface_gha;

    let mut overrides = FlagOverrides::default();
    match std::env::var(TRACK_SURFACE_GHA_ENV_VAR).as_deref() {
        Ok("1") => {
            overrides
                .local
                .insert(track_surface_gha::KEY.into(), "enabled".into());
        }
        Ok("0") => {
            overrides
                .local
                .insert(track_surface_gha::KEY.into(), "disabled".into());
        }
        _ => {}
    }
    let definition = track_surface_gha::definition();
    let context = cli_evaluation_context("gate-session", None);
    resolve_flag(&definition, &context, Some(&overrides)).variant == "enabled"
}

/// Env var that overrides the SURFDOCK governance surface for the session
/// (SURFDOCK-005); mirrors [`TRACK_SURFACE_SQL_ENV_VAR`] (`=1` on, `=0` off).
pub const TRACK_SURFACE_DOCK_ENV_VAR: &str = "ANVIL_TRACK_SURFACE_DOCK";

/// Whether the SURFDOCK gate check is active this session. Default-on
/// (graduated after the v0.8.1-beta clean release per OPSUP-005);
/// `ANVIL_TRACK_SURFACE_DOCK=0` forces it off and `=1` forces it on via the
/// shared resolver against the generated `track.surface.dock` definition.
#[must_use]
pub fn track_surface_dock_enabled() -> bool {
    use anvil_kernel_types::feature_flags_catalogue::track_surface_dock;

    let mut overrides = FlagOverrides::default();
    match std::env::var(TRACK_SURFACE_DOCK_ENV_VAR).as_deref() {
        Ok("1") => {
            overrides
                .local
                .insert(track_surface_dock::KEY.into(), "enabled".into());
        }
        Ok("0") => {
            overrides
                .local
                .insert(track_surface_dock::KEY.into(), "disabled".into());
        }
        _ => {}
    }
    let definition = track_surface_dock::definition();
    let context = cli_evaluation_context("gate-session", None);
    resolve_flag(&definition, &context, Some(&overrides)).variant == "enabled"
}

/// Env var that overrides the SURFSH governance surface for the session
/// (SURFSH-005); mirrors [`TRACK_SURFACE_SQL_ENV_VAR`] (`=1` on, `=0` off).
pub const TRACK_SURFACE_SH_ENV_VAR: &str = "ANVIL_TRACK_SURFACE_SH";

/// Whether the SURFSH gate check is active this session. Default-on (graduated
/// after the v0.8.1-beta clean release per OPSUP-005); `ANVIL_TRACK_SURFACE_SH=0`
/// forces it off and `=1` forces it on via the shared resolver against the
/// generated `track.surface.sh` definition.
#[must_use]
pub fn track_surface_sh_enabled() -> bool {
    use anvil_kernel_types::feature_flags_catalogue::track_surface_sh;

    let mut overrides = FlagOverrides::default();
    match std::env::var(TRACK_SURFACE_SH_ENV_VAR).as_deref() {
        Ok("1") => {
            overrides
                .local
                .insert(track_surface_sh::KEY.into(), "enabled".into());
        }
        Ok("0") => {
            overrides
                .local
                .insert(track_surface_sh::KEY.into(), "disabled".into());
        }
        _ => {}
    }
    let definition = track_surface_sh::definition();
    let context = cli_evaluation_context("gate-session", None);
    resolve_flag(&definition, &context, Some(&overrides)).variant == "enabled"
}

/// The `cli.licence-gate` variant that means the gate is **off**. Mirrors
/// the `disabled` variant in `flags/manifest.json`; the sibling `enabled`
/// variant (the manifest default) means the gate is enforced. Kept as a
/// literal to match the existing `"enabled"` checks in this module;
/// [`tests::manifest_variants_match_gate_constants`] pins it against the
/// generated catalogue so it cannot drift.
const GATE_DISABLED_VARIANT: &str = "disabled";

/// USAGE-005: the local credential pre-check decision for a gated command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalAuthPrecheck {
    /// Run the local credential pre-check — the gate is enforced.
    Enforce,
    /// Skip the local pre-check; carries why, for the operator note.
    ///
    /// **What this actually relaxes:** most gated commands (`check`,
    /// `audit`, `export`, `status`, …) run entirely locally and never call
    /// the server, so the local pre-check *is* their only licence
    /// enforcement — skipping it runs them ungated. That is the intended
    /// operator control (a `disabled` gate turns licence enforcement off),
    /// not a "UX-only" relaxation. Only the network-touching commands
    /// (`auth`, `mcp`) additionally fail closed server-side without a valid
    /// token, so for those the server remains an independent backstop.
    Skip(LocalAuthSkipReason),
}

/// Why the local credential pre-check was skipped (USAGE-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalAuthSkipReason {
    /// `ANVIL_DEV=1` local override — the developer bypass.
    DevBypass,
    /// The licence gate resolved to its `disabled` variant — the gate is off.
    GateDisabled,
}

/// USAGE-005: decide whether a gated command must run the local credential
/// pre-check, from the resolved `cli.licence-gate` alone.
///
/// **Precedence (the recorded decision):**
/// 1. **Developer bypass** — `ANVIL_DEV=1` (a `LocalOverride` forcing
///    `enabled`) skips the local pre-check, exactly as before USAGE-005.
/// 2. **Flag variant** — otherwise the resolved variant decides: the
///    `disabled` variant (the gate is off, from a targeting rule, operator
///    config, or emergency override) skips the local pre-check; `enabled`
///    (including the manifest default) enforces it.
///
/// [`CLI_GATED_COMMANDS`] is orthogonal and unchanged: it selects *which*
/// commands consult the gate at all ([`command_needs_licence_gate`] →
/// `main::requires_auth`). This function governs only what happens once a
/// gated command has already entered `check_auth`.
///
/// **Scope of a `Skip` (security contract):** skipping the pre-check runs
/// the gated command without a local credential check. For the local-only
/// commands (the majority — `check`, `audit`, `export`, `status`, …) that
/// never contact the server, the local check *is* the licence enforcement,
/// so a `Skip` runs them fully ungated. This is the intended meaning of a
/// `disabled` licence gate, not an oversight. The network-touching commands
/// (`auth`, `mcp`) independently require a valid server token, so for those
/// the server remains a backstop even when this pre-check is skipped.
#[must_use]
pub fn local_auth_precheck(licence_gate: &ResolutionDetails) -> LocalAuthPrecheck {
    if is_dev_bypass(licence_gate) {
        return LocalAuthPrecheck::Skip(LocalAuthSkipReason::DevBypass);
    }
    if licence_gate.variant == GATE_DISABLED_VARIANT {
        return LocalAuthPrecheck::Skip(LocalAuthSkipReason::GateDisabled);
    }
    LocalAuthPrecheck::Enforce
}

// ── DASH-012: default-off gate for `anvil dashboard --web` ───────────

/// The `dashboard.web` flag key, sourced from the generated catalogue so it
/// cannot drift from `flags/manifest.json`.
pub const DASHBOARD_WEB_GATE_KEY: &str = dashboard_web::KEY;

/// Session override for the browser dashboard. `=1` forces
/// `dashboard.web` to `"enabled"` and `=0` forces it to `"disabled"`, via a
/// local override routed through the resolver — not a bespoke env branch —
/// so the FLAGCAT flag stays the single source of truth.
pub const DASHBOARD_WEB_ENV_VAR: &str = "ANVIL_DASHBOARD_WEB";

/// Whether the caller may open the local browser dashboard
/// (`anvil dashboard --web`).
///
/// Default-off for the v0.10.0-beta cut (foundations landed; UX not yet
/// release-default). Opt in with `ANVIL_DASHBOARD_WEB=1` or `ANVIL_DEV=1`.
/// `ANVIL_DASHBOARD_WEB=0` forces the surface off even under `ANVIL_DEV=1`.
/// Terminal `anvil dashboard` TUI surfaces are independent of this flag.
#[must_use]
pub fn web_dashboard_access_allowed() -> bool {
    let mut overrides = local_overrides_from_env();
    // Dedicated surface override applies after the broad ANVIL_DEV map so
    // `ANVIL_DASHBOARD_WEB=0` can still kill-switch the browser dashboard
    // during a full-dev session.
    match std::env::var(DASHBOARD_WEB_ENV_VAR).as_deref() {
        Ok("1") => {
            overrides
                .local
                .insert(DASHBOARD_WEB_GATE_KEY.into(), "enabled".into());
        }
        Ok("0") => {
            overrides
                .local
                .insert(DASHBOARD_WEB_GATE_KEY.into(), "disabled".into());
        }
        _ => {}
    }
    web_dashboard_access_allowed_with(&overrides)
}

/// Pure gate decision for tests: resolves `dashboard.web` against the
/// supplied overrides. Access is granted when the resolved variant is
/// `"enabled"`.
fn web_dashboard_access_allowed_with(overrides: &FlagOverrides) -> bool {
    let definition = dashboard_web::definition();
    let context = cli_evaluation_context("cli-session", None);
    let details = resolve_flag(&definition, &context, Some(overrides));
    details.variant == dashboard_web::variants::ENABLED
}

// ── KFIT-006: default-off embedded kindling runtime ────────────────

/// The `kindling.embedded-runtime` rollout key, generated from the canonical
/// flag manifest so the future runtime consumer cannot drift from catalogue
/// policy.
#[cfg(feature = "kindling-embedded-runtime")]
pub const KINDLING_EMBEDDED_RUNTIME_GATE_KEY: &str = kindling_embedded_runtime::KEY;

/// Dedicated local opt-in for feature-enabled development builds. The broad
/// `ANVIL_DEV` override deliberately does not open this gate: until KFIT-005
/// publishes the approved runtime, activation must be explicit and narrow.
#[cfg(feature = "kindling-embedded-runtime")]
pub const KINDLING_EMBEDDED_RUNTIME_ENV_VAR: &str = "ANVIL_KINDLING_EMBEDDED_RUNTIME";

/// Whether a feature-enabled build may use the embedded kindling runtime.
///
/// Normal release builds exclude the optional Cargo dependency entirely. In a
/// build that explicitly enables it, the rollout flag still resolves disabled
/// unless this dedicated local override is `1`; `0` forces it disabled.
#[must_use]
#[cfg(feature = "kindling-embedded-runtime")]
pub fn kindling_embedded_runtime_enabled() -> bool {
    let mut overrides = FlagOverrides::default();
    match std::env::var(KINDLING_EMBEDDED_RUNTIME_ENV_VAR).as_deref() {
        Ok("1") => {
            overrides.local.insert(
                KINDLING_EMBEDDED_RUNTIME_GATE_KEY.into(),
                kindling_embedded_runtime::variants::ENABLED.into(),
            );
        }
        Ok("0") => {
            overrides.local.insert(
                KINDLING_EMBEDDED_RUNTIME_GATE_KEY.into(),
                kindling_embedded_runtime::variants::DISABLED.into(),
            );
        }
        _ => {}
    }
    kindling_embedded_runtime_enabled_with(&overrides)
}

#[cfg(feature = "kindling-embedded-runtime")]
fn kindling_embedded_runtime_enabled_with(overrides: &FlagOverrides) -> bool {
    let definition = kindling_embedded_runtime::definition();
    let context = cli_evaluation_context("kindling-runtime", None);
    resolve_flag(&definition, &context, Some(overrides)).variant
        == kindling_embedded_runtime::variants::ENABLED
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
        // ADR-080 (UJ-004): `welcome` is deliberately ungated — the beta
        // demo surface sits in front of the licence wall.
        assert!(!command_needs_licence_gate("welcome"));
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
        assert_eq!(audience.account_tier.as_deref(), Some("plan-pro"));
    }

    #[test]
    fn canonical_account_tier_maps_bare_plan_and_passes_through_ids() {
        assert_eq!(canonical_account_tier("beta"), "plan-beta");
        assert_eq!(canonical_account_tier("plan-beta"), "plan-beta");
        assert_eq!(canonical_account_tier("enterprise"), "plan-enterprise");
        assert_eq!(canonical_account_tier("platinum"), "platinum");
        assert_eq!(canonical_account_tier(""), "");
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

    // ── SURFSQL-005: track.surface.sql default-on + opt-out ─────────

    #[test]
    fn track_surface_sql_enabled_by_default() {
        temp_env::with_var(TRACK_SURFACE_SQL_ENV_VAR, None::<&str>, || {
            assert!(
                track_surface_sql_enabled(),
                "Track 3 surface graduated to default-on (OPSUP-005)"
            );
        });
    }

    #[test]
    fn track_surface_sql_force_on_via_env() {
        temp_env::with_var(TRACK_SURFACE_SQL_ENV_VAR, Some("1"), || {
            assert!(track_surface_sql_enabled());
        });
    }

    #[test]
    fn track_surface_sql_force_off_via_env() {
        temp_env::with_var(TRACK_SURFACE_SQL_ENV_VAR, Some("0"), || {
            assert!(
                !track_surface_sql_enabled(),
                "ANVIL_TRACK_SURFACE_SQL=0 forces the surface off"
            );
        });
    }

    #[test]
    fn track_surface_sql_ignores_non_zero_one_values() {
        for value in ["true", "", "1 ", "yes"] {
            temp_env::with_var(TRACK_SURFACE_SQL_ENV_VAR, Some(value), || {
                assert!(
                    track_surface_sql_enabled(),
                    "ANVIL_TRACK_SURFACE_SQL={value:?} must not change the default-on surface"
                );
            });
        }
    }

    // ── SURFGHA-006: track.surface.gha default-on + opt-out ─────────

    #[test]
    fn track_surface_gha_enabled_by_default() {
        temp_env::with_var(TRACK_SURFACE_GHA_ENV_VAR, None::<&str>, || {
            assert!(track_surface_gha_enabled());
        });
    }

    #[test]
    fn track_surface_gha_force_on_via_env() {
        temp_env::with_var(TRACK_SURFACE_GHA_ENV_VAR, Some("1"), || {
            assert!(track_surface_gha_enabled());
        });
    }

    #[test]
    fn track_surface_gha_force_off_via_env() {
        temp_env::with_var(TRACK_SURFACE_GHA_ENV_VAR, Some("0"), || {
            assert!(
                !track_surface_gha_enabled(),
                "ANVIL_TRACK_SURFACE_GHA=0 forces the surface off"
            );
        });
    }

    #[test]
    fn track_surface_gha_ignores_non_zero_one_values() {
        for value in ["true", "", "1 ", "yes"] {
            temp_env::with_var(TRACK_SURFACE_GHA_ENV_VAR, Some(value), || {
                assert!(
                    track_surface_gha_enabled(),
                    "ANVIL_TRACK_SURFACE_GHA={value:?} must not change the default-on surface"
                );
            });
        }
    }

    // ── SURFDOCK-005: track.surface.dock default-on + opt-out ───────

    #[test]
    fn track_surface_dock_enabled_by_default() {
        temp_env::with_var(TRACK_SURFACE_DOCK_ENV_VAR, None::<&str>, || {
            assert!(track_surface_dock_enabled());
        });
    }

    #[test]
    fn track_surface_dock_force_on_via_env() {
        temp_env::with_var(TRACK_SURFACE_DOCK_ENV_VAR, Some("1"), || {
            assert!(track_surface_dock_enabled());
        });
    }

    #[test]
    fn track_surface_dock_force_off_via_env() {
        temp_env::with_var(TRACK_SURFACE_DOCK_ENV_VAR, Some("0"), || {
            assert!(
                !track_surface_dock_enabled(),
                "ANVIL_TRACK_SURFACE_DOCK=0 forces the surface off"
            );
        });
    }

    #[test]
    fn track_surface_dock_ignores_non_zero_one_values() {
        for value in ["true", "", "1 ", "yes"] {
            temp_env::with_var(TRACK_SURFACE_DOCK_ENV_VAR, Some(value), || {
                assert!(
                    track_surface_dock_enabled(),
                    "ANVIL_TRACK_SURFACE_DOCK={value:?} must not change the default-on surface"
                );
            });
        }
    }

    // ── SURFSH-005: track.surface.sh default-on + opt-out ───────────

    #[test]
    fn track_surface_sh_enabled_by_default() {
        temp_env::with_var(TRACK_SURFACE_SH_ENV_VAR, None::<&str>, || {
            assert!(track_surface_sh_enabled());
        });
    }

    #[test]
    fn track_surface_sh_force_on_via_env() {
        temp_env::with_var(TRACK_SURFACE_SH_ENV_VAR, Some("1"), || {
            assert!(track_surface_sh_enabled());
        });
    }

    #[test]
    fn track_surface_sh_force_off_via_env() {
        temp_env::with_var(TRACK_SURFACE_SH_ENV_VAR, Some("0"), || {
            assert!(
                !track_surface_sh_enabled(),
                "ANVIL_TRACK_SURFACE_SH=0 forces the surface off"
            );
        });
    }

    #[test]
    fn track_surface_sh_ignores_non_zero_one_values() {
        for value in ["true", "", "1 ", "yes"] {
            temp_env::with_var(TRACK_SURFACE_SH_ENV_VAR, Some(value), || {
                assert!(
                    track_surface_sh_enabled(),
                    "ANVIL_TRACK_SURFACE_SH={value:?} must not change the default-on surface"
                );
            });
        }
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

    // ── DASH-012: browser dashboard default-off gate ────────────────

    #[test]
    fn dashboard_web_gate_key_matches_catalogue() {
        assert_eq!(DASHBOARD_WEB_GATE_KEY, "dashboard.web");
        let definition = dashboard_web::definition();
        assert_eq!(definition.key, DASHBOARD_WEB_GATE_KEY);
        assert_eq!(definition.default_variant, "disabled");
        assert_eq!(definition.class, anvil_kernel_types::FlagClass::Rollout);
    }

    #[test]
    fn web_dashboard_denied_by_default() {
        assert!(!web_dashboard_access_allowed_with(&FlagOverrides::default()));
        let definition = dashboard_web::definition();
        let context = cli_evaluation_context("cli-session", None);
        let details = resolve_flag(&definition, &context, Some(&FlagOverrides::default()));
        assert_eq!(details.variant, "disabled");
        assert_eq!(details.reason, ResolutionReason::Default);
    }

    #[test]
    fn web_dashboard_allowed_with_dev_override() {
        temp_env::with_var(DEV_BYPASS_ENV_VAR, Some("1"), || {
            let overrides = local_overrides_from_env();
            assert_eq!(
                overrides
                    .local
                    .get(DASHBOARD_WEB_GATE_KEY)
                    .map(String::as_str),
                Some("enabled"),
                "ANVIL_DEV=1 must insert a local override on {DASHBOARD_WEB_GATE_KEY}"
            );
            assert!(web_dashboard_access_allowed_with(&overrides));
        });
    }

    #[test]
    fn web_dashboard_env_gate_allows_with_explicit_opt_in() {
        temp_env::with_vars(
            [
                (DASHBOARD_WEB_ENV_VAR, Some("1")),
                (DEV_BYPASS_ENV_VAR, None::<&str>),
            ],
            || {
                assert!(web_dashboard_access_allowed());
            },
        );
    }

    #[test]
    fn web_dashboard_env_gate_force_off_beats_dev_override() {
        temp_env::with_vars(
            [
                (DASHBOARD_WEB_ENV_VAR, Some("0")),
                (DEV_BYPASS_ENV_VAR, Some("1")),
            ],
            || {
                assert!(
                    !web_dashboard_access_allowed(),
                    "ANVIL_DASHBOARD_WEB=0 must kill-switch even under ANVIL_DEV=1"
                );
            },
        );
    }

    #[test]
    fn web_dashboard_env_gate_denies_when_no_env_set() {
        temp_env::with_vars(
            [
                (DASHBOARD_WEB_ENV_VAR, None::<&str>),
                (DEV_BYPASS_ENV_VAR, None::<&str>),
            ],
            || {
                assert!(!web_dashboard_access_allowed());
            },
        );
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
        // targeting rule, and `cli_evaluation_context` populates only the
        // plan/tier fields (`licence_plan`/`account_tier`) — there is no
        // staff-axis field to carry that audience. So even a fully-populated
        // plan/tier context resolves to the default "disabled" variant; nothing
        // short of the escape hatches can open the gate today. When the
        // staff-axis follow-up lands, this test should be updated alongside the
        // new targeting rule — it exists to flag that the gap is intentional.
        let definition = tui_dashboard_aps_dashboard::definition();
        assert!(
            definition.targeting.is_none(),
            "MVP flag must carry no targeting; the only open paths are the escape hatches"
        );
        let context = cli_evaluation_context("cli-session", Some("plan-enterprise"));
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

    // ── KFIT-006: default-off embedded kindling runtime ────────────────

    #[cfg(feature = "kindling-embedded-runtime")]
    #[test]
    fn kindling_embedded_runtime_gate_key_matches_catalogue() {
        assert_eq!(
            KINDLING_EMBEDDED_RUNTIME_GATE_KEY,
            "kindling.embedded-runtime"
        );
        let definition = kindling_embedded_runtime::definition();
        assert_eq!(definition.key, KINDLING_EMBEDDED_RUNTIME_GATE_KEY);
    }

    #[cfg(feature = "kindling-embedded-runtime")]
    #[test]
    fn kindling_embedded_runtime_is_disabled_by_default() {
        assert!(!kindling_embedded_runtime_enabled_with(
            &FlagOverrides::default()
        ));
    }

    // ── USAGE-005: flag-driven licence-gate enforcement ─────────────────

    /// Build a `cli.licence-gate` resolution with a given variant + reason.
    fn gate(variant: &str, reason: ResolutionReason) -> ResolutionDetails {
        ResolutionDetails {
            value: serde_json::Value::Bool(variant == "enabled"),
            variant: variant.to_owned(),
            reason,
            flag_key: CLI_LICENCE_GATE_KEY.to_owned(),
            error_code: None,
            error_message: None,
        }
    }

    #[test]
    fn manifest_variants_match_gate_constants() {
        // Pin the literal `disabled` variant against the catalogue so the
        // enforcement decision cannot silently drift from the manifest.
        let flag = cli_licence_gate_flag();
        let variants: Vec<&str> = flag
            .definition
            .variants
            .iter()
            .map(|v| v.key.as_str())
            .collect();
        assert!(
            variants.contains(&GATE_DISABLED_VARIANT),
            "manifest must define the `{GATE_DISABLED_VARIANT}` variant: {variants:?}"
        );
        assert!(
            variants.contains(&"enabled"),
            "manifest must define the `enabled` variant: {variants:?}"
        );
        assert_eq!(
            flag.definition.default_variant, "enabled",
            "default must stay `enabled` so production enforcement is unchanged"
        );
    }

    #[test]
    fn local_auth_precheck_enforces_enabled_default() {
        // The production path: manifest default `enabled` → enforce the
        // local credential pre-check exactly as before USAGE-005.
        assert_eq!(
            local_auth_precheck(&gate("enabled", ResolutionReason::Default)),
            LocalAuthPrecheck::Enforce
        );
    }

    #[test]
    fn local_auth_precheck_skips_when_gate_disabled() {
        // An operator/targeting rule turning the gate off → skip the local
        // pre-check. For local-only commands that means they run ungated
        // (the intended effect of a `disabled` gate); only the
        // network-touching commands still require a server token.
        assert_eq!(
            local_auth_precheck(&gate(
                GATE_DISABLED_VARIANT,
                ResolutionReason::TargetingMatch
            )),
            LocalAuthPrecheck::Skip(LocalAuthSkipReason::GateDisabled)
        );
        // Same outcome whatever the (non-dev) source that produced `disabled`.
        assert_eq!(
            local_auth_precheck(&gate(GATE_DISABLED_VARIANT, ResolutionReason::Default)),
            LocalAuthPrecheck::Skip(LocalAuthSkipReason::GateDisabled)
        );
    }

    #[test]
    fn local_auth_precheck_skips_for_dev_bypass() {
        // ANVIL_DEV=1 → LocalOverride forcing `enabled` is the developer
        // bypass; it skips with the DevBypass rationale (not GateDisabled).
        assert_eq!(
            local_auth_precheck(&gate("enabled", ResolutionReason::LocalOverride)),
            LocalAuthPrecheck::Skip(LocalAuthSkipReason::DevBypass)
        );
    }

    #[test]
    fn local_auth_precheck_dev_bypass_does_not_mask_a_disabled_override() {
        // A LocalOverride that resolved to `disabled` is not the dev bypass
        // (which forces `enabled`); it still skips, but as GateDisabled.
        assert_eq!(
            local_auth_precheck(&gate(
                GATE_DISABLED_VARIANT,
                ResolutionReason::LocalOverride
            )),
            LocalAuthPrecheck::Skip(LocalAuthSkipReason::GateDisabled)
        );
    }

    #[test]
    fn local_auth_precheck_enforces_emergency_enabled() {
        // An emergency override re-enabling the gate enforces the pre-check.
        assert_eq!(
            local_auth_precheck(&gate("enabled", ResolutionReason::EmergencyOverride)),
            LocalAuthPrecheck::Enforce
        );
    }
}
