# Account plan, activity, and entitlements

| Type  | Authority     | Owner | Status | Freshness                                                                                                                                                                   |
| ----- | ------------- | ----- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | BACT  | Live   | Last reviewed 2026-08-20 against `flags/audiences.json`, `flags/manifest.json`, `apps/anvil-api/src/lib/feature-flags.ts`, `apps/anvil-api/src/lib/licence.ts`, and ADR-121 |

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

**JWT claim mapping (BACT-013):** the licence JWT's primary claim is now
**`plan`**, sourced from `account.plan` at mint (session mint, session refresh,
and access-token/licence re-verify all sign the account's current plan, never a
hardcoded value). A **`tier`** claim is still written on the wire,
byte-identical to `plan`, purely as a compat alias for edge verifiers that read
the raw JWT without a DB round trip (`apps/docs-shell`) — it is not a second
semantic axis.

**Claim resolution (SEC-012):** a token never elevates itself. `verifyLicence`
resolves in three cases: a `plan` claim is used verbatim; a licence carrying
**only** `tier: 'pro'` — the one legacy shape pre-BACT-013 `signLicence` ever
minted — resolves to `beta`, the plan those accounts actually hold; anything
else resolves to `null` and the gate denies. The earlier rule, which promoted
any `tier` into `plan` so as not to "downgrade an in-flight session", was an
over-entitlement vector: no account has ever held `pro` (`beta_users.plan` CHECK
admits only `beta`) and the edge verifiers believe the claim without a DB round
trip. De-escalating keeps those sessions working at their real plan with no
forced re-authentication. `LicenceClaims.plan` is therefore `string | null` on
the verify side; `signLicence` rejects a null plan rather than mint an
unevaluable licence. Drop the alias — and the `tier: 'pro'` branch — once the
last pre-BACT-013 licence expires (90-day TTL from 2026-08-13, so ~2026-11-11).

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
