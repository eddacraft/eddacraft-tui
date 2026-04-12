# Feature Flag Inventory

This document classifies existing feature-flag-like controls in Anvil and maps
them onto the shared flagging model defined in `FLAGS`.

## Classification Key

| Action      | Meaning                                                             |
| ----------- | ------------------------------------------------------------------- |
| **migrate** | Move onto the shared manifest and OpenFeature-backed resolution     |
| **adopt**   | New capability — will use the shared model from the start           |
| **defer**   | Not ready to migrate yet; document what a future migration involves |

## Current Controls

### CLI licence-gated actions

- **Current state:** Rust CLI calls `/api/v1/whoami` to get a plan string;
  access is gated by API 401 responses rather than in-process flag evaluation.
- **Classification:** **migrate**
- **Target flag:** `cli.licence-gate` (class: `entitlement`)
- **Migration path:** Resolve the flag locally via the snapshot-backed provider
  using `licencePlan` and `accountTier` from the evaluation context. The API
  still validates the licence token, but the CLI can make in-process access
  decisions without a round-trip for every gated command.
- **Featureboard swap impact:** Provider replacement only — the evaluation
  context and flag key stay the same.

### Docs access gating

- **Current state:** Vercel Edge Middleware checks for a valid JWT in the
  `anvil-docs-session` cookie. It is a binary authenticated/not check with no
  tier or plan awareness.
- **Classification:** **migrate**
- **Target flag:** `docs.access` (class: `entitlement`)
- **Migration path:** After JWT validation, resolve the `docs.access` flag using
  the authenticated user's `accountTier`. The middleware can then allow or deny
  based on targeting rules (e.g. beta, pro, enterprise) rather than just
  authentication presence.
- **Featureboard swap impact:** Provider replacement only — middleware still
  resolves via the same evaluation context.

### OPA agent orchestration rollout

- **Current state:** No formal flag exists. Rollout is controlled by
  configuration and manual environment promotion.
- **Classification:** **defer**
- **Reason:** The orchestration system is still being built (`OPAE` module).
  Once stable, a `rollout` class flag with environment targeting and a kill
  switch will be appropriate.
- **Featureboard swap impact:** None yet — no flag to swap.

### Tier-based product capabilities

- **Current state:** No formal flag exists. Future tiers will need gating of
  specific Anvil features by plan level.
- **Classification:** **adopt**
- **Target flags:** TBD — individual `entitlement` class flags per gated
  capability, each with `accountTier`/`licencePlan` targeting.
- **Featureboard swap impact:** Provider replacement only, assuming Featureboard
  supports the same evaluation context dimensions.

## Provider Swap Summary

A future Featureboard provider swap would affect:

| Layer                   | Impact                                               |
| ----------------------- | ---------------------------------------------------- |
| Application call sites  | None — they use OpenFeature                          |
| Evaluation context      | None — dimensions are vendor-neutral                 |
| Targeting rules         | Minimal — rewrite rules in Featureboard's format     |
| Snapshot publication    | Replace — Featureboard provides its own distribution |
| Provider implementation | Replace — new provider wraps Featureboard SDK        |
| Telemetry               | Adjust — Featureboard may have its own observability |

The migration is isolated to the provider boundary. Application code that calls
`resolveFlag` or uses the OpenFeature client does not change.
