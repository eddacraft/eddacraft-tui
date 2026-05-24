# Feature Flag Inventory

| Type  | Authority | Owner   | Status | Freshness                                                                                                       |
| ----- | --------- | ------- | ------ | --------------------------------------------------------------------------------------------------------------- |
| Guide | Derived   | FLAGCAT | Live   | Last reviewed 2026-05-25 against `plans/modules/feature-flag-catalogue.aps.md` and FLAGM/FLAGS archived modules |

| Upstream                                                                                                                                                                                           | Downstream                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `plans/archive/modules/feature-flagging.aps.md`, `plans/archive/modules/feature-flag-migration.aps.md`, `plans/modules/feature-flag-catalogue.aps.md`, source flag definitions listed in the table | Feature-flag migration audits, `feature-flag-reference.md` |

This document classifies existing feature-flag-like controls in Anvil and maps
them onto the shared flagging model defined in `FLAGS`.

Created for: `FLAGS-009`

**Migration status:** All **migrated** controls below are **retired** — the
`FLAGM` module closed under FLAGM-006. Each migrated entry now routes through
the shared resolver as its sole source of truth; the legacy hard-coded checks
and dual-evaluation parity scaffolding have been deleted. Per-control flag keys,
evaluation context, and rollback paths are documented in
[`plans/specs/2026-04-20-feature-flag-migration-design.md`](../../plans/specs/2026-04-20-feature-flag-migration-design.md).

## Classification Key

| Action       | Meaning                                                             |
| ------------ | ------------------------------------------------------------------- |
| **migrated** | Retired onto the shared manifest and OpenFeature-backed resolution  |
| **adopt**    | New capability — will use the shared model from the start           |
| **defer**    | Not ready to migrate yet; document what a future migration involves |

## Summary Table

| Control                          | Location                                      | Classification | Flag class    | Mechanism      |
| -------------------------------- | --------------------------------------------- | -------------- | ------------- | -------------- |
| CLI licence-gated actions        | `crates/anvil-cli/src/feature_flags.rs`       | migrated       | `entitlement` | default/target |
| Docs access gating               | `apps/docs-site/lib/feature-flags.ts`         | migrated       | `entitlement` | targeting      |
| `ANVIL_DEV=1` auth bypass        | `crates/anvil-cli/src/feature_flags.rs`       | migrated       | `entitlement` | local override |
| `ADMIN_KEY` admin gating         | `apps/anvil-api/src/middleware/admin-auth.ts` | defer          | `entitlement` | —              |
| API access scopes                | `apps/anvil-api/src/lib/feature-flags.ts`     | migrated       | `entitlement` | default        |
| Policy profiles                  | `crates/anvil-policy/src/profiles.rs`         | defer          | —             | —              |
| Per-policy enabled/disabled      | `crates/anvil-policy/src/config.rs`           | defer          | —             | —              |
| OPA agent orchestration rollout  | (no flag yet)                                 | defer          | `rollout`     | —              |
| Tier-based product capabilities  | (no flag yet)                                 | adopt          | `entitlement` | —              |
| Web dashboard capabilities       | (no flag yet)                                 | adopt          | `entitlement` | —              |
| Dashboard AI builder             | (no flag yet)                                 | adopt          | `rollout`     | —              |
| Tutorial / advanced TUI surfaces | (no flag yet)                                 | adopt          | `rollout`     | —              |

## Retired Controls — Migrated

The four controls below were migrated onto the shared resolver across FLAGM-002
through FLAGM-005 and closed out in FLAGM-006. Each control's legacy hard-coded
check and dual-evaluation parity scaffolding have been deleted; the shared flag
is now the sole source of truth.

### CLI licence-gated actions — Migrated (FLAGM-002, closed FLAGM-006)

- **Resolver location:** `crates/anvil-cli/src/feature_flags.rs` —
  `command_needs_licence_gate()`; wired from `crates/anvil-cli/src/main.rs` —
  `requires_auth()`.
- **Flag key:** `cli.licence-gate` (class: `entitlement`).
- **Current state:** `requires_auth()` delegates to the shared resolver, which
  reads the gated-command list from the flag's metadata. The legacy hard-coded
  match (`requires_auth_legacy`) and its parity-test suite
  (`PARITY_COMMAND_CASES`) were retired in FLAGM-006.
- **Evaluation context:** `licencePlan` and `accountTier` from the loaded
  credential / `/api/v1/whoami` response, resolved in-process via the
  snapshot-backed provider (no per-command round-trip).
- **Featureboard swap impact:** Provider replacement only — the evaluation
  context and flag key stay the same.

### Docs access gating — Migrated (FLAGM-004, closed FLAGM-006)

- **Resolver location:** `apps/docs-site/lib/feature-flags.ts` —
  `evaluateDocsAccess()` is called from the Docusaurus middleware.
- **Flag key:** `docs.access` (class: `entitlement`).
- **Current state:** After JWT validation, the middleware resolves `docs.access`
  directly via `resolveFlag` from `@eddacraft/anvil-runtime/feature-flags`.
  Gating decisions are driven by `accountTier` targeting (beta, pro, enterprise)
  rather than just authentication presence. The runtime exemplar parity block
  that cross-checked an inline legacy evaluator was retired in FLAGM-006.
- **Evaluation context:** `accountTier` from the authenticated docs session
  claim.
- **Featureboard swap impact:** Provider replacement only — middleware still
  resolves via the same evaluation context.

### `ANVIL_DEV=1` auth bypass — Migrated (FLAGM-003, closed FLAGM-006)

- **Resolver location:** `crates/anvil-cli/src/feature_flags.rs` —
  `local_overrides_from_env()` + `cli_dev_bypass_active()`; called from
  `crates/anvil-cli/src/main.rs` — `check_auth()`.
- **Flag key:** `cli.licence-gate` with a local operator override (no dedicated
  `cli.dev-bypass` flag was needed — the resolver's local override precedence
  covers it).
- **Current state:** `ANVIL_DEV=1` is read by `local_overrides_from_env()` and
  surfaces as a local override on the `cli.licence-gate` flag. `check_auth()`
  calls `cli_dev_bypass_active()` to ask the resolver whether the override is
  active, then logs and skips the local auth pre-check. The legacy
  `legacy_dev_bypass_active` helper and its three parity tests were retired in
  FLAGM-006.
- **Status:** Kept as a documented local-override shortcut; no deprecation
  planned. The bypass is visible in flag telemetry and auditable.
- **Featureboard swap impact:** Local overrides are resolved before the provider
  is consulted, so no impact.

### API access scopes — Migrated (FLAGM-005, closed FLAGM-006)

- **Resolver location:** `apps/anvil-api/src/lib/feature-flags.ts` —
  `API_SCOPE_FLAGS`, `resolveApiScope()`, `isScopeAllowed()`. Admin request
  handling lives in `apps/anvil-api/src/routes/admin.ts`; the allowed-scope
  tuple is re-exported from `apps/anvil-api/src/routes/admin-schemas.ts`.
- **Flag keys:** `api.scope.beta`, `api.scope.preview`, `api.scope.internal`
  (class: `entitlement`).
- **Current state:** `API_SCOPE_NAMES` is the single source of truth; the legacy
  `ALLOWED_SCOPES = ['beta', 'preview', 'internal']` constant has been deleted
  and `admin-schemas.ts` derives the allowed-scope list from the flag manifest.
  `POST /admin/invite` calls `resolveApiScope` per request body scope and
  returns 403 `scope_not_allowed` when the flag resolves disabled — operator
  overrides have real runtime effect on the hot path. `/admin/approve` reads
  `DEFAULT_APPROVAL_SCOPES` from the manifest module. The
  `LEGACY_ALLOWED_SCOPES` tuple and its parity describe-block were retired in
  FLAGM-006.
- **Evaluation context:** `VERCEL_ENV`/`NODE_ENV` mapped to `environment`;
  `accountTier`/`userRole` available for future targeting rules.
- **Featureboard swap impact:** Provider replacement only — scope-to-tier
  mapping lives in the evaluation context adapter.

## Existing Controls — Defer

### `ADMIN_KEY` admin gating

- **Location:** `apps/anvil-api/src/middleware/admin-auth.ts`
- **Current state:** All admin operations (user approval, waitlist management)
  require `ADMIN_KEY` to be set. Requests without a matching key are rejected.
- **Classification:** **defer**
- **Reason:** This is an infrastructure secret, not audience targeting. It
  protects sensitive admin endpoints and would remain as a server-side secret
  check even if a flag were added. A flag could later gate admin surface
  availability per environment or operator role, but it is not a migration
  priority.
- **Future flag class:** `entitlement` with `userRole` targeting, if needed.
- **Featureboard swap impact:** None — secret-based gating sits below the flag
  layer.

### Policy profiles (Minimal / Standard / Strict)

- **Location:** `crates/anvil-policy/src/profiles.rs`
- **Current state:** Four profiles (`Minimal`, `Standard`, `Strict`, `Custom`)
  control which built-in policies are enabled and at what severity. Selected at
  config time via `anvil.toml` or CLI argument.
- **Classification:** **defer**
- **Reason:** Profiles are a configuration concern, not a rollout or entitlement
  gate. They control _what_ policies run, not _who_ can run them. Moving profile
  selection behind a feature flag would conflate configuration with access
  control. However, future tier-based restrictions on which profiles are
  available (e.g. `Strict` requires a pro plan) would be a natural `entitlement`
  flag.
- **Featureboard swap impact:** None unless profile access becomes tier-gated.

### Per-policy enabled/disabled

- **Location:** `crates/anvil-policy/src/config.rs` — `enabled: bool` field on
  `PolicyEntry`; `crates/anvil-policy/src/bundle.rs` — per-policy
  `enabled: Option<bool>` in the bundle manifest
- **Current state:** Individual policies can be toggled on or off in project
  configuration. Bundle manifests can also override enablement on a per-policy
  basis.
- **Classification:** **defer**
- **Reason:** Same reasoning as profiles — this is per-project configuration,
  not audience or rollout control. A policy being enabled or disabled is an
  author decision, not a product gate.
- **Featureboard swap impact:** None.

### OPA agent orchestration rollout

- **Location:** No formal flag exists. Rollout is controlled by configuration
  and manual environment promotion.
- **Classification:** **defer**
- **Reason:** The orchestration system is still being built (`OPAE` module).
  Once stable, a `rollout` class flag with environment targeting and an
  `ops_kill_switch` companion will be appropriate.
- **Future flag class:** `rollout` + `ops_kill_switch`
- **Featureboard swap impact:** None yet — no flag to swap.

## Future Controls — Adopt

These capabilities do not yet exist but will use the shared flagging model from
the start when they are built.

### Tier-based product capabilities

- **Current state:** No formal flag exists. Future tiers will need gating of
  specific Anvil features by plan level.
- **Classification:** **adopt**
- **Target flags:** Individual `entitlement` class flags per gated capability,
  each with `accountTier`/`licencePlan` targeting.
- **Featureboard swap impact:** Provider replacement only, assuming Featureboard
  supports the same evaluation context dimensions.

### Web dashboard capabilities

- **Current state:** Dashboard is under development (`DASH*` modules). No
  feature gating exists yet.
- **Classification:** **adopt**
- **Target flags:** Per-view `entitlement` flags gating advanced dashboard
  features by tier, audience, or environment.
- **Featureboard swap impact:** Provider replacement only.

### Dashboard AI builder

- **Current state:** AI-assisted dashboard generation is planned (`DASHAI`
  module) but not yet shipped.
- **Classification:** **adopt**
- **Target flag:** `dashboard.ai-builder` (class: `rollout`) — gate separately
  from baseline dashboard access to allow staged rollout and independent kill
  switch.
- **Featureboard swap impact:** Provider replacement only.

### Tutorial and advanced TUI surfaces

- **Current state:** Tutorial and onboarding are complete (`WELCOME` module).
  Future premium or experimental TUI experiences may need gating.
- **Classification:** **adopt**
- **Target flags:** Per-surface `rollout` flags for staged audience enablement.
- **Featureboard swap impact:** Provider replacement only.

## Excluded from Inventory

The following environment variables and controls were reviewed but are **not
feature flags**. They are operational configuration, debug tooling, or
infrastructure secrets and do not belong in the shared flagging model.

| Control                        | Location                                          | Reason excluded                                  |
| ------------------------------ | ------------------------------------------------- | ------------------------------------------------ |
| `ANVIL_DEBUG` / `DEBUG`        | `packages/anvil/core/src/utils/debug.ts`          | Debug logging — not feature gating               |
| `ANVIL_OPA_VERSION`            | `packages/anvil/policy/src/opa-binary-manager.ts` | Toolchain version override — not rollout control |
| `ANVIL_OPA_PATH`               | `packages/anvil/policy/src/opa-binary-manager.ts` | Custom binary path — infrastructure config       |
| `ANVIL_AGENT_TYPE` and related | `packages/anvil/runtime/src/concurrency/agent.ts` | Telemetry/metadata — not access gating           |
| CI detection env vars          | `packages/anvil/core/src/provenance/collector.ts` | Build metadata collection — not rollout control  |
| `RESEND_API_KEY`               | `apps/anvil-api/src/lib/email.ts`                 | Service credential — graceful degradation        |
| `RESEND_BETA_AUDIENCE_ID`      | `apps/anvil-api/src/lib/audience.ts`              | Mailing list config — not feature gating         |
| `DOCS_UPSTREAM_SECRET`         | `apps/docs-public/middleware.ts`                  | Infrastructure routing secret                    |

## Provider Swap Summary

A future Featureboard provider swap would affect:

| Layer                   | Impact                                               |
| ----------------------- | ---------------------------------------------------- |
| Application call sites  | None — they use OpenFeature                          |
| Evaluation context      | None — dimensions are vendor-neutral                 |
| Targeting rules         | Minimal — rewrite rules in Featureboard's format     |
| Snapshot publication    | Replace — Featureboard provides its own distribution |
| Provider implementation | Replace — new provider wraps Featureboard SDK        |
| Local overrides         | None — resolved before provider is consulted         |
| Telemetry               | Adjust — Featureboard may have its own observability |

The migration is isolated to the provider boundary. Application code that calls
`resolveFlag` or uses the OpenFeature client does not change. Controls
classified as **defer** sit below the flag layer entirely and are unaffected by
a provider swap.
