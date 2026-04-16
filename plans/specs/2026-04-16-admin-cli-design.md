# Admin CLI

**Date:** 2026-04-16
**Branch:** feat/admin
**Module:** ADMINCLI (new)

## Problem

Day-to-day beta admin operations (approving waitlist entries, seeing who is
waiting, checking audit history) require logging into Neon to read the
`waitlist` table for emails, then hand-rolling `curl` against
`apps/anvil-api/src/routes/admin.ts` with the shared `ADMIN_KEY` bearer token.
There is no list endpoint on the admin API and no local tool to browse state.

The archived `archive/anvil-cli-node/` Node CLI covered admin approval but is
no longer maintained and predates the current API shape.

## Goals

1. Remove the Neon detour for the common flow: *see who is pending → approve
   them*.
2. Provide read surfaces for waitlist, user state, and audit history, usable
   from a terminal.
3. Wrap existing admin mutations (`invite`, `approve`, `revoke`,
   `send-migration`) with confirmation prompts, safer defaults, and readable
   output.
4. Stay lightweight — one small TS package in the monorepo, no new runtime
   toolchain, no web UI.

## Non-goals

- Web admin panel or TUI (separate design, later if needed).
- Replacing or wrapping the end-user `anvil` CLI — this is an operator tool
  with a different audit/risk profile.
- Per-operator auth with scoped permissions. The shared `ADMIN_KEY` bearer
  model is retained; the CLI only adds an `X-Admin-Actor` header for audit
  attribution.
- Listing all beta users, token rotation flows, or audit export.

## Design Decisions

### DD-A: CLI lives in `apps/admin-cli/`, TypeScript, monorepo-local

A new `apps/admin-cli/` package, sibling to `apps/anvil-api/`. TypeScript so
we can import the API's Zod request schemas directly and stay in lockstep
with the server. No separate build pipeline for v1 — runs via `tsx` locally;
optional `tsc` build before shipping a bin entry.

**Rationale:** shared Zod schemas prevent request-shape drift; no new
toolchain; easy to pair-edit with the API; zero impact on the end-user Rust
`anvil` binary.

**Rejected:** subcommand on the Rust `anvil` CLI (mixes operator + end-user
concerns, every admin tweak means a kernel rebuild); reviving the archived
Node CLI (fights the lightweight goal, predates current API); shell
scripts + `jq` (no schema safety, painful as surface grows).

### DD-B: New list endpoints on the admin API

Two additions to `apps/anvil-api/src/routes/admin.ts`, both behind
`adminAuth`:

**`GET /admin/waitlist`** — the endpoint that closes the Neon gap.
- Query params:
  - `status`: `pending` | `approved` | `all` (default `pending`)
  - `source`: `manual` | `website` | `import` | `all` (default `all`)
  - `limit`: 1–200 (default 50)
  - `offset`: ≥ 0 (default 0)
- Returns: `{ total: number, items: WaitlistEntry[] }` where
  `WaitlistEntry = { email, name, source, created_at, approved_at | null }`.
- Implementation: new `findWaitlistPaginated(sql, filters)` in
  `apps/anvil-api/src/db/queries.ts`. The existing
  `findUnapprovedWaitlistEntries` stays, called only by batch-approve.

**`GET /admin/audit`**
- Query params:
  - `action`: optional exact match (e.g. `user.approved`)
  - `actor`: optional exact match
  - `limit`: 1–200 (default 50)
  - `offset`: ≥ 0 (default 0)
- Returns: `{ total: number, items: AuditEntry[] }` where
  `AuditEntry = { id, action, actor, metadata, created_at }`. `total` is
  the filtered count (so the CLI can show "showing N of M").
- Order: `created_at DESC`. Verify index exists; add one in a new migration
  if missing.

### DD-C: Extend `GET /admin/user/:email` with recent audit

Include up to 10 most recent audit entries whose `metadata->>'email'` equals
the looked-up email. Removes a second request in the common "who is this
person and what happened to them" flow.

Response addition: `recentAudit: AuditEntry[]` alongside existing `user` and
`tokens`.

### DD-D: Deliberately not adding `GET /admin/users`

Listing all beta users is not a stated need. `GET /admin/user/:email`
already covers lookup. YAGNI — add later in ~20 lines if a real use case
appears.

### DD-E: Schema sharing via a split-out module

Extract the existing Zod request schemas from `admin.ts` into a new
`apps/anvil-api/src/routes/admin-schemas.ts`. `admin.ts` imports from it for
`zValidator`; `apps/admin-cli` imports from it for client-side request
validation. This is the structural payoff of keeping the CLI in the
monorepo — no separate types package, no drift.

### DD-F: Commands and flags

```
anvil-admin list [--status pending|approved|all]
                 [--source manual|website|import|all]
                 [--limit N] [--offset N] [--json]

anvil-admin show <email> [--json]

anvil-admin approve <email> [--yes]
anvil-admin approve --batch <N> [--yes]

anvil-admin invite <email>
                   [--name NAME] [--notes TEXT]
                   [--days 90] [--scope beta,preview,internal]
                   [--token-only]

anvil-admin revoke <email> [--yes]
anvil-admin revoke --token <raw-token> [--yes]

anvil-admin audit [--action ...] [--actor ...]
                  [--limit 50] [--offset N] [--json]

anvil-admin send-migration [--source import|website|manual]
                           [--limit N] [--dry-run] [--yes]
```

`list` is the headline command — the one that removes the Neon detour.
Default invocation `anvil-admin list` shows pending waitlist entries.

### DD-G: Config resolution (first hit wins)

1. CLI flags: `--key`, `--url`.
2. Env: `ANVIL_ADMIN_KEY`, `ANVIL_ADMIN_URL` (default
   `https://api.eddacraft.ai`).

v1 is env-only. The `~/.anvil/admin.json` dotfile + `login`/`logout`
helpers are deferred — add if operators ask for them (~20 lines, file
mode `0600`).

### DD-H: Actor identity for audit

The CLI sends `X-Admin-Actor` on every request. Resolution order:
1. `ANVIL_ADMIN_ACTOR` env var.
2. `git config user.email` (executed once, cached for the process).
3. `os.userInfo().username`.

The API already sanitises this header (`admin.ts` line 56). Net effect: the
audit log stops recording `"actor": "admin"` for every action and starts
attributing to the real operator.

### DD-I: Confirmations gated by risk

- **No prompt:** `list`, `show`, `audit`.
- **Summary prompt (y/N):** `approve <email>`, `approve --batch N`,
  `send-migration` without `--dry-run`.
- **Strong prompt:** `revoke <email>` and `revoke --token` show the
  affected scope and require literal typing of `revoke` to proceed.
- `invite --token-only` prints a one-time banner explaining the raw token
  will never be shown again. The token is never written to logs or history.

All prompting commands accept `--yes` to skip; non-TTY invocations must
pass `--yes` or exit with code 4.

Uses `@clack/prompts` for interactive bits; non-TTY runs require `--yes` on
any prompting command or exit with code 4 ("refusing to prompt without a
TTY").

### DD-J: Output format

- Default: compact ANSI tables via a small homegrown padder (no
  `cli-table3`-class dep). Colours via `picocolors` or inline ANSI.
- `--json`: raw JSON passthrough from the API response, newline-terminated,
  for scripting.
- `--quiet`: errors only, for CI.
- Auto-plain when stdout is not a TTY (no colours, no box-drawing).

### DD-K: Error handling and exit codes

| Condition                     | Exit | stderr                                      |
|-------------------------------|------|---------------------------------------------|
| 4xx from API                  | 1    | `error.error` from response body            |
| 5xx from API                  | 2    | Status + truncated body                     |
| Network / DNS / TLS failure   | 3    | "cannot reach <url>: <cause>"               |
| Refusing to prompt (non-TTY)  | 4    | "pass --yes to confirm without a TTY"       |
| Missing config (no key/url)   | 5    | "missing admin key; set ANVIL_ADMIN_KEY"    |
| Invalid arguments             | 64   | commander's default usage output            |

### DD-L: Testing

- **API:** extend `apps/anvil-api/src/__tests__/admin.test.ts` with cases
  for the two new endpoints and the extended `user/:email` response. Cover
  filter combinations, pagination bounds, and authz.
- **CLI:** unit-test `client.ts` (mocked `fetch`), `config.ts` resolution
  order, and `format.ts` table output. One happy-path smoke test per
  command with a mocked client. No full e2e — the API tests cover the wire
  contract.

## Out of scope for v1

- Interactive picker (select-from-list → approve). `list` → copy email →
  `approve <email>` is the v1 flow; a multiselect picker is a ~30-line
  follow-up using `@clack/prompts`.
- Local caching of responses.
- Shell completions.
- `GET /admin/users` list endpoint (see DD-D).
- Per-operator auth / token-per-operator (future auth design).

## File-by-file scope

**New:**
- `apps/admin-cli/package.json`
- `apps/admin-cli/tsconfig.json`
- `apps/admin-cli/src/index.ts` — commander entry
- `apps/admin-cli/src/client.ts` — typed fetch wrapper
- `apps/admin-cli/src/config.ts` — env + dotfile resolution
- `apps/admin-cli/src/format.ts` — table / JSON helpers
- `apps/admin-cli/src/commands/{list,show,approve,invite,revoke,audit,send-migration}.ts`
- `apps/admin-cli/src/__tests__/*.test.ts`
- `apps/anvil-api/src/routes/admin-schemas.ts` — extracted Zod schemas
- `docs/runbooks/admin-cli.md` — operator-facing usage doc

**Modified:**
- `apps/anvil-api/src/routes/admin.ts` — add `GET /waitlist`, `GET /audit`,
  extend `GET /user/:email`; import schemas from the split-out module.
- `apps/anvil-api/src/db/queries.ts` — add `findWaitlistPaginated`,
  `findAuditEntries`, and a user-scoped `findRecentAuditForEmail`.
- `apps/anvil-api/src/__tests__/admin.test.ts` — new endpoint coverage.
- Root `package.json` — add `anvil-admin` bin mapping; add `pnpm admin`
  script (`tsx apps/admin-cli/src/index.ts`).
- Possibly a new migration under `apps/anvil-api/db/migrations/` to add
  `audit_log(created_at DESC)` index if missing.

## Rough size

| Piece                                          | Lines (approx) |
|------------------------------------------------|----------------|
| API list + audit endpoints, schema split, tests | ~200           |
| CLI package scaffold, 7 commands, helpers      | ~500           |
| Runbook doc                                    | ~50            |
| **Total**                                      | **~750**       |

## Success criteria

1. `anvil-admin list` shows pending waitlist entries without touching Neon.
2. `anvil-admin show <email>` returns user + tokens + last 10 audit entries
   in one call.
3. `anvil-admin approve <email>` approves and sends the invite email, with
   an audit entry attributed to the real operator (not `"admin"`).
4. All existing admin operations (`invite`, `revoke`, `send-migration`) are
   available from the CLI with safer defaults than raw curl.
5. API and CLI tests pass; one runbook page documents the v1 surface.
