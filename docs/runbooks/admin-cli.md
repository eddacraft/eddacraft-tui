# Admin CLI Operator Runbook

`anvil-admin` is the operator CLI that wraps Anvil's admin HTTP API
(`/admin/*`). It is the supported way to approve waitlist signups, invite beta
users, revoke tokens, browse the audit log, and send migration emails during
beta.

This runbook covers install, configuration, every command with an example, the
exit-code taxonomy, and troubleshooting.

## Install

`anvil-admin` ships from this monorepo as `@eddacraft/admin-cli`. It is not
published to npm; operators run it from a local checkout.

```bash
git clone git@github.com:eddacraft/anvil-001.git
cd anvil-001
pnpm install
pnpm -F @eddacraft/admin-cli build
```

Then either run via the package script:

```bash
pnpm -F @eddacraft/admin-cli exec anvil-admin --help
```

Or link the binary globally:

```bash
pnpm -F @eddacraft/admin-cli link --global
anvil-admin --help
```

Requires Node.js `>=22.13`.

## Configuration

The CLI reads configuration from environment variables with per-invocation
flag overrides. Every command uses the same resolution order.

| Setting        | Env var              | Flag        | Default                      |
| -------------- | -------------------- | ----------- | ---------------------------- |
| API base URL   | `ANVIL_ADMIN_URL`    | `--url`     | `https://api.eddacraft.ai`   |
| Admin API key  | `ANVIL_ADMIN_KEY`    | `--key`     | *(required — no default)*    |
| Operator ident | `ANVIL_ADMIN_ACTOR`  | `--actor`   | `git config user.email`, else OS username |

- `--key` is sent as `Authorization: Bearer <key>`.
- `--actor` is sent as `X-Admin-Actor: <actor>` and recorded in the audit log.
  Prefer a real email so audit rows are attributable.

Missing key exits `5` (see **Exit codes** below):

```
✖ missing admin key; set ANVIL_ADMIN_KEY or pass --key
```

### Example shell setup

```bash
export ANVIL_ADMIN_URL="https://api.eddacraft.ai"
export ANVIL_ADMIN_KEY="sk_admin_…"       # from 1Password / Pulumi
export ANVIL_ADMIN_ACTOR="you@eddacraft.ai"
```

## Commands

All commands accept `--json` for machine-readable output. Without `--json` they
render human tables with colour (auto-disabled on non-TTY stdout).

### `list` — show waitlist entries

```bash
anvil-admin list                               # pending, all sources, 50 rows
anvil-admin list --status approved --limit 10
anvil-admin list --source website --status all
```

Flags:

- `--status <pending|approved|all>` (default `pending`)
- `--source <manual|website|import|all>` (default `all`)
- `--limit <1-200>` (default `50`)
- `--offset <n>` (default `0`)

### `show <email>` — full profile for one email

Prints the user row, any tokens, and the most recent audit entries.

```bash
anvil-admin show alice@example.com
anvil-admin show alice@example.com --json
```

### `approve [email]` — approve one or a batch

Single approve:

```bash
anvil-admin approve alice@example.com
```

Oldest N pending (prompts for confirmation unless `--yes`):

```bash
anvil-admin approve --batch 10
anvil-admin approve --batch 10 --yes   # skip confirmation
```

Flags:

- `--batch <1-100>` — approve the oldest N unapproved entries (mutually exclusive with `[email]`)
- `-y, --yes` — skip the confirmation prompt
- `--json`

### `invite <email>` — invite to beta

Creates a user (if needed), issues a beta token, and sends the invite email.

```bash
anvil-admin invite alice@example.com --name "Alice Example"
anvil-admin invite alice@example.com --days 30
anvil-admin invite alice@example.com --token-only          # suppress email, print raw token once
anvil-admin invite alice@example.com --scope beta docs     # restrict token scopes
```

Flags:

- `--name <name>` — display name
- `--notes <text>` — internal notes stored on the user row
- `--days <1-365>` (default `90`)
- `--scope <scopes...>` — one or more of the allowed scopes
- `--token-only` — skip the invite email and print the raw token once (you will
  not be able to retrieve it again)
- `--json`

### `revoke [email]` — revoke tokens

Revoke all active tokens for an email, or a specific raw token string.

```bash
anvil-admin revoke alice@example.com            # prompts for confirmation
anvil-admin revoke alice@example.com --yes
anvil-admin revoke --token "betatok_…"          # revoke one specific token
```

Flags:

- `--token <raw>` — revoke a specific raw token (mutually exclusive with `[email]`)
- `-y, --yes` — skip confirmation
- `--json`

### `audit` — browse the audit log

```bash
anvil-admin audit
anvil-admin audit --action user.approved
anvil-admin audit --filter-actor you@eddacraft.ai --limit 20
anvil-admin audit --offset 50
```

Flags:

- `--action <action>` — exact-match filter (e.g. `user.approved`, `token.revoked`)
- `--filter-actor <email>` — filter by operator
- `--limit <1-200>` (default `50`), `--offset <n>` (default `0`)
- `--json`

### `send-migration` — email migration flow

Sends the migration email to waitlist users imported from the previous system.
**Dry-run is the default** — you must opt out with `--no-dry-run` to actually
send.

```bash
anvil-admin send-migration                              # dry-run, source=import, limit=20
anvil-admin send-migration --source website --limit 5   # dry-run, different filter
anvil-admin send-migration --no-dry-run                  # preview → prompt → send (interactive)
anvil-admin send-migration --no-dry-run --yes            # send without prompting (non-interactive)
anvil-admin send-migration --json                        # raw JSON for the dry-run
```

Flags:

- `--source <import|website|manual>` (default `import`)
- `--limit <1-100>` (default `20`)
- `--no-dry-run` — actually send; by default the command only previews
- `-y, --yes` — skip the interactive confirmation when sending. **Required in
  non-TTY sessions** (scripts, CI) when sending for real
- `--json`

Flow when sending for real (`--no-dry-run`):

1. CLI fetches a dry-run preview (count + recipient list) from the server
2. If count is `0`, prints `No recipients match the filter. Nothing to send.`
   on stdout and exits `0`
3. Writes the recipient table plus the warning `About to send migration email
   to N recipient(s) …` to **stderr**, then prompts on stderr:
   `Continue? [y/N]`
4. On `y`/`yes`, calls the server again with `dryRun=false`
5. Renders the per-recipient send/failure table

The preview and the real send are two separate API calls. If rows are added
or removed between them, the sent cohort may differ from the previewed one.
For a migration rollout, snapshot the waitlist (`list --status all --json`)
before starting if you need a stable record.

Non-TTY refusal (`exit 4`) applies only to the **real-send** path. A plain
dry-run works in any session. In non-TTY sessions without `--yes`, the CLI
refuses to prompt and exits `4`.

## Exit codes

| Code | Meaning                                      | Typical cause                              |
| ---- | -------------------------------------------- | ------------------------------------------ |
| `0`  | Success                                      | —                                          |
| `1`  | HTTP 4xx, or `send-migration` had ≥1 failed recipient | Bad request, unauthorised, partial send failure |
| `2`  | HTTP 5xx or malformed JSON response          | Server bug; check logs                     |
| `3`  | Network / cannot reach the API               | DNS, TLS, connection refused               |
| `4`  | Refused to prompt in a non-TTY session       | CI/script without `--yes` on a real send   |
| `5`  | Missing required config                      | `ANVIL_ADMIN_KEY` not set                  |
| `64` | Invalid argument (EX_USAGE)                  | Out-of-range `--limit`, bad enum choice    |

All errors go to stderr; `--json` payloads go to stdout.

## Troubleshooting

### "missing admin key; set ANVIL_ADMIN_KEY or pass --key"

Export `ANVIL_ADMIN_KEY` or pass `--key`. The value is in 1Password under
"Anvil Admin API Key".

### "cannot reach …"

- Check `ANVIL_ADMIN_URL` — default is `https://api.eddacraft.ai`
- Check VPN / network egress
- Retry with `--url https://api.eddacraft.ai` to rule out a bad env var

### "server error 5xx"

The API is the issue, not the CLI. Check the observability dashboard
(`docs/runbooks/observability-triage.md`) and the recent deploys on Vercel.
Rerun once the issue is cleared.

### "refusing to send migration without --yes in a non-interactive session"

You're running in a script or CI job without a TTY. Either run interactively
or pass `--yes` after you've verified the dry-run output.

### Sent the wrong thing

- For a bad approve/invite: use `anvil-admin revoke` to invalidate the token,
  then re-invite
- For a migration email sent to the wrong cohort: there is no unsend — escalate
  in `#beta-ops`

### Looking for what happened

Every admin mutation writes to the audit log. Use `anvil-admin audit` to
review, filtering by `--filter-actor` (who) and `--action` (what). The admin
API also logs request/response metadata in the Vercel logs for 7 days.

## Related

- Admin CLI design spec: `plans/specs/2026-04-16-admin-cli-design.md`
- Admin CLI module plan: `plans/modules/admin-cli.aps.md`
- Waitlist email operations: `docs/runbooks/waitlist-email-operations.md`
- Observability triage: `docs/runbooks/observability-triage.md`
