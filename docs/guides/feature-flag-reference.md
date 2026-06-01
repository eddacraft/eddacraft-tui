# Feature Flag Reference

| Type  | Authority     | Owner   | Status | Freshness                                                                                                                                    |
| ----- | ------------- | ------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | FLAGCAT | Live   | Last reviewed 2026-05-25 against `packages/anvil/contracts/src/schemas/feature-flags.schema.ts` and `docs/guides/feature-flag-governance.md` |

| Upstream                                                                                                                                                                                             | Downstream                                                               |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `docs/guides/feature-flag-governance.md`, `packages/anvil/contracts/src/schemas/feature-flags.schema.ts`, `packages/anvil/runtime/src/feature-flags/`, `plans/modules/feature-flag-catalogue.aps.md` | Feature-flag authors, resolver integrations, `feature-flag-inventory.md` |

Quick reference for defining, resolving, and operating feature flags in Anvil.
Covers the shared flagging contract used across TypeScript and Rust surfaces.

For lifecycle policy and retirement rules, see `feature-flag-governance.md`. For
migration of existing controls, see `feature-flag-inventory.md`.

## Add a Flag (Step by Step)

### 1. Define the flag

```jsonc
// In your manifest (FeatureFlagManifest)
{
  "schemaVersion": 1,
  "flags": [
    {
      "key": "docs.access", // lowercase, dot/hyphen/underscore separated
      "owner": "DOCSAUTH", // APS module code
      "intent": "Gate /anvil docs for authenticated beta users",
      "class": "entitlement", // rollout | entitlement | ops_kill_switch
      "valueType": "boolean", // boolean | string | number | object
      "variants": [
        { "key": "enabled", "value": true },
        { "key": "disabled", "value": false },
      ],
      "defaultVariant": "disabled", // must match a variant key
      "status": "draft", // start as draft
      "createdFor": "FLAGS-008", // APS work item ID
      "expiryOrReviewDate": "2026-07-01T00:00:00Z", // required for rollout class
      "targeting": [
        // optional — rules evaluated in order
        {
          "conditions": [
            {
              "attribute": "accountTier",
              "operator": "in_set",
              "value": ["beta", "pro", "enterprise"],
            },
          ],
          "variant": "enabled",
        },
      ],
    },
  ],
}
```

### 2. Validate

```typescript
import { validateManifest } from '@eddacraft/anvil-contracts';

const result = validateManifest(manifestData);
if (!result.success) {
  console.error(result.errors);
}
```

### 3. Create a snapshot

```typescript
import { createSnapshot } from '@eddacraft/anvil-runtime/feature-flags';

const snapshot = createSnapshot(manifest);
// Write snapshot JSON to disk or distribution endpoint
```

### 4. Resolve at runtime

```typescript
import {
  resolveFlag,
  loadSnapshot,
} from '@eddacraft/anvil-runtime/feature-flags';

const snapshot = loadSnapshot(snapshotJson);
const flag = snapshot.flags.find((f) => f.key === 'docs.access');
const result = resolveFlag(flag, {
  targetingKey: 'session-abc',
  environment: { environment: 'production' },
  audience: { accountTier: 'beta' },
});

if (result.value === true) {
  // grant access
}
```

### 5. Set status to `active` when ready for evaluation

Change `"status": "draft"` to `"status": "active"` in the manifest and republish
the snapshot. Draft flags always resolve to their default variant.

---

## Field Reference

### Flag Definition

| Field                | Type         | Required                          | Description                                                     |
| -------------------- | ------------ | --------------------------------- | --------------------------------------------------------------- |
| `key`                | string       | yes                               | Unique identifier. Pattern: `^[a-z][a-z0-9]*([._-][a-z0-9]+)*$` |
| `owner`              | string       | yes                               | APS module code responsible for this flag                       |
| `intent`             | string       | yes                               | One sentence: why this flag exists                              |
| `class`              | enum         | yes                               | `rollout`, `entitlement`, or `ops_kill_switch`                  |
| `valueType`          | enum         | yes                               | `boolean`, `string`, `number`, or `object`                      |
| `variants`           | array        | yes                               | At least 2 variants, each with unique `key` and `value`         |
| `defaultVariant`     | string       | yes                               | Must match a variant `key`                                      |
| `status`             | enum         | yes                               | `draft`, `active`, `retiring`, or `retired`                     |
| `createdFor`         | string       | yes                               | APS work item ID (e.g. `FLAGS-008`)                             |
| `expiryOrReviewDate` | ISO datetime | rollout: yes, others: recommended | Sunset trigger for review                                       |
| `description`        | string       | no                                | Longer explanation                                              |
| `targeting`          | array        | no                                | Targeting rules evaluated in order                              |

### Flag Classes

| Class             | Failure mode           | Lifetime   | Use for                                                |
| ----------------- | ---------------------- | ---------- | ------------------------------------------------------ |
| `rollout`         | fail open (disabled)   | temporary  | Progressive feature enablement. Must have expiry date. |
| `entitlement`     | **fail closed** (deny) | long-lived | Tier/plan/licence gating. Needs periodic review.       |
| `ops_kill_switch` | **fail closed** (deny) | permanent  | Emergency disable. Always active, rarely toggled.      |

**Fail closed** means: if the default variant is missing, or an override
references a nonexistent variant, resolution returns an error with
`__fail_closed` variant. Rollout flags fail open (resolve to default) instead.

### Flag Status Lifecycle

```
draft → active → retiring → retired → (delete from manifest)
```

| Status     | Runtime behaviour                                            |
| ---------- | ------------------------------------------------------------ |
| `draft`    | Not evaluated — resolves to default with reason `disabled`   |
| `active`   | Fully evaluated — targeting and overrides apply              |
| `retiring` | Still evaluated — but no new targeting rules should be added |
| `retired`  | Not evaluated — resolves to default with reason `disabled`   |

---

## Targeting

### Evaluation Context

```typescript
{
  targetingKey: "session-abc",      // unique per evaluation session (required)
  environment: {
    environment: "production",      // local | development | preview | demo | production
    channel: "production",          // development | beta | production (optional)
    deploymentRing: "canary"        // freeform string (optional)
  },
  audience: {                       // optional — omit if unauthenticated
    accountTier: "pro",             // subscription tier
    licencePlan: "team",            // licence level
    organisationId: "org-123",      // specific org
    userRole: "admin",              // role within org
    cohort: "early-adopter"         // named cohort
  }
}
```

### Targeting Operators

| Operator     | Value type | Example           | Matches when                                           |
| ------------ | ---------- | ----------------- | ------------------------------------------------------ |
| `equals`     | string     | `"production"`    | attribute == value                                     |
| `not_equals` | string     | `"development"`   | attribute != value (undefined → no match)              |
| `in_set`     | string[]   | `["beta", "pro"]` | attribute is in the set (undefined → no match)         |
| `not_in_set` | string[]   | `["free"]`        | attribute is not in the set (undefined → no match)     |
| `percentage` | number     | `25.0`            | deterministic hash of `targetingKey` falls within 0–N% |
| `segment`    | string     | `"beta"`          | reserved — currently acts as `equals`                  |

### Targeting Rules

Rules are evaluated **in order**. First matching rule wins. Within a rule, all
conditions must match (AND semantics). If no rule matches, the `defaultVariant`
is used.

```jsonc
"targeting": [
  {
    // Rule 1: beta users in production get "enabled"
    "conditions": [
      { "attribute": "environment", "operator": "equals", "value": "production" },
      { "attribute": "accountTier", "operator": "in_set", "value": ["beta", "pro"] }
    ],
    "variant": "enabled"
  },
  {
    // Rule 2: 10% rollout in demo
    "conditions": [
      { "attribute": "environment", "operator": "equals", "value": "demo" },
      { "attribute": "targetingKey", "operator": "percentage", "value": 10.0 }
    ],
    "variant": "enabled"
  }
]
```

---

## Resolution Precedence

Resolution follows this order. First match wins:

```
1. emergency override     → reason: "emergency_override"
2. local override         → reason: "local_override"
3. targeting rules        → reason: "targeting_match"
4. default variant        → reason: "default"
```

Draft and retired flags skip all of the above and return the default variant
with reason `disabled`.

### Overrides

```typescript
const overrides = {
  emergency: { 'docs.access': 'disabled' }, // highest precedence
  local: { 'cli.licence-gate': 'enabled' }, // operator convenience
};

resolveFlag(flag, context, overrides);
```

Emergency overrides are for incidents — they bypass everything. Local overrides
are for operators and testing.

### Resolution Result

```typescript
interface ResolutionDetails {
  value: unknown; // the variant's value (boolean, string, number, or object)
  variant: string; // the variant key that was selected
  reason: ResolutionReason; // why this variant was chosen
  flagKey: string; // which flag was resolved
  errorCode?: string; // set when reason is "error"
  errorMessage?: string; // human-readable error detail
}
```

---

## Snapshots

Flag state is delivered to runtimes as versioned JSON snapshots.

### Shape

```jsonc
{
  "schemaVersion": 1,
  "snapshotVersion": 42, // monotonically increasing (positive integer)
  "issuedAt": "2026-04-12T14:30:00Z", // ISO 8601, second precision, UTC
  "flags": [
    /* FeatureFlagDefinition[] */
  ],
}
```

### Freshness

- Default max age: **300 seconds** (5 minutes)
- Snapshots issued more than 60 seconds in the future are rejected (clock skew
  protection)
- When a snapshot is stale: fail-closed flags deny, rollout flags use defaults
- Refresh failures: serve last-known-good until max age expires

### Cross-runtime Notes

- Timestamps use **second precision** (no milliseconds) for TS/Rust parity
- `snapshotVersion` is a positive integer (TS `number`, Rust `u64`)
- Percentage rollout uses identical hash function across runtimes
- Both runtimes accept timestamps with or without fractional seconds

---

## Telemetry

Three event types, all without PII:

| Event                  | When emitted                   | Key fields                                               |
| ---------------------- | ------------------------------ | -------------------------------------------------------- |
| `FlagSessionTelemetry` | Once at session start          | `snapshotVersion`, `environment`, `runtime`, `timestamp` |
| `FlagEvaluationEvent`  | First use per flag per session | `flagKey`, `variant`, `reason`, `timestamp`              |
| `FlagOverrideEvent`    | When an override is applied    | `flagKey`, `variant`, `source`, `timestamp`              |

```typescript
import {
  createSessionTelemetry,
  createEvaluationEvent,
  createOverrideEvent,
} from '@eddacraft/anvil-runtime/feature-flags';
```

---

## Examples by Class

### Rollout — progressive feature enablement

```jsonc
{
  "key": "onboarding.v2",
  "owner": "WELCOME",
  "intent": "Roll out redesigned onboarding flow",
  "class": "rollout",
  "valueType": "boolean",
  "variants": [
    { "key": "enabled", "value": true },
    { "key": "disabled", "value": false },
  ],
  "defaultVariant": "disabled",
  "status": "active",
  "createdFor": "WELCOME-020",
  "expiryOrReviewDate": "2026-06-01T00:00:00Z",
  "targeting": [
    {
      "conditions": [
        {
          "attribute": "environment",
          "operator": "equals",
          "value": "production",
        },
        {
          "attribute": "targetingKey",
          "operator": "percentage",
          "value": 10.0,
        },
      ],
      "variant": "enabled",
    },
  ],
}
```

Promote by increasing the percentage: 10 → 25 → 50 → 100. When stable at 100%,
set status to `retiring`, then `retired`, then delete.

### Entitlement — tier-based access gating

```jsonc
{
  "key": "cli.licence-gate",
  "owner": "BAUTH",
  "intent": "Gate CLI features behind licence validation",
  "class": "entitlement",
  "valueType": "boolean",
  "variants": [
    { "key": "enabled", "value": true },
    { "key": "disabled", "value": false },
  ],
  "defaultVariant": "disabled",
  "status": "active",
  "createdFor": "FLAGS-008",
  "targeting": [
    {
      "conditions": [
        {
          "attribute": "licencePlan",
          "operator": "in_set",
          "value": ["pro", "enterprise"],
        },
      ],
      "variant": "enabled",
    },
  ],
}
```

### Kill switch — emergency disable

```jsonc
{
  "key": "payment-processing",
  "owner": "BILLING",
  "intent": "Emergency disable for payment processing",
  "class": "ops_kill_switch",
  "valueType": "boolean",
  "variants": [
    { "key": "enabled", "value": true },
    { "key": "disabled", "value": false },
  ],
  "defaultVariant": "enabled",
  "status": "active",
  "createdFor": "BILLING-001",
}
```

To activate the kill switch, add an emergency override — no code change needed:

```jsonc
{ "emergency": { "payment-processing": "disabled" } }
```

### Multi-variant — string values

```jsonc
{
  "key": "checkout.layout",
  "owner": "DASH",
  "intent": "A/B test checkout page layouts",
  "class": "rollout",
  "valueType": "string",
  "variants": [
    { "key": "control", "value": "single-page" },
    { "key": "treatment-a", "value": "multi-step" },
    { "key": "treatment-b", "value": "accordion" },
  ],
  "defaultVariant": "control",
  "status": "active",
  "createdFor": "DASH-050",
  "expiryOrReviewDate": "2026-08-01T00:00:00Z",
  "targeting": [
    {
      "conditions": [
        {
          "attribute": "targetingKey",
          "operator": "percentage",
          "value": 33.0,
        },
      ],
      "variant": "treatment-a",
    },
    {
      "conditions": [
        {
          "attribute": "targetingKey",
          "operator": "percentage",
          "value": 66.0,
        },
      ],
      "variant": "treatment-b",
    },
  ],
}
```

---

## Rust API

The Rust kernel mirrors the TypeScript API:

```rust
use anvil_kernel_types::{
    FeatureFlagDefinition, FeatureFlagManifest, EvaluationContext,
    FEATURE_FLAG_SCHEMA_VERSION,
};
use eddacraft_anvil_kernel::feature_flags::{
    resolve_flag, create_snapshot, load_snapshot, is_snapshot_fresh,
    FlagOverrides, SnapshotConfig,
};

// Resolve a flag
let result = resolve_flag(&flag, &context, None);
match result.reason {
    ResolutionReason::TargetingMatch => { /* granted */ }
    ResolutionReason::Error => { /* check result.error_code */ }
    _ => { /* default or disabled */ }
}

// Snapshot lifecycle
let snapshot = create_snapshot(&manifest.flags);
let loaded = load_snapshot(&json_string)?;
let fresh = is_snapshot_fresh(&loaded, &SnapshotConfig::default());
```

---

## Key Pattern

Flag keys must match: `^[a-z][a-z0-9]*([._-][a-z0-9]+)*$`

| Valid               | Invalid                                 |
| ------------------- | --------------------------------------- |
| `cli.licence-gate`  | `CLI.LicenceGate` (uppercase)           |
| `docs.access`       | `docs..access` (consecutive separators) |
| `onboarding_v2`     | `_onboarding` (leading separator)       |
| `feature-flag.test` | `feature flag` (space)                  |
| `a`                 | `1flag` (starts with digit)             |

---

## Source Files

| What             | Path                                                           |
| ---------------- | -------------------------------------------------------------- |
| Schema (Zod)     | `packages/anvil/contracts/src/schemas/feature-flags.schema.ts` |
| Resolver (TS)    | `packages/anvil/runtime/src/feature-flags/resolver.ts`         |
| Snapshot (TS)    | `packages/anvil/runtime/src/feature-flags/snapshot.ts`         |
| Telemetry (TS)   | `packages/anvil/runtime/src/feature-flags/telemetry.ts`        |
| Exemplar tests   | `packages/anvil/runtime/src/feature-flags/exemplars.test.ts`   |
| Barrel exports   | `packages/anvil/runtime/src/feature-flags/index.ts`            |
| Types (Rust)     | `crates/anvil-kernel-types/src/feature_flags.rs`               |
| Resolver (Rust)  | `crates/anvil-kernel/src/feature_flags/resolver.rs`            |
| Snapshot (Rust)  | `crates/anvil-kernel/src/feature_flags/snapshot.rs`            |
| Telemetry (Rust) | `crates/anvil-kernel/src/feature_flags/telemetry.rs`           |
| Governance       | `docs/guides/feature-flag-governance.md`                       |
| Inventory        | `docs/guides/feature-flag-inventory.md`                        |
