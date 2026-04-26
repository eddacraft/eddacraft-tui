# Waitlist Email Operations (Admin)

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
curl -X POST https://api.eddacraft.ai/api/v1/waitlist \
  -H "Content-Type: application/json" \
  -d '{"email":"you@example.com"}'
```

Expected response fields include `emailSent`, `emailStatus`, and `isNewSignup`.

## Force resend as admin

Use one of these auth headers:

- `Authorization: Bearer <WAITLIST_RESEND_ADMIN_TOKEN>`
- `x-waitlist-admin-token: <WAITLIST_RESEND_ADMIN_TOKEN>`

```bash
curl -X POST https://api.eddacraft.ai/api/v1/waitlist/resend \
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
5. `ANVIL_CORS_ORIGINS` not including every live website origin, especially
   both apex and `www` (CORS rejection)

## Ops notes

- Standard `/api/v1/waitlist` signup sends confirmation only for new signups.
- Existing signups are still accepted but email is skipped (no duplicate sends).
- Use the authenticated `/api/v1/waitlist/resend` endpoint for explicit re-sends
  during support/testing.
