# Post-Deploy Smoke Check Runbook

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
curl -sS https://<api-host>/health
curl -I https://<site-host>/
```

Expected: API healthy; website returns 200.

### 2) Waitlist submission flow

```bash
curl -sS https://<site-host>/api/waitlist \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"email":"smoke-test@example.com"}'
```

Expected: `success: true` with delivery fields (`emailSent`, `emailStatus`).

### 3) Optional admin resend flow

```bash
curl -sS https://<site-host>/api/waitlist/resend \
  -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $WAITLIST_RESEND_ADMIN_TOKEN" \
  -d '{"email":"smoke-test@example.com"}'
```

Expected: `success: true`, `emailSent: true`.

### 4) Verify no immediate error spikes

- Check API logs for 5xx bursts
- Check Neon for error/latency spikes
- Check Resend for provider rejections

## Expected success output

- Health checks pass
- Waitlist API returns success and meaningful email status fields
- No immediate post-deploy error spike

## Failure modes + recovery

1. **Health degraded**
   - Recovery: halt rollout, inspect latest deploy diff, rollback if needed.

2. **Waitlist success but email not sent**
   - Recovery: check `emailStatus`, validate Resend config/domain.

3. **Admin resend unauthorized**
   - Recovery: verify `WAITLIST_RESEND_ADMIN_TOKEN` in runtime env.

## Rollback / safety notes

- Prefer rolling back deploy before ad-hoc prod patching.
- If rollback occurs, rerun smoke checks on rolled-back version.
