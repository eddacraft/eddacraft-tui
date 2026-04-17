<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Admin CLI

| Scope    | Owner  | Priority | Status      |
| -------- | ------ | -------- | ----------- |
| ADMINCLI | @aneki | high     | In Progress |

## Purpose

Add a lightweight TypeScript operator CLI that removes the Neon detour for
beta admin operations and wraps the existing admin API with readable output
and safer defaults.

**Problem:** Approving waitlist entries today requires logging into Neon to
copy an email, then hand-rolling `curl` against
`apps/anvil-api/src/routes/admin.ts`. There is no list endpoint, no browsing
surface, and no operator attribution on audit entries.

**Solution:** Two new list endpoints (`GET /admin/waitlist`,
`GET /admin/audit`), an extended user lookup that returns recent audit
context, and a small `apps/admin-cli` package that consumes them. Shared
Zod request schemas keep the client and server in lockstep.

**Design Spec:** `plans/specs/2026-04-16-admin-cli-design.md`

## In Scope

**API additions:**

- `GET /admin/waitlist` with `status` / `source` / `limit` / `offset` filters
- `GET /admin/audit` with `action` / `actor` / `limit` / `offset` filters
- `GET /admin/user/:email` response extended with `recentAudit` (up to 10)
- Zod request schemas split to a shared module importable by the CLI
- Migration adding a `created_at DESC` index on `audit_log` if absent

**CLI package (`apps/admin-cli`):**

- `list`, `show`, `approve`, `invite`, `revoke`, `audit`, `send-migration`
- Config from CLI flags, env (`ANVIL_ADMIN_KEY`, `ANVIL_ADMIN_URL`)
- `X-Admin-Actor` header populated from env / git / OS user
- Risk-gated confirmations (summary prompt or strong "type `revoke`" prompt)
- Table output by default, `--json` for scripting, `--quiet` for CI
- Consistent exit codes (4xx → 1, 5xx → 2, network → 3, non-TTY → 4, …)

**Docs:**

- `docs/runbooks/admin-cli.md` covering install, config, and common flows

## Out of Scope (v1)

- Web admin panel or TUI — later design if needed
- `~/.anvil/admin.json` dotfile + `login` / `logout` helpers (env-only v1)
- `GET /admin/users` list endpoint (no current use case)
- Interactive picker (select-from-list → approve)
- Per-operator auth, scoped tokens, shell completions, response caching

## Interfaces

**Depends on:**

- Existing `beta_users`, `waitlist`, `access_tokens`, `audit_log` tables
- Existing `adminAuth` middleware in `apps/anvil-api/src/middleware/`
- Existing `ADMIN_KEY` bearer model
- Existing `findUnapprovedWaitlistEntries`, `findUserWithTokens` query helpers

**Exposes:**

- `GET /admin/waitlist`, `GET /admin/audit`
- Extended `GET /admin/user/:email` response shape
- `anvil-admin` bin (monorepo-local: `pnpm admin …` or `pnpm exec anvil-admin`)

## Boundary Rules

- Shared `ADMIN_KEY` stays authoritative — the CLI does not introduce a new
  auth model
- The CLI never persists raw tokens; `invite --token-only` prints a one-time
  banner and exits
- Non-TTY invocations of prompting commands must pass `--yes` or exit 4 —
  never silently proceed
- `send-migration` defaults remain best-effort: email failures must not
  cascade into audit inconsistency
- Schemas are imported, not duplicated — if a client-side schema diverges
  from the server schema, the PR is rejected

## Acceptance Criteria

- [ ] `anvil-admin list` returns pending waitlist entries without touching Neon
- [ ] `anvil-admin show <email>` returns user, tokens, and last 10 audit
      entries in a single round-trip
- [ ] `anvil-admin approve <email>` approves, sends the invite email, and
      writes an audit entry attributed to the real operator (not `"admin"`)
- [ ] `invite`, `revoke`, `send-migration` available from the CLI with
      risk-gated confirmations
- [ ] `--json` output is raw API passthrough suitable for piping to `jq`
- [ ] Non-TTY execution of prompting commands without `--yes` exits non-zero
- [ ] API tests cover the two new endpoints and the extended user lookup
- [ ] Operator runbook published at `docs/runbooks/admin-cli.md`

## Risks & Mitigations

| Risk                                      | Mitigation                                                       |
| ----------------------------------------- | ---------------------------------------------------------------- |
| Schema drift between CLI and API          | Shared `admin-schemas.ts` module; both sides import from it      |
| `audit_log` grows large → slow `GET /audit` | Pagination is required on the endpoint; add index if missing    |
| Operator pastes `ADMIN_KEY` into shell history | Docs emphasise env var / dotenv; `--key` is for CI only     |
| Missed confirmation prompt in scripts     | Non-TTY + no `--yes` exits 4 instead of proceeding               |
| `--json` leaks internal fields            | Response shapes are whitelisted in the route, not raw DB rows    |

## Tasks

### Phase A: API foundation

#### ADMINCLI-001: Extract admin request schemas to shared module

- **Intent:** Move Zod request schemas out of the route file so the CLI can import them without pulling in Hono
- **Expected Outcome:** `apps/anvil-api/src/routes/admin-schemas.ts` exports all admin request schemas; `admin.ts` re-imports them; behaviour unchanged
- **Scope:** `apps/anvil-api/src/routes/`
- **Non-scope:** New endpoints, CLI package
- **Files:**
  - `apps/anvil-api/src/routes/admin-schemas.ts` (new)
  - `apps/anvil-api/src/routes/admin.ts`
- **Dependencies:** —
- **Validation:** Existing admin tests pass unchanged
- **Confidence:** high
- **Status:** Complete

#### ADMINCLI-002: Waitlist list endpoint

- **Intent:** Expose paginated, filterable waitlist listing behind `adminAuth`
- **Expected Outcome:** `GET /admin/waitlist` honours `status`, `source`, `limit`, `offset`; returns `{ total, items }`; covered by tests
- **Scope:** `apps/anvil-api/src/routes/`, `apps/anvil-api/src/db/`, `apps/anvil-api/src/__tests__/`
- **Non-scope:** CLI consumption
- **Files:**
  - `apps/anvil-api/src/db/queries.ts` (add `findWaitlistPaginated`)
  - `apps/anvil-api/src/routes/admin.ts`
  - `apps/anvil-api/src/routes/admin-schemas.ts`
  - `apps/anvil-api/src/__tests__/admin.test.ts`
- **Dependencies:** ADMINCLI-001
- **Validation:** `pnpm -F @eddacraft/anvil-api test -- --testNamePattern="waitlist list"`
- **Confidence:** high
- **Status:** Complete

#### ADMINCLI-003: Audit list endpoint

- **Intent:** Expose paginated, filterable audit log behind `adminAuth`
- **Expected Outcome:** `GET /admin/audit` honours `action`, `actor`, `limit`, `offset`; returns `{ total, items }` ordered `created_at DESC`; index verified or added
- **Scope:** `apps/anvil-api/src/routes/`, `apps/anvil-api/src/db/`, `apps/anvil-api/db/migrations/`
- **Non-scope:** User-scoped audit (see ADMINCLI-004)
- **Files:**
  - `apps/anvil-api/src/db/queries.ts` (add `findAuditEntries`)
  - `apps/anvil-api/src/routes/admin.ts`
  - `apps/anvil-api/src/routes/admin-schemas.ts`
  - `apps/anvil-api/src/__tests__/admin.test.ts`
  - New migration file if index missing
- **Dependencies:** ADMINCLI-001
- **Validation:** `pnpm -F @eddacraft/anvil-api test -- --testNamePattern="audit list"`
- **Confidence:** high
- **Status:** Complete

#### ADMINCLI-004: Extend user lookup with recent audit

- **Intent:** Return up to 10 recent audit entries for the looked-up email alongside the existing user + tokens
- **Expected Outcome:** `GET /admin/user/:email` includes `recentAudit` field; existing callers unaffected
- **Scope:** `apps/anvil-api/src/routes/`, `apps/anvil-api/src/db/`
- **Non-scope:** Pagination on recentAudit (fixed cap of 10)
- **Files:**
  - `apps/anvil-api/src/db/queries.ts` (add `findRecentAuditForEmail`)
  - `apps/anvil-api/src/routes/admin.ts`
  - `apps/anvil-api/src/__tests__/admin.test.ts`
- **Dependencies:** ADMINCLI-003
- **Validation:** `pnpm -F @eddacraft/anvil-api test -- --testNamePattern="user lookup"`
- **Confidence:** high
- **Status:** Complete

### Phase B: CLI foundation

#### ADMINCLI-005: Scaffold admin-cli package

- **Intent:** Stand up `apps/admin-cli` with bin entry, shared client, config resolution, and output helpers
- **Expected Outcome:** `pnpm exec anvil-admin --help` prints the command list; `client.ts`, `config.ts`, `format.ts` exist with unit tests
- **Scope:** `apps/admin-cli/`
- **Non-scope:** Individual commands (separate tasks)
- **Files:**
  - `apps/admin-cli/package.json`
  - `apps/admin-cli/tsconfig.json`
  - `apps/admin-cli/src/index.ts`
  - `apps/admin-cli/src/client.ts`
  - `apps/admin-cli/src/config.ts`
  - `apps/admin-cli/src/format.ts`
  - `apps/admin-cli/src/__tests__/{client,config,format}.test.ts`
  - Root `package.json` (bin + `admin` script)
- **Dependencies:** ADMINCLI-001
- **Validation:** `pnpm admin --help` exits 0 and lists all seven subcommands
- **Confidence:** high
- **Status:** Complete

#### ADMINCLI-006: `list` command

- **Intent:** Surface waitlist entries in the terminal
- **Expected Outcome:** `anvil-admin list` lists pending entries by default; filters and `--json` work; table output on TTY, plain on pipe
- **Scope:** `apps/admin-cli/src/commands/`
- **Non-scope:** Interactive picker
- **Files:**
  - `apps/admin-cli/src/commands/list.ts`
  - `apps/admin-cli/src/__tests__/list.test.ts`
- **Dependencies:** ADMINCLI-002, ADMINCLI-005
- **Validation:** `pnpm -F @eddacraft/admin-cli test -- list`
- **Confidence:** high
- **Status:** Complete

#### ADMINCLI-007: `show` command

- **Intent:** Display user, tokens, and recent audit for a given email
- **Expected Outcome:** `anvil-admin show <email>` renders a grouped view; `--json` passes through the extended response
- **Scope:** `apps/admin-cli/src/commands/`
- **Non-scope:** Filtering the audit entries (cap is 10)
- **Files:**
  - `apps/admin-cli/src/commands/show.ts`
  - `apps/admin-cli/src/__tests__/show.test.ts`
- **Dependencies:** ADMINCLI-004, ADMINCLI-005
- **Validation:** `pnpm -F @eddacraft/admin-cli test -- show`
- **Confidence:** high
- **Status:** Complete

#### ADMINCLI-008: `approve` command

- **Intent:** Approve a single email or the oldest N pending, with summary confirmation
- **Expected Outcome:** `anvil-admin approve <email>` and `approve --batch N` confirm then POST; `--yes` skips; non-TTY without `--yes` exits 4
- **Scope:** `apps/admin-cli/src/commands/`
- **Non-scope:** Bulk retry / partial-failure reporting beyond what the API returns
- **Files:**
  - `apps/admin-cli/src/commands/approve.ts`
  - `apps/admin-cli/src/__tests__/approve.test.ts`
- **Dependencies:** ADMINCLI-005
- **Validation:** `pnpm -F @eddacraft/admin-cli test -- approve`
- **Confidence:** high
- **Status:** Complete

#### ADMINCLI-009: `invite` command

- **Intent:** Wrap `POST /admin/invite` including `--token-only`
- **Expected Outcome:** `anvil-admin invite <email>` accepts name, notes, days, scopes; `--token-only` prints a one-time banner and the raw token; token never written to logs or shell history
- **Scope:** `apps/admin-cli/src/commands/`
- **Non-scope:** Token rotation flows
- **Files:**
  - `apps/admin-cli/src/commands/invite.ts`
  - `apps/admin-cli/src/__tests__/invite.test.ts`
- **Dependencies:** ADMINCLI-005
- **Validation:** `pnpm -F @eddacraft/admin-cli test -- invite`
- **Confidence:** medium
- **Status:** Complete

#### ADMINCLI-010: `revoke` command

- **Intent:** Wrap `POST /admin/revoke` with a strong confirmation prompt
- **Expected Outcome:** `anvil-admin revoke <email>` and `revoke --token <raw>` show the affected scope and require typing `revoke`; `--yes` skips; non-TTY without `--yes` exits 4
- **Scope:** `apps/admin-cli/src/commands/`
- **Non-scope:** Partial revocation (e.g. single scope)
- **Files:**
  - `apps/admin-cli/src/commands/revoke.ts`
  - `apps/admin-cli/src/__tests__/revoke.test.ts`
- **Dependencies:** ADMINCLI-005
- **Validation:** `pnpm -F @eddacraft/admin-cli test -- revoke`
- **Confidence:** high
- **Status:** Complete

#### ADMINCLI-011: `audit` command

- **Intent:** Browse the audit log from the terminal
- **Expected Outcome:** `anvil-admin audit` shows the most recent entries; `--action` / `--actor` filters work; `--json` passes through
- **Scope:** `apps/admin-cli/src/commands/`
- **Non-scope:** Live tail / streaming
- **Files:**
  - `apps/admin-cli/src/commands/audit.ts`
  - `apps/admin-cli/src/__tests__/audit.test.ts`
- **Dependencies:** ADMINCLI-003, ADMINCLI-005
- **Validation:** `pnpm -F @eddacraft/admin-cli test -- audit`
- **Confidence:** high
- **Status:** Complete

#### ADMINCLI-012: `send-migration` command

- **Intent:** Wrap `POST /admin/send-migration` with `--dry-run` preview and summary confirmation
- **Expected Outcome:** `anvil-admin send-migration` defaults to `--dry-run`; without it, confirms count then sends; per-recipient results rendered
- **Scope:** `apps/admin-cli/src/commands/`
- **Non-scope:** Template editing
- **Files:**
  - `apps/admin-cli/src/commands/send-migration.ts`
  - `apps/admin-cli/src/__tests__/send-migration.test.ts`
- **Dependencies:** ADMINCLI-005
- **Validation:** `pnpm -F @eddacraft/admin-cli test -- send-migration`
- **Confidence:** medium
- **Status:** Ready

### Phase C: Polish

#### ADMINCLI-013: Operator runbook

- **Intent:** Document install, config, and the common flows so another operator can pick this up cold
- **Expected Outcome:** `docs/runbooks/admin-cli.md` covers env setup, each command with an example, exit codes, and troubleshooting; linked from the existing admin docs
- **Scope:** `docs/runbooks/`
- **Non-scope:** Architecture narrative (that lives in the design spec)
- **Files:**
  - `docs/runbooks/admin-cli.md`
- **Dependencies:** ADMINCLI-006 … ADMINCLI-012
- **Validation:** Peer read-through; linkcheck passes
- **Confidence:** high
- **Status:** Ready
