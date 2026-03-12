# Product Licensing Design

## Summary

Evolve Anvil's current beta token system to support offline-capable product
licensing. The API issues a signed licence blob (JWT) during login. The CLI
validates the blob locally using a baked-in public key — no network call needed
per run. A background refresh mechanism handles revocation and renewal.

## Goals

- Offline-first licence validation — no network required per CLI invocation
- Rich claims (tier, org, identity, seats) ready for future feature flagging
- Social identity anchor (GitHub first) — works across physical machines,
  codespaces, containers, and headless environments
- Minimal disruption to the existing token-based login flow
- Testable on current infrastructure with manual token issuance

## Non-Goals

- OAuth / device code flow (future — when dashboard lands)
- Feature gating per tier (future — when feature flags land)
- Seat management / enforcement (future — team/enterprise tiers)
- Dashboard integration

---

## Architecture

### Two-File Model

The raw beta token and the licence blob serve different purposes and are stored
separately:

| File                 | Purpose                                     | Used by     |
| -------------------- | ------------------------------------------- | ----------- |
| `~/.anvil/auth.json` | Raw token for authenticated API calls       | API client  |
| `~/.anvil/license`   | Signed licence blob for offline entitlement | CLI startup |

This separation means:

- The token can be rotated without re-issuing the licence
- The licence can be verified without network access
- When OAuth replaces the token exchange, the licence side stays the same

### Licence Blob Format

A JWT signed with ES256 (ECDSA P-256).

**Header:**

```json
{
  "alg": "ES256",
  "kid": "2026-03"
}
```

**Claims:**

```json
{
  "sub": "user_abc123",
  "email": "aneki@eddacraft.ai",
  "identity": { "provider": "github", "id": "joshuaboys" },
  "org": "eddacraft",
  "tier": "pro",
  "scopes": ["beta"],
  "seats": 1,
  "iat": 1741737600,
  "exp": 1749513600,
  "rcAfter": 1742342400
}
```

| Claim      | Type           | Purpose                                                                 |
| ---------- | -------------- | ----------------------------------------------------------------------- |
| `sub`      | string         | Internal user ID from DB                                                |
| `email`    | string         | User email                                                              |
| `identity` | object         | Social identity anchor — `{provider, id}`                               |
| `org`      | string \| null | Organisation slug (null until org model exists)                         |
| `tier`     | string         | `free` / `pro` / `team` / `enterprise` (hardcoded to `"pro"` initially) |
| `scopes`   | string[]       | Feature scopes (currently `["beta"]`)                                   |
| `seats`    | number         | Seat count (future use, default `1`)                                    |
| `iat`      | number         | Issued at (Unix timestamp)                                              |
| `exp`      | number         | Expires at — 90-day TTL                                                 |
| `rcAfter`  | number         | Revocation check after — 7-day window from issuance                     |

**Signing:**

- API holds an ES256 private key (`LICENSE_SIGNING_KEY` env var, PEM format)
- CLI binary contains the matching public key
- `kid` header identifies the key version for rotation
- Key rotation: embed both current and previous public keys in the CLI, keyed by
  `kid`. API signs with the current key. Old licences still verify against the
  previous key until they naturally expire. **Deployment dependency:** a new CLI
  build embedding the next public key must be shipped before the API starts
  signing with the next private key. Rotation is manual and on-demand — not
  calendar-based despite the `kid` format.

### File Layout

**User-level (default):**

```
~/.anvil/
  auth.json      # existing — raw token for API calls
  license        # new — signed JWT blob (plain text string)
```

**Project-level override:**

```
<project>/.anvil/
  license        # optional — overrides user-level for this project
```

**Resolution order:**

1. `ANVIL_LICENSE` env var (for CI / containers)
2. `.anvil/license` in project root
3. `~/.anvil/license` in home dir

**File permissions:** `0o600` (read/write owner only).

The licence file contains just the raw JWT string — no JSON wrapper. Simple to
inspect, pipe, or set as an env var.

`.anvil/license` must be added to the default `.gitignore` template. The
concrete location to patch is `updateGitignore()` in
`apps/anvil-cli/src/services/template-generator.ts`.

**Project-level override use case:** The project-level file is primarily for CI
environments and service accounts. In team settings, each contributor uses their
own `~/.anvil/license` — the project-level file should not be used for
individual developer licences. The JWT payload contains PII (email, GitHub
identity, org) so accidental commit is a data leak risk. The `.gitignore` entry
is necessary but teams should also consider `.anvil/license` in their global
gitignore.

---

## Flows

### Login

```
1. User runs `anvil login --token anvil_beta_...` (or interactive prompt)
2. CLI calls POST /api/v1/auth/verify with the token
3. API validates token, builds + signs licence blob, returns:
   {
     valid: true,
     user: { email },
     scopes: [],
     expiresAt: "...",
     license: "eyJhbG..."
   }
4. CLI saves auth.json (token, user, scopes, expiry — same as today)
5. CLI saves ~/.anvil/license (JWT string)
6. CLI prints:
   "Logged in as aneki@eddacraft.ai
    Licence valid until 2026-06-10"
```

**Implementation note:** The CLI's `VerifyResponseSchema` in `auth-client.ts`
must be updated to include `license: z.string()` (required, not optional — the
API always returns it after this change). The `login.ts` command action must
capture and persist the licence field to `~/.anvil/license`.

### Per-Command Verification

On every command that requires auth (all except `login`, `logout`, `whoami`,
`tutorial`, `help`):

```
1. Resolve licence file (env var → project-level → user-level)
2. No file found → "Your session needs to be refreshed. Run `anvil login` to continue."
3. Decode JWT header, find matching public key (current or previous by kid)
4. Verify signature → invalid: "Your licence could not be verified. Run `anvil login` or contact support@eddacraft.ai if this is unexpected."
5. Check exp → expired: "Your licence needs to be renewed. Run `anvil login` to continue."
6. Check rcAfter → if past AND no refresh attempted in the last 60s, schedule background refresh (non-blocking)
7. Proceed with command
```

**Key principle:** verification never makes a network call. The signature check
and expiry check are purely local. Network is only used for background refresh.

**Expiry alignment:** The token `expiresAt` in `auth.json` and the licence `exp`
are set to the same value (90 days from issuance). If `auth.json` is absent or
expired but the licence is still valid, the background refresh cannot fire (no
token to send). In this case the CLI continues to work offline until `exp`
passes, then prompts re-login. This is acceptable — the user gets the full
licence window regardless.

**Refresh deduplication:** The CLI persists a `lastRefreshAttempt` timestamp in
a sidecar file `~/.anvil/refresh-state` (not in `auth.json`, because
`loadAuth()` returns null for expired tokens and the timestamp would be lost).
If a refresh was attempted within the last 60 seconds, skip the attempt even if
`rcAfter` has passed. This prevents burst-running commands from flooding the
refresh endpoint.

### Background Refresh

When `rcAfter` (revocation check after) has passed:

1. Fire a non-blocking HTTP call to `POST /api/v1/auth/license/refresh` using
   the raw token from `auth.json`
2. **Success:** API returns a fresh licence blob → overwrite `~/.anvil/license`
   silently. Updated `exp` and `rcAfter`.
3. **Revoked/expired:** API returns `{valid: false}` → delete the licence file.
   After the current command finishes, print: "Your licence could not be
   verified. Run `anvil login` or contact support@eddacraft.ai if this is
   unexpected."
4. **Network failure:** Do nothing. Try again next run.

The background refresh never blocks the current command. Worst case: one
additional command executes after revocation before the user is locked out.

### Logout

```
1. Delete auth.json via clearAuth()
2. Delete ~/.anvil/license via clearLicence()
3. Print: "Logged out. Local credentials removed."
```

**Implementation note:** `clearAuth()` in `auth-store.ts` must be extended to
also delete the licence file, or a separate `clearLicence()` function must be
created and called from `logout.ts` alongside `clearAuth()`. Leaving a stale
licence file after logout is both a functional bug and a PII leak.

### Backwards Compatibility

If a user has `auth.json` but no `license` file (logged in before this change):

> "Your session needs to be refreshed. Run `anvil login` to continue."

No silent migration — a fresh login is the cleanest path.

### `anvil whoami` (enhanced)

The existing `whoami` command is extended to show licence information. No new
`auth` subcommand group is introduced — `login`, `logout`, and `whoami` remain
top-level commands.

```
Authenticated: yes
Email:         aneki@eddacraft.ai
Identity:      github:joshuaboys
Tier:          pro
Org:           eddacraft
Expires:       2026-06-10
Next check:    2026-03-19 (in 5 days)
Licence:       ~/.anvil/license
```

---

## API Changes

### Extended `POST /api/v1/auth/verify`

The existing verify endpoint gains licence issuance. No new endpoint needed for
login.

**Response (updated):**

```json
{
  "valid": true,
  "user": { "email": "aneki@eddacraft.ai" },
  "scopes": ["beta"],
  "expiresAt": "2026-06-10T00:00:00Z",
  "license": "eyJhbGciOiJFUzI1NiIs..."
}
```

Internally after existing validation:

1. Look up user, org, tier, scopes from DB
2. Build JWT claims
3. Sign with ES256 private key
4. Include in response

### New `POST /api/v1/auth/license/refresh`

Called by the CLI background refresh.

**Request:**

```json
{ "token": "anvil_beta_..." }
```

**Response (success):**

```json
{ "license": "eyJhbGciOiJFUzI1NiIs..." }
```

**Response (revoked/expired):**

```json
{ "valid": false, "reason": "revoked" }
```

Same logic as the verify licence flow — revalidates the token, issues a fresh
blob with updated `exp` and `rcAfter`. Returns `valid: false` if the token has
been revoked or the user suspended.

**Security:** The refresh endpoint must apply the same `isValidTokenFormat`
guard and `hashToken` before any DB lookup — matching the existing pattern in
`/auth/verify`. The raw token must never be stored or logged server-side.

### New Environment Variables

| Variable              | Location | Purpose                                      |
| --------------------- | -------- | -------------------------------------------- |
| `LICENSE_SIGNING_KEY` | API      | ES256 private key (PEM)                      |
| `LICENSE_PUBLIC_KEY`  | API      | ES256 public key (PEM) — also baked into CLI |

### Database Changes

**Phase 1 (this implementation):** None. The `org`, `tier`, `identity`, and
`seats` claims are hardcoded stubs: `org: null`, `tier: "pro"`,
`identity: { provider: "github", id: null }`, `seats: 1`. The API derives `sub`
and `email` from the existing `beta_users` table.

**Phase 2 (deferred):** Add columns to `beta_users`:

- `github_id text` — social identity anchor
- `org_id uuid references organisations(id)` — organisation membership
- `tier text default 'free'` — subscription tier

These columns are required before the claims can carry real values. A migration
plan will be written when the org/team model is designed.

---

## Security Considerations

- **Key management:** The private signing key must never be committed or logged.
  Store in Vercel environment variables or a secret manager.
- **Public key in CLI:** This is safe — the public key can only verify, not
  sign. It's the same model as TLS certificates.
- **Token vs licence:** The raw token in `auth.json` remains the sensitive
  credential. The licence blob is safe to inspect (it's just signed, not
  encrypted) but should still be file-permission protected to avoid information
  leakage.
- **Revocation latency:** Worst case is `rcAfter` window (7 days) plus one
  additional command. For immediate revocation needs (security incident), the
  API can also reject the raw token on any API call, and the CLI already handles
  401/403 by clearing auth.
- **Key rotation:** Two public keys embedded in CLI (current + previous) gives a
  full release cycle to rotate. Old licences verify against the previous key
  until they expire naturally.
- **Tampering:** Any modification to the licence blob invalidates the ES256
  signature. A user cannot change their tier, extend their expiry, or alter any
  claim.

---

## Testing Plan

- Unit: JWT signing and verification with ES256 (API + CLI)
- Unit: licence file resolution order (env var → project → user)
- Unit: expiry and `rcAfter` threshold checks
- Unit: background refresh success/revoked/network-failure paths
- Unit: refresh deduplication (60s cooldown)
- Unit: logout deletes both auth.json and licence file
- Integration: full login flow — token exchange returns licence blob
- Integration: licence refresh endpoint (must use `isValidTokenFormat` +
  `hashToken`)
- Integration: backwards compatibility — auth.json without licence file
- Manual: dogfood on 4 physical machines + virtual environments

**Implementation note:** The licence file path must be resolved from the same
overridable auth directory used by `auth-store.ts` (`setAuthDir()` /
`_authDirOverride`). Tests that set a custom auth dir must also redirect licence
file reads/writes — otherwise tests will touch `~/.anvil/license` on the real
filesystem.

---

## Future Extensions

These are documented for context but are explicitly out of scope:

- **OAuth / device code flow:** Replace manual token entry with
  `anvil auth login` → browser/device code → GitHub OAuth → licence blob. Same
  licence format, different issuance path.
- **Feature gating:** Read `tier` and `scopes` from licence claims, check
  against feature flag configuration before running gated commands.
- **Seat management:** Use `seats` claim + server-side tracking to enforce
  concurrent user limits per org.
- **Dashboard auth:** Same social identity (`identity` claim) used for web
  dashboard login. Single identity across CLI and web.
- **Device code flow:** GitHub supports `https://github.com/login/device` for
  headless environments. Natural fit for SSH sessions, codespaces, containers.
