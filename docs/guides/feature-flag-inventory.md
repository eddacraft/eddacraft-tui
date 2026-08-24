# Feature Flag Inventory

| Type  | Authority | Owner   | Status | Freshness                                                                                                                                                                            |
| ----- | --------- | ------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Guide | Derived   | FLAGCAT | Live   | Last reviewed 2026-08-25 against FLAGCAT-012 host completeness, FLAGCAT-013 linkage, FLAGCAT-011/-016, ADR-076, `flags/surfaces.json`, the catalogue loader, and live flag consumers |

| Upstream                                                                                                                                                                                                                  | Downstream                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `plans/archive/modules/feature-flagging.aps.md`, `plans/archive/modules/feature-flag-migration.aps.md`, `plans/modules/feature-flag-catalogue.aps.md`, `flags/surfaces.json`, source flag definitions listed in the table | Feature-flag migration audits, `feature-flag-reference.md` |

This document classifies existing feature-flag-like controls in anvil and maps
them onto the shared flagging model defined in `FLAGS`. It is a migration and
operations inventory for controls, not the definitive list of product features
or product feature groups.

Created for: `FLAGS-009`

## Catalogue Boundaries

| Concern                                                                                                   | Authority                            |
| --------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| Product feature groups, product features, delivery surfaces, reviewed exclusions, delivery-key migrations | `flags/surfaces.json` under ADR-076  |
| Operational rollout, entitlement, and kill-switch controls                                                | `flags/manifest.json`                |
| Technical defaults shared by flags                                                                        | `flags/groups.json`                  |
| Evaluation audiences                                                                                      | `flags/audiences.json`               |
| Deployment environments                                                                                   | `flags/environments.json`            |
| Comprehensive human-readable feature views                                                                | Generated from `flags/surfaces.json` |

The legacy `surfaces.json` filename does not narrow its authority to UI or CLI
entry points. A product feature may be available through several delivery
surfaces, and it may be intentionally unflagged. The strict v2 catalogue
separates product feature groups, product features, delivery surfaces, reviewed
`internal-plumbing` exclusions, and explicit retired-source to active-target
delivery-key migrations. Its schema version `2` is independent from operational
flag schema version `1`.

The current catalogue covers CLI, MCP, API, daemon, dashboard, documentation,
hook, and integration delivery hosts. `productCatalogue()` is the authoritative
accessor. The deprecated `flagSurfaces()` accessor projects only the legacy 46
CLI features and is explicitly incomplete; it must not drive completeness,
entitlement, or runtime enforcement. Its frozen v1 fixture is the exact
compatibility payload authority for that deprecated accessor during the
compatibility window, but it is not v2 product, completeness, or enforcement
authority. Those concerns remain canonical in `flags/surfaces.json` through
`productCatalogue()`.

FLAGCAT-012 ships host completeness checks against live CLI, MCP, API, daemon,
dashboard, docs, hook, and integration registries. FLAGCAT-013 links operational
flags to product features. The generated feature view is
[product-feature-catalogue.md](./product-feature-catalogue.md). FLAGCAT-015
remains responsible for any approved product-tier mapping. FLAGCAT-011 does not
add runtime cascade-off or catalogue-derived host enforcement. See
[ADR-076](../../plans/decisions/076-feature-catalogue-surface-registry.md), the
[host-completeness contract](../../plans/specs/2026-08-23-product-catalogue-host-completeness.md),
and the
[v2 physical schema](../../plans/specs/2026-08-23-product-catalogue-v2-schema.md)
instead of maintaining a second full feature list here.

Recovery identities are delivery surfaces, not a coarse feature-wide exception.
The pinned floor includes the CLI credential/login/refresh paths, the usable API
login issuance and refresh routes, and documentation-shell login/callback. It
protects those routes only from future catalogue-derived refusal; each host
still applies its own authentication and credential checks.

**Migration status:** All **migrated** controls below are **retired** — the
`FLAGM` module closed under FLAGM-006. Each migrated entry now routes through
the shared resolver as its sole source of truth; the legacy hard-coded checks
and dual-evaluation parity scaffolding have been deleted. Per-control flag keys,
evaluation context, and rollback paths are documented in
[`plans/specs/2026-04-20-feature-flag-migration-design.md`](../../plans/specs/2026-04-20-feature-flag-migration-design.md).

## Adding a Flag

The operational flag catalogue is the **single source of truth for flags**: a
flag is defined once in `flags/manifest.json` and consumed everywhere through
generated/typed accessors — there is no per-surface flag literal to keep in sync
(FLAGCAT-002…005). The per-surface modules in the table below are now thin
consumers of the catalogue (`@eddacraft/anvil-flags-catalogue` re-exports +
host-local evaluation glue), not flag definitions.

To add a flag:

1. **Edit one file** — add a flag entry to `flags/manifest.json` (keep the array
   sorted by `key`). Declare the parts of a feature flag: `key`; `primaryGroup`
   (an id from `flags/groups.json`, which is a flag default group carrying
   default `class` and audiences); audience targeting using canonical
   `flags/audiences.json` ids; behaviour (`variants` + `defaultVariant`); and,
   for a `rollout`, the `flags/environments.json` ids it is enabled in. Declare
   `controlsProductFeatures` (product-feature keys the flag controls; `[]` if
   none). Matching `flagLinkage` on `flags/surfaces.json` must agree.
2. **Regenerate (no hand-syncing)** — TS accessors load the manifest at module
   load via `@eddacraft/anvil-flags-catalogue`; the Rust constants regenerate
   from the same file via the `eddacraft-anvil-kernel-types` `build.rs` on the
   next build.
3. **CI checks it** — `pnpm nx test flags-catalogue` validates the manifest
   against the `groups` / `audiences` / `environments` inventories, the JSON-key
   → Rust/TS naming map, and bidirectional FLAGCAT-013 linkage;
   `cargo test -p eddacraft-anvil-kernel-types` confirms the generated Rust
   constants are byte-equal to the manifest. Between them, manifest ↔ TS ↔ Rust
   can't drift.

## Classification Key

| Action       | Meaning                                                                                      |
| ------------ | -------------------------------------------------------------------------------------------- |
| **migrated** | Retired onto the shared manifest and OpenFeature-backed resolution                           |
| **adopt**    | New capability — will use the shared model from the start                                    |
| **defer**    | Not ready to migrate yet; document what a future migration involves                          |
| **orphaned** | Defined in the manifest but with no runtime consumer — adopt it somewhere or retire the flag |

## Summary Table

| Control                          | Location                                      | Classification | Flag class    | Mechanism         |
| -------------------------------- | --------------------------------------------- | -------------- | ------------- | ----------------- |
| CLI licence-gated actions        | `crates/anvil-cli/src/feature_flags.rs`       | migrated       | `entitlement` | default/target    |
| Docs access gating               | `apps/docs-shell/lib/feature-flags.ts`        | migrated       | `entitlement` | targeting         |
| `ANVIL_DEV=1` auth bypass        | `crates/anvil-cli/src/feature_flags.rs`       | migrated       | `entitlement` | local override    |
| `ADMIN_KEY` admin gating         | `apps/anvil-api/src/middleware/admin-auth.ts` | defer          | `entitlement` | —                 |
| API access scopes                | `apps/anvil-api/src/lib/feature-flags.ts`     | migrated       | `entitlement` | default           |
| Warm-graph persistence           | `crates/anvil-graph-cache/src/snapshot.rs`    | adopt          | `rollout`     | default/opt-out   |
| Policy profiles                  | `crates/anvil-policy/src/profiles.rs`         | defer          | —             | —                 |
| Per-policy enabled/disabled      | `crates/anvil-policy/src/config.rs`           | defer          | —             | —                 |
| OPA agent orchestration rollout  | (no flag yet)                                 | defer          | `rollout`     | —                 |
| Tier-based product capabilities  | (no flag yet)                                 | adopt          | `entitlement` | —                 |
| Web dashboard (`--web`)          | `crates/anvil-cli/src/feature_flags.rs`       | migrated       | `rollout`     | default-off / env |
| Dashboard AI builder             | (no flag yet)                                 | adopt          | `rollout`     | —                 |
| Tutorial / advanced TUI surfaces | (no flag yet)                                 | adopt          | `rollout`     | —                 |

## Retired Controls — Migrated

The four controls below were migrated onto the shared resolver across FLAGM-002
through FLAGM-005 and closed out in FLAGM-006. Each control's legacy hard-coded
check and dual-evaluation parity scaffolding have been deleted; the shared flag
is now the sole source of truth.

### Web dashboard (`anvil dashboard --web`) — Migrated (DASH-012 gate)

- **Resolver location:** `crates/anvil-cli/src/feature_flags.rs` —
  `web_dashboard_access_allowed()`; wired from
  `crates/anvil-cli/src/commands/dashboard/mod.rs` before the browser server
  starts.
- **Flag key:** `dashboard.web` (class: `rollout`, group: `dashboard`).
- **Current state:** Default-off for the `v0.10.0-beta` cut. Session opt-in via
  `ANVIL_DASHBOARD_WEB=1` or `ANVIL_DEV=1`; `ANVIL_DASHBOARD_WEB=0` forces off
  even under `ANVIL_DEV`. Terminal `anvil dashboard` TUI surfaces are not gated.
- **Review:** `expiryOrReviewDate` 2026-10-31 — re-evaluate default-on after UX
  hardening (overview message truncation, retained history).

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

### Docs access gating — Migrated (FLAGM-004, adopted by FLAGCAT-016)

- **Resolver location:** `apps/docs-shell/lib/feature-flags.ts` —
  `evaluateDocsAccess()`; called by `verifyLicense()` in
  `apps/docs-shell/lib/jwt.ts` after signature, issuer, audience, subject, and
  trusted plan-claim validation.
- **Flag key:** `docs.access` (class: `entitlement`).
- **Current state:** The live docs shell resolves `DOCS_ACCESS_FLAG` via
  `resolveFlag` from `@eddacraft/anvil-runtime/feature-flags`. It grants only
  the boolean `enabled` result, so missing or unknown plans and evaluation
  failures deny access. The former local entitled-plan set has been deleted.
- **Evaluation context:** Canonical `accountTier` from the authenticated,
  SEC-012-resolved plan claim; deployment environment from
  `VERCEL_ENV`/`NODE_ENV`; constant non-PII targeting key `docs-shell`.
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

- **Current state:** Individual entitlement flags exist, but there is no
  approved, comprehensive product-feature-to-plan mapping. FLAGCAT-015 remains
  Draft until that product boundary is approved.
- **Classification:** **adopt**
- **Target flags:** Individual `entitlement` class flags per gated capability,
  each with `accountTier`/`licencePlan` targeting.
- **Featureboard swap impact:** Provider replacement only, assuming Featureboard
  supports the same evaluation context dimensions.

### Web dashboard capabilities

- **Current state:** The web-dashboard launch has the `dashboard.web` rollout
  flag documented above, and terminal dashboard features ship. Per-view product
  packaging and entitlement mapping are not yet defined; those depend on the
  completed ADR-076 registry rather than a dashboard-only list.
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

## Shipped Controls — Adopt

### Warm-graph persistence (`daemon.persist-graph` / `ANVIL_PERSIST_GRAPH`)

- **Current state:** Shipped and **default-on** (graduated by GBASE-010 after
  the ADR-105 §11 gate). The save-time daemon persists and warm-restarts its
  resident graph from a shared, content-addressed base snapshot plus
  per-worktree overlays.
- **Classification:** **adopt** — `rollout` class, boolean.
- **Flag key / manifest:** `daemon.persist-graph` in `flags/manifest.json`
  (`defaultVariant: enabled`); the code default is
  `anvil_graph_cache::snapshot::persist_graph_enabled`, and a test asserts the
  two agree.
- **Semantics:** absence of the variable, an affirmative
  (`1`/`true`/`yes`/`on`), an empty value, or an unparseable value all resolve
  **on**. Only an explicit **opt-out** — `0`/`false`/`no`/`off` (trimmed,
  case-insensitive) — disables.
- **Rollback:** `ANVIL_PERSIST_GRAPH=0`, set in the **daemon's spawn
  environment** (a login-shell rc does not reach a systemd-user- or IDE-launched
  daemon; use `systemctl --user set-environment` / an `Environment=` line, or
  the IDE/session environment). Persistence also requires a resolvable state
  dir, so no state dir ⇒ off regardless.
- **Where bases live:** `<state-dir>/graph-cache/base` (one write-once artefact
  per repo per merge-base commit). Reclaimed by refcount GC over registered
  worktrees; on-demand relief via `anvil graph-base gc [--purge-all]`.
- **Privacy:** identity-only sealed snapshots (names, import/path identity,
  edges, content hashes) — never source text; machine-local `0600` under `0700`.

## Excluded from Inventory

This section excludes controls from the _operational flag_ inventory. It is
distinct from the canonical product catalogue's `excludedDeliverySurfaces[]`,
which is limited to reviewed internal plumbing with a stable delivery identity,
APS-module owner, reason, and review reference.

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
