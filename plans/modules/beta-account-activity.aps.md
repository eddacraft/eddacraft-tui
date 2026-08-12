<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Beta Account Activity

| ID   | Owner | Priority | Status | Progress |
| ---- | ----- | -------- | ------ | -------- |
| BACT | —     | High     | In Progress | 0/6      |

**Last reviewed:** 2026-08-12 — BACT-003..006 merged via PR #3782; FLEET
stays anonymous population evidence (ADR-107); this module owns identity-bound
customer-success signals (who logged in, who used allowlisted core features).

> **Exclusive module.** Feature PRs flip item `Status:` only; do not bump the
> header or index `N/M` counts (ADR-053). Reconcile counts on a bookkeeping
> branch with `pnpm aps:index`.

## Purpose

Answer **customer-success** questions for named beta accounts:

- Did this invitee ever log in?
- When did they last log in?
- Have they used a small allowlist of core product surfaces (e.g. `watch`)?

That is distinct from **FLEET**, which answers population questions without
identity:

- Is anyone using `watch`?
- How many active installs / which install methods?

The original product instinct (“Elliot ran `anvil watch`”) and the investor
instinct (“is anyone using it?”) are both valid. They must not share one
phone-home channel. Putting email or user ids on the anonymous fleet beacon
would break [ADR-107](../decisions/107-fleet-telemetry-consent-posture.md).
This module is the second pipe: **authenticated account activity only**.

## In Scope

- Document the two-pipe boundary (FLEET anonymous vs BACT identity-bound) in
  operator/privacy docs and a short decision-log entry (no rewrite of ADR-107
  identity rules).
- `beta_users.first_login_at`, `last_login_at`, and `last_login_method` stamped
  on every successful session mint (GitHub device, OTP, legacy device confirm).
- Server-side, user-keyed, **allowlisted** feature-touch markers (first seen +
  last seen per feature key) for a closed set of CS-relevant surfaces.
- Authenticated CLI emission of those feature keys only when a session is
  present — never as part of the FLEET beacon payload.
- Operator surfaces: enrich `anvil admin show` and add list/filter (or cohort)
  views for never-logged-in, idle (no recent login), and logged-in-but-missing
  core feature.
- Align with existing EMAIL cohort proxies (`beta:active-recent` /
  `beta:active-idle` via refresh tokens) so docs and admin language stay
  coherent; prefer login stamps as the durable CS signal going forward.

## Out of Scope

- Re-identifying FLEET beacons or joining `install_id` → email on the public
  telemetry path (forbidden under ADR-107).
- Free-form command lines, argv, paths, repo names, hostnames, or raw email in
  feature-touch payloads.
- Full product analytics / session replay / third-party marketing SDKs.
- Changing the local USAGE/Kindling privacy contract (local pipe stays local).
- Resend open/click ingest (optional later promotion; operator uses the Resend
  dashboard until then — see Future work below).
- Export/tracing sink work (EXPORT).

## Interfaces

**Depends on:**

- [fleet-telemetry](./fleet-telemetry.aps.md) (FLEET, Done) — population half;
  BACT must not alter the anonymous beacon allowlist.
- [ADR-107](../decisions/107-fleet-telemetry-consent-posture.md) — FLEET consent
  and dimension contract remains binding for FLEET only.
- Auth mint paths in `apps/anvil-api` (`mintSession` / GitHub device poll / OTP
  verify / device confirm) — stamp login fields at the same moment a session
  is issued.
- Admin surface (`GET /admin/user/:email`, `anvil admin show|list`) and EMAIL
  audience resolvers for coherent “active / idle” language.

**Coordinates with:**

- [usage-analytics](../archive/modules/usage-analytics.aps.md) (USAGE, archived)
  — local feature/command observations; BACT is the remote identity-bound half
  for beta CS, not a second local Kindling pipe.
- FLAGS / feature-flag governance — feature keys should stay enumerated and
  preferably align with FLEET’s feature-key vocabulary where the same surface
  is counted anonymously.
- [email-broadcast](../archive/modules/email-broadcast.aps.md) (EMAIL) —
  `beta:active-recent` / `beta:active-idle` remain refresh-token based until
  login stamps exist; docs should describe both and prefer login stamps once
  shipped.
- Beta ops skill / admin runbook — operator workflow for “help people who are
  not using features.”

**Exposes:**

- Login timestamps on `beta_users` and in admin show/list.
- Allowlisted per-user feature-touch store + admin engagement filters.
- Documented two-pipe privacy story (FLEET vs BACT).

## Design decisions (operator-accepted 2026-08-11)

1. **Two pipes, two jobs.** FLEET = “is anyone?” BACT = “is this person?”
2. **FLEET identity unchanged.** No email, no user id, no principal on the
   anonymous beacon; no `install_id` → account join on that path.
3. **Auth-bound only.** Account feature touches require a successful login /
   licensed session path. Unsigned installs contribute only to FLEET
   aggregates (if telemetry is on).
4. **Allowlist-only feature keys.** Same discipline as FLEET dimensions: closed
   enum (e.g. `watch`, `start`, `check`, `auth` — final set at BACT-004
   execution). No free-form command strings.
5. **Login first.** Highest CS value for the smallest change is first/last
   login; feature touch follows.
6. **Email engagement stays Resend-side** until a separate item is promoted.

## Open Questions

- **OQ1 (feature allowlist):** **Resolved 2026-08-12** — first ship keys:
  `watch`, `start`, `check`, `auth` (enforced server-side and in CLI emit).
- **OQ2 (consent copy):** **Resolved in BACT-001** — two-pipe docs cover the
  signed-in CS pipe; no extra public marketing SDK language.
- **OQ3 (idle definition):** **Resolved 2026-08-12** — admin `users --engagement
  idle` uses `last_login_at` with default **30 days**; EMAIL
  `beta:active-*` refresh-token cohorts unchanged.

## Ready Checklist

- [x] Operator accepted two-pipe design (2026-08-11 conversation)
- [x] Explicit non-goals: no FLEET re-identification; no free-form argv
- [x] Work items sequenced: boundary docs → login stamps → admin surface →
      feature allowlist store → authenticated emit → engagement filters
- [x] OQ1–OQ3 deferred to named items without blocking login-stamp start

## Work Items

Sequencing: **BACT-001** and **BACT-002** can proceed in parallel; **BACT-003**
needs login fields; **BACT-004** can design the store while -002 lands;
**BACT-005** needs -004; **BACT-006** needs -002 and benefits from -004/-005.

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

## Future work (not authorised; not in 0/6)

When email engagement should leave the Resend dashboard, promote a new item
(suggested id `BACT-010`) to mirror verified `email.opened` / `email.clicked`
webhooks onto account or email-event rows for `admin show`. Depends on BACT-003
and Resend webhook verification on anvil-api. Do not implement under FLEET.
