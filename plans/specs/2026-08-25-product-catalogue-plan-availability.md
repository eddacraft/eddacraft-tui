# Product catalogue plan-audience availability

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for FLAGCAT-015 plan mapping | [FLAGCAT](../modules/feature-flag-catalogue.aps.md) | Accepted | 2026-08-25 — live plan-axis vocabulary; no Individual/Teams/Enterprise ids |

| Upstream | Downstream |
| -------- | ---------- |
| ADR-076; ADR-121; FLAGCAT-011..014; `flags/audiences.json`; `flags/surfaces.json` | FLAGCAT-015 implementation and generated catalogue view |

**Execution authority** is FLAGCAT-015. This specification records the
operator-declined grill default: use the live plan-axis audience ids as the
approved vocabulary. It does not authorise runtime enforcement, JWT or account
migration, FLAGCAT-017/018, or new commercial names.

## Approved vocabulary

The canonical plan vocabulary is the active `axis: "plan"` audience ids:

- `plan-free`
- `plan-beta`
- `plan-pro`
- `plan-enterprise`

Individual, Teams, and Enterprise remain marketing labels, not catalogue ids.
ADR-121 still issues only account plan `beta` mapped to `plan-beta`; this item
does not widen issued plans.

## Required field

Every `productFeatures[]` entry declares `planAvailability`: a strict object
with exactly those four keys. Each value is `available`, `unavailable`, or
`undecided`.

Missing keys, extra keys, and unknown dispositions fail validation. v1
compatibility projections emit all four keys as `undecided`.

## Evidence for initial fill

This is a reviewed catalogue declaration, not host enforcement.

1. Linked **entitlement** flags whose targeting lists `accountTier` values in
   the plan-axis set are evidence. Those plan ids are `available`; the other
   plan-axis ids are `unavailable` for that feature.
2. Rollout and kill-switch flags are not SKU evidence.
3. Entitlement flags with no plan-axis targeting leave the feature `undecided`.
4. Unflagged features are `undecided` on every plan id.
5. Do not invent `unavailable` without targeting evidence.

## Non-goals

- Runtime availability enforcement or daemon dispatch changes
- Account `plan`, JWT, or audience-id migration
- New plan vocabulary or billing SKUs
- FLAGCAT-017/018 entitlement assertions
