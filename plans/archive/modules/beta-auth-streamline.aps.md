<!-- Archived: 2026-03-27 | Reason: All work items complete (20/20) -->
<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Beta Auth Streamline

| Scope | Owner | Priority | Status      |
| ----- | ----- | -------- | ----------- |
| BAUTH | —     | high     | Complete    |

## Purpose

Streamline the beta access flow from admin approval to CLI activation. Replace
the manual "admin generates token, sends it to user, user pastes it" process
with automated device code and email OTP activation flows.

**Problem:** The waitlist and invite systems are disconnected. Admin must
manually generate tokens via curl, deliver them out-of-band, and users must
paste long token strings into the CLI. This creates friction, is error-prone,
and doesn't scale for cohort-based onboarding.

**Solution:** A thin bridge connecting waitlist → approval → automated token
delivery → CLI activation via device code or email OTP, with short-lived JWT
sessions and rotating refresh tokens.

**Design Spec:** `docs/specs/2026-03-15-beta-auth-streamline-design.md`

## In Scope

**Admin approval CLI + API:**

- `anvil admin approve <email>` and `--batch N` mode
- Single endpoint promotes waitlist → beta_users, generates token, sends invite
- Resend audience management (waitlist → beta-users)

**Device code activation:**

- CLI-initiated: `anvil auth login` → device code → browser confirmation → JWT
- Email-initiated: magic link in invite email → same confirmation flow
- Browser confirmation page at `eddacraft.ai/auth/activate`

**Email OTP activation:**

- CLI-initiated: `anvil auth login --otp` → email OTP → JWT
- Headless fallback for SSH/remote environments

**JWT session model:**

- 7-day JWTs replacing 90-day tokens (for new flows)
- Rotating refresh tokens with 90-day expiry
- Family-based theft detection
- Auto-refresh in CLI

**Email templates:**

- Beta invite email (approval notification + activation instructions)
- OTP code email (minimal, just the code)

## Out of Scope (v1)

- GitHub OAuth (future — separate design)
- Dashboard approval UI (CLI command suffices)
- Automatic waitlist → approval (remains manual)
- Organisation/team model (stays hardcoded `null`)
- Tier differentiation (stays hardcoded `pro`)
- Shared rate limiting (stays in-memory best-effort)
- JWKS endpoint (future)

## Interfaces

**Depends on:**

- Existing `beta_users`, `access_tokens`, `waitlist` tables
- Existing `signLicence` function (parameterised for TTL)
- Existing Resend email infrastructure (`packages/transactional`)
- Existing admin auth middleware

**Exposes:**

- `POST /api/v1/admin/approve` — admin approval endpoint
- `POST /api/v1/auth/device/{start,confirm,poll}` — device code flow
- `POST /api/v1/auth/otp/{request,verify}` — email OTP flow
- `POST /api/v1/auth/session/refresh` — refresh token rotation
- `anvil auth login [--otp]` — CLI activation command
- `anvil admin approve <email | --batch N>` — CLI admin command
- `eddacraft.ai/auth/activate` — browser confirmation page

## Boundary Rules

- Users never see or handle raw `anvil_beta_*` tokens
- All unauthenticated endpoints return identical response shapes (anti-enumeration)
- Device codes and OTP codes are single-use and time-limited
- Refresh tokens use family-based rotation with theft detection
- Audience management is best-effort — failures must not block approval/revocation
- Old `/auth/verify` and `/auth/license/refresh` remain for backward compat
- This is beta scaffolding — prefer simple over extensible

## Acceptance Criteria

- [ ] Admin can approve a waitlist user with one CLI command
- [ ] Approved user receives invite email with magic link + device code
- [ ] User can activate via device code flow (browser + CLI)
- [ ] User can activate via email OTP flow (headless)
- [ ] Activation returns 7-day JWT + rotating refresh token
- [ ] CLI auto-refreshes JWT before expiry
- [ ] Expired/revoked refresh tokens trigger re-authentication prompt
- [ ] Reuse of consumed refresh token revokes entire family
- [ ] Waitlist signups are added to Resend waitlist audience
- [ ] Approved users move from waitlist to beta-users audience
- [ ] Existing token-based auth continues to work (backward compat)
- [ ] Anti-enumeration: no endpoint reveals whether an email is in the system

## Risks & Mitigations

| Risk                                  | Mitigation                                                |
| ------------------------------------- | --------------------------------------------------------- |
| Device code brute force               | 15-min expiry, single-use, rate limiting                  |
| OTP brute force                       | 3 attempts per code, 3 codes per user, 10-min expiry      |
| Refresh token theft                   | Family rotation — consumed token reuse revokes all         |
| Rate limiter resets on cold start     | Known limitation (G-04); acceptable for beta scale         |
| Resend audience API shape mismatch    | contacts.remove needs ID lookup; best-effort with logging  |
| Old-flow users get different JWT TTL  | signLicence parameterised; old flows keep 90-day default   |
| Magic link in email forwarded         | Link expires with device code; single-use                  |

## Tasks

### Phase A: Database & Foundation

#### BAUTH-001: Database schema migration

- **Intent:** Add tables for device codes, OTP codes, and refresh tokens
- **Expected Outcome:** Three new tables with indexes, added to schema.sql
- **Scope:** `apps/anvil-api/src/db/`
- **Non-scope:** Query functions (separate task)
- **Files:**
  - `apps/anvil-api/src/db/schema.sql` (append new tables)
- **Dependencies:** —
- **Validation:** Tables created in Neon without errors
- **Confidence:** high
- **Status:** Complete

#### BAUTH-002: Query functions for new tables

- **Intent:** Add Zod-validated query functions for device_codes, otp_codes, refresh_tokens
- **Expected Outcome:** CRUD + lookup functions following existing queries.ts patterns
- **Scope:** `apps/anvil-api/src/db/`
- **Non-scope:** Route handlers
- **Files:**
  - `apps/anvil-api/src/db/queries.ts` (extend)
- **Dependencies:** BAUTH-001
- **Validation:** `pnpm -F @eddacraft/anvil-api typecheck`
- **Confidence:** high
- **Status:** Complete

#### BAUTH-003: Parameterise signLicence TTL

- **Intent:** Add optional `ttlDays` parameter to signLicence for 7-day JWTs
- **Expected Outcome:** Old callers unchanged (90-day default), new flows pass 7
- **Scope:** `apps/anvil-api/src/lib/`
- **Non-scope:** Removing rcAfter from old flows
- **Files:**
  - `apps/anvil-api/src/lib/licence.ts`
  - `apps/anvil-api/src/lib/__tests__/licence.test.ts`
- **Dependencies:** —
- **Validation:** `pnpm -F @eddacraft/anvil-api test -- --testNamePattern="licence"`
- **Confidence:** high
- **Status:** Complete

### Phase B: Email Templates

#### BAUTH-004: Beta invite email template

- **Intent:** Create invite email with magic link, device code, and CLI instructions
- **Expected Outcome:** React Email component in transactional package
- **Scope:** `packages/transactional/emails/`
- **Non-scope:** OTP template (separate task)
- **Files:**
  - `packages/transactional/emails/beta-invite.tsx`
  - `packages/transactional/emails/index.ts` (re-export)
- **Dependencies:** —
- **Validation:** `pnpm -F @eddacraft/transactional build` + visual preview via react-email dev
- **Confidence:** high
- **Status:** Complete

#### BAUTH-005: OTP code email template

- **Intent:** Create minimal OTP email — just the 6-digit code
- **Expected Outcome:** React Email component, terminal aesthetic
- **Scope:** `packages/transactional/emails/`
- **Non-scope:** Invite email (separate task)
- **Files:**
  - `packages/transactional/emails/otp-code.tsx`
  - `packages/transactional/emails/index.ts` (re-export)
- **Dependencies:** —
- **Validation:** `pnpm -F @eddacraft/transactional build` + visual preview via react-email dev
- **Confidence:** high
- **Status:** Complete

### Phase C: API Endpoints — Auth Flows

#### BAUTH-006: Device code start endpoint

- **Intent:** Generate user_code + poll_token for a given email
- **Expected Outcome:** `POST /auth/device/start` creates device_codes row, returns codes
- **Scope:** `apps/anvil-api/src/routes/`
- **Non-scope:** Confirmation and polling (separate tasks)
- **Files:**
  - `apps/anvil-api/src/routes/auth-device.ts`
- **Dependencies:** BAUTH-001, BAUTH-002
- **Validation:** `curl POST` returns user_code + poll_token; anti-enumeration verified
- **Confidence:** high
- **Status:** Complete

#### BAUTH-007: Device code confirm endpoint

- **Intent:** Browser submits user_code + email to confirm activation
- **Expected Outcome:** `POST /auth/device/confirm` verifies email matches the
  code's originating email, then marks as confirmed. Max 3 attempts per code.
- **Scope:** `apps/anvil-api/src/routes/`
- **Non-scope:** JWT issuance (happens on poll)
- **Files:**
  - `apps/anvil-api/src/routes/auth-device.ts` (extend)
- **Dependencies:** BAUTH-006
- **Validation:** Confirm requires matching email; mismatched email rejected;
  expired/invalid codes rejected; 3 attempt limit enforced
- **Confidence:** high
- **Status:** Complete

#### BAUTH-008: Device code poll endpoint

- **Intent:** CLI polls until confirmation, then receives JWT pair
- **Expected Outcome:** `POST /auth/device/poll` returns pending/confirmed with JWT on success
- **Scope:** `apps/anvil-api/src/routes/`
- **Non-scope:** CLI polling logic (separate task)
- **Files:**
  - `apps/anvil-api/src/routes/auth-device.ts` (extend)
- **Dependencies:** BAUTH-007, BAUTH-003
- **Validation:** Poll returns `pending` before confirm, JWT pair after; 5s rate enforced
- **Confidence:** high
- **Status:** Complete

#### BAUTH-009: OTP request endpoint

- **Intent:** Send 6-digit OTP code to a beta user's email
- **Expected Outcome:** `POST /auth/otp/request` generates code, stores hash, sends email
- **Scope:** `apps/anvil-api/src/routes/`
- **Non-scope:** OTP verification (separate task)
- **Files:**
  - `apps/anvil-api/src/routes/auth-otp.ts`
- **Dependencies:** BAUTH-001, BAUTH-002, BAUTH-005
- **Validation:** OTP email received; max 3 active codes per user enforced
- **Confidence:** high
- **Status:** Complete

#### BAUTH-010: OTP verify endpoint

- **Intent:** Exchange OTP code for JWT pair
- **Expected Outcome:** `POST /auth/otp/verify` validates code, returns JWT + refresh token
- **Scope:** `apps/anvil-api/src/routes/`
- **Non-scope:** Request endpoint (separate task)
- **Files:**
  - `apps/anvil-api/src/routes/auth-otp.ts` (extend)
- **Dependencies:** BAUTH-009, BAUTH-003
- **Validation:** Valid code → JWT pair; 3 attempt limit; expired code rejected
- **Confidence:** high
- **Status:** Complete

#### BAUTH-011: Session refresh endpoint

- **Intent:** Rotate refresh token and issue new 7-day JWT
- **Expected Outcome:** `POST /auth/session/refresh` with family-based theft detection
- **Scope:** `apps/anvil-api/src/routes/`
- **Non-scope:** CLI refresh logic (separate task)
- **Files:**
  - `apps/anvil-api/src/routes/auth-session.ts`
- **Dependencies:** BAUTH-001, BAUTH-002, BAUTH-003
- **Validation:** Rotation works; consumed token reuse revokes family; expired rejected
- **Confidence:** medium
- **Status:** Complete

### Phase D: Admin Approval

#### BAUTH-012: Admin approve endpoint

- **Intent:** Single endpoint to promote waitlist → beta_users and send invite
- **Expected Outcome:** `POST /admin/approve` with single email and batch modes
- **Scope:** `apps/anvil-api/src/routes/`
- **Non-scope:** CLI command (separate task)
- **Files:**
  - `apps/anvil-api/src/routes/admin.ts` (extend)
- **Dependencies:** BAUTH-004, BAUTH-006
- **Validation:** Waitlist user promoted; token generated; invite email sent
- **Confidence:** high
- **Status:** Complete

#### BAUTH-013: Resend audience management

- **Intent:** Add/remove contacts from waitlist and beta-users audiences
- **Expected Outcome:** Audience ops wired into waitlist signup, approve, and revoke flows
- **Scope:** `apps/anvil-api/src/lib/`, `apps/anvil-api/src/routes/`
- **Non-scope:** Audience creation (manual in Resend console)
- **Files:**
  - `apps/anvil-api/src/lib/audience.ts` (new)
  - `apps/anvil-api/src/routes/waitlist.ts` (add audience on signup)
  - `apps/anvil-api/src/routes/admin.ts` (move audience on approve/revoke)
- **Dependencies:** BAUTH-012
- **Validation:** Contacts appear in correct Resend audiences; failures logged, not blocking
- **Confidence:** medium
- **Status:** Complete

### Phase E: CLI Commands

#### BAUTH-014: `anvil auth login` command

- **Intent:** Interactive CLI activation with device code (default) and OTP (--otp)
- **Expected Outcome:** User can authenticate from terminal; credentials stored locally
- **Scope:** `apps/anvil-cli/src/commands/`
- **Non-scope:** Auto-refresh logic (separate task)
- **Files:**
  - `apps/anvil-cli/src/commands/auth-login.ts`
- **Dependencies:** BAUTH-008, BAUTH-010
- **Validation:** Device code flow works end-to-end; OTP flow works end-to-end
- **Confidence:** medium
- **Status:** Complete

#### BAUTH-015: CLI auto-refresh

- **Intent:** Automatically refresh JWT before expiry on any anvil command
- **Expected Outcome:** Transparent re-auth; prompt `anvil auth login` on failure
- **Scope:** `apps/anvil-cli/src/`
- **Non-scope:** Credential storage format
- **Files:**
  - `apps/anvil-cli/src/lib/auth.ts` (new or extend)
- **Dependencies:** BAUTH-011, BAUTH-014
- **Validation:** Expired JWT triggers background refresh; revoked token prompts re-login
- **Confidence:** medium
- **Status:** Complete

#### BAUTH-016: `anvil admin approve` command

- **Intent:** CLI command for admin to approve waitlist users
- **Expected Outcome:** Single email and batch mode, calls admin approve endpoint
- **Scope:** `apps/anvil-cli/src/commands/`
- **Non-scope:** Dashboard UI
- **Files:**
  - `apps/anvil-cli/src/commands/admin-approve.ts`
- **Dependencies:** BAUTH-012
- **Validation:** `anvil admin approve test@example.com` succeeds; batch mode processes N
- **Confidence:** high
- **Status:** Complete

### Phase F: Website & Integration

#### BAUTH-017: Device code confirmation page

- **Intent:** Static page at `/auth/activate` for entering device codes
- **Expected Outcome:** Terminal-aesthetic page, submits to API, shows success/error
- **Scope:** `apps/website/app/auth/activate/`
- **Non-scope:** Complex UI, progress indicators
- **Files:**
  - `apps/website/app/auth/activate/page.tsx`
- **Dependencies:** BAUTH-007
- **Validation:** Enter code → confirmed; pre-filled via `?code=` query param works
- **Confidence:** high
- **Status:** Complete

#### BAUTH-018: Route wiring and Hono registration

- **Intent:** Register all new route modules in the Hono app
- **Expected Outcome:** New auth-device, auth-otp, auth-session routes mounted
- **Scope:** `apps/anvil-api/src/`
- **Non-scope:** Individual endpoint logic
- **Files:**
  - `apps/anvil-api/src/index.ts` (add route imports + mounts)
- **Dependencies:** BAUTH-006, BAUTH-009, BAUTH-011, BAUTH-012
- **Validation:** `pnpm -F @eddacraft/anvil-api build` passes; health check still works
- **Confidence:** high
- **Status:** Complete

### Phase G: Documentation & Cleanup

#### BAUTH-019: Update API README and env documentation

- **Intent:** Document new endpoints, env vars, and deprecation notes
- **Expected Outcome:** README, as-built doc, and infra config updated
- **Scope:** `apps/anvil-api/`, `docs/`, `infra/`
- **Non-scope:** Runbook updates (already done)
- **Files:**
  - `apps/anvil-api/README.md`
  - `docs/architecture/auth-as-built.md`
  - `infra/src/vercel.ts` (add audience env vars)
- **Dependencies:** BAUTH-013, BAUTH-018
- **Validation:** All env vars documented; README endpoint table complete
- **Confidence:** high
- **Status:** Complete

#### BAUTH-020: Expired code cleanup job

- **Intent:** Periodic purge of expired device codes and OTP codes
- **Expected Outcome:** Vercel Cron or pg_cron removes stale rows
- **Scope:** `apps/anvil-api/src/`
- **Non-scope:** Refresh token cleanup (long-lived, less urgent)
- **Files:**
  - `apps/anvil-api/src/routes/cron.ts` (or DB-side pg_cron)
- **Dependencies:** BAUTH-001
- **Validation:** Expired codes purged; active codes unaffected
- **Confidence:** medium
- **Status:** Complete

## Decisions

**D-BAUTH-001:** Device code over OAuth for CLI activation

- **Rationale:** Simpler than full OAuth; works without client registration;
  familiar from GitHub/Azure CLI
- **Alternatives:** Full OAuth 2.0 PKCE, API key distribution
- **Trade-offs:** Non-standard naming (user_code/poll_token vs RFC 8628)

**D-BAUTH-002:** Email OTP as headless fallback, not primary

- **Rationale:** Device code is more secure (user confirms in browser); OTP
  covers the SSH/headless gap
- **Alternatives:** OTP-only (simpler); device code only (no headless)
- **Trade-offs:** Two flows to maintain, but covers all environments

**D-BAUTH-003:** 7-day JWT with 90-day refresh token

- **Rationale:** Short JWT limits blast radius of leaked tokens; 90-day refresh
  avoids frequent re-authentication; rotation detects theft
- **Alternatives:** 30-day JWT (fewer refreshes); 7-day refresh (more friction)
- **Trade-offs:** More refresh traffic; rotation adds complexity

**D-BAUTH-004:** Thin bridge over session rewrite

- **Rationale:** Existing token model works; beta is temporary; GitHub auth will
  change the model again. Adding activation flows on top is cheaper than
  replacing the foundation.
- **Alternatives:** Full session-based rewrite
- **Trade-offs:** Dual model during transition (old tokens + new refresh tokens)

**D-BAUTH-005:** Best-effort Resend audience management

- **Rationale:** Audience sync is operational convenience, not security-critical.
  Blocking approval on a Resend API failure would be worse than a stale audience.
- **Alternatives:** Strict consistency with retry queue
- **Trade-offs:** Audiences may briefly lag reality

## Notes

**Wave execution plan:**

- **Wave 1** (parallel): BAUTH-001, BAUTH-003, BAUTH-004, BAUTH-005
  (foundation — no interdependencies)
- **Wave 2** (parallel): BAUTH-002, BAUTH-006, BAUTH-009
  (queries + start endpoints — depend on Wave 1)
- **Wave 3** (parallel): BAUTH-007, BAUTH-010, BAUTH-011, BAUTH-012, BAUTH-013
  (remaining API endpoints)
- **Wave 4** (parallel): BAUTH-014, BAUTH-016, BAUTH-017, BAUTH-018
  (CLI + website + wiring)
- **Wave 5** (parallel): BAUTH-015, BAUTH-019, BAUTH-020
  (auto-refresh, docs, cleanup)

**File map for CLAUDE.md `aps-project.md`:**

```text
apps/anvil-api/src/routes/auth-device.ts: BAUTH-006, BAUTH-007, BAUTH-008
apps/anvil-api/src/routes/auth-otp.ts: BAUTH-009, BAUTH-010
apps/anvil-api/src/routes/auth-session.ts: BAUTH-011
apps/anvil-api/src/routes/admin.ts: BAUTH-012, BAUTH-013
apps/anvil-api/src/routes/waitlist.ts: BAUTH-013
apps/anvil-api/src/routes/cron.ts: BAUTH-020
apps/anvil-api/src/db/schema.sql: BAUTH-001
apps/anvil-api/src/db/queries.ts: BAUTH-002
apps/anvil-api/src/lib/licence.ts: BAUTH-003
apps/anvil-api/src/lib/audience.ts: BAUTH-013
apps/anvil-api/src/index.ts: BAUTH-018
apps/anvil-cli/src/commands/auth-login.ts: BAUTH-014
apps/anvil-cli/src/commands/admin-approve.ts: BAUTH-016
apps/anvil-cli/src/lib/auth.ts: BAUTH-015
apps/website/app/auth/activate/page.tsx: BAUTH-017
packages/transactional/emails/beta-invite.tsx: BAUTH-004
packages/transactional/emails/otp-code.tsx: BAUTH-005
apps/anvil-api/README.md: BAUTH-019
docs/architecture/auth-as-built.md: BAUTH-019
infra/src/vercel.ts: BAUTH-019
```
