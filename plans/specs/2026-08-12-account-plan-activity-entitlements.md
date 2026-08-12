<!-- Design spec: account plan, activity metrics, and flag-backed entitlements -->

# Account plan, activity metrics, and entitlements

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative (design) | BACT | Ready | 2026-08-12 — operator design conversation (plan name, DAI/DAA, flag entitlements) |

| Upstream | Downstream |
| -------- | ---------- |
| [ADR-107](../decisions/107-fleet-telemetry-consent-posture.md), [ADR-048](../decisions/048-feature-group-architectural-model.md), [feature gating model](./2026-05-19-feature-gating-model.md), [BACT](../modules/beta-account-activity.aps.md), `flags/audiences.json` | BACT-007..013, FLAGCAT coordination, admin runbook, auth/JWT claim alignment |

## Goal

Pin how Anvil models **users**, **plans**, **entitlements**, and **activity
metrics** so implementation does not invent a parallel access or analytics
system.

Specifically:

1. Accounts are **users** with a durable **`plan` name** (today only `beta`).
2. **Entitlements** are **feature flags** of class `entitlement`, targeted via
   the existing plan-axis audiences (`plan-beta`, …) — not free-form lists on
   the user row.
3. **Activity** for named accounts supports honest **daily active users (DAA)**
   without joining FLEET install IDs to email (forbidden under ADR-107).
4. **Daily active installs (DAI)** remains FLEET-only and stays labelled as
   directional install evidence.

## Non-goals

- Renaming table `beta_users` → `users` in phase 2 (optional later migration).
- Billing, seats, or a full `plans` product catalogue.
- Re-identifying FLEET beacons or `install_id` → account join.
- Free-form remote command analytics or third-party product analytics SDKs.
- Collapsing DAI and DAA into one “true DAU” number.
- Resend open/click ingest (still future BACT-010).

## Context (project truth, 2026-08-12)

### What already exists

| Layer | Location | Role |
| ----- | -------- | ---- |
| Account row | `beta_users` | Identity; lifecycle `status` |
| Login stamps | BACT-002 | Interactive mint only (`first_login_at`, …) |
| Feature touches | BACT-004/005 | Allowlisted authenticated emit |
| CS filters | BACT-006 | never_logged_in / idle / missing_feature |
| Token scopes | `access_tokens.scopes` (default `{beta}`) | Grant set on tokens |
| Scope flags | `api.scope.*` in `flags/manifest.json` | Catalogue-backed entitlement class |
| Plan audiences | `flags/audiences.json` axis `plan` | `plan-free`, `plan-beta`, `plan-pro`, `plan-enterprise` |
| Group defaults | `flags/groups.json` | Default audiences per surface group |
| FLEET DAI/WAU/MAU | `admin fleet` | Anonymous install activity |
| JWT “plan” today | `/auth` returns `plan: claims.tier` | **Name mismatch** — tier exposed as plan |

### Live observation (operator)

- FLEET reports install activity (DAI often low; MAU directional).
- BACT login stamps show **zero** interactive logins; invite/token users do not
  mint sessions, so `first_login_at` stays null.
- “Never logged in” ≠ “never used product with an invite token.”

## Decision summary

Authoritative durable form: [ADR-121](../decisions/121-account-plan-activity-and-flag-entitlements.md).

### 1. User + plan

- Product language: **users**, not “beta users forever.”
- Durable field: **`plan text NOT NULL DEFAULT 'beta'`** on the account row
  (table may remain `beta_users` until a rename).
- Closed set initially: `CHECK (plan IN ('beta'))`; widen with catalogue
  audiences when new plans ship.
- Mapping: plan name `beta` ↔ audience id `plan-beta` (strip `plan-` prefix on
  the account column for readability; audience inventory keeps the `plan-`
  prefix).

### 2. Three layers stay distinct

| Field / system | Meaning |
| -------------- | ------- |
| `status` | Account lifecycle (`active`, `pending`, `suspended`, `banned`) |
| `plan` | Commercial / cohort plan **name** |
| `scopes` on tokens | Capability grants; each scope has `api.scope.*` entitlement flag |
| Entitlement flags | Surface and product gates; target `plan-*` audiences (+ env, later role/channel) |
| Rollout flags | % / channel / kill-switch — not commercial plan |

### 3. Entitlements via feature flags

- Do **not** store free-form feature lists on the user.
- Authenticated evaluation builds `EvaluationContext` including:
  - stable `targetingKey` (user id preferred)
  - **`plan`** from the account row (maps to audience membership)
  - `environment` from deploy env (existing)
- Issue scopes only when the matching `api.scope.*` flag is enabled for that
  context (existing `resolveApiScope` glue).
- Product surfaces (`docs.access`, CLI entitlement flags, …) use the same
  context; avoid parallel `if (plan === 'beta')` trees.
- Invite/approve defaults: `plan = 'beta'` and default scopes from catalogue
  defaults (`DEFAULT_APPROVAL_SCOPES`), still gated by scope flags.

### 4. Activity model (DAA)

**Account activity** on day D (UTC) means any of:

1. Interactive session mint (existing login stamps).
2. Successful **session refresh** (new stamp path).
3. Authenticated **feature-touch** accept (existing BACT-005).

Invite/approve alone does **not** count.

**Field:** `last_activity_at timestamptz` (nullable) on the account row.

- Interactive mint → updates login stamps **and** `last_activity_at`.
- Refresh success → updates **`last_activity_at` only** (not `last_login_*`).
- Feature-touch upsert → `last_activity_at = max(existing, now)`.

Optional: `last_activity_kind` closed enum `login | refresh | feature`.

**Window metrics (v1):**

| Metric | Definition |
| ------ | ---------- |
| DAA | Trailing 24 h on `last_activity_at` (BACT-009 deviation, recorded 2026-08-13: implemented as a trailing window for consistency with WAA/MAA, not the calendar-day cut originally drafted here; FLEET DAI remains calendar-day — the runbook documents the difference and the surfaces are never compared directly) |
| WAA / MAA | Trailing 7 / 30 days on `last_activity_at` |
| Quiet | Active + (`last_activity_at` null or older than N days) |
| By plan | Same filters + `plan = …` |

**Historical daily series (follow-up):** daily rollup table of distinct active
account counts (and optionally per-plan). `last_activity_at` alone cannot
reconstruct “was active on day D” after the user goes quiet.

**Backfill (optional, separate item):** seed `last_activity_at` from max
`refresh_tokens.created_at` as a **proxy**; never write `first_login_at` from
that path.

### 5. Metrics labelling

| Label | Source | Use |
| ----- | ------ | --- |
| **DAI** | FLEET `activeInstalls.daily` | Investor / population “is anyone?” |
| **DAA** | BACT activity | Named-account engagement |
| **Admitted quiet** | BACT + waitlist | CS outreach |

Admin surfaces must not mix these into one undifferentiated “DAU.” Prefer a
dedicated activity/users overview; do not overload `anvil.fleet-overview.v1`.

### 6. JWT claim alignment

- Prefer durable claim name **`plan`** matching the account column.
- Today’s `plan: claims.tier` mapping is transitional debt: either emit `plan`
  from account.plan at mint, or keep `tier` as deprecated alias with explicit
  docs — implementation chooses one coherent story under BACT-013.
- Do not invent a second commercial axis beside catalogue audiences.

## Implementation sequencing (APS)

See [beta-account-activity](../modules/beta-account-activity.aps.md) phase 2:

| ID | Intent |
| -- | ------ |
| BACT-007 | Docs + decision-log/ADR wiring; operator vocabulary |
| BACT-008 | Schema `plan` + `last_activity_at`; stamp paths |
| BACT-009 | Admin activity / user metrics surface |
| BACT-011 | Daily account-activity rollup (history) |
| BACT-012 | Optional refresh-token activity backfill |
| BACT-013 | Evaluation context + JWT plan alignment |

Coordinates with FLAGCAT inventories (no second audience list) and FLEET (no
identity join).

## Validation expectations

- Invite/approve set `plan = 'beta'`; do not set login or activity stamps.
- Refresh advances `last_activity_at` without touching `last_login_*`.
- Feature-touch advances `last_activity_at`.
- Scope issuance still fails closed when `api.scope.*` is disabled.
- Fleet overview contract unchanged.
- Docs distinguish DAI vs DAA; link this spec and ADR-121.

## Open questions (non-blocking for Ready items)

| ID | Question | Default |
| -- | -------- | ------- |
| OQ-A | Historical daily rollup in same release as window metrics? | No — BACT-011 after BACT-009 |
| OQ-B | Backfill from refresh_tokens at first deploy? | Optional ops step under BACT-012 |
| OQ-C | JWT: replace `tier` or dual-write `plan`? | Prefer emit `plan` from account; keep `tier` only if needed for compat |

## References

- ADR-107 FLEET consent; BACT amendment 2026-08-11
- ADR-048 feature group model; 2026-05-19 feature gating model
- `flags/audiences.json`, `flags/groups.json`, `flags/manifest.json`
- `apps/anvil-api/src/lib/feature-flags.ts` (`resolveApiScope`)
- Operator conversation 2026-08-12 (stats, plan name, flag entitlements)
