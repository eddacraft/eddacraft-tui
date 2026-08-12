# Account Activity Backfill Runbook (BACT-012)

| Type    | Authority     | Owner  | Status | Freshness                                      |
| ------- | ------------- | ------ | ------ | ---------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | Last reviewed 2026-08-13 against BACT-012 ship |

| Upstream                                                                                               | Downstream                                              |
| ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------- |
| `apps/anvil-api/scripts/backfill-activity.mjs`, `apps/anvil-api/src/db/activity-backfill.ts`, BACT-008 | on-call operators, CS/product metrics, `admin activity` |

## Purpose

One-shot operational script that seeds `beta_users.last_activity_at` (BACT-008,
ADR-121) from `max(refresh_tokens.created_at)` for accounts that have never
recorded any activity. Token-era accounts refresh sessions without ever minting
a fresh interactive login, so without this backfill they show up as "never
active" in `anvil admin activity` (BACT-009) even though they were recently
using the product.

This is a **proxy for activity, not for login history**. It never sets
`first_login_at` / `last_login_at` / `last_login_method` — those columns are
owned exclusively by the interactive-mint paths (BACT-002, OQ-B). An account
backfilled by this script still shows "never logged in" in `admin show` /
`admin users --engagement never_logged_in`; it only stops showing as
activity-quiet in `admin activity`.

## When to use

- **Once**, shortly after BACT-008 (`last_activity_at`) first ships to
  production, to avoid every pre-existing token-era account reading as
  activity-quiet on day one.
- Not a recurring or scheduled job — there is no cron wiring for this script, by
  design (BACT-012 non-scope: continuous sync). Ongoing activity is kept current
  by the live stamp paths (login, refresh, feature-touch) shipped under
  BACT-008.
- Safe to re-run at any time; the only-null guard makes it a no-op once every
  eligible account has been backfilled (see **Idempotency** below).

## Required env vars

- `DATABASE_URL` — Postgres connection string. Pull production from Vercel via
  `vercel env pull .env.production --environment=production` (do not commit).
  Staging or local use a different value — keep them separated.

## Behaviour

The script issues exactly one SQL statement per invocation:

- **Dry-run (default):** a read-only `SELECT count(*)` of accounts where
  `last_activity_at IS NULL` **and** at least one `refresh_tokens` row exists
  for the account. Nothing is written.
- **`--apply`:** an `UPDATE beta_users … WHERE last_activity_at IS NULL` joined
  against `max(refresh_tokens.created_at)` per account, setting
  `last_activity_at` to that max and `last_activity_kind = 'refresh'`. The
  `WHERE last_activity_at IS NULL` guard means an account with any prior
  activity (login, refresh-stamp, or feature-touch) is never touched.

Accounts with **no** `refresh_tokens` rows at all are left `NULL` — this script
only ever proxies from token history that exists; it does not fabricate activity
for accounts that never authenticated at all.

Both modes report the affected-row count.

## Exact commands

The CLI imports the compiled lib from `apps/anvil-api`'s generated `dist/`
output, so the API must be built first. From the repo root:

```bash
pnpm --filter @eddacraft/anvil-api run build
```

### 1. Dry-run (default — report only)

```bash
DATABASE_URL='postgres://...' \
  node apps/anvil-api/scripts/backfill-activity.mjs
```

Expected output (example):

```
--dry-run: would backfill last_activity_at (kind=refresh) for 42 account(s) from max(refresh_tokens.created_at). No rows written. Re-run with --apply to write.

42 account(s) would be backfilled. Re-run with --apply to write.
```

Use this before `--apply` to confirm the affected count is sane (compare against
a rough sense of admitted-but-quiet accounts from
`anvil admin users --engagement never_logged_in`).

### 2. Apply (write)

```bash
DATABASE_URL='postgres://...' \
  node apps/anvil-api/scripts/backfill-activity.mjs --apply
```

Expected output (example):

```
backfilled last_activity_at (kind=refresh) for 42 account(s) from max(refresh_tokens.created_at).

backfilled 42 account(s).
```

### 3. `pnpm` shortcuts

From `apps/anvil-api`:

```bash
pnpm backfill:activity          # dry-run
pnpm backfill:activity:apply    # write
```

## Idempotency

Re-running `--apply` after a successful run reports `0` — every account that had
refresh tokens and a null `last_activity_at` now has a non-null
`last_activity_at`, so the `WHERE last_activity_at IS NULL` guard excludes them
on the next pass. Safe to re-run after new accounts are admitted; it only ever
picks up accounts that are still genuinely null.

## What this does NOT do (non-scope)

- **Never** sets `first_login_at`, `last_login_at`, or `last_login_method`
  (OQ-B) — do not read a backfilled `last_activity_at` as evidence of
  interactive login. Use `admin show` /
  `admin users --engagement never_logged_in` for login history, which this
  script never changes.
- Does not run continuously or on a schedule — it is a one-shot proxy for the
  gap between "BACT-008 ships" and "every token-era account eventually refreshes
  and stamps its own activity."
- Does not touch accounts that already have any `last_activity_at` (login,
  refresh-stamp, or feature-touch) — only the genuinely-null rows.
- Does not compute or write the historical daily rollup (BACT-011, a separate
  in-flight item) — this script only ever sets the single current
  `last_activity_at` field.

## Cross-references

- Schema and stamp paths: `plans/modules/beta-account-activity.aps.md`
  (BACT-008, BACT-012),
  [ADR-121](../../plans/decisions/121-account-plan-activity-and-flag-entitlements.md),
  [design spec](../../plans/specs/2026-08-12-account-plan-activity-entitlements.md)
- Operator vocabulary and `admin activity` metrics: `docs/runbooks/admin-cli.md`
- Migration script pattern this mirrors: `docs/runbooks/db-migrations.md`
