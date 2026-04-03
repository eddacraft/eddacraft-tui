<!--
APS Module: Docs Auth Gating
==============================
Gate /anvil docs behind GitHub OAuth via Vercel Edge + BAUTH API.

Scopes: DOCSAUTH (main)
-->

# Docs Auth Gating

| ID       | Owner | Status |
| -------- | ----- | ------ |
| DOCSAUTH | —     | Ready  |

## Purpose

Gate the `/anvil` section of `docs.eddacraft.ai` behind GitHub OAuth so only
authenticated beta users can access Anvil documentation. All other sections
(Kindling, APS, edda-stack, blog) remain public.

**Why:** Anvil is a commercial product in closed beta. Public docs expose
product surface before launch. Gating docs behind auth controls access while
keeping open-source docs (APS, Kindling) freely available.

## In Scope

- Vercel Edge Function on `docs-site` intercepting `/anvil/*` requests
- GitHub OAuth flow via new BAUTH API endpoint
- Session cookie (BAUTH JWT) with stateless ES256 verification at the edge
- Login interstitial page ("Sign in with GitHub")
- Pulumi env vars for docs-site and anvil-api
- Azure Key Vault secrets for GitHub OAuth credentials and ES256 public key

## Out of Scope

- Per-page granularity within `/anvil` (all or nothing)
- Refresh token rotation in the browser
- Auto-approval of new GitHub users
- Better-Auth integration (future migration, not this module)
- Changes to CLI auth flows
- Docusaurus config or plugin changes

## Interfaces

**Depends on:**

- BAUTH module (complete) — JWT signing, `beta_users` table, `signLicence()`
- Pulumi IAC (complete) — Vercel project config, Key Vault access
- GitHub OAuth App (manual setup) — registered under EddaCraft org

**Exposes:**

- `/anvil/*` auth gate on `docs.eddacraft.ai`
- `POST /api/v1/auth/github/callback` on `api.eddacraft.ai`
- `/auth/login`, `/auth/callback`, `/auth/logout` on `docs.eddacraft.ai`

## Constraints

- Docusaurus must remain a pure static build — no framework changes
- Edge function must verify JWTs statelessly (no API call per request)
- Cookie must be `HttpOnly; Secure; SameSite=Lax`
- `next` URL in OAuth state must be validated to `/anvil` paths only
- GitHub OAuth App callback URL must match exactly

## Design Spec

`plans/specs/2026-04-03-docs-auth-gating-design.md`

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified (BAUTH, IAC)
- [x] Design spec written and approved
- [x] File locations agreed
- [x] Infrastructure pattern agreed (Key Vault + Pulumi)

---

## Phase 1 — BAUTH GitHub OAuth Endpoint

### DOCSAUTH-001: Add GitHub OAuth callback to BAUTH API

- **Status:** Ready
- **Intent:** Add `POST /api/v1/auth/github/callback` that exchanges a GitHub
  OAuth code for a BAUTH JWT, creating or linking the user in `beta_users`
- **Expected Outcome:** Calling the endpoint with a valid GitHub OAuth code
  returns `{ license, refreshToken, expiresAt }` with the same shape as
  device code and OTP responses. New users are created with `status = pending`.
  Active users receive a valid JWT with `identity.provider = "github"`
- **Validation:** Unit tests covering: valid code exchange, existing user
  linking by email, new user creation with pending status, invalid code
  rejection
- **Files:** `apps/anvil-api/src/routes/auth-github.ts`,
  `apps/anvil-api/src/index.ts`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None (BAUTH already complete)

---

## Phase 2 — Edge Function + Auth Routes

### DOCSAUTH-002: Create Vercel Edge Function for /anvil auth gate

- **Status:** Ready
- **Intent:** Edge function that intercepts `/anvil/*` requests, verifies the
  `anvil-docs-session` cookie contains a valid ES256 JWT, and either passes
  through or redirects to `/auth/login`
- **Expected Outcome:** Unauthenticated requests to `/anvil/*` are redirected
  to `/auth/login?next=/anvil/...`. Authenticated requests pass through to
  the static Docusaurus HTML. All non-`/anvil` requests are unaffected
- **Validation:** Manual test: clear cookies, navigate to `/anvil/overview`,
  verify redirect. Set a valid cookie, verify page loads
- **Files:** `apps/docs-site/middleware/index.ts`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** DOCSAUTH-006 (needs public key env var deployed)

---

### DOCSAUTH-003: Create /auth serverless functions

- **Status:** Ready
- **Intent:** Three Vercel serverless functions that handle the GitHub OAuth
  browser flow: login (redirect to GitHub), callback (exchange code via BAUTH
  API, set cookie), and logout (clear cookie)
- **Expected Outcome:**
  - `GET /auth/login?next=/anvil/overview` redirects to GitHub OAuth with
    encrypted state
  - `GET /auth/callback?code=...&state=...` calls BAUTH API, sets
    `anvil-docs-session` cookie, redirects to the `next` URL
  - `GET /auth/logout` clears the cookie and redirects to `/`
  - Login page shows "Sign in with GitHub" interstitial for users without JS
    redirect
- **Validation:** End-to-end manual test of the full OAuth flow on a preview
  deployment
- **Files:** `apps/docs-site/api/auth/login.ts`,
  `apps/docs-site/api/auth/callback.ts`,
  `apps/docs-site/api/auth/logout.ts`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** DOCSAUTH-001

---

### DOCSAUTH-005: Update vercel.json with rewrites

- **Status:** Ready
- **Intent:** Configure `vercel.json` to route `/anvil/*` through the edge
  function and `/auth/*` to the serverless functions
- **Expected Outcome:** `vercel.json` contains the function config and rewrite
  rules. Docusaurus static build is unaffected
- **Validation:** `vercel build` succeeds; preview deployment routes correctly
- **Files:** `apps/docs-site/vercel.json`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** DOCSAUTH-002, DOCSAUTH-003

---

## Phase 3 — Infrastructure

### DOCSAUTH-004: Store GitHub OAuth secrets in Key Vault

- **Status:** Ready
- **Intent:** Register a GitHub OAuth App under EddaCraft org and store the
  client ID, client secret, and ES256 public key in Azure Key Vault
  (`kv-iac-anvil`)
- **Expected Outcome:** Three new secrets exist in Key Vault:
  `github-oauth-client-id`, `github-oauth-client-secret`,
  `license-public-key`
- **Validation:** `az keyvault secret show --vault-name kv-iac-anvil --name
  github-oauth-client-id` returns a value
- **Files:** None (manual Key Vault operation + GitHub OAuth App registration)
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### DOCSAUTH-006: Add Pulumi env vars for docs-site and anvil-api

- **Status:** Ready
- **Intent:** Add Pulumi resources that read the GitHub OAuth secrets from Key
  Vault and set them as Vercel environment variables on the `anvil-api` and
  `docs-site` projects
- **Expected Outcome:** `pulumi preview` shows three new env vars:
  `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET` on anvil-api,
  `LICENSE_PUBLIC_KEY` on docs-site
- **Validation:** `pulumi preview --expect-no-changes` after apply
- **Files:** `infra/src/vercel.ts`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** DOCSAUTH-004

---

## Phase 4 — Error Handling + UX

### DOCSAUTH-007: Pending approval and error pages

- **Status:** Ready
- **Intent:** Handle edge cases in the auth flow: pending approval (403 from
  BAUTH), GitHub OAuth denied/cancelled, and expired sessions
- **Expected Outcome:**
  - Users with `status = pending` see "Your access is pending approval" page
  - OAuth cancellation redirects to `/` with `?error=denied`
  - Expired JWTs trigger a seamless re-auth via GitHub (instant if GitHub
    session is still active)
- **Validation:** Manual test of each error path on a preview deployment
- **Files:** `apps/docs-site/api/auth/callback.ts` (error handling),
  static error pages or inline HTML responses
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** DOCSAUTH-003

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Vercel Edge + Docusaurus compat | Medium | High | Verify on preview deployment before merging |
| JWT size exceeds cookie limit | Low | Medium | BAUTH JWTs are ~500 bytes, well under 4KB |
| GitHub OAuth rate limits | Low | Low | OAuth is one-time per 7 days; rate limits are generous |
| Public key rotation | Low | Medium | Document rotation process; future JWKS endpoint |
| Better-Auth migration friction | Low | Medium | Design is intentionally a stepping stone; middleware shape survives |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — BAUTH GitHub OAuth | 1 | Ready |
| 2 — Edge Function + Auth Routes | 3 | Ready |
| 3 — Infrastructure | 2 | Ready |
| 4 — Error Handling + UX | 1 | Ready |
| **Total** | **7** | **0/7 done** |
