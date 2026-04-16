# Neon Project Consolidation

| ID    | Owner      | Status   |
| ----- | ---------- | -------- |
| DBCON | @eddacraft | Proposed |

## Purpose

The anvil-api connects to two separate Neon projects — one for the waitlist
and one for beta auth/tokens — despite only one being intended. The beta DB
has all 7 tables (beta_users, access_tokens, audit_log, device_codes,
otp_codes, refresh_tokens, waitlist) but is missing the actual waitlist rows.
The waitlist DB only has the waitlist table with the live data. This module
merges the waitlist data into the beta DB and decommissions the waitlist
project.

## Known State

- **Beta DB** (target): full schema per `apps/anvil-api/src/db/schema.sql`,
  has citext + pgcrypto extensions, all tables present. May have expired
  device codes and OTP codes that the cron never cleaned up (the cron was returning HTTP 500s
  against the wrong DB for 24+ hours).
- **Waitlist DB** (secondary): only the waitlist table with live rows.
  Missing: beta_users, access_tokens, audit_log, device_codes, otp_codes,
  refresh_tokens.
- **KeyVault secret**: `website-database-url` is misleading — it's used by
  anvil-api, not the website. Rename to `api-database-url` during cutover.
- **Env sync**: `DATABASE_URL` update via `pulumi up` is sufficient — no
  redeploy needed since serverless functions read env vars at invocation
  time.

## In Scope

- Migrate waitlist rows into the beta DB
- Run expired-row cleanup on beta DB (backlog from broken cron)
- Rename KeyVault secret from `website-database-url` to `api-database-url`
- Update Pulumi config and sync env vars via `pulumi up`
- Decommission the waitlist Neon project

## Out of Scope

- Schema changes or new tables
- ORM selection or migration
- Query-layer test coverage (separate module)
- Performance or load testing

## Interfaces

**Depends on:**

- `infra/src/vercel.ts` — environment variable configuration
- `apps/anvil-api/src/db/client.ts` — database connection setup

**Exposes:**

- Single consolidated database connection for all anvil-api routes

## Constraints

- Data migration must be zero-downtime (both projects temporarily active)
- Zero-downtime cutover must explicitly handle concurrent waitlist writes:
  perform an initial copy while the source DB stays live, then pause new
  waitlist signups briefly for a final delta sync immediately before
  switching `DATABASE_URL`
- No production connection strings in logs or CI output
- Must stay within Neon free-tier limits
- Both citext and pgcrypto extensions must be enabled in target DB

## Migration Strategy

1. Verify target beta DB schema matches `apps/anvil-api/src/db/schema.sql`
   and confirm `citext` + `pgcrypto` are enabled.
2. Take a backup/export of the waitlist DB before any writes are copied.
3. Run an initial bulk copy of `waitlist` rows from the waitlist DB into the
   beta DB while the existing waitlist DB remains the live write path.
4. Compare row counts and sample a small set of records by deterministic key
   (email) between source and target to confirm the initial copy succeeded.
5. Immediately before cutover, temporarily pause new waitlist signups so no
   new rows can land only in the source DB during the final migration window.
6. Run a final delta sync from the waitlist DB to the beta DB for any rows
   created after the initial copy, matching on email to avoid duplicates.
7. Re-run row-count and spot-check verification, then switch KeyVault/Pulumi
   from the old secret to `api-database-url` and update `DATABASE_URL` to the
   consolidated beta DB.
8. Resume waitlist signups after the env var change is live and validated.
9. Keep the old waitlist DB available briefly as a rollback source, then
   decommission it once production traffic is confirmed healthy.

## Risks

| Risk                                        | Impact | Mitigation                                                           |
| ------------------------------------------- | ------ | -------------------------------------------------------------------- |
| Data loss during waitlist row migration     | high   | Backup first, initial copy + final delta sync, pause signups briefly |
| KeyVault rename breaks references           | high   | Grep for old secret name across all infra/config first               |
| Connection string confusion during cutover  | medium | Stage env var changes, verify in preview before prod                 |

## Ready Checklist

Change status to **Ready** when:

- [x] Inventory of data in both Neon projects documented
- [x] Target Neon project chosen — beta DB survives

---

### DBCON-001: migrate waitlist data

- **Intent:** Copy waitlist rows from the waitlist DB into the beta DB's
  existing waitlist table. Verify row counts match after migration.
- **Expected Outcome:** All waitlist rows present in the beta DB. Waitlist
  DB still operational as fallback.
- **Validation:** Row count in beta DB waitlist table matches source.
  Spot-check records by email.
- **Confidence:** high

### DBCON-002: clean up expired rows in beta DB

- **Intent:** Run the expired-row cleanup that the cron has been failing to
  do — clear out expired device codes, OTP codes, and refresh tokens that
  accumulated while the cron was returning HTTP 500s.
- **Expected Outcome:** No expired rows remain in device_codes, otp_codes,
  or refresh_tokens.
- **Validation:** Query for rows past expiry returns zero.
- **Confidence:** high

### DBCON-003: infrastructure cutover

- **Intent:** Rename KeyVault secret from `website-database-url` to
  `api-database-url`. Update Pulumi config to point `DATABASE_URL` at the
  beta DB for all environments. Run `pulumi up` to sync.
- **Expected Outcome:** All environments use the single Neon project.
  Secret name accurately reflects its consumer.
- **Validation:** `pulumi preview` shows the rename and URL update.
  All API routes functional in preview deployment.
- **Confidence:** medium
- **Files:**
  - Modify: `infra/src/vercel.ts`

### DBCON-004: decommission waitlist project

- **Intent:** After validation period, delete the waitlist Neon project.
- **Expected Outcome:** Single Neon project with all tables and data.
  No dangling references to old connection string or secret name.
- **Validation:** `neon projects list` shows one project. Grep codebase
  for old secret name returns zero hits.
- **Confidence:** high
