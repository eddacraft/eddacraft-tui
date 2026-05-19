# ADR-048: Feature Group architectural model

## Status

Accepted

## Date

2026-05-19

## Context

FLAGS shipped the resolver, manifest schema, and snapshot loader. FLAGM
migrated five ad-hoc gates onto the shared resolver. FLAGCAT (in progress) is
landing `flags/manifest.json` and codegen so the five definitions live in one
place instead of three.

What none of those modules answered is **how new gating decisions are
structured going forward**. Today every flag is a free-standing entry with
ad-hoc targeting; there is no group concept, no canonical audience inventory,
and no default-class policy. That ambiguity blocks the operator from
authoring new gates with confidence — every new gate re-litigates "what tier
does this belong to," "is this fail-open or fail-closed," "can ops kill it."

FeatureBoard (which we plan to adopt once it ships OpenFeature support) uses
a `Feature → categoryIds[]` model: features carry many-to-many category
references, categories are first-class entities, audiences are a separate
inventory, and audience exceptions are the targeting mechanism. We want our
shape to translate to FB by configuration rather than rewrite.

Three architectural questions need pinning before the operator can start
gating new features:

1. What does a "Feature Group" *do* — pure tag, defaults carrier, or
   authoritative policy unit?
2. Is grouping primarily by surface (where the flag lives) or by capability
   (what the flag does)?
3. Where does kill-switch capability live — a flag class, a per-flag opt-in
   field, or a universal runtime channel?

These are model decisions, not inventory or layout decisions. Inventories
(audiences, environments, group taxonomy) and layout (file structure, naming)
are revisable; the architectural shape is not.

## Decision

### D-1: Feature Group is a defaults carrier

A Feature Group is a first-class entity in the catalogue. Each group sets
**default** values for member flags:

- default `class` (`entitlement` or `rollout`)
- default `audiences` (the audience ids member flags can target)
- default lifecycle policy (`status` posture, `expiryOrReviewDate` cadence)

Per-flag override is always allowed. A flag in the `cli` group can declare
`class: rollout` explicitly even though the group default is `entitlement`;
the override is visible in the flag definition and reviewable in diff.

Rejected alternatives:

- **Pure taxonomy** (groups are tags with no semantics). Provides inventory
  but no leverage; every new flag still re-litigates class and audience.
- **Authoritative** (groups are policy units; flags inherit with no
  override). Maximum leverage, maximum rigidity; first edge case forces a
  governance amendment to the group rather than a flag-local fix.
- **Display-only** (groups affect docs/dashboard rendering, not runtime).
  Trivial leverage; nothing prevents drift between display and behaviour.

### D-2: Hybrid taxonomy — surface-primary, capability-secondary

Each flag has exactly one `primaryGroup` (a *surface*: `cli`, `docs`, `api`,
`dashboard`, `ide`, `daemon`, `hook`). The primary group carries defaults
(per D-1).

Each flag may have zero or more `tags` (capability labels: `auth`,
`entitlements`, `rollout`, `dx`, `governance`, `ops`). Tags are taxonomy
only — they do not inherit defaults, they do not constrain targeting, they
exist for filter and discovery.

Surface-primary matches the existing flag-key convention (`cli.licence-gate`,
`docs.access`, `api.scope.*`) and aligns with how operators reason about
features (you discover a CLI feature by running CLI commands, not by reading
a capability taxonomy). Capability-secondary preserves the cross-surface
view (every `entitlements` flag across surfaces is findable as one set).

Rejected alternatives:

- **Capability primary, surface secondary.** Forces capability decisions
  before surface decisions, which inverts how features are actually built
  (you build a CLI command first, then decide whether it is auth or DX).
- **Plan-tier primary** (groups are `tier-pro`, `tier-enterprise`).
  Couples groups to the unresolved licensing/pricing brainstorm; pushes
  audience information into the group identifier rather than the audience
  inventory; breaks down for rollout flags that are not tier-bound.
- **Single axis, no tags.** Loses the cross-surface view; pushes the
  capability question into ad-hoc flag descriptions.

### D-3: Kill-switch is a universal capability via the emergency-override channel

Every flag is killable at runtime via the existing `FlagOverrides.emergency`
channel implemented in
`packages/anvil/runtime/src/feature-flags/resolver.ts`. No per-flag
declaration is required to make a flag killable; no schema field opts a
flag in or out; the operator path is the same for every flag in the
catalogue.

`FlagClass::OpsKillSwitch` remains in the schema enum for back-compat and
for the rare case of a **purpose-built** kill flag (a flag whose sole reason
to exist is being the kill control for something). It is **not** a per-group
default. Group default class is always `entitlement` or `rollout`.

Rejected alternatives:

- **Kill-switch as a class** (current schema's `ops_kill_switch` value as a
  per-group default). Conflates "this flag is a kill control" with "this
  feature has an emergency-disable path." Every entitlement and rollout flag
  needs the second; almost none need the first. Modelling it as a class
  forces a parallel kill-switch flag next to every gated feature.
- **`killable: boolean` field on FeatureFlagDefinition.** Adds a schema
  field for a property that is universally true in the resolver today
  (emergency overrides already apply to every flag). Field would default to
  `true` and rarely be `false`, which is the textbook smell of a field that
  should not exist.

## Rationale

The three decisions form one coherent architectural picture: **groups carry
behaviour (D-1), groups are surface-shaped with capability tags overlaid
(D-2), and ops kill-switch is a cross-cutting runtime channel not a group
default (D-3).** They are all "decisions that change the meaning of the
model," not "decisions about what to put in the model."

Changing any of them later would force data migration:

- D-1 reversal (group → pure tag): every flag's group-derived defaults
  must move to the flag itself.
- D-2 reversal (capability primary): every flag's `primaryGroup` rotates;
  inventory documents rewrite; reviewer mental model shifts.
- D-3 reversal (kill-switch as class): every entitlement/rollout flag
  needs an audit to determine whether to add a paired `ops_kill_switch`
  flag.

The inventory itself (the actual audience list, environment list, group
list, default-class table) is revisable without retroactively re-shaping
existing data — adding `plan-team` between `plan-pro` and `plan-enterprise`
or moving `dashboard` from entitlement default to mixed-default is a manifest
edit, not a migration. Those decisions belong in the spec.

### Alternatives Considered

| Option                                                   | Pros                                                                                                              | Cons                                                                                                                                       |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| **Chosen: defaults carrier + hybrid taxonomy + universal kill-switch** | One coherent model; FB-translation is configuration; operator authoring path is mechanical                        | Three coupled decisions; reversing any one is migration-grade                                                                              |
| Pure-tag groups + capability primary + class-based kill  | Minimal new concept (groups are just tags); kill-switch is already an enum value                                  | Zero leverage from groups; every flag re-litigates defaults; kill-switch model forces parallel flags or muddies entitlement/rollout intent |
| Authoritative groups + plan-tier primary + per-flag killable | Maximum gating discipline; tier mapping is explicit; killable bit is visible                                      | First exception forces ADR amendment; group identity is licensing-dependent (unresolved); killable bit defaults true and rarely flips      |

## Consequences

### Positive

- **Operator unblock.** Author of a new gating decision walks a deterministic
  path: pick the surface (primary group → default class and audiences),
  declare the flag, override per-flag only when justified. The seven
  primary groups + nine canonical audiences make the decision space
  finite.
- **Reviewer enforcement is mechanical.** Council and PR reviewers check
  three things: (a) does a new user-visible capability ship with a flag
  entry, (b) is the flag's primary group one of the canonical seven,
  (c) does its audience set draw only from `audiences.json`. None of
  those checks require product judgement; all can be automated.
- **FB migration stays a configuration translation.** Our `primaryGroup` +
  `tags` collapses to FB `categoryIds[]`; our `audiences.json` is FB's
  audience inventory; our `targeting[]` is FB's `audienceExceptions[]`.
  No data shape rewrite required.
- **Kill-switch posture matches existing runtime.** The resolver already
  implements `FlagOverrides.emergency`; the decision pins what already
  works rather than introducing a new path.
- **ops_kill_switch class stays valid for purpose-built kill flags.**
  Schema doesn't break; existing tests don't change.

### Negative

- **Three coupled decisions.** Reversing any one is migration-grade
  (see Rationale). Future amendments need cross-decision impact analysis.
- **Primary-group requirement is friction for cross-surface flags.** A
  single flag that legitimately touches `cli` + `api` + `dashboard` must
  pick one primary group; the others move to tags. The override-defaults
  rule mitigates this, but the choice is forced.
- **Existing five flags need retrofit.** `cli.licence-gate`, `docs.access`,
  `api.scope.{beta,preview,internal}` must declare `primaryGroup` in
  `flags/manifest.json`. FLAGCAT-002 absorbs this; no behaviour change.
- **Audience inventory governance becomes load-bearing.** Adding or renaming
  an audience now affects every flag that targets it. Retired audiences
  must follow the ADR-041 reservation rule (no reuse).
- **`ops_kill_switch` class value becomes semi-vestigial.** Reserved for
  rare purpose-built kill flags; no group defaults to it. Some readers will
  expect kill-switch behaviour to flow through it.

## Related decisions

- [ADR-019](019-flags-observability-alignment.md) — Flag observability
  alignment; this decision does not change ADR-019's gate-affecting-only
  rule for standalone Kindling flag facts.
- [ADR-041](041-flag-snapshot-usage-join-contract.md) — Manifest `key` as
  stable join key; this decision is additive (adding `primaryGroup`/`tags`
  is not a `key` change and does not affect the join contract).
- [ADR-035](035-three-pipe-observability-rule.md) — Three-pipe
  observability; kill-switch decisions land on Kindling (governance facts),
  not tracing.

## Implementation

FLAGCAT-002 absorbs the schema additions (`primaryGroup`, `tags`) and lands
all four `flags/*.json` files together (`manifest.json`, `groups.json`,
`audiences.json`, `environments.json`). Reviewer enforcement of the day-1
gating policy is documented in
[`plans/specs/2026-05-19-feature-gating-model.md`](../specs/2026-05-19-feature-gating-model.md).
No new APS module is created.
