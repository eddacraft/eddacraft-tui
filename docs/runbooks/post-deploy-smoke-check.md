# Post-Deploy Smoke Check Runbook

| Type    | Authority     | Owner                         | Status | Freshness                                                                                                  |
| ------- | ------------- | ----------------------------- | ------ | ---------------------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | @aneki (`aneki@eddacraft.ai`) | Live   | Last reviewed 2026-05-24 against post-deploy verification of the Anvil API and `scripts/release/verify.sh` |

| Upstream                                                     | Downstream                                            |
| ------------------------------------------------------------ | ----------------------------------------------------- |
| `scripts/release/verify.sh`, `.github/workflows/release.yml` | release council, on-call operators, rollback runbooks |

## Purpose

Validate critical Anvil user flows immediately after deployment.

## When to use

- Every production deploy
- Any hotfix touching website/API/auth/waitlist

## Required access / env vars

- Deploy URL(s) for website and API
- Optional admin token for waitlist resend endpoint
- Access to logs/dashboard for rapid confirmation

## Exact commands

### 1) Basic health

```bash
curl -sS https://api.eddacraft.ai/api/v1/health
curl -I https://eddacraft.ai/
curl -I https://www.eddacraft.ai/
```

Expected: API returns `{ "status": "ok" }`; website returns 200.

### 1b) Waitlist CORS origins

```bash
curl -sS -X OPTIONS -D - -o /dev/null https://api.eddacraft.ai/api/v1/waitlist \
  -H "Origin: https://eddacraft.ai" \
  -H "Access-Control-Request-Method: POST" \
  -H "Access-Control-Request-Headers: content-type"

curl -sS -X OPTIONS -D - -o /dev/null https://api.eddacraft.ai/api/v1/waitlist \
  -H "Origin: https://www.eddacraft.ai" \
  -H "Access-Control-Request-Method: POST" \
  -H "Access-Control-Request-Headers: content-type"
```

Expected: both responses include `access-control-allow-origin` matching the
request origin.

### 2) Waitlist submission flow

```bash
curl -sS -w '\nHTTP %{http_code}\n' \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"email":"smoke-test@example.com"}' \
  https://api.eddacraft.ai/api/v1/waitlist
```

Expected: `success: true` with delivery fields (`emailSent`, `emailStatus`).

If this returns HTTP `503`, confirm whether the `WAITLIST_PAUSED` kill-switch is
intentionally set on `anvil-api`. See
[`waitlist-email-operations.md`](./waitlist-email-operations.md#pause-waitlist-signups)
for the pause, verification, and unpause steps.

### 3) Optional admin resend flow

```bash
curl -sS -w '\nHTTP %{http_code}\n' \
  https://api.eddacraft.ai/api/v1/waitlist/resend \
  -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $WAITLIST_RESEND_ADMIN_TOKEN" \
  -d '{"email":"smoke-test@example.com"}'
```

Expected: `success: true`, `emailSent: true`.

### 4) Auth verify (requires a valid beta token)

```bash
curl -sS https://api.eddacraft.ai/api/v1/auth/verify \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"token":"anvil_beta_<test-token>"}'
```

Expected: `valid: true` with `user`, `scopes`, `expiresAt`, `license` fields.

### 5) Verify no immediate error spikes

- Check API logs for 5xx bursts. If `WAITLIST_PAUSED=true` is intentional,
  filter or annotate expected `/api/v1/waitlist` `503` responses separately.
- Escalate non-waitlist `5xx` responses, non-`503` waitlist failures, or
  waitlist `503` volume that does not match the expected pause window.
- Check Neon for error/latency spikes
- Check Resend for provider rejections

## Expected success output

- Health checks pass
- Waitlist API returns success and meaningful email status fields
- Auth verify returns a valid licence JWT (if test token available)
- No immediate post-deploy error spike

## Failure modes + recovery

1. **Health degraded**
   - Recovery: halt rollout, inspect latest deploy diff, rollback if needed.

2. **Waitlist success but email not sent**
   - Recovery: check `emailStatus`, validate Resend config/domain.

3. **Waitlist returns 503**
   - Recovery: if `WAITLIST_PAUSED=true` is intentional, leave the smoke check
     in degraded-but-expected state until the incident ends. If not intentional,
     unset it in Vercel, redeploy `anvil-api`, and rerun the waitlist submission
     flow.

4. **Admin resend unauthorized**
   - Recovery: verify `WAITLIST_RESEND_ADMIN_TOKEN` in runtime env.

5. **Auth verify returns 500**
   - Recovery: check `LICENSE_SIGNING_KEY` is set in API env vars.

## Rollback / safety notes

- Prefer rolling back deploy before ad-hoc prod patching.
- If rollback occurs, rerun smoke checks on rolled-back version.
