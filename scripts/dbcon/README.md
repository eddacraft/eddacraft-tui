# DBCON operator scripts (Option B — Neon reset)

Scripts for the Neon reset laid out in
`plans/modules/database-consolidation.aps.md` and the runbook at
`plans/execution/DBCON.steps.md`.

## Env vars

| Var               | Role                                                             |
| ----------------- | ---------------------------------------------------------------- |
| `WAITLIST_DB_URL` | A legacy source being exported (eddacraft-web, beta-user-tokens) |
| `BETA_DB_URL`     | The new target project — `anvil-api-prod`                        |

The variable names are kept (rather than renamed to SOURCE/TARGET) so the
scripts stay identical to what the runbook snippets expect. Keep URLs in the
shell — never paste them into a file.

All three connection strings come from Neon (Vercel stores env vars encrypted
and cannot read them back; KeyVault only has the old `eddacraft-web` URL under
the misleading `website-database-url` name).

```bash
neonctl auth          # one-time OAuth
neonctl projects list # grab the IDs

export EDDACRAFT_WEB_URL=$(neonctl connection-string \
  --project-id <eddacraft-web-id>)
export BETA_USER_TOKENS_URL=$(neonctl connection-string \
  --project-id <beta-user-tokens-id>)
export ANVIL_API_PROD_URL=$(neonctl connection-string \
  --project-id <anvil-api-prod-id>)
```

## Scripts

| Script               | Purpose                                                    |
| -------------------- | ---------------------------------------------------------- |
| `snapshot-db.sh`     | `pg_dump \| gzip` a Neon DB into `snapshots/` (gitignored) |
| `apply-schema.sh`    | `psql -f apps/anvil-api/src/db/schema.sql` on a target DB  |
| `export-waitlist.sh` | Export waitlist rows from a source to CSV                  |
| `import-waitlist.sh` | Idempotent import of a CSV into the target DB              |
| `verify-counts.sh`   | Row-count + email-set parity between one source and target |

## Order of operations

Full sequence lives in `plans/execution/DBCON.steps.md`. In short:

```bash
cd scripts/dbcon

# DBCON-001 — snapshot both legacy projects
./snapshot-db.sh "$EDDACRAFT_WEB_URL"     eddacraft-web
./snapshot-db.sh "$BETA_USER_TOKENS_URL"  beta-user-tokens

# DBCON-002 — create anvil-api-prod via neonctl, then:
./apply-schema.sh "$ANVIL_API_PROD_URL"

# DBCON-003 — import waitlist data from both legacy sources
WAITLIST_DB_URL="$EDDACRAFT_WEB_URL"     ./export-waitlist.sh waitlist-eddacraft-web.csv
WAITLIST_DB_URL="$BETA_USER_TOKENS_URL"  ./export-waitlist.sh waitlist-beta-user-tokens.csv

BETA_DB_URL="$ANVIL_API_PROD_URL" ./import-waitlist.sh waitlist-eddacraft-web.csv
BETA_DB_URL="$ANVIL_API_PROD_URL" ./import-waitlist.sh waitlist-beta-user-tokens.csv

WAITLIST_DB_URL="$EDDACRAFT_WEB_URL"     BETA_DB_URL="$ANVIL_API_PROD_URL" ./verify-counts.sh
WAITLIST_DB_URL="$BETA_USER_TOKENS_URL"  BETA_DB_URL="$ANVIL_API_PROD_URL" ./verify-counts.sh
```

All scripts are idempotent — re-running after a partial success is safe.
`import-waitlist.sh` runs inside a transaction, so a failure mid-copy rolls back
without leaving partial rows.

## Snapshots

`snapshots/` is gitignored. Retain snapshots for ≥ 30 days after DBCON-004
decommissions the legacy projects before purging.
