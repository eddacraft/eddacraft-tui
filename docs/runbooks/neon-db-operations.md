# Neon DB Operations Runbook

| Type    | Authority     | Owner  | Status | Freshness                                                                                                                                                                                                                                                        |
| ------- | ------------- | ------ | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | Last reviewed 2026-09-05 against `.github/workflows/neon-integration.yml`, `apps/anvil-api/src/__tests__/support/neon-test-database.ts`, and the Neon branch API (CLAWOPEN-011). Prior review 2026-08-31 against APGOV-008 and `apps/anvil-api/src/db/client.ts` |

| Upstream                                                                                                                                                                              | Downstream                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `apps/anvil-api/scripts/migrate.mjs`, `apps/anvil-api/src/db/client.ts` (APGOV-008 retry), `.github/workflows/neon-integration.yml`, Neon branch and project-scoped API-key contracts | on-call operators, test-project operators, post-deploy smoke check, db-migrations runbook |

## Purpose

Triage and recover Neon-related production issues for Anvil services, and
operate the isolated Neon project used for credentialed integration assurance.

## When to use

- API requests return DB errors/timeouts
- `/health` is degraded due to DB checks
- Waitlist/API writes are failing or slow
- Suspected connection exhaustion or query latency spike
- The `Neon OTP Integration` workflow cannot provision, test, or clean up its
  ephemeral branch
- The dedicated test-project API key needs rotation

## Required access / env vars

- Access to deployment logs
- Access to Neon project dashboard
- Access to the private repository's Actions secrets and variables when
  operating the integration project
- `DATABASE_URL` value in runtime environment
- API endpoint URL for health checks

## Dedicated integration-test project

CLAWOPEN-011 uses a separate Neon project named `anvil-api-test`. It must not
contain production data, credentials, logical replication links, or a branch
copied from the production project. Its default branch provides database
`anvil_test` and role `anvil_test_owner`, which each ephemeral branch inherits.
The role must be able to create the minimum auth tables when absent; the harness
inserts only run-specific synthetic rows.

### Provision GitHub Actions access

1. Create or select the dedicated `anvil-api-test` project and confirm the
   `anvil_test` database and `anvil_test_owner` role exist on its default
   branch.
2. Create a
   [project-scoped Neon API key](https://neon.com/docs/manage/orgs-api#organization-api-keys)
   for this project. For an organisation-owned project, prefer the member-level
   project-scoped key so it can manage branches but cannot delete the project.
   Never reuse a production or organisation-wide administrative key.
3. In the private `eddacraft/anvil-001` repository, add the key as the Actions
   secret `NEON_API_KEY`. Do not put its value in repository files, workflow
   logs, issue comments, or local shell history.
4. Add the dedicated project's ID as the Actions variable `NEON_PROJECT_ID`.
   Check the ID against the `anvil-api-test` project before saving it; the
   variable name alone does not prove the target is non-production.
5. Manually dispatch `Neon OTP Integration`. A successful run must create a
   branch named `ci-test-clawopen-011-<run>-<attempt>`, pass the live OTP test,
   and leave no branch with that name after cleanup.

The workflow passes the direct branch URL only as `ANVIL_API_TEST_DATABASE_URL`;
production `DATABASE_URL` is never an input or fallback. Hosted pull-request
runs are limited to internal branches. Fork pull requests remain credential-free
and skip the job. The repo-owned control-plane helper retrieves the project
first and requires its name to equal `anvil-api-test` before it submits a branch
create request. It retrieves only the direct `anvil_test` / `anvil_test_owner`
connection URI, registers the complete URI and its password with GitHub's mask
command, and only then writes the URI to step output. It never prints Neon API
response bodies. Neon's API references for
[project lookup](https://api-docs.neon.tech/reference/getproject),
[branch creation](https://api-docs.neon.tech/reference/createprojectbranch), and
[connection URI retrieval](https://api-docs.neon.tech/reference/getconnectionuri)
are the external control-plane authority.

### Branch lifecycle and cleanup

Each workflow run gives its branch a two-hour expiry and deletes it through the
Neon branch API in an `always()` step. The provisioner emits the exact branch
name and expiry after project preflight, then emits the branch ID immediately
after a successful create response. If creation is ambiguous before an ID is
available, cleanup searches the branch API and deletes only an exact name match.
The expiry is the recovery path when a runner is cancelled or lost before
cleanup executes; it is not a reason to omit or ignore a failed deletion step.

If cleanup fails:

1. Record the GitHub run ID and attempt, then find the exact matching
   `ci-test-clawopen-011-<run>-<attempt>` branch in the `anvil-api-test`
   project.
2. Confirm that the run is finished or abandoned and that the branch is not
   attached to another active run.
3. Delete only that exact branch through the Neon Console or branch API. Never
   delete the default branch, a branch without the expected test prefix, or any
   branch in another project.
4. Confirm it disappears from the branch inventory. If API deletion remains
   unavailable, monitor until the recorded expiry deletes it and escalate if it
   survives beyond that time.

### Rotate the API key

1. Create a replacement project-scoped key for `anvil-api-test`.
2. Replace the `NEON_API_KEY` repository secret without exposing either value.
3. Manually dispatch the workflow and verify test success plus branch cleanup.
4. Revoke the previous key only after the replacement run is green.

### Integration workflow failure guide

| Symptom                                                | Response                                                                                                                                                           |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Branch provisioning reports a missing database or role | Provision `anvil_test` and `anvil_test_owner` on the dedicated project's default branch; do not loosen the harness identity checks.                                |
| Project-name preflight fails                           | Confirm `NEON_PROJECT_ID` identifies the dedicated project whose Neon name is exactly `anvil-api-test`; never bypass or broaden the comparison.                    |
| Neon returns `401` or `403`                            | Confirm the secret is current and project-scoped to the project referenced by `NEON_PROJECT_ID`; rotate it if its status is uncertain.                             |
| The safety gate rejects the URL or identity            | Check for a direct non-pooled TLS URL and the exact test database, role, project, and branch names. Never substitute production `DATABASE_URL` or bypass the gate. |
| A fork pull request has no Neon result                 | Expected: credentials are withheld from forks. Reproduce through an authorised internal branch or manual dispatch after reviewing the change.                      |
| The OTP assertion fails                                | Treat it as a correctness signal. Preserve the run URL and database error summary, verify cleanup, and investigate the production query before rerunning.          |
| The cleanup step fails                                 | Follow the exact-branch cleanup procedure above; branch expiry remains the final fallback.                                                                         |

## Exact commands

### 1) Confirm API health and DB status

```bash
curl -sS https://<api-host>/health
```

Expected: status includes DB reachable/healthy.

### 2) Verify runtime DB configuration exists

```bash
# adjust command for your deployment platform
printenv | rg "^DATABASE_URL="
```

Expected: `DATABASE_URL` is present and uses Neon connection string.

### 3) Exercise a read path and a write path

```bash
curl -sS https://<site-host>/api/waitlist \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"email":"ops-test@example.com"}'
```

Expected: JSON success response, no 5xx.

### 4) Check Neon dashboard signals

- Connection count / saturation
- Query latency (p95/p99)
- Error rate
- Compute/storage limits

### 5) If degraded, reduce pressure

- Pause non-critical background jobs hitting DB
- Temporarily disable high-volume write paths if needed
- Retry once pressure drops

## Expected success output

- `/health` returns healthy DB status
- Write path returns success JSON (no DB error)
- Neon dashboard error rates and latency return to baseline

## Failure modes + recovery

1. **`DATABASE_URL` missing**
   - Recovery: set env var, redeploy, re-check `/health`.

2. **Connection/timeout spikes**
   - Look for
     `NeonDbError: Error connecting to database: TypeError: fetch failed` with
     `ETIMEDOUT` / `internalConnectMultiple`, especially on
     `POST /api/v1/account/activity` (~750ms). Node Happy Eyeballs can
     black-hole Neon IPv6 from Vercel; the anvil-api Neon client prefers IPv4
     DNS results.
   - The shared Neon client retries once on connect-class failures (`ETIMEDOUT`,
     `fetch failed`). Two consecutive failures still surface to the caller. SQL
     and constraint errors are not retried.
   - Account-activity ingest is best-effort (202 + log) on persist failure,
     including after exhausted connect retries; `/health` still reports
     `db: unreachable` when `SELECT 1` fails.
   - Recovery: reduce traffic, inspect long-running queries, verify connection
     limits.

3. **Auth/role permission errors**
   - Recovery: rotate/check DB credentials and Neon role grants.

4. **Persistent high latency**
   - Recovery: investigate hot queries/indexes, scale Neon compute tier if
     required.

## Rollback / safety notes

- Prefer reversible actions first (traffic shaping, pausing non-critical jobs).
- Avoid destructive schema/database changes during incidents.
- Record exact timestamps + actions in incident notes for postmortem.
