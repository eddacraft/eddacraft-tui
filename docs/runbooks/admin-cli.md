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

The CLI reads configuration from environment variables with per-invocation flag
overrides. Every command uses the same resolution order.

| Setting        | Env var             | Flag      | Default                                   |
| -------------- | ------------------- | --------- | ----------------------------------------- |
| API base URL   | `ANVIL_ADMIN_URL`   | `--url`   | `https://api.eddacraft.ai`                |
| Admin API key  | `ANVIL_ADMIN_KEY`   | `--key`   | _(required — no default)_                 |
| Operator ident | `ANVIL_ADMIN_ACTOR` | `--actor` | `git config user.email`, else OS username |

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

### Local key storage — guidance

- **Do not `export ANVIL_ADMIN_KEY=…` inline** in a terminal session: the key
  lands in `.zsh_history` / `.bash_history` and in `ps`-visible env for every
  child process. If you've done this, rotate the key and scrub history.
- Prefer a secrets-manager-backed loader: e.g. `op` (1Password CLI) shells that
  inject the key only for the admin-cli subshell:
  ```bash
  op run --env-file=admin-cli.env -- anvil-admin list
  ```
  where `admin-cli.env` references 1Password items (`ANVIL_ADMIN_KEY="op://…"`).
- Alternatively, source a private `~/.anvil-admin.env` file (mode `0600`,
  outside the repo) and `unset ANVIL_ADMIN_KEY` when done. Never commit this
  file.
- For CI, inject via the platform's secret store; never echo the key in logs.

## Per-operator admin keys

The admin surface supports two authentication paths during the dual-auth
rollout:

1. **Shared `ADMIN_KEY`** — a single high-entropy key configured via Vercel
   env. Every request authenticated this way is attributed to the sentinel
   actor `shared-key@anvil` and marked `auth_method: "shared"` in the audit
   log. The `X-Admin-Actor` header is **ignored** on this path.
2. **Per-operator keys** — rows in the `admin_keys` table keyed on a peppered
   hash of the raw bearer. Each row maps to a real `actor_email`. Audit rows
   are stamped with that email and `auth_method: "per_operator"`.

The dual path is gated on the `ADMIN_PER_OPERATOR_KEYS` server env var (set
to `1` or `true` to enable the lookup). When enabled, the middleware tries
the per-operator path first; on hash miss or DB error it falls back to
shared-key comparison so a DB hiccup does not take down the admin surface.

### Provisioning a per-operator key

v1 provisions keys via reviewed IaC (Pulumi). A follow-up PR wires that up;
until then, provision manually via a reviewed SQL migration or psql session
executed by two operators:

```sql
-- 1. Generate a 32-byte random bearer out-of-band (e.g. `openssl rand -hex 32`).
-- 2. Compute hashed_key = HMAC-SHA-256(ADMIN_KEY_PEPPER, bearer) and insert:
INSERT INTO admin_keys (hashed_key, actor_email, note)
VALUES ('<hex-digest>', 'alice@eddacraft.ai', 'onboard 2026-04');

-- 3. Record the provisioning event:
INSERT INTO admin_keys_audit
  (admin_key_id, action, change_actor, pulumi_commit_sha, note)
VALUES
  ((SELECT id FROM admin_keys WHERE hashed_key = '<hex-digest>'),
   'created',
   'ops-pair@eddacraft.ai',
   '<git-sha-of-the-migration-PR>',
   'manual provision pre-IaC');
```

Distribute the plaintext bearer to the operator **out-of-band** (1Password
shared vault). Never store the plaintext in git, Slack, email, or ticket
systems. The server only ever sees the hash.

`ADMIN_KEY_PEPPER` is a server-side secret. It is separate from
`TOKEN_PEPPER` (used for access-token hashing) so rotation of either does
not invalidate the other.

### Revoking a per-operator key

```sql
UPDATE admin_keys SET revoked_at = now() WHERE actor_email = 'alice@eddacraft.ai'
  AND revoked_at IS NULL;

INSERT INTO admin_keys_audit (admin_key_id, action, change_actor, pulumi_commit_sha, note)
SELECT id, 'revoked', 'ops-pair@eddacraft.ai', '<git-sha>', 'offboarding'
FROM admin_keys WHERE actor_email = 'alice@eddacraft.ai' AND revoked_at IS NOT NULL;
```

A revoked key that is presented authenticates with **401
`admin_key_revoked`** and writes an audit row with
`outcome: "rejected_revoked"`. Unknown or malformed bearers return **401**
with `outcome: "rejected_unknown"` / `"rejected_malformed"`. The raw bearer
is never logged — only its peppered hash.

### Rotating the shared `ADMIN_KEY`

The shared key is the break-glass path and should be rotated on operator
offboarding, suspected exposure, or at least once per quarter:

1. Generate a new random key (≥256 bits): `openssl rand -hex 32`.
2. Update the Vercel env `ADMIN_KEY` for both preview and production.
3. Redeploy — the old key stops working as soon as the new deploy is live.
4. Distribute the new key to active operators via 1Password. Remove the old
   item from the vault.
5. Confirm `anvil-admin list` works with the new key.

The final cutover (removal of the shared-key path) happens only after the
shared-key request rate is zero for ≥7 consecutive days, tracked via the
`auth_method` audit column.

### Rollout tracking

Every mutation writes `auth_method` on the audit row. To monitor adoption:

```sql
SELECT auth_method, COUNT(*)
FROM audit_log
WHERE occurred_at > now() - interval '7 days'
GROUP BY auth_method;
```

When the `shared` count holds at zero for seven consecutive days, schedule
the cutover PR that removes the shared-key branch.

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

- `--batch <1-100>` — approve the oldest N unapproved entries (mutually
  exclusive with `[email]`)
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

- `--token <raw>` — revoke a specific raw token (mutually exclusive with
  `[email]`)
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

- `--action <action>` — exact-match filter (e.g. `user.approved`,
  `token.revoked`)
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
anvil-admin send-migration --no-dry-run --yes            # preview (silent) → send (non-interactive)
anvil-admin send-migration --json                        # raw JSON for the dry-run (includes previewToken)
```

A dry-run prints a preview token and an `expiresAt` stamp; you must re-run
without `--dry-run` within 10 minutes for the send to consume that exact
recipient snapshot. The CLI always re-fetches a fresh token when you run with
`--no-dry-run`, so operators don't need to carry the token manually. If you run
multiple previews without sending, always use the **most recently printed**
token on the real-send — earlier tokens remain valid (and bound to you) until
they age out, which can cause confusion if you paste the wrong one.

Flags:

- `--source <import|website|manual>` (default `import`)
- `--limit <1-100>` (default `20`)
- `--no-dry-run` — actually send; by default the command only previews
- `-y, --yes` — skip the interactive confirmation when sending. **Required in
  non-TTY sessions** (scripts, CI) when sending for real
- `--json`

Flow when sending for real (`--no-dry-run`):

1. CLI calls the server with `dryRun=true`. The server records a **recipient
   snapshot** keyed by a single-use **preview token** (10-minute TTL, bound to
   the calling operator) and returns the recipient list plus the token.
2. If count is `0`, prints `No recipients match the filter. Nothing to send.` on
   stdout and exits `0`.
3. Writes the recipient table plus the warning
   `About to send migration email to N recipient(s) …` to **stderr**, then
   prompts on stderr: `Continue? [y/N]`. With `--yes`, the prompt is skipped but
   the dry-run still runs — the token is always fetched.
4. On `y`/`yes`, calls the server with `dryRun=false` and the preview token. The
   server atomically consumes the token, compares the snapshotted recipients
   against a fresh cohort query, and either sends (to the **snapshotted** set,
   not a re-queried set) or rejects with a specific error code.
5. Renders the per-recipient send/failure table.

This snapshot-plus-token flow is the defence against cohort drift: between
preview and send, the waitlist may change (new signups, deletions, source
re-tags), and the operator's intent is "send to the exact set I just saw" — not
"send to whoever matches the filter at the moment of send".

#### Real-send error recovery

The real-send request can fail with distinct, actionable codes. The CLI surfaces
these with recovery-specific messages; the runbook equivalents:

| HTTP | `code`                   | What it means                                                              | Recovery                                                                                           |
| ---- | ------------------------ | -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| 409  | `cohort_drift`           | Recipients changed since the preview. Response includes `added`, `removed` | Re-run `anvil-admin send-migration --no-dry-run` — the CLI will fetch a fresh snapshot first       |
| 410  | `preview_token_expired`  | The 10-minute TTL elapsed                                                  | Re-run `anvil-admin send-migration --no-dry-run` (within 10 minutes next time)                     |
| 410  | `preview_token_consumed` | A prior send call already used this token                                  | **Verify the previous send completed** in the audit log, then decide whether to re-send            |
| 410  | `preview_token_missing`  | Token not found, reaped, or owned by a different operator                  | Re-run `anvil-admin send-migration --no-dry-run` under the correct `ANVIL_ADMIN_ACTOR` / `--actor` |
| 400  | `preview_token_required` | Should not occur via the CLI; indicates a client bug                       | File a ticket; workaround is to re-run the CLI (it always fetches a token first)                   |

For a `preview_token_consumed` recovery, confirm before re-sending:

```bash
anvil-admin audit --action migration.email.sent --limit 5
```

Look for an entry matching the source, count, and preview token you expect. If
the previous send completed successfully, do **not** re-run. If it partially
sent and was interrupted, coordinate in `#beta-ops` before re-running — the
second run will re-email every recipient in the snapshot.

Non-TTY refusal (`exit 4`) applies only to the **real-send** path. A plain
dry-run works in any session. In non-TTY sessions without `--yes`, the CLI
refuses to prompt and exits `4`.

## Exit codes

| Code | Meaning                                               | Typical cause                                   |
| ---- | ----------------------------------------------------- | ----------------------------------------------- |
| `0`  | Success                                               | —                                               |
| `1`  | HTTP 4xx, or `send-migration` had ≥1 failed recipient | Bad request, unauthorised, partial send failure |
| `2`  | HTTP 5xx or malformed JSON response                   | Server bug; check logs                          |
| `3`  | Network / cannot reach the API                        | DNS, TLS, connection refused                    |
| `4`  | Refused to prompt in a non-TTY session                | CI/script without `--yes` on a real send        |
| `5`  | Missing required config                               | `ANVIL_ADMIN_KEY` not set                       |
| `64` | Invalid argument (EX_USAGE)                           | Out-of-range `--limit`, bad enum choice         |

All errors go to stderr; `--json` payloads go to stdout.

## Troubleshooting

### "missing admin key; set ANVIL_ADMIN_KEY or pass --key"

Export `ANVIL_ADMIN_KEY` or pass `--key`. The value is in 1Password under "Anvil
Admin API Key".

### "cannot reach …"

- Check `ANVIL_ADMIN_URL` — default is `https://api.eddacraft.ai`
- Check VPN / network egress
- Retry with `--url https://api.eddacraft.ai` to rule out a bad env var

### "server error 5xx"

The API is the issue, not the CLI. Check the observability dashboard
(`docs/runbooks/observability-triage.md`) and the recent deploys on Vercel.
Rerun once the issue is cleared.

### "refusing to send migration without --yes in a non-interactive session"

You're running in a script or CI job without a TTY. Either run interactively or
pass `--yes` after you've verified the dry-run output.

### Sent the wrong thing

- For a bad approve/invite: use `anvil-admin revoke` to invalidate the token,
  then re-invite
- For a migration email sent to the wrong cohort: there is no unsend — escalate
  in `#beta-ops`

### Looking for what happened

Every admin mutation writes to the audit log. Use `anvil-admin audit` to review,
filtering by `--filter-actor` (who) and `--action` (what). The admin API also
logs request/response metadata in the Vercel logs for 7 days.

## Related

- Admin CLI design spec: `plans/specs/2026-04-16-admin-cli-design.md`
- Admin CLI module plan (archived): `plans/archive/modules/admin-cli.aps.md`
- Admin CLI hardening plan: `plans/modules/admin-cli-hardening.aps.md`
- Waitlist email operations: `docs/runbooks/waitlist-email-operations.md`
- Observability triage: `docs/runbooks/observability-triage.md`
