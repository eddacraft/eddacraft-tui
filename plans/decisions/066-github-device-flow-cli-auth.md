# ADR-066: GitHub Device Authorization Grant as the default `anvil auth login`

## Status

**Proposed**

## Date

2026-06-03

## Context

`anvil auth login` must work from an SSH/tmux session on a headless box — no
local browser, no reachable loopback callback. The current homegrown
device-code flow does not, and is in fact **un-completable** today.

The website activation page `apps/website/app/auth/activate/page.tsx` posts to
`/device/confirm` with **no** `Authorization` header. That endpoint was
hardened in #1779 (commit `1b0a30a36`) to **require** an
`Authorization: Bearer` licence and to derive the bound identity from that
token — but the website has no login surface and no token store, so it can
never send one. Every confirm returns `401 "Authorization header required"`,
and the CLI simply times out. Admin-invited users are broken by the same change:
`admin.ts` inserts `device_codes` rows and emails an `ACTIVATE_URL?code=` link
that lands on the same dead confirm path.

The robust, well-trodden answer to "log in on a device with no browser" is the
**GitHub Device Authorization Grant** (RFC 8628): the user opens
`github.com/login/device` on *any* device, types a short code, and the CLI
polls for completion. GitHub also already backs our docs-auth GitHub OAuth path
(DOCSAUTH), so the identity provider and the licence-mint plumbing already
exist in-tree and can be reused.

A planning council (problem framing, security council, ops council, delivery
council) reviewed the topology, the account-linking model, the security
invariants, and the cutover sequence. This ADR records the locked decisions.

### Forces

- Login must succeed over SSH/tmux with no local browser and no loopback port.
- The existing `/device/confirm` identity-binding model is structurally broken
  (#1779) and cannot be un-broken without removing the website-as-confirmer
  assumption.
- We already have a working GitHub OAuth → licence-mint path (DOCSAUTH /
  `auth-github.ts`) and an ES256 licence schema that already supports
  `provider: 'github'` — no licence schema change is needed.
- `anvil-api` runs on Vercel serverless: in-memory session maps do not survive
  across instances, so device-flow session state must be DB-backed.
- Outstanding invite emails (~48h window) point at the activation page, so the
  page cannot simply 404.

## Decision

Adopt the **GitHub Device Authorization Grant (RFC 8628)** as the default
`anvil auth login`, **brokered server-side through `anvil-api`**, replacing the
homegrown device-code browser-confirm flow. Email OTP (`anvil auth login
--otp`) is retained unchanged as the no-GitHub fallback.

The locked decisions:

1. **Topology — broker GitHub's device flow server-side in `anvil-api`.** The
   CLI talks only to `anvil-api`; `anvil-api` calls
   `github.com/login/device/code` and the device-token poll endpoint. The
   reason to broker is **identity-brokering** — running the active-status gate
   and minting the Anvil licence server-side — **not** secret custody. GitHub's
   device grant is a **public-client** flow: `client_secret` is **not** used by
   the device-code or token-poll requests (only `client_id` + `device_code`).
   The client secret is therefore **not** the security boundary; do not treat
   it as one.

2. **Dedicated OAuth app.** Register a new GitHub OAuth app **"Anvil CLI"**,
   separate from the existing `eddacraft Docs` app, with **"Enable Device
   Flow" ticked** (a manual GitHub setting). New Key Vault secrets
   `github-cli-client-id` / `github-cli-client-secret` (the secret may be unused
   for the device grant, but store it for completeness/future), wired into
   `anvil-api` **only** (not docs-shell). This avoids shared per-app GitHub
   rate-limit contention with docs auth, avoids the wrong consent-screen
   branding on CLI login, and keeps audit trails separate.

3. **New versioned endpoints.** Add `POST /api/v1/auth/github-device/start` and
   `POST /api/v1/auth/github-device/poll`. Do **not** repurpose the existing
   `/device/start` + `/device/poll`. The CLI is updated to call the new
   endpoints. The old homegrown endpoints remain until retired so
   already-shipped CLIs do not misroute.

4. **Account linking by GitHub numeric id.** Add a `github_id` column to
   `beta_users`; match returning users on `github_id`. Verified-primary-email is
   the fallback **only** for first-link of a pre-existing email-invited beta
   user. This closes the email-change/takeover vector and the
   private-noreply-email problem. Requires a DB migration plus first-login
   linking logic.

5. **Retire via tombstone + admin-invite rebuild.** Replace
   `apps/website/app/auth/activate/page.tsx` with a redirect/tombstone (no 404
   for outstanding ~48h invite-email links); give admin-invited users a working
   activation path (they are currently *also* broken by #1779); and remove
   `POST /auth/device/confirm` **only after** the new CLI ships. Admin-invite
   repointing is in scope for this work.

6. **Email OTP retained.** `anvil auth login --otp` stays the no-GitHub
   fallback and is untouched.

## Rationale

Brokering server-side (rather than the CLI talking to GitHub directly) is the
load-bearing choice, and the reason is **not** secret custody — the device
grant needs no secret. It is that the Anvil **licence mint** must run
server-side: the active-status gate, scope resolution, audit logging, and the
ES256 signing key all live in `anvil-api` and must not move to the client. The
broker is the only place that can turn "this is GitHub user N" into "here is an
Anvil licence", and it is where the active-cohort access control belongs.

A dedicated OAuth app keeps CLI login isolated from docs auth on rate limits,
consent branding, and audit trails — cheap insurance against one app's traffic
or revocation affecting the other.

New versioned endpoints (rather than repurposing the broken ones) mean
already-shipped CLIs that still call `/device/*` keep their current
(failing-closed) behaviour and never silently misroute onto a path with
different semantics; the old path is deleted only once no shipped CLI needs it.

Linking on the GitHub **numeric id** (not email) is the correct identity key:
GitHub usernames and emails change, the numeric id does not, and GitHub users
may present a `noreply` email. Email is used only to first-link a pre-existing
email-invited beta record, behind a verified-primary-email check.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **GitHub device grant brokered in `anvil-api` (chosen)** | Works headless over SSH/tmux (verification on any device); reuses the GitHub identity + licence-mint path already in-tree; licence mint, active-status gate, scopes, and signing stay server-side; standard RFC 8628 semantics | One new OAuth app + Key Vault secrets to provision; a new DB-backed session table; broker proxies a credentialed upstream call (rate-limit + timeout discipline needed) |
| **Fix the homegrown `/device/confirm` website-confirm flow** | No new provider; smallest surface on paper | Structurally broken by #1779 — the website has no login/token store to send the required `Authorization` header; "fixing" it means building a website auth surface, the larger project the device flow was meant to avoid; still needs a local browser to reach the activation page |
| **CLI talks to GitHub directly (no broker)** | No server hop on the hot path | The licence mint, active-status gate, scope resolution, and ES256 signing key cannot move to the client; would either ship secrets/policy to the CLI or require a second server round-trip anyway; loses the single server-side audit + gate point |
| **GitHub-only auth (drop OTP)** | One login path to maintain | Removes the no-GitHub fallback for users without a usable GitHub account; out of scope here — OTP is retained |

## Consequences

- **Positive:** `anvil auth login` works from a headless SSH/tmux box — the
  original bug is closed. The GitHub identity path and the ES256 licence mint
  are reused (no licence schema change; `LicenceClaims` already supports
  `provider: 'github'`). Account linking becomes stable against email/username
  change. Docs auth and CLI auth no longer share an OAuth app, so their rate
  limits, consent branding, and audit trails are independent.

- **Reusable assets (consequence of brokering server-side):**
  - `apps/anvil-api/src/routes/auth-github.ts` (~153–244) already carries
    code→token exchange, `fetchGitHubUser`, `insertPendingUser`, the
    active-status gate, `signLicence` with `identity { provider: 'github', id }`,
    refresh-token issue, and the audit log. Extract a shared
    `mintLicenceForGitHubUser(sql, ghUser)` (a.k.a. `mintSession`) helper, cut
    at the **`ghUser`** boundary (the GitHub *token* stays per-caller — token
    revoke is the caller's responsibility), reused by `/github/callback` and the
    new device-poll path. The same inline block is duplicated across
    `auth-device.ts`, `auth-otp.ts`, and `auth-session.ts` and is folded in by
    the same extraction.
  - `apps/anvil-api/src/lib/licence.ts` `signLicence` (ES256, issuer
    `https://api.eddacraft.ai`, aud `anvil-cli`) is unchanged.

- **Security invariants (must hold):**
  - `/github-device/start` accepts **no** email and binds **no** user. The bound
    `user_id` is derived **solely** from `fetchGitHubUser(github_access_token)`
    at poll-confirmation time — this is what prevents the #1779 re-entry.
  - The GitHub device request includes `scope=read:user user:email`; the mint
    **requires** `email.primary && email.verified` (fail-closed) on the
    first-link fallback path.
  - Active-status gate **parity**: device-poll mint enforces
    `status === 'active'` identically to `auth-github.ts:195-202`, emits a
    `github_oauth_blocked` audit on non-active, and inherits scopes via
    `findActiveScopesForUser`.
  - Per-`poll_token` isolation + single-use mint: the poll lookup is keyed
    strictly on `poll_token = $hash` (never "latest confirmed"); the mint is
    gated by an atomic `DELETE … RETURNING` (reuse the `consumeDeviceCode`
    pattern). The 256-bit `poll_token` is hashed at rest with the existing
    pepper; the GitHub `device_code` is also hashed at rest.
  - **DB-backed** session state (Vercel serverless — in-memory maps do not
    survive across instances). A new `github_device_sessions` table keyed by
    `poll_token_hash` → `(github_device_code_hash, interval_s, expires_at,
    last_polled_at, minted_*)` — cleaner than overloading `device_codes`, whose
    `user_code UNIQUE NOT NULL` + start-time `user_id` invariants the GitHub
    flow structurally breaks.
  - Mint **exactly once, re-returnable within TTL** — a lost response must not
    turn a success into a false "expired".
  - **Revoke** the GitHub access token immediately after `fetchGitHubUser`,
    before returning the licence.
  - **No secrets in structured logs.** Never pass `access_token` /
    `device_code` / `poll_token` / `license` as `debug()` object fields —
    `sanitizeForLog` redacts strings, not object values
    (`apps/anvil-api/src/lib/debug.ts`). Log booleans, ids, and counts only.
  - **CSRF is N/A by design** (no redirect, no `anvil` session cookie on the
    device path) — recorded so nobody re-adds nonce ceremony.

- **Ops preconditions (must hold):**
  - Explicit request timeouts (~8s, below the Vercel function ceiling) on every
    `github.com` fetch on the login hot path.
  - Rate-limit `/github-device/start` (per-IP + global) since it proxies a
    credentialed upstream call; keep a per-`poll_token` cooldown on `/poll`;
    honour GitHub `interval` + `slow_down` (RFC 8628 §3.5) and pass them through
    to the CLI.
  - A cross-instance (DB-backed) gate so at most one Vercel instance polls
    GitHub per `device_code` per interval window.
  - A boot-time probe for the new GitHub CLI credentials wired into
    `apps/anvil-api/src/index.ts` and reflected in `/health`.
  - Structured `console.info` (not gated `console.debug`) for upstream call
    outcomes (latency, error class) — no secret values.
  - A runbook/smoke step verifying "Device Flow enabled" on the new OAuth app
    before cutover.
  - CLI 429/`slow_down` handling is currently a **fatal bail**
    (`crates/anvil-cli/src/auth/device_flow.rs` `check_status` →
    `friendly_http_error(429)` → `bail`) — it must become a back-off+retry, and
    `DeviceStartResponse` must gain an `interval` field (the server already
    returns one but the struct drops it).

- **Negative / cost:** A new GitHub OAuth app and Key Vault secrets to
  provision (one manual "Enable Device Flow" GitHub step); a DB migration
  (`github_id` column + `github_device_sessions` table); two new endpoints plus
  a CLI rewrite of the default login path; a website page tombstoned and the
  admin-invite path repointed and rebuilt.

- **Risks:** (1) The manual "Enable Device Flow" tick is easy to miss — the
  runbook smoke step is the mitigation, and it is a hard gate for live
  end-to-end testing. (2) The broker proxies a credentialed upstream call, so a
  missing timeout or rate limit could expose the login path to upstream
  slowness/abuse — addressed by the ops preconditions above. (3) Removing
  `/device/confirm` before the new CLI is in users' hands would strand them —
  hence "remove only after the new CLI ships".

- **Out of scope:** GitHub-only auth (dropping OTP); docs-site web OAuth
  changes; feature-flagging the device path; metrics dashboards; Windows-specific
  auth; and an explicit `--github` selector flag (device flow is simply the
  default).

## References

- Related ADRs: [ADR-043](043-ssh-remote-host-daemon.md) (SSH remote host
  daemon — shares the SSH-first motivation; cross-reference only, no
  dependency).
- Supersedes the device-confirm sub-flow of **BAUTH**
  ([`beta-auth-streamline`](../archive/modules/beta-auth-streamline.aps.md),
  archived Complete) and reuses the GitHub OAuth path from **DOCSAUTH**
  ([`docs-auth-gating`](../archive/modules/docs-auth-gating.aps.md), archived
  Complete).
- APS module: GHCLIAUTH
  ([`github-cli-auth`](../modules/github-cli-auth.aps.md)).
- External: [RFC 8628 — OAuth 2.0 Device Authorization Grant](https://datatracker.ietf.org/doc/html/rfc8628)
  (§3.5 `slow_down`/`interval` polling).
- Evidence: `apps/anvil-api/src/routes/auth-device.ts` (`/device/confirm`,
  #1779 / `1b0a30a36`); `apps/website/app/auth/activate/page.tsx`;
  `apps/anvil-api/src/routes/auth-github.ts:153-244`;
  `apps/anvil-api/src/lib/licence.ts`; `apps/anvil-api/src/lib/debug.ts`;
  `apps/anvil-api/src/routes/admin.ts`;
  `crates/anvil-cli/src/auth/device_flow.rs`.
