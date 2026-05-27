# Waitlist Email Operations (Admin)

| Type    | Authority     | Owner | Status | Freshness                                        |
| ------- | ------------- | ----- | ------ | ------------------------------------------------ |
| Runbook | Authoritative | API   | Live   | Metadata backfilled 2026-05-27 during DOCGOV-011 |

| Upstream                     | Downstream                      |
| ---------------------------- | ------------------------------- |
| Anvil API waitlist endpoints | Waitlist email admin operations |

This guide covers how to preview, test, and resend waitlist confirmation emails
via the Anvil API (`api.eddacraft.ai`).

## What changed

Waitlist signup and email delivery are consolidated in the Anvil API (Hono). The
website frontend submits directly to `https://api.eddacraft.ai/api/v1/waitlist`.
Website-side API routes have been removed.

Response fields:

- `emailSent` (`true`/`false`)
- `emailStatus` (`sent`, `skipped`, `resend_not_configured`, `provider_error`,
  etc.)
- `isNewSignup` (`true` when first seen in DB)

Admin resend endpoint:

- `POST /api/v1/waitlist/resend`

## Required environment variables

Set these on the **Anvil API** deployment (not the website):

- `DATABASE_URL`
- `RESEND_API_KEY`
- `WAITLIST_RESEND_ADMIN_TOKEN` (required for admin resend endpoint)
- `ANVIL_CORS_ORIGINS` (must include the live website origins:
  `https://eddacraft.ai` and `https://www.eddacraft.ai`)

The website only needs `NEXT_PUBLIC_API_URL` (defaults to
`https://api.eddacraft.ai`).

## Preview email template locally

From `packages/transactional`:

```bash
pnpm exec email dev --dir emails
```

Open the local preview URL and select the waitlist confirmation template.

## Test normal waitlist flow

```bash
curl -sS -w '\nHTTP %{http_code}\n' \
  -X POST https://api.eddacraft.ai/api/v1/waitlist \
  -H "Content-Type: application/json" \
  -d '{"email":"you@example.com"}'
```

Expected response fields include `emailSent`, `emailStatus`, and `isNewSignup`.

## Pause waitlist signups

Use `WAITLIST_PAUSED` only as a short-lived kill-switch when accepting new
waitlist writes are riskier than rejecting them, for example during a write
storm, database maintenance, or a Resend/API incident where signup attempts are
causing secondary failures.

To pause signups:

1. In Vercel, open the **anvil-api** project environment variables.
2. Set `WAITLIST_PAUSED=true` for the affected target, usually Production.
3. Redeploy `anvil-api`; environment variable changes do not affect the running
   deployment until redeploy.
4. Verify `POST /api/v1/waitlist` returns HTTP `503` with
   `Waitlist temporarily paused for maintenance`:

   ```bash
   curl -sS -w '\nHTTP %{http_code}\n' \
     -X POST https://api.eddacraft.ai/api/v1/waitlist \
     -H "Content-Type: application/json" \
     -d '{"email":"you@example.com"}'
   ```

Expected caller behaviour while paused: new waitlist submissions receive `503`
and should be treated as temporarily unavailable. The admin resend endpoint is
not the normal signup path and should be used only for explicit support/testing.

In observability dashboards, filter or annotate intentional `/api/v1/waitlist`
`503` responses separately from incidents. Continue to escalate non-waitlist
`5xx` responses, non-`503` waitlist failures, or waitlist `503` volume that does
not match the expected pause window.

Unset `WAITLIST_PAUSED` as soon as the incident or maintenance window is over,
then redeploy `anvil-api` again. Verify normal signups return `success: true`
with `emailSent`, `emailStatus`, and `isNewSignup` fields before closing the
incident.

## Force resend as admin

Use one of these auth headers:

- `Authorization: Bearer <WAITLIST_RESEND_ADMIN_TOKEN>`
- `x-waitlist-admin-token: <WAITLIST_RESEND_ADMIN_TOKEN>`

```bash
curl -sS -w '\nHTTP %{http_code}\n' \
  -X POST https://api.eddacraft.ai/api/v1/waitlist/resend \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $WAITLIST_RESEND_ADMIN_TOKEN" \
  -d '{"email":"you@example.com"}'
```

### Success response

```json
{
  "success": true,
  "email": "you@example.com",
  "emailSent": true,
  "emailStatus": "sent"
}
```

### Failure response (example)

```json
{
  "success": false,
  "email": "you@example.com",
  "emailSent": false,
  "emailStatus": "provider_error",
  "error": "..."
}
```

## Common failure causes

1. Sending domain not verified in Resend (`updates.eddacraft.ai`)
2. `RESEND_API_KEY` missing or incorrect in API deployment
3. Resend account restrictions/sandbox recipient limits
4. DNS SPF/DKIM not fully propagated
5. `ANVIL_CORS_ORIGINS` not including every live website origin, especially both
   apex and `www` (CORS rejection)

## Ops notes

- Standard `/api/v1/waitlist` signup sends confirmation only for new signups.
- Existing signups are still accepted but email is skipped (no duplicate sends).
- Use the authenticated `/api/v1/waitlist/resend` endpoint for explicit re-sends
  during support/testing.
