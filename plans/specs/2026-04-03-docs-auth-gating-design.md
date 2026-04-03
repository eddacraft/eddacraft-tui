# Docs Auth Gating — Design Spec

> Gate `/anvil` docs behind GitHub OAuth, integrated with the existing BAUTH
> system. All other doc sections (Kindling, APS, edda-stack, blog) remain
> public.

**Date:** 2026-04-03
**Status:** Draft
**Relates to:** BAUTH module (complete), DIST module (in progress)
**Future:** Replace GitHub OAuth with Better-Auth when adopted

---

## 1. Goals

- Anvil docs require authentication; all other docs sections remain public
- Authentication uses GitHub OAuth, integrated as a third BAUTH activation
  mechanism (alongside device code and OTP)
- Unified identity — docs access and CLI access share the same user in
  `beta_users`
- Docusaurus remains a pure static build with zero config changes
- Infrastructure follows existing patterns: Pulumi for env vars, Azure Key
  Vault for secrets

## 2. Non-Goals

- Per-page or per-section granularity within `/anvil` (all or nothing)
- Refresh token rotation in the browser (JWT-only, re-auth on expiry)
- Auto-approval of new GitHub users (manual admin approval remains)
- Better-Auth integration (future work, this spec is the stepping stone)
- Changes to the CLI auth flow

---

## 3. Architecture

```
Request → Vercel Edge Middleware (middleware.ts)
              │
              ├─ /anvil/* → has valid `anvil-docs-session` cookie?
              │     ├─ yes → pass through to Docusaurus static HTML
              │     └─ no  → 302 to /auth/login?next=/anvil/...
              │
              └─ everything else → pass through (public)

/auth/login      → 302 to GitHub OAuth authorize URL
/auth/callback   → exchange code → call BAUTH API → set cookie → 302 to next
/auth/logout     → clear cookie → 302 to /
```

### Components

| Component | Location | Purpose |
| --- | --- | --- |
| Edge Function | `apps/docs-site/middleware/index.ts` | Intercept `/anvil/*`, verify JWT cookie |
| Login function | `apps/docs-site/api/auth/login.ts` | Redirect to GitHub OAuth |
| Callback function | `apps/docs-site/api/auth/callback.ts` | Exchange code, set cookie |
| Logout function | `apps/docs-site/api/auth/logout.ts` | Clear cookie |
| BAUTH GitHub route | `apps/anvil-api/src/routes/auth-github.ts` | New API endpoint |
| Pulumi config | `infra/src/vercel.ts` | Env vars for docs-site and anvil-api |

### Why Edge Middleware

- Content never leaves the edge without auth (real security, not client-side
  hiding)
- Docusaurus stays untouched — no plugins, no React wrappers
- Stateless JWT verification at the edge using the ES256 public key
- Naturally extends to Better-Auth later (swap JWT verification logic)

---

## 4. Auth Flow

### 4.1 Login Initiation

1. User clicks an Anvil docs link (e.g. `/anvil/overview`)
2. Edge middleware checks for `anvil-docs-session` cookie
3. No cookie or invalid/expired JWT → 302 to `/auth/login?next=/anvil/overview`
4. `/auth/login` generates a `state` parameter (encrypted `next` URL + CSRF
   nonce) and redirects to:
   ```
   https://github.com/login/oauth/authorize
     ?client_id={GITHUB_CLIENT_ID}
     &redirect_uri=https://docs.eddacraft.ai/auth/callback
     &scope=read:user,user:email
     &state={encrypted_state}
   ```

### 4.2 Callback + Session Creation

5. GitHub redirects to `/auth/callback?code=...&state=...`
6. `/auth/callback` decrypts `state`, validates CSRF nonce
7. Calls **new BAUTH API endpoint**: `POST /api/v1/auth/github/callback`
   with the GitHub `code`
8. BAUTH API:
   - Exchanges the code with GitHub for an access token
   - Fetches the user's GitHub profile (email, username, avatar)
   - Looks up user in `beta_users` by GitHub ID or email
   - If not found: creates user with `status = pending`
   - If found and `status = active`: issues JWT + refresh token
   - If found and `status != active`: returns 403
   - JWT `identity` field: `{ provider: "github", id: "<github-user-id>" }`
9. `/auth/callback` receives the JWT and sets cookie:
   ```
   Set-Cookie: anvil-docs-session={JWT};
     HttpOnly; Secure; SameSite=Lax;
     Path=/; Max-Age=604800
   ```
10. 302 redirect to the `next` URL from state

### 4.3 JWT Verification (per request)

- Middleware imports the ES256 public key from `LICENSE_PUBLIC_KEY` env var
- Verifies signature and `exp` claim
- No API call per request — fully stateless
- On failure: strips cookie, redirects to `/auth/login`

### 4.4 Logout

- `GET /auth/logout` clears the `anvil-docs-session` cookie
- 302 redirect to `/`
- No server-side session to revoke — JWT simply stops being sent

---

## 5. BAUTH API Addition

### New Endpoint

```
POST /api/v1/auth/github/callback
Content-Type: application/json

{ "code": "<github_oauth_code>" }
```

**Success response (200):**

```json
{
  "license": "<jwt>",
  "refreshToken": "<token>",
  "expiresAt": "2026-04-10T00:00:00.000Z"
}
```

Same shape as device code and OTP responses — the docs callback function
ignores `refreshToken` (no refresh in browser).

**Error responses:**

| Status | Body | When |
| --- | --- | --- |
| 403 | `{ "error": "Account pending approval" }` | User exists but not active |
| 401 | `{ "error": "GitHub authentication failed" }` | Invalid code or GitHub error |

### JWT Claims

Same structure as existing BAUTH JWTs:

```json
{
  "sub": "<user-id>",
  "email": "user@example.com",
  "identity": { "provider": "github", "id": "12345678" },
  "org": null,
  "tier": "pro",
  "scopes": ["beta"],
  "seats": 1,
  "rcAfter": 1712345678,
  "iat": 1712345678,
  "exp": 1712950478
}
```

### Database Changes

None. The existing `beta_users` table is sufficient. GitHub users are
identified by email match or a new lookup. The `identity` field in the JWT
captures the provider — no schema changes needed.

If a GitHub user's email matches an existing `beta_users` row, their accounts
are linked. If no match, a new row is created with `status = pending`.

---

## 6. Infrastructure

### Azure Key Vault Secrets

| Secret | Purpose |
| --- | --- |
| `github-oauth-client-id` | GitHub OAuth App client ID |
| `github-oauth-client-secret` | GitHub OAuth App client secret |
| `license-public-key` | ES256 public key for edge JWT verification |

Stored via:

```bash
az keyvault secret set --vault-name kv-iac-anvil --name github-oauth-client-id --value '<ID>'
az keyvault secret set --vault-name kv-iac-anvil --name github-oauth-client-secret --value '<SECRET>'
az keyvault secret set --vault-name kv-iac-anvil --name license-public-key --value '<PEM>'
```

### Pulumi Changes

In `infra/src/vercel.ts`:

**anvil-api** — two new env vars:

```typescript
GITHUB_CLIENT_ID: getSecret('github-oauth-client-id'),
GITHUB_CLIENT_SECRET: getSecret('github-oauth-client-secret'),
```

**docs-site** — one new env var:

```typescript
LICENSE_PUBLIC_KEY: getSecret('license-public-key'),
```

The public key is the verification half of the ES256 keypair. The API signs
with the private key (`LICENSE_SIGNING_KEY`), the middleware verifies with the
public key. No secret leakage.

### GitHub OAuth App

Created manually under the EddaCraft GitHub organisation:

- **Application name:** EddaCraft Docs
- **Homepage URL:** `https://docs.eddacraft.ai`
- **Authorisation callback URL:** `https://docs.eddacraft.ai/auth/callback`
- **Scopes:** `read:user`, `user:email`

Client ID and secret stored in Key Vault immediately after creation.

---

## 7. Error Handling + UX

### Unauthenticated Experience

- Anvil still appears in the Docusaurus navbar (rendered as normal)
- Clicking any `/anvil/*` link triggers the middleware redirect
- `/auth/login` shows a brief interstitial page: "Sign in with GitHub to
  access Anvil docs" with a single button
- After auth, user lands on the page they originally requested

### Edge Cases

| Scenario | Behaviour |
| --- | --- |
| JWT expired (after 7 days) | Redirect to `/auth/login`; re-auth via GitHub is instant if GitHub session is still active |
| User not in `beta_users` | BAUTH API creates them with `status = pending`; docs shows "Your access is pending approval" page |
| GitHub OAuth denied/cancelled | `/auth/callback` redirects to `/` with query param `?error=denied` |
| Invalid/tampered cookie | Middleware strips cookie, redirects to `/auth/login` |
| Cookie on non-`/anvil` routes | Ignored — middleware only checks `/anvil/*` paths |

### No Refresh Token in Browser

The cookie holds only the JWT. When it expires, the user re-authenticates via
GitHub OAuth (instant if their GitHub session is still active). This avoids
storing refresh tokens in cookies and keeps the middleware stateless.

7-day JWT TTL matches the existing BAUTH session duration.

---

## 8. Vercel Project Configuration

The docs-site is currently a pure Docusaurus static build. Adding middleware
and serverless functions requires Vercel to treat it as a framework project
that supports these features.

### Edge Middleware

Docusaurus is not a Next.js app, so Vercel's auto-detected `middleware.ts`
convention does not apply. Instead, the middleware is configured in
`vercel.json` and implemented as an Edge Function.

The middleware file lives at `apps/docs-site/middleware/index.ts` and is
referenced from `vercel.json`:

```json
{
  "functions": {
    "middleware/index.ts": {
      "runtime": "edge"
    }
  },
  "rewrites": [
    { "source": "/anvil/:path*", "destination": "/middleware" },
    { "source": "/auth/:path*", "destination": "/api/auth/:path*" }
  ]
}
```

The edge function checks the cookie and either returns `NextResponse.next()`
to serve the static Docusaurus page, or returns a redirect to `/auth/login`.

### Serverless Functions

The `/auth/*` handlers are Vercel serverless functions placed in
`apps/docs-site/api/auth/`:

```
api/auth/login.ts     → /auth/login
api/auth/callback.ts  → /auth/callback
api/auth/logout.ts    → /auth/logout
```

These handlers are auto-routed by Vercel's file-based routing. The
`/auth/:path*` rewrite in the `vercel.json` shown above maps the public
URL prefix to these serverless functions.

---

## 9. Security Considerations

| Concern | Mitigation |
| --- | --- |
| CSRF on OAuth flow | `state` parameter with encrypted nonce, validated on callback |
| Cookie theft | `HttpOnly; Secure; SameSite=Lax` — not accessible via JS, not sent cross-site |
| JWT forgery | ES256 signature verification with published public key |
| Open redirect | `next` URL validated to only allow relative paths starting with `/anvil` |
| Token leakage | Public key only on docs-site; private key stays on API |
| Rate limiting | GitHub OAuth has its own rate limits; BAUTH API has existing per-endpoint limits |

---

## 10. Migration Path to Better-Auth

This design is intentionally a stepping stone. When Better-Auth is adopted:

1. Replace the custom `/auth/*` serverless functions with Better-Auth's
   built-in GitHub provider
2. Replace the raw JWT cookie with Better-Auth's session management
3. The edge middleware verification logic stays the same shape — check for a
   valid session before allowing `/anvil/*` access
4. The BAUTH API's `POST /api/v1/auth/github/callback` can be retired once
   Better-Auth handles the full flow

The key invariant that survives the migration: edge middleware gates
`/anvil/*`, everything else is public.

---

## 11. Files Changed

| File | Change |
| --- | --- |
| `apps/docs-site/middleware/index.ts` | New — edge function for `/anvil/*` auth gate |
| `apps/docs-site/api/auth/login.ts` | New — GitHub OAuth redirect |
| `apps/docs-site/api/auth/callback.ts` | New — OAuth callback, set cookie |
| `apps/docs-site/api/auth/logout.ts` | New — clear cookie |
| `apps/docs-site/vercel.json` | Update — add auth rewrites |
| `apps/anvil-api/src/routes/auth-github.ts` | New — GitHub OAuth exchange endpoint |
| `apps/anvil-api/src/index.ts` | Update — mount new route |
| `infra/src/vercel.ts` | Update — new env vars on both projects |
