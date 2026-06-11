<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# GitHub CLI Auth

| ID        | Owner  | Status      | Progress |
| --------- | ------ | ----------- | -------- |
| GHCLIAUTH | @aneki | In Progress | 7/11     |

**Last reviewed:** 2026-06-11 (GHCLIAUTH-007 Merged via PR #2549: the
activation page is a tombstone, the invite email directs to
`anvil auth login` (`--otp` fallback), and the interactive device-code
generation is gone from **both** `/admin/invite` and `/admin/approve` —
the approve path was newly discovered scope, recorded as a drift correction in
the item; its `access_tokens` scope-record insert is kept. Admin-invited
activation works again via first-login GitHub linking or OTP.
Remaining: GHCLIAUTH-009 (observability + runbook) and 011 (headless E2E
smoke) are unblocked — 009 is picked up, its In Progress flip lands with
its PR; 008 (confirm-endpoint removal) waits for the next CLI tag per the
Release wave; 010 (docs sync) depends on 008.)

## Purpose

Make `anvil auth login` work from a headless SSH/tmux session by replacing the
broken homegrown device-code browser-confirm flow with the **GitHub Device
Authorization Grant** (RFC 8628), brokered server-side through `anvil-api`
([ADR-066](../decisions/066-github-device-flow-cli-auth.md)). The user opens
`github.com/login/device` on any device and the CLI polls `anvil-api` for the
minted Anvil licence. Email OTP (`anvil auth login --otp`) is retained as the
no-GitHub fallback.

**Why now:** `/device/confirm` was hardened by #1779 to require an
`Authorization` header that the website activation page cannot send, so device
login is **un-completable** today and the CLI just times out. Admin-invited
users are broken by the same change. This module fixes login end-to-end and
retires the dead path.

## MVP cut line

- **MVP (SSH/tmux login works end-to-end):** GHCLIAUTH-001..-006, with
  GHCLIAUTH-002 (the OAuth-app + Key Vault ops gate) as a hard precondition for
  live end-to-end testing.
- **Correctness / cleanup / validation:** GHCLIAUTH-007..-011 (tombstone +
  admin-invite rebuild, confirm-endpoint removal, observability, docs, and the
  acceptance smoke test).

## In Scope

- The GitHub Device Authorization Grant (RFC 8628) brokered server-side in
  `anvil-api`, behind new versioned endpoints
  `POST /api/v1/auth/github-device/{start,poll}`
- A dedicated "Anvil CLI" GitHub OAuth app (separate from `eddacraft Docs`,
  Device Flow enabled) + Key Vault `github-cli-client-id` / `github-cli-client-secret`, wired into
  `anvil-api` only
- Account linking on the GitHub numeric `github_id` (first-link of an
  email-invited record via **any verified** GitHub email) and the DB migration
  it needs; invitation stays email-keyed with GitHub as a linked auth method
  (ADR-066 decision 7)
- A DB-backed `github_device_sessions` table and the single-use, hashed-at-rest
  session/mint lifecycle
- A shared `mintLicenceForGitHubUser` helper extracted from `auth-github.ts`,
  reused by `/github/callback` and the new device-poll path
- The CLI default-login rewrite (call the new endpoints, drop the email prompt,
  honour `interval` + `slow_down` back-off, map terminal states)
- Tombstoning the website activation page and rebuilding the admin-invite
  activation path; removing `POST /auth/device/confirm` after the new CLI ships
- Observability + ops hardening (upstream-call logging, rate limits, timeouts,
  cross-instance poll gate, boot probe, runbook)
- Docs sync and a headless end-to-end acceptance smoke test

## Out of Scope

- GitHub-only auth / dropping Email OTP — `--otp` is retained and untouched
- Docs-site web OAuth changes (DOCSAUTH stays as-is; CLI auth gets its own app)
- Feature-flagging the device path (device flow is simply the default)
- Metrics dashboards for auth
- Windows-specific auth work
- An explicit `--github` selector flag (device flow is the default, not a
  selectable mode)
- Any change to the ES256 licence schema — `LicenceClaims` already supports
  `provider: 'github'` (ADR-066)

## Interfaces

**Depends on:**

- [ADR-066](../decisions/066-github-device-flow-cli-auth.md) — the topology,
  account-linking model, security invariants, and cutover sequence
- `apps/anvil-api/src/routes/auth-github.ts` — the GitHub code→token exchange,
  `fetchGitHubUser`, active-status gate, `signLicence`, refresh token, and audit
  log to extract from and reuse
- `apps/anvil-api/src/lib/licence.ts` `signLicence` (ES256) — unchanged
- `infra/src/vercel.ts` + Key Vault — for the new OAuth-app secrets
- GitHub `github.com/login/device/code` + device-token poll endpoint (RFC 8628)

**Exposes:**

- `POST /api/v1/auth/github-device/start` and `.../poll`
- A working headless `anvil auth login` default path, with `--otp` retained
- A rebuilt admin-invite activation path
- The shared `mintLicenceForGitHubUser` helper for GitHub-identity licence mint

## Cross-references

- **Supersedes** the device-confirm sub-flow of **BAUTH**
  ([`beta-auth-streamline`](../archive/modules/beta-auth-streamline.aps.md),
  archived Complete).
- **Reuses** the GitHub OAuth path from **DOCSAUTH**
  ([`docs-auth-gating`](../archive/modules/docs-auth-gating.aps.md), archived
  Complete).
- Shares the SSH-first motivation with **SSHREMOTE**
  ([`ssh-remote-host-daemon`](ssh-remote-host-daemon.aps.md), Proposed,
  ADR-043) — cross-reference only, no dependency.

## Release wave

- **Latest tag:** `v0.7.4-beta` (shipped 2026-05-31). **Active window:**
  `v0.8.0-beta` "The Save-Time Daemon" (six-week cadence retired; minors cut
  when ready + gates green) — see
  [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md). (The `plans/index.aps.md` prose
  is stale at `v0.7.2-beta`/the `v0.7.3-beta` candidate.)
- The **TS API slice** (GHCLIAUTH-001/-003/-004/-005, the server endpoints) is
  continuous-deploy and can land on `main` before any tag.
- The **Rust CLI slice** (GHCLIAUTH-006) gates on the next CLI tag.
- The confirm-endpoint removal (GHCLIAUTH-008) is sequenced after the new CLI
  ships, per ADR-066.

## Ready Checklist

Change status to **Ready** when:

- [x] The "Anvil CLI" GitHub OAuth app is registered with Device Flow enabled
      and `github-cli-client-id` / `github-cli-client-secret` are provisioned (GHCLIAUTH-002 —
      verified live via `/api/v1/health` `"githubCliCreds":"ok"`, 2026-06-11)
- [x] The security invariants in ADR-066 are signed off (no email on `/start`,
      identity from `fetchGitHubUser` only, fail-closed verified-email, gate
      parity, single-use hashed-at-rest mint) — owner sign-off 2026-06-11
- [x] The ops preconditions in ADR-066 are signed off (timeouts, rate limits,
      cross-instance poll gate, boot probe, `slow_down` back-off) — owner
      sign-off 2026-06-11

## Work Items

### GHCLIAUTH-001: Extract shared GitHub-user licence-mint helper

- **Status:** Merged 2026-06-04 via PR #2302
- **Intent:** Give the GitHub-identity licence mint a single home so the new
  device-poll path and the existing callback path mint identically.
- **Expected Outcome:** A `mintLicenceForGitHubUser(sql, ghUser)` (a.k.a.
  `mintSession`) helper owns the `ghUser`→licence path (active-status gate,
  scope resolution via `findActiveScopesForUser`, `signLicence`, refresh token,
  audit log); `/github/callback` and the duplicated inline blocks in
  `auth-device.ts` / `auth-otp.ts` / `auth-session.ts` call it; behaviour is
  byte-identical (the cut is at the `ghUser` boundary — the GitHub token revoke
  stays per-caller).
- **Validation:** `pnpm nx test @eddacraft/anvil-api` — the existing
  GitHub-callback and OTP mint tests pass unchanged against the extracted
  helper.
- **Files:** `apps/anvil-api/src/routes/auth-github.ts`,
  `apps/anvil-api/src/routes/{auth-device,auth-otp,auth-session}.ts`,
  `apps/anvil-api/src/lib/` (new helper module)
- **Dependencies:** None
- **Confidence:** high
- **Size:** S
- **Source:** ADR-066 "Reusable assets".

---

### GHCLIAUTH-002: Provision the "Anvil CLI" GitHub OAuth app + Key Vault + boot probe

- **Status:** Merged 2026-06-08 via PR #2318
- **Operator gate:** Cleared. The OAuth app is registered (Device Flow enabled)
  and the `github-cli-client-id`/`-secret` Key Vault secrets are provisioned —
  verified live 2026-06-11: `/api/v1/health` reports `"githubCliCreds":"ok"`.
  The formal runbook still lands in GHCLIAUTH-009.
- **Intent:** Stand up the dedicated CLI OAuth app and its credentials so the
  device flow has an isolated, Device-Flow-enabled identity provider.
- **Expected Outcome:** A new "Anvil CLI" GitHub OAuth app exists with **Device
  Flow enabled** (manual GitHub step); `github-cli-client-id` /
  `github-cli-client-secret` are provisioned in Key Vault and wired into
  `anvil-api` **only** (not docs-shell) via `infra/src/vercel.ts`; a boot-time
  probe in `apps/anvil-api/src/index.ts` validates the credentials and `/health`
  reflects their presence.
- **Validation:** `/health` reports the GitHub CLI credential as present on a
  deploy with the secrets set and absent without them;
  `pnpm nx test @eddacraft/anvil-api` covers the probe wiring. The "Device Flow
  enabled" tick is verified by the GHCLIAUTH-009 runbook smoke step.
- **Files:** `infra/src/vercel.ts`, `apps/anvil-api/src/index.ts`,
  `apps/anvil-api/src/routes/health.ts` (or equivalent `/health` source)
- **Dependencies:** None (parallelable; **hard precondition** for live
  end-to-end testing)
- **Confidence:** medium
- **Size:** S–M
- **Source:** ADR-066 decision 2; ops preconditions.

---

### GHCLIAUTH-003: `beta_users.github_id` migration + first-login linking

- **Status:** Merged 2026-06-04 via PR #2322
- **Intent:** Link GitHub identities on the stable numeric id so returning users
  match deterministically and the email-change/takeover vector is closed, while
  binding an email-keyed invite to the authenticating GitHub account on first
  login (ADR-066 decision 7 — email-keyed account, GitHub as a linked method).
- **Expected Outcome:** A DB migration adds `github_id` to `beta_users`;
  first-login linking matches a returning user on `github_id`. For first-link of
  a pre-existing email-invited record it matches **any of the account's
  `verified` emails** (not just the primary) against the active invited row and
  then stores `github_id`; an active invite is **never** shadowed by a silently
  created `pending` duplicate, and an **unverified** email never matches
  (fail-closed). Once `github_id` is stored, email is not consulted again.
- **Validation:** `pnpm nx test @eddacraft/anvil-api` — linking tests cover
  returning-by-`github_id`, first-link via a non-primary verified email,
  noreply primary email, unverified-email-rejected, "active invite is linked not
  duplicated", and the email-takeover-rejected case; the migration applies
  cleanly forward.
- **Files:** `apps/anvil-api/migrations/` (new migration),
  `apps/anvil-api/src/lib/` (linking logic), the new
  `mintLicenceForGitHubUser` helper
- **Dependencies:** GHCLIAUTH-001
- **Confidence:** medium
- **Size:** M
- **Source:** ADR-066 decision 4; security invariants.

---

### GHCLIAUTH-004: `github_device_sessions` table + `/github-device/start` broker

- **Status:** Merged 2026-06-11 via PR #2540
- **Intent:** Begin a device-flow session by brokering GitHub's device-code
  request server-side and persisting the session for cross-instance polling.
- **Expected Outcome:** A new `github_device_sessions` table keyed by
  `poll_token_hash` holds `(github_device_code_enc, interval_s, expires_at,
  last_polled_at, minted_*)`; `POST /api/v1/auth/github-device/start` calls
  `github.com/login/device/code` (scope `read:user user:email`), persists the
  encrypted `device_code` (keyed off the client-held `poll_token` — a one-way
  hash cannot support the poll-time exchange; see the corrected ADR-066
  invariant) + interval/expiry, and returns the RFC 8628 fields
  (`user_code`, `verification_uri`, `interval`, `expires_in`) plus the
  256-bit `poll_token`; the endpoint accepts **no** email and binds **no**
  user; it is rate-limited (per-IP + global) and uses an ~8s upstream timeout.
- **Validation:** `pnpm nx test @eddacraft/anvil-api` — start tests cover the
  no-email/no-user-bind invariant, hashed-at-rest persistence, RFC 8628 response
  shape, rate-limit rejection, and the upstream-timeout path (GitHub mocked).
- **Files:** `apps/anvil-api/migrations/` (session table),
  `apps/anvil-api/src/routes/auth-github-device.ts` (new),
  `apps/anvil-api/src/index.ts` (route registration)
- **Dependencies:** GHCLIAUTH-001; GHCLIAUTH-002 for live end-to-end testing
- **Confidence:** medium
- **Size:** M
- **Source:** ADR-066 decision 3; security + ops invariants.

---

### GHCLIAUTH-005: `/github-device/poll` broker — exchange, gate, single-use mint

- **Status:** Merged 2026-06-11 via PR #2543
- **Intent:** Complete a device-flow session by exchanging the device code,
  deriving identity from GitHub, enforcing the access gate, and minting the
  Anvil licence exactly once.
- **Expected Outcome:** `POST /api/v1/auth/github-device/poll` exchanges the
  stored `device_code`, maps GitHub's `authorization_pending` / `slow_down` /
  `expired_token` / `access_denied` responses (passing `slow_down`/`interval`
  through to the CLI); derives `user_id` **solely** from
  `fetchGitHubUser(token)`'s `github_id` (linking per GHCLIAUTH-003, matching any
  `verified` email on first-link, fail-closed on unverified); enforces
  active-status gate parity (`status === 'active'`, `github_oauth_blocked` audit
  on non-active, scopes via `findActiveScopesForUser`) and returns a clear
  terminal "awaiting approval" poll status for non-active/uninvited users (not a
  generic timeout); mints **single-use** via an atomic UPDATE-where-unminted
  (the `consumeDeviceCode` atomicity model — UPDATE, not `DELETE … RETURNING`,
  because deleting the row would make re-returning the minted session
  impossible; corrected during implementation), keyed strictly on
  `poll_token_hash = $hash`; mints **exactly once, re-returnable within TTL**;
  revokes the GitHub token immediately after `fetchGitHubUser`; a cross-instance
  DB gate ensures at most one instance polls GitHub per `device_code` per
  interval window; secrets never appear in structured logs.
- **Validation:** `pnpm nx test @eddacraft/anvil-api` — poll tests cover
  pending/slow_down/expired/declined mapping, identity-from-token only, the
  fail-closed verified-email check, gate parity + `github_oauth_blocked`,
  single-use mint (second mint rejected), re-returnable-within-TTL, token
  revoke, and the cross-instance poll gate.
- **Files:** `apps/anvil-api/src/routes/auth-github-device.ts`, the new
  `mintLicenceForGitHubUser` helper, `apps/anvil-api/src/lib/debug.ts` (log
  hygiene)
- **Dependencies:** GHCLIAUTH-003, GHCLIAUTH-004
- **Confidence:** medium
- **Size:** M–L
- **Source:** ADR-066 decision 1/3; security + ops invariants.

---

### GHCLIAUTH-006: CLI default login on the new endpoints + back-off

- **Status:** Merged 2026-06-11 via PR #2545 (+ PR #2546, `/health` gate)
- **Intent:** Make the default `anvil auth login` drive the brokered device flow
  and survive GitHub's polling back-off instead of bailing.
- **Expected Outcome:** The default `login_device_flow` calls the new
  `/github-device/{start,poll}` endpoints and drops the email prompt;
  `DeviceStartResponse` gains an `interval` field (the server already returns
  one); the CLI honours `interval` + `slow_down` back-off (the current fatal
  429 bail in `check_status` → `friendly_http_error(429)` → `bail` becomes a
  back-off+retry); terminal states (`expired`, `declined`, errors) map to clear
  messages; `--otp` and `--edict` are untouched; `auth login --help` and prompts
  are updated. Once the device flow is live by default, `/health` should treat
  missing `GITHUB_CLI_*` credentials as `degraded` (today the field is
  informational only — GHCLIAUTH-002 left it non-gating because the flow
  wasn't user-facing yet).
- **Validation:** `cargo test -p eddacraft-anvil -- auth` — device-flow unit
  tests cover the new endpoints, the `interval` field round-trip, `slow_down`
  back-off (no fatal bail), and terminal-state messages; `--otp` tests unchanged.
- **Files:** `crates/anvil-cli/src/auth/device_flow.rs`,
  `crates/anvil-cli/src/auth/` (login dispatch + prompts),
  `crates/anvil-cli/src/commands/auth.rs`
- **Dependencies:** GHCLIAUTH-005; GHCLIAUTH-002 for live end-to-end testing
- **Confidence:** medium
- **Size:** M
- **Source:** ADR-066 decision 1/6; ops preconditions (CLI 429 handling).

---

### GHCLIAUTH-007: Tombstone the activation page + rebuild admin-invite activation

- **Status:** Merged 2026-06-11 via PR #2549
- **Intent:** Stop the dead activation page from 404-ing outstanding invite
  links and rebuild admin-invite activation around the email-keyed model
  (ADR-066 decision 7).
- **Expected Outcome:** `apps/website/app/auth/activate/page.tsx` becomes a
  redirect/tombstone (no 404 for outstanding ~48h invite-email links); the
  `sendBetaInvite` email is rewritten to **drop** the `ACTIVATE_URL`/`userCode`
  and direct the recipient to `anvil auth login` (GitHub device flow) with
  `--otp` to the invited address as the fallback; the now-vestigial interactive
  device-code generation is removed from **both** `/admin/invite` and
  `/admin/approve` (drift correction 2026-06-11: the approve path writes the
  same `device_codes` row and sends the same activate email — the item
  originally named only `/admin/invite`); the `tokenOnly` CI/service-account
  path is **unaffected**, and `/admin/approve` keeps its `access_tokens`
  insert — that row is the scope record `findActiveScopesForUser` reads at
  mint time, not a usable bearer token; admin-invited-user activation works
  again (it is currently broken by #1779) via first-login GitHub linking or OTP.
- **Validation:** `pnpm nx test @eddacraft/anvil-api` + the website build/test —
  a freshly invited user logs in via `anvil auth login` (links `github_id`) and
  via `--otp` to the invited email; neither `/admin/invite` nor `/admin/approve`
  writes a `device_codes` row on the interactive path while `tokenOnly` still
  mints a token; an old `ACTIVATE_URL?code=` link resolves to the tombstone
  (not a 404).
- **Files:** `apps/website/app/auth/activate/page.tsx`,
  `apps/anvil-api/src/routes/admin.ts`, `apps/anvil-api/src/lib/email.ts`
  (`sendBetaInvite`), `apps/anvil-api/src/lib/email-registry.ts`
  (`betaInvitePropsSchema`), `packages/transactional/emails/beta-invite.tsx`
  (`BetaInvite` template)
- **Dependencies:** GHCLIAUTH-006
- **Confidence:** medium
- **Size:** M
- **Source:** ADR-066 decisions 5/7.

---

### GHCLIAUTH-008: Remove `POST /auth/device/confirm` + #1779 dead code

- **Status:** Proposed
- **Intent:** Retire the broken confirm path once no shipped CLI needs it.
- **Expected Outcome:** `POST /auth/device/confirm` and its #1779
  attempt-counter / anti-enumeration dead code (and tests) are removed; orphaned
  `device_codes` confirm-only queries are inventoried and deleted; this lands
  **only after** the new CLI ships.
- **Validation:** `pnpm nx test @eddacraft/anvil-api` passes with the route and
  its tests removed; `rg "device/confirm|confirmDeviceCode"` (or equivalent)
  returns no live references.
- **Files:** `apps/anvil-api/src/routes/auth-device.ts`,
  `apps/anvil-api/src/index.ts`, related test files
- **Dependencies:** GHCLIAUTH-006, GHCLIAUTH-007
- **Confidence:** medium
- **Size:** M
- **Source:** ADR-066 decision 5.

---

### GHCLIAUTH-009: Observability + ops hardening + runbook

- **Status:** In Progress
- **Intent:** Make the login hot path observable and operable without leaking
  secrets, and document the cutover gate.
- **Expected Outcome:** Structured `console.info` (not gated `console.debug`)
  logs upstream-call outcomes (latency, error class) with a distinct debug
  namespace and **no** secret values (`access_token`/`device_code`/`poll_token`/
  `license` never passed as `debug()` object fields); a runbook under
  `docs/runbooks/` documents the device-flow operations including a
  "verify Device Flow enabled on the Anvil CLI OAuth app" smoke step before
  cutover.
- **Validation:** `test -f docs/runbooks/*github-device*`; a log-hygiene test
  asserts the structured-log fields carry no secret values;
  `pnpm run format:check`.
- **Files:** `apps/anvil-api/src/routes/auth-github-device.ts`,
  `apps/anvil-api/src/lib/debug.ts`, `docs/runbooks/` (new runbook)
- **Dependencies:** GHCLIAUTH-004, GHCLIAUTH-005
- **Confidence:** medium
- **Size:** S–M
- **Source:** ADR-066 ops preconditions; security log-hygiene invariant.

---

### GHCLIAUTH-010: Docs sync — auth/activation/quickstart/beta guide

- **Status:** Proposed
- **Intent:** Bring the user and architecture docs in line with the new login
  flow.
- **Expected Outcome:** `docs/architecture/auth-as-built.md`,
  `docs/architecture/activation-as-built.md`,
  `docs/public/anvil/quickstart.md`, and
  `docs/public/anvil/beta-testing-guide.md` describe the device flow ("open the
  URL on any device", no email prompt, no activation page) and drop the retired
  confirm/activation narrative.
- **Validation:** `pnpm docs:check` (or the docs governance/lint gate) passes;
  `pnpm run format:check`; the device-flow language is present and the
  activation-page narrative is gone.
- **Files:** `docs/architecture/auth-as-built.md`,
  `docs/architecture/activation-as-built.md`,
  `docs/public/anvil/quickstart.md`,
  `docs/public/anvil/beta-testing-guide.md`
- **Dependencies:** GHCLIAUTH-008
- **Confidence:** high
- **Size:** S
- **Source:** ADR-066 cutover sequence.

---

### GHCLIAUTH-011: End-to-end headless smoke test

- **Status:** Proposed
- **Intent:** Prove the original bug is closed — a headless login completes end
  to end — and keep it from regressing.
- **Expected Outcome:** A wiremock-backed CLI integration test drives
  start→poll→confirmed and the `expired` / `declined` / `slow_down` branches,
  asserts credentials are saved and `anvil auth whoami` resolves, and asserts
  `--otp` still works. This is the acceptance test for the original
  un-completable-login bug.
- **Validation:** `cargo test -p eddacraft-anvil -- device_flow_e2e` (or the
  equivalent integration test target) passes against the mocked broker.
- **Files:** `crates/anvil-cli/tests/` (new integration test + wiremock
  fixtures)
- **Dependencies:** GHCLIAUTH-006
- **Confidence:** medium
- **Size:** M
- **Source:** ADR-066 — the acceptance test that closes the original bug.

## Decisions

1. **Broker server-side, not for secret custody** — the device grant is a
   public-client flow (no `client_secret`); brokering exists so the licence
   mint, active-status gate, scope resolution, and ES256 signing stay
   server-side (ADR-066 decision 1).
2. **New endpoints, old path retired last** — ship
   `/github-device/{start,poll}` and remove `/device/confirm` only after the
   new CLI is in users' hands (ADR-066 decisions 3/5).
3. **Link on `github_id`, not email** — first-link of an email-invited record
   matches **any verified** GitHub email, then `github_id` is authoritative
   (ADR-066 decision 4).
4. **OTP retained as the email-proof fallback** — `--otp` is the guaranteed way
   an invited user proves the invited email when no verified GitHub email matches
   (ADR-066 decision 6).
5. **Invitation stays email-keyed; GitHub is a linked auth method** — the
   email funnel (waitlist → approve → invite) is unchanged; the invite email
   drops the retired activate URL/code and points at `anvil auth login`/`--otp`,
   and `/admin/invite`'s vestigial interactive device code is removed
   (ADR-066 decision 7).

## Stats

| Slice | Items | Completion | Status |
| ----- | ----- | ---------- | ------ |
| MVP — headless login (001–006, gated on 002) | 6 | 6/6 done | Complete |
| Correctness / cleanup / validation (007–011) | 5 | 1/5 done | In Progress |
| **Total** | **11** | **7/11 done** | **In Progress** |
