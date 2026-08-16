# Admin CLI Operator Runbook

| Type    | Authority     | Owner                | Status | Freshness                                                                                                                                                                                                                                                          |
| ------- | ------------- | -------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Runbook | Authoritative | CIB, FLEET-007, BACT | Live   | Last reviewed 2026-08-16: CIB-339/340 intake (Git Bash path-shape, entropy mixed-case) appends to the module tail and does not touch CIB-004 (admin-key credential-source); prior 2026-08-15 CIB-336..338, 2026-08-14 pack-06, 2026-08-13 BACT-003/006/007/009/011 |

| Upstream                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Downstream                                                                                   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `crates/anvil-cli/src/commands/admin.rs`, `apps/anvil-api/src/middleware/admin-auth.ts`, `apps/anvil-api/src/lib/fleet-overview.ts`, `apps/anvil-api/src/lib/account-activity-metrics.ts`, `apps/anvil-api/src/lib/account-activity-rollup.ts`, `apps/anvil-api/src/routes/cron.ts`, `plans/modules/fleet-telemetry.aps.md#fleet-007-operator-fleet-view`, `plans/modules/continuous-improvement-backlog.aps.md#cib-004-simplify-admin-key-retrieval-with-credential-source-config`, `plans/archive/modules/admin-cli-hardening.aps.md`, GitHub issue #952 | Operator admin procedures; release/support handoff for admin key handling and fleet evidence |

`anvil admin` is the Rust operator CLI surface that wraps Anvil's admin HTTP API
(`/admin/*`). It is the supported way to approve waitlist signups, invite beta
users, revoke tokens, browse the audit log, and send migration emails during
beta.

This runbook covers install, configuration, every command with an example, the
exit-code taxonomy, and troubleshooting.

> **Legacy Node CLI retired (V060F-019, 2026-06-19).** The original Node
> operator CLI (`anvil-admin`) has been archived to
> `anvil-archive/admin-cli-node/` (out of the pnpm workspace). `anvil admin` is
> now the only supported surface — RCLI2-009 ported all of its subcommands and
> the Node tool's `X-Admin-Actor` attribution no longer matches the live API
> (ADMINCLIH-002). Do not run the archived tool.

## Install

`anvil admin` ships as part of the Rust `anvil` binary. Operators can use an
installed release build or run it from a local checkout.

```bash
git clone git@github.com:eddacraft/anvil-001.git
cd anvil-001
pnpm install
cargo build -p eddacraft-anvil
```

Then run the local binary:

```bash
./target/debug/anvil admin --help
```

Release installs expose the same surface as `anvil admin --help`.

## Configuration

The CLI reads admin configuration from environment variables and, when no admin
key env var is present, from the configured admin credential source. The Rust
admin surface does not accept per-invocation admin URL, key, or actor override
flags.

| Setting       | Env var           | Default                    |
| ------------- | ----------------- | -------------------------- |
| API base URL  | `ANVIL_API_URL`   | `https://api.eddacraft.ai` |
| Admin API key | `ANVIL_ADMIN_KEY` | _(required — no default)_  |

Resolution order, highest priority first:

1. `ANVIL_ADMIN_KEY` env var (CI, scoped child processes).
2. The configured credential source in `admin-auth.json` (`1password` or `key`).

So set the source **once** and you never `export` again — env still wins when
present, so CI is unaffected.

- `ANVIL_ADMIN_KEY` is sent as `Authorization: Bearer <key>`.
- Per-operator keys determine the audit actor server-side. Shared-key requests
  are attributed to the sentinel actor `shared-key@anvil`.

### Quick start — set the key once, no more `export`

Pick one of these and you're done; subsequent `anvil admin …` commands resolve
the key automatically.

```bash
# A. Store the key in Anvil's local config (mode 0600). Simplest.
#    Use `-` to read from stdin so the key never hits your shell history:
anvil admin auth set key -        # then paste the key + Enter
#    …or, if you don't mind it in history, pass it directly:
anvil admin auth set key <your-admin-key>

# B. Or point at a 1Password item (no plaintext at rest; needs the `op` CLI):
anvil admin auth set 1password op://Anvil/admin-key/credential

# Verify (the key is shown masked, never in full):
anvil admin auth status
anvil admin list
```

Missing or invalid admin credentials exit `3` (see **Exit codes** below):

```
Authentication required: no admin credential configured. Run `anvil admin auth set key <key>` to store it once, or `anvil admin auth set 1password <op://reference>`, or set ANVIL_ADMIN_KEY.
```

### Example shell setup

```bash
export ANVIL_API_URL="https://api.eddacraft.ai"
export ANVIL_ADMIN_KEY="sk_admin_…"       # placeholder — see "Handling the admin key"
```

> The `export ANVIL_ADMIN_KEY=…` line above is illustrative only; pasting a real
> key into a terminal this way puts it in shell history and `ps`-visible env.
> See **Handling the admin key** for the supported patterns (1Password, direnv,
> etc.) before wiring this up.

### Handling the admin key

`ANVIL_ADMIN_KEY` is a production secret. How you hold it on your workstation
matters as much as how the server stores its hash. These guidelines are in
preference order — start at the top and only drop down when a workflow forces
your hand.

The supported local pattern is **secret-manager-backed retrieval**.
`anvil admin` persists only the retrieval source, not the plaintext admin key.
The operator's approved password manager remains the authority for storage,
rotation, audit, and device unlock policy.

- **Do not put the key on the command line.** `ps(1)`, `htop`, and
  `/proc/<pid>/cmdline` expose argv to every other user on the host for the life
  of the process. Shell history captures it too. Use the `ANVIL_ADMIN_KEY` env
  var (resolved via one of the patterns below) so the raw bearer never appears
  in argv.

- **Do not `export ANVIL_ADMIN_KEY=…` inline** in a terminal session: the key
  lands in `.zsh_history` / `.bash_history` and is inherited by every child
  process. If you've done this, rotate the key and scrub shell history.

- **Preferred (no plaintext at rest) — configure the 1Password source once.**
  Store the 1Password item reference in Anvil's owner-only local config:

  ```bash
  anvil admin auth set 1password op://Anvil/admin-key/credential
  anvil admin auth status
  anvil admin list
  ```

  Normal `anvil admin ...` commands now run `op read` for that reference when
  `ANVIL_ADMIN_KEY` is not set. If 1Password is locked or `op` is unavailable,
  the CLI exits with authentication-required guidance instead of falling back to
  an unsafe prompt or storing the key.

  To remove the source:

  ```bash
  anvil admin auth unset
  ```

- **Simplest (convenience, plaintext at rest) — store the key in local config.**
  When you don't have `op` set up, or you just want the key to work without any
  per-shell ceremony, store it directly. Pass `-` to read from stdin so the key
  never reaches argv or shell history:

  ```bash
  anvil admin auth set key -        # paste the key, press Enter
  anvil admin auth status           # shows e.g. "****1234" — never the full key
  ```

  The key is written to `admin-auth.json` (mode `0600`, owner-only) and resolved
  automatically on every `anvil admin` call. **Tradeoff:** unlike the 1Password
  source, this keeps the plaintext key on disk — the same posture as `gh`,
  `npm`, and `aws` CLIs. It's appropriate for a single trusted operator
  workstation with full-disk encryption; prefer the 1Password source on shared
  hosts, and never use it on CI (use the `ANVIL_ADMIN_KEY` env var there). The
  status output and the JSON form both mask the key to a trailing fingerprint;
  the raw value lives only in the `0600` file. `anvil admin auth unset` removes
  it.

- **Alternative — 1Password CLI, scoped child process.** Keep a private
  `admin.env` file outside the repo (or in a directory covered by your global
  gitignore) with references, not plaintext secrets:

  ```dotenv
  ANVIL_ADMIN_KEY="op://Anvil/admin-key/credential"
  ```

  Then shell the key in only for the life of the admin invocation so it never
  reaches `~/.zshrc`, shell history, or a parent shell's environment:

  ```bash
  op run --env-file=admin.env -- anvil admin list
  ```

  `op run` injects the resolved secret only for the child process. Use this form
  for scripts that should not depend on the user's Anvil credential-source
  config.

- **Fallback — one-off 1Password read.** If `op run` is unavailable, command
  substitution is acceptable for one invocation because the key is assigned only
  in that process environment:

  ```bash
  ANVIL_ADMIN_KEY="$(op read 'op://Anvil/admin-key/credential')" \
    anvil admin list
  ```

  Do not promote this to shell startup files or long-lived exports; prefer
  `op run` as soon as the environment can support it.

- **Alternative — `direnv` scoped per project directory.** Put a `.envrc` beside
  the repo (or in a parent directory) that resolves the key at `cd`-time:

  ```bash
  # keep this OUTSIDE the repo, or add .envrc to your global gitignore
  # (`git config --global core.excludesfile ~/.gitignore_global` and append `.envrc`).
  # this repo does NOT ignore .envrc by default.
  export ANVIL_ADMIN_KEY="$(op read 'op://Anvil/admin-key/credential')"
  ```

  Then `direnv allow` the directory. The key is live only when `$PWD` matches
  the scope — leaving the directory unsets it.

- **Interactive fallback — `read -rs`.** When the manager isn't available (a new
  laptop, a container) and you need to paste from the password manager once, use
  a silent read instead of `export`:

  ```bash
  read -rs ANVIL_ADMIN_KEY && export ANVIL_ADMIN_KEY
  ```

  The key is never echoed, nor written to history. `unset ANVIL_ADMIN_KEY` when
  you're done.

- **Private dotenv as last resort.** A `~/.anvil-rust-admin.env` with mode
  `0600`, outside the repo, `source`'d into a scoped subshell. Never commit this
  file; add its path to your global git ignore.

- **CI/automation.** Inject via the platform's secret store (GitHub Actions
  secrets, Azure Key Vault, Vercel env). Never echo the key to logs, and never
  pass it on the command line. For pre-merge pipelines, prefer per-operator keys
  (see **Per-operator admin keys**) over the shared key.

**Rotation:** on suspected exposure or operator offboarding, rotate immediately.
Per-operator keys: edit the row out of `infra/src/admin-keys.ts` and run
`pulumi up` (see **Revoking a per-operator key** below). Shared key: see
**Rotating the shared `ADMIN_KEY`**.

## Per-operator admin keys

The admin surface supports two authentication paths during the dual-auth
rollout:

1. **Shared `ADMIN_KEY`** — a single high-entropy key configured via Vercel env.
   Every request authenticated this way is attributed to the sentinel actor
   `shared-key@anvil` and marked `auth_method: "shared"` in the audit log. The
   `X-Admin-Actor` header is **ignored** on this path.
2. **Per-operator keys** — rows in the `admin_keys` table keyed on a peppered
   hash of the raw bearer. Each row maps to a real `actor_email`. Audit rows are
   stamped with that email and `auth_method: "per_operator"`.

The dual path is gated on the `ADMIN_PER_OPERATOR_KEYS` server env var (set to
`1` or `true` to enable the lookup). When enabled, the middleware tries the
per-operator path first; on hash miss or DB error it falls back to shared-key
comparison so a DB hiccup does not take down the admin surface.

### Provisioning a per-operator key (Pulumi / IaC)

Per-operator keys are managed declaratively in `infra/src/admin-keys.ts`. The
IaC review (a normal PR) acts as the two-person rule — a reviewer approves the
actor_email + note pair before the key exists.

1. Edit `infra/src/admin-keys.ts` and add an entry to the `seed` array:

   ```ts
   {
     name: 'alice-eddacraft',       // Pulumi resource name (kebab-case)
     actorEmail: 'alice@eddacraft.ai',
     note: 'onboard 2026-04',
   }
   ```

2. Open a PR. The reviewer confirms the actor is authorised to hold an admin
   key.
3. After merge, the `Pulumi Up` workflow runs (or an operator runs `pulumi up`
   against the `dev` / `prod` stack). Pulumi:
   - generates a 32-byte bearer via `random.RandomBytes`
   - HMACs it with `ADMIN_KEY_PEPPER` (fetched from Key Vault)
   - inserts a row in `admin_keys`
   - writes a matching `admin_keys_audit` entry with `action: 'created'`,
     `change_actor` = the CI/user running Pulumi, `pulumi_commit_sha` =
     `GITHUB_SHA` (or `git rev-parse HEAD` for local runs).

4. Retrieve the bearer for distribution to the operator:

   ```bash
   pulumi stack select <dev|prod>
   pulumi stack output adminKeyBearers --show-secrets --json \
     | jq -r '."alice@eddacraft.ai"'
   ```

   The bearer lands in the operator's 1Password shared vault — never in git,
   Slack, email, or ticket systems. The server only ever sees the hash; losing
   the bearer means rotating the row.

`ADMIN_KEY_PEPPER` is a server-side secret (Key Vault: `admin-key-pepper`). It
is separate from `TOKEN_PEPPER` (access-token hashing) so rotation of either
does not invalidate the other. Rotating the pepper invalidates every
per-operator key and requires re-running Pulumi to re-seed them.

### Revoking a per-operator key (Pulumi / IaC)

Remove the entry from `seed` in `infra/src/admin-keys.ts` (or change the
`actorEmail` — same thing). On the next `pulumi up`, the delete lifecycle runs:
`admin_keys.revoked_at = now()` and a matching audit row with
`action: 'revoked'`. The bearer is dropped from Pulumi state; presenting it to
the API thereafter returns 401 `admin_key_revoked`.

### Break-glass: manual SQL provisioning

Only when the IaC path is unavailable (e.g. Pulumi backend down, emergency
operator rotation outside a review window). Two operators must co-sign the
change in `#beta-ops` before running.

```sql
-- 1. Generate a 32-byte bearer out-of-band (`openssl rand -hex 32`).
-- 2. Compute hashed_key = HMAC-SHA-256(ADMIN_KEY_PEPPER, bearer):
INSERT INTO admin_keys (hashed_key, actor_email, note)
VALUES ('<hex-digest>', 'alice@eddacraft.ai', 'break-glass 2026-04');

INSERT INTO admin_keys_audit
  (admin_key_id, action, change_actor, pulumi_commit_sha, note)
VALUES
  ((SELECT id FROM admin_keys WHERE hashed_key = '<hex-digest>'),
   'created',
   'ops-pair@eddacraft.ai',
   '<git-sha-of-the-tracking-PR>',
   'break-glass manual provision');
```

Follow up with a reviewed PR moving the entry into IaC — the manual row becomes
a no-op once Pulumi observes the matching `hashed_key` for the same
`actor_email`.

Revoking manually:

```sql
UPDATE admin_keys SET revoked_at = now() WHERE actor_email = 'alice@eddacraft.ai'
  AND revoked_at IS NULL;

INSERT INTO admin_keys_audit (admin_key_id, action, change_actor, pulumi_commit_sha, note)
SELECT id, 'revoked', 'ops-pair@eddacraft.ai', '<git-sha>', 'break-glass revoke'
FROM admin_keys WHERE actor_email = 'alice@eddacraft.ai' AND revoked_at IS NOT NULL;
```

Status codes:

- **401 `admin_key_revoked`** — a presented key matched a row that has been
  revoked. Writes an audit row with `outcome: "rejected_revoked"`.
- **401** for missing/malformed `Authorization` headers. Writes an audit row
  with `outcome: "rejected_malformed"`.
- **403 Forbidden** — the bearer did not match any per-operator key or the
  shared `ADMIN_KEY`. Writes an audit row with `outcome: "rejected_unknown"`.

The raw bearer is never logged — only its peppered hash.

### Rotating the shared `ADMIN_KEY`

The shared key is the break-glass path and should be rotated on operator
offboarding, suspected exposure, or at least once per quarter:

1. Generate a new random key (≥256 bits): `openssl rand -hex 32`.
2. Update the Vercel env `ADMIN_KEY` for both preview and production.
3. Redeploy — the old key stops working as soon as the new deploy is live.
4. Distribute the new key to active operators via 1Password. Remove the old item
   from the vault.
5. Confirm `anvil admin list` works with the new key.

The final cutover (removal of the shared-key path) happens only after the
shared-key request rate is zero for ≥7 consecutive days, tracked via the
`auth_method` audit column.

### Rollout tracking

Every admin mutation writes `auth_method` on the audit row. To monitor adoption,
filter to admin-authenticated rows only — `audit_log` also carries non-admin
traffic (GitHub OAuth, auth failures) that would otherwise drown the signal:

```sql
SELECT auth_method, COUNT(*)
FROM audit_log
WHERE occurred_at > now() - interval '7 days'
  AND action <> 'admin.auth.failed'
  AND (actor = 'shared-key@anvil' OR auth_method = 'per_operator')
GROUP BY auth_method;
```

When the `shared` count holds at zero for seven consecutive days, schedule the
cutover PR that removes the shared-key branch.

## Commands

All commands accept global `--json` for machine-readable output. Without
`--json` they render human tables. Put `--json` before or after `admin`, for
example `anvil --json admin list` or `anvil admin list --json`.

### `list` — show waitlist entries

```bash
anvil admin list                               # server defaults, 50 rows
anvil admin list --status approved --limit 10
anvil admin list --source website --status all
```

Flags:

- `--status <pending|approved|all>` (default `pending`)
- `--source <manual|website|import|all>` (default `all`)
- `--limit <1-200>` (default `50`)
- `--offset <n>` (default `0`)

**Status is not a column on `waitlist`.** It is derived from the durable
`approved_at` timestamp:

| `--status` | Meaning                                                                |
| ---------- | ---------------------------------------------------------------------- |
| `pending`  | `approved_at IS NULL` — still in the queue                             |
| `approved` | `approved_at IS NOT NULL` — operator admitted via `approve` / `invite` |
| `all`      | full signup ledger (rows are kept after admission)                     |

`approved_at` is set once on first admin grant (re-invite keeps the original
timestamp). Revoke does **not** clear it — admission history stays. A matching
`beta_users` row alone (for example a pending GitHub OAuth signup) does **not**
count as approved.

Neon equivalents:

```sql
-- Queue
SELECT email, name, source, created_at
FROM waitlist
WHERE approved_at IS NULL
ORDER BY created_at;

-- Admitted
SELECT email, name, source, created_at, approved_at
FROM waitlist
WHERE approved_at IS NOT NULL
ORDER BY approved_at;
```

### `fleet` — show the current fleet overview

```bash
anvil admin fleet
anvil --json admin fleet
```

The human view shows active installs, version and install-method distributions,
feature adoption, and recent retention cohorts. The JSON form passes through the
stable `anvil.fleet-overview.v1` API contract for evidence tooling.

This is a current snapshot; v1 accepts no date argument. Postgres `current_date`
is authoritative. DAU, WAU, and MAU mean distinct installs with a beacon
observed today, in the inclusive trailing 7 days, and in the inclusive trailing
30 days respectively. Distributions use each MAU install's latest beacon.
Feature adoption counts MAU installs with positive usage, while `usageCount`
sums positive observations.

Retention is labelled **observed-beacon retention**: absence means no retained
beacon was observed, not proven product abandonment. Cohorts and current
identity-based metrics come from the retained raw window, which is capped at 90
calendar dates and may be configured lower; the response's `rawRetentionDays`
states the effective value. The boundary cohort is excluded because its earlier
history may already have been purged.

`historicalAggregates` exposes kept-indefinitely daily dimension cells and
feature totals after their raw rows expire. Each `dailyInstallDimensions` row is
a distinct-install count only within its exact day/version/install-method/
platform/channel cell. An install that changed dimensions can appear in more
than one cell, so **do not sum cells or derive a share denominator from them**.
These aggregates preserve directional history but cannot reconstruct install
identities, so they are never used to manufacture historical MAU or retention
cohorts.

> **Data-quality caveat:** beacons carry anonymous random install IDs and are
> not authenticated or independently verified. Installs can reset or fabricate
> IDs and payloads. Treat every fleet metric as directional product evidence,
> not audit-grade evidence or a verified customer count. The API and human CLI
> repeat this caveat in `notes.dataQuality` / the `Data quality` line.

### `show <email>` — full profile for one email

Prints the user row (including **login stamps** when present), allowlisted
**feature touches**, tokens, and the most recent audit entries.

Login fields (`first_login_at`, `last_login_at`, `last_login_method`) are set
only after an interactive session mint (GitHub device, OTP, or legacy device).
Invite/approve alone does **not** stamp login — human output shows
`login: never logged in` when null.

Feature touches are the BACT identity-bound allowlist (`watch`, `start`,
`check`, `auth`). They are **not** FLEET beacons.

```bash
anvil admin show alice@example.com
anvil --json admin show alice@example.com
```

### `users` — CS engagement cohorts (BACT-006)

List **active** beta users (not waitlist) by engagement filter. Distinct from
`admin list` (waitlist) and `admin fleet` (anonymous population).

| `--engagement`    | Meaning                                                                      |
| ----------------- | ---------------------------------------------------------------------------- |
| `never_logged_in` | `first_login_at IS NULL`                                                     |
| `idle`            | Has logged in, but `last_login_at` older than `--idle-days` (default **30**) |
| `missing_feature` | Has logged in, but no touch row for `--feature`                              |

```bash
anvil admin users --engagement never_logged_in --limit 50
anvil admin users --engagement idle --idle-days 30
anvil admin users --engagement missing_feature --feature watch
anvil --json admin users --engagement idle --idle-days 14 --limit 20
```

### `activity` — DAA/WAA/MAA account activity metrics (BACT-009)

Named-account activity window metrics computed from `last_activity_at`
(BACT-008). Reports **accounts**, never installs — distinct from `admin fleet`'s
anonymous FLEET DAI/WAU/MAU (see **DAI vs DAA** below). This is an aggregate
metrics surface, not a listing — use `admin users --engagement idle` /
`--engagement never_logged_in` to list the accounts behind a cohort, or
`admin show <email>` for one account's `plan` and `last_activity_at`.

```bash
anvil admin activity
anvil admin activity --plan beta
anvil admin activity --idle-days 14
anvil --json admin activity --plan beta --idle-days 14
```

Flags:

- `--plan <name>` — restrict to one account plan (today only `beta`). The CLI
  does **not** validate this against a hardcoded plan list — an older `anvil`
  binary would otherwise reject a plan a newer server has since added, breaking
  the backward-tolerance the admin surface relies on elsewhere (see BACT-003).
  The plan set is the **server's** closed list; an unrecognised value is a
  **server-rejected API error** (exit `1`, not a clap usage error), never a
  silent empty result.
- `--idle-days <1-365>` (default **30**) — the quiet-cohort window. Unlike
  `--plan`, this range **is** enforced client-side by clap before any request is
  sent — out-of-range values are a usage error (exit `2`).

Output:

- `activeAccounts.daily` / `.weekly` / `.monthly` — DAA/WAA/MAA: accounts with
  `last_activity_at` within the trailing 1/7/30 days. An account active exactly
  N days ago still counts in that window (inclusive boundary); one millisecond
  older falls into the next window out.
- `neverActive` — accounts with `last_activity_at IS NULL` (admitted but never
  logged in, refreshed, or touched an allowlisted feature)
- `quiet.count` — accounts with no activity ever, or activity strictly older
  than `--idle-days` (an account exactly `--idle-days` old is not yet quiet)

### Daily historical-DAA rollup (BACT-011)

`admin activity` (above) answers "how many accounts are active **right now**"
from the live `last_activity_at` pointer. It cannot answer "how many accounts
were active on day D" once accounts have gone quiet or become active again on a
later day — that evidence is gone from `beta_users` the moment the pointer moves
on. **BACT-011** adds a daily snapshot table, `activity_rollup_daily` (migration
`022-account-activity-rollup-daily.sql`), for that historical question.

**The job.** No new scheduling mechanism was introduced — the rollup piggybacks
on the **existing hourly Vercel Cron sweep**, `GET /cron/cleanup`
(`apps/anvil-api/src/routes/cron.ts`), the same
`Bearer ${CRON_SECRET}`-protected, non-public route BACT-011 shares with
telemetry-beacon rollup and expired-token cleanup (`vercel.json`:
`{ "path": "/api/v1/cron/cleanup", "schedule": "0 * * * *" }`). Every hourly run
recomputes the trailing **7** completed UTC days (per `plan`, plus a reserved
`__all__` total row) and **upserts** each `(day, plan)` row.

The rollup is **error-isolated** from the other six cleanup steps on that route:
if it throws (e.g. a transient DB error), the sweep still responds `200` with
the cleanup counts intact, and the failure is reported as
`activityRollup: { error: <message> }` in the response body instead of
`{ days, rows }` — a rollup failure never masks successful cleanup or makes
Vercel Cron treat the whole hourly run as failed.

**Grain and retention.** One row per completed UTC day × plan (today only
`beta`, plus the `__all__` total). Kept **indefinitely** — volume is trivial (at
most a handful of rows per day), and unlike FLEET's raw `telemetry_beacons`
there is no per-event table behind this rollup to prune; the rollup itself _is_
the retained aggregate.

**Best-observation upsert — stored counts never decrease.** The write is
`SET active_accounts = GREATEST(stored, newly-observed)`, not a plain overwrite.
`last_activity_at` only ever _advances_, so a later re-roll of an
already-written day can only observe the same or a **smaller** set of accounts
still pointing at that day (accounts active again since have moved their pointer
past it). A plain overwrite would let that later, smaller recount shrink an
already-correct earlier snapshot every single hour; GREATEST instead keeps each
day's best-ever observation. Re-running the same day any number of times still
never double-counts (it's a max, not a sum), and a short outage still self-heals
on the next run.

**Honest limitation — a day's first rollup can still undercount if it's late.**
`last_activity_at` is a _latest-pointer_ column, not an activity log. A day's
count is only as good as whichever accounts show `last_activity_at` falling on
that UTC day **the first time the job ever looks**. If a day's _first_ rollup
happens late — after an account's `last_activity_at` has already advanced past
it (the account was active again on a later day before the job ever saw the
earlier day) — that account is invisible to every rollup from then on, and
GREATEST has nothing to raise the stored value from. The 7-day trailing window
means a Vercel Cron outage shorter than a week self-heals (every day still gets
its first look in time); an outage or backfill gap longer than that does not
recover the lost evidence. Treat `activity_rollup_daily` as **accounts observed
active that day**, not an exact audit log — the GREATEST upsert guarantees the
stored value never falls below the best snapshot ever taken, but cannot
manufacture a snapshot that was never taken.

**Reading history.** `GET /admin/activity?history=true` (optional
`&historyDays=N`, default **14**, max **90**) attaches a `history` block —
`{ days, series: [{ day, plan, activeAccounts }, …] }`, most-recent day first —
to the same envelope `admin activity` already returns. Add `plan=` to scope the
series to one plan; omit it for the `__all__` cross-plan total series. Omitting
`history` entirely leaves the BACT-009 response unchanged (no second query runs)
— this is additive and backward tolerant.

```bash
anvil --json admin activity --plan beta   # existing BACT-009 window metrics
# History is not yet wired to a CLI flag (deliberately deferred — the admin
# API + this runbook satisfy BACT-011's read-path requirement). Operators
# call the API directly for now:
curl -H "Authorization: Bearer $ADMIN_KEY" \
  "$ANVIL_API_URL/admin/activity?plan=beta&history=true&historyDays=30"
```

### Plan, activity, and DAA vocabulary

([ADR-121](../../plans/decisions/121-account-plan-activity-and-flag-entitlements.md),
[design spec](../../plans/specs/2026-08-12-account-plan-activity-entitlements.md),
BACT-007..013) adds an account-level vocabulary. `show`, `users`, and `activity`
(above) expose it as follows.

**`plan`** — every account carries a durable plan name. Today the only value is
**`beta`**, mapping to feature-flag catalogue audience `plan-beta`. `plan` is a
separate axis from account `status` (lifecycle: `active` / `pending` /
`suspended` / `banned`) and from token `scopes` (capability grants gated by
`api.scope.*` flags). **Live (BACT-009):** `anvil admin show <email>` and
`anvil --json admin show <email>` surface `plan` alongside login stamps.

**`last_activity_at`** — a single durable “did this account do anything lately”
stamp, distinct from the login stamps `show` already prints:

| Action                                                                      | Advances `last_activity_at`? | Advances login stamps?                                         |
| --------------------------------------------------------------------------- | ---------------------------- | -------------------------------------------------------------- |
| Interactive session mint (GitHub, GitHub device, OTP, legacy device)        | Yes                          | Yes (`first_login_at` / `last_login_at` / `last_login_method`) |
| Successful session refresh                                                  | Yes                          | No                                                             |
| Authenticated allowlisted feature-touch (`watch`, `start`, `check`, `auth`) | Yes                          | No                                                             |
| Invite / approve                                                            | No                           | No                                                             |

Invite/approve alone never counts as activity or login — an admitted account
that has not completed interactive login still shows `last_activity_at: null`
and `login: never logged in`. **Live (BACT-009):** `anvil admin show <email>`
also surfaces `last_activity_at` (and `last_activity_kind` when set).

**DAI vs DAA** — do not conflate these two labels:

| Label   | Full name               | Counts                                                          | Source                                  |
| ------- | ----------------------- | --------------------------------------------------------------- | --------------------------------------- |
| **DAI** | Daily active _installs_ | Anonymous FLEET beacons (`install_id`)                          | `anvil admin fleet` (live)              |
| **DAA** | Daily active _accounts_ | Named, authenticated accounts with `last_activity_at` in-window | `anvil admin activity` (live, BACT-009) |

FLEET DAI is directional population evidence and carries no identity (ADR-107).
DAA is identity-bound and answers “how many customers used the product,” never
the reverse. Never report FLEET DAI as a customer login count, and never join a
FLEET `install_id` to an account.

**“Quiet” vs “never (interactively) logged in”** — two idle definitions coexist:

- **Never (interactively) logged in** — today's
  `admin users --engagement never_logged_in` (`first_login_at IS NULL`). Says
  nothing about token-only refresh or feature-touch use.
- **Login-idle** — today's `admin users --engagement idle` (`last_login_at`
  older than `--idle-days`, default 30). Also blind to
  refresh/feature-touch-only accounts.
- **Quiet (activity-idle)** — covers login, refresh, and feature-touch via
  `last_activity_at`. **Live (BACT-009):** `anvil admin activity --idle-days N`
  reports the quiet count; prefer it over login-idle for CS “who has gone quiet”
  queries. (`admin users --engagement idle` still lists individual accounts by
  login-idle when you need names, not just a count.)

Full vocabulary, entitlement model, and evaluation-context wiring:
[account plan, activity, and entitlements](../guides/account-plan-activity-and-entitlements.md).

**EMAIL cohort note:** broadcast audiences `beta:active-recent` /
`beta:active-idle` still resolve via refresh-token age until a deliberate
follow-up migrates them. Prefer `admin users` login stamps for CS follow-up.

### `approve [email]` — approve one or a batch

Single approve:

```bash
anvil admin approve alice@example.com
```

Oldest N pending:

```bash
anvil admin approve --batch 10
```

Flags:

- `--batch <n>` — approve the oldest N unapproved entries (mutually exclusive
  with `[email]`)
- `--json`

### `invite <email>` — invite to beta

Creates a user (if needed), issues a beta token, and sends the invite email.

```bash
anvil admin invite alice@example.com --name "Alice Example"
anvil admin invite alice@example.com --notes "VIP customer"
anvil admin invite ci@example.com --token       # print raw token once
anvil admin invite early@example.com --edict    # print revokable edict once
```

Flags:

- `--name <name>` — display name
- `--notes <text>` — internal notes stored on the user row
- `--token` — skip the invite email and print the raw token once
- `--edict` — issue a revokable early-access edict and print it once
- `--json`

### `revoke [email]` — revoke tokens

Revoke all active tokens for an email, or a specific raw token string.

```bash
anvil admin revoke alice@example.com            # prompts for confirmation
anvil admin revoke alice@example.com --yes
anvil admin revoke --token "betatok_…" --yes    # revoke one specific token
```

Flags:

- `--token <raw>` — revoke a specific raw token (mutually exclusive with
  `[email]`)
- `-y, --yes` — skip confirmation
- `--json`

### `audit` — browse the audit log

```bash
anvil admin audit
anvil admin audit --action user.approved
anvil admin audit --filter-actor you@eddacraft.ai --limit 20
anvil admin audit --offset 50
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
anvil admin send-migration                              # dry-run, source=import, limit=20
anvil admin send-migration --source website --limit 5   # dry-run, different filter
anvil admin send-migration --no-dry-run                 # preview -> prompt -> send (interactive)
anvil admin send-migration --no-dry-run --yes           # preview -> send (non-interactive)
anvil --json admin send-migration                       # raw JSON dry-run, includes previewToken
```

A human dry-run prints the recipient table and preview expiry; JSON dry-runs
also include `previewToken` for automation. Operators do not need to copy the
token manually: running with `--no-dry-run` always fetches a fresh preview token
and sends against that exact snapshot within the 10-minute TTL.

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
2. If count is `0`, prints `No recipients match the filter. Nothing to send.`
   and exits `0`. Goes to stdout by default; routed to stderr when `--json` is
   set so stdout stays reserved for JSON output (empty in this case). Pipe
   stdout to `jq` and keep stderr separate — do not use `2>&1` when you need
   clean JSON.
3. Writes the recipient table plus the warning
   `About to send migration email to N recipient(s) …` to **stderr**, then
   prompts on stderr: `Continue? [y/N]`. With `--json`, the recipient table is
   replaced by a one-line `preview: N recipient(s) …` status on stderr so the
   stdout contract holds. With `--yes`, the prompt is skipped but the dry-run
   still runs — the token is always fetched.
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

| HTTP | `code`                   | What it means                                                              | Recovery                                                                                     |
| ---- | ------------------------ | -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| 409  | `cohort_drift`           | Recipients changed since the preview. Response includes `added`, `removed` | Re-run `anvil admin send-migration --no-dry-run` — the CLI will fetch a fresh snapshot first |
| 410  | `preview_token_expired`  | The 10-minute TTL elapsed                                                  | Re-run `anvil admin send-migration --no-dry-run` (within 10 minutes next time)               |
| 410  | `preview_token_consumed` | A prior send call already used this token                                  | **Verify the previous send completed** in the audit log, then decide whether to re-send      |
| 410  | `preview_token_missing`  | Token not found, reaped, or owned by a different operator                  | Re-run `anvil admin send-migration --no-dry-run` with the same per-operator admin key        |
| 400  | `preview_token_required` | Should not occur via the CLI; indicates a client bug                       | File a ticket; workaround is to re-run the CLI (it always fetches a token first)             |

For a `preview_token_consumed` recovery, confirm before re-sending:

```bash
anvil admin audit --action migration.email.sent --limit 5
```

Look for an entry matching the source, count, and preview token you expect. If
the previous send completed successfully, do **not** re-run. If it partially
sent and was interrupted, coordinate in `#beta-ops` before re-running — the
second run will re-email every recipient in the snapshot.

Non-TTY refusal (`exit 1`) applies only to the **real-send** path. A plain
dry-run works in any session. In non-TTY sessions without `--yes`, the CLI
refuses to prompt and exits `1`.

### `email-update <current-email> <new-email>` — update a beta user's email

This surface is Rust-only (`anvil admin email-update`) and wraps
`POST /admin/user/email-update`. It updates the `beta_users.email` value for a
user who cannot self-service an email mismatch. The historical waitlist row is
left unchanged.

```bash
anvil admin email-update old@example.com new@example.com
anvil --json admin email-update old@example.com new@example.com
```

The API rejects same-address updates, missing users, and collisions with an
existing beta user email.

### `email-send <email> --template <key>` — one-off template email

Send a **broadcast-kind** template to a single address (preview / operator
one-off). Does not use the broadcast snapshot/cohort machinery.

```bash
anvil admin email-send person@example.com --template release-announcement
anvil admin email-send person@example.com --template release-announcement \
  --props-file ./props.json
anvil --json admin email-send person@example.com --template waitlist-migration --name "Alex"
```

- Allowed templates: `release-announcement`, `waitlist-migration`
- Transactional templates (`beta-invite`, `otp-code`, …) stay on invite / OTP /
  waitlist routes
- Empty `templateProps` is fine for release-announcement (template defaults
  apply)
- Audits as `email.sent`
- Rate-limited separately from full-cohort `/admin/broadcast`

### `name-update <email> --name <name>` — enrich display name without inviting

Operator enrichment path: set display name (and optional `beta_users` notes)
**without** invite, approve, token issue, or outbound email. Use this when you
learn a person's name from a follow-up message or other ops context and need to
correct waitlist / beta_users records.

Wraps `POST /admin/user/name-update`.

```bash
anvil admin name-update person@example.com --name "Full Name"
anvil admin name-update person@example.com --name "Full Name" --notes "design partner"
anvil --json admin name-update person@example.com --name "Full Name"
```

Behaviour:

- Overwrites `waitlist.name` when a waitlist row exists.
- Overwrites `beta_users.name` when a beta user exists; optional `--notes`
  updates only when a beta user exists (API returns 400 if notes are sent for a
  waitlist-only email).
- Returns 404 when neither waitlist nor beta_users has the email.
- Audits as `user.name.updated`. Prefer this over re-running `invite` solely to
  set a name (invite re-sends beta mail and activates).

## Exit codes

| Code | Meaning                               | Typical cause                                                                       |
| ---- | ------------------------------------- | ----------------------------------------------------------------------------------- |
| `0`  | Success                               | —                                                                                   |
| `1`  | Command failed                        | API error, network error, invalid API URL, partial send failure, or non-TTY refusal |
| `2`  | Usage error from clap                 | Out-of-range `--limit`, bad enum choice, missing required argument                  |
| `3`  | Authentication required               | `ANVIL_ADMIN_KEY` missing, invalid, or not authorised                               |
| `4`  | Reserved Rust CLI configuration error | Used by auth preflight surfaces, not currently by `anvil admin`                     |

All errors go to stderr; `--json` payloads go to stdout. The Rust CLI currently
uses the common `1` error path for HTTP, network, schema, and prompt-refusal
failures rather than a typed admin-specific exit taxonomy.

## Troubleshooting

### "Authentication required"

Set `ANVIL_ADMIN_KEY` from the approved secret manager. For per-operator keys,
confirm the key has not been revoked. For the shared break-glass key, confirm
the server-side `ADMIN_KEY` value has not rotated.

### "cannot reach …"

- Check `ANVIL_API_URL` — default is `https://api.eddacraft.ai`
- Check VPN / network egress
- Retry with `ANVIL_API_URL=https://api.eddacraft.ai anvil admin list` to rule
  out a bad environment value

### "server error 5xx"

The API is the issue, not the CLI. Check the observability dashboard
(`docs/runbooks/observability-triage.md`) and the recent deploys on Vercel.
Rerun once the issue is cleared.

### Response validation or malformed JSON errors

The admin API returned a payload whose shape does not match the Rust CLI structs
in `crates/anvil-cli/src/auth/client.rs`. This is contract drift: server and CLI
are on different versions. The failing field path is in the error message, and
the raw response body may be visible with `--verbose` / in CI logs.

What to do:

1. Note which endpoint failed (e.g.
   `response validation failed at items.3.approved_at: Expected string, received null`).
2. Check whether the CLI or the API was deployed most recently. The lagging side
   needs to be upgraded.
3. If the server change is intentional, update the Rust response types and
   release a new `anvil` binary.
4. If the CLI change is ahead of a rolled-back server, use the previous `anvil`
   release until the server catches up.

This code is strict on purpose: silently accepting a drifted payload would let
us operate on stale or malformed admin data. If the drift is harmless and you
need to unblock urgently, use the raw API directly (`curl`) while coordinating a
fix.

### "refusing to send migration without --yes in a non-interactive session"

You're running in a script or CI job without a TTY. Either run interactively or
pass `--yes` after you've verified the dry-run output.

### Sent the wrong thing

- For a bad approve/invite: use `anvil admin revoke` to invalidate the token,
  then re-invite
- For a migration email sent to the wrong cohort: there is no unsend — escalate
  in `#beta-ops`

### Looking for what happened

Every admin mutation writes to the audit log. Use `anvil admin audit` to review,
filtering by `--filter-actor` (who) and `--action` (what). The admin API also
logs request/response metadata in the Vercel logs for 7 days.

## Related

- Canonical CLI surface reference (all commands): `docs/runbooks/cli-surface.md`
- Historical Node admin CLI design spec:
  `plans/specs/2026-04-16-admin-cli-design.md`
- Historical Node admin CLI module plan (archived):
  `plans/archive/modules/admin-cli.aps.md`
- Historical Node admin CLI hardening plan:
  `plans/archive/modules/admin-cli-hardening.aps.md`
- Waitlist email operations: `docs/runbooks/waitlist-email-operations.md`
- Release announcement email: `docs/runbooks/release-announcement-email.md`
- Observability triage: `docs/runbooks/observability-triage.md`
- One-shot `last_activity_at` refresh-token backfill (BACT-012):
  `docs/runbooks/account-activity-backfill.md`
