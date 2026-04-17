# DBCON — Neon Reset Runbook (Option B)

Operational sequence for resetting the anvil-api database onto a brand-new
Neon project (`anvil-api-prod`), importing only the waitlist rows we want to
keep, and decommissioning both legacy projects. Covers DBCON-001 through
DBCON-004.

Paired scripts: `scripts/dbcon/*` (snapshot / apply-schema / export /
import / verify).

## Goal

All anvil-api traffic runs against a single new Neon project,
`anvil-api-prod`, populated from the canonical schema with only the waitlist
rows curated from the legacy projects. Both legacy projects
(`eddacraft-web`, `beta-user-tokens`) deleted. KeyVault secret renamed to
`anvil-api-database-url`. Old KV secret (`website-database-url`) deleted.

## Preconditions

- Azure CLI logged in (`az account show` succeeds)
- Pulumi logged in, `prod` stack selected (`pulumi stack`)
- `psql` and `pg_dump` available locally
- `neonctl` installed and authenticated (`neonctl me` succeeds). MCP-based
  provisioning is optional — MCP is known to misbehave from git worktrees,
  so `neonctl` is the primary path.
- Vercel project access for anvil-api (to toggle `WAITLIST_PAUSED` and
  trigger redeploys in the UI)
- Rename branch (this one) merged to `dev` before step 9 — Pulumi reads
  `anvil-api-database-url` from KV, so the secret must exist and the
  program must reference the new name before `pulumi up`
- Legacy connection strings exported into the shell for snapshot + export:
  ```bash
  export EDDACRAFT_WEB_URL=$(az keyvault secret show --vault-name kv-iac-anvil \
    --name website-database-url --query value -o tsv)
  # beta-user-tokens connection string is what's currently live on the
  # anvil-api Vercel project (drift from KV). Copy it from the Vercel UI
  # into the shell without pasting into any file:
  read -rs -p 'BETA_USER_TOKENS_URL: ' BETA_USER_TOKENS_URL; export BETA_USER_TOKENS_URL; echo
  ```

## Steps

### DBCON-001 — snapshot legacy projects

> Safe to run at any time. Both legacy DBs stay live.

1. **Snapshot `eddacraft-web`**
   ```bash
   cd scripts/dbcon
   ./snapshot-db.sh "$EDDACRAFT_WEB_URL" eddacraft-web
   ```
   Produces `scripts/dbcon/snapshots/eddacraft-web-<ts>.sql.gz`.

2. **Snapshot `beta-user-tokens`**
   ```bash
   ./snapshot-db.sh "$BETA_USER_TOKENS_URL" beta-user-tokens
   ```

3. **Round-trip test** — restore each snapshot into a throwaway local
   Postgres and confirm row counts match:
   ```bash
   docker run --rm -d --name dbcon-verify -e POSTGRES_PASSWORD=verify \
     -p 55432:5432 postgres:16
   sleep 3
   for snap in snapshots/*.sql.gz; do
     db="verify_$(basename "$snap" .sql.gz | tr -c 'a-z0-9' _)"
     psql "postgres://postgres:verify@localhost:55432/postgres" \
       -c "CREATE DATABASE \"$db\""
     gunzip -c "$snap" | psql "postgres://postgres:verify@localhost:55432/$db"
   done
   docker rm -f dbcon-verify
   ```

4. **Retention** — keep `scripts/dbcon/snapshots/` for 30+ days after
   DBCON-004 completes. Directory is gitignored.

### DBCON-002 — provision anvil-api-prod + apply schema

5. **Create the project**
   ```bash
   neonctl projects create --name anvil-api-prod --region-id <same-as-vercel>
   # capture the connection string into a local env var — do NOT paste or log it
   export ANVIL_API_PROD_URL=$(neonctl connection-string --project-id <id> --role-name <role>)
   ```

6. **Apply canonical schema**
   ```bash
   cd scripts/dbcon
   ./apply-schema.sh "$ANVIL_API_PROD_URL"
   ```

7. **Verify schema**
   ```bash
   psql "$ANVIL_API_PROD_URL" -c "\dt"
   # expect 7 tables: beta_users, access_tokens, audit_log, waitlist,
   # device_codes, otp_codes, refresh_tokens

   psql "$ANVIL_API_PROD_URL" -c \
     "SELECT extname FROM pg_extension ORDER BY extname"
   # must include: citext, pgcrypto

   psql "$ANVIL_API_PROD_URL" -c "SELECT count(*) FROM waitlist"
   # expect 0
   ```

### DBCON-003 — selective import + infra cutover

> **This is the irreversible-ish block.** Once `pulumi up` flips
> `DATABASE_URL`, the legacy projects stop being the source of truth.

8. **Pause signups (belt-and-braces)**
   - Vercel UI → anvil-api → Environment Variables → add
     `WAITLIST_PAUSED=true` (Production target).
   - Redeploy anvil-api on the latest production deployment.
   - Smoke:
     ```bash
     curl -s -o /dev/null -w '%{http_code}\n' -X POST \
       -H 'content-type: application/json' \
       -d "{\"email\":\"pause-smoke-$(date +%s)@example.com\"}" \
       https://api.eddacraft.ai/waitlist
     # expect: 503
     ```
   Skippable at current volume; recommended anyway.

9. **Export waitlist rows from both legacy projects**
   ```bash
   cd scripts/dbcon
   WAITLIST_DB_URL="$EDDACRAFT_WEB_URL"     ./export-waitlist.sh waitlist-eddacraft-web.csv
   WAITLIST_DB_URL="$BETA_USER_TOKENS_URL"  ./export-waitlist.sh waitlist-beta-user-tokens.csv
   ```

10. **Import into `anvil-api-prod`** — run the import twice; the staging
    table + `ON CONFLICT (email) DO NOTHING` dedups across sources.
    ```bash
    BETA_DB_URL="$ANVIL_API_PROD_URL" ./import-waitlist.sh waitlist-eddacraft-web.csv
    BETA_DB_URL="$ANVIL_API_PROD_URL" ./import-waitlist.sh waitlist-beta-user-tokens.csv
    ```

11. **Verify**
    ```bash
    BETA_DB_URL="$ANVIL_API_PROD_URL" \
    WAITLIST_DB_URL="$EDDACRAFT_WEB_URL" \
      ./verify-counts.sh
    # expect MISSING = 0 (every source row landed in target).
    # EXTRA is fine if beta-user-tokens contributed additional unique emails.
    ```

12. **Set KeyVault secret to the new URL**
    ```bash
    az keyvault secret set --vault-name kv-iac-anvil \
      --name anvil-api-database-url \
      --value "$ANVIL_API_PROD_URL" >/dev/null
    ```

13. **Merge the rename branch to `dev`** so Pulumi references
    `anvil-api-database-url` (and no longer `website-database-url`).

14. **Pulumi preview + apply**
    ```bash
    cd infra
    pulumi preview   # confirm DATABASE_URL change only; no unexpected diffs
    pulumi up
    ```

15. **Redeploy anvil-api** — Vercel UI → Deployments → Redeploy latest.
    Vercel bakes env vars into deployments, so this is required.

16. **Smoke test against the new DB**
    ```bash
    curl -s https://api.eddacraft.ai/health | jq .
    # any cheap authenticated read that exercises the DB, e.g.:
    curl -s -H "Authorization: Bearer $BETA_TOKEN" \
      https://api.eddacraft.ai/admin/waitlist | jq '. | length'
    ```

17. **Unpause**
    - Vercel UI → remove or blank `WAITLIST_PAUSED` → redeploy anvil-api.
    - Smoke:
      ```bash
      curl -s -o /dev/null -w '%{http_code}\n' -X POST \
        -H 'content-type: application/json' \
        -d "{\"email\":\"unpause-smoke-$(date +%s)@example.com\"}" \
        https://api.eddacraft.ai/waitlist
      # expect 2xx (not 503)
      ```
      Then clean up the smoke row:
      ```bash
      psql "$ANVIL_API_PROD_URL" -c \
        "DELETE FROM waitlist WHERE email LIKE 'unpause-smoke-%@example.com'"
      ```

18. **Delete the legacy KV secret**
    ```bash
    az keyvault secret delete --vault-name kv-iac-anvil \
      --name website-database-url
    ```

### DBCON-004 — decommission legacy Neon projects

> Soak period: ≥ 48h of healthy traffic on `anvil-api-prod`. Legacy
> projects stay up as rollback targets in the meantime (nothing reads
> from them, but they exist).

19. **Final reference grep** — must return zero hits in live code/config:
    ```bash
    rg -n 'website-database-url|eddacraft-web|beta-user-tokens' \
      --glob '!plans/**' --glob '!**/archive/**' --glob '!**/snapshots/**'
    ```

20. **Delete legacy Neon projects**
    ```bash
    neonctl projects delete --project-id <eddacraft-web-id>
    neonctl projects delete --project-id <beta-user-tokens-id>
    # or via the Neon web console if neonctl isn't available
    ```

21. **Confirm**
    ```bash
    neonctl projects list
    # expect: only anvil-api-prod
    ```

22. **Snapshots** — retain `scripts/dbcon/snapshots/` for 30+ days before
    purging.

## Rollback

- **Before step 12** (KV set): abort — no production change yet. Delete
  the new Neon project via `neonctl` if desired.
- **Between steps 12 and 14**: revert the KV secret to the old URL
  (available in the Vercel UI or the snapshot metadata) and stop; no
  `pulumi up` has run.
- **After step 14, before step 20**: set
  `anvil-api-database-url` back to the previously-live URL (the one
  Vercel was using before this module — capture it before step 12 into a
  local variable like `PRE_CUTOVER_URL`), `pulumi up`, redeploy
  anvil-api. Legacy projects are still intact, so no data is lost.
- **After step 20**: disaster. Create a fresh Neon project, restore
  from the appropriate `scripts/dbcon/snapshots/*.sql.gz`, repeat the
  cutover. This is why the 48h soak exists.

## Exit Criteria

- [ ] All 4 DBCON items marked Complete in `plans/index.aps.md`
- [ ] `neonctl projects list` shows only `anvil-api-prod`
- [ ] `az keyvault secret show --name website-database-url` returns
      NotFound; `az keyvault secret show --name anvil-api-database-url`
      returns the `anvil-api-prod` URL
- [ ] `POST /waitlist` returns 2xx and the row lands in
      `anvil-api-prod.waitlist`
- [ ] No live-code references to `website-database-url`,
      `eddacraft-web`, or `beta-user-tokens` remain (grep clean)
- [ ] Snapshots retained for 30+ days before purge
