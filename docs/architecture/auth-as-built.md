# Auth System — As-Built

> **Status:** Live (beta) **Last reviewed:** 2026-03-15 **Service:**
> `apps/anvil-api` (Hono on Vercel) **Database:** Neon Postgres (`beta_users`,
> `access_tokens`, `audit_log`)

## Overview

The auth system manages beta access to Anvil. An admin invites users by email,
generating a one-time token. The CLI verifies that token against the API and
receives a signed JWT licence in return.

```text
┌─────────┐         ┌────────────┐         ┌──────────┐
│  Admin   │─invite─▶│  Anvil API │◀─verify─│ Anvil CLI│
│ (curl)   │◀─token──│  (Hono)    │─licence─▶│          │
└─────────┘         └─────┬──────┘         └──────────┘
                          │
                     ┌────▼────┐
                     │  Neon   │
                     │ Postgres│
                     └─────────┘
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

## JWT Licence

Signed with ES256 (ECDSA P-256) using `LICENSE_SIGNING_KEY` (PKCS#8 PEM).

### Claims

| Claim      | Source                 | Description                          |
| ---------- | ---------------------- | ------------------------------------ |
| `sub`      | `beta_users.id`        | User UUID                            |
| `email`    | `beta_users.email`     | User email                           |
| `identity` | Hardcoded              | `{ provider: "github", id: null }`   |
| `org`      | Hardcoded              | `null`                               |
| `tier`     | Hardcoded              | `"pro"`                              |
| `scopes`   | `access_tokens.scopes` | e.g. `["beta"]`                      |
| `seats`    | Hardcoded              | `1`                                  |
| `rcAfter`  | Computed               | `iat + 7 days` (refresh-check-after) |
| `iat`      | Auto                   | Issued-at timestamp                  |
| `exp`      | Computed               | `min(token.expires_at, iat + 90d)`   |

**Header:** `{ alg: "ES256", kid: "2026-03" }`

### Intended client behaviour

The `rcAfter` claim signals when the CLI should re-verify. After `rcAfter`, the
CLI should call `/auth/license/refresh` to get a fresh JWT. This keeps the
offline window short while avoiding verify calls on every invocation.

## Database Schema

```sql
beta_users    (id uuid PK, email citext UNIQUE, name, status, notes, created_at, updated_at)
access_tokens (id uuid PK, user_id FK, token_hash UNIQUE, scopes text[], expires_at, revoked_at, created_at)
audit_log     (id uuid PK, action, actor, metadata jsonb, created_at)
waitlist      (id serial PK, email citext UNIQUE, source, created_at, updated_at)
```

Extensions: `citext`, `pgcrypto`.

Indexes on: `access_tokens(user_id)`, `access_tokens(token_hash)`,
`audit_log(action)`, `audit_log(created_at)`.

## Environment Variables

| Variable                      | Required | Used by                                 | Description                      |
| ----------------------------- | -------- | --------------------------------------- | -------------------------------- |
| `DATABASE_URL`                | Yes      | All routes                              | Neon Postgres connection string  |
| `ADMIN_KEY`                   | Yes      | Admin middleware                        | Bearer token for admin endpoints |
| `LICENSE_SIGNING_KEY`         | Yes      | `/auth/verify`, `/auth/license/refresh` | ES256 private key (PKCS#8 PEM)   |
| `RESEND_API_KEY`              | Yes      | Waitlist routes                         | Resend email API key             |
| `WAITLIST_RESEND_ADMIN_TOKEN` | Yes      | `/waitlist/resend`                      | Token for admin resend endpoint  |
| `ANVIL_CORS_ORIGINS`          | Yes      | CORS middleware                         | Comma-separated allowed origins  |
| `TOKEN_PEPPER`                | No       | Token hashing                           | Extra secret mixed into SHA-256  |

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

### G-01: `LICENSE_SIGNING_KEY` not in README or infra

The env var is required for verify and refresh to work, but is not documented in
the API README or provisioned in `infra/src/vercel.ts`. If it's missing at
runtime, the route throws an unhandled error (no graceful fallback).

**Risk:** Deploy without it → 500 on every verify call. **Fix:** Add to README
env table, add to infra config, consider a startup check.

### G-02: Identity, org, tier, seats are hardcoded

```typescript
identity: { provider: 'github', id: null },
org: null,
tier: 'pro',
seats: 1,
```

Every licence claims `pro` tier with GitHub identity even though no GitHub
integration exists. This is fine for beta but will need to become dynamic before
GA — particularly `tier` and `org`.

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

| File                                          | Role                           |
| --------------------------------------------- | ------------------------------ |
| `apps/anvil-api/src/index.ts`                 | App entry, routing, middleware |
| `apps/anvil-api/src/routes/auth.ts`           | Verify + refresh endpoints     |
| `apps/anvil-api/src/routes/admin.ts`          | Invite, revoke, lookup         |
| `apps/anvil-api/src/routes/waitlist.ts`       | Waitlist signup + resend       |
| `apps/anvil-api/src/middleware/admin-auth.ts` | Admin bearer auth              |
| `apps/anvil-api/src/middleware/rate-limit.ts` | In-memory rate limiter         |
| `apps/anvil-api/src/lib/token.ts`             | Token generation + hashing     |
| `apps/anvil-api/src/lib/licence.ts`           | JWT signing                    |
| `apps/anvil-api/src/lib/email.ts`             | Resend email sender            |
| `apps/anvil-api/src/lib/audit.ts`             | Audit log helper               |
| `apps/anvil-api/src/db/client.ts`             | Neon client singleton          |
| `apps/anvil-api/src/db/queries.ts`            | All SQL queries + Zod schemas  |
| `apps/anvil-api/src/db/schema.sql`            | DDL for all tables             |
| `infra/src/vercel.ts`                         | Deployment config + env vars   |
