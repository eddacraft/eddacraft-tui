# anvil-api Service — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                                                                         |
| -------- | --------- | ----- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | APGOV | Live   | Last reviewed 2026-06-10 (targeted delta review: broadcast generalisation, middleware, migrations 012-015) against main `45dd1047a`; full review 2026-05-07 against `v0.6.0-beta` |

| Upstream                                            | Downstream                                                                                                                                 |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `apps/anvil-api`, `anvil-archive/admin-cli-node`, ADR-018 | anvil CLI (auth flows, license refresh, update-check), operator admin CLI, anvil admin Rust command (RCLI2-009), eddacraft.ai install site |

> **Status:** Live (beta) **Last reviewed:** 2026-06-10 (targeted delta review:
> broadcast generalisation, middleware, migrations 012-015) against main
> `45dd1047a`; full review 2026-05-07 against `v0.6.0-beta` slate (HEAD
> `d223b8d9`) **Service / location:** `apps/anvil-api` (Hono on Vercel; Neon
> Postgres backing) **Module owner (APS):** APGOV
> (`plans/modules/api-governance.aps.md` — Draft); BAUTH
> (`plans/archive/modules/beta-auth-streamline.aps.md`, 20/20 Complete) owns the
> auth flows; ADMINCLI / ADMINCLIH (`plans/archive/modules/admin-cli.aps.md`,
> `plans/archive/modules/admin-cli-hardening.aps.md`) own the historical Node
> operator CLI; RCLI2-009
> (`plans/modules/rust-cli-tier2.aps.md#rcli2-009-admin-command-parity-listshowrevokeauditsend-migrationemail-update`)
> ports it into `anvil admin` **Used by:** `anvil` CLI (auth flows, license
> refresh, update-check), `anvil-archive/admin-cli-node/` (Node `anvil-admin` operator
> CLI — being retired into `anvil admin`),
> `crates/anvil-cli/src/commands/admin.rs` (Rust admin parity), the eddacraft.ai
> install/landing site (waitlist intake)

> **Auth flows are documented in [`auth-as-built.md`](auth-as-built.md).** This
> doc covers everything else in `apps/anvil-api`: non-auth admin surfaces,
> health / observability, the migration runner, middleware stack, DB layer,
> deploy posture, plus the historical `anvil-archive/admin-cli-node/` and its
> retirement path.

## 1. Overview

`anvil-api` is the only first-party HTTP service in the Anvil monorepo. It is
the trust boundary for token minting, the operator surface for waitlist / beta
cohort management, and the deploy-time owner of the Postgres schema. It runs as
a single Hono app (`apps/anvil-api/src/index.ts:45`) mounted under `/api/v1`,
deployed as a Vercel Function with a Neon Postgres backing DB and Resend for
transactional email.

The service has four broad surface areas:

1. **Auth** — token verify, license refresh, device-code, OTP, GitHub OAuth,
   session refresh. **Canonical doc: [`auth-as-built.md`](auth-as-built.md).**
   This doc cross-links and does not re-document.
2. **Admin** — beta-cohort operator endpoints under `/admin/*`. Auth via
   `adminAuth` middleware (shared `ADMIN_KEY` or per-operator `admin_keys` row).
3. **Public ingress** — `/waitlist` (anonymous), `/health`, and a
   token-protected `/waitlist/resend`.
4. **Scheduled** — `/cron/cleanup` invoked hourly by Vercel Cron.

The first-party SQL migration runner (`apps/anvil-api/src/db/migrate.ts`,
`apps/anvil-api/scripts/migrate.mjs`) sits alongside the runtime and is the
single deploy-time path for schema changes.

## 2. Architecture diagram

```text
                 ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
                 │ anvil CLI    │  │ admin-cli /  │  │ install site │
                 │ (Rust)       │  │ anvil admin  │  │ (eddacraft)  │
                 └───────┬──────┘  └──────┬───────┘  └──────┬───────┘
                         │ /auth/*        │ /admin/*        │ /waitlist
                         ▼                ▼                 ▼
              ┌─────────────────────────────────────────────────────┐
              │  Vercel Function (Node 22)                          │
              │  ┌───────────────────────────────────────────────┐  │
              │  │ Hono app  basePath('/api/v1')                 │  │
              │  │   logger → CORS → traceContext → rateLimiter  │  │
              │  └───────────────────────────────────────────────┘  │
              │                                                     │
              │  routes/                                            │
              │   ├─ auth.ts          ── see auth-as-built.md       │
              │   ├─ auth-device.ts   ── see auth-as-built.md       │
              │   ├─ auth-otp.ts      ── see auth-as-built.md       │
              │   ├─ auth-session.ts  ── see auth-as-built.md       │
              │   ├─ auth-github.ts   ── see auth-as-built.md       │
              │   ├─ admin.ts         ── invite/approve/revoke/     │
              │   │                      audit/waitlist/show/       │
              │   │                      send-migration/broadcast/  │
              │   │                      email-update               │
              │   ├─ waitlist.ts      ── public ingress + resend    │
              │   ├─ cron.ts          ── /cron/cleanup (hourly)     │
              │   └─ health (inline)  ── /health (DB + key probe)   │
              │                                                     │
              │  middleware/   trace-context.ts (global)            │
              │                rate-limit.ts (global)               │
              │                admin-auth.ts (admin/*)              │
              │                admin-rate-limit.ts (admin/*)        │
              │                                                     │
              │  lib/   token, licence, email, audience, audit,     │
              │         debug, device-code, feature-flags,          │
              │         email-registry, broadcast-audiences         │
              └────────────────┬───────────────────┬────────────────┘
                               │ neon()            │ Resend
                               ▼                   ▼
                    ┌───────────────────┐  ┌────────────────┐
                    │ Neon Postgres     │  │ Resend (email) │
                    │ (single instance) │  │ + audiences    │
                    └───────────────────┘  └────────────────┘

         Deploy edge:  apps/anvil-api/scripts/migrate.mjs  →  Neon
         (advisory-locked, dry-run + drift-detection, see §6)
```

## 3. Lifecycle / cold-start

1. Module load — `apps/anvil-api/src/index.ts:1-43` imports routes and runs
   `verifySigningKey()` / `verifyVerifyingKey()` as fire-and-forget cold-start
   probes (plus an informational GitHub CLI credential presence check —
   auth-owned, see [`auth-as-built.md`](auth-as-built.md)); failure logs
   `[boot] … unavailable: <error>` and is reflected in `/health`.
2. Hono app constructed with `basePath('/api/v1')` (`src/index.ts:45`).
3. Middleware mounted in order: `logger`, `cors`, `traceContext`, `rateLimiter`
   (`src/index.ts:47-87`). See §5.
4. Inline `GET /health` registered (`src/index.ts:89-129`). See §9.1.
5. Global error handler logs and returns `500` (`src/index.ts:131-134`).
6. Sub-apps mounted: `/auth`, `/auth/device`, `/auth/otp`, `/auth/session`,
   `/auth/github`, `/admin`, `/waitlist`, `/cron` (`src/index.ts:136-143`).
7. The Neon client and the licence signing key are lazy module-level singletons
   (`src/db/client.ts:8-20`, `src/lib/licence.ts:18-28`) — first request in a
   cold instance pays the connect / PEM-parse cost, all subsequent requests on
   that instance reuse the singleton.

## 4. Route surface (non-auth)

Auth routes (`/auth/*`) are documented in
[`auth-as-built.md`](auth-as-built.md). The table below covers the rest. All
paths are prefixed by `/api/v1`. Source pointers are file + first load-bearing
line.

| Method | Path                       | Auth                                                                              | Purpose                                                                                                                                                                                                                                       | Source                           |
| ------ | -------------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- |
| GET    | `/health`                  | None                                                                              | DB ping + licence-key parse probes; `status: 'degraded'` + `503` on any probe failure                                                                                                                                                         | `src/index.ts:89-129`            |
| POST   | `/waitlist`                | None                                                                              | Public waitlist signup (gated by `WAITLIST_PAUSED`)                                                                                                                                                                                           | `src/routes/waitlist.ts:13-100`  |
| POST   | `/waitlist/resend`         | `WAITLIST_RESEND_ADMIN_TOKEN` (Bearer or `X-Waitlist-Admin-Token`), constant-time | Force resend of confirmation email                                                                                                                                                                                                            | `src/routes/waitlist.ts:102-165` |
| POST   | `/admin/invite`            | `adminAuth`                                                                       | Invite a user; default = waitlist + approve + email; `tokenOnly=true` returns a raw token once for CI/service accounts                                                                                                                        | `src/routes/admin.ts:120`        |
| POST   | `/admin/approve`           | `adminAuth`                                                                       | Approve a single waitlisted email or the oldest N pending; preserves graded scopes; flag-gates each scope                                                                                                                                     | `src/routes/admin.ts:345`        |
| POST   | `/admin/revoke`            | `adminAuth`                                                                       | Revoke all tokens for an email, or a specific raw token                                                                                                                                                                                       | `src/routes/admin.ts:263`        |
| GET    | `/admin/waitlist`          | `adminAuth`                                                                       | Paginated waitlist listing filtered by status × source                                                                                                                                                                                        | `src/routes/admin.ts:535`        |
| GET    | `/admin/audit`             | `adminAuth`                                                                       | Paginated audit log (most-recent first) with `action`/`actor` filters                                                                                                                                                                         | `src/routes/admin.ts:552`        |
| GET    | `/admin/user/:email`       | `adminAuth`                                                                       | User + tokens (no hashes) + up to 10 recent audit entries; `auditError: true` on enrichment failure                                                                                                                                           | `src/routes/admin.ts:571`        |
| POST   | `/admin/send-migration`    | `adminAuth` + 5/hr per actor                                                      | Back-compat shim over the broadcast flow (EMAIL-006): maps `{source}` to (template `waitlist-migration`, audience `waitlist:source`) and reuses the shared two-phase snapshot contract                                                        | `src/routes/admin.ts:754`        |
| POST   | `/admin/broadcast`         | `adminAuth` + 5/hr per actor                                                      | Generalised two-phase send: dry-run snapshots template + audience + recipients behind an actor-bound `previewToken` (TTL 10 min); real-send atomically consumes the snapshot, refetches the cohort, and rejects on drift (`409 cohort_drift`) | `src/routes/admin.ts:1050`       |
| POST   | `/admin/user/email-update` | `adminAuth`                                                                       | Rewrite `beta_users.email` (only) when GitHub-account self-service isn't viable; existing licence JWTs continue working (sub-bound to user.id, not email)                                                                                     | `src/routes/admin.ts:960`        |
| GET    | `/cron/cleanup`            | `Bearer ${CRON_SECRET}` (constant-time)                                           | Purge expired device codes, OTP codes, and refresh tokens; runs hourly via Vercel Cron                                                                                                                                                        | `src/routes/cron.ts:27-61`       |

Request shapes for the admin surface are defined in
`src/routes/admin-schemas.ts:9-160` and exported as `*Input` / `*Query` Zod
types alongside. All `/admin/*` routes additionally sit behind a coarse
per-actor 60/min rate limit (§5.4).

### 4.1 Notes on individual surfaces

- **`/admin/invite`** has two modes. Default mode runs the full approve flow
  (`upsert beta_users` → `device_codes` row → `audit_log` → invite email) inside
  one `sql.transaction([...])` batch (`src/routes/admin.ts:173-193`);
  `tokenOnly: true` skips the email and returns a raw `anvil_beta_*` token
  (3-statement batch, `src/routes/admin.ts:131-148`). Both stamp `auth_method`
  (`shared` / `per_operator`) into `audit_log`.
- **`/admin/approve`** preserves graded scopes by reading
  `findActiveScopesForUser` and unioning with `DEFAULT_APPROVAL_SCOPES`
  (`src/routes/admin.ts:296-304`). Each requested scope is then flag-resolved
  via `resolveApiScope` (`src/routes/admin.ts:308-315`); a fully empty set
  returns `409 no_scopes` and writes a `user.approve.scopes_dropped` audit row.
  Batch mode classifies skip reasons (`not_found`, `collision`, `no_scopes`,
  `error`) and records `user.approve.collision` audits on user-code retries
  (`src/routes/admin.ts:390-449`).
- **`/admin/broadcast`** dispatches any registered broadcast template
  (`release-announcement`, `waitlist-migration` —
  `src/lib/email-registry.ts:118-147`) to any named audience
  (`src/lib/broadcast-audiences.ts`) under the dry-run → `previewToken` →
  real-send snapshot/consume contract (`409 cohort_drift` body shape =
  `driftDiffSchema`, `src/routes/admin-schemas.ts:61-66`). On a real-send the
  consumed snapshot is the source of truth — the body's template / audience
  fields are ignored, the bait-and-switch defence
  (`src/routes/admin.ts:1032-1035, 1057-1063`). Registry lookup uses
  `Object.hasOwn` rather than `in` so prototype keys like `toString` can't pass
  the guard (`src/routes/admin.ts:1081-1083`). Error codes are enumerated at
  `src/routes/admin.ts:1037-1048`. Snapshot rows live in
  `send_broadcast_snapshots` (§6.3); TTL 10 min (`src/routes/admin.ts:614`).
  Find / consume scoping is `(token, actor)` so a non-owner caller falls into
  `preview_token_missing` rather than learning the token exists
  (`src/routes/admin.ts:1167-1178`).
- **`/admin/send-migration`** is now a back-compat shim over the generalised
  broadcast flow (EMAIL-006 — `src/routes/admin.ts:754`, rationale comment
  `:760-770`): it maps `{source}` to (template `waitlist-migration`, audience
  `waitlist:source`), reuses the shared snapshot insert / find / consume path,
  then translates the result back to the legacy response shape and the
  `migration.email.*` audit names. It is no longer the only `cohort_drift`
  endpoint. Per EMAIL-001, the `waitlist:source` audience excludes addresses
  already present in `beta_users`, narrowing the cohort vs the pre-EMAIL-006
  `findWaitlistBySource` behaviour.
- **`/admin/user/email-update`** updates `beta_users.email` only — the
  `waitlist` row is intentionally left at the original address as the signup
  record (`src/routes/admin.ts:728-734`). Collisions are detected pre-write
  _and_ via Postgres unique-violation 23505
  (`src/routes/admin.ts:755-758, 780-784`).
- **`/waitlist`** honours `WAITLIST_PAUSED=true`
  (`src/routes/waitlist.ts:19-21`) for the DBCON-003 Neon consolidation cutover;
  flipping the env requires a redeploy. It is the only route that intentionally
  degrades to `503` on a feature flag.
- **`/cron/cleanup`** preserves rows for 1 hour after expiry to allow for clock
  skew / debug, and keeps revoked refresh tokens for 7 days
  (`src/routes/cron.ts:18-21`, query bodies in `src/db/queries.ts:1068-1110`).

## 5. Middleware stack

Order is load-bearing. All middleware runs before any route handler.

| Order          | Middleware         | Source                                      | Notes                                                                                                                                                                                                           |
| -------------- | ------------------ | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1              | `hono/logger`      | `src/index.ts:47`                           | Request log line per request                                                                                                                                                                                    |
| 2              | `hono/cors`        | `src/index.ts:73-84`                        | Allowlist via `ANVIL_CORS_ORIGINS`; supports exact match and `*.example.com` wildcards (`src/index.ts:50-71`); `maxAge: 300` (5 min preflight cache); `allowHeaders` includes `traceparent` (`src/index.ts:78`) |
| 3              | `traceContext`     | `src/middleware/trace-context.ts:15`        | Parses an inbound W3C `traceparent`, threads it on context, and echoes it as `X-Anvil-Traceparent` on the response; `400` on a malformed header; pass-through when the header is absent (no trace originated)   |
| 4              | `rateLimiter()`    | `src/middleware/rate-limit.ts:15-54`        | In-memory sliding window, 60 req / 60s / IP keyed off `x-forwarded-for[0]`; emits `X-RateLimit-{Limit,Remaining,Reset}`; per-instance only                                                                      |
| 5 (admin only) | `adminAuth`        | `src/middleware/admin-auth.ts:109-191`      | Mounted on the `admin` sub-app via `admin.use('*', adminAuth)` (`src/routes/admin.ts:55`). See `auth-as-built.md` for the canonical description; non-auth context below                                         |
| 6 (admin only) | `adminRateLimit()` | `src/middleware/admin-rate-limit.ts:51-106` | Per-admin-actor limits mounted after `adminAuth`: coarse `'all'` scope 60/min plus dedicated 5/hr caps on `/send-migration` and `/broadcast`. See §5.4                                                          |

### 5.1 CORS preflight cache (`0.5.0-beta` fix)

`maxAge: 300` is set deliberately in `src/index.ts:82` — a longer TTL would mean
an API outage poisons browsers for the full TTL after recovery, because a failed
preflight gets cached as "no preflight allowed". The 5-minute value balances
cost against blast-radius. The `Allow-Headers` set was also extended to include
`X-Waitlist-Admin-Token` for the resend endpoint, and later `traceparent` for
trace propagation (`src/index.ts:78`).

### 5.2 Rate limiter scope

The limiter is a per-Vercel-instance `Map` (`src/middleware/rate-limit.ts:19`).
On serverless, the store resets on every cold start and is not shared across
concurrent instances — see auth-as-built G-04. A 60-second `setInterval` reaps
expired entries to bound memory growth and is `unref()`'d so it doesn't keep the
process alive (`src/middleware/rate-limit.ts:22-29`).

### 5.3 Admin auth (cross-link)

`adminAuth` resolves the bearer in priority order:

1. Per-operator key — when `ADMIN_PER_OPERATOR_KEYS` is on **and**
   `ADMIN_KEY_PEPPER` is set, hashes the bearer with HMAC-SHA-256 keyed by the
   pepper, then `findAdminKeyByHash`. Active rows authenticate under the key's
   `actor_email` with `auth_method: 'per_operator'`. Revoked rows reject with
   `401 admin_key_revoked` and write an audit failure row.
2. Shared `ADMIN_KEY` — constant-time match; sentinel actor `shared-key@anvil`
   (`src/middleware/admin-auth.ts:13`); `auth_method: 'shared'`.

`X-Admin-Actor` is **ignored on both paths**
(`src/middleware/admin-auth.ts:88-108`) — the admin actor is set by the
middleware on context (`adminActor`, `adminAuthMethod`, `adminKeyId` —
`src/middleware/admin-auth.ts:19-25`) and admin handlers pull from there
(`src/routes/admin.ts:57-80`). A DB hiccup during per-operator lookup falls
through to shared-key rather than failing the admin surface
(`src/middleware/admin-auth.ts:148-154`). Misconfiguration
(`ADMIN_PER_OPERATOR_KEYS=1` without a pepper) is logged on every request
(`src/middleware/admin-auth.ts:43-52`) so it cannot hide in a long-running
deployment.

For the canonical description of the admin auth machine — including the dual key
model, attribution rules, and the `audit_log.auth_method` column — see
[`auth-as-built.md`](auth-as-built.md).

### 5.4 Admin per-actor rate limiting

`adminRateLimit` is mounted on the `admin` sub-app **after** `adminAuth`, in
three scopes: a coarse `'all'` cap of 60/min across the whole admin surface
(`src/routes/admin.ts:60`), plus dedicated 5/hr caps on `/send-migration`
(`src/routes/admin.ts:65-68`) and `/broadcast` (`src/routes/admin.ts:73`) — both
trigger outbound bulk email, so they get a much smaller burst budget. The
broadcast scope is keyed on the endpoint, not per template, so alternating
templates can't dodge the budget.

Buckets key on `adminActor` (set by `adminAuth`), so each per-operator key gets
its own counter and shared-key callers collapse into the `shared-key@anvil`
bucket. On exceed it returns `429 admin_rate_limited` with `Retry-After`, and
every response carries `X-RateLimit-Scope` alongside the standard
`X-RateLimit-*` headers (`src/middleware/admin-rate-limit.ts:85-102`). Like the
global limiter, it is per-instance best-effort on serverless — the stated goal
is to make a compromised key visibly noisy, not to enforce a cluster-wide quota
(`src/middleware/admin-rate-limit.ts:37-50`).

## 6. DB layer

### 6.1 Connection model

- Driver: `@neondatabase/serverless` (`src/db/client.ts:1`).
- Singleton, lazy-initialised, module-level (`src/db/client.ts:8-20`). One
  TCP-pooled HTTP-tunnel client per Vercel-Function instance.
- `setClient()` is gated behind `NODE_ENV === 'test'` (`src/db/client.ts:23-28`)
  so production code cannot rebind it.
- Multi-statement atomicity uses `sql.transaction([...])` (e.g.
  `src/routes/admin.ts:131-148, 173-193, 230-238`); single statements use the
  tagged-template form.

### 6.2 Queries

`src/db/queries.ts` (1415 lines) is the single SQL boundary. All queries are
defined here, validated against Zod schemas at the parse boundary, and returned
as typed shapes. Notable groupings:

- Beta users + access tokens (`findUserByEmail`, `findUserWithTokens`,
  `findActiveScopesForUser`, `revokeTokensByEmail`, `revokeTokenByHash` —
  `queries.ts:77-232`).
- Audit log (`insertAuditLog`, `findAuditEntries`, `findRecentAuditForEmail` —
  `queries.ts:251-1066`).
- Device codes / OTP codes / refresh tokens (auth flows — `queries.ts:285-680`;
  covered in [`auth-as-built.md`](auth-as-built.md)).
- Waitlist (`upsertWaitlistEntry`, `upsertWaitlistWithName`,
  `findWaitlistEntryByEmail`, `findUnapprovedWaitlistEntries`,
  `findWaitlistBySource`, `findWaitlistPaginated` — `queries.ts:687-1006`).
- Broadcast snapshots (`insertBroadcastSnapshot`, `findBroadcastSnapshot`,
  `consumeBroadcastSnapshot` — `queries.ts:1018, 1121, 1145`; shared by
  `/admin/broadcast` and the `/admin/send-migration` shim). The raw token is
  returned once by the insert; the table stores only its hash (§6.3).
- Cleanup helpers used by `/cron/cleanup` (`cleanupExpiredDeviceCodes`,
  `cleanupExpiredOtpCodes`, `cleanupExpiredRefreshTokens` —
  `queries.ts:1068-1110`).
- Admin keys (`findAdminKeyByHash` — `queries.ts:1128`).

### 6.3 Schema

The full schema is `src/db/schema.sql` (`171 lines`). The auth-relevant tables
(`beta_users`, `access_tokens`, `audit_log`, `device_codes`, `otp_codes`,
`refresh_tokens`, `admin_keys`) are documented in
[`auth-as-built.md`](auth-as-built.md). The non-auth tables are:

- `waitlist` — public signup ledger and source-of-truth for cohort filtering.
  `serial` PK (legacy — pre-uuid era); `email citext UNIQUE`; nullable `name`,
  `company`, `role`, `use_case`; `source text NOT NULL DEFAULT 'website'`
  (`src/db/schema.sql:41-51`, plus migrations `001`, `004`).
- `send_broadcast_snapshots` — append-with-consume contract for the broadcast
  (and send-migration shim) two-phase commit. Created as
  `send_migration_snapshots` by migration `006`, renamed and widened by
  migration `013`, token hashed at rest by migration `014`. Current columns:
  `token_hash text PRIMARY KEY` (SHA-256 of the opaque preview token),
  `template`, `template_props jsonb`, `audience_key`, `audience_params jsonb`,
  `recipients jsonb`, `created_by_actor`, `created_at`, `expires_at`,
  `consumed_at` (`src/db/schema.sql:104-122`). There is no `consumed_by_actor`
  column — consume scoping is `(token, actor)` at the query level.
- `admin_keys` — per-operator admin credentials (added by migration `007`); see
  auth-as-built §"Admin auth" for the trust model.
- `admin_keys_audit` — append-only mutation trail for `admin_keys` carrying the
  Pulumi commit SHA that authorised each provisioning / revocation (migration
  `008`). Two-person-rule evidence.
- `_migrations` — the runner's own tracking table (`src/db/migrate.ts:26-32`).
  Schema: `filename TEXT PRIMARY KEY`, `sha256 TEXT NOT NULL`,
  `applied_at TIMESTAMPTZ DEFAULT NOW()`.

Index footprint on the non-auth side: `idx_audit_log_actor`,
`idx_audit_log_created_at`, `idx_audit_log_metadata_email_lower`
(case-insensitive expression index for `/admin/user/:email`'s recent-audit join
— `src/db/schema.sql:136-138`, migration `005`).

## 7. Migration runner (`0.5.0-beta` Added)

The runner is a first-party SQL apply tool with dry-run preview, drift
detection, and Postgres advisory-lock-based serialisation.

### 7.1 Entry points

- Library: `src/db/migrate.ts:131-194` (`runMigrations(runner, options)`).
- CLI wrapper: `apps/anvil-api/scripts/migrate.mjs` — Node ESM script bound by
  the `migrate` / `migrate:dry-run` package scripts
  (`apps/anvil-api/package.json:scripts`).

The CLI wrapper opens a `Pool` from `@neondatabase/serverless` and adapts
`pool.query` into the runner's `QueryRunner` shape
(`apps/anvil-api/scripts/migrate.mjs:36-52`). Exit codes: `0` on success or
no-op, `1` on drift / `DATABASE_URL` missing / SQL error
(`apps/anvil-api/scripts/migrate.mjs:11-14, 60-64`).

### 7.2 Flow

1. Acquire a Postgres advisory lock keyed by
   `SHA-256("apps/anvil-api:db:migrations")` split into two int4s — two
   concurrent runners (CI + manual operator) serialise instead of racing on DDL
   (`src/db/migrate.ts:38-50, 137`).
2. Ensure the `_migrations` tracking table (`src/db/migrate.ts:64-66, 139`).
3. `discoverMigrations(dir)` reads every `.sql` from `src/db/migrations/`, sorts
   lexicographically, computes `sha256(content)` per file
   (`src/db/migrate.ts:52-62`).
4. `fetchAppliedMigrations` reads `_migrations` ordered by filename
   (`src/db/migrate.ts:68-71`).
5. `detectDrift` compares each applied row against the on-disk hash —
   missing-on-disk and hash-mismatch both surface as drift entries with short
   SHA prefixes (`src/db/migrate.ts:73-100, 145-159`). Drift throws; the runner
   refuses to apply.
6. `selectPending` picks the on-disk files whose filename isn't in the applied
   set (`src/db/migrate.ts:102-108`).
7. If `--dry-run`, log every pending file and return without applying
   (`src/db/migrate.ts:171-177`). Tracking table mutations (the `_migrations`
   insert + the file body itself) are wrapped in `BEGIN; …; COMMIT` per file
   with rollback on error (`src/db/migrate.ts:110-123`).
8. Always release the advisory lock in a `finally`
   (`src/db/migrate.ts:191-193`).

### 7.3 Operator ergonomics

- Drift errors render the recorded vs on-disk hash for every offending file,
  prefixed by filename, with explicit "refusing to apply" guidance
  (`src/db/migrate.ts:154-159`).
- The runbook is `docs/runbooks/db-migrations.md`. CI wiring lives under
  V050F-014 (`plans/modules/v050-release-followups.aps.md` line 315); manual
  invocation is the path until the workflow change lands.

### 7.4 Build-time guards

`apps/anvil-api/scripts/check-runtime-cjs.cjs` runs in `postbuild` and
`require()`s `svix` to reproduce the exact CommonJS resolution path Vercel hits
at cold start. uuid v14 is ESM-only; a workspace-wide `uuid` floor would crash
svix on require with `ERR_REQUIRE_ESM`. The guard exists because the regression
has already shipped to prod once
(`apps/anvil-api/scripts/check-runtime-cjs.cjs:7-15`); see also pnpm.overrides
`svix>uuid` in the workspace root.

## 8. Migration history

All files in `src/db/migrations/`. Sorted lexicographically and applied in that
order. Idempotent (`IF NOT EXISTS` / `CREATE OR REPLACE` /
`ALTER … ADD COLUMN IF NOT EXISTS`) so re-running is safe. Migrations
`013`-`015` additionally guard structural renames behind `information_schema`
lookups (so a fresh install whose `schema.sql` is already at the post-migration
shape is a no-op) and bound DDL lock acquisition with
`SET LOCAL lock_timeout = '30s'` so a deploy fails fast rather than queuing
behind in-flight traffic.

| File                                     | Shape                                                                                                                                                                                               | Rationale                                                                                                                                                |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `001-waitlist-add-columns.sql`           | ALTER waitlist (add `name`, `company`, `role`, `use_case`) via `DO $$ … information_schema` guard                                                                                                   | Backfill `waitlist` columns on prod DBs that were stood up before `schema.sql` carried them                                                              |
| `002-beta-users-add-pending-status.sql`  | DROP/CREATE constraint adding `'pending'` to `beta_users.status`                                                                                                                                    | DOCSAUTH GitHub OAuth needed a `pending` state pre-confirm                                                                                               |
| `003-auth-tables.sql`                    | CREATE `device_codes`, `otp_codes`, `refresh_tokens` + indexes                                                                                                                                      | BAUTH-001 baseline                                                                                                                                       |
| `004-waitlist-on-beta-db.sql`            | CREATE `waitlist` on the beta DB (with `citext` extension)                                                                                                                                          | DBCON-003 — colocate waitlist on the same DB the admin routes hit, ahead of full Neon consolidation                                                      |
| `005-audit-log-indexes.sql`              | CREATE INDEX on `audit_log(actor)`, `audit_log(created_at)`, expression index `LOWER(metadata->>'email')`                                                                                           | Removes seq scans behind `/admin/audit` and `/admin/user/:email` recent-audit join                                                                       |
| `006-admin-send-migration-snapshots.sql` | CREATE `send_migration_snapshots`                                                                                                                                                                   | Backs the send-migration dry-run → real-send contract                                                                                                    |
| `007-admin-keys.sql`                     | CREATE `admin_keys` (hashed bearer + actor_email + revoked_at)                                                                                                                                      | Per-operator admin auth (ADMINCLIH-002)                                                                                                                  |
| `008-admin-keys-audit.sql`               | CREATE `admin_keys_audit` (append-only with Pulumi commit SHA)                                                                                                                                      | Two-person-rule evidence for key provisioning                                                                                                            |
| `009-audit-log-auth-method.sql`          | ADD COLUMN `audit_log.auth_method` (`shared`/`per_operator`) backfilled `'shared'`                                                                                                                  | Differentiates dual-auth rollout window                                                                                                                  |
| `010-access-tokens-scope-index.sql`      | CREATE INDEX `idx_access_tokens_user_id` + partial composite supporting `findActiveScopesForUser`                                                                                                   | Fixes seq scan on `/session/refresh`, `/auth/device/poll`, `/auth/github/callback`, `/auth/otp/verify` for prod DBs that pre-date the `schema.sql` index |
| `011-access-tokens-edict-flag.sql`       | ADD COLUMN `access_tokens.is_edict boolean DEFAULT false`                                                                                                                                           | Mark long-lived early-access edicts as token metadata without splitting the entitlement model                                                            |
| `012-device-codes-attempts.sql`          | ADD COLUMN `device_codes.attempts int NOT NULL DEFAULT 0`                                                                                                                                           | Per-code brute-force counter for `/device/confirm` lockout (#922) — see [`auth-as-built.md`](auth-as-built.md)                                           |
| `013-broadcast-snapshots.sql`            | RENAME `send_migration_snapshots` → `send_broadcast_snapshots`; ADD `template` / `template_props` / `audience_key` / `audience_params`; backfill `source` into `audience_params` then DROP `source` | Generalise the snapshot table so `/admin/broadcast` and the send-migration shim share one two-phase contract (EMAIL-002/-006)                            |
| `014-broadcast-snapshot-token-hash.sql`  | RENAME COLUMN `token` → `token_hash`; SHA-256(token) stored at rest                                                                                                                                 | Align preview tokens with the `access_tokens` / `refresh_tokens` at-rest hashing convention; a read-only DB leak no longer yields usable tokens          |
| `015-beta-users-add-github-id.sql`       | ADD COLUMN `beta_users.github_id bigint UNIQUE` (nullable)                                                                                                                                          | GitHub device-flow account linking by stable numeric id (GHCLIAUTH-003, ADR-066) — see [`auth-as-built.md`](auth-as-built.md)                            |

## 9. Health / observability

### 9.1 `/health`

`src/index.ts:89-129`. Multi-probe health check, run in parallel:

- DB probe — `SELECT 1` against the Neon client; failure marks
  `db: unreachable`.
- Signing-key probe — `verifySigningKey()` attempts to parse the PKCS#8 PEM into
  a `CryptoKey`. Cached after first successful call. Failure marks
  `signingKey: unavailable`.
- Additional auth-owned probes / fields (`verifyingKey`, plus a non-gating
  `githubCliCreds` presence field) are documented in
  [`auth-as-built.md`](auth-as-built.md).

If every gating probe passes, returns `200 { status: ok, … }`. On any probe
failure the response is `status: 'degraded'` with a `503` and the granular
per-probe field set (`src/index.ts:109-128`), so the operator can tell which
probe failed without needing logs (auth-as-built G-01).

### 9.2 Cold-start probe

The same key probes are fired at module load (`src/index.ts:25-34`) so a missing
/ malformed key surfaces as a deploy-time stderr line and a `503 /health` rather
than only on the first device-flow mint.

### 9.3 Logging

- Request logs come from `hono/logger` (`src/index.ts:47`).
- Internal debug logs use the in-house `createDebugger`
  (`src/lib/debug.ts:61-65`) keyed by `'api'`, `'auth-device'`, `'auth-session'`
  namespaces; gated by `ANVIL_DEBUG=1` or `DEBUG` containing `anvil:*` /
  `anvil:api` (`src/lib/debug.ts:9-24`). The debug emitter sanitises bearer
  tokens, `sk-` / `ghp_` / `ghu_` secrets, long hex / base64 strings before
  writing (`src/lib/debug.ts:26-32`).
- Unhandled errors are logged with stack and surfaced as
  `500 Internal Server Error` (`src/index.ts:131-134`).
- Trace correlation relies on the client supplying a W3C `traceparent`, which
  `traceContext` threads through the request and echoes as `X-Anvil-Traceparent`
  (`src/middleware/trace-context.ts:15`, §5). No trace ID is originated
  server-side when the header is absent, and `createDebugger` output is not
  trace-keyed (G-02 below).

### 9.4 Rate-limit telemetry

The limiter sets `X-RateLimit-Limit` / `X-RateLimit-Remaining` /
`X-RateLimit-Reset` on every response and emits a debug log on exceed
(`src/middleware/rate-limit.ts:43-49`). No metrics are exported.

## 10. Deploy posture (Vercel)

- Runtime: Vercel **Node** (not Edge). `@neondatabase/serverless` and
  `node:crypto` (used everywhere) require Node.
- Engine pin: `node >= 22.13.0` (`apps/anvil-api/package.json:engines`).
- Module format: ESM (`"type": "module"`); the build emits ESM, but transitive
  `svix` requires CJS resolution of `uuid`. The `pnpm.overrides["svix>uuid"]`
  exception (workspace root) and the `postbuild` `check-runtime-cjs.cjs` guard
  (`apps/anvil-api/scripts/check-runtime-cjs.cjs`) defend the cold-start path
  against re-regression.
- Function entry: `apps/anvil-api/src/index.ts` exports the Hono app default
  (`src/index.ts:112`), which Vercel invokes via the standard Hono adapter.
- Build:
  `cd ../.. && pnpm --filter '@eddacraft/anvil-api^...' build && cd apps/anvil-api && tsc`
  (`apps/anvil-api/vercel.json:3`).
- Cron: `vercel.json` declares
  `{ "path": "/api/v1/cron/cleanup", "schedule": "0 * * * *" }`. The endpoint
  validates `Bearer ${CRON_SECRET}` constant-time (`src/routes/cron.ts:28-39`);
  a missing secret 401s.
- Ignore-build: previews skip non-release branches via
  `tools/scripts/vercel-ignore-build.sh --skip-preview apps/anvil-api packages/transactional`
  (`apps/anvil-api/vercel.json:2`).
- Cold-start posture: connection + signing-key both lazy and module-level; first
  request in a fresh instance pays both, but the cold-start probe pre-warms the
  signing key in the background so the cost lands before the first auth call
  lands.
- The `0.5.0-beta` post-deploy fix bundle (CORS preflight cache, `svix>uuid`
  runtime override, Vercel API routing, Hono entrypoint, scoped tsconfig) is
  documented in `apps/anvil-api/README.md:58-66` and `CHANGELOG.md:202-204`.
  Operators upgrading should redeploy from `main` rather than cherry-picking
  fixes individually.

`infra/src/vercel.ts` owns env-var wiring for the EddaCraft-managed deployment;
the README env table (`apps/anvil-api/README.md:31-46`) is the canonical
operator-facing list.

## 11. `anvil-archive/admin-cli-node/` — historical Node operator CLI

`anvil-archive/admin-cli-node` is a thin Commander-based Node CLI
(`@eddacraft/admin-cli`, binary `anvil-admin`) that pre-dates the Rust CLI admin
surface. Authentication uses `Bearer ${ANVIL_ADMIN_KEY}` directly — no
user-credential bypass, no auth subsystem
(`anvil-archive/admin-cli-node/src/client.ts:72-82`). It is the canonical contract
that RCLI2-009 ports.

### 11.1 Surface (7 subcommands)

`anvil-archive/admin-cli-node/src/index.ts:32-147` registers exactly seven
subcommands. All seven have been ported to `anvil admin` under RCLI2-009
(Status: Complete — `plans/modules/rust-cli-tier2.aps.md:302-360`).

| Subcommand                                                                    | API                                      | File                             | Ported to `anvil admin`?                                                 |
| ----------------------------------------------------------------------------- | ---------------------------------------- | -------------------------------- | ------------------------------------------------------------------------ |
| `list [--status …] [--source …] [--limit N] [--offset N]`                     | `GET /admin/waitlist`                    | `src/commands/list.ts`           | Yes (RCLI2-009)                                                          |
| `show <email>`                                                                | `GET /admin/user/:email`                 | `src/commands/show.ts`           | Yes (RCLI2-009)                                                          |
| `approve [email] [--batch N] [-y]`                                            | `POST /admin/approve`                    | `src/commands/approve.ts`        | Yes (RCLI-016 originally; RCLI2-009 confirmed)                           |
| `invite <email> [--name …] [--notes …] [--days N] [--scope …] [--token-only]` | `POST /admin/invite`                     | `src/commands/invite.ts`         | Yes (RCLI-016 originally; RCLI2-009 confirmed)                           |
| `revoke [email] [--token <raw>] [-y]`                                         | `POST /admin/revoke`                     | `src/commands/revoke.ts`         | Yes (RCLI2-009)                                                          |
| `audit [--action …] [--filter-actor …] [--limit N] [--offset N]`              | `GET /admin/audit`                       | `src/commands/audit.ts`          | Yes (RCLI2-009)                                                          |
| `send-migration [--source …] [--limit N] [--no-dry-run] [-y]`                 | `POST /admin/send-migration` (two-phase) | `src/commands/send-migration.ts` | Yes (RCLI2-009; preserves `previewToken` flow + `cohort_drift` handling) |

### 11.2 New `anvil admin email-update`

`POST /admin/user/email-update` is the only admin endpoint with **no Node CLI
surface** — RCLI2-009 added `anvil admin email-update <current> <new>` as a
net-new Rust command. Operators using the Node CLI for this case must call the
API directly.

### 11.3 Retirement path

RCLI2-009 declared parity Complete. The retirement plan (per its Notes block,
`plans/modules/rust-cli-tier2.aps.md:354-360`) is to archive
`anvil-archive/admin-cli-node/` alongside `anvil-archive/anvil-cli-node/` once the Rust
binary is a release-grade replacement, leaving one operator surface. Until that
archival lands, both CLIs are functional and both target the same `/admin/*`
API. Auth contract is identical: `ANVIL_ADMIN_KEY` (or per-operator key) on
`Authorization: Bearer …`. Note that the Node CLI still sends `X-Admin-Actor`
(`anvil-archive/admin-cli-node/src/client.ts:74`), which the API now ignores by design
(`adminAuth` middleware §5.3) — attribution comes from the key itself, not the
header.

### 11.4 Module structure

| File                | Lines          | Role                                                                                                          |
| ------------------- | -------------- | ------------------------------------------------------------------------------------------------------------- |
| `src/index.ts`      | 214            | Commander program builder + error handler + bin entry                                                         |
| `src/client.ts`     | 161            | `AdminClient` with `Bearer` + `X-Admin-Actor` headers, Zod response validation, exit-code-tagged `AdminError` |
| `src/config.ts`     | 64             | `ANVIL_ADMIN_{URL,KEY,ACTOR}` resolution (with `--key` / `--url` / `--actor` overrides)                       |
| `src/format.ts`     | 86             | Table / colour rendering via `picocolors`                                                                     |
| `src/parsers.ts`    | 14             | Bounded-int Commander parser                                                                                  |
| `src/prompt.ts`     | 54             | Interactive yes/no prompt; `PromptEOFError` on EOF                                                            |
| `src/commands/*.ts` | 720 (combined) | One file per subcommand                                                                                       |

## 12. Cross-cutting concerns

### 12.1 Trust boundary

`anvil-api` is the **only** place beta tokens are minted. All clients (Rust CLI,
Node admin CLI, Rust admin parity, install site) are consumers. The CLI never
holds the `LICENSE_SIGNING_KEY` or the admin key material. Token verification on
the client (e.g. the Rust CLI parsing a licence) is purely consumption — no
client-side issuance path exists. See auth-as-built §"Cross-Cutting Concerns"
for the canonical statement.

### 12.2 Idempotency

| Endpoint                                      | Idempotent?                                       | Notes                                                                                                                                                                                                                     |
| --------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `POST /admin/invite` (default)                | Partially                                         | `beta_users` upsert is idempotent (ON CONFLICT DO UPDATE); `device_codes` insert is **not** (a fresh `user_code` per call); audit logs always written. Re-calling produces a new code but doesn't duplicate the user row. |
| `POST /admin/invite` (`tokenOnly`)            | No                                                | Always mints a new token row + audit row. Re-call = new token.                                                                                                                                                            |
| `POST /admin/approve`                         | Partially                                         | Same shape as invite default; safe to re-call for "pending" entries; once approved, behaviour depends on the existing-user scope read.                                                                                    |
| `POST /admin/revoke`                          | Effectively yes                                   | `UPDATE … WHERE revoked_at IS NULL`; re-call returns `revoked: 0`. Audit row written each call.                                                                                                                           |
| `POST /admin/send-migration` (`dryRun: true`) | No (creates a new snapshot row + token each call) | Old snapshots time out at 10 min                                                                                                                                                                                          |
| `POST /admin/send-migration` (real-send)      | No (one-shot consume)                             | `consumeBroadcastSnapshot` is atomic; second call hits `preview_token_consumed`                                                                                                                                           |
| `POST /admin/broadcast` (both phases)         | No (same shape as send-migration)                 | Shares the snapshot insert / consume path with the send-migration shim                                                                                                                                                    |
| `POST /admin/user/email-update`               | No                                                | Strict precondition: `currentEmail` must match an existing user; second call with the same args 404s                                                                                                                      |
| `POST /waitlist`                              | Yes                                               | Upsert keyed on `email`; returns `isNewSignup: false` for re-signups; email is only sent on new                                                                                                                           |
| `GET /cron/cleanup`                           | Yes (idempotent under empty pending)              | Always returns count of rows deleted in this call                                                                                                                                                                         |

### 12.3 Audit logging

Every admin write goes through `insertAuditLog` (`src/db/queries.ts:230`) with
`(action, actor, metadata, auth_method)`. Verify / refresh / health do not write
audit rows by design — see auth-as-built G-07.

Actions written by non-auth admin paths (cross-link auth-as-built for the
auth-side actions):

- `token.created`, `tokens.revoked`, `token.revoked` (invite, revoke).
- `user.invited`, `user.approved`, `user.approve.collision`,
  `user.approve.scopes_dropped` (approve flow).
- `user.email.updated` (email-update).
- `broadcast.email.dispatch_started`, `broadcast.email.blocked`,
  `broadcast.email.sent` (broadcast real-send — dispatch is audited _before_ the
  send loop so a mid-loop crash still leaves a forensic record; dry-run does not
  write — `src/routes/admin.ts:1210, 1231/1252, 1278`). Metadata carries the
  SHA-256 hash of the preview token, matching the `token_hash` column.
- `migration.email.dispatch_started`, `migration.email.blocked`,
  `migration.email.sent` (send-migration real-send only — legacy event names
  kept by the shim; dry-run does not write).
- `admin.auth.failed` (rejection paths in `adminAuth`).

### 12.4 Determinism

The migration runner is deterministic: same migration set + same DB state ⇒ same
applied set, same hashes, same drift verdict (`src/db/migrate.ts:73-100`). Drift
detection compares recorded SHAs against on-disk SHAs — re-saving a migration
file with whitespace changes is a drift event, not a no-op. This is intentional
and is the guard against "someone edited an applied migration after the fact".

## 13. Known gaps

Auth-side gaps live in [`auth-as-built.md`](auth-as-built.md) §"Gaps and
Hardcoded Values" (G-01 through G-10 there). The gaps below are non-auth or
operationally adjacent.

### G-01: V050F open items still touch this surface

V050F-007 to V050F-015 (open items as of 2026-05-07,
`plans/modules/v050-release-followups.aps.md`):

- **V050F-014 (migration runner CI wiring)** — the runner exists and is callable
  manually, but the GitHub Actions workflow does not yet drive it between the
  `host` job and the Pulumi Up step. Operator workflow is
  `docs/runbooks/db-migrations.md`. **Risk:** Medium — a deploy can ship code
  that depends on an unapplied migration.
- **V050F-015 (`svix>uuid` override removal)** — the workspace-root
  `pnpm.overrides["svix>uuid"]` exception is still required because `svix` has
  not yet shipped an ESM-aware uuid release. The `check-runtime-cjs.cjs` guard
  prevents regression. **Risk:** Low — guarded; remove the override once
  upstream ships.
- **V050F-010 (`WAITLIST_PAUSED` runbook)** — flag is honoured at
  `apps/anvil-api/src/routes/waitlist.ts:19-21` but the operator procedure for
  the DBCON-003 cutover still needs runbook documentation.

V050F items 1-6 are kernel-side and do not touch the API surface.

### G-02: Trace propagation exists; auto-generated request-IDs do not

`traceContext` threads an inbound W3C `traceparent` through the request and
echoes it as `X-Anvil-Traceparent` on the response
(`src/middleware/trace-context.ts:9, 25`), so a client that supplies a trace can
correlate its own calls. Still missing: originating a trace ID when the client
omits `traceparent` (the middleware passes straight through,
`src/middleware/trace-context.ts:17-19`), and keying `createDebugger` output by
the trace — so server-side log correlation for a multi-step admin flow still
depends on the caller. **Risk:** Low. **Fix:** generate a trace context when the
header is absent and thread it through `createDebugger`.

### G-03: Rate limiter is per-instance (cross-link auth-as-built G-04)

Vercel Functions are stateless — the in-memory `Map`
(`src/middleware/rate-limit.ts:19`) resets on cold start and is not shared
across concurrent instances. **Risk:** Medium under load. **Fix:** move to
Vercel KV / Upstash, or use the platform WAF.

### G-04: Node admin CLI retirement — Resolved 2026-06-19 via V060F-019

RCLI2-009 declared parity Complete (the Rust CLI covers all seven Node
subcommands plus a new `email-update`), and admin attribution flows through the
API key rather than the `X-Admin-Actor` header the Node CLI emitted (the API
ignores it — `src/middleware/admin-auth.ts:88-108`, ADMINCLIH-002).

**Resolved 2026-06-19 (V060F-019):** the Node binary was moved out of the
workspace to `anvil-archive/admin-cli-node/` (excluded via `!archive/**`), dropped
from the root `tsconfig.json` references and the `pnpm admin` script.
`anvil admin` is now the only supported operator surface; the archived tool
carries a retirement banner.

### G-05: `_migrations` runner has no rollback path

The runner is forward-only. Each migration is applied in its own transaction
(`src/db/migrate.ts:110-123`), but there is no `down` section and no `revert`
command — the only recovery is a hand-written fix-forward migration. **Risk:**
Low for the current monotonic schema-add pattern. **Fix:** track explicitly
if/when a migration that needs rollback ships.

### G-06: `WAITLIST_PAUSED` is a string env, not a structured flag

`WAITLIST_PAUSED=true` short-circuits new signups
(`src/routes/waitlist.ts:19-21`). Changing it requires a redeploy because Vercel
bakes env vars per deployment. The flag is not in the feature-flag manifest
(`src/lib/feature-flags.ts`). **Risk:** Low — intentional during the DBCON-003
cutover; semantics are clear. **Fix:** migrate to a flag-manifest entry with
operator-visible runbook semantics under V050F-010.

### G-07: `audit_log.actor` carries free-form strings

Actor values are email-shaped on the per-operator path
(`admin_keys.actor_email`) but free-form on the legacy `X-Admin-Actor` path
before ADMINCLIH-002, and the sentinel `shared-key@anvil` is also present.
Filter queries match printable-ASCII (`admin-schemas.ts:85-90`); no
normalisation pass has run over historical rows. **Risk:** Low. **Fix:** add a
one-shot data-cleanup migration once the dual-auth window closes.

## 14. Source references

### `apps/anvil-api/`

| File                                           | Lines | Role                                                                                                                                              |
| ---------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/index.ts`                                 | 145   | Hono app + middleware mount + `/health` + cold-start probes                                                                                       |
| `src/routes/admin.ts`                          | 1306  | All `/admin/*` handlers (invite, approve, revoke, waitlist list, audit list, user lookup, send-migration shim, broadcast two-phase, email-update) |
| `src/routes/admin-schemas.ts`                  | 170   | Zod schemas + types for the admin surface                                                                                                         |
| `src/routes/auth.ts`                           | 143   | `/auth/verify`, `/auth/license/refresh` (canonical doc: auth-as-built)                                                                            |
| `src/routes/auth-device.ts`                    | 235   | Device-code flow (canonical doc: auth-as-built)                                                                                                   |
| `src/routes/auth-otp.ts`                       | 181   | OTP flow (canonical doc: auth-as-built)                                                                                                           |
| `src/routes/auth-session.ts`                   | 122   | JWT refresh (canonical doc: auth-as-built)                                                                                                        |
| `src/routes/auth-github.ts`                    | 246   | GitHub OAuth (canonical doc: auth-as-built)                                                                                                       |
| `src/routes/waitlist.ts`                       | 165   | Public `/waitlist` ingress + token-protected `/waitlist/resend`                                                                                   |
| `src/routes/cron.ts`                           | 63    | `/cron/cleanup` purge job                                                                                                                         |
| `src/middleware/admin-auth.ts`                 | 191   | Per-operator + shared-key admin auth (canonical doc: auth-as-built)                                                                               |
| `src/middleware/rate-limit.ts`                 | 54    | In-memory sliding-window rate limiter                                                                                                             |
| `src/middleware/admin-rate-limit.ts`           | 107   | Per-admin-actor rate limiter (coarse + per-endpoint scopes)                                                                                       |
| `src/middleware/trace-context.ts`              | 39    | W3C `traceparent` parse / echo middleware                                                                                                         |
| `src/lib/token.ts`                             | 42    | `anvil_beta_*` token generation, peppered SHA-256 hashing                                                                                         |
| `src/lib/licence.ts`                           | 81    | ES256 PKCS#8 PEM parse, JWT signing, cold-start verify probe                                                                                      |
| `src/lib/email.ts`                             | 332   | Resend transactional email senders (waitlist, OTP, invite, migration, admin notification)                                                         |
| `src/lib/email-registry.ts`                    | 170   | Template registry (kind: transactional vs broadcast; per-template props schemas)                                                                  |
| `src/lib/broadcast-audiences.ts`               | 189   | Named audience keys + cohort resolvers for the broadcast surface                                                                                  |
| `src/lib/audience.ts`                          | 73    | Resend audience-membership maintenance                                                                                                            |
| `src/lib/audit.ts`                             | 14    | Trivial audit-log helper around `insertAuditLog`                                                                                                  |
| `src/lib/debug.ts`                             | 65    | Namespace-gated debug logger with secret sanitisation                                                                                             |
| `src/lib/device-code.ts`                       | 40    | `user_code` generation + collision retry helper                                                                                                   |
| `src/lib/feature-flags.ts`                     | 129   | `api.scope.*` flag manifest + `resolveApiScope` resolver                                                                                          |
| `src/db/client.ts`                             | 28    | Neon-serverless singleton                                                                                                                         |
| `src/db/queries.ts`                            | 1415  | All SQL queries + Zod row schemas                                                                                                                 |
| `src/db/migrate.ts`                            | 194   | Migration runner library (advisory lock, drift detection, dry-run)                                                                                |
| `src/db/schema.sql`                            | 171   | Authoritative schema for fresh installs                                                                                                           |
| `src/db/migrations/*.sql`                      | —     | 15 forward-only migrations (see §8)                                                                                                               |
| `apps/anvil-api/scripts/migrate.mjs`           | 70    | CLI wrapper around `runMigrations`                                                                                                                |
| `apps/anvil-api/scripts/check-runtime-cjs.cjs` | 43    | `postbuild` guard for `svix>uuid` ESM regression                                                                                                  |
| `vercel.json`                                  | 10    | Build / ignore / cron config                                                                                                                      |
| `package.json`                                 | —     | Engine pin (Node ≥22.13), `migrate` / `migrate:dry-run` scripts                                                                                   |
| `README.md`                                    | 109   | Operator-facing endpoint + env var reference                                                                                                      |

### `anvil-archive/admin-cli-node/`

| File                             | Lines | Role                                                   |
| -------------------------------- | ----- | ------------------------------------------------------ |
| `src/index.ts`                   | 214   | Commander program + bin entry                          |
| `src/client.ts`                  | 161   | `AdminClient` HTTP client with Zod-validated responses |
| `src/config.ts`                  | 64    | `ANVIL_ADMIN_*` env resolution                         |
| `src/format.ts`                  | 86    | Table / colour rendering                               |
| `src/parsers.ts`                 | 14    | Bounded-int Commander parser                           |
| `src/prompt.ts`                  | 54    | Interactive yes/no prompt                              |
| `src/commands/list.ts`           | 78    | `anvil-admin list`                                     |
| `src/commands/show.ts`           | 115   | `anvil-admin show <email>`                             |
| `src/commands/approve.ts`        | 106   | `anvil-admin approve`                                  |
| `src/commands/invite.ts`         | 97    | `anvil-admin invite`                                   |
| `src/commands/revoke.ts`         | 78    | `anvil-admin revoke`                                   |
| `src/commands/audit.ts`          | 86    | `anvil-admin audit`                                    |
| `src/commands/send-migration.ts` | 260   | `anvil-admin send-migration` (two-phase)               |

## 15. Related docs

- Spec / sister as-built: [`auth-as-built.md`](auth-as-built.md) — the canonical
  doc for the auth surface this service mounts; covers JWT licence claims, token
  lifecycle, device-code / OTP / GitHub OAuth flows, the `beta_users` /
  `access_tokens` / `audit_log` schema, and admin auth in detail.
- Sister as-built (shape reference):
  [`intercept-as-built.md`](intercept-as-built.md).
- Runbooks:
  - [`docs/runbooks/admin-cli.md`](../runbooks/admin-cli.md) — operator
    procedures for the Node admin CLI (still authoritative until the Rust port
    is release-grade).
  - [`docs/runbooks/db-migrations.md`](../runbooks/db-migrations.md) — operator
    procedure for the migration runner including drift recovery.
  - [`docs/runbooks/neon-db-operations.md`](../runbooks/neon-db-operations.md) —
    DB-side operations (backups, point-in-time, branch creation).
  - [`docs/runbooks/release-token-scope.md`](../runbooks/release-token-scope.md)
    — token-scope expectations across releases.
  - [`docs/runbooks/waitlist-email-operations.md`](../runbooks/waitlist-email-operations.md)
    — operating the waitlist / migration email flows.
- APS modules:
  - [`plans/modules/api-governance.aps.md`](../../plans/modules/api-governance.aps.md)
    (APGOV — Draft).
  - [`plans/archive/modules/beta-auth-streamline.aps.md`](../../plans/archive/modules/beta-auth-streamline.aps.md)
    (BAUTH — Complete).
  - [`plans/archive/modules/admin-cli.aps.md`](../../plans/archive/modules/admin-cli.aps.md)
    (ADMINCLI — Complete).
  - [`plans/archive/modules/admin-cli-hardening.aps.md`](../../plans/archive/modules/admin-cli-hardening.aps.md)
    (ADMINCLIH — Complete).
  - [`plans/modules/rust-cli-tier2.aps.md`](../../plans/modules/rust-cli-tier2.aps.md)
    (RCLI2-009 — Complete; admin parity).
  - [`plans/modules/v050-release-followups.aps.md`](../../plans/modules/v050-release-followups.aps.md)
    (V050F — open items touching the API surface; see §13).
- Shipping history:
  - [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md) — `0.5.0-beta` migration runner +
    post-release fix bundle context.
  - [`CHANGELOG.md`](../../CHANGELOG.md) — `0.5.0-beta` Added: API migration
    runner; Fixed: API deploy stability (CORS preflight, Vercel routing,
    `svix`/`uuid` runtime override); `0.5.1-beta` follow-ups.
- Public docs: [`apps/anvil-api/README.md`](../../apps/anvil-api/README.md) —
  endpoint table + env-var reference.
