# ADR-121: Account plan, activity metrics, and flag-backed entitlements

## Status

Accepted 2026-08-12 (operator) — accepted as written from the 2026-08-12 design
conversation (user plan name, DAI/DAA split, entitlements via feature-flag
catalogue).

## Date

2026-08-12

## Context

Anvil has two remote visibility needs that must not share one pipe:

1. **Population evidence** (FLEET / ADR-107) — anonymous install beacons;
   directional DAI/WAU/MAU for “is anyone using it?”
2. **Named account customer-success** (BACT) — who was admitted, who
   authenticated, who used allowlisted surfaces.

BACT-001..006 shipped login stamps, allowlisted feature touches, and admin
filters. Production still shows zero `first_login_at` values for most admitted
accounts because **invite tokens never mint interactive sessions**, so login
stamps alone understate engagement.

Separately, commercial and product gating already exist in the feature-flag
catalogue: plan-axis audiences (`plan-beta`, `plan-pro`, …), entitlement-class
flags (`api.scope.*`, `cli.licence-gate`, `docs.access`, …), and group default
audiences. Account rows do not yet carry a durable **plan** that evaluation
context can use; JWT surfaces expose `plan` mapped from `tier`, which confuses
axes.

Without a written decision, implementers risk inventing parallel entitlement
lists, mixing FLEET install IDs with accounts, or calling install DAI “user
DAU.”

## Decision

1. **Users have a plan name.** Account rows gain `plan text NOT NULL DEFAULT
   'beta'` (closed set initially: only `beta`). The table may remain named
   `beta_users` until a deliberate rename. Plan name `beta` maps to catalogue
   audience `plan-beta`.

2. **Three axes stay distinct:**
   - `status` — account lifecycle
   - `plan` — product/cohort plan name
   - token `scopes` — capability grants, each backed by `api.scope.*`
     entitlement flags

3. **Entitlements are feature flags.** Product access is decided by catalogue
   flags of class `entitlement` (and related surfaces), targeted using plan-axis
   audiences and evaluation context that includes the account’s `plan` (plus
   environment). Do not store free-form feature lists on the user row. Do not
   hardcode commercial if-trees that bypass the catalogue.

4. **Activity for named accounts (DAA).** Introduce `last_activity_at` (and
   optionally `last_activity_kind`) updated on:
   - interactive session mint (also keeps existing login stamps)
   - successful session refresh (activity only — not a fake login)
   - authenticated allowlisted feature-touch accept  
   Invite/approve alone does not set activity or login stamps.

5. **Metrics labelling:**
   - **DAI** = FLEET daily active *installs* (ADR-107; directional)
   - **DAA** = distinct *accounts* with activity in the window (BACT)
   - Never join `install_id` to email on the public telemetry path
   - Do not overload the FLEET overview contract with identity metrics

6. **JWT:** Prefer a `plan` claim aligned with the account column. Resolve the
   existing `tier` / `plan` aliasing under implementation (compat alias
   allowed; dual semantic axes forbidden).

7. **Historical DAA charts** require a daily rollup (or event grain), not only
   `last_activity_at`. Window metrics ship first; rollup is sequenced APS work.

## Rationale

- Catalogue audiences already define plans; account `plan` is the runtime
  principal attribute those audiences describe.
- Token-era beta users refresh and use scopes without re-login; activity must
  include refresh and feature-touch or CS metrics stay empty forever.
- Keeping DAI and DAA separate preserves ADR-107 privacy and avoids false
  precision.
- Flag-backed entitlements avoid a second product matrix that drifts from
  FLAGCAT / ADR-048.

### Alternatives considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| **Chosen:** plan column + flag audiences + activity stamps | Aligns catalogue and accounts; honest DAA; small schema | Requires refresh/feature stamp work; JWT cleanup |
| Login stamps only | Already shipped | Misses token-only use |
| Use FLEET install DAU as user DAU | Single number | Wrong unit; privacy violation if re-id |
| Free-form features on user row | Flexible | Bypasses catalogue; unbounded analytics |
| Full `users` rename + plan catalogue now | Clean names | Large blast radius; delays activity |

## Consequences

- **Positive:** Clear operator language (users + plan); DAA can reflect real
  licensed use; entitlements stay single-sourced in flags; FLEET remains clean.
- **Negative:** Additional columns and stamp paths; temporary dual vocabulary
  (`beta_users` table name vs user language).
- **Risks:** Refresh-heavy clients inflate DAA (accepted: licensed session use
  is activity); incomplete CLI emit undercounts feature path (mitigate via
  refresh stamp).
- **Mitigations:** Docs and admin labels; tests that invite ≠ activity; fleet
  contract tests remain green.

## References

- Spec: [2026-08-12-account-plan-activity-entitlements.md](../specs/2026-08-12-account-plan-activity-entitlements.md)
- APS: [beta-account-activity](../modules/beta-account-activity.aps.md) (BACT-007..013)
- ADR-107 (FLEET); ADR-048 (feature groups); [feature gating model](../specs/2026-05-19-feature-gating-model.md)
- `flags/audiences.json`, `flags/manifest.json`, `apps/anvil-api/src/lib/feature-flags.ts`
