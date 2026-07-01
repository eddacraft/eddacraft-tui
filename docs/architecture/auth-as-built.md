# Auth System — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| -------- | --------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | BAUTH | Live   | As-built drift sweep 2026-07-02 against main `d1fded280` (G-08 admin-actor attribution corrected to key-derived, matching api-as-built §5.3; `index.ts` line anchors re-pinned after 145→179 growth). Last reviewed 2026-06-11 against `apps/anvil-api/src/routes/auth-github-device.ts`, `apps/anvil-api/src/routes/auth-device.ts`, `apps/anvil-api/src/db/queries.ts`, `apps/website/app/auth/activate/page.tsx` (GHCLIAUTH-010 device-flow cutover); GHCLIAUTH-003 GitHub OAuth delta against main `45dd1047a`; full review 2026-04-23 against `v0.6.0-beta` and `apps/anvil-api` |

| Upstream                  | Downstream                                                                                          |
| ------------------------- | --------------------------------------------------------------------------------------------------- |
| `apps/anvil-api`, ADR-018 | anvil CLI (token verify, license refresh, GitHub device flow, OTP, docs-site GitHub OAuth callback) |

> **Status:** Live (beta) **Last reviewed:** 2026-07-02 as-built drift sweep
> against main `d1fded280` (G-08 admin-actor attribution + `index.ts`
> re-anchor); 2026-06-11 against the device-flow cutover (GHCLIAUTH-010); GitHub
> OAuth delta (GHCLIAUTH-003) against main `45dd1047a`; full review 2026-04-23
> against `v0.6.0-beta` **Service:** `apps/anvil-api` (Hono on Vercel)
> **Database:** Neon Postgres (`beta_users`, `access_tokens`, `audit_log`)

## Overview

The auth system manages beta access to Anvil. The CLI's default
`anvil auth login` is the brokered GitHub Device Authorisation Grant (RFC 8628,
GHCLIAUTH-005/006): the CLI prints a short user code and the
`github.com/login/device` URL — opened on any device, no email prompt and no
website activation page — then polls the broker until the user authorises.
`anvil auth login --otp` is the retained no-GitHub fallback, emailing a one-time
code to the invited address. Alongside the CLI login the system also exposes the
original admin-invite token flow and the docs-site GitHub OAuth callback
(GHCLIAUTH-003). All interactive flows issue JWT + refresh token pairs minted
through the shared `mintSession` helper (`apps/anvil-api/src/lib/session.ts`).

```text
┌─────────┐         ┌────────────┐         ┌──────────┐
│  Admin   │─invite─▶│  Anvil API │◀─verify─│ Anvil CLI│
│ (curl)   │◀─token──│  (Hono)    │─licence─▶│          │
└─────────┘         └─────┬──────┘         └─────┬────┘
                          │                      │
                     ┌────▼────┐          ┌───────▼────────┐
                     │  Neon   │          │ github.com/    │
                     │ Postgres│          │ login/device   │
                     └─────────┘          │ (any device)   │
                                          └────────────────┘
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

### GitHub Device Flow (default `anvil auth login`)

The CLI's default login is a brokered GitHub Device Authorisation Grant (RFC
8628). The CLI never holds a GitHub client secret — the API brokers the
credentialed upstream calls (ADR-066). Route
`apps/anvil-api/src/routes/auth-github-device.ts`, mounted at
`apps/anvil-api/src/index.ts:174`; CLI client
`crates/anvil-cli/src/auth/device_flow.rs`.

1. CLI calls `POST /api/v1/auth/github-device/start` with an empty JSON object
   (`{}`) — the schema is strict-empty, so no email and no user reference are
   accepted. The API requests a device/user code pair from
   `github.com/login/device/code`, persists a session (hashed `pollToken`,
   encrypted `device_code`, no user binding), and returns `userCode`,
   `verificationUri`, `interval`, `expiresIn`, and an opaque `pollToken`
2. The CLI prints the short user code plus the `github.com/login/device` URL —
   opened **on any device**; there is no email prompt and no website activation
   page
3. CLI polls `POST /api/v1/auth/github-device/poll` with `pollToken`, honouring
   `interval` and the upstream `slow_down` back-off
4. The poll exchanges the stored `device_code` with GitHub, derives the user
   **solely** from the resulting token (`fetchGitHubUser` → `github_id` linking,
   GHCLIAUTH-003), revokes the GitHub token immediately, runs the active-status
   gate, and mints the Anvil licence exactly once (re-returnable within TTL)
5. Terminal poll states: `confirmed` (returns a JWT + refresh token pair),
   `expired`, `declined`, and `awaiting_approval` — the last for a non-active
   Anvil user, whose CLI message points at `anvil auth login --otp`

Account linking (`linkOrCreateGitHubUser`, `queries.ts`): returning users match
on `github_id` (authoritative once set); a first link matches **any** verified
GitHub email against an active invited row and fails closed on a verified email
already bound to a different `github_id`; otherwise a `pending` row is created
and the active-status gate rejects it. The invitation stays email-keyed — GitHub
is a linked auth method (ADR-066 decision 7).

Operator topology, health signals, and incident triage for this flow live in the
[GitHub device-flow login runbook](../runbooks/github-device-flow.md).

### Email OTP Flow

1. CLI calls `POST /auth/otp/request` with the user's email
2. API sends a one-time code via Resend
3. User enters the code in the CLI
4. CLI calls `POST /auth/otp/verify` with email + code — receives a JWT +
   refresh token pair

### GitHub OAuth Flow

1. The docs-site callback (`apps/docs-site/api/auth/callback.ts`) validates the
   OAuth state parameter, then calls `POST /api/v1/auth/github/callback`
   server-to-server (`apps/anvil-api/src/routes/auth-github.ts:99`). CSRF/state
   validation lives entirely in the docs-site layer — the API trusts the caller
   to have validated the state (`auth-github.ts:92-98`)
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
   (`auth-github.ts:172`)

Audit events: `github_oauth_signup`, `github_oauth_link`,
`github_oauth_blocked`, `github_oauth_login`, `github_oauth_link_conflict`
(`auth-github.ts:127-175`). The same audit events are written by the CLI device
flow with `method: "device_flow"`. The route is mounted at
`apps/anvil-api/src/index.ts:173`.

### Legacy Device Code Flow (shipped-CLI compatibility)

`POST /api/v1/auth/device/start` + `/poll`
(`apps/anvil-api/src/routes/auth-device.ts`) predate the GitHub device flow and
remain live only for already-shipped CLIs: `/start` takes an email and returns a
`verificationUrl` (the `ACTIVATE_URL`-configured page), `/poll` returns the
session once the code is confirmed. The matching
`POST /api/v1/auth/device/confirm` browser-confirmation endpoint was **removed**
in GHCLIAUTH-008 (along with its `requireAuth`), so the legacy session can no
longer be confirmed via the website — outstanding sessions stay pending until
they expire. Retiring `/start` + `/poll` and the `device_codes` table is a
future pass; new logins use the GitHub device flow or `--otp`.

### Admin Approval Flow

1. Admin CLI calls `POST /api/v1/admin/approve` with the waitlisted user's email
2. API activates the user and sends a beta invite email; the invite points the
   user at `anvil auth login` (no per-invite device code is generated —
   GHCLIAUTH-007)
3. The user then completes login on their first `anvil auth login` — the GitHub
   device flow, or `--otp` for the no-GitHub fallback

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
legacy device flows stamp `{ provider: "email", id: null }`
(`apps/anvil-api/src/routes/auth.ts:76`, `auth.ts:127`); both GitHub paths — the
docs-site OAuth callback and the CLI GitHub device flow — stamp
`{ provider: "github", id: <github_id> }` (`auth-github.ts:172`,
`auth-github-device.ts`). The shared `mintSession` helper takes identity from
the caller (`apps/anvil-api/src/lib/session.ts:65`).

`scopes` is resolved via `findActiveScopesForUser(sql, user.id)` — the union of
the user's active `access_tokens.scopes` (`session.ts:60`). Graded scopes (e.g.
`["preview"]`) are preserved through every flow; first-time GitHub sign-ups
default to `["beta"]` (`auth-github.ts:167-170`).

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
github_device_sessions (id uuid PK, poll_token_hash UNIQUE, github_device_code_enc, interval_s, expires_at, last_polled_at, minted_at, minted_session_enc, created_at)
otp_codes       (id uuid PK, user_id FK, code_hash, attempts, expires_at, consumed_at, created_at)
refresh_tokens  (id uuid PK, user_id FK, token_hash UNIQUE, family_id uuid, consumed_at, revoked_at, expires_at, created_at)
admin_keys      (id uuid PK, hashed_key UNIQUE, actor_email, note, created_at, revoked_at)
```

`beta_users.status` is constrained to `active | pending | suspended | banned`
via a CHECK (`apps/anvil-api/src/db/schema.sql:12-13`). `github_id` is sparse
and nullable, linked on first GitHub login; once set it is the authoritative
match key for returning users (`schema.sql:18`).

`github_device_sessions` is the DB-backed state for the brokered GitHub Device
Authorisation Grant (GHCLIAUTH-004, ADR-066;
`apps/anvil-api/src/db/migrations/016-github-device-sessions.sql`). It is a
dedicated table — not `device_codes` — because the GitHub flow structurally
breaks that table's `user_code UNIQUE NOT NULL` + start-time `user_id`
invariants. At-rest model: `poll_token_hash` stores only a hash of the
client-held poll token (`lib/token.ts` `hashToken`); `github_device_code_enc`
holds the GitHub `device_code` **encrypted, not hashed**, because the poll
broker must recover the plaintext for the token exchange (RFC 8628 §3.4) — the
key is derived from the client-held poll token (`lib/github-device-crypto.ts`),
so a DB dump alone recovers neither. There is **no user column by design**: the
bound user is derived solely from the GitHub token at poll-confirmation time.
`minted_at` / `minted_session_enc` back the GHCLIAUTH-005 "mint exactly once,
re-returnable within TTL" semantics.

Extensions: `citext`, `pgcrypto`.

Indexes on: `access_tokens(user_id)`, `access_tokens(token_hash)`,
`audit_log(action)`, `audit_log(created_at)`,
`github_device_sessions(expires_at)`.

## Environment Variables

| Variable                      | Required | Used by                                 | Description                                                                                                        |
| ----------------------------- | -------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `DATABASE_URL`                | Yes      | All routes                              | Neon Postgres connection string                                                                                    |
| `ADMIN_KEY`                   | Yes      | `adminAuth` middleware (`/admin/*`)     | Bearer token for admin endpoints; unset fails closed with `500`                                                    |
| `ADMIN_PER_OPERATOR_KEYS`     | No       | `adminAuth` middleware                  | Enables per-operator admin-key lookup                                                                              |
| `ADMIN_KEY_PEPPER`            | No       | `adminAuth` middleware                  | HMAC secret used to hash per-operator admin keys                                                                   |
| `LICENSE_SIGNING_KEY`         | Yes      | `/auth/verify`, `/auth/license/refresh` | ES256 private key (PKCS#8 PEM)                                                                                     |
| `RESEND_API_KEY`              | Yes      | Waitlist routes                         | Resend email API key                                                                                               |
| `WAITLIST_RESEND_ADMIN_TOKEN` | Yes      | `/waitlist/resend`                      | Token for admin resend endpoint                                                                                    |
| `ANVIL_CORS_ORIGINS`          | Yes      | CORS middleware                         | Comma-separated allowed origins                                                                                    |
| `TOKEN_PEPPER`                | No       | Token hashing                           | Extra secret mixed into SHA-256                                                                                    |
| `RESEND_WAITLIST_AUDIENCE_ID` | No       | Audience management                     | Resend audience ID for waitlist                                                                                    |
| `RESEND_BETA_AUDIENCE_ID`     | No       | Audience management                     | Resend audience ID for beta users                                                                                  |
| `ACTIVATE_URL`                | No       | Legacy device code flow                 | Verification URL returned by `/auth/device/start` (default: `https://eddacraft.ai/auth/activate`, now a tombstone) |
| `GITHUB_CLIENT_ID`            | Yes      | `/auth/github/callback`                 | Docs-site GitHub OAuth app client ID                                                                               |
| `GITHUB_CLIENT_SECRET`        | Yes      | `/auth/github/callback`                 | Docs-site GitHub OAuth app client secret                                                                           |
| `GITHUB_CLI_CLIENT_ID`        | Yes      | CLI device flow (`/auth/github-device`) | Dedicated "Anvil CLI" OAuth app client ID                                                                          |
| `GITHUB_CLI_CLIENT_SECRET`    | Yes      | CLI device flow (`/auth/github-device`) | Dedicated "Anvil CLI" OAuth app client secret                                                                      |

The docs-site OAuth app pair is consumed in `auth-github.ts:28-29` and wired in
`infra/src/vercel.ts:92-93`. The CLI pair backs the dedicated "Anvil CLI"
device-flow OAuth app (kept separate so CLI login and docs auth do not share
rate limits, consent branding, or audit trails); it is consumed in
`apps/anvil-api/src/lib/github-cli-credentials.ts:21-22`, wired in
`infra/src/vercel.ts:96-97`, and validated by the `verifyGitHubCliCredentials`
boot probe (imported at `apps/anvil-api/src/index.ts:17`). Since the device flow
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
`audit_log` with key-derived actor identity — the per-operator key's
`actor_email` or the `shared-key@anvil` sentinel for shared-`ADMIN_KEY` callers
(`apps/anvil-api/src/middleware/admin-auth.ts:166`, `admin-auth.ts:177`). The
client-supplied `X-Admin-Actor` header is ignored (`admin-auth.ts:88-108`), so
attribution cannot be forged. Verify/refresh calls are not audit-logged.

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

### G-08: Shared-key admin actions share one attribution identity

Admin attribution is derived from the authenticating key, not from client input.
The `X-Admin-Actor` header is intentionally ignored on both paths
(`apps/anvil-api/src/middleware/admin-auth.ts:88-108`), closing the
attribution-forgery vector. Per-operator keys attribute to the key's
`actor_email` (`admin-auth.ts:166`); shared-`ADMIN_KEY` callers all collapse
into the sentinel `shared-key@anvil` (`SHARED_KEY_ACTOR`, `admin-auth.ts:13`,
`admin-auth.ts:177`). The residual limitation is that when several admins share
the single `ADMIN_KEY`, their actions are indistinguishable in `audit_log`. This
matches the sister doc's description in [`api-as-built.md`](api-as-built.md)
§5.3.

**Risk:** Low — the admin key is still the trust boundary, and a forged
`X-Admin-Actor` no longer changes attribution. Individual accountability is
absent only while operators share the one key. **Fix:** Provision per-operator
keys (`ADMIN_PER_OPERATOR_KEYS` + `admin_keys`) so each admin authenticates
under their own `actor_email`.

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

| File                                              | Role                                          |
| ------------------------------------------------- | --------------------------------------------- |
| `apps/anvil-api/src/index.ts`                     | App entry, routing, middleware                |
| `apps/anvil-api/src/routes/auth.ts`               | Verify + refresh endpoints                    |
| `apps/anvil-api/src/routes/auth-github-device.ts` | CLI GitHub device-flow broker (default login) |
| `apps/anvil-api/src/routes/auth-device.ts`        | Legacy device code flow endpoints             |
| `apps/anvil-api/src/routes/auth-otp.ts`           | Email OTP flow endpoints                      |
| `apps/anvil-api/src/routes/auth-session.ts`       | Session refresh endpoint                      |
| `apps/anvil-api/src/routes/auth-github.ts`        | Docs-site GitHub OAuth callback endpoint      |
| `apps/anvil-api/src/routes/admin.ts`              | Invite, revoke, lookup, approve               |
| `apps/anvil-api/src/routes/waitlist.ts`           | Waitlist signup + resend                      |
| `apps/anvil-api/src/routes/cron.ts`               | Scheduled cleanup tasks                       |
| `apps/anvil-api/src/middleware/admin-auth.ts`     | Admin bearer auth                             |
| `apps/anvil-api/src/middleware/rate-limit.ts`     | In-memory rate limiter                        |
| `apps/anvil-api/src/lib/token.ts`                 | Token generation + hashing                    |
| `apps/anvil-api/src/lib/licence.ts`               | JWT signing                                   |
| `apps/anvil-api/src/lib/session.ts`               | Shared session minting helper                 |
| `apps/anvil-api/src/lib/email.ts`                 | Resend email sender                           |
| `apps/anvil-api/src/lib/audience.ts`              | Resend audience management                    |
| `apps/anvil-api/src/lib/audit.ts`                 | Audit log helper                              |
| `apps/anvil-api/src/db/client.ts`                 | Neon client singleton                         |
| `apps/anvil-api/src/db/queries.ts`                | All SQL queries + Zod schemas                 |
| `apps/anvil-api/src/db/schema.sql`                | DDL for all tables                            |
| `infra/src/vercel.ts`                             | Deployment config + env vars                  |
