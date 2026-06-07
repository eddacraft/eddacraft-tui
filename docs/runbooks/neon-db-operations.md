# Neon DB Operations Runbook

| Type    | Authority     | Owner                         | Status | Freshness                                                                                     |
| ------- | ------------- | ----------------------------- | ------ | --------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | @aneki (`aneki@eddacraft.ai`) | Live   | Last reviewed 2026-05-24 against production Neon API and `apps/anvil-api/scripts/migrate.mjs` |

| Upstream                             | Downstream                                                        |
| ------------------------------------ | ----------------------------------------------------------------- |
| `apps/anvil-api/scripts/migrate.mjs` | on-call operators, post-deploy smoke check, db-migrations runbook |

## Purpose

Triage and recover Neon-related production issues for Anvil services quickly and
safely.

## When to use

- API requests return DB errors/timeouts
- `/health` is degraded due to DB checks
- Waitlist/API writes are failing or slow
- Suspected connection exhaustion or query latency spike

## Required access / env vars

- Access to deployment logs
- Access to Neon project dashboard
- `DATABASE_URL` value in runtime environment
- API endpoint URL for health checks

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
