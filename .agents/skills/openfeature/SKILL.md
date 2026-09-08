---
name: openfeature
description: >
  Implement and operate feature flags with OpenFeature using audience-first
  evaluation context. Use when adding, gating, or removing flags, wiring an
  SDK or provider, or debugging wrong variants.
---

# OpenFeature (Audience-First)

Use this skill for feature-flag work. The default mental model is **audience and
environment gating**, not per-user toggles. OpenFeature is the application-facing
API; a provider or in-process resolver sits behind it.

## When To Use

Invoke when:

- Adding, changing, rolling out, or retiring a feature flag
- Wiring OpenFeature SDK + provider in a service or client
- Designing a flag manifest or catalogue
- Debugging unexpected flag values or targeting
- Migrating env vars or hard-coded checks to flags
- Evaluating flagd, commercial providers, or custom in-process providers

## Core Model

Feature flags gate **who can see what, where** — not individual users.

| Dimension         | Purpose                                         | Examples                                                |
| ----------------- | ----------------------------------------------- | ------------------------------------------------------- |
| **Environment**   | Where the binary runs                           | `local`, `development`, `preview`, `demo`, `production` |
| **Audience**      | Entitlement / access class                      | plan tier, licence, role, cohort, staff channel         |
| **Targeting key** | Stable bucketing id for percentage rollout only | session id, request id — **not** identity               |

Do **not** model flags as `if (userId === 'alice')`. Model them as
`if (audience.plan in ['pro', 'enterprise'] && environment === 'production')`.

See `references/evaluation-context.md` for the full context shape, operators, and
mapping into OpenFeature evaluation context.

## Project Detection

Before changing flags, discover project truth:

| Signal                                                               | Action                                                                  |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `flags/manifest.json` + `flags/{audiences,groups,environments}.json` | anvil-shaped catalogue — follow project `docs/guides/feature-flag-*.md` |
| `.openfeature.yaml` or `flags.json` (OpenFeature CLI schema)         | Use `openfeature generate` accessors; validate against CLI schema       |
| `@openfeature/*` in dependencies                                     | Wire SDK + provider; call sites use OpenFeature API only                |
| `@eddacraft/anvil-runtime/feature-flags` or `anvil-contracts`        | Use project manifest validation and snapshot/resolver helpers           |
| `flagd` in compose/k8s or `@openfeature/flagd-provider`              | Connect provider to flagd; definitions live in flagd sync sources       |
| None of the above                                                    | Introduce audience-first manifest + OpenFeature SDK (see Greenfield)    |

## Router

| Situation                     | Path                                                                                                |
| ----------------------------- | --------------------------------------------------------------------------------------------------- |
| New flag in catalogue project | Edit manifest → validate → regenerate typed accessors → wire evaluation context at boundary         |
| New flag, no catalogue yet    | Create manifest + audience/environment inventories → OpenFeature SDK                                |
| Provider setup                | Choose backend (in-process, flagd, commercial) → register provider once at startup                  |
| Rollout                       | `draft` → env-scoped targeting → percentage within env → promote environments in order              |
| Entitlement gate              | `entitlement` class, fail-closed default, audience attributes from auth boundary                    |
| Emergency disable             | `ops_kill_switch` or emergency override channel — bypasses normal targeting                         |
| Flag cleanup                  | `retiring` → `retired` → remove runtime branches → delete from manifest                             |
| anvil project                 | Defer governance detail to `docs/guides/feature-flag-governance.md` and `feature-flag-reference.md` |

## Flag Classes

| Class             | Default failure mode           | Lifetime   | Use for                                                |
| ----------------- | ------------------------------ | ---------- | ------------------------------------------------------ |
| `rollout`         | Fail to safe default (typically off) | Temporary  | Progressive enablement; requires expiry or review date |
| `entitlement`     | Fail closed (deny)             | Long-lived | Plan, licence, tier, or surface access                 |
| `ops_kill_switch` | Fail closed (deny)             | Permanent  | Emergency disable; rarely toggled                      |

## Manifest Shape (Audience-First)

Prefer a **catalogue** with separate inventories:

```
flags/
  manifest.json       # flag definitions (sorted by key)
  audiences.json      # canonical audience ids (plan, role, staff, channel axes)
  groups.json         # surface groups with default class + default audiences
  environments.json   # deploy environments
```

Each flag should declare at minimum:

- `key` — hierarchical, lowercase (`surface.capability` or `track.surface.sql`)
- `owner` — team or module code
- `intent` — one sentence why the flag exists
- `class` — `rollout`, `entitlement`, or `ops_kill_switch`
- `variants` + `defaultVariant`
- `status` — `draft` | `active` | `retiring` | `retired`
- `createdFor` — work item id for provenance
- `primaryGroup` — surface group id from `groups.json`
- `expiryOrReviewDate` — required for `rollout`

Targeting rules reference **audience and environment attributes**, not user ids.

### Lifecycle

```
draft → active → retiring → retired → (remove runtime use) → (delete from manifest)
```

- `draft` and `retired` resolve to default only
- Do not add new targeting while `retiring`
- Remove `if (flag)` branches from code before deleting the manifest entry

## OpenFeature Integration

### Call-site rule

Application code uses **OpenFeature evaluation API only** — never a vendor SDK or
raw env var at the call site.

```typescript
import { OpenFeature } from "@openfeature/server-sdk";

const client = OpenFeature.getClient();
const enabled = await client.getBooleanValue("docs.access", false, {
  targetingKey: sessionId,
  environment: "production",
  accountTier: "beta", // audience attribute — set at auth boundary
});
```

Map structured context to flat OpenFeature attributes at the provider boundary if
needed. Keep audience fields derived from authenticated session claims, not
looked up per flag evaluation.

### Provider choices

| Backend                                                  | When                                                                                |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **In-process custom provider**                           | Manifest + snapshot resolver; lowest latency; full control over audience semantics  |
| **flagd**                                                | OSS OpenFeature engine; sidecar, central service, or in-process; file/HTTP/K8s sync |
| **Commercial** (LaunchDarkly, ConfigCat, DevCycle, etc.) | Managed UI, audit, experiments — still audience rules in the provider console       |

Register the provider once at process start. Use **domains** only when multiple
providers or isolated client contexts are required.

### Defaults

Always pass an explicit default at evaluation time. The default must match the
manifest `defaultVariant` value. On provider failure, behaviour follows flag
class (fail to safe default — typically off — for rollout, fail closed for
entitlement/kill-switch).

## Practices

### 1. Evaluate at boundaries, not deep in logic

**Wrong:** flag check inside every helper and nested call.

**Right:** resolve once at HTTP handler, CLI command entry, or render boundary;
pass the resolved capability decision inward.

### 2. Audience attributes come from auth, not flag lookups

**Wrong:** fetch user profile inside the provider to decide entitlements.

**Right:** middleware or session layer populates `audience` from token claims /
licence service; evaluation is attribute matching only.

### 3. Percentage rollout uses targetingKey, not userId

**Wrong:** `percentage` keyed on `userId` (ties rollout to identity).

**Right:** `targetingKey` = session or request id; audience rules gate eligibility,
percentage gates blast radius within the eligible audience.

### 4. One manifest, typed accessors

**Wrong:** string literal flag keys scattered across codebases.

**Right:** single manifest; generate or re-export typed accessors (OpenFeature CLI,
build script, or shared package). CI validates manifest ↔ codegen parity.

### 5. No PII in flag telemetry

Emit `flagKey`, `variant`, `reason`, `environment` — not emails, names, or raw
user ids.

### 6. Retire rollouts on schedule

Rollout flags must have `expiryOrReviewDate`. When stable at 100%, move to
`retiring`, then `retired`, then delete code paths.

## Greenfield Setup

When no flag system exists:

1. Create `flags/audiences.json` and `flags/environments.json` first — agree axes
   (`plan`, `role`, `channel`, `staff`) before writing flags.
2. Add `flags/groups.json` for product surfaces with default class and audiences.
3. Add `flags/manifest.json`; start new flags as `draft`.
4. Install OpenFeature SDK for the runtime language.
5. Implement or choose a provider:
   - small service: in-process resolver over manifest snapshot
   - polyglot/k8s: flagd with file sync
6. Optionally add `.openfeature.yaml` and run `openfeature generate` for typed
   accessors (CLI is experimental — pin version if adopted).
7. Add CI validation for manifest schema and inventory cross-references.

## Verification

Before claiming flag work is complete:

1. Manifest validates against project schema (or OpenFeature CLI schema minimum).
2. Every `primaryGroup` / audience / environment reference resolves in inventories.
3. Typed accessors regenerated if the project uses codegen.
4. Evaluation tests cover: default, audience match, environment mismatch, draft,
   retired, and percentage boundary if used.
5. Telemetry events contain no PII.
6. Rollout flags have expiry/review metadata.

Discover project-specific validation commands from `AGENTS.md`, `package.json`,
or CI workflows (e.g. `pnpm nx test flags-catalogue`, `cargo test` on kernel
types).

## Anti-Patterns

| Anti-pattern                              | Fix                                                       |
| ----------------------------------------- | --------------------------------------------------------- |
| `process.env.FEATURE_X` in business logic | Manifest flag + OpenFeature evaluation                    |
| Vendor SDK at call sites                  | OpenFeature API + provider adapter                        |
| Per-user flag targeting                   | Audience attributes + optional percentage on targetingKey |
| Flag without owner, intent, or createdFor | Add governance fields before `active`                     |
| Rollout without sunset date               | Add `expiryOrReviewDate`                                  |
| Deleted manifest entry, dead code remains | Remove branches first, then manifest row                  |

## References

- Evaluation context model: `references/evaluation-context.md`
- OpenFeature intro: https://openfeature.dev/docs/reference/intro
- OpenFeature providers: https://openfeature.dev/ecosystem/
- flagd: https://flagd.dev/
- OpenFeature CLI: https://openfeature.dev/docs/tutorials/open-feature-cli/
- anvil (when present): `docs/guides/feature-flag-reference.md`,
  `docs/guides/feature-flag-governance.md`, `docs/guides/feature-flag-inventory.md`

## Output Contract

When reporting completed flag work, state:

1. Changes made (manifest entries, call sites, accessors)
2. Validation commands run and their results
3. Lifecycle state of every touched flag
4. Follow-ups (e.g. pending expiry/review dates, retirements due)
