# Account plan, activity, and entitlements

| Type  | Authority     | Owner | Status | Freshness                                                                                                                              |
| ----- | ------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | BACT  | Live   | Last reviewed 2026-08-12 against `flags/audiences.json`, `flags/manifest.json`, `apps/anvil-api/src/lib/feature-flags.ts`, and ADR-121 |

| Upstream                                                                                                                                                                                                | Downstream                                                         |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `plans/decisions/121-account-plan-activity-and-flag-entitlements.md`, `plans/specs/2026-08-12-account-plan-activity-entitlements.md`, `flags/audiences.json`, `apps/anvil-api/src/lib/feature-flags.ts` | Admin runbook, beta ops, auth/JWT mint, FLAGCAT evaluation context |

## Purpose

Operational vocabulary for **named accounts** (users), **plans**,
**entitlements**, and **activity metrics**. Implementation tracks
[BACT](../../plans/modules/beta-account-activity.aps.md) phase 2
(BACT-007..013).

Source truth for the catalogue and account store:

- Plan audiences: `flags/audiences.json`
- Entitlement and scope flags: `flags/manifest.json`
- API scope evaluation: `apps/anvil-api/src/lib/feature-flags.ts`
- Account schema: `apps/anvil-api/src/db/schema.sql`

Do not use this guide as a substitute for FLEET fleet numbers or as a billing
spec.

## Users and plan

- Accounts are **users** (table may still be named `beta_users` until renamed).
- Each user has a **`plan` name**. Today the only value is **`beta`**, which
  maps to the catalogue audience **`plan-beta`**.
- **`status`** is lifecycle only (`active`, `pending`, `suspended`, `banned`).
- Do not put free-form feature lists on the user row.

## Entitlements (feature flags)

| Layer             | Authority                                                       |
| ----------------- | --------------------------------------------------------------- |
| Plan audiences    | `flags/audiences.json` (`plan-beta`, `plan-pro`, …)             |
| Entitlement flags | `flags/manifest.json` class `entitlement`                       |
| Token scopes      | `access_tokens.scopes`, gated by `api.scope.*`                  |
| Evaluation        | Authenticated context includes account **`plan`** + environment |

Product code should resolve catalogue flags with that context, not hardcode
commercial if-trees that bypass FLAGCAT.

See also [feature-flag-governance.md](./feature-flag-governance.md) and the
[feature gating model](../../plans/specs/2026-05-19-feature-gating-model.md).

## Activity and metrics

| Label     | Meaning                               | Source                                       |
| --------- | ------------------------------------- | -------------------------------------------- |
| **DAI**   | Daily active _installs_               | FLEET `anvil admin fleet`                    |
| **DAA**   | Daily active _accounts_               | BACT `last_activity_at` (after BACT-008/009) |
| **Quiet** | Admitted user with no recent activity | BACT filters                                 |

**Account activity** (sets/advances `last_activity_at`) means any of:

1. Interactive session mint (also sets login stamps).
2. Successful session refresh.
3. Authenticated allowlisted feature-touch (`watch`, `start`, `check`, `auth`).

Invite/approve alone is **not** activity and does **not** set login stamps.

Login stamps alone understate token-era users who never re-run interactive
login.

## Operator commands (target)

| Need                               | Command                                                          |
| ---------------------------------- | ---------------------------------------------------------------- |
| Install population                 | `anvil admin fleet`                                              |
| Named user profile                 | `anvil admin show <email>`                                       |
| CS cohorts (phase 1)               | `anvil admin users --engagement …`                               |
| Account activity metrics (phase 2) | `anvil admin activity` (or documented equivalent after BACT-009) |

Never treat FLEET DAI as “how many customers logged in.”

## Privacy boundary

- FLEET: anonymous `install_id`; no email; ADR-107.
- BACT: authenticated account path only; no join of fleet install IDs to users.

## Related

- [admin-cli.md](../runbooks/admin-cli.md)
- [usage-analytics.md](../observability/usage-analytics.md)
- APS module
  [beta-account-activity](../../plans/modules/beta-account-activity.aps.md)
