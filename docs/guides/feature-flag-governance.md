# Feature Flag Governance

| Type  | Authority     | Owner   | Status | Freshness                                                                                                                           |
| ----- | ------------- | ------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | FLAGCAT | Live   | Last reviewed 2026-05-25 against `plans/modules/feature-flag-catalogue.aps.md` and `crates/anvil-kernel-types/src/feature_flags.rs` |

| Upstream                                                                                                                                                        | Downstream                                                                                                                      |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `plans/modules/feature-flag-catalogue.aps.md`, `packages/anvil/contracts/src/schemas/feature-flags.schema.ts`, `crates/anvil-kernel-types/src/feature_flags.rs` | `AGENTS.md`, `docs/guides/feature-flag-reference.md`, `docs/guides/feature-flag-inventory.md`, account plan/entitlements (BACT) |

This guide defines the operational rules for creating, rolling out, promoting,
disabling, and retiring feature flags in Anvil.

**Account entitlements:** commercial / cohort access is modelled as catalogue
flags of class `entitlement` targeted via plan-axis audiences (`plan-beta`, …).
Account rows carry a durable `plan` name that evaluation context uses — see
[ADR-121](../../plans/decisions/121-account-plan-activity-and-flag-entitlements.md)
and
[account plan, activity, and entitlements](./account-plan-activity-and-entitlements.md).
Do not invent free-form feature lists on the user row.

## Flag Lifecycle

```
draft → active → retiring → retired → (runtime use removed)
```

| Status     | Meaning                                                                         |
| ---------- | ------------------------------------------------------------------------------- |
| `draft`    | Defined in the manifest but not evaluated at runtime                            |
| `active`   | Evaluated normally — targeting and overrides apply                              |
| `retiring` | Still evaluated but scheduled for removal; no new targeting rules               |
| `retired`  | Resolves to default only — ready for runtime-code removal; key remains reserved |

## Creating a Flag

Every production flag must define:

| Field                | Purpose                                                |
| -------------------- | ------------------------------------------------------ |
| `key`                | Lowercase dot/hyphen/underscore-separated identifier   |
| `owner`              | APS module code responsible for the flag               |
| `intent`             | One sentence explaining why the flag exists            |
| `class`              | `rollout`, `entitlement`, or `ops_kill_switch`         |
| `defaultVariant`     | The variant used when no targeting or override matches |
| `createdFor`         | The APS work item that introduced the flag             |
| `status`             | Initial status is `draft` until the rollout begins     |
| `expiryOrReviewDate` | Required for `rollout` class; recommended for others   |

### Class selection

- **`rollout`** — temporary progressive enablement. Must have an expiry or
  review date. Expected to reach 100% and be removed.
- **`entitlement`** — tier, plan, or licence gating. May be long-lived but still
  needs an owner and periodic review.
- **`ops_kill_switch`** — emergency disable for operational safety. Always
  active, rarely toggled.

## Per-Track Flags (Track 3 / Track 4)

Language & Coverage governance surfaces (Track 3) and semantic packs (Track 4)
ship behind a shared per-track flag taxonomy (OPSUP-005) so a user can disable a
noisy surface or pack without rolling back the whole release.

### Naming

Flags are hierarchical under two umbrella namespaces, each resolving to a
`flags/groups.json` group of the same shape:

| Namespace         | Umbrella flag   | Group           | Gates                       |
| ----------------- | --------------- | --------------- | --------------------------- |
| `track.surface.*` | `track.surface` | `track-surface` | Track 3 governance surfaces |
| `track.pack.*`    | `track.pack`    | `track-pack`    | Track 4 semantic packs      |

The umbrella flag gates the whole track. A surface or pack adds a per-leaf
override (e.g. `track.surface.sql`, `track.pack.pulumi`) **only when it needs
independent control** — this keeps the flag count from exploding
one-per-surface. Each leaf flag carries `createdFor` pointing at its owning
surface/pack work item and resolves to the same umbrella group as its namespace.

### Default-state policy

Each new track surface or pack ships **opt-in** (`defaultVariant` resolves to
`disabled`) for one release. Reviewers verify the opt-in default when the leaf
flag is first added. After a clean release with no false-positive regressions,
the default flips on by changing `defaultVariant` to `enabled` in a follow-up
change. Every per-track flag is `rollout` class and therefore carries an
`expiryOrReviewDate`.

The Rust invariant `FeatureFlagDefinition::track_flag_violations` enforces the
**permanent** contract for any `track.*` flag in the manifest — `rollout` class,
a sunset date, `createdFor` provenance, and umbrella-group resolution — and the
catalogue test `per_track_flags_obey_opsup_005_taxonomy` runs it across the
shipped manifest. It deliberately does **not** pin the default variant, so the
opt-in→enabled flip is permitted without tripping CI; the opt-in-at-first-ship
expectation is a review-time check, not a manifest-wide invariant.

## Rollout Policy

### Progressive enablement

1. Start with `draft` status and targeting rules scoped to
   `local`/`development`.
2. Promote to `active` when the feature is ready for broader testing.
3. Add environment targeting in order: `development` → `preview` → `demo` →
   `production`.
4. Use percentage rollout within each environment to control blast radius.
5. Monitor telemetry between each promotion step.

### Environment promotion order

```
local → development → preview → demo → production
```

Within `production`, use channels and deployment rings for further control:

```
development → beta → production
```

### Percentage rollout

- Start at 1–5% in the target environment.
- Observe for at least one evaluation cycle before increasing.
- Double the percentage at each step (5% → 10% → 25% → 50% → 100%).
- The hash-based bucketing is deterministic — the same targeting key always
  resolves the same way.

### Audience targeting

Use audience dimensions for beta/tier access:

- `accountTier` — gate by subscription tier
- `licencePlan` — gate by licence level
- `organisationId` — gate by specific organisation
- `userRole` — gate by role within the organisation
- `cohort` — gate by named cohort (e.g. `early-adopter`, `beta`)

## Kill Switch

### When to use

Use `ops_kill_switch` class flags for:

- Features with known failure modes in production
- Features that interact with external services
- High-risk rollouts that need instant rollback

### Emergency disable procedure

1. Add an emergency override to the snapshot or override source:
   ```json
   { "emergency": { "feature.key": "disabled" } }
   ```
2. The override takes highest precedence — above local overrides and targeting.
3. No code change or redeployment is required.
4. Document the incident and the override in the flag's manifest description.

### Fail-closed policy

Kill switch and entitlement flags **fail closed**:

- If the default variant is missing, resolution returns an error with
  `__fail_closed` variant.
- If an override references a nonexistent variant, fail-closed classes return an
  error rather than silently falling through.
- Rollout flags fail open (disabled state) by default.

## Retiring a Flag

### When to retire

- The rollout has reached 100% in all environments and is stable.
- The entitlement is no longer needed (e.g. feature is now universally
  available).
- The kill switch protected a migration that is now complete.

### Retirement steps

1. Set status to `retiring` in the manifest. This signals that no new targeting
   rules should be added.
2. Verify all consumers reference the expected stable variant.
3. Set status to `retired`. The flag now resolves to its default variant only.
4. Remove runtime references and targeting code. Do not reuse the manifest
   `key`; ADR-041 treats it as the historical usage join key.
5. Close the originating APS work item if not already closed.

### Key retention for historical queries

The flag `key` is the stable join key for usage analytics (ADR-041):

- Display names, accessors, owners, intent text, and documentation can be
  renamed without changing `key`.
- Changing `key` creates a new logical flag, not a refactor of the old flag.
- Retired keys remain reserved forever and must not be reused.
- If a retired definition is removed from the active manifest after retention
  allows it, keep an explicit migration note mapping old key to new key or
  stating that the key ended with no replacement.

### Preventing flag rot

- `rollout` flags **must** have an `expiryOrReviewDate` as their sunset trigger.
  A flag past its expiry triggers a review action in the owning module.
- Periodic manifest audits should identify flags with no recent targeting
  changes or evaluation activity.
- The `retiring` status exists specifically to make the intent visible in
  review: if a flag is still `active` past its review date, it needs attention.

## Snapshot Refresh

Flag state is delivered to runtimes via versioned snapshots. Operational
considerations:

- Default freshness window is 300 seconds (5 minutes).
- Runtimes should refresh asynchronously and not block on a fresh snapshot.
- If the snapshot is missing or stale beyond tolerance, fail-closed classes deny
  access and rollout flags resolve to their default.
- Snapshot version is monotonically increasing — consumers can detect rollbacks.

### Refresh failure policy

When a snapshot refresh attempt fails:

1. Continue serving the last-known-good snapshot until `maxAgeSec` is exceeded.
2. Emit a telemetry event with the failure reason on every failed refresh.
3. Once staleness exceeds `maxAgeSec`, fail-closed classes deny access and
   rollout flags resolve to their default variant.
4. Use atomic file replacement (write to temp file, then rename) to avoid
   partial-write corruption of the on-disk snapshot.

### Clock skew tolerance

Snapshot freshness depends on wall-clock alignment between the publisher and
consumers. The system assumes NTP synchronisation within ±30 seconds.

- A snapshot with `issuedAt` more than 60 seconds in the future (relative to the
  consumer's clock) is rejected as stale to prevent a fast-clock publisher from
  issuing "forever fresh" snapshots.
- Operators should verify NTP configuration on all hosts that publish or consume
  snapshots.

### Concurrent writes

Snapshot publication must be serialised through a single writer process. If
multiple writers attempt concurrent publication, version ordering becomes
unreliable. In horizontally scaled deployments, use an external coordination
mechanism (database sequence, Redis INCR, or compare-and-swap on version) to
guarantee monotonicity.

## Observability

- Session start emits minimal OTEL usage metrics (snapshot version, environment,
  runtime) with no PII.
- Feature usage metrics are emitted on first evaluation per session.
- Detailed evaluation traces are available on demand for debugging.
- Override application is logged so operators can verify kill switch state.

## Review Gates

- New flags require review in the PR that introduces them.
- Flag class changes (e.g. promoting from `rollout` to `entitlement`) require
  explicit review.
- Flags past their `expiryOrReviewDate` are flagged in periodic audits.
- Council review should verify that flag retirement steps are followed before
  the manifest entry is deleted.
- For existing controls being migrated onto the shared model, see the inventory
  and migration classifications in `docs/guides/feature-flag-inventory.md`.
