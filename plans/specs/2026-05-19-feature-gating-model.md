<!-- APS: Design spec for the Anvil Feature Gating Model -->

# Feature Gating Model

Date: 2026-05-19
Module: `FLAGCAT` (extends FLAGCAT-002..-006 scope)
Status: Ready
Architectural pin:
[ADR-048](../decisions/048-feature-group-architectural-model.md)
Coordinates with:
[`plans/specs/2026-04-09-feature-flagging-design.md`](./2026-04-09-feature-flagging-design.md),
[`plans/specs/2026-04-20-feature-flag-migration-design.md`](./2026-04-20-feature-flag-migration-design.md),
[`plans/specs/2026-05-18-feature-flag-catalogue-design.md`](./2026-05-18-feature-flag-catalogue-design.md),
[ADR-019](../decisions/019-flags-observability-alignment.md),
[ADR-041](../decisions/041-flag-snapshot-usage-join-contract.md)

## Goal

Pin the *shape* of the Anvil feature gating system so the operator can start
gating new features today without re-litigating model questions per PR.
Specifically: define the canonical audience and environment inventories,
the primary group taxonomy, the default-class table, the file layout, the
day-1 gating policy, and the FeatureBoard translation table.

ADR-048 pins the architectural shape (defaults carrier, hybrid taxonomy,
universal kill-switch). This spec carries the inventories and policies that
sit on top of that shape and that the operator and reviewer reference each
time a new flag is authored.

Behaviour-preserving: no existing flag changes resolution semantics. The five
shipped flags pick up `primaryGroup` and optional `tags` from this spec; their
keys, defaults, variants, and targeting rules stay byte-identical.

## Context

`docs/guides/feature-flag-inventory.md` documents the five shipped flags. The
FLAGCAT module migrates them onto a single `flags/manifest.json`. The
operator's blocker today is: "I want to start properly gating features, but
the shape of the model isn't pinned, so I cannot start." This spec is the
shape.

FeatureBoard is the planned migration target once it ships OpenFeature
support. FB's model is `Feature → categoryIds[] → audienceExceptions[]` with
separate inventories for audiences and environments
(reviewed against the FeatureBoard SDK's
`libs/code-generator/src/lib/api/get-project-features.ts` and the FeatureBoard
docs `terminology.md`; both held locally in sibling clones during design, not
mirrored into this repo).
The spec's translation table at the end keeps FB adoption a configuration
change.

## Canonical inventories

### Audiences (`flags/audiences.json`)

Nine audiences in four axes (four plans, two customer roles, one
Anvil-staff bit, two channels). Audiences are OR-ed when a flag references
multiple — any match grants the targeting rule.

```jsonc
{
  "schemaVersion": 1,
  "audiences": [
    // Plans — customer licence tier
    { "id": "plan-free",           "name": "Free plan",       "axis": "plan",    "status": "active" },
    { "id": "plan-beta",           "name": "Beta plan",       "axis": "plan",    "status": "active" },
    { "id": "plan-pro",            "name": "Pro plan",        "axis": "plan",    "status": "active" },
    { "id": "plan-enterprise",     "name": "Enterprise plan", "axis": "plan",    "status": "active" },

    // Customer roles — role within the customer's own organisation
    { "id": "role-admin",          "name": "Customer admin",     "axis": "role", "status": "active" },
    { "id": "role-developer",      "name": "Customer developer", "axis": "role", "status": "active" },

    // Anvil staff — internal-only audience, separate axis from customer role
    { "id": "staff-anvil-internal","name": "Anvil staff",        "axis": "staff","status": "active" },

    // Rollout channels — exposure stage
    { "id": "channel-early-access","name": "Early access channel","axis": "channel","status": "active" },
    { "id": "channel-general",     "name": "Generally available", "axis": "channel","status": "active" }
  ]
}
```

Axes are descriptive metadata, not enforced disjointness — a flag may target
audiences from multiple axes (e.g., `[plan-pro, staff-anvil-internal]` =
"either a Pro customer or any Anvil staff member"). The `axis` field exists
for documentation and dashboard grouping.

**Renames and retirements** follow the same rule as flag keys per ADR-041:
retired audience ids stay reserved forever; renaming an id creates a new
logical audience and the old id remains in the manifest with
`status: "retired"` until retention expires.

### Environments (`flags/environments.json`)

Seven environments. Environment is single-valued per evaluation (a session
is in exactly one environment); environment matching uses equality, not OR.

```jsonc
{
  "schemaVersion": 1,
  "environments": [
    { "id": "local",      "name": "Local development",       "status": "active" },
    { "id": "dev",        "name": "Dev cluster / branch",    "status": "active" },
    { "id": "test",       "name": "Test / QA cluster",       "status": "active" },
    { "id": "preview",    "name": "Vercel preview deploys",  "status": "active" },
    { "id": "staging",    "name": "Pre-production staging",  "status": "active" },
    { "id": "demo",       "name": "Customer demo deploys",   "status": "active" },
    { "id": "production", "name": "Production",              "status": "active" }
  ]
}
```

**Rename of `prod` → `production`.** The current `EnvironmentNameSchema`
enum value is `prod`; this spec renames it to `production` for FB-compat
(FB and FeatureBoard SDK use `production` consistently). No flag *targeting
rule* references `prod` today — environment lives in `EvaluationContext`,
not in flag definitions — so the rename does not change any flag's
resolution semantics. It does, however, touch every runtime construction
site of `EnvironmentName::Prod` (CLI `cli_evaluation_context`, kernel
resolver evaluation paths, and their tests in
`crates/anvil-kernel-types/src/feature_flags.rs` and
`crates/anvil-kernel/src/feature_flags/resolver.rs`). FLAGCAT-002 amends
`EnvironmentNameSchema`, renames the Rust enum variant
`EnvironmentName::Prod` → `EnvironmentName::Production`, and updates the
construction sites in the same change.

### Primary groups (`flags/groups.json`)

Seven primary groups, each with a default class and default audience set.
Member flags inherit defaults unless they override per-flag.

```jsonc
{
  "schemaVersion": 1,
  "groups": [
    {
      "id": "cli",
      "name": "CLI surface",
      "defaultClass": "entitlement",
      "defaultAudiences": ["plan-pro", "plan-enterprise"],
      "defaultStatus": "active"
    },
    {
      "id": "docs",
      "name": "Documentation surface",
      "defaultClass": "entitlement",
      "defaultAudiences": ["plan-beta", "plan-pro", "plan-enterprise"],
      "defaultStatus": "active"
    },
    {
      "id": "api",
      "name": "Anvil API surface",
      "defaultClass": "entitlement",
      "defaultAudiences": ["plan-pro", "plan-enterprise"],
      "defaultStatus": "active"
    },
    {
      "id": "dashboard",
      "name": "Web dashboard surface",
      "defaultClass": "entitlement",
      "defaultAudiences": ["plan-pro", "plan-enterprise"],
      "defaultStatus": "active"
    },
    {
      "id": "ide",
      "name": "Editor / IDE surface",
      "defaultClass": "rollout",
      "defaultAudiences": ["channel-early-access", "channel-general"],
      "defaultStatus": "active"
    },
    {
      "id": "daemon",
      "name": "Anvil daemon (internal)",
      "defaultClass": "rollout",
      "defaultAudiences": ["staff-anvil-internal", "channel-general"],
      "defaultStatus": "active"
    },
    {
      "id": "hook",
      "name": "Commit / push hook surface",
      "defaultClass": "rollout",
      "defaultAudiences": ["channel-general"],
      "defaultStatus": "active"
    }
  ]
}
```

The default audience sets above are suggested starting points, not policy
ADR pins. A future tier-matrix decision (the in-flight licensing/pricing
brainstorm) may adjust these without an ADR-048 amendment.

### Tags (open set)

Tags are a free-form string list on each flag. Suggested initial vocabulary:

- `auth` — authentication / licence-gating
- `entitlements` — tier-bound capability
- `rollout` — gradual exposure of new behaviour
- `dx` — developer experience experiment
- `governance` — policy / witness / baseline gates
- `ops` — kill switches, operator overrides

Tag drift is acceptable; the manifest validates uniqueness within a flag
entry but does not constrain the vocabulary. If tags proliferate into
noise, a future spec amendment can pin a canonical tag list — the schema
change to constrain them later is non-breaking.

## Schema additions

Additive changes to `FeatureFlagDefinitionSchema` in
`packages/anvil/contracts/src/schemas/feature-flags.schema.ts`:

```ts
export const FeatureFlagDefinitionSchema = z.object({
  // ── existing fields (unchanged) ──
  key: z.string(),
  owner: z.string(),
  intent: z.string(),
  class: FlagClassSchema,
  valueType: FlagValueTypeSchema,
  variants: z.array(FlagVariantSchema),
  defaultVariant: z.string(),
  status: FlagStatusSchema,
  createdFor: z.string(),
  expiryOrReviewDate: z.string().optional(),
  description: z.string().optional(),
  targeting: z.array(TargetingRuleSchema).optional(),

  // ── new fields (this spec) ──
  primaryGroup: z.string(),        // required; must match an id in groups.json
  tags: z.array(z.string()).optional(),
});
```

New manifest schemas (`FlagGroupManifestSchema`, `FlagAudienceManifestSchema`,
`FlagEnvironmentManifestSchema`) sit alongside `FeatureFlagManifestSchema` in
the same file.

Cross-manifest validation rules (enforced by the FLAGCAT-006 consistency
check):

1. Every flag's `primaryGroup` exists in `groups.json`.
2. Every audience id referenced in flag `targeting[].value` (when the
   targeting rule operates on a **canonical-audience attribute**) exists in
   `audiences.json`. The validation walks `TargetingCondition` entries with
   `attribute` matching `accountTier`, `licencePlan`, `userRole`, or
   `cohort`; values referenced under those attributes must subset the
   audience inventory. `organisationId` is **explicitly excluded** — it is
   a free-form per-tenant identifier (e.g. `org-123`), not a canonical
   audience id, and validating it against the inventory would break
   legitimate per-tenant targeting.
3. Every environment id referenced in flag targeting exists in
   `environments.json`. Same walk, attribute matching `environment`.
4. Group `defaultAudiences[]` ids must exist in `audiences.json`.
5. Retired ids in any manifest are never reused for a new active id (ADR-041
   key-reservation rule, generalised to audiences/groups/environments).

### Migration of existing bare-value targeting

Shipped flags currently target on bare tier values rather than canonical
audience ids. For example, `docs.access` targets
`accountTier in_set ['beta', 'pro', 'enterprise']`, not
`['plan-beta', 'plan-pro', 'plan-enterprise']`. FLAGCAT-002 ships the
following migration when it lands the inventories:

| Bare value (today) | Canonical audience id (after FLAGCAT-002) |
| ------------------ | ----------------------------------------- |
| `beta`             | `plan-beta`                               |
| `pro`              | `plan-pro`                                |
| `enterprise`       | `plan-enterprise`                         |
| `free`             | `plan-free`                               |

The mapping is mechanical and behaviour-preserving — the underlying
`accountTier` attribute on `AudienceContext` remains a string, but the
*values* that targeting rules reference move from bare tier names to the
canonical audience ids. Callers building `AudienceContext` (e.g.
`cli_evaluation_context` in `crates/anvil-cli/src/feature_flags.rs`) pick
up the new values via the catalogue accessors and pass them through
unchanged.

`organisationId` retains its free-form per-tenant value space (e.g.
`org-123`); no migration applies.

## File layout

```text
flags/
  manifest.json         # flag definitions (FLAGCAT-002 target — gains
                        # primaryGroup + tags)
  groups.json           # primary groups + defaults
  audiences.json        # canonical audience inventory
  environments.json     # canonical environment inventory
  .openfeature.yaml     # optional OF/FB generator config
```

FLAGCAT-001's design note pinned `flags/manifest.json`; this spec extends the
directory with three sibling files. The sibling-file layout (vs one combined
manifest) keeps diffs scoped to the axis being changed — a PR adjusting the
audience list does not churn the flag manifest.

## TS loader package (extends FLAGCAT-002)

The `@eddacraft/anvil-flags-catalogue` package adds three accessors alongside
the flag accessors pinned in the FLAGCAT-001 design note:

```ts
// packages/anvil/flags-catalogue/src/index.ts

// Existing (from FLAGCAT-001 design note)
export function featureFlagManifest(): FeatureFlagManifest;
export function flagByKey(key: string): FeatureFlagDefinition;
export const CLI_LICENCE_GATE: FeatureFlagDefinition;
// …other existing accessors…

// New (this spec)
export function flagGroupsManifest(): FlagGroupManifest;
export function flagAudiencesManifest(): FlagAudienceManifest;
export function flagEnvironmentsManifest(): FlagEnvironmentManifest;

export function groupById(id: string): FlagGroupDefinition;
export function audienceById(id: string): FlagAudienceDefinition;
export function environmentById(id: string): FlagEnvironmentDefinition;

// Typed id unions, generated from manifest content at module load.
export type FlagGroupId = 'cli' | 'docs' | 'api' | 'dashboard' | 'ide' | 'daemon' | 'hook';
export type FlagAudienceId =
  | 'plan-free' | 'plan-beta' | 'plan-pro' | 'plan-enterprise'
  | 'role-admin' | 'role-developer'
  | 'staff-anvil-internal'
  | 'channel-early-access' | 'channel-general';
export type FlagEnvironmentId =
  | 'local' | 'dev' | 'test' | 'preview' | 'staging' | 'demo' | 'production';
```

## Rust codegen (extends FLAGCAT-001)

The FLAGCAT-004 `build.rs` extends to read `groups.json`, `audiences.json`,
and `environments.json` from the workspace root alongside `manifest.json`.
Generated output adds three modules:

```rust
// generated — do not hand-edit
pub mod groups {
    pub mod cli {
        pub const ID: &str = "cli";
        pub const DEFAULT_CLASS: crate::FlagClass = crate::FlagClass::Entitlement;
        // …
    }
    // …docs, api, dashboard, ide, daemon, hook…
    pub const ALL_IDS: &[&str] = &["api", "cli", "daemon", "dashboard", "docs", "hook", "ide"];
}

pub mod audiences { /* same shape, ALL_IDS sorted */ }
pub mod environments { /* same shape, ALL_IDS sorted */ }
```

The naming map (`.`/`-` → `_`) from FLAGCAT-001 applies: `plan-pro` →
`audiences::plan_pro`, `staff-anvil-internal` → `audiences::staff_anvil_internal`.

## Day-1 gating policy

Every new **user-visible capability** that lands in the repo must ship a
matching entry in `flags/manifest.json` in the same PR. Reviewer-enforced;
Council includes the check in its quick-pass criteria.

### What counts as user-visible

- new CLI command or subcommand on `anvil` or `anvil-run`
- new HTTP route on `apps/anvil-api`
- new docs surface gated by `docs.access` or by a sibling docs flag
- new dashboard panel (future)
- new IDE / editor action (future)

### What does not

- internal refactors, performance work, bug fixes that do not change the
  user-visible surface
- new ADRs, plans, schemas, design notes
- test infrastructure, CI changes, build scripts
- typo fixes and documentation edits to existing surfaces
- changes inside an existing flagged feature (the feature is already gated;
  internal changes inherit the existing flag)

### Author checklist

For each new user-visible capability:

1. Pick the **primary group** from `flags/groups.json` (the surface the
   capability lives on).
2. Inherit the group's `defaultClass` and `defaultAudiences` unless the
   capability demands otherwise; if so, override at the flag level and
   record why in the flag's `description` field.
3. Pick a **flag key** following the `<group>.<feature>` convention
   (`cli.audit-export`, `docs.api-reference`, `api.scope.team`,
   `dashboard.usage-trends`).
4. Set `status: "draft"` while in development, `status: "active"` at GA.
5. Set `expiryOrReviewDate` for rollout-class flags (mandatory) and for
   entitlement flags expected to retire (optional).
6. Add zero or more `tags` for cross-surface discovery.
7. If the flag introduces a new audience that is not in `audiences.json`,
   amend `audiences.json` in the same PR with a justification in the PR
   description.

### Reviewer checklist

Council and PR reviewers verify in this order:

1. **Coverage.** Did the PR introduce a new user-visible capability without
   a flag? If yes, block until a flag entry lands.
2. **Group membership.** Is `primaryGroup` one of the canonical seven?
3. **Audience hygiene.** Do all targeting-referenced audience ids exist in
   `audiences.json`? (FLAGCAT-006 consistency check enforces structurally;
   reviewer enforces conceptually.)
4. **Override justification.** Did the flag override the group default? If
   yes, is the reason captured in `description`?
5. **Key naming.** Does the flag key follow `<group>.<feature>` convention?
   (Soft check; deviations allowed with reviewer ack.)

## APS spec convention (advisory)

Module specs may include a `## Gating` section listing flag keys the module
introduces, but this is **not required**. The `flags/manifest.json` entry is
the canonical evidence. The APS spec convention exists so module readers can
see at a glance which flags are in scope; absence of a section is not a
review block.

Recommended `## Gating` shape:

```markdown
## Gating

| Flag key                  | Primary group | Class       | Default audiences          |
| ------------------------- | ------------- | ----------- | -------------------------- |
| `cli.audit-export`        | cli           | entitlement | plan-pro, plan-enterprise  |
| `cli.audit-export.diff`   | cli           | rollout     | channel-early-access       |
```

## FeatureBoard translation table

When FB adopts OpenFeature and Anvil adopts FB, the following structural
mapping applies. The mapping is configuration-level — schema-translation in a
boundary adapter rather than a re-shaping of source files.

| Anvil concept                                | FeatureBoard concept           | Notes                                                                                              |
| -------------------------------------------- | ------------------------------ | -------------------------------------------------------------------------------------------------- |
| `flags/manifest.json` `flags[]`              | `features[]`                   | Direct mapping; our richer governance fields (class, owner, status, createdFor, expiryOrReviewDate) survive as additive metadata FB ignores. |
| flag `primaryGroup` + `tags[]`               | feature `categoryIds[]`        | We collapse to a flat array on translation: `[primaryGroup, ...tags]`. FB does not distinguish primary from secondary. |
| `flags/groups.json` group definitions        | `categories` (Audience+Feature)| FB has a single `Category` table referenced by both audiences and features. Our group `defaultClass` and `defaultAudiences` are Anvil-specific governance metadata; they do not translate. Defaults inheritance happens client-side. |
| `flags/audiences.json`                       | `audiences`                    | FB audiences have `id`, `key`, `name`, `description`, `categoryId`. Our `axis` field maps to FB `categoryId` via a per-axis category. |
| flag `targeting[]`                           | feature `audienceExceptions[]` | Our targeting rules express more (operators, conditions); FB allows boolean per-audience overrides only. Lossy in the FB direction; we fall back to flag-level defaults plus per-audience exceptions. Complex Anvil targeting rules stay client-resolved until FB matches OF expressiveness. |
| `flags/environments.json`                    | `environments`                 | Direct mapping. `production` is the canonical name on both sides. |
| `FlagOverrides.local` / `FlagOverrides.emergency` | FB does not have a runtime override channel | We keep the resolver's override layer client-side; FB sees only the manifest. |
| ADR-041 `key` join key                       | FB `feature.key`               | Stable across FB lifecycle; ADR-041's reservation rules continue to apply. |

The boundary adapter that performs this translation is out of scope for this
spec — when FB ships OpenFeature, we file a follow-up module that consumes
this table.

## Implementation impact on FLAGCAT

FLAGCAT-002, -003, -004, -005, -006 inherit this spec without re-litigating
shape. Concrete additions to each task:

- **FLAGCAT-002 (Bootstrap):** also lands `groups.json`, `audiences.json`,
  `environments.json`. Adds `primaryGroup` and optional `tags` to the five
  shipped flag entries. Renames `EnvironmentNameSchema` `prod` → `production`
  in `packages/anvil/contracts/src/schemas/feature-flags.schema.ts` and the
  Rust `EnvironmentName::Prod` enum.
- **FLAGCAT-003 (TS surface migration):** no extra work — the five flags
  pick up their new fields from the catalogue accessors, not from per-surface
  modules.
- **FLAGCAT-004 (Rust codegen):** `build.rs` reads four files instead of
  one; generated module gains `groups`, `audiences`, `environments`
  sub-modules.
- **FLAGCAT-005 (CLI migration):** unchanged — `cli.licence-gate` picks up
  `primaryGroup: "cli"` automatically via codegen.
- **FLAGCAT-006 (Consistency check):** extends to validate cross-manifest
  references (every `primaryGroup` exists in groups; every audience target
  exists in audiences; etc).

The FLAGCAT module's Acceptance Criteria gains three rows for the new
inventories. The module's Ready Checklist now has its design-spec item
satisfied by *both* the FLAGCAT-001 design note and this spec; an amendment
to the module file records the dual reference.

## Open questions deferred

- **Tier matrix (which plans Anvil sells and what each includes).** Tracked
  by the in-flight
  [licensing/pricing brainstorm](./2026-05-17-licensing-pricing-brainstorming-checklist.md).
  When it lands, group `defaultAudiences[]` may be adjusted. Schema is
  stable across whatever tier matrix lands.
- **Operator runtime override surface.** How an operator flips a flag in
  production today is via the snapshot pipeline; the operator-facing
  command/UI for emergency overrides is not pinned. Future task, separate
  from this spec.
- **Audit trail.** ADR-041 covers per-invocation flag context on USAGE
  rows. "Who flipped what, when" on the manifest side (manifest commit
  history) is currently implicit-in-git; a future task may surface this.

## Acceptance

This spec is satisfied when:

- `flags/groups.json`, `flags/audiences.json`, `flags/environments.json`
  exist and validate against their schemas (FLAGCAT-002).
- The five shipped flags carry `primaryGroup` set to their surface
  (FLAGCAT-002).
- `EnvironmentNameSchema` has `production` (not `prod`); the Rust
  `EnvironmentName::Prod` variant is renamed to `Production` (FLAGCAT-002).
- The cross-manifest consistency check enforces the four validation rules
  in §"Schema additions" (FLAGCAT-006).
- A PR introducing a new user-visible capability without a `flags/manifest.json`
  entry is blocked in review (operational, not validated by the
  consistency check; reviewer training and the section above are the
  controls).
- The day-1 gating policy is referenced from `CLAUDE.md` or `AGENTS.md` so
  reviewers (human and agent) read it before opening a PR.
