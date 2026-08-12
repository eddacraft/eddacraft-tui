<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Beta Account Activity

| ID   | Owner | Priority | Status | Progress |
| ---- | ----- | -------- | ------ | -------- |
| BACT | —     | High     | Ready | 6/12     |

**Last reviewed:** 2026-08-12 — phase 2 planned: account **`plan`** (only
`beta` initially), **DAA** activity stamps (login + refresh + feature touch),
entitlements via feature-flag catalogue audiences; FLEET remains DAI only
([ADR-121](../decisions/121-account-plan-activity-and-flag-entitlements.md),
[spec](../specs/2026-08-12-account-plan-activity-entitlements.md)). Phase 1
(BACT-001..006) merged via PR #3782.

> **Exclusive module.** Feature PRs flip item `Status:` only; do not bump the
> header or index `N/M` counts (ADR-053). Reconcile counts on a bookkeeping
> branch with `pnpm aps:index`.

## Purpose

Answer **customer-success and product metrics** for named **users** (accounts):

- Which **plan** is this user on? (today only `beta` ↔ audience `plan-beta`)
- Did they ever complete interactive login?
- When were they last **active** (login, session refresh, or allowlisted feature)?
- Have they used allowlisted core surfaces (e.g. `watch`)?
- How many **daily active accounts (DAA)** vs FLEET **daily active installs (DAI)**?

That is distinct from **FLEET**, which answers population questions without
identity:

- Is anyone using `watch`?
- How many active installs / which install methods?

The product instinct (“Elliot ran `anvil watch`”) and the investor instinct
(“is anyone using it?”) are both valid. They must not share one phone-home
channel. Putting email or user ids on the anonymous fleet beacon would break
[ADR-107](../decisions/107-fleet-telemetry-consent-posture.md). This module is
the second pipe: **authenticated account activity and plan membership only**.

## In Scope

**Phase 1 (shipped):**

- Two-pipe boundary docs; login stamps; feature-touch store + emit; admin
  show/filters (BACT-001..006).

**Phase 2 (this plan):**

- Durable account **`plan`** name (`beta` only initially) aligned with
  `flags/audiences.json` plan axis.
- **`last_activity_at`** (optional kind) stamped on interactive mint, session
  refresh, and authenticated feature-touch — so token-era users appear in DAA.
- Operator **activity / user metrics** surface (DAA/WAA/MAA, quiet cohort, by
  plan) without overloading FLEET overview JSON.
- Authenticated **evaluation context** carries `plan` for entitlement flags
  (`api.scope.*` and surface entitlements); JWT `plan` aligned with account.
- Optional: daily account-activity **rollup** for historical DAA; optional
  refresh-token **backfill** of `last_activity_at` (never fake login stamps).
- Design authority: ADR-121 +
  [2026-08-12-account-plan-activity-entitlements.md](../specs/2026-08-12-account-plan-activity-entitlements.md).

## Out of Scope

- Re-identifying FLEET beacons or joining `install_id` → email on the public
  telemetry path (forbidden under ADR-107).
- Free-form command lines, argv, paths, repo names, hostnames, or raw email in
  feature-touch or activity payloads.
- Full product analytics / session replay / third-party marketing SDKs.
- Changing the local USAGE/Kindling privacy contract (local pipe stays local).
- Billing engine, seats, or a full multi-plan commerce catalogue (plans beyond
  naming + catalogue audience mapping are future).
- Renaming `beta_users` → `users` (optional later; not required for phase 2).
- Resend open/click ingest (optional later — BACT-010).
- Export/tracing sink work (EXPORT).
- Inventing a second entitlement inventory outside `flags/`.

## Interfaces

**Depends on:**

- [fleet-telemetry](./fleet-telemetry.aps.md) (FLEET, Done) — population half;
  BACT must not alter the anonymous beacon allowlist.
- [ADR-107](../decisions/107-fleet-telemetry-consent-posture.md) — FLEET consent
  remains binding for FLEET only.
- [ADR-121](../decisions/121-account-plan-activity-and-flag-entitlements.md) —
  plan, DAA, and flag-backed entitlements (phase 2 design gate).
- Auth mint and refresh paths in `apps/anvil-api` — login + activity stamps.
- Feature-flag catalogue (`flags/audiences.json`, `api.scope.*`, entitlement
  class) and `resolveApiScope` glue.
- Admin surface and EMAIL audience resolvers for coherent language.

**Coordinates with:**

- [feature-flag-catalogue](./feature-flag-catalogue.aps.md) (FLAGCAT) — single
  audience inventory; no parallel plan list.
- [usage-analytics](../archive/modules/usage-analytics.aps.md) (USAGE, archived)
  — local observations; BACT is remote identity-bound half.
- [email-broadcast](../archive/modules/email-broadcast.aps.md) (EMAIL) —
  `beta:active-*` refresh-token cohorts remain until deliberately migrated;
  prefer `last_activity_at` for CS once shipped.
- Beta ops skill / admin runbook — user activity and plan vocabulary.

**Exposes:**

- Login stamps, feature touches, **`plan`**, **`last_activity_at`**.
- DAA/WAA/MAA and quiet-user filters (and optional daily rollup).
- Evaluation context including plan for entitlement flags.
- Documented DAI vs DAA vocabulary.

## Design decisions

**Phase 1 (operator-accepted 2026-08-11):**

1. **Two pipes, two jobs.** FLEET = “is anyone?” BACT = “is this person?”
2. **FLEET identity unchanged.** No email / user id on the anonymous beacon.
3. **Auth-bound feature touches.** Allowlist-only keys; no free-form argv.
4. **Login stamps first**, then feature touch (shipped).

**Phase 2 (operator-accepted 2026-08-12):**

5. **Users have a `plan` name** (only `beta` initially) mapping to audience
   `plan-beta`.
6. **Entitlements are catalogue feature flags** (class entitlement), not
   free-form user feature lists. Scopes remain token grants backed by
   `api.scope.*`.
7. **DAA uses `last_activity_at`** from login **or refresh or feature-touch**;
   invite alone is not activity. DAI remains FLEET installs only.
8. **Do not mix** FLEET overview contract with identity metrics.

## Open Questions

- **OQ1 (feature allowlist):** **Resolved 2026-08-12** — `watch`, `start`,
  `check`, `auth`.
- **OQ2 (consent copy):** **Resolved in BACT-001**.
- **OQ3 (idle definition):** **Resolved 2026-08-12** for login-based idle;
  phase 2 should prefer **`last_activity_at`** for new “quiet” filters while
  documenting login-idle vs activity-idle.
- **OQ-A (historical DAA):** Default **BACT-011 after** window metrics
  (BACT-009). Resolve at BACT-011 start.
- **OQ-B (backfill):** Optional BACT-012; never set `first_login_at` from
  refresh history.
- **OQ-C (JWT plan vs tier):** Prefer emit `plan` from account; compat alias
  for `tier` only if required — resolve in BACT-013.

## Ready Checklist

- [x] Phase 1 shipped (BACT-001..006, PR #3782)
- [x] Operator accepted plan name, DAA activity model, flag entitlements
      (2026-08-12)
- [x] Design gate ADR-121 + design spec filed
- [x] Phase 2 items sequenced with validation and non-goals
- [x] OQ-A..C deferred to named items without blocking BACT-007/008 start

## Work Items

### Phase 1 (complete)

Sequencing was: **BACT-001** ∥ **BACT-002** → **BACT-003**; **BACT-004** →
**BACT-005**; **BACT-006** after login + (for missing-feature) touches.

### BACT-001: Two-pipe boundary decision and docs

- **Status:** Done
- **Priority:** High
- **Confidence:** high
- **Intent:** Make the FLEET-vs-BACT split durable so future work does not
  re-open ADR-107 to attach identity to the anonymous beacon.
- **Expected Outcome:** A short decision-log entry (or thin ADR amendment that
  **does not** change FLEET identity rules) records: FLEET remains anonymous
  population evidence; BACT owns identity-bound beta CS signals; no
  `install_id`→account join on the public beacon path. Privacy / telemetry
  docs (`docs/observability/usage-analytics.md` and the public telemetry page)
  distinguish the three stories: local USAGE, anonymous FLEET, signed-in BACT.
  FLEET module prose notes the coordination.
- **Non-scope:** Implementing login stamps or feature-touch storage.
- **Validation:** `pnpm docs:check` (or narrow docs validation for touched
  paths); decision log entry present; `rg` shows no instruction to put email
  on the fleet beacon.
- **Files:** `plans/decisions/107-fleet-telemetry-consent-posture.md`,
  `plans/decisions/DECISION-LOG.md`, `docs/observability/usage-analytics.md`,
  `docs/public/anvil/operations/telemetry.md`,
  `plans/modules/fleet-telemetry.aps.md`, `plans/index.aps.md`,
  `plans/modules/beta-account-activity.aps.md`
- **Dependencies:** none (design already operator-accepted).
- **changeType:** docs
- **releaseIntent:** candidate

### BACT-002: Login stamps on successful session mint

- **Status:** Done
- **Priority:** High
- **Confidence:** high
- **Intent:** Durable per-account answer to “have they logged in?” without
  scanning token tables.
- **Expected Outcome:** Migration adds nullable
  `beta_users.first_login_at`, `last_login_at` (`timestamptz`), and
  `last_login_method` (closed text enum: at least `github`, `otp`, `device`).
  Every successful session mint path sets `first_login_at` if null, always
  updates `last_login_at` and `last_login_method`. Invite/approve token mint
  that does **not** complete interactive login does **not** stamp login.
  Optional audit row `auth.login` with method only (no secrets).
- **Non-scope:** Feature-touch markers; admin CLI display (BACT-003); FLEET.
- **Validation:** API unit/integration tests cover first vs subsequent login,
  each mint path, and prove invite-only token creation does not set login
  stamps; migration applies cleanly.
- **Files:** `apps/anvil-api/src/db/migrations/019-beta-users-login-stamps.sql`,
  `apps/anvil-api/src/db/schema.sql`, `apps/anvil-api/src/db/queries.ts`,
  `apps/anvil-api/src/lib/session.ts`,
  `apps/anvil-api/src/routes/auth-{github,github-device,otp,device}.ts`,
  tests under `apps/anvil-api/src/__tests__/`
- **Dependencies:** none for schema; coordinates with BACT-001 docs.
- **changeType:** feature
- **releaseIntent:** candidate

### BACT-003: Admin show/list login surface

- **Status:** Merged 2026-08-12 via PR #3782
- **Priority:** High
- **Confidence:** high
- **Intent:** Operators can see login state without Neon SQL.
- **Expected Outcome:** `GET /admin/user/:email` and `anvil admin show`
  surface `first_login_at`, `last_login_at`, `last_login_method` (and a clear
  “never logged in” presentation when null). List or waitlist-adjacent admin
  output can filter or flag never-logged-in accounts (exact CLI flag shape at
  implementation). Contracts stay backward-tolerant for older servers.
- **Non-scope:** Feature-touch columns (BACT-006); Resend engagement.
- **Validation:** Admin API + CLI tests for show (and filter if shipped);
  human-readable and `--json` paths.
- **Files:** `apps/anvil-api/src/routes/admin.ts`, `crates/anvil-cli/src/auth/client.rs`, `crates/anvil-cli/src/commands/admin.rs`, tests
- **Dependencies:** BACT-002.
- **changeType:** feature
- **releaseIntent:** candidate

### BACT-004: Allowlisted account feature-touch store

- **Status:** Merged 2026-08-12 via PR #3782
- **Priority:** High
- **Confidence:** medium — final key set is OQ1.
- **Intent:** Persist “this account used this core surface” without free-form
  analytics.
- **Expected Outcome:** A user-keyed store (table or equivalent) recording
  allowlisted `feature_key`, `first_seen_at`, `last_seen_at` (and optional
  coarse count). Closed allowlist documented and enforced server-side
  (unknown keys rejected or ignored). Default first ship keys unless OQ1
  changes them: `watch`, `start`, `check`, `auth`. No IP, path, argv, or
  install_id required on the row.
- **Non-scope:** Client emission (BACT-005); joining to FLEET install ids.
- **Validation:** Schema/migration tests; reject/ignore unknown keys; upsert
  first/last semantics.
- **Files:** `apps/anvil-api/src/db/migrations/020-account-feature-touches.sql`, `apps/anvil-api/src/db/schema.sql`, `apps/anvil-api/src/db/queries.ts`, `apps/anvil-api/src/lib/account-activity.ts`
- **Dependencies:** BACT-002 recommended (accounts exist); OQ1 resolved in
  this item’s PR description.
- **changeType:** feature
- **releaseIntent:** candidate

### BACT-005: Authenticated emission of feature-touch events

- **Status:** Merged 2026-08-12 via PR #3782
- **Priority:** High
- **Confidence:** medium — must not block command latency or leak when logged
  out.
- **Intent:** When a signed-in user exercises a core surface, the account
  record updates so CS can help people who never did.
- **Expected Outcome:** After successful auth, allowlisted commands (or a
  single authenticated heartbeat/report path) notify anvil-api to upsert
  feature-touch rows. Fire-and-forget / non-blocking; failures never break the
  user command. Logged-out runs emit nothing identity-bound (FLEET may still
  count anonymously if on). Payload carries only allowlisted keys.
- **Non-scope:** Expanding the allowlist ad hoc; anonymous beacon changes.
- **Validation:** CLI/API tests for auth-required emit, unknown-key rejection,
  and non-blocking failure; latency-sensitive paths stay free of synchronous
  remote waits beyond existing auth patterns.
- **Files:** `apps/anvil-api/src/routes/account-activity.ts`, `apps/anvil-api/src/index.ts`, `crates/anvil-cli/src/account_activity.rs`, `crates/anvil-cli/src/main.rs`
- **Dependencies:** BACT-004.
- **changeType:** feature
- **releaseIntent:** candidate

### BACT-006: CS engagement filters and runbook

- **Status:** Merged 2026-08-12 via PR #3782
- **Priority:** Medium
- **Confidence:** high
- **Intent:** Operators can list who needs help without hand-written SQL.
- **Expected Outcome:** Admin list/show (and runbook + beta-ops skill notes)
  support at least: never logged in; idle beyond a documented window (default
  30d on `last_login_at`, OQ3); logged in but missing a named core feature
  (e.g. no `watch` touch). Docs describe how this relates to EMAIL
  `beta:active-recent` / `beta:active-idle` without silently changing those
  cohort SQL definitions.
- **Non-scope:** Automated outreach emails; Resend webhooks.
- **Validation:** Admin API/CLI tests for each filter; `docs/runbooks/admin-cli.md`
  (or beta-ops) updated; docs:check for touched docs.
- **Files:** `apps/anvil-api/src/routes/admin.ts`, `apps/anvil-api/src/routes/admin-schemas.ts`, `crates/anvil-cli/src/commands/admin.rs`, `docs/runbooks/admin-cli.md`
- **Dependencies:** BACT-003; BACT-004/-005 for the missing-feature filter.
- **changeType:** feature
- **releaseIntent:** candidate

### Phase 2 (authorised design; Ready for implementation)

Sequencing: **BACT-007** and **BACT-008** may start in parallel (007 is docs;
008 is schema/stamps). **BACT-013** needs `plan` on the account (008) and
coordinates with FLAGCAT inventories. **BACT-009** needs 008. **BACT-011** and
**BACT-012** after 008; 011 preferably after 009 so operator read path exists.

### BACT-007: Phase-2 vocabulary and operator docs

- **Status:** In Progress
- **Priority:** High
- **Confidence:** high
- **Intent:** Durable operator/agent language: users + plan, DAI vs DAA,
  entitlements via flags — without reopening ADR-107 identity rules.
- **Expected Outcome:** Admin runbook (and brief privacy/observability pointers)
  document: `plan` (only `beta` today), `last_activity_at` semantics, DAI
  (FLEET) vs DAA (BACT), quiet vs never-interactive-login. Links ADR-121 and
  the phase-2 design spec. No product code required beyond doc cross-links if
  schema not yet landed (describe as “target once BACT-008 ships” if needed).
- **Non-scope:** Implementing stamps or admin metrics (008/009).
- **Validation:** `pnpm docs:check` (or path-scoped docs validation); links to
  ADR-121 and spec resolve.
- **Files:** `docs/runbooks/admin-cli.md`, `docs/observability/usage-analytics.md`
  (pointers only), `plans/decisions/DECISION-LOG.md` (if log row needs refresh),
  `plans/modules/beta-account-activity.aps.md`
- **Dependencies:** ADR-121 Accepted; design spec Ready.
- **changeType:** docs
- **releaseIntent:** candidate

### BACT-008: Account `plan` + `last_activity_at` stamps

- **Status:** In Progress
- **Priority:** High
- **Confidence:** high
- **Intent:** Schema and write paths so every licensed use can update activity
  without faking interactive login.
- **Expected Outcome:** Migration adds `plan text NOT NULL DEFAULT 'beta'`
  (`CHECK (plan IN ('beta'))` initially) and nullable `last_activity_at`
  (optional `last_activity_kind` in `login|refresh|feature`). Interactive mint
  continues login stamps and sets activity; **session refresh** and
  **feature-touch** upsert set/advance `last_activity_at` only; invite/approve
  set `plan='beta'` and do **not** stamp login or activity. Schema.sql and
  queries/tests cover all paths.
- **Non-scope:** Admin DAA UI (009); JWT/context (013); daily rollup (011);
  backfill (012); table rename.
- **Validation:** API unit/integration tests: invite clean; refresh updates
  activity not login; feature-touch updates activity; login updates both;
  migration applies cleanly.
- **Files:** `apps/anvil-api/src/db/migrations/`, `schema.sql`, `queries.ts`,
  `lib/session.ts`, auth mint routes, `routes/account-activity.ts`, tests under
  `apps/anvil-api/src/__tests__/`
- **Dependencies:** BACT-002/004/005 shipped; ADR-121.
- **changeType:** feature
- **releaseIntent:** candidate

### BACT-009: Admin user activity metrics surface

- **Status:** In Progress
- **Priority:** High
- **Confidence:** high
- **Intent:** Operators see DAA/WAA/MAA and quiet users without Neon SQL and
  without overloading `admin fleet`.
- **Expected Outcome:** Admin API + CLI surface (e.g. `anvil admin activity` or
  documented equivalent) reports account window metrics from
  `last_activity_at`, optional filter by `plan`, and quiet/never-activity
  cohorts. `admin show` includes `plan` and `last_activity_at`. Labels
  explicitly **accounts** not installs. FLEET overview JSON unchanged.
- **Non-scope:** Historical chart series (011); EMAIL SQL cohort rewrite.
- **Validation:** Admin API/CLI tests; docs runbook section; fleet contract
  tests still pass.
- **Files:** `apps/anvil-api/src/routes/admin.ts`, admin schemas, CLI admin
  command, `docs/runbooks/admin-cli.md`, tests
- **Dependencies:** BACT-008.
- **changeType:** feature
- **releaseIntent:** candidate

### BACT-011: Daily account-activity rollup (historical DAA)

- **Status:** In Progress
- **Priority:** Medium
- **Confidence:** medium — scheduling surface (cron vs external) at execution.
- **Intent:** Reconstruct “how many accounts were active on day D” after users
  go quiet (window metrics alone cannot).
- **Expected Outcome:** Table (or equivalent) of daily active account counts
  (and preferably per-`plan` breakdown). Populated for completed UTC days via
  cron/job already used by anvil-api. Admin read path for recent history.
  Documented retention.
- **Non-scope:** Per-user day grain unless needed later; FLEET historical
  aggregates redesign.
- **Validation:** Migration + job tests; idempotent day write; admin read test.
- **Files:** migrations, cron route or job, admin read, tests, runbook
- **Dependencies:** BACT-008; benefits from BACT-009.
- **changeType:** feature
- **releaseIntent:** candidate

### BACT-012: Optional refresh-token activity backfill

- **Status:** Ready
- **Priority:** Low
- **Confidence:** high
- **Intent:** One-shot proxy so pre-stamp token users are not all “never
  active” on day one of `last_activity_at`.
- **Expected Outcome:** Documented ops/script or admin-safe job sets
  `last_activity_at` from `max(refresh_tokens.created_at)` (or similar) where
  null. **Never** sets `first_login_at` / `last_login_*`. Dry-run default.
- **Non-scope:** Continuous sync; claiming interactive login history.
- **Validation:** Dry-run vs apply tests; proof login columns unchanged.
- **Files:** script or admin path, runbook, tests
- **Dependencies:** BACT-008.
- **changeType:** feature
- **releaseIntent:** candidate

### BACT-013: Plan on evaluation context and JWT alignment

- **Status:** In Progress
- **Priority:** High
- **Confidence:** medium — JWT compat for `tier` vs `plan` (OQ-C).
- **Intent:** Entitlements evaluate with the account’s plan against catalogue
  audiences; stop anonymous-only context on authenticated paths.
- **Expected Outcome:** Authenticated API (and licensed CLI where applicable)
  build `EvaluationContext` including `plan` (and stable targeting key). Scope
  issuance continues to respect `api.scope.*`. JWT mint emits `plan` from
  account.plan; document/handle `tier` compat. Invite/approve ensure plan
  default. No second audience inventory.
- **Non-scope:** New commercial plans beyond `beta`; billing; FeatureBoard
  migration.
- **Validation:** Unit tests for context + resolveApiScope with plan audience;
  JWT claim tests; docs for claim mapping.
- **Files:** `apps/anvil-api/src/lib/feature-flags.ts`, session/licence mint,
  invite/approve paths, tests; pointers in feature-flag governance docs
- **Dependencies:** BACT-008; coordinates with FLAGCAT inventories (read-only
  catalogue).
- **changeType:** feature
- **releaseIntent:** candidate

## Future work (not authorised; not in 6/12)

- **BACT-010 (email engagement):** When email engagement should leave the
  Resend dashboard, promote a new item to mirror verified `email.opened` /
  `email.clicked` webhooks onto account rows for `admin show`. Depends on
  BACT-003 and Resend webhook verification. Do not implement under FLEET.
- **Table rename** `beta_users` → `users` — optional bookkeeping migration when
  product language fully leaves “beta” as the table name.
- **Additional plan names** (`pro`, `enterprise`, …) — widen CHECK + audience
  mapping when commercial plans ship; still catalogue-driven.
