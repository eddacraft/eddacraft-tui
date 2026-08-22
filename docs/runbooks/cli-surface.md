# CLI Surface Reference

| Type    | Authority     | Owner | Status | Freshness                                                                                                                                                                                                                                                                                                            |
| ------- | ------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | CLIC  | Live   | Targeted 2026-08-22 addition of `anvil impact` (IMPV-001); prior: 2026-08-16 updates for the CLI-wide `--json` contract (#3947 operator decision A), the per-surface repairs on `config` (#3938/#3943) and `migrate format` (#3946), and the #3962 canonical YAML metadata correction; not a full CLI-surface review |

| Upstream                                                         | Downstream                                                  |
| ---------------------------------------------------------------- | ----------------------------------------------------------- |
| `crates/anvil-cli/src/main.rs`, `crates/anvil-cli/src/commands/` | Operator procedures, onboarding docs, CI integration guides |

Canonical per-command reference for the `anvil` CLI. Documents the **current**
command set; the spec (`plans/specs/2026-05-07-cli-surface-coherence.md`)
describes planned future renames and consolidations — each entry notes these
where relevant.

**Global flags** available on every command:

| Flag               | Description                                                        |
| ------------------ | ------------------------------------------------------------------ |
| `--json`           | Success stdout is exactly one JSON document (CLI-wide, see below). |
| `--no-tui`         | Disable TUI rendering; use plain text output.                      |
| `--verbose` / `-v` | Enable verbose logging.                                            |

### The CLI-wide `--json` contract (#3947, decision A)

`--json` binds CLI-wide: an accepted flag is a promise, and no command may
accept it and silently print prose. Both placements are equivalent
(`anvil --json <command>` and `anvil <command> --json`). On success, stdout is
exactly **one JSON document** — no leading banners, no trailing hints; facts the
human output reports survive as fields. Diagnostics, warnings, and progress stay
on stderr. Runtime failures under `--json` emit a structured `{"error": …}`
envelope on stderr, and the pre-dispatch auth wall emits its documented
envelopes on stdout with the documented exit codes.

Bounded deviations, all documented per command below:

- **`--format` surfaces** (`audit`, `check`, `gate`, `capsule verify` /
  `explain`, ADR-056): `--json` is the compatibility alias for `--format json`;
  an explicit non-JSON `--format` takes precedence.
- **Machine streams** keep their stream contracts: `watch --json` is NDJSON (one
  JSON object per line), `mcp serve` / `lsp` are JSON-RPC protocol servers,
  `watch --save-time-driver` is silent by contract, and `dashboard --web --json`
  prints one document then serves.
- **Silent surfaces** (git-invoked `hook` execution paths, `auth login` device
  flows, `l4-validate`) have no success stdout to convert; their contracts are
  exit codes and stderr.
- **Interactive-only**: `anvil tutorial` refuses `--json` with a structured
  error (`--reset` still emits a document). Contradictory combinations
  (`start --watch --json`, `insights --share --json`, …) are refused the same
  way, never silently ignored.
- **Delegated children** (`update` via the sidecar or a package manager): the
  child's stdout is captured or suppressed so anvil's stdout stays one document.

Every terminal command path is classified against this contract in
`crates/anvil-cli/src/json_surface_audit.rs`; a new command fails that test
until it is classified, and the process tests in
`crates/anvil-cli/tests/json_stdout_contract.rs` parse the whole of stdout on
the repaired surfaces.

> **Class is descriptive, not an auth marker.** The **Class** field below
> describes _who_ typically runs a command and _when_ (User-explicit, Setup,
> Admin, Background, Internal) — it does **not** indicate whether authentication
> is enforced. Auth posture is independent: a command requires auth only if it
> is licence-gated (`feature_flags::CLI_GATED_COMMANDS`), admin-key-gated
> (`anvil admin`), or feature-gated (e.g. `anvil plan dashboard`, CIB-046).
> Several `Admin`-class commands are unauthenticated because they act on local
> state with no server authority. Do not infer gating from Class — see
> [ADR-076](../../plans/decisions/076-feature-catalogue-surface-registry.md).

---

## anvil audit

**Class:** User-explicit **Purpose:** Run a full project audit. **When to use:**
On-demand deep scan of the current project for anti-patterns and issues across
all source files.

**Synopsis:** `anvil audit [--format <fmt>]`

**Flags:**

| Flag             | Description                                                          |
| ---------------- | -------------------------------------------------------------------- |
| `--format <fmt>` | Output format: `auto` (default), `tui`, `plain`, `json`, or `sarif`. |

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Common errors:**

- `--format sarif` not yet wired: SARIF output for `audit` is pending — use
  `check` or `gate` for SARIF today.

**Examples:**

```
$ anvil audit
$ anvil audit --format json | jq .
```

---

## anvil audit-chain

**Class:** User-explicit (on-demand) / Background (CI cron) **Purpose:** Audit
the witness chain for commits that bypassed protection. **When to use:** Nightly
in CI to detect force-pushed or admin-overridden commits; on-demand when a
bypass is suspected.

**Synopsis:**
`anvil audit-chain [--branch <ref>] [--since <ref>] [--threshold <n>] [--rescan] [--max-runtime <secs>]`

**Flags:**

| Flag                   | Description                                                                   |
| ---------------------- | ----------------------------------------------------------------------------- |
| `--branch <ref>`       | Branch tip to walk back from. Default: `HEAD`.                                |
| `--since <ref>`        | Earliest ancestor to include (walks `<since>..<branch>`).                     |
| `--threshold <n>`      | Drift count threshold for the `degraded:audit-drift` marker. Default: `5`.    |
| `--rescan`             | Re-run the rule engine across history in addition to witness-presence checks. |
| `--max-runtime <secs>` | Wall-clock cap on the audit walk. Stops with `partial: true` when exceeded.   |

**Exit codes:** 0 (clean), 2 (drift threshold exceeded), 1 (scan error)

**Common errors:**

- `degraded:audit-drift`: unwitnessed commit count meets or exceeds
  `--threshold`. Review with `anvil doctor`.

**Examples:**

```
$ anvil audit-chain
$ anvil audit-chain --since main --threshold 10
$ anvil audit-chain --rescan --max-runtime 60
```

---

## anvil check

**Class:** User-explicit **Purpose:** Scan files for anti-patterns and hardcoded
secrets (planless mode). **When to use:** Quick ad-hoc scan of specific files or
changed files without a full gate profile. For architecture, policy, and
dependency checks use `anvil gate`.

**Synopsis:**
`anvil check [files...] [--changed] [--staged] [--since <ref>] [--all] [--extensions <exts>] [--severity <level>] [--artifact <kind>] [--format <fmt>]`

**Flags:**

| Flag                  | Description                                                                            |
| --------------------- | -------------------------------------------------------------------------------------- |
| `files`               | Files to analyse (positional). Overrides `--changed`/`--staged`/`--since`.             |
| `--changed`           | Analyse git-changed files only.                                                        |
| `--staged`            | Analyse only staged files (implies `--changed`).                                       |
| `--since <ref>`       | Compare against a git ref (implies `--changed`).                                       |
| `--all`               | Analyse all source files in the project.                                               |
| `--extensions <exts>` | Comma-separated file extensions to analyse (e.g. `.ts,.tsx,.html`).                    |
| `--severity <level>`  | Minimum severity for blocking: `error`, `warning`, `info`. Default: `error`.           |
| `--include-opt-in`    | Include opt-in patterns.                                                               |
| `--artifact <kind>`   | Artifact kind: `source` (default), `pr-description`, `commit-message`, `agent-output`. |
| `--format <fmt>`      | Output format: `auto`, `tui`, `plain`, `json`, `sarif`.                                |

**Exit codes:** 0 (no blocking findings), 1 (blocked or error), 2 (gate check
failed), 3 (auth required)

**Examples:**

```
$ anvil check src/main.rs
$ anvil check --staged
$ anvil check --all --format json
$ anvil check --since main --severity warning
```

---

## anvil doctor

**Class:** User-explicit **Purpose:** Run diagnostic checks on your environment.
**When to use:** When Anvil is behaving unexpectedly or a setup step failed.
Also useful as a pre-flight in CI. Doctor also runs daily MCP self-heal (rewrite
drifted owned entries and poke live children) unless MCP heal is pinned.

**Synopsis:** `anvil doctor [--fix]`

**Flags:**

| Flag    | Description                     |
| ------- | ------------------------------- |
| `--fix` | Auto-fix issues where possible. |

**Exit codes:** 0 (healthy), 1 (degraded), 2 (gate problem found), 3 (auth
required)

**Examples:**

```
$ anvil doctor
$ anvil doctor --fix
$ anvil doctor --json | jq .checks
```

---

## anvil config

**Class:** Admin **Purpose:** Show, set, and convert Anvil project config.
**When to use:** To inspect the project config, change an enforcement rule mode,
or convert `.anvil.<ext>` (a leftover `.anvilrc` is still read as a fallback).

**When to use (`show`):** To print the effective project config and current rule
modes.

**When to use (`set`):** To set one enforcement rule mode in the project config.
Rules: `public-api-expansion`, `new-dependency-introduction`,
`cross-layer-violation`, `privilege-expansion`. Modes: `off`, `warn`, `enforce`.

**When to use (`convert`):** To rewrite the discovered project config as
`.anvil.<ext>` (`yaml`, `yml`, `json`, or `toml`). Never writes `.anvilrc`.

**Synopsis:** `anvil config <show|set <rule> <mode>|convert --to <fmt>>`

**Subcommands:**

| Subcommand           | Description                                                                                                                                                     |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `show`               | Show the effective Anvil config.                                                                                                                                |
| `set <rule> <mode>`  | Set a rule mode. Rules: `public-api-expansion`, `new-dependency-introduction`, `cross-layer-violation`, `privilege-expansion`. Modes: `off`, `warn`, `enforce`. |
| `convert --to <fmt>` | Convert the project config to another canonical format. Never writes `.anvilrc`.                                                                                |

**`--json` (all three subcommands):** each emits exactly one JSON document on
stdout on success — no banner, no trailing hint — so the whole stream parses.
Both flag placements work (`anvil --json config <verb>` and
`anvil config <verb> --json`). Human output is unchanged when `--json` is
absent.

| Invocation                | Document                                                                                                                                              |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `show --json`             | `config` (the discovered file, or `defaults`), `rule_modes` (one key per rule), `note` (the legacy-key deprecation warning, `null` when none applies) |
| `set --json`              | `rule`, `mode`, and `config` — the file the mode was written to                                                                                       |
| `convert --json`          | `source`, `destination`, `source_removed`                                                                                                             |
| `convert --stdout --json` | `format` and `converted` (the converted config text)                                                                                                  |

Without `--json` every one of these prints its human line instead; the rows
above describe the `--json` invocations only.

Three things a parser should know:

- There is no `rewrote_in_place` field. The in-place rewrite — the case whose
  human line reads `anvil: rewrote <path>` — appears as `source` equal to
  `destination` with `source_removed: false`, which keeps one stable three-field
  shape. `source_removed` is `true` only when `--remove-old` actually deleted a
  source at a different path.
- `--stdout --json` does **not** print the converted config raw. `--stdout`
  emits the config in the _target_ format, which would break the
  one-JSON-document contract for every non-JSON target, so the converted text
  travels inside the `converted` field instead.
- Destination spelling and metadata are intentionally distinct. `--to yml`
  writes `.anvil.yml`, while `--to yaml` writes `.anvil.yaml`; for either
  spelling, existing embedded `format` metadata is rewritten to canonical
  `yaml`. `migrate format --format yml|yaml` follows the same owned-write rule.
  The existing `convert --stdout --json` `format` field likewise reports
  `"yaml"` for either spelling; the stable three-field write-mode JSON envelope
  does not gain a `format` field.

**Exit codes:** 0 (success), 1 (error), 4 (config error)

**Examples:**

```
$ anvil config show
$ anvil config show --json
$ anvil config set cross-layer-violation warn
$ anvil config set cross-layer-violation warn --json
$ anvil config convert --to yaml
$ anvil config convert --to toml --remove-old --json
$ anvil config convert --to toml --stdout --json
```

---

## anvil drift

**Class:** User-explicit **Purpose:** Track architecture drift over time. **When
to use:** To capture architecture snapshots, compare them, or generate a drift
report against a baseline.

**Synopsis:** `anvil drift <snapshot|compare|report|list|migrate>`

**Subcommands:**

| Subcommand                        | Description                                                          |
| --------------------------------- | -------------------------------------------------------------------- |
| `snapshot [--name <name>]`        | Capture current state as a snapshot.                                 |
| `compare <snapshot1> <snapshot2>` | Compare two snapshots.                                               |
| `report [--since <snapshot>]`     | Generate a drift report.                                             |
| `list [--limit <n>]`              | List available snapshots.                                            |
| `migrate [--prune-backups]`       | Upgrade drift baselines and optionally prune older rollback backups. |

**Exit codes:** 0 (success), 1 (error or partial migration), 3 (auth required)

`drift migrate` writes a fresh rollback backup before migrating each older
baseline. It exits 1 after printing its normal report when any baseline is
skipped, including corrupt JSON, unreadable files, invalid schema versions,
future-schema baselines, or scan-cap omissions. `--prune-backups` is explicit:
it keeps the latest `.bak` generation for each live `snapshot-*.json` baseline
and ignores unrelated backup-like files.

**Examples:**

```
$ anvil drift snapshot --name before-refactor
$ anvil drift report
$ anvil drift compare snapshot-1 snapshot-2
$ anvil drift list
$ anvil drift migrate --prune-backups
```

---

## anvil edda

**Class:** User-explicit **Purpose:** List, show, and trace Edda canonical
memories. **When to use:** To inspect memories stored in `.anvil/edda/` — useful
for debugging APS state and understanding past decisions.

**Synopsis:** `anvil edda <list|show>`

**Subcommands:**

| Subcommand           | Description                                   |
| -------------------- | --------------------------------------------- |
| `list` (alias: `ls`) | List Edda memories with filtering.            |
| `show <id>`          | Show a single Edda memory with full metadata. |

**`list` flags:**

| Flag                   | Description                                             |
| ---------------------- | ------------------------------------------------------- |
| `--json`               | Output as JSON.                                         |
| `--type <TYPE>`        | Filter by memory type (comma-separated).                |
| `--status <status>`    | Filter by memory status. Default: `active`.             |
| `--confidence <LEVEL>` | Filter by confidence level(s): `low`, `medium`, `high`. |
| `--since <DURATION>`   | Filter by age: `30m`, `24h`, `7d`.                      |
| `--limit <n>`          | Maximum memories to display. Default: 20.               |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil edda list
$ anvil edda list --type decision --confidence high
$ anvil edda show mem_abc123
```

---

## anvil ember

**Class:** User-explicit **Purpose:** List Ember proposals awaiting promotion to
Edda. **When to use:** To inspect proposal backlog items stored in the local
Ember database before promoting or reconciling them into durable Edda memories.

**Synopsis:** `anvil ember <list>`

**Subcommands:**

| Subcommand           | Description                          |
| -------------------- | ------------------------------------ |
| `list` (alias: `ls`) | List Ember proposals with filtering. |

**`list` flags:**

| Flag                | Description                                 |
| ------------------- | ------------------------------------------- |
| `--json`            | Output as JSON.                             |
| `--type <TYPE>`     | Filter by proposal type.                    |
| `--status <status>` | Filter by proposal status. Default: active. |
| `--limit <n>`       | Maximum proposals to display. Default: 20.  |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil ember list
$ anvil ember list --status active --limit 10
$ anvil ember list --json
```

---

## anvil exception

**Class:** User-explicit **Purpose:** Grant, revoke, and inspect tracked policy
exceptions. **When to use:** To record an intentional, scoped, expiring policy
deviation in the repo-tracked store (`anvil/exceptions/store.json`) so it
travels with the repository and is visible in PR review — or to revoke, list,
and verify existing grants. Writes are explicit-only; checks never write this
store.

**Synopsis:** `anvil exception <grant|revoke|list|show|verify|migrate>`

**Subcommands:**

| Subcommand                    | Description                                                                       |
| ----------------------------- | --------------------------------------------------------------------------------- |
| `grant`                       | Grant a scoped, attributed exception. Refuses unattributed records.               |
| `revoke <id> --reason <TEXT>` | Revoke a grant, preserving the audit trail (soft delete); `--reason` is required. |
| `list`                        | List exceptions with their verdicts.                                              |
| `show <id>`                   | Show a single exception in full.                                                  |
| `verify`                      | List plus per-verdict summary counts; always exits zero.                          |
| `migrate`                     | Copy the legacy local store into the tracked store (explicit, one-time).          |

**`grant` flags:**

| Flag                    | Description                                                      |
| ----------------------- | ---------------------------------------------------------------- |
| `--policy <ID>`         | Policy or rule identifier the exception suppresses.              |
| `--reason <TEXT>`       | Justification recorded on the grant.                             |
| `--scope <GLOB>`        | Glob scope (default: empty, meaning every file — warned about).  |
| `--owner <TEXT>`        | Accountable team or owner; git identity is used when omitted.    |
| `--expires-in-days <N>` | Days until the grant lapses. Omitted means never (warned about). |
| `--finding-hash <HASH>` | Pin the grant to one concrete finding instance.                  |

**Exit codes:** 0 (success), 1 (error, including a write that did not persist on
a read-only worktree — re-run with `--verbose` for the underlying error)

**Notes:** attribution (`created_by`/`revoked_by`) comes from local git config
and is advisory — reviewers should check it against PR authorship. Grants are
enforced at the L4 gate **only once the store is committed**: gates read
`anvil/exceptions/store.json` from the pushed commit's tree, never the worktree,
so grant → commit → push. An unattributed legacy grant only ever downgrades a
finding to a warning, it never silently suppresses it.

**Examples:**

```
$ anvil exception grant --policy AP-001 --reason "legacy module" --scope "src/legacy/**" --expires-in-days 30
$ anvil exception list
$ anvil exception revoke exc_1a2b3c4d5e6f708192a3b4c5 --reason "module removed"
$ anvil exception verify --json
```

---

## anvil status

**Class:** User-explicit **Purpose:** Show project status and health. **When to
use:** Daily check of Anvil's protection state for the current repo. Also useful
in CI as a health probe.

**Synopsis:** `anvil status [--verify] [--why]`

**Flags:**

| Flag       | Description                                                                             |
| ---------- | --------------------------------------------------------------------------------------- |
| `--verify` | Run a non-mutating activation probe — reports protection state without touching config. |
| `--why`    | Print per-tier activation evidence to stderr (requires `--verify`).                     |

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil status
$ anvil status --verify
$ anvil status --verify --why
$ anvil status --json | jq .protection_state
```

---

## bare `anvil` (no subcommand)

**Class:** Setup / daily **Purpose:** Daily ensure surface — turn protection on
without reinstall. **When to use:** After the repo is activated; re-run to
ensure the save-time daemon is up, re-attest the worktree, and refresh
already-owned MCP entries. Does **not** install NotPresent MCP clients,
workflows, or hooks — use `anvil start` for that.

**Synopsis:** `anvil` · `anvil --json`

**Behaviour (ADR-114):**

| Situation                          | Result                                                                            |
| ---------------------------------- | --------------------------------------------------------------------------------- |
| Project config absent              | Exit 1; recovery names `anvil start` / `anvil welcome` (no silent install)        |
| Not a registerable git worktree    | Exit 1; refuse early (worktree validation fails closed)                           |
| Activated worktree                 | Daemon ensure + worktree registration + MCP ensure-only; short confidence summary |
| MCP never installed / declined     | One recovery line → `anvil start` (no re-offer picker)                            |
| `ANVIL_NO_DAEMON` / `ANVIL_NO_MCP` | Skip that ensure step                                                             |

**Exit codes:** 0 (success), 1 (not activated or ensure failure), 3 (auth
required)

**Examples:**

```
$ anvil
$ anvil --json
$ anvil --help
```

---

## anvil start

**Class:** Setup **Purpose:** Activate or **reconfigure** Anvil in this
repository. **When to use:** First-time setup, adding MCP after a prior decline,
or repairing activation after a config change. Writes `.anvil.yaml` (or the
`--format` choice) if no project config exists and can install MCP / workflow
consent items. For a quiet daily on-switch without reinstall, use bare `anvil`
instead.

**Synopsis:**
`anvil start [--verify] [--watch] [--format <fmt>] [--new-identity] [--why]`

**Flags:**

| Flag             | Description                                                                                                                                                        |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `--verify`       | Run a read-only activation probe instead of writing files.                                                                                                         |
| `--watch`        | After activation, run the save-time watch fallback when MCP cannot pre-write attach.                                                                               |
| `--format <fmt>` | Config file format for first-run `.anvil.<ext>`: `yaml`, `yml`, `json`, or `toml`. Default: `yaml`. Ignored (with a warning) when a project config already exists. |
| `--new-identity` | Mint a fresh project UUID (use after forking a repo). Incompatible with `--verify`.                                                                                |
| `--why`          | Print per-tier activation evidence to stderr alongside the normal verdict.                                                                                         |

**Exit codes:** 0 (success), 1 (error), 4 (config error)

**Common errors:**

- `ready_restart_required`: MCP client must be restarted to pick up the new
  config — restart your editor.
- `needs_action`: run `anvil doctor` to identify the missing step.

**Examples:**

```
$ anvil start
$ anvil start --verify
$ anvil start --format yaml
$ anvil start --new-identity
```

---

## anvil tutorial

**Class:** Setup **Purpose:** Run an interactive guided tutorial. **When to
use:** First time using Anvil, or to revisit the onboarding flow.

**Synopsis:** `anvil tutorial [--reset]`

**Flags:**

| Flag      | Description              |
| --------- | ------------------------ |
| `--reset` | Reset tutorial progress. |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil tutorial
```

---

## anvil welcome

**Class:** Setup **Purpose:** Show the welcome screen with quick-start options.
**When to use:** To access the Anvil main menu and onboarding options.

**Synopsis:** `anvil welcome [--reset]`

**Flags:**

| Flag      | Description                                                  |
| --------- | ------------------------------------------------------------ |
| `--reset` | Reset onboarding state and re-run the first-time experience. |

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil welcome
```

---

## anvil init

**Class:** Setup **Purpose:** Initialise Anvil configuration for a project.
**When to use:** To create the canonical `.anvil.<ext>` configuration
(`.anvil.yaml` by default; the wizard can pick JSON/TOML) in a repo that doesn't
have one yet (an existing config, including a legacy `.anvilrc`, makes init bail
without `--force`; `--force` replaces it and removes a legacy `.anvilrc`).

**Synopsis:** `anvil init [--force]`

**Flags:**

| Flag      | Description                                         |
| --------- | --------------------------------------------------- |
| `--force` | Overwrite existing configuration without prompting. |

**Exit codes:** 0 (success), 1 (error), 3 (auth required), 4 (config error)

**Examples:**

```
$ anvil init
```

---

## anvil insights

**Class:** User-explicit **Purpose:** Show local-only weekly activity insights.
**When to use:** To review your weekly Anvil activity and suppression health
without a network dependency.

**Synopsis:** `anvil insights [--suppressions | --drift]`

**Flags:**

| Flag             | Description                                                                                     |
| ---------------- | ----------------------------------------------------------------------------------------------- |
| `--suppressions` | Show the suppression health view — stale `@anvil-ignore` directives first.                      |
| `--drift`        | Show the drift trend — new cross-boundary edges per week over the last 8 weeks, as a sparkline. |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil insights
$ anvil insights --suppressions
$ anvil insights --json
```

---

## anvil kindling

**Class:** User-explicit **Purpose:** Query local command-invocation usage
views. **When to use:** To answer "what is being used and what is not" from the
local usage sidecar and daemon-backed Kindling observations without ad-hoc SQL
or `jq`.

**Synopsis:** `anvil kindling usage <top|unused|flags|principals>`

**Subcommands:**

| Subcommand         | Description                                             |
| ------------------ | ------------------------------------------------------- |
| `usage top`        | Show top commands by invocation count.                  |
| `usage unused`     | Show registered commands that have not been invoked.    |
| `usage flags`      | Show flag-dependent command paths exercised in the log. |
| `usage principals` | Show anonymised principals grouped by activity level.   |

**`usage top` flags:**

| Flag                | Description                               |
| ------------------- | ----------------------------------------- |
| `--period <period>` | Count window: `week`, `month`, or `all`.  |
| `--limit <n>`       | Maximum rows to show. `0` means no limit. |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil kindling usage top
$ anvil kindling usage top --period week --limit 20
$ anvil kindling usage unused
$ anvil kindling usage flags --json
```

> Usage views are signal, not evidence; see
> [`docs/observability/usage-analytics.md`](../observability/usage-analytics.md).

---

## anvil telemetry

**Class:** User-explicit **Purpose:** Show or change anonymous usage telemetry
consent. **When to use:** To audit exactly what the fleet beacon would send,
turn telemetry on or off, or rotate the anonymous install id.

**Synopsis:** `anvil telemetry [on|off|reset-id]`

**Subcommands:**

| Subcommand | Description                                                         |
| ---------- | ------------------------------------------------------------------- |
| _(none)_   | Print the on/off state, the send gate, and the dimension allowlist. |
| `on`       | Turn anonymous usage telemetry on for this user.                    |
| `off`      | Turn anonymous usage telemetry off for this user (persisted).       |
| `reset-id` | Rotate the anonymous install id to a fresh random one.              |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil telemetry
$ anvil telemetry off
$ anvil telemetry reset-id
$ anvil telemetry --json
```

> `ANVIL_TELEMETRY=off` and `DO_NOT_TRACK=1` always override persisted consent,
> and gated (`ANVIL_HOME`) or non-interactive first runs that never showed the
> disclosure notice do not beacon. The consent posture and the exact dimension
> allowlist are decided in
> [ADR-107](../../plans/decisions/107-fleet-telemetry-consent-posture.md).

---

## anvil report-fp

**Class:** User-explicit **Purpose:** Record or inspect a local false-positive
report. **When to use:** When a check flags something that is not actionable and
you want to keep local evidence for support or later review.

**Synopsis:**
`anvil report-fp [--list] <check-id> <file:line> [--include-snippet]`

**Flags:**

| Flag                | Description                                                                                         |
| ------------------- | --------------------------------------------------------------------------------------------------- |
| `--list`            | List locally recorded reports as check ID, hashed path, line, and timestamp. Supports `--json`.     |
| `--include-snippet` | Opt in to storing the single source line for a new report. Not valid with `--list`; off by default. |

**Privacy posture:** reports are local-only (ADR-089). The file path is stored
and listed as a salted hash, never plaintext. `--list` does not print snippets,
even when a report was recorded with `--include-snippet`.

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil report-fp ANV-CORE-001 src/main.rs:42
$ anvil report-fp --list
$ anvil --json report-fp --list
```

---

## anvil migrate

**Class:** Admin **Purpose:** Migrate legacy config surfaces onto the canonical
`.anvil.<ext>` project config. **When to use:** To convert a legacy `.anvilrc`,
reconcile the config schema after upgrading Anvil (including the legacy
`camelCase` → `snake_case` key rename), fold a retired
`.anvil/gate-config.json`, or move a standalone architecture file under the
`architecture` section.

**Synopsis:** `anvil migrate [format|schema|gate-config|architecture]`

**Subcommands:**

| Subcommand     | Description                                                                                                                                                                        |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `format`       | Migrate a legacy `.anvilrc` to `.anvil.<ext>` (yaml/yml/json/toml).                                                                                                                |
| `schema`       | Reconcile an existing config's schema across Anvil versions (dry-run by default); the registered casing migration fires for pre-`0.10.0-beta` projects on `0.10.0-beta`+ binaries. |
| `gate-config`  | Fold a retired `.anvil/gate-config.json` into the main config (absent fields only), then remove it (dry-run by default).                                                           |
| `architecture` | Record a standalone `.anvil/architecture.yaml` as the config's `architecture.source` (dry-run by default).                                                                         |

**`format` flags:**

| Flag           | Description                                              |
| -------------- | -------------------------------------------------------- |
| `--format <f>` | Target format. Default: `yaml`.                          |
| `--force`      | Overwrite an existing `.anvil.<ext>` file.               |
| `--remove-old` | Remove the legacy `.anvilrc` after writing the new file. |

**`schema` / `gate-config` / `architecture` flags:**

| Flag                               | Description                                                                                                        |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `--apply`                          | Write the migration (default is dry-run preview).                                                                  |
| `--accept-weakening` (gate-config) | Confirm a fold that would deselect currently-effective checks (enforcement-weakening folds are refused otherwise). |

**Exit codes:** 0 (success), 1 (error), 4 (config error)

**Examples:**

```
$ anvil migrate format --format yaml
$ anvil migrate schema
$ anvil migrate schema --apply
$ anvil migrate gate-config --apply
$ anvil migrate architecture --apply
```

---

## anvil intercept

**Class:** Background (start/ensure) / Admin (stop/status) **Purpose:** Manage
the Anvil intercept daemon. **When to use:** To start, stop, inspect, or unblock
the local intercept daemon that enables pre-write MCP validation.

**Synopsis:** `anvil intercept <start|status|unblock|stop>`

**Subcommands:**

| Subcommand | Description                                                                                  |
| ---------- | -------------------------------------------------------------------------------------------- |
| `start`    | Start the intercept daemon in the foreground. `--foreground` is required.                    |
| `status`   | Print the daemon's status snapshot (sessions, fences, latency).                              |
| `unblock`  | Clear fence state from the daemon.                                                           |
| `stop`     | Stop the per-user daemon recorded in the daemon PID file; idempotent when no daemon is live. |

**When to use (`start`):** To start the intercept daemon in this terminal.
`--foreground` is required; use `anvil start` or `anvil watch` when you want the
daemon ensured in the background.

**`start` flags:**

| Flag           | Description                                                                                                   |
| -------------- | ------------------------------------------------------------------------------------------------------------- |
| `--foreground` | Required. Stay in the foreground; logs stream to stdout/stderr. This command does not background the process. |

**`status` flags:**

| Flag     | Description                                  |
| -------- | -------------------------------------------- |
| `--json` | Emit the raw JSON-RPC `query_status` result. |

**`unblock` flags:**

| Flag                    | Description                                                       |
| ----------------------- | ----------------------------------------------------------------- |
| `--worktree <PATH>`     | Remove a single worktree's fence record from the daemon.          |
| `--all`                 | Clear every fenced worktree in one call.                          |
| `--dry-run`             | Print what would be cleared without modifying daemon state.       |
| `--acknowledge-cascade` | Confirm intent to clear a `degraded:fence-cascade` engaged state. |

**Exit codes:** 0 (success), 1 (error)

**Common errors:**

- Daemon not running: start with `anvil intercept start --foreground`.

**Examples:**

```
$ anvil intercept start --foreground
$ anvil intercept status
$ anvil intercept status --json
$ anvil intercept unblock --worktree /path/to/worktree
$ anvil intercept unblock --all --dry-run
$ anvil intercept stop
```

---

## anvil workspace

**Class:** Admin **Purpose:** Manage the save-time daemon's workspace admission
(which project roots the `anvil-intercept` daemon will serve). **When to use:**
To switch the daemon between `open` (first-touch adopt) and `allowlist` mode, or
to curate the `allowlist` of served roots (confinement), and to register
worktrees for durable daemon protection.

**Synopsis:**
`anvil workspace <mode|allow|deny|register|unregister|list|install-hook>`

**Subcommands:**

| Subcommand     | Description                                                                                                    |
| -------------- | -------------------------------------------------------------------------------------------------------------- |
| `mode`         | Set admission mode: `open` (first-touch adopt, default) or `allowlist`.                                        |
| `allow`        | Add an allow entry (exact by default; `--prefix` confines a subtree). Allowlist only.                          |
| `deny`         | Remove an allow entry by path.                                                                                 |
| `register`     | Register a worktree for durable daemon protection (ACTMO-015). With no PATH, registers the current worktree.   |
| `unregister`   | Unregister a worktree's durable protection (ACTMO-015). Idempotent.                                            |
| `list`         | Show the admission mode, allow entries, and registered worktrees.                                              |
| `install-hook` | Install a guided `git` alias so newly-created worktrees auto-register (ACTMO-020). Adds a portable `sh` alias. |

**`register` flags:**

| Flag        | Description                                                                         |
| ----------- | ----------------------------------------------------------------------------------- |
| `PATH`      | Worktree to register (positional; defaults to the current directory).               |
| `--all`     | Register every exact allowlist entry that is a live, unfenced worktree (ACTMO-018). |
| `--persist` | Also record the worktree in `register_on_start` so the daemon re-registers it.      |

**`unregister` flags:**

| Flag        | Description                                                             |
| ----------- | ----------------------------------------------------------------------- |
| `PATH`      | Worktree to unregister (positional; defaults to the current directory). |
| `--persist` | Also remove the worktree from `register_on_start`.                      |

**`install-hook` flags:**

| Flag              | Description                                                         |
| ----------------- | ------------------------------------------------------------------- |
| `--alias <ALIAS>` | Name of the Git alias to install (default `wt-add`).                |
| `--print`         | Print the alias (and the PowerShell equivalent) without installing. |

> **Registration does not grant admission.** In `allowlist` mode a registered
> worktree is still served only if it is also an allow entry — add it with
> `anvil workspace allow <path>`. See
> [`operations/config.md`](../public/anvil/operations/config.md) for the
> `register_on_start` persistence and downgrade-safety details.

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil workspace list
$ anvil workspace mode allowlist
$ anvil workspace allow /path/to/project --prefix
$ anvil workspace register /path/to/project --persist
$ anvil workspace register --all --persist
$ anvil workspace unregister /path/to/project --persist
$ anvil workspace install-hook --print
```

---

## anvil l4-validate

**Class:** Background (CI lane) **Purpose:** Validate commits against policy
using the L4 rule engine. **When to use:** In CI pipelines that don't sit inside
git's pre-push hook and need to validate an explicit commit range against
`anvil/policy.yml`.

**Synopsis:** `anvil l4-validate <RANGE> [--branch <name>] [--repo <PATH>]`

**Flags:**

| Flag              | Description                                                                     |
| ----------------- | ------------------------------------------------------------------------------- |
| `RANGE`           | Commit range: `<base>..<head>` or bare `<head>` SHA (ancestry walk).            |
| `--branch <name>` | Branch name for policy resolution. Defaults to `git symbolic-ref --short HEAD`. |
| `--repo <PATH>`   | Repo root override. Defaults to the current working directory.                  |

**Exit codes:** 0 (clean), 2 (one or more commits blocked), 3 (engine
unavailable on every commit), 1 (error)

**Examples:**

```
$ anvil l4-validate origin/main..HEAD
$ anvil l4-validate HEAD~5 --branch feature/my-branch
$ anvil l4-validate abc123..def456 --repo /path/to/repo
```

---

## anvil licenses

**Class:** User-explicit **Purpose:** Show Anvil's acknowledgements and
third-party licence attribution. **When to use:** To inspect bundled dependency
licences, e.g. for compliance review.

**Synopsis:** `anvil licenses [--format <plain|markdown>]`

**Flags:**

| Flag             | Description                                                                              |
| ---------------- | ---------------------------------------------------------------------------------------- |
| `--format <fmt>` | Output format: `plain` (version banner + ACKNOWLEDGEMENTS, default) or `markdown` (raw). |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil licenses
$ anvil licenses --json
```

---

## anvil mcp-config

**Class:** Setup / Admin **Purpose:** Generate MCP server configuration for AI
clients. **When to use:** To preview, write, or verify the Anvil MCP entry for a
specific supported client and scope, including compatibility scripts that still
use this older command surface.

Planned: `anvil mcp-config` will eventually consolidate into `anvil mcp config`
(subsystem rename per the CLI surface coherence spec).

**Synopsis:**
`anvil mcp-config --target <client> [--scope <global|project>] [--transport <t>] [--port <n>] [--write] [--verify] [--workspace <path>] [--yes]`

**Flags:**

| Flag                 | Description                                                                                            |
| -------------------- | ------------------------------------------------------------------------------------------------------ |
| `--target <client>`  | Registered client target. Use `anvil mcp-config --help` for the current values.                        |
| `--scope <scope>`    | Resolve `global` (default) or `project` configuration. Zed is project-only.                            |
| `--transport <t>`    | Transport: `stdio` (default), or legacy `http` for Claude Code and Cursor.                             |
| `--port <n>`         | Port for `--transport http`. Default: `7616`.                                                          |
| `--write`            | Write the generated config to the target's well-known path.                                            |
| `--verify`           | Inspect the existing config entry without writing.                                                     |
| `--workspace <path>` | Override the selected scope root for resolving config paths.                                           |
| `--yes`              | Skip the "outside workspace root" confirmation (required for non-interactive writes to a custom path). |

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil mcp-config --target cursor
$ anvil mcp-config --target codex --write
$ anvil mcp-config --target zed --scope project --verify
$ anvil mcp-config --target cursor --transport http --port 7616 --write
```

---

## anvil mcp

**Class:** Background (serve) / User-explicit (install, refresh, pin)
**Purpose:** Manage and serve MCP integrations. **When to use:** `serve` is
invoked by editors automatically as `anvil mcp serve --stdio`. `install` wires
the MCP config for a supported client. Daily `anvil`, `anvil start`, and
`anvil doctor` rewrite drifted owned entries and poke live children when the
CLI, daemon, or configs are stale. `refresh` is the emergency cascade. `pin` /
`unpin` freeze or resume that daily self-heal (`ANVIL_MCP_PIN` is the session
override).

**When to use (`serve`):** To serve anvil's MCP tools over stdin/stdout. Editors
launch this automatically as `anvil mcp serve --stdio`. `--stdio` is required.

**When to use (`install`):** To preview, write, or verify anvil MCP
configuration for a supported editor.

**When to use (`refresh`):** Emergency cascade after an upgrade when daily
self-heal is not enough, or when heal is pinned and you still want a rewrite.

**When to use (`pin` / `unpin`):** Freeze daily MCP updates if you do not want
auto-heal; unpin to resume.

**Synopsis:** `anvil mcp <install|serve|refresh|pin|unpin>`

**Subcommands:**

| Subcommand                  | Description                                                     |
| --------------------------- | --------------------------------------------------------------- |
| `install --client <client>` | Install Anvil MCP configuration for a client.                   |
| `serve --stdio`             | Serve anvil MCP tools over stdin/stdout. `--stdio` is required. |
| `refresh`                   | Emergency bulk rewrite, daemon recycle, and live poke.          |
| `pin [version]`             | Freeze daily MCP self-heal and in-process recycle.              |
| `unpin`                     | Resume daily MCP self-heal.                                     |

**`refresh` flags:**

| Flag                 | Description                                                                                                                                                                                                     |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--dry-run`          | Preview config, daemon, and generation actions without mutating them. With `--processes orphan-reap`, lists orphans and does not signal.                                                                        |
| `--clients <ids>`    | `all` (default), `detected`, or one or more registry ids.                                                                                                                                                       |
| `--daemon <mode>`    | `auto` (default), `restart`, or `reuse`.                                                                                                                                                                        |
| `--processes <mode>` | `report` (default, no signals), `orphan-reap` (opt-in: SIGTERM same-user `anvil mcp serve --stdio` children whose parent PID is gone), or `none`. `force-skewed` is rejected. Live parents are never signalled. |
| `--workspace <path>` | Override the client config roots.                                                                                                                                                                               |

**`install` flags:**

| Flag                 | Description                                                                      |
| -------------------- | -------------------------------------------------------------------------------- |
| `--client <client>`  | Registered client target. Use `anvil mcp install --help` for the current values. |
| `--scope <scope>`    | Install globally (default) or into the project. Zed is project-only.             |
| `--verify`           | Verify the existing client config instead of writing it.                         |
| `--dry-run`          | Preview the resolved destination and entry without writing.                      |
| `--command <path>`   | Override the command path written into stdio configs.                            |
| `--workspace <path>` | Override the selected scope root.                                                |

**Exit codes:** 0 (success), 1 (error), 3 (auth required for `install`)

**Examples:**

```
$ anvil mcp install --client codex
$ anvil mcp install --client zed --scope project
$ anvil mcp install --client copilot-cli --verify
$ anvil mcp serve --stdio
$ anvil mcp refresh --dry-run
$ anvil mcp refresh --processes orphan-reap
$ anvil mcp pin
$ anvil mcp pin 0.9.2-beta
$ anvil mcp unpin
```

---

## anvil lsp

**Class:** Background (serve), experimental **Purpose:** Serve a minimal
Language Server Protocol surface for mid-edit diagnostics. **When to use:**
invoked by an LSP-speaking editor or agent client, not run manually in normal
use. Experimental — not production-hardened.

**Synopsis:** `anvil lsp --stdio`

**Flags:**

| Flag      | Description                  |
| --------- | ---------------------------- |
| `--stdio` | Serve LSP over stdin/stdout. |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil lsp --stdio
```

---

## anvil skill

**Class:** Setup / Admin **Purpose:** Install and verify the beta Anvil Agent
Skill bundle. **When to use:** To make Anvil's developer-function guidance
available to one or more detected agent harnesses without fetching the private
skill catalogue.

**Synopsis:**
`anvil skill install [--client <client>]... [--scope <global|project>]`

With an interactive terminal and no `--client`, Anvil detects supported
harnesses and asks which ones to install into. Scope remains a choice and
defaults to global. Non-interactive callers must select clients explicitly;
omitting `--client` is an error by design. Scripted fleets enumerate every
destination by repeating `--client` rather than silently writing to all detected
harnesses.

**`install` flags:**

| Flag                 | Description                                             |
| -------------------- | ------------------------------------------------------- |
| `--client <client>`  | Client to install into. Repeat to select more than one. |
| `--scope <scope>`    | Install globally (default) or into the current project. |
| `--workspace <path>` | Override the selected scope root.                       |
| `--verify`           | Verify the installed managed bundle without writing.    |
| `--dry-run`          | Preview resolved destinations without writing.          |

**Safety:** Anvil records per-file hashes and bundle provenance. Repeat
installation is idempotent; unmanaged, modified, or symlinked destinations are
refused rather than overwritten. Move refused unmanaged content outside the
relevant skills directory tree (or to another path the client does not scan)
before retrying.

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil skill install --client codex
$ anvil skill install --client claude-code --client opencode --scope project
$ anvil skill install --client codex --verify
```

See
[`docs/public/anvil/integrations/skills.md`](../public/anvil/integrations/skills.md).

---

## anvil plan

**Class:** Internal **Purpose:** Inspect Anvil's own APS planning state. **When
to use:** Internal developers browsing active APS work items and module status
in a read-only dashboard while dogfooding Anvil.

**Access (CIB-046):** the APS dashboard is gated behind the
`tui-dashboard.aps-dashboard` feature flag (catalogue group `tui-dashboard`,
audience `staff-internal-developer`) and is **default-disabled**. It opens for a
caller who sets `ANVIL_DEV=1` (local development) or a non-empty
`ANVIL_ADMIN_KEY` (the same credential `anvil admin` uses); otherwise the
command refuses with exit code 3 (authentication required). Plumbing a
staff-axis audience signal from `/auth/verify` so the flag can target
`staff-internal-developer` for an authenticated caller is a deferred follow-up.

**Synopsis:** `anvil plan dashboard`

**Subcommands:**

| Subcommand  | Description                                    |
| ----------- | ---------------------------------------------- |
| `dashboard` | Show active APS work in a read-only dashboard. |

**Exit codes:** 0 (success), 1 (error), 3 (gate closed — set `ANVIL_DEV=1` or
`ANVIL_ADMIN_KEY`)

**Examples:**

```
$ ANVIL_DEV=1 anvil plan dashboard
$ ANVIL_DEV=1 anvil plan dashboard --json
```

---

## anvil dashboard

**Class:** User-explicit **Purpose:** Open a native read-only dashboard over
local Anvil state. **When to use:** To browse architecture health, drift
snapshots, or suppression state in an interactive TUI.

**Synopsis:** `anvil dashboard [<name>]`

**Flags:**

| Flag     | Description                                                                         |
| -------- | ----------------------------------------------------------------------------------- |
| `<name>` | Dashboard to open: `architecture`, `drift`, or `suppressions`. Omit for the picker. |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil dashboard
$ anvil dashboard architecture
$ anvil dashboard drift
$ anvil dashboard suppressions
```

---

## anvil impact

**Class:** User-explicit **Purpose:** Open an interactive boundary/impact graph
of the current repository — the crate-level used-import graph derived from the
warm graph snapshot, with drill-down into a crate's direct neighbourhood or its
internal module graph. **When to use:** To read the repository's real dependency
structure (imports actually used, not Cargo declarations), scope the blast
radius of a change, or orient in an unfamiliar codebase.

**Synopsis:** `anvil impact`

**Flags:** none beyond the global set. `--json` prints the crate-level graph as
one JSON document; `--no-tui` (or a non-interactive stdout) prints a text
summary. Keyboard: arrows select, `enter` drills into a neighbourhood, `i` into
a crate's internals, `z` zooms to read a dense graph, `+`/`-`/`0`/`f` zoom,
`esc` backs out, `q` quits.

The view is read-only and needs a warm graph snapshot, which appears after the
anvil daemon has scanned the repository; without one (or with a graph too large
to lay out) the surface names the state on screen instead of rendering an empty
canvas.

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil impact
$ anvil impact --json
$ anvil impact --no-tui
```

---

## anvil new

**Class:** Setup **Purpose:** Scaffold a new project from a template. **When to
use:** To bootstrap a new project with Anvil pre-configured.

**Synopsis:**
`anvil new [<template-id>] [--list] [--category <c>] [--output <path>] [--force] [--var <key=value>]`

**Flags:**

| Flag                     | Description                                                   |
| ------------------------ | ------------------------------------------------------------- |
| `<template-id>`          | Template to scaffold from (omit for the interactive browser). |
| `--list` / `-l`          | List all available templates without launching the browser.   |
| `--category <c>` / `-c`  | Filter templates by category.                                 |
| `--output <path>` / `-o` | Output file path (default: `<template-id>.md`).               |
| `--force` / `-f`         | Overwrite an existing output file.                            |
| `--var <key=value>`      | Set a template variable (repeatable).                         |

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil new
```

---

## anvil wizard

**Class:** Setup **Purpose:** Run a guided project setup wizard. **When to
use:** For interactive onboarding when `anvil start` or `anvil init` alone isn't
enough.

**Synopsis:** `anvil wizard`

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil wizard
```

---

## anvil admin

**Class:** Admin **Purpose:** Run administrative commands (approvals, user
management). **When to use:** Operator tasks: approving waitlist signups,
inviting beta users, revoking tokens, browsing the audit log. Requires
`ANVIL_ADMIN_KEY`.

**Synopsis:**
`anvil admin <list|show|approve|invite|revoke|audit|send-migration|email-update|auth>`

See `docs/runbooks/admin-cli.md` for the full operator runbook including
credential handling, per-operator keys, and troubleshooting.

**Exit codes:** 0 (success), 1 (command failed), 3 (auth required —
`ANVIL_ADMIN_KEY` missing or invalid)

**Examples:**

```
$ anvil admin list
$ anvil admin approve alice@example.com
$ anvil admin invite alice@example.com --name "Alice"
$ anvil admin audit --action user.approved
```

---

## anvil gate

**Class:** User-explicit / Background (CI) **Purpose:** Run gate checks against
the current project. **When to use:** Pre-commit, pre-push (via hooks), CI
pipelines, or on-demand code quality checks with full profile support.

**Synopsis:**
`anvil gate [plan] [--profile <p>] [--skip-checks <checks>] [--only-checks <checks>] [--fail-fast] [--progress] [--format <fmt>]`

**Flags:**

| Flag                     | Description                                                                                                                                                                                        |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `plan`                   | Plan file to run gates against (positional; omit for full codebase scan).                                                                                                                          |
| `--profile <p>`          | Gate profile: `dev`, `ci`, `production`, `ai`.                                                                                                                                                     |
| `--skip-checks <checks>` | Comma-separated list of checks to skip.                                                                                                                                                            |
| `--only-checks <checks>` | Run only these checks (comma-separated canonical names or aliases). `architecture` is an alias of `import-boundaries`; results use the canonical name. `secret` is an alias of `secret-detection`. |
| `--fail-fast`            | Stop on first check failure.                                                                                                                                                                       |
| `--progress`             | Show real-time progress.                                                                                                                                                                           |
| `--format <fmt>`         | Output format: `auto`, `tui`, `plain`, `json`, `sarif`.                                                                                                                                            |

**Exit codes:** 0 (all checks passed), 2 (one or more checks failed), 1 (error),
3 (auth required)

**Examples:**

```
$ anvil gate
$ anvil gate --profile ci
$ anvil gate --only-checks secret-detection,antipattern-scan
$ anvil gate --only-checks architecture
$ anvil gate --format sarif > results.sarif
```

`--only-checks architecture` selects the `import-boundaries` check; gate output
uses that canonical name.

---

## anvil gate-config

**Class:** Admin **Purpose:** Configure gate check settings and thresholds.
**When to use:** To enable, disable, or list gate checks. Reads and writes the
main project config (`.anvil.<ext>`): the top-level `checks` list carries
selection and the `gate` section carries composition. The legacy
`.anvil/gate-config.json` is retired — gate runs ignore it, this command never
reads or writes it, and `anvil doctor` warns on a stray file (fold it with
`anvil migrate gate-config`). Disabling `secret-detection` or `command-safety`
is refused (their rule classes are hard-pinned for safety and cannot be turned
off here).

Planned: `anvil gate-config` will eventually consolidate into
`anvil gate config` (subsystem rename per the CLI surface coherence spec).

**Synopsis:**
`anvil gate-config [--list] [--enable <check>] [--disable <check>]`

**Flags:**

| Flag                | Description                      |
| ------------------- | -------------------------------- |
| `--list` / `-l`     | List current gate configuration. |
| `--enable <check>`  | Enable a specific check.         |
| `--disable <check>` | Disable a specific check.        |

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil gate-config --list
$ anvil gate-config --enable import-boundaries
$ anvil gate-config --disable antipattern-scan
```

---

## anvil watch

**Class:** User-explicit **Purpose:** Watch files and report save-time findings
after the baseline scan. **When to use:** During active development as a
real-time save-time feedback loop. Falls back gracefully when MCP pre-write
validation is not available.

**Synopsis:**
`anvil watch [--file <path>] [--action <action>] [--plans] [--source] [--all] [--patterns <globs>] [--exclude <globs>] [--debounce <ms>]`

**Flags:**

| Flag                       | Description                                                         |
| -------------------------- | ------------------------------------------------------------------- |
| `--file <path>` / `-f`     | File or directory to scope the watcher.                             |
| `--action <action>` / `-a` | Action on each change: `check` (default), `gate`, or `none`.        |
| `--plans`                  | Watch planning documents.                                           |
| `--source`                 | Watch source files.                                                 |
| `--all`                    | Watch everything except built-in noise/generated/cache directories. |
| `--patterns <globs>`       | Glob patterns to watch (comma-separated).                           |
| `--exclude <globs>`        | Glob patterns to exclude (comma-separated).                         |
| `--debounce <ms>`          | Debounce interval in milliseconds.                                  |

**Exit codes:** 0 (clean exit), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil watch
$ anvil watch --action gate
$ anvil watch --file src/ --patterns "**/*.rs"
$ anvil watch --all --exclude "vendor/**"
```

---

## anvil export

**Class:** User-explicit **Purpose:** Export constraints and configuration.
**When to use:** To export suppression lists, plan files, or configuration in
various formats for external tooling.

**Synopsis:**
`anvil export [source] [--to <fmt>] [--format <fmt>] [--output <path>] [--compact]`

**Flags:**

| Flag                     | Description                                                   |
| ------------------------ | ------------------------------------------------------------- |
| `source`                 | Source file path (for plan conversion, positional).           |
| `--to <fmt>`             | Target format: `aps`, `json`, `yaml`.                         |
| `--format <fmt>`         | Output format: `llms.txt`, `mcp-resource`, `prompt-fragment`. |
| `--output <path>` / `-o` | Output file path.                                             |
| `--compact`              | Compact JSON output.                                          |

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil export --format llms.txt
$ anvil export plan.aps.md --to json
```

---

## anvil hooks

**Class:** Admin / Setup **Purpose:** Install and manage git hooks. **When to
use:** To install or remove Anvil's pre-commit and pre-push git hooks in a
repository.

Planned: `anvil hooks` (plural) will eventually align with `anvil hook`
(singular subsystem) per the CLI surface coherence spec.

**Synopsis:** `anvil hooks <install|uninstall|status>`

**`install` flags:**

| Flag                | Description                                                |
| ------------------- | ---------------------------------------------------------- |
| `--force` / `-f`    | Overwrite existing hooks.                                  |
| `--pre-commit-only` | Only install the pre-commit hook.                          |
| `--pre-push-only`   | Only install the pre-push hook.                            |
| `--husky`           | Install hooks in the `.husky` directory.                   |
| `--config`          | Install via Git 2.54 native `hook.<event>.command` config. |

**`uninstall` flags:**

| Flag                | Description                                                                         |
| ------------------- | ----------------------------------------------------------------------------------- |
| `--pre-commit-only` | Only remove the pre-commit hook.                                                    |
| `--pre-push-only`   | Only remove the pre-push hook.                                                      |
| `--config`          | Remove Git 2.54 native `hook.<event>.command` config entries instead of hook files. |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil hooks install
$ anvil hooks install --force --husky
$ anvil hooks install --config
$ anvil hooks uninstall
$ anvil hooks status
```

---

## anvil hook

**Class:** Background **Purpose:** Runtime hook subcommands invoked by the shell
wrapper. **When to use:** These are called by git hooks automatically — not
invoked directly by users. `bootstrap` is the exception: run it after a fresh
clone to recover hook-runtime files.

**Synopsis:**
`anvil hook <pre-commit|pre-push|post-commit|post-merge|post-rewrite|bootstrap>`

**Subcommands:**

| Subcommand     | Description                                                                 |
| -------------- | --------------------------------------------------------------------------- |
| `pre-commit`   | L3 pre-commit hook — validates the staged diff and appends a witness line.  |
| `pre-push`     | L4 pre-push hook — walks the pushed commit range and applies branch policy. |
| `post-commit`  | Records that the commit succeeded.                                          |
| `post-merge`   | Appends a DAG-aware witness for merge joins.                                |
| `post-rewrite` | Regenerates witnesses for amended or rebased commits.                       |
| `bootstrap`    | Recover hook-runtime files in a worktree that hasn't been bootstrapped yet. |

**`bootstrap` flags:**

| Flag               | Description                                                            |
| ------------------ | ---------------------------------------------------------------------- |
| `--dry-run`        | Print the plan rather than executing.                                  |
| `--witness-recent` | Walk `<remote>..HEAD` after bootstrap and write retroactive witnesses. |

**Exit codes:** 0 (proceed / success), 2 (block decision or internal error), 1
(error)

**Examples:**

```
$ anvil hook bootstrap
$ anvil hook bootstrap --dry-run
$ anvil hook bootstrap --witness-recent
```

---

## anvil baseline

**Class:** Setup (one-shot) / User-explicit (re-runs) **Purpose:** Manage the
`anvil/baseline.json` adoption record. **When to use:** When first adopting
Anvil in a repo with existing findings, or to refresh/verify the baseline after
a significant code change.

**Synopsis:**
`anvil baseline [--refresh] [--new-identity] [--accept-suspicious] [--scan-budget <n>] [verify]`

**Flags:**

| Flag                          | Description                                                           |
| ----------------------------- | --------------------------------------------------------------------- |
| `--refresh`                   | Refresh an existing baseline at HEAD; updates `created_at`.           |
| `--new-identity`              | Mint a fresh project UUID (use after forking a repo).                 |
| `--suspicion-ratio <f>`       | Override the adversarial-refresh drop-ratio threshold (default 0.75). |
| `--suspicion-min-removed <n>` | Override the minimum-removed gate (default 10).                       |
| `--accept-suspicious`         | Acknowledge that a large finding drop is intentional.                 |
| `--scan-budget <n>`           | Cap the number of files scanned per invocation (default 50000).       |

**Subcommands:**

| Subcommand | Description                                            |
| ---------- | ------------------------------------------------------ |
| `verify`   | Re-read `anvil/baseline.json` and report its contents. |

**Exit codes:** 0 (clean), 2 (security-class finding refuses to grandfather), 1
(scan failed)

**Examples:**

```
$ anvil baseline
$ anvil baseline --refresh
$ anvil baseline verify
$ anvil baseline --new-identity
$ anvil baseline --refresh --accept-suspicious
```

---

## anvil capsule

**Class:** User-explicit (on-demand) / Background (CI) **Purpose:** Create,
verify, explain, and prune review capsules — file-first, inspectable governance
evidence directories for a commit range (GITGOV;
[ADR-074](../../plans/decisions/074-review-capsule-v0-format.md)). **When to
use:** To package a commit range's witness/policy/baseline evidence for review
or audit, to verify or summarise a capsule's closed-state verdict, and to stage
explicit retention cleanup for old in-repo capsule evidence.

**Synopsis:** `anvil capsule <create|verify|explain|prune>`

**Subcommands:**

| Subcommand | Description                                                                                                          |
| ---------- | -------------------------------------------------------------------------------------------------------------------- |
| `create`   | Create a review capsule directory for a commit range.                                                                |
| `verify`   | Verify a capsule directory and print closed-state verdicts; re-collects digests from the repo.                       |
| `explain`  | Print a human-readable summary of a capsule (range, policy, witness coverage, verdict). Read-only, repo-independent. |
| `prune`    | Plan or stage deletion of old in-repo capsule directories; dry-run by default and never commits.                     |

**`--json` (`verify`, `explain`):** emit machine-readable output for CI instead
of the human text. `verify --json` prints the `anvil.capsule-verification.v1`
document (verdict + per-check results); the ADR-074 exit code is unchanged, so a
CI step gates on the exit status and parses the verdict from the same run.
`explain --json` prints an `anvil.capsule-explain.v1` summary (range, evidence
states, recorded verdict).

**`--json` (`create`):** the global `--json` flag makes `create` print an
`anvil.capsule-create.v1` report instead of its two prose lines — `out`, `range`
(full base/head SHAs), `commit_count`, `file_count`, and `verify_command`. The
capsule written is identical either way.

**Exit codes (`verify`):** 0 (pass/warn), 1 (block), 2 (degraded), 3 (error).
`explain` exits 0 on success regardless of the recorded verdict — gate on the
verdict with `anvil capsule verify`, not `explain`.

**Flags (`prune`):**

| Flag              | Description                                                                                                        |
| ----------------- | ------------------------------------------------------------------------------------------------------------------ |
| `--root <dir>`    | Staging root to prune; defaults to `anvil/evidence/capsules/`. Must stay inside the repository and outside `.git`. |
| `--keep-last <n>` | Keep the newest N orderable capsules; required and must be at least 1.                                             |
| `--apply`         | Stage the selected deletions with `git rm`. Without this flag, `prune` is a dry run and touches nothing.           |

**Examples:**

```
$ anvil capsule create --range main..HEAD --out ./capsule-dir
$ anvil --json capsule create --range main..HEAD --out ./capsule-dir
$ anvil capsule verify ./capsule-dir
$ anvil capsule verify --json ./capsule-dir
$ anvil capsule explain --json ./capsule-dir
$ anvil capsule prune --keep-last 10
$ anvil capsule prune --keep-last 10 --apply
```

---

## anvil architecture

**Class:** Admin **Purpose:** Manage architecture boundary definitions. **When
to use:** To validate or inspect the project's architecture definition. Both
subcommands resolve the main config's `architecture` section first (inline, or
delegated via `architecture.source`), falling back to a standalone
`.anvil/architecture.yaml` (legacy form, still valid); `--file` inspects an
explicit path instead. Save-time watch uses the same resolution: the main config
for an inline section, the delegated `source` file, or the standalone file when
no section is present.

**Synopsis:** `anvil architecture <validate|show>`

**Subcommands:**

| Subcommand              | Description                               |
| ----------------------- | ----------------------------------------- |
| `validate [--file <f>]` | Validate the architecture definition.     |
| `show [--file <f>]`     | Show the current architecture definition. |

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil architecture validate
$ anvil architecture show
$ anvil architecture validate --file .anvil/architecture.yaml
```

---

## anvil auth

**Class:** Admin **Purpose:** Authenticate with the Anvil service. **When to
use:** To log in, log out, check your current identity, or refresh an expired
session.

**Synopsis:** `anvil auth <login|logout|whoami|refresh>`

**Subcommands:**

| Subcommand | Description                                                                                                 |
| ---------- | ----------------------------------------------------------------------------------------------------------- |
| `login`    | Authenticate with the Anvil service. Use `--otp` for email OTP or `--edict` for early-access.               |
| `logout`   | Remove stored credentials.                                                                                  |
| `whoami`   | Show the current authenticated identity, naming the credential source and whether it was verified this run. |
| `refresh`  | Exchange the stored refresh token for a fresh licence without a full re-login.                              |

**Exit codes:** 0 (success), 1 (login failed), 3 (auth required — for `whoami`)

**`whoami` auth states (GH #2587):** the auth gate that runs before every gated
command reports the two failure states and exits 3 before `whoami`'s own output
is produced:

- **Not authenticated** — no stored credential found. The gate prints
  "Authentication required. Run `anvil auth login` to authenticate." (exit 3).
- **Session expired** — a stored credential exists but has lapsed. The gate
  prints "Session expired. Run `anvil auth login` to re-authenticate." (exit 3).
  When a refresh token is present the gate first attempts a silent refresh, so
  this appears only if no refresh token is available or the refresh did not
  succeed.

When a present, valid credential lets the gate through, `whoami` then reports
the identity — and, crucially, **where it came from** (GH #2587), so an identity
known without an explicit login this session is no longer surprising:

- **Authenticated** — the server verified the identity on this run; the
  `Source:` line names the credential (`stored credentials (<path>)` from a
  previous login, or the `ANVIL_LICENSE` environment variable).
- **Authenticated (offline …)** — the server was unreachable, so a cached
  credential is reported `verified: false` (not checked this run).

Agents probing auth should read `anvil auth whoami --json` (the `verified` and
`source` fields) or the exit code rather than assuming a login is needed — a
valid cached credential means no `anvil auth login` is required.

**Common errors:**

- Not logged in: run `anvil auth login`.
- Session expired: run `anvil auth refresh` first; if that fails, run
  `anvil auth login`.

**Examples:**

```
$ anvil auth login
$ anvil auth whoami
$ anvil auth whoami --json   # { ..., "source": "...", "verified": true }
$ anvil auth refresh
$ anvil auth logout
```

---

## anvil policy

**Class:** Admin **Purpose:** Manage and evaluate policies. **When to use:** To
install starter packs, validate pack manifests, evaluate Rego, run
trust-regression CI locally, or inspect active policies.

**Synopsis:**
`anvil policy <eval|eval-regression|attack-regression|probe-trends|list|explain|diff|validate|install|show|test>`

**Subcommands:**

| Subcommand                             | Description                                                                                 |
| -------------------------------------- | ------------------------------------------------------------------------------------------- |
| `eval`                                 | Evaluate a Rego policy against an input document.                                           |
| `eval-regression`                      | Run trust-regression eval suites against the persisted baseline (report-only).              |
| `attack-regression`                    | Run a prompt-attack regression pack and gate on the fail-policy verdict.                    |
| `probe-trends`                         | Show adversarial probe pass/fail trends by category from eval history.                      |
| `list [--category <c>] [--enabled]`    | List available policies.                                                                    |
| `explain <policy-id>`                  | Explain a specific policy.                                                                  |
| `diff <base> <head>`                   | Show policy differences.                                                                    |
| `validate [path]`                      | Validate a policy pack (`pack.yaml`, metadata, structure, tests).                           |
| `install [pack-id] [--list] [--force]` | Install a bundled starter pack into `.anvil/policies/`.                                     |
| `show [pack-id]`                       | Show a bundled starter pack without installing it.                                          |
| `test [path] [--list-files]`           | Run pack or free-form `*_test.rego` tests (non-zero on failure); `--list-files` lists only. |

Gate evaluation uses the regorus engine in-process; the Go `opa` binary is
reference-only for `opa test` parity. See `docs/guides/policy-validation.md`.

**Exit codes:** 0 (success), 1 (error), 3 (auth required)

**Examples:**

```
$ anvil policy install --list
$ anvil policy install anvil-baseline
$ anvil policy validate .anvil/policies/
$ anvil policy list
$ anvil policy eval --json policies/fixtures/example.rego
$ anvil gate --only-checks policy
```

---

## anvil gctx

**Class:** Config **Purpose:** Graph-context operator settings. **When to use:**
To view or change whether source-text snippets are shared with AI assistants for
this workspace. Identity-only graph context is always available; source-text
snippets are off by default and require an explicit, per-workspace opt-in.

**Synopsis:** `anvil gctx egress <enable|disable|status>`

**Subcommands:**

| Subcommand       | Description                                                      |
| ---------------- | ---------------------------------------------------------------- |
| `egress enable`  | Consent to sending source-text snippets for this workspace.      |
| `egress disable` | Revoke consent and revert to identity-only.                      |
| `egress status`  | Show the effective snippet-egress state and where it comes from. |

`egress enable` prints the consequence of sharing source text and asks for
confirmation; pass `--yes` (alias `--consent`) to acknowledge non-interactively.
Consent is recorded per workspace. The `ANVIL_GCTX_EGRESS` environment variable
overrides the persisted state per process (`1` forces snippets on, `0` is the
kill-switch).

All three verbs honour `--json` and emit the same document: `egress` (`enabled`
/ `identity-only`) and `source` (`env` / `config` / `default`). `enable` and
`disable` add `action` (`enabled` / `disabled` / `unchanged`). State and source
are always the **effective** ones after the write, so consent recorded under an
`ANVIL_GCTX_EGRESS=0` kill-switch still reports `identity-only`. `action` names
the verb that ran, not a state delta: an idempotent `disable` still reports
`disabled`, and `unchanged` appears only when an interactive operator declines
the confirmation — never in a non-interactive run, which fails closed instead.

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil gctx egress status
$ anvil gctx egress status --json
$ anvil gctx egress enable
$ anvil gctx egress enable --yes --json
$ anvil gctx egress disable
```

---

## anvil update

**Class:** User-explicit **Purpose:** Update Anvil to the latest version. **When
to use:** To check for or install a newer release. Package-manager installs
(Homebrew, Scoop, WinGet) remain owned by their package manager; anvil can run
the manager's fixed latest-version command after explicit consent.

**Synopsis:** `anvil update [--check] [--version <ver>] [--force] [--yes]`

**Flags:**

| Flag              | Description                                                                                      |
| ----------------- | ------------------------------------------------------------------------------------------------ |
| `--check`         | Check for updates without installing; always read-only, including with `--yes`.                  |
| `--version <ver>` | Install a specific version instead of latest; unsupported for package-manager-owned installs.    |
| `--force`         | Reinstall even if already on the latest version; unsupported for package-manager-owned installs. |
| `--yes` / `-y`    | Consent to run the detected package manager without an interactive prompt.                       |

For a Homebrew, WinGet, or Scoop install, interactive `anvil update` prints the
detected manager and exact command on stderr, then asks `Run it now? [y/N]`.
Only `y` or `yes` starts the manager; any other response or end-of-input exits
successfully without changing the installation. The allowlisted commands are:

| Manager  | Command                               |
| -------- | ------------------------------------- |
| Homebrew | `brew upgrade eddacraft/tap/anvil`    |
| WinGet   | `winget upgrade --id eddacraft.anvil` |
| Scoop    | `scoop update anvil`                  |

Scoop is PowerShell-based rather than a native `.exe`; anvil launches the fixed
displayed command with `powershell.exe -NoProfile -Command`. The executable,
arguments, and command text are static allowlist entries and never contain
user-supplied shell text.

Package-manager commands always select the manager's configured latest version.
Use `--yes` for CI and other non-interactive callers. JSON mode never prompts
and refuses package-manager execution without `--yes`; with consent it captures
manager output and emits one JSON document on stdout, without additional human
stderr. Human mode streams manager output directly, writes anvil-owned guidance
and completion messages to stderr, and leaves anvil-owned stdout empty. A
missing executable or non-zero manager exit fails the command and names the
attempted command.

**Exit codes:** 0 (success or already up to date, including Windows when no
update is needed), 1 (update available when `--check` is used, Windows
self-update decline when an update is needed, or install error)

**Examples:**

```
$ anvil update
$ anvil update --check
$ anvil update --yes
$ anvil update --yes --json
$ anvil update --version 0.6.1
```

---

## anvil uninstall

**Class:** Admin **Purpose:** Remove project Anvil state. **When to use:** To
clean up Anvil from a project, or use `--global` to remove user state and the
daemon. Safe to run even with expired or missing credentials.

**Synopsis:**
`anvil uninstall [--global] [--dry-run] [--yes] [--force] [--keep-mcp] [--keep-daemon]`

**Flags:**

| Flag               | Description                                               |
| ------------------ | --------------------------------------------------------- |
| `--global`         | Remove user state and the daemon as well.                 |
| `--dry-run` / `-n` | Preview what would be removed without deleting.           |
| `--yes` / `-y`     | Skip the interactive confirmation prompt.                 |
| `--force`          | Continue past per-step errors instead of stopping.        |
| `--keep-mcp`       | Do not edit MCP config files even when `--global` is set. |
| `--keep-daemon`    | Do not attempt to stop the running daemon.                |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil uninstall --dry-run
$ anvil uninstall
$ anvil uninstall --global
```

---

## anvil validate

**Class:** User-explicit **Purpose:** Validate an APS plan file (structure, task
format, hash integrity). **When to use:** To check a plan file before committing
or submitting it. Does not require authentication.

**Synopsis:** `anvil validate <file> [--format <fmt>] [--no-validate-hash]`

**Flags:**

| Flag                 | Description                                                    |
| -------------------- | -------------------------------------------------------------- |
| `<file>`             | Plan file path to validate (positional, required).             |
| `--format <fmt>`     | Explicitly specify the input format (bypasses auto-detection). |
| `--no-validate-hash` | Skip hash integrity validation.                                |

**Exit codes:** 0 (valid), 1 (invalid or error)

**Examples:**

```
$ anvil validate plans/modules/my-module.aps.md
$ anvil validate plans/index.aps.md
```

---

## anvil version

**Class:** User-explicit **Purpose:** Show install-method-aware version and
upgrade guidance. **When to use:** To check the current version, latest
available version, install method, and recommended upgrade command.

**Synopsis:** `anvil version [--offline] [--check]`

**Flags:**

| Flag        | Description                                                                      |
| ----------- | -------------------------------------------------------------------------------- |
| `--offline` | Skip the network probe for the latest release version.                           |
| `--check`   | Probe the releases feed for security advisories attached to the running version. |

**Exit codes:** 0 (success), 1 (error)

**Examples:**

```
$ anvil version
$ anvil version --offline
$ anvil version --check
$ anvil version --json
```

---

## Related

- Canonical spec: `plans/specs/2026-05-07-cli-surface-coherence.md`
- Admin operator runbook: `docs/runbooks/admin-cli.md`
- Output stream policy: `docs/guides/cli-output-streams.md`
