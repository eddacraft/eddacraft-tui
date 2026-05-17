# anvil-001

## What this codebase does

Anvil is an agentic engineering governance platform. This monorepo mixes a Rust
CLI/kernel/intercept daemon with TypeScript packages, Hono API services, Next.js
marketing/docs shells, Docusaurus docs apps, Pulumi infrastructure, and Nx/pnpm
tooling. Security-relevant runtime surfaces are mainly `apps/anvil-api`,
`apps/docs-shell`, `apps/website/app/api`, `crates/anvil-cli`, and
`crates/anvil-intercept`.

## Auth shape

- Hono API routes live under `/api/v1`; `adminAuth` must protect the entire
  `/admin` router, and `adminRateLimit` should remain per actor.
- Admin auth accepts per-operator bearer keys via `ADMIN_PER_OPERATOR_KEYS` +
  `ADMIN_KEY_PEPPER` first, then falls back to shared `ADMIN_KEY`; route code
  should use `resolveAdminActor` / `resolveAuthMethod`, never `X-Admin-Actor`.
- Public auth flows use `hashToken`, `signLicence`, `findActiveScopesForUser`,
  `consumeDeviceCode`, `consumeRefreshToken`, and family revocation helpers.
- Docs shell auth uses GitHub OAuth state from `encryptState` / `decryptState`,
  an `oauth-nonce` cookie, `exchangeGithubCode`, and `verifyLicense` before
  proxying `/anvil` private docs.
- Rust driver trust uses `is_driver_allowed`,
  `DriverManifest::validate_workspace_roots`, and method-advertisement checks
  before promotion to enforcement participation.

## Threat model

- Highest impact: minting or preserving licences/scopes without approval,
  bypassing token revocation, or abusing refresh/device-code races.
- Admin endpoints can invite, approve, revoke, migrate, update emails, and view
  audit state; any unauthenticated mount, actor spoofing, or missing audit write
  is high signal.
- Docs shell protects private Anvil docs by cookie/JWT and proxies upstreams;
  open redirects, upstream auth leakage, or forwarding sensitive headers matter.
- Intercept daemon/driver code is a local trust boundary: arbitrary driver
  promotion, workspace-root spoofing, or JSON-RPC method capability bypasses are
  security-relevant even without a remote HTTP attacker.

## Project-specific patterns to flag

- New `apps/anvil-api/src/routes/admin.ts` handlers without
  `admin.use('*', adminAuth)` protection, missing `resolveAdminActor`, or
  trusting caller-supplied actors.
- Licence/token issuance paths that hardcode scopes instead of calling
  `findActiveScopesForUser`, or that issue `license` / `refreshToken` before an
  atomic consume/update succeeds.
- Device, OTP, refresh, and migration preview flows that split check-then-update
  logic instead of using existing atomic helpers such as `pollDeviceCode`,
  `incrementDeviceCodeAttempts`, `consumeDeviceCode`, and `consumeRefreshToken`.
- Docs-shell proxy changes that forward `authorization`, `cookie`, `set-cookie`,
  `x-docs-upstream-secret`, or upstream `/auth/*` redirects across trust
  boundaries.
- Rust intercept changes that treat same-UID IPC as sufficient, skip allowlist
  canonicalisation, accept unmatched `workspace_roots`, or promote drivers that
  do not advertise `ANVIL_ENFORCEMENT_ACK`.

## Known false-positives

- `apps/anvil-api/src/routes/auth.ts` intentionally returns `{ valid: false }`
  with HTTP 200 for `/auth/verify` failures to avoid reason leakage.
- `apps/anvil-api/src/routes/auth-device.ts` intentionally inserts dummy device
  rows for invalid/inactive users so `/device/start` and `/device/confirm` have
  similar timing and response shape.
- `apps/anvil-api/src/middleware/admin-auth.ts` intentionally falls back from
  per-operator DB lookup failure to shared `ADMIN_KEY` during rollout; do not
  flag that alone unless the shared-key path is weakened.
- `apps/docs-shell/proxy.ts` intentionally proxies public docs without a docs
  session; only `/anvil` and `/anvil/*` require `verifyLicense`.
- Test fixtures include sample tokens, GitHub-like tokens, admin keys, and fake
  secrets under `__tests__`, `fixtures`, and docs examples; treat those as test
  data unless they are wired into production config.
