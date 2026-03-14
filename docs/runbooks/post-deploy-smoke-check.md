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
curl -sS https://api.eddacraft.ai/api/v1/health
curl -I https://eddacraft.ai/
```

Expected: API returns `{ "status": "ok" }`; website returns 200.

### 2) Waitlist submission flow

```bash
curl -sS https://api.eddacraft.ai/api/v1/waitlist \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"email":"smoke-test@example.com"}'
```

Expected: `success: true` with delivery fields (`emailSent`, `emailStatus`).

### 3) Optional admin resend flow

```bash
curl -sS https://api.eddacraft.ai/api/v1/waitlist/resend \
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

- Check API logs for 5xx bursts
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

3. **Admin resend unauthorized**
   - Recovery: verify `WAITLIST_RESEND_ADMIN_TOKEN` in runtime env.

4. **Auth verify returns 500**
   - Recovery: check `LICENSE_SIGNING_KEY` is set in API env vars.

## Rollback / safety notes

- Prefer rolling back deploy before ad-hoc prod patching.
- If rollback occurs, rerun smoke checks on rolled-back version.
