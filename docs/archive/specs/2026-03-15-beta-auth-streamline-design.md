# Beta Auth Streamline — Design Spec

> **Date:** 2026-03-15 **Status:** Draft **Author:** Josh + Claude **Scope:**
> Anvil API (`apps/anvil-api`), CLI (`apps/anvil-cli`), Website
> (`apps/website`), Transactional emails (`packages/transactional`)

## Problem

The current beta access flow has a manual gap between waitlist signup and token
delivery. A user signs up on the website, then an admin must separately generate
a token via `curl`, manually send it to the user, and the user pastes it into
the CLI. The waitlist and invite systems are disconnected.

## Goal

Create a seamless, automated path from admin approval to CLI activation while
keeping beta access gated behind individual approval. Users should never see or
handle raw tokens.

## Design Principles

- **Thin bridge** — connect existing systems, don't rewrite them
- **Throwaway scaffolding** — this is beta infrastructure; avoid over-building
- **Multiple activation styles** — device code for browser-capable environments,
  email OTP for headless
- **Short-lived sessions** — 7-day JWTs with rotating refresh tokens
- **Audience tracking** — Resend audiences for waitlist and beta user broadcasts

## Out of Scope

- GitHub OAuth (future — separate design)
- Dashboard approval UI (CLI command suffices for beta)
- Automatic waitlist → approval (remains manual)
- Organisation/team model (stays hardcoded `null`)
- Tier differentiation (stays hardcoded `pro`)

---

## 1. Admin Approval Flow

### CLI Command

```
anvil admin approve user@example.com
anvil admin approve --batch 10
```

### API Endpoint

```
POST /api/v1/admin/approve
Authorization: Bearer <ADMIN_KEY>
Body: { "email": "user@example.com" }
  or: { "batch": 10 }
```

### Behaviour

1. Look up email in `waitlist` table (404 if not found — must be on waitlist)
2. Upsert into `beta_users` with `status: 'active'`
3. Generate `anvil_beta_*` token, store hash in `access_tokens`
4. Move contact from `waitlist` audience to `beta-users` audience in Resend
5. Send invite email via Resend (see Section 6)
6. Write `user.approved` to `audit_log`
7. Return `{ email, expiresAt, scopes }` — raw token is NOT returned

For batch mode: process the oldest N unapproved waitlist entries in FIFO order.
"Unapproved" means present in `waitlist` but not in `beta_users`.

The admin never handles the token. The system delivers it to the user via the
invite email's activation mechanisms.

---

## 2. Device Code Flow

Primary CLI activation path. Modelled after GitHub/Azure CLI device flow.

### User Experience

```
$ anvil auth login
  Enter your email: user@example.com

  To authenticate, open this URL:
    https://eddacraft.ai/auth/activate

  And enter code: ANVIL-7F3A

  Waiting for confirmation...
  ✓ Authenticated as user@example.com
```

### Terminology

This flow uses two codes. We intentionally deviate from RFC 8628 naming to avoid
confusion — RFC names are counterintuitive (`device_code` is the one the user
never sees).

| Spec term    | RFC 8628 equivalent | Who uses it           | Format                                  |
| ------------ | ------------------- | --------------------- | --------------------------------------- |
| `user_code`  | `user_code`         | User types in browser | `ANVIL-` + 4 hex chars (10 chars total) |
| `poll_token` | `device_code`       | CLI polls with        | 64-char random opaque                   |

### Sequence

```text
CLI                           API                          Browser
 │                             │                            │
 ├─ POST /auth/device/start ──▶│                            │
 │  { email }                  ├─ Validate beta_users       │
 │                             ├─ Generate user_code        │
 │                             │  (ANVIL-XXXX, shown to     │
 │                             │   user, 10 chars)          │
 │                             ├─ Generate poll_token       │
 │                             │  (random 64-char, CLI-only)│
 │                             ├─ Store in device_codes     │
 │                             │  (expires 15 min)          │
 │◀─ { userCode,              │                            │
 │     verificationUrl,        │                            │
 │     pollToken,              │                            │
 │     expiresIn: 900,         │                            │
 │     interval: 5 }           │                            │
 │                             │                            │
 │  (CLI displays code + URL)  │                    User opens URL,
 │                             │                    enters ANVIL-7F3A
 │                             │◀── POST /auth/device/confirm
 │                             │    { userCode, email }     │
 │                             ├─ Verify email matches      │
 │                             ├─ Mark confirmed            │
 │                             │                            │
 │─ POST /auth/device/poll ──▶│                            │
 │  { pollToken }              ├─ Confirmed → sign JWT      │
 │◀─ { license, refreshToken,  │                            │
 │     expiresAt }             │                            │
 └─ Store pair locally         │                            │
```

### Endpoints

| Method | Path                   | Auth | Description           |
| ------ | ---------------------- | ---- | --------------------- |
| POST   | `/auth/device/start`   | None | Generate device code  |
| POST   | `/auth/device/confirm` | None | Browser confirms code |
| POST   | `/auth/device/poll`    | None | CLI polls for result  |

### Security

- **Email binding on confirm:** The `/device/confirm` endpoint requires both
  `userCode` AND `email`. The API verifies the email matches the one used in
  `/device/start`. This prevents an attacker who calls `/device/start` with a
  victim's email from confirming the code themselves — they would need to know
  which email is bound to the code. The confirmation page prompts for both
  fields. Magic links in invite emails pre-fill both via query params.
- Device codes expire after 15 minutes (CLI-initiated) or 48 hours
  (invite-email-originated)
- Codes are single-use — consumed on confirmation
- Poll returns `pending`, `confirmed`, or `expired` — no information leakage
- Anti-enumeration: identical response shape regardless of whether email exists
  in `beta_users` (but only create a code for valid users)
- 5-second polling interval enforced server-side (429 if faster). Note: like all
  rate limiting in the current stack, this is best-effort on serverless
  (per-instance, resets on cold start — see as-built gap G-04)
- Rate limit on `/device/start` and `/device/confirm` to prevent abuse
- Max 3 confirm attempts per device code — burned after that

### Magic Link Variant

The invite email includes a pre-filled link:
`eddacraft.ai/auth/activate?code=ANVIL-7F3A`. Same confirmation page and
endpoint, just skips the user typing the code.

### Browser Confirmation Page

A single static page at `eddacraft.ai/auth/activate`:

- Input field for device code
- Submits to `POST /api/v1/auth/device/confirm`
- Shows success or error
- Terminal aesthetic consistent with the rest of the site

---

## 3. Email OTP Flow

Headless fallback for SSH, remote servers, CI — no browser needed.

### User Experience

```
$ anvil auth login --otp
  Enter your email: user@example.com

  A verification code has been sent to your email.
  Enter code: 847291

  ✓ Authenticated as user@example.com
```

### Sequence

```text
CLI                           API                          Email
 │                             │                            │
 ├─ POST /auth/otp/request ──▶│                            │
 │  { email }                  ├─ Validate beta_users       │
 │                             ├─ Generate 6-digit code     │
 │                             ├─ Store hash in otp_codes   │
 │                             ├─ Send code via Resend ────▶│
 │◀─ { sent: true,            │                            │
 │     expires_in: 600 }       │                            │
 │                             │                            │
 │  (user checks email)        │                            │
 │                             │                            │
 ├─ POST /auth/otp/verify ───▶│                            │
 │  { email, code }            ├─ Validate code + sign JWT  │
 │◀─ { license, refreshToken,  │                            │
 │     expiresAt }             │                            │
 └─ Store pair locally         │                            │
```

### Endpoints

| Method | Path                | Auth | Description               |
| ------ | ------------------- | ---- | ------------------------- |
| POST   | `/auth/otp/request` | None | Send OTP to email         |
| POST   | `/auth/otp/verify`  | None | Exchange OTP for JWT pair |

### Security

- 6-digit code, cryptographically random
- Stored as SHA-256 hash, not plaintext
- Max 3 verification attempts per code — burned after that
- Max 3 active codes per user (prevents spam)
- 10-minute expiry
- Anti-enumeration: identical response shape regardless of email validity
- Rate limit: 3 requests per email per hour on `/otp/request`

---

## 4. JWT Session Model

### Changes from Current Model

| Aspect            | Current                             | New                                     |
| ----------------- | ----------------------------------- | --------------------------------------- |
| JWT TTL           | min(token expiry, 90 days)          | 7 days                                  |
| `rcAfter` claim   | 7 days (not enforced)               | Removed — JWT expiry is trigger         |
| Refresh mechanism | `/auth/license/refresh` + raw token | `/auth/session/refresh` + refresh token |
| CLI stores        | Raw `anvil_beta_*` token            | JWT + refresh token pair                |
| User sees token   | Yes                                 | Never                                   |

### Token Pair

On successful activation (device code or OTP), the API returns:

```json
{
  "license": "<7-day ES256 JWT>",
  "refreshToken": "<opaque 64-char token>",
  "expiresAt": "2026-03-22T..."
}
```

### Refresh Flow

```text
CLI                              API
 │                                │
 │  (JWT expired or near-expiry)  │
 │                                │
 ├─ POST /auth/session/refresh ─▶│
 │  { refreshToken }              ├─ Lookup refresh token hash
 │                                ├─ Check: not revoked/expired
 │                                ├─ Check: user still active
 │                                ├─ Rotate: new refresh token
 │                                ├─ Invalidate old refresh token
 │                                ├─ Sign new 7-day JWT
 │◀─ { license, refreshToken,    │
 │     expiresAt }                │
 └─ Store new pair locally        │
```

### Refresh Endpoint

| Method | Path                    | Auth | Description                    |
| ------ | ----------------------- | ---- | ------------------------------ |
| POST   | `/auth/session/refresh` | None | Rotate refresh token + new JWT |

### Refresh Token Properties

- Stored hashed (SHA-256) in `refresh_tokens` table
- 90-day expiry
- Single-use with rotation — each refresh issues a new refresh token
- Family-based theft detection: if a consumed token is reused, revoke all tokens
  in that family (indicates token was stolen and both parties are trying to use
  it)

### CLI Behaviour

- On any `anvil` command, check JWT expiry
- If expired or within 1 hour of expiry, auto-refresh in the background
- If refresh fails (revoked, expired, family revoked), prompt `anvil auth login`
  again
- Store credentials in `~/.config/anvil/credentials.json` (XDG-compliant)

### JWT TTL Migration

The current `signLicence` function has a hardcoded `LICENCE_TTL_DAYS = 90`. To
support both old and new flows during transition:

- `signLicence` gains an optional `ttlDays` parameter (defaults to 90 for
  backward compat)
- Old `/auth/verify` and `/auth/license/refresh` call `signLicence()` with no
  argument → 90-day JWT (unchanged behaviour)
- New device code, OTP, and session refresh flows call
  `signLicence(claims, exp, 7)` → 7-day JWT

This avoids breaking existing token holders. Once all beta users have
re-authenticated via the new flows, the old endpoints and 90-day default can be
deprecated together.

### Migration Path

- Existing `/auth/verify` and `/auth/license/refresh` stay in place for backward
  compatibility with any tokens already issued
- New activation flows issue JWT + refresh token pairs
- Once all beta users have re-authenticated, old endpoints can be deprecated

### What Happens to `anvil_beta_*` Tokens

They still exist in `access_tokens` as the server-side credential linking a user
to their access grant. They are never exposed to users. Device code and OTP
handlers confirm active approval by querying
`access_tokens WHERE user_id = <user> AND revoked_at IS NULL AND expires_at > now()`,
then issue the JWT + refresh pair.

---

## 5. Resend Audience Management

### Audiences

| Audience     | When contact is added | When contact is removed        |
| ------------ | --------------------- | ------------------------------ |
| `waitlist`   | Waitlist signup       | Admin approval (moved to beta) |
| `beta-users` | Admin approval        | Revocation or ban              |

### Integration Points

**Waitlist signup** (`POST /api/v1/waitlist`):

```typescript
resend.contacts.create({ email, audienceId: WAITLIST_AUDIENCE_ID });
```

**Admin approve** (`POST /api/v1/admin/approve`):

```typescript
// Note: Resend SDK contacts.remove requires the contact ID, not email.
// Use contacts.list to find the ID first, or store it on creation.
const contact = await resend.contacts.list({
  audienceId: WAITLIST_AUDIENCE_ID,
  email,
});
if (contact)
  await resend.contacts.remove({
    id: contact.id,
    audienceId: WAITLIST_AUDIENCE_ID,
  });
await resend.contacts.create({ email, audienceId: BETA_AUDIENCE_ID });
```

**Admin revoke** (`POST /api/v1/admin/revoke`):

```typescript
const contact = await resend.contacts.list({
  audienceId: BETA_AUDIENCE_ID,
  email,
});
if (contact)
  await resend.contacts.remove({
    id: contact.id,
    audienceId: BETA_AUDIENCE_ID,
  });
```

### Environment Variables

| Variable                      | Description                     |
| ----------------------------- | ------------------------------- |
| `RESEND_WAITLIST_AUDIENCE_ID` | Resend audience ID for waitlist |
| `RESEND_BETA_AUDIENCE_ID`     | Resend audience ID for beta     |

Audience operations are best-effort — failures are logged but do not block the
primary flow. A contact failing to move between audiences should not prevent
approval or revocation.

---

## 6. Email Templates

Two new templates in `packages/transactional/emails/`:

### Invite Email (`beta-invite.tsx`)

Subject: "You're in — Anvil beta access"

Content:

- "You've been approved for the Anvil beta"
- Primary CTA: magic link (`eddacraft.ai/auth/activate?code=ANVIL-XXXX`)
- Secondary: "Or run `anvil auth login` in your terminal"
- Device code shown for reference
- Terminal aesthetic, dark theme

### OTP Email (`otp-code.tsx`)

Subject: "Your Anvil verification code"

Content:

- "Your code is 847291"
- "Expires in 10 minutes"
- Minimal — no CTAs, no links, just the code
- Designed to be read quickly from any email client or notification

---

## 7. Database Changes

### New Tables

```sql
-- Device code flow state
CREATE TABLE device_codes (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  user_code     text UNIQUE NOT NULL,     -- ANVIL-XXXX (shown to user)
  poll_token    text UNIQUE NOT NULL,     -- opaque 64-char (CLI polls with)
  confirmed_at  timestamptz,
  expires_at    timestamptz NOT NULL,
  created_at    timestamptz NOT NULL DEFAULT now()
);

-- Email OTP state
CREATE TABLE otp_codes (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  code_hash     text NOT NULL,
  attempts      int NOT NULL DEFAULT 0,
  expires_at    timestamptz NOT NULL,
  consumed_at   timestamptz,
  created_at    timestamptz NOT NULL DEFAULT now()
);

-- Refresh token chain with family-based theft detection
CREATE TABLE refresh_tokens (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  token_hash    text UNIQUE NOT NULL,
  family_id     uuid NOT NULL,
  expires_at    timestamptz NOT NULL,
  revoked_at    timestamptz,
  consumed_at   timestamptz,
  created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_device_codes_user_code ON device_codes(user_code);
CREATE INDEX idx_device_codes_poll_token ON device_codes(poll_token);
CREATE INDEX idx_device_codes_user_id ON device_codes(user_id);
CREATE INDEX idx_device_codes_expires_at ON device_codes(expires_at);
CREATE INDEX idx_otp_codes_user_id ON otp_codes(user_id);
CREATE INDEX idx_otp_codes_expires_at ON otp_codes(expires_at);
CREATE INDEX idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
CREATE INDEX idx_refresh_tokens_family_id ON refresh_tokens(family_id);
CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
```

### Cleanup

Expired device codes and OTP codes should be periodically purged. On serverless,
this can be a scheduled Vercel Cron or a DB-side `pg_cron` job:

```sql
DELETE FROM device_codes WHERE expires_at < now() - interval '1 hour';
DELETE FROM otp_codes WHERE expires_at < now() - interval '1 hour';
```

---

## 8. Endpoint Summary

### New Endpoints (7)

| Method | Path                           | Auth  | Purpose                              |
| ------ | ------------------------------ | ----- | ------------------------------------ |
| POST   | `/api/v1/admin/approve`        | Admin | Promote waitlist → beta, send invite |
| POST   | `/api/v1/auth/device/start`    | None  | Generate device code                 |
| POST   | `/api/v1/auth/device/confirm`  | None  | Browser confirms code                |
| POST   | `/api/v1/auth/device/poll`     | None  | CLI polls for result                 |
| POST   | `/api/v1/auth/otp/request`     | None  | Send OTP email                       |
| POST   | `/api/v1/auth/otp/verify`      | None  | Exchange OTP for JWT pair            |
| POST   | `/api/v1/auth/session/refresh` | None  | Rotate refresh token + new JWT       |

### Existing Endpoints (unchanged)

| Method | Path                           | Status                               |
| ------ | ------------------------------ | ------------------------------------ |
| POST   | `/api/v1/auth/verify`          | Keep — backward compat               |
| POST   | `/api/v1/auth/license/refresh` | Keep — backward compat               |
| POST   | `/api/v1/admin/invite`         | Keep — still works, not primary      |
| POST   | `/api/v1/admin/revoke`         | Keep — also revokes refresh families |
| GET    | `/api/v1/admin/user/:email`    | Keep — add refresh token info        |

### New CLI Commands (2)

| Command                                    | Description              |
| ------------------------------------------ | ------------------------ |
| `anvil auth login [--otp]`                 | Interactive activation   |
| `anvil admin approve <email \| --batch N>` | Approve waitlist user(s) |

### New Website Page (1)

| Path             | Description                            |
| ---------------- | -------------------------------------- |
| `/auth/activate` | Device code confirmation (static page) |

### New Email Templates (2)

| Template          | Trigger        |
| ----------------- | -------------- |
| `beta-invite.tsx` | Admin approval |
| `otp-code.tsx`    | OTP request    |

### New Environment Variables (2)

| Variable                      | Service | Description                   |
| ----------------------------- | ------- | ----------------------------- |
| `RESEND_WAITLIST_AUDIENCE_ID` | API     | Resend waitlist audience ID   |
| `RESEND_BETA_AUDIENCE_ID`     | API     | Resend beta-users audience ID |

---

## 9. Security Summary

| Concern              | Mitigation                                            |
| -------------------- | ----------------------------------------------------- |
| Token exposure       | Users never see raw tokens; device code + OTP only    |
| Enumeration          | All unauthenticated endpoints return identical shapes |
| Brute force (OTP)    | 3 attempts per code, 3 codes per user, 10-min expiry  |
| Brute force (device) | 15-min expiry, single-use codes                       |
| Token theft          | Refresh token rotation with family-based revocation   |
| Rate limiting        | Per-endpoint limits (best-effort on serverless)       |
| Admin auth           | Timing-safe ADMIN_KEY comparison, audit logging       |
| Credential storage   | SHA-256 hashes for all stored secrets                 |

---

## 10. Future Considerations

- **GitHub OAuth:** Add as a third activation mechanism alongside device code
  and OTP. The JWT session model supports this — GitHub auth would just be
  another way to obtain the initial token pair.
- **Shared rate limiting:** Move from in-memory to Upstash Redis or Vercel KV
  when traffic warrants it.
- **JWKS endpoint:** Publish public keys at `/.well-known/jwks.json` to enable
  offline JWT verification and key rotation.
- **Per-admin keys:** Replace single `ADMIN_KEY` with individual admin
  credentials when the team grows.
