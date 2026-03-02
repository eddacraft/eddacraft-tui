# Waitlist Email Operations (Admin)

This guide covers how to preview, test, and resend website waitlist confirmation
emails.

## What changed

Website waitlist API now returns delivery state so failures are visible:

- `emailSent` (`true`/`false`)
- `emailStatus` (`sent`, `skipped_existing`, `resend_not_configured`,
  `provider_error`, etc.)
- `isNewSignup` (`true` when first seen in DB)

A new admin endpoint is available to force re-send confirmations:

- `POST /api/waitlist/resend`

## Required environment variables

Set these in the website runtime environment:

- `DATABASE_URL`
- `RESEND_API_KEY`
- `WAITLIST_RESEND_ADMIN_TOKEN` (required for admin resend endpoint)

## Preview email template locally

From `apps/website`:

```bash
pnpm exec react-email dev
```

Open the local preview URL and select the waitlist confirmation template.

## Test normal waitlist flow

```bash
curl -X POST https://<your-site>/api/waitlist \
  -H "Content-Type: application/json" \
  -d '{"email":"you@example.com"}'
```

Expected response fields include `emailSent`, `emailStatus`, and `isNewSignup`.

## Force resend as admin

Use one of these auth headers:

- `Authorization: Bearer <WAITLIST_RESEND_ADMIN_TOKEN>`
- `x-waitlist-admin-token: <WAITLIST_RESEND_ADMIN_TOKEN>`

```bash
curl -X POST https://<your-site>/api/waitlist/resend \
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

## Common failure causes after Resend migration

1. Sending domain not verified in Resend (`updates.eddacraft.ai`)
2. `RESEND_API_KEY` missing or incorrect in deployed environment
3. Resend account restrictions/sandbox recipient limits
4. DNS SPF/DKIM not fully propagated

## Ops notes

- Standard `/api/waitlist` signup sends confirmation only for new signups.
- Existing signups are still accepted but always report
  `emailStatus: skipped_existing` (no new email).
- Use the authenticated `/api/waitlist/resend` endpoint for explicit re-sends
  during support/testing.
