# Audience-First Evaluation Context

This reference defines the default evaluation context shape for the `openfeature`
skill. It is generic — any project can adopt it — and matches the anvil flag
catalogue model.

## Design Principle

Flags answer: **"Should this capability be available in this environment for this
audience?"**

They do not answer: **"Should Alice see this?"**

Identity may _derive_ audience attributes (e.g. a token carries `plan: pro`), but
targeting rules match on **attributes**, not user records.

## Context Shape

```typescript
interface EvaluationContext {
  /** Stable id for percentage bucketing only — session, request, or anonymous cookie */
  targetingKey: string;

  /** Where the evaluator runs */
  environment: {
    environment: "local" | "development" | "preview" | "demo" | "production";
    channel?: "development" | "beta" | "production";
    deploymentRing?: string; // e.g. canary, stable
  };

  /** Entitlement attributes — omit when unauthenticated / anonymous */
  audience?: {
    accountTier?: string; // subscription tier
    licencePlan?: string; // licence level
    organisationId?: string; // tenant scope (hashed in telemetry if logged)
    userRole?: string; // role within org (admin, developer, viewer)
    cohort?: string; // named rollout cohort (early-adopter, beta-tester)
  };
}
```

### Field rules

| Field                     | Rule                                                                                                           |
| ------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `targetingKey`            | Required. Used for `percentage` operator hashing. Rotate per session/request; do not use as analytics user id. |
| `environment.environment` | Required. Must be a known id from `environments.json` when using a catalogue.                                  |
| `audience`                | Optional. Omit for anonymous surfaces; entitlement flags typically require it.                                 |
| Audience fields           | Set once at the request boundary from auth claims or licence lookup.                                           |

## Inventories

Keep canonical lists separate from flag definitions.

### audiences.json

```json
{
  "schemaVersion": 1,
  "audiences": [
    { "id": "plan-pro", "name": "Pro plan", "axis": "plan", "status": "active" },
    { "id": "channel-early-access", "name": "Early access", "axis": "channel", "status": "active" }
  ]
}
```

Axes group audiences for governance: `plan`, `role`, `staff`, `channel`. Targeting
rules reference the **attribute values** that correspond to these ids (e.g.
`accountTier: "pro"` maps to audience `plan-pro`), not the ids directly — unless
the project normalises ids into context fields.

### groups.json

Surface groups carry defaults so individual flags stay small:

```json
{
  "id": "api",
  "name": "API surface",
  "defaultClass": "entitlement",
  "defaultAudiences": ["plan-pro", "plan-enterprise"],
  "defaultStatus": "active"
}
```

Each flag's `primaryGroup` links to one group. Per-flag overrides are allowed;
the group is the architectural home for class and audience defaults.

### environments.json

```json
{
  "schemaVersion": 1,
  "environments": [{ "id": "preview", "name": "Preview deploys", "status": "active" }]
}
```

## Targeting Operators

Rules are evaluated **in order**; first match wins. Conditions within a rule are
**AND**.

| Operator     | Value type   | Matches when                                                  |
| ------------ | ------------ | ------------------------------------------------------------- |
| `equals`     | string       | attribute == value                                            |
| `not_equals` | string       | attribute != value (undefined → no match)                     |
| `in_set`     | string[]     | attribute in set (undefined → no match)                       |
| `not_in_set` | string[]     | attribute not in set                                          |
| `percentage` | number 0–100 | hash(`targetingKey`) falls within N%                          |
| `segment`    | string       | reserved; treat as `equals` unless provider defines otherwise |

Attribute paths typically use flat names (`environment`, `accountTier`) or
dotted paths if the resolver supports nested context (`environment.environment`).

## Example Rules

### Entitlement — plan gating in production

```json
{
  "conditions": [
    { "attribute": "environment", "operator": "equals", "value": "production" },
    { "attribute": "accountTier", "operator": "in_set", "value": ["beta", "pro", "enterprise"] }
  ],
  "variant": "enabled"
}
```

### Rollout — percentage within demo, no user identity

```json
{
  "conditions": [
    { "attribute": "environment", "operator": "equals", "value": "demo" },
    { "attribute": "targetingKey", "operator": "percentage", "value": 10.0 }
  ],
  "variant": "enabled"
}
```

### Staff-only surface

```json
{
  "conditions": [{ "attribute": "cohort", "operator": "equals", "value": "staff-internal" }],
  "variant": "enabled"
}
```

## Resolution Precedence

When the resolver supports overrides:

```
1. emergency override   → reason: emergency_override
2. local override       → reason: local_override  (dev/operator convenience only)
3. targeting rules      → reason: targeting_match
4. default variant      → reason: default
```

`draft` and `retired` flags skip targeting and return `defaultVariant` with
reason `disabled`.

## OpenFeature Mapping

OpenFeature evaluation context is a flat `string | number | boolean` attribute
map. Map the structured shape at the provider or client boundary:

```typescript
// Structured (application layer)
const ctx = {
  targetingKey: session.id,
  environment: { environment: "production", channel: "production" },
  audience: { accountTier: "pro" },
};

// Flat (OpenFeature EvaluationContext)
const ofContext = {
  targetingKey: ctx.targetingKey,
  environment: ctx.environment.environment,
  channel: ctx.environment.channel,
  accountTier: ctx.audience?.accountTier,
};
```

Keep mapping centralised in one module or provider hook — not duplicated at every
call site.

## Telemetry

Safe fields:

- `flagKey`, `variant`, `reason`, `snapshotVersion`
- `environment` (id only)
- `runtime` / `surface`

Never log: email, display name, raw `targetingKey` tied to identity, or full
audience objects with tenant-identifying details unless explicitly approved.

## Contrasts (What Not To Do)

| User-focused (avoid)     | Audience-first (prefer)                                                |
| ------------------------ | ---------------------------------------------------------------------- |
| `userId in allowlist`    | `accountTier in ['pro']` or `cohort == 'beta'`                         |
| Flag per customer        | Entitlement + org attribute when tenant-specific gating is unavoidable |
| `percentage` on `userId` | `percentage` on `targetingKey` after audience eligibility              |
| Lookup user in provider  | Pass claims-derived audience into context                              |
