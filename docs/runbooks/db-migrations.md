# Database Migrations Runbook

| Type    | Authority     | Owner  | Status | Freshness                                                                                           |
| ------- | ------------- | ------ | ------ | --------------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | Last reviewed 2026-05-24 against v0.4.0-beta prod backfill and `apps/anvil-api/scripts/migrate.mjs` |

| Upstream                                                            | Downstream                                                  |
| ------------------------------------------------------------------- | ----------------------------------------------------------- |
| `apps/anvil-api/scripts/migrate.mjs`, `.github/workflows/infra.yml` | on-call operators, release council, post-deploy smoke check |

## Purpose

Apply pending SQL migrations to the Anvil API database (Neon Postgres), verify
drift, and recover from a failed deploy.

Migrations live as plain SQL files under `apps/anvil-api/src/db/migrations/` and
are applied in lexical filename order by the runner at
`apps/anvil-api/scripts/migrate.mjs`.

## When to use

- CI applies migrations automatically. The Infrastructure workflow's `up` job
  runs the runner against prod between Azure Login and Pulumi Up, on push to
  `main` whenever `apps/anvil-api/src/db/migrations/**`,
  `apps/anvil-api/src/db/migrate.ts`, `apps/anvil-api/scripts/migrate.mjs`,
  `infra/**`, `pnpm-lock.yaml`, or `.github/workflows/infra.yml` changes.
- Manual invocation is for: recovery after a failed deploy where the CI migrate
  step did not run, ad-hoc apply against a staging database, or local
  development. The procedure below covers those paths.

## Required env vars

- `DATABASE_URL` — Postgres connection string. Pull production from Vercel via
  `vercel env pull .env.production --environment=production` (do not commit).
  Staging or local use a different value — keep them separated.

## Behaviour

The runner does three things, in order:

1. **Ensures the tracking table.** Creates
   `_migrations(filename TEXT PRIMARY KEY, sha256 TEXT NOT NULL, applied_at TIMESTAMPTZ DEFAULT NOW())`
   if it does not already exist.
2. **Verifies drift.** Reads existing rows from `_migrations`. For every
   filename present on disk, compares the on-disk sha256 to the recorded sha. If
   any recorded migration's sha differs from disk, the runner refuses to apply
   anything and exits 1 with a per-file diff.
3. **Applies pending.** For every `*.sql` file in lexical order whose filename
   is not in `_migrations`, executes the file contents in a transaction (`BEGIN`
   → file SQL → `INSERT INTO _migrations` → `COMMIT`). On any error the
   transaction rolls back.

A `--dry-run` flag stops after step 2 and prints the pending list without
applying anything.

## Exact commands

The CLI imports the compiled lib from `apps/anvil-api`'s generated `dist/`
output, so the API must be built first. From the repo root:

```bash
pnpm --filter @eddacraft/anvil-api run build
```

The build also runs the postbuild CJS smoke check; expect `ok: require('svix')`
and exit 0.

### 1. Apply pending migrations

```bash
DATABASE_URL='postgres://...' \
  node apps/anvil-api/scripts/migrate.mjs
```

Expected output (typical):

```
Discovered 10 migration files. 8 applied, 2 pending.
applying 009-audit-log-auth-method.sql (sha256=ab12cd34ef56)
applying 010-access-tokens-scope-index.sql (sha256=78aa90bb12cd)

applied 2 migration(s):
  ✓ 009-audit-log-auth-method.sql
  ✓ 010-access-tokens-scope-index.sql
```

Idempotent on re-run — a second invocation reports `no pending migrations.` and
exits 0.

### 2. Dry-run (report without applying)

```bash
DATABASE_URL='postgres://...' \
  node apps/anvil-api/scripts/migrate.mjs --dry-run
```

Use before applying against production to confirm the pending list matches what
the release introduces.

### 3. Verify against a database without applying

A dry-run on a current database logs the discovery summary
(`Discovered N migration files. N applied, 0 pending.`) and exits 0 without
printing the regular `no pending migrations.` line — the runner reaches the
dry-run early-return after the discovery log. The drift check still runs every
time, so a tampered file fails the dry-run with a non-zero exit and a per-file
report regardless of whether anything is pending.

## Failure modes + recovery

### A. Drift detected

```
migration failed: Migration drift detected — applied migrations have
changed on disk:
  005-audit-log-indexes.sql: recorded sha=ab12cd34ef56 on-disk sha=78aa90bb12cd
Refusing to apply. Investigate the diff or revert the file before
re-running.
```

This means a SQL file that was already applied to the target database has been
edited on disk after the fact. The runner refuses to proceed because re-applying
would either fail (DDL conflicts) or silently leave the database in a state that
no longer matches its `_migrations` record.

**Recovery**:

1. `git log --follow apps/anvil-api/src/db/migrations/<file>.sql` — identify
   when the file was edited.
2. Decide intent:
   - **The edit was a mistake** — revert the file to the state matching
     `_migrations.sha256`, then re-run.
   - **The edit was intentional and is already applied to the database** —
     manually update the recorded sha:
     ```sql
     UPDATE _migrations
        SET sha256 = '<new-on-disk-sha>'
      WHERE filename = '<file>.sql';
     ```
     Compute the new sha with
     `sha256sum apps/anvil-api/src/db/migrations/<file>.sql`. Document the
     rationale in the next deploy summary.
   - **The edit was intentional but NOT yet applied** — never. Add a new
     migration file (e.g. `011-…`) that performs the additional change instead.
     Editing an already-applied migration is forward-incompatible with anyone
     who has run the older version.

### B. SQL error during apply

```
applying 011-foo.sql (sha256=...)
migration failed: relation "bar" does not exist
```

Transaction rolled back; the database is unchanged for this migration. Earlier
migrations in the same run remain applied (each file is its own transaction).
Fix the migration on disk, push the fix, re-run — the runner picks up where it
left off because the failed file was never recorded in `_migrations`.

### C. `DATABASE_URL` missing

```
error: DATABASE_URL environment variable is required
```

Set the env var (see "Required env vars") and retry.

## Backfilling `_migrations` on an existing database

This is needed once, the first time the runner is enabled against a database
that already has migrations applied by hand (the v0.4.0-beta case for prod).
Only backfill migrations known to have already run in that environment; do not
record new pending migrations as applied. For the v0.4.0-beta prod cut, that is
`001` through `010` only:

```bash
psql "$DATABASE_URL" <<'SQL'
CREATE TABLE IF NOT EXISTS _migrations (
  filename    TEXT PRIMARY KEY,
  sha256      TEXT NOT NULL,
  applied_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
SQL

for f in apps/anvil-api/src/db/migrations/00[1-9]-*.sql apps/anvil-api/src/db/migrations/010-*.sql; do
  name=$(basename "$f")
  sha=$(sha256sum "$f" | awk '{print $1}')
  psql "$DATABASE_URL" -c "
    INSERT INTO _migrations (filename, sha256)
    VALUES ('$name', '$sha')
    ON CONFLICT (filename) DO NOTHING;
  "
done
```

After backfill, the next runner invocation should report
`011-access-tokens-edict-flag.sql` as pending. Apply it with the migration
runner so the schema change and `_migrations` row are written together.

## Cross-references

- Module: `plans/modules/v050-release-followups.aps.md` §V050F-014
- Smoke check: `docs/runbooks/post-deploy-smoke-check.md`
- DB operations: `docs/runbooks/neon-db-operations.md`
- A related one-shot data script (not a schema migration) using the same runner
  shape: `docs/runbooks/account-activity-backfill.md` (BACT-012)
