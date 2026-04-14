# Feature Flag Inventory

This document classifies existing feature-flag-like controls in Anvil and maps
them onto the shared flagging model defined in `FLAGS`.

Created for: `FLAGS-009`

## Classification Key

| Action      | Meaning                                                             |
| ----------- | ------------------------------------------------------------------- |
| **migrate** | Move onto the shared manifest and OpenFeature-backed resolution     |
| **adopt**   | New capability — will use the shared model from the start           |
| **defer**   | Not ready to migrate yet; document what a future migration involves |

## Summary Table

| Control                          | Location                                | Classification | Target class      |
| -------------------------------- | --------------------------------------- | -------------- | ----------------- |
| CLI licence-gated actions        | `crates/anvil-cli/src/main.rs`          | migrate        | `entitlement`     |
| Docs access gating               | `apps/docs-shell/proxy.ts`              | migrate        | `entitlement`     |
| `ANVIL_DEV=1` auth bypass        | `crates/anvil-cli/src/main.rs:171`      | migrate        | `ops_kill_switch` |
| `ADMIN_KEY` admin gating         | `apps/anvil-api/src/middleware/admin-auth.ts` | defer    | `entitlement`     |
| Beta access scopes               | `apps/anvil-api/src/routes/admin.ts:15` | migrate        | `entitlement`     |
| Policy profiles                  | `crates/anvil-policy/src/profiles.rs`   | defer          | —                 |
| Per-policy enabled/disabled      | `crates/anvil-policy/src/config.rs`     | defer          | —                 |
| OPA agent orchestration rollout  | (no flag yet)                           | defer          | `rollout`         |
| Tier-based product capabilities  | (no flag yet)                           | adopt          | `entitlement`     |
| Web dashboard capabilities       | (no flag yet)                           | adopt          | `entitlement`     |
| Dashboard AI builder             | (no flag yet)                           | adopt          | `rollout`         |
| Tutorial / advanced TUI surfaces | (no flag yet)                           | adopt          | `rollout`         |

## Existing Controls — Migrate

### CLI licence-gated actions

- **Location:** `crates/anvil-cli/src/main.rs` — `requires_auth()` (line 104)
  and `check_auth()` (line 164)
- **Current state:** Rust CLI calls `/api/v1/whoami` to get a plan string;
  access is gated by API 401 responses rather than in-process flag evaluation.
  The `requires_auth()` function hard-codes which commands need authentication
  (Audit, Check, Drift, Status, Admin, Gate, GateConfig, Watch, Export,
  Architecture, Policy, Whoami) and which bypass it (Doctor, Tutorial, Welcome,
  Init, New, Wizard, Hooks, Update, Validate, Login, Logout).
- **Classification:** **migrate**
- **Target flag:** `cli.licence-gate` (class: `entitlement`)
- **Migration path:** Resolve the flag locally via the snapshot-backed provider
  using `licencePlan` and `accountTier` from the evaluation context. The API
  still validates the licence token, but the CLI can make in-process access
  decisions without a round-trip for every gated command.
- **Featureboard swap impact:** Provider replacement only — the evaluation
  context and flag key stay the same.

### Docs access gating

- **Location:** `apps/docs-shell/proxy.ts` (lines 132–142),
  `apps/docs-shell/lib/jwt.ts`
- **Current state:** Proxy middleware checks for a valid JWT in the
  `anvil-docs-session` cookie. It is a binary authenticated/not check with no
  tier or plan awareness. Gated paths: `/anvil/*`. Public paths: `/kindling/*`,
  `/aps/*`, `/edda-stack/*`, `/blog/*`, `/assets/*`, `/img/*`.
- **Classification:** **migrate**
- **Target flag:** `docs.access` (class: `entitlement`)
- **Migration path:** After JWT validation, resolve the `docs.access` flag using
  the authenticated user's `accountTier`. The middleware can then allow or deny
  based on targeting rules (e.g. beta, pro, enterprise) rather than just
  authentication presence.
- **Featureboard swap impact:** Provider replacement only — middleware still
  resolves via the same evaluation context.

### `ANVIL_DEV=1` auth bypass

- **Location:** `crates/anvil-cli/src/main.rs:171`
- **Current state:** When `ANVIL_DEV=1` is set, `check_auth()` returns `Ok(())`
  unconditionally, bypassing all local auth pre-checks. Intended for CLI UX
  testing without a live token. API calls still require real tokens server-side.
- **Classification:** **migrate**
- **Target flag:** `cli.dev-bypass` (class: `ops_kill_switch`)
- **Migration path:** Replace the raw env-var check with a local operator
  override on the `cli.licence-gate` flag. The shared resolver already supports
  local overrides at higher precedence than targeting rules. This preserves the
  existing developer workflow while making the bypass visible in flag telemetry
  and auditable.
- **Featureboard swap impact:** Local overrides are resolved before the provider
  is consulted, so no impact.

### Beta access scopes

- **Location:** `apps/anvil-api/src/routes/admin.ts:15`
- **Current state:** `ALLOWED_SCOPES = ['beta', 'preview', 'internal']` controls
  which scope strings can be assigned to access tokens. Default scope is
  `['beta']`. Scopes are stored in the `access_tokens` table and validated on
  every API call. This is an ad-hoc audience segmentation mechanism.
- **Classification:** **migrate**
- **Target flag:** Per-scope entitlement flags (e.g. `api.scope.preview`,
  `api.scope.internal`) with `accountTier` targeting.
- **Migration path:** Map current scope strings to `accountTier` or `userRole`
  dimensions in the evaluation context. The token table retains scope data, but
  feature gating decisions move to the shared resolver. The `ALLOWED_SCOPES`
  constant becomes derivable from the flag manifest rather than hard-coded.
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
