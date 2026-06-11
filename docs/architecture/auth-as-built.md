# Auth System — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                                                                                                                     |
| -------- | --------- | ----- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | BAUTH | Live   | Last reviewed 2026-06-10 (targeted delta review: GitHub OAuth flow GHCLIAUTH-003, identity/scopes claims, schema + env additions) against main `45dd1047a`; full review 2026-04-23 against `v0.6.0-beta` and `apps/anvil-api` |

| Upstream                  | Downstream                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------ |
| `apps/anvil-api`, ADR-018 | anvil CLI (token verify, license refresh, device-code, OTP, GitHub OAuth), browser activate flow |

> **Status:** Live (beta) **Last reviewed:** 2026-06-10 (targeted delta review:
> GitHub OAuth flow GHCLIAUTH-003, identity/scopes claims, schema + env
> additions) against main `45dd1047a`; full review 2026-04-23 against
> `v0.6.0-beta` **Service:** `apps/anvil-api` (Hono on Vercel) **Database:**
> Neon Postgres (`beta_users`, `access_tokens`, `audit_log`)

## Overview

The auth system manages beta access to Anvil. It supports four authentication
flows: the original admin-invite token flow, a device code flow for CLI login,
an email OTP flow, and a GitHub OAuth flow (GHCLIAUTH-003). All interactive
flows issue JWT + refresh token pairs minted through the shared `mintSession`
helper (`apps/anvil-api/src/lib/session.ts`).

```text
┌─────────┐         ┌────────────┐         ┌──────────┐
│  Admin   │─invite─▶│  Anvil API │◀─verify─│ Anvil CLI│
│ (curl)   │◀─token──│  (Hono)    │─licence─▶│          │
└─────────┘         └─────┬──────┘         └──────────┘
                          │                      │
                     ┌────▼────┐          ┌──────▼──────┐
                     │  Neon   │          │   Browser   │
                     │ Postgres│          │  (activate) │
                     └─────────┘          └─────────────┘
```

## Token Lifecycle

### 1. Admin creates invite

```
POST /api/v1/admin/invite
Authorization: Bearer <ADMIN_KEY>
```

- Upserts user in `beta_users` (email, name, notes)
- Generates token: `anvil_beta_<base64url(32 random bytes)>` (56 chars total)
- Stores `SHA-256(TOKEN_PEPPER + raw_token)` in `access_tokens`
- Writes `token.created` to `audit_log`
- Returns the raw token exactly once — it is never stored or retrievable

### 2. CLI verifies token

```
POST /api/v1/auth/verify
Body: { "token": "anvil_beta_..." }
```

Validation chain:

1. Format check: starts with `anvil_beta_`, payload is 43 chars of base64url
2. Hash lookup: `SHA-256(pepper + token)` → `access_tokens JOIN beta_users`
3. Revocation check: `revoked_at IS NULL`
4. Expiry check: `expires_at > now()`
5. User status check: `beta_users.status = 'active'`

On success, signs and returns an ES256 JWT licence (see below).

On any failure, returns `{ valid: false }` with no reason — prevents
enumeration.

### 3. Licence refresh

```
POST /api/v1/auth/license/refresh
Body: { "token": "anvil_beta_..." }
```

Same validation as verify. Returns only `{ license }` (fresh JWT, no user
metadata). The refresh endpoint does leak failure reasons (`revoked`, `expired`,
`invalid`) — see gap G-03 below.

### 4. Admin revokes

```
POST /api/v1/admin/revoke
Authorization: Bearer <ADMIN_KEY>
Body: { "email": "..." } or { "token": "..." }
```

- By email: revokes all active tokens for that user
- By token: revokes the specific token
- Both are audit-logged (`tokens.revoked` / `token.revoked`)
- Revocation is soft — sets `revoked_at` timestamp, row is preserved

### 5. Admin lookup

```
GET /api/v1/admin/user/:email
Authorization: Bearer <ADMIN_KEY>
```

Returns user record + all tokens (id, scopes, expires_at, revoked_at,
created_at). Token hashes are not returned.

## Authentication Flows (BAUTH)

### Device Code Flow

1. CLI calls `POST /auth/device/start` — receives `userCode`, `verificationUrl`,
   `pollToken`, `expiresIn`, and `interval`
2. User opens `verificationUrl` in a browser and enters `userCode` to confirm
3. CLI polls `POST /auth/device/poll` with `pollToken` until the user confirms
   or the code expires
4. On confirmation, the poll response returns a JWT + refresh token pair

### Email OTP Flow

1. CLI calls `POST /auth/otp/request` with the user's email
2. API sends a one-time code via Resend
3. User enters the code in the CLI
4. CLI calls `POST /auth/otp/verify` with email + code — receives a JWT +
   refresh token pair

### GitHub OAuth Flow

1. The docs-site callback (`apps/docs-site/api/auth/callback.ts`) validates the
   OAuth state parameter, then calls `POST /auth/github/callback`
   server-to-server (`apps/anvil-api/src/routes/auth-github.ts:154`). CSRF/state
   validation lives entirely in the docs-site layer — the API trusts the caller
   to have validated the state (`auth-github.ts:142-153`)
2. The API exchanges the code with GitHub, fetches the verified primary email
   plus the full verified-email set, then immediately revokes the upstream
   GitHub token
3. `linkOrCreateGitHubUser` resolves the `beta_users` row: match on `github_id`;
   else first-link an active invited row via any verified email (ADR-066); else
   create a new row with `status = 'pending'`
4. Re-binding a verified email already linked to a different `github_id` writes
   a `github_oauth_link_conflict` audit event and returns the same generic 401
   as other auth failures — no account enumeration. Non-active users get 403
5. On success the API mints a JWT + refresh token pair via `mintSession`,
   stamping identity `{ provider: "github", id: <github_id> }`
   (`auth-github.ts:227`)

Audit events: `github_oauth_signup`, `github_oauth_link`,
`github_oauth_blocked`, `github_oauth_login`, `github_oauth_link_conflict`
(`auth-github.ts:182-235`). The route is mounted at
`apps/anvil-api/src/index.ts:140`.

### Admin Approval Flow

1. Admin CLI calls `POST /admin/approve` with the waitlisted user's email
2. API activates the user and sends a beta invite email
3. The user then completes device-code or OTP login through the standard auth
   surfaces

### JWT Session Refresh

`POST /auth/session/refresh` accepts a refresh token and returns a fresh JWT +
refresh token pair. Refresh tokens use family-based rotation — reuse of a
consumed token revokes the entire family to detect theft.

## Backward Compatibility

The original token-based flows remain unchanged:

- `POST /auth/verify` — validates `anvil_beta_*` tokens as before
- `POST /auth/license/refresh` — refreshes licence via beta token as before

New flows issue JWT + refresh token pairs with 7-day access tokens and 90-day
refresh tokens. Old `anvil_beta_*` tokens continue to work alongside the new
flows.

## JWT Licence

Signed with ES256 (ECDSA P-256) using `LICENSE_SIGNING_KEY` (PKCS#8 PEM).

### Claims

| Claim      | Source             | Description                          |
| ---------- | ------------------ | ------------------------------------ |
| `sub`      | `beta_users.id`    | User UUID                            |
| `email`    | `beta_users.email` | User email                           |
| `identity` | Per-flow (caller)  | `email` or `github` — see below      |
| `org`      | Hardcoded          | `null`                               |
| `tier`     | Hardcoded          | `"pro"`                              |
| `scopes`   | Active-token union | e.g. `["beta"]` — see below          |
| `seats`    | Hardcoded          | `1`                                  |
| `rcAfter`  | Computed           | `iat + 7 days` (refresh-check-after) |
| `iat`      | Auto               | Issued-at timestamp                  |
| `exp`      | Computed           | `min(token.expires_at, iat + 90d)`   |

`identity` is resolved per flow (GHCLIAUTH-003): token verify and the OTP /
device flows stamp `{ provider: "email", id: null }`
(`apps/anvil-api/src/routes/auth.ts:76`, `auth.ts:127`); GitHub OAuth stamps
`{ provider: "github", id: <github_id> }` (`auth-github.ts:227`). The shared
`mintSession` helper takes identity from the caller
(`apps/anvil-api/src/lib/session.ts:65`).

`scopes` is resolved via `findActiveScopesForUser(sql, user.id)` — the union of
the user's active `access_tokens.scopes` (`session.ts:60`). Graded scopes (e.g.
`["preview"]`) are preserved through every flow; first-time GitHub sign-ups
default to `["beta"]` (`auth-github.ts:222-224`).

**Header:** `{ alg: "ES256", kid: "2026-03" }`

### Intended client behaviour

The `rcAfter` claim signals when the CLI should re-verify. After `rcAfter`, the
CLI should call `/auth/license/refresh` to get a fresh JWT. This keeps the
offline window short while avoiding verify calls on every invocation.

## Database Schema

```sql
beta_users      (id uuid PK, email citext UNIQUE, name, status, notes, github_id bigint UNIQUE, created_at, updated_at)
access_tokens   (id uuid PK, user_id FK, token_hash UNIQUE, scopes text[], expires_at, revoked_at, created_at)
audit_log       (id uuid PK, action, actor, auth_method, metadata jsonb, created_at)
waitlist        (id serial PK, email citext UNIQUE, source, created_at, updated_at)
device_codes    (id uuid PK, user_id FK, user_code UNIQUE, poll_token UNIQUE, confirmed_at, expires_at, last_polled_at, created_at)
otp_codes       (id uuid PK, user_id FK, code_hash, attempts, expires_at, consumed_at, created_at)
refresh_tokens  (id uuid PK, user_id FK, token_hash UNIQUE, family_id uuid, consumed_at, revoked_at, expires_at, created_at)
admin_keys      (id uuid PK, hashed_key UNIQUE, actor_email, note, created_at, revoked_at)
```

`beta_users.status` is constrained to `active | pending | suspended | banned`
via a CHECK (`apps/anvil-api/src/db/schema.sql:12-13`). `github_id` is sparse
and nullable, linked on first GitHub login; once set it is the authoritative
match key for returning users (`schema.sql:18`).

Extensions: `citext`, `pgcrypto`.

Indexes on: `access_tokens(user_id)`, `access_tokens(token_hash)`,
`audit_log(action)`, `audit_log(created_at)`.

## Environment Variables

| Variable                      | Required | Used by                                 | Description                                                      |
| ----------------------------- | -------- | --------------------------------------- | ---------------------------------------------------------------- |
| `DATABASE_URL`                | Yes      | All routes                              | Neon Postgres connection string                                  |
| `ADMIN_KEY`                   | Yes      | `adminAuth` middleware (`/admin/*`)     | Bearer token for admin endpoints; unset fails closed with `500`  |
| `ADMIN_PER_OPERATOR_KEYS`     | No       | `adminAuth` middleware                  | Enables per-operator admin-key lookup                            |
| `ADMIN_KEY_PEPPER`            | No       | `adminAuth` middleware                  | HMAC secret used to hash per-operator admin keys                 |
| `LICENSE_SIGNING_KEY`         | Yes      | `/auth/verify`, `/auth/license/refresh` | ES256 private key (PKCS#8 PEM)                                   |
| `RESEND_API_KEY`              | Yes      | Waitlist routes                         | Resend email API key                                             |
| `WAITLIST_RESEND_ADMIN_TOKEN` | Yes      | `/waitlist/resend`                      | Token for admin resend endpoint                                  |
| `ANVIL_CORS_ORIGINS`          | Yes      | CORS middleware                         | Comma-separated allowed origins                                  |
| `TOKEN_PEPPER`                | No       | Token hashing                           | Extra secret mixed into SHA-256                                  |
| `RESEND_WAITLIST_AUDIENCE_ID` | No       | Audience management                     | Resend audience ID for waitlist                                  |
| `RESEND_BETA_AUDIENCE_ID`     | No       | Audience management                     | Resend audience ID for beta users                                |
| `ACTIVATE_URL`                | No       | Device code flow                        | Confirmation URL (default: `https://eddacraft.ai/auth/activate`) |
| `GITHUB_CLIENT_ID`            | Yes      | `/auth/github/callback`                 | Docs-site GitHub OAuth app client ID                             |
| `GITHUB_CLIENT_SECRET`        | Yes      | `/auth/github/callback`                 | Docs-site GitHub OAuth app client secret                         |
| `GITHUB_CLI_CLIENT_ID`        | Yes      | CLI device flow (`/auth/github-device`) | Dedicated "Anvil CLI" OAuth app client ID                        |
| `GITHUB_CLI_CLIENT_SECRET`    | Yes      | CLI device flow (`/auth/github-device`) | Dedicated "Anvil CLI" OAuth app client secret                    |

The docs-site OAuth app pair is consumed in `auth-github.ts:44-45` and wired in
`infra/src/vercel.ts:92-93`. The CLI pair backs the dedicated "Anvil CLI"
device-flow OAuth app (kept separate so CLI login and docs auth do not share
rate limits, consent branding, or audit trails); it is consumed in
`apps/anvil-api/src/lib/github-cli-credentials.ts:21-22`, wired in
`infra/src/vercel.ts:96-97`, and validated by the `verifyGitHubCliCredentials`
boot probe (imported at `apps/anvil-api/src/index.ts:16`). Since the device flow
became the CLI's default login (GHCLIAUTH-006), missing CLI credentials degrade
`/health` to 503 — boot still completes, and a fresh environment can be deployed
first and provisioned after (the env vars are read per-request).

`ANVIL_ADMIN_ACTOR` belongs to the separate `anvil-admin` operator CLI, not the
API service itself.

## Cross-Cutting Concerns

### Rate limiting

In-memory sliding window, 60 requests/minute per IP. Resets on cold start
(serverless), so this is best-effort DoS protection only.

Headers returned: `X-RateLimit-Limit`, `X-RateLimit-Remaining`,
`X-RateLimit-Reset`.

### Admin auth

All `/admin/*` routes use `adminAuth` middleware:
`Authorization: Bearer <ADMIN_KEY>` with `timingSafeEqual`. Returns 500 if
`ADMIN_KEY` is not configured (fail-closed).

### Audit trail

Admin actions (`token.created`, `tokens.revoked`, `token.revoked`) are logged to
`audit_log` with actor identity (from `X-Admin-Actor` header, defaults to
`"admin"`). Verify/refresh calls are not audit-logged.

### CORS

Configured via `ANVIL_CORS_ORIGINS`. Supports exact matches and wildcard
patterns (e.g. `https://*.vercel.app`). Currently set to:
`https://eddacraft.ai,https://*.vercel.app,http://localhost:3000`.

---

## Gaps and Hardcoded Values

Items below are known limitations that should be addressed as the system
matures. Roughly ordered by risk.

### G-01: `LICENSE_SIGNING_KEY` not in README or infra — RESOLVED

_Resolved 2026-04-16._ The env var is sourced from KeyVault secret
`license-signing-key` and wired into `anvil-api` via `infra/src/vercel.ts`; the
README env table (`apps/anvil-api/README.md`) lists it. A module-level
cold-start probe in `apps/anvil-api/src/index.ts` parses the PEM at boot and
logs `[boot] licence signing key unavailable: <error>` if the env var is missing
or malformed, and `/health` reports `signingKey: unavailable` with HTTP 503 when
the key can't load — so misconfiguration surfaces at deploy time rather than on
the first `/device/poll` that reaches the licence-minting path.

### G-02: org, tier, seats are hardcoded

```typescript
org: null,
tier: 'pro',
seats: 1,
```

`identity` is resolved per flow now (GHCLIAUTH-003) and `scopes` come from the
user's active tokens, but every licence still claims `pro` tier with no org and
a single seat — hardcoded in the shared claims block
(`apps/anvil-api/src/lib/session.ts:66-69`) and in the token verify/refresh
paths (`apps/anvil-api/src/routes/auth.ts:77-80`). This is fine for beta but
will need to become dynamic before GA — particularly `tier` and `org`.

**Risk:** Low for beta. High if external systems start consuming these claims.
**Fix:** Derive from `beta_users` or a separate `organisations` table when
needed.

### G-03: Refresh endpoint leaks failure reasons

`/auth/verify` returns `{ valid: false }` on all failures (good — no
enumeration). `/auth/license/refresh` returns
`{ valid: false, reason: "revoked" | "expired" | "invalid" }`. This lets an
attacker distinguish between a revoked token and a non-existent one.

**Risk:** Low — requires a valid-format token to trigger. Information value is
minimal. **Fix:** Either remove `reason` from the public response or restrict it
to a debug header.

### G-04: Rate limiter is per-instance

Vercel Functions are stateless — the in-memory `Map` resets on every cold start
and is not shared across instances. Under load, rate limiting is effectively
absent.

**Risk:** Medium — a motivated attacker can bypass rate limits by triggering
fresh instances. **Fix:** Move to a shared store (Vercel KV, Upstash Redis) or
use Vercel's built-in WAF/rate-limiting if available.

### G-05: No key rotation mechanism

`LICENSE_SIGNING_KEY` uses a static `kid: "2026-03"`. There's no published
public key, no JWKS endpoint, and no rotation procedure. Clients would need a
code update to trust a new key.

**Risk:** Low for beta (small user base). High at scale — a compromised key
cannot be rotated without breaking all existing clients. **Fix:** Add a
`/.well-known/jwks.json` endpoint. Support multiple `kid` values. Document
rotation procedure.

### G-06: `rcAfter` not enforced client-side

The JWT includes `rcAfter` (refresh-check-after, 7 days) but there's no evidence
the CLI implements this. If the CLI never refreshes, it operates on a
potentially stale licence for up to 90 days.

**Risk:** Revocations take up to 90 days to take effect from the CLI's
perspective. **Fix:** Implement refresh logic in the CLI. Consider a shorter
licence TTL (e.g. 7–14 days) so stale licences expire naturally.

### G-07: No `verify` audit logging

Successful verifications are debug-logged but not written to `audit_log`. This
means there's no record of when or how often a token is used.

**Risk:** Low for beta. Limits forensic capability if a token is compromised.
**Fix:** Log successful verifications to `audit_log` (with rate-limiting to
avoid log bloat).

### G-08: Admin actor identity is self-reported

The `X-Admin-Actor` header is trusted at face value. Any admin key holder can
claim to be any actor.

**Risk:** Low — the admin key is the trust boundary, and there's currently one
admin. Becomes a problem with multiple admins sharing a key. **Fix:** Per-admin
keys or integrate with an identity provider.

### G-09: Token pepper is optional

If `TOKEN_PEPPER` is not set, tokens are hashed with `SHA-256("")` prefix. This
is still a valid hash, but the pepper adds no value if empty. There's no warning
when it's missing.

**Risk:** Low — the hash is still one-way. The pepper mainly defends against
rainbow tables, which are impractical for 32-byte random tokens. **Fix:** Log a
warning at startup if `TOKEN_PEPPER` is empty.

### G-10: No token usage limits

Tokens have no usage cap — a single token can be verified unlimited times from
unlimited IPs. There's no mechanism to detect credential sharing.

**Risk:** Low for beta. Relevant when seat-based licensing matters. **Fix:**
Track verification count and distinct IPs per token. Alert on anomalies.

## Source Files

| File                                          | Role                            |
| --------------------------------------------- | ------------------------------- |
| `apps/anvil-api/src/index.ts`                 | App entry, routing, middleware  |
| `apps/anvil-api/src/routes/auth.ts`           | Verify + refresh endpoints      |
| `apps/anvil-api/src/routes/auth-device.ts`    | Device code flow endpoints      |
| `apps/anvil-api/src/routes/auth-otp.ts`       | Email OTP flow endpoints        |
| `apps/anvil-api/src/routes/auth-session.ts`   | Session refresh endpoint        |
| `apps/anvil-api/src/routes/auth-github.ts`    | GitHub OAuth callback endpoint  |
| `apps/anvil-api/src/routes/admin.ts`          | Invite, revoke, lookup, approve |
| `apps/anvil-api/src/routes/waitlist.ts`       | Waitlist signup + resend        |
| `apps/anvil-api/src/routes/cron.ts`           | Scheduled cleanup tasks         |
| `apps/anvil-api/src/middleware/admin-auth.ts` | Admin bearer auth               |
| `apps/anvil-api/src/middleware/rate-limit.ts` | In-memory rate limiter          |
| `apps/anvil-api/src/lib/token.ts`             | Token generation + hashing      |
| `apps/anvil-api/src/lib/licence.ts`           | JWT signing                     |
| `apps/anvil-api/src/lib/session.ts`           | Shared session minting helper   |
| `apps/anvil-api/src/lib/email.ts`             | Resend email sender             |
| `apps/anvil-api/src/lib/audience.ts`          | Resend audience management      |
| `apps/anvil-api/src/lib/audit.ts`             | Audit log helper                |
| `apps/anvil-api/src/db/client.ts`             | Neon client singleton           |
| `apps/anvil-api/src/db/queries.ts`            | All SQL queries + Zod schemas   |
| `apps/anvil-api/src/db/schema.sql`            | DDL for all tables              |
| `infra/src/vercel.ts`                         | Deployment config + env vars    |
