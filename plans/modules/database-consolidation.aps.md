# Neon Project Consolidation

| ID    | Owner      | Status      |
| ----- | ---------- | ----------- |
| DBCON | @eddacraft | In Progress |

## Purpose

The anvil-api historically straddled two Neon projects — `eddacraft-web`
(waitlist only) and `beta-user-tokens` (all seven tables) — with a misleading
KeyVault secret (`website-database-url`) and Pulumi drift between the managed
value and what was actually live on Vercel. Volume is effectively zero today
and the data is almost entirely internal test rows, so rather than migrate the
tangle forward we are resetting: provision a single new Neon project
(`anvil-api-prod`) with the canonical schema, import only the waitlist rows we
care about keeping, cut `anvil-api-database-url` over, and decommission both
legacy projects.

## Known State

- **Two legacy Neon projects** — `eddacraft-web` and `beta-user-tokens` — are
  both reachable and will both be snapshotted before any destructive action.
- **Canonical schema** lives in `apps/anvil-api/src/db/schema.sql` (7 tables:
  beta_users, access_tokens, audit_log, waitlist, device_codes, otp_codes,
  refresh_tokens). Extensions required: `citext`, `pgcrypto`.
- **KeyVault secret rename in flight:** `website-database-url` →
  `anvil-api-database-url` has already been applied to the Pulumi program and
  infra scripts in this branch (`infra/src/vercel.ts`,
  `infra/src/__tests__/vercel.test.ts`, `infra/README.md`,
  `infra/scripts/generate-import-json.sh`,
  `infra/scripts/bootstrap-backend.sh`). The new KeyVault secret value is
  set to the new project's connection string in DBCON-003.
- **Pulumi drift:** the live Vercel `DATABASE_URL` was manually pointed at the
  beta DB while KV still held the waitlist URL. Any `pulumi up` without this
  module's changes would have reverted Vercel to the waitlist DB.
- **`WAITLIST_PAUSED` kill switch** already exists on anvil-api (landed via
  #925, see `apps/anvil-api/src/routes/waitlist.ts`); the operator toggles
  it in the Vercel UI — Pulumi does not manage it.
- **Snapshots retained** for at least 30 days after decommission so any
  "oh wait we needed that one row" ask is recoverable.

## In Scope

- Snapshot both legacy Neon projects (pg_dump) before any destructive action.
- Provision a new Neon project `anvil-api-prod` (Neon MCP / neonctl
  preferred; Neon console as fallback).
- Apply canonical schema from `apps/anvil-api/src/db/schema.sql` to the new
  project.
- Selectively import waitlist rows from `eddacraft-web` (and any curated
  rows from `beta-user-tokens.waitlist`) into the new project, deduped on
  email.
- Set the `anvil-api-database-url` secret in KeyVault to the new project's
  connection string.
- Run `pulumi up` and redeploy anvil-api so functions pick up the new URL.
- Smoke test all anvil-api routes against the new DB.
- Delete the legacy KeyVault secret `website-database-url`.
- Decommission both legacy Neon projects after a soak period.

## Out of Scope

- Schema evolution (additive migrations land in separate modules).
- Beta-user / access-token / audit-log / device-code / otp-code /
  refresh-token row migration — these are test data, not retained.
- ORM selection or query-layer rewrites.
- Performance or load testing.
- Any change to `WAITLIST_PAUSED` wiring — kept as an operator-only toggle.

## Interfaces

**Depends on:**

- `infra/src/vercel.ts` — env var wiring for `DATABASE_URL` via
  `anvil-api-database-url`.
- `infra/src/keyvault.ts` — KeyVault secret fetch during `pulumi up`.
- `apps/anvil-api/src/db/schema.sql` — canonical schema applied to the new
  project.
- `apps/anvil-api/src/db/client.ts` — consumer of `DATABASE_URL`.

**Exposes:**

- Single Neon project (`anvil-api-prod`) backing anvil-api.
- KeyVault secret `anvil-api-database-url` with accurate naming.

## Constraints

- No production connection strings in commits, logs, CI output, or
  conversation transcripts — always fetch via `az keyvault secret show` at
  the point of use and keep them in shell env vars.
- Stay within Neon free-tier limits (one non-trivial project at a time
  outside the migration window).
- Both `citext` and `pgcrypto` extensions must be enabled on
  `anvil-api-prod` before anvil-api connects.
- Snapshots retained on local disk (and/or a private Azure Blob) for 30+
  days post-decommission before purge.
- Cutover window expects brief (≤ 5 min) waitlist write unavailability via
  `WAITLIST_PAUSED=true` on anvil-api — acceptable at current volume.

## Migration Strategy

1. **Snapshot** both legacy Neon projects via `pg_dump` into
   `scripts/dbcon/snapshots/` with ISO-8601 timestamps. Gzip. Verify each
   archive is restorable by round-tripping into a throwaway local
   container.
2. **Provision** `anvil-api-prod` via `neonctl` in region
   `aws-eu-west-2` (London). MCP is optional and known to misbehave from
   git worktrees. Capture the connection string into a local env var,
   never into a file.
3. **Apply schema** by running `psql -f apps/anvil-api/src/db/schema.sql`
   against the new project. Verify `\dt` shows all 7 tables and
   `citext`/`pgcrypto` extensions are present.
4. **Export waitlist rows** from `eddacraft-web` (and separately from
   `beta-user-tokens.waitlist` if it has anything worth keeping) to CSV
   via `scripts/dbcon/export-waitlist.sh`.
5. **Import** into `anvil-api-prod` via `scripts/dbcon/import-waitlist.sh`
   — uses a TEMP staging table and `INSERT … ON CONFLICT (email) DO
   NOTHING` so two sources can be imported back-to-back without dupes.
6. **Verify** counts and spot-check a few emails via
   `scripts/dbcon/verify-counts.sh`.
7. **Pause waitlist writes:** set `WAITLIST_PAUSED=true` on the anvil-api
   Vercel project (Vercel UI, not Pulumi) and redeploy. `POST /waitlist`
   now returns 503. At current volume this is a belt-and-braces step; it
   can be skipped if we accept the tiny delta risk.
8. **Set KeyVault secret** `anvil-api-database-url` to the new connection
   string (`az keyvault secret set --vault-name kv-iac-anvil --name
   anvil-api-database-url --value "$ANVIL_API_PROD_URL"`).
9. **Merge rename branch** to `dev` so Pulumi reads the new secret name.
10. **`pulumi up`** from the new state — Vercel `DATABASE_URL` now comes
    from `anvil-api-prod`.
11. **Redeploy anvil-api** so functions pick up the new env var.
12. **Smoke test:** `/health`, `POST /waitlist` (after unpause),
    admin-flow readbacks, cron cleanup endpoint.
13. **Unset `WAITLIST_PAUSED`** and redeploy once more.
14. **Delete legacy KV secret** `website-database-url` via
    `az keyvault secret delete`.
15. **Soak** for ~48h.
16. **Decommission** both legacy Neon projects after the soak window, with
    snapshots retained for 30+ days.

## Risks

| Risk                                         | Impact | Mitigation                                                                       |
| -------------------------------------------- | ------ | -------------------------------------------------------------------------------- |
| Losing a row someone actually cared about    | medium | Full pg_dump snapshots of both projects retained 30+ days before destructive act |
| Schema drift between schema.sql and live DB  | high   | Apply schema.sql fresh on a new DB — becomes the definition, not the observer   |
| Pulumi up reverts DATABASE_URL mid-migration | high   | Do KV set + merge rename before `pulumi up`; do not run `pulumi up` before step 8 |
| Connection string leaks in logs/transcripts  | high   | Env-var indirection via `az keyvault secret show`; never paste into chat         |
| MCP servers flaky from git worktrees         | low    | `neonctl` is the primary tool; MCP is a convenience only. Web console as last resort |

## Ready Checklist

Change status to **Ready** when:

- [x] Inventory of data in both legacy Neon projects documented
- [x] Target project name agreed (`anvil-api-prod`)
- [x] Canonical schema confirmed at `apps/anvil-api/src/db/schema.sql`
- [x] Secret rename (`website-database-url` → `anvil-api-database-url`)
      applied across infra files in this branch
- [ ] `neonctl` installed and authenticated locally (MCP is optional)

---

### DBCON-001: snapshot legacy Neon projects

- **Status:** Complete
- **Intent:** Take gzipped `pg_dump` snapshots of both `eddacraft-web`
  and `beta-user-tokens` before any destructive action, using
  `scripts/dbcon/snapshot-db.sh`. Verify each archive is restorable into
  a throwaway local Postgres.
- **Expected Outcome:** Two snapshot files under
  `scripts/dbcon/snapshots/` (gitignored), each round-trip-tested.
  Retained ≥ 30 days past decommission.
- **Validation:** `ls scripts/dbcon/snapshots/*.sql.gz` shows two recent
  archives. `gunzip -c <snap> | psql <throwaway> -f -` completes without
  errors and row counts match `SELECT count(*)` against the live
  source.
- **Confidence:** high
- **Files:**
  - Add: `scripts/dbcon/snapshot-db.sh`
  - Modify: `scripts/dbcon/README.md`
  - Modify: `.gitignore` (exclude `scripts/dbcon/snapshots/`)

### DBCON-002: provision anvil-api-prod and apply schema

- **Status:** Complete
- **Intent:** Create a new Neon project named `anvil-api-prod` via
  `neonctl` in region `aws-eu-west-2` (London). Apply the canonical
  schema from `apps/anvil-api/src/db/schema.sql`. Ensure `citext` and
  `pgcrypto` extensions are enabled. Keep the connection string in a
  local env var — never commit or paste it.
- **Expected Outcome:** `anvil-api-prod` exists, reachable via psql,
  has all 7 tables from schema.sql and both required extensions.
- **Validation:** `\dt` returns 7 tables; `SELECT extname FROM
  pg_extension` includes `citext` and `pgcrypto`; `SELECT count(*) FROM
  waitlist` returns 0.
- **Confidence:** medium
- **Files:**
  - Add: `scripts/dbcon/apply-schema.sh`
  - Modify: `scripts/dbcon/README.md`

### DBCON-003: selective import and infra cutover

- **Status:** Complete
- **Intent:** Import waitlist rows from the legacy projects into
  `anvil-api-prod` (deduped on email), then flip infra over. Sequence:
  (1) `WAITLIST_PAUSED=true` + redeploy so `POST /waitlist` returns
  503; (2) export/import waitlist rows from `eddacraft-web` and any
  curated rows from `beta-user-tokens`; (3) set KeyVault secret
  `anvil-api-database-url` to the new connection string; (4) merge the
  rename branch so Pulumi reads the new secret name; (5) `pulumi up`;
  (6) redeploy anvil-api; (7) smoke-test; (8) unset `WAITLIST_PAUSED`
  + redeploy; (9) delete the legacy KV secret
  `website-database-url`.
- **Expected Outcome:** anvil-api serves all routes against
  `anvil-api-prod`. Waitlist count on the new DB equals dedup union of
  both sources. Legacy secret removed from KeyVault.
- **Validation:**
  - Pre-cutover: `POST /waitlist` returns 503 while paused.
  - Post-cutover: `GET /health` 200; `POST /waitlist` 2xx; cron cleanup
    returns 200; row counts on `anvil-api-prod.waitlist` match expected
    dedup total.
  - `az keyvault secret show --vault-name kv-iac-anvil --name
    website-database-url` returns NotFound.
- **Confidence:** medium
- **Files:**
  - Add: `scripts/dbcon/export-waitlist.sh`
  - Add: `scripts/dbcon/import-waitlist.sh`
  - Add: `scripts/dbcon/verify-counts.sh`
  - Modify: `scripts/dbcon/README.md`

### DBCON-004: decommission legacy Neon projects

- **Status:** Ready
- **Intent:** After a ≥ 48h soak period on `anvil-api-prod`, delete both
  `eddacraft-web` and `beta-user-tokens` Neon projects. Retain local
  snapshots (gitignored) for 30+ days afterwards.
- **Expected Outcome:** Only `anvil-api-prod` remains in the Neon
  account. No references to `website-database-url`,
  `eddacraft-web`, or `beta-user-tokens` remain in the codebase or
  infra.
- **Validation:** Neon console/API lists exactly one project. `rg -n
  "website-database-url|eddacraft-web|beta-user-tokens"` returns no
  hits outside archived plan docs.
- **Confidence:** high
- **Files:**
  - Modify: `scripts/dbcon/README.md` (decommission checklist)
