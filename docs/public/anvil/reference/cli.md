---
id: cli-reference
title: CLI command reference
description:
  Discover every public top-level anvil command, plus flags and subcommands for
  the daily set.
owner: CLICT
upstream:
  - crates/anvil-cli/src/main.rs
  - crates/anvil-cli/src/commands/start.rs
  - crates/anvil-cli/src/commands/check.rs
  - crates/anvil-cli/src/commands/gate.rs
  - crates/anvil-cli/src/commands/config.rs
  - crates/anvil-cli/src/commands/watch.rs
  - crates/anvil-cli/src/commands/doctor.rs
  - crates/anvil-cli/src/commands/init.rs
  - crates/anvil-cli/src/commands/policy/mod.rs
  - scripts/docs/generate-anvil-public-reference.mjs
verified_against: 0.9.7-beta
---

<!-- Generated from shipped product sources. Do not edit by hand. -->

# CLI command reference

This page is generated from the command definitions shipped with anvil
0.9.7-beta. Global flags appear once. Hidden clap commands are unpublished.
Flags and subcommands below cover the daily set (`start`, `check`, `gate`,
`config`, `watch`, `doctor`, `init`, `policy`). Use `anvil <command> --help` for
other commands and for examples on your installed version.

For a first installation, use the [quickstart](../quickstart.md).

## Daily ensure

With no subcommand, bare `anvil` runs the daily ensure surface: it turns
protection on for an already-activated project (daemon + existing MCP entries).
It does not install clients you skipped or rewrite configuration — use
`anvil start` to activate or reconfigure.

| Command              | Purpose                                                                   |
| -------------------- | ------------------------------------------------------------------------- |
| `anvil`              | Turn protection on for an already-activated project (daily ensure)        |
| `anvil admin`        | Manage service approvals and users (administrators only)                  |
| `anvil architecture` | Manage architecture boundary definitions                                  |
| `anvil audit`        | Run a full project audit                                                  |
| `anvil audit-chain`  | Check commits that bypassed protection for missing evidence               |
| `anvil auth`         | Authenticate with the anvil service                                       |
| `anvil baseline`     | Manage the record of findings accepted when anvil was introduced          |
| `anvil capsule`      | Package review evidence for a commit range into a portable file           |
| `anvil check`        | Scan files for anti-patterns and hardcoded secrets (planless mode)        |
| `anvil config`       | Show, set, and convert anvil project config                               |
| `anvil dashboard`    | Open a native read-only dashboard over local anvil state (flag-gated)     |
| `anvil doctor`       | Run diagnostic checks on your environment                                 |
| `anvil drift`        | Track architecture drift over time                                        |
| `anvil edda`         | Inspect durable local memory records used by eddacraft workflows          |
| `anvil ember`        | Inspect proposed memory records before they become durable records        |
| `anvil exception`    | Manage recorded policy exceptions                                         |
| `anvil export`       | Export constraints and configuration                                      |
| `anvil gate`         | Run gate checks against the current project                               |
| `anvil gate-config`  | Set which checks and thresholds a gate uses                               |
| `anvil gctx`         | Control whether graph-context snippets may leave the local machine        |
| `anvil hook`         | Run Git-hook operations; normally invoked by anvil-managed hooks          |
| `anvil hooks`        | Install and manage git hooks                                              |
| `anvil init`         | Initialise anvil configuration for a project                              |
| `anvil insights`     | Show local-only weekly activity insights                                  |
| `anvil intercept`    | Manage the local process that protects supported AI-assisted writes       |
| `anvil kindling`     | Inspect the local command-usage record used for activity insights         |
| `anvil l4-validate`  | Validate a commit range against policy in continuous integration          |
| `anvil licenses`     | Show anvil's acknowledgements and third-party licence attribution         |
| `anvil lsp`          | Serve a minimal Language Server Protocol surface for mid-edit diagnostics |
| `anvil mcp`          | Manage Model Context Protocol (MCP) connections for supported AI clients  |
| `anvil mcp-config`   | Print MCP configuration for a supported AI client                         |
| `anvil migrate`      | Migrate anvil config to a new format or schema version                    |
| `anvil new`          | Scaffold a new project from a template                                    |
| `anvil plan`         | Inspect planning files written in APS, a Markdown-based plan format       |
| `anvil policy`       | Manage and evaluate policies                                              |
| `anvil report-fp`    | Report a false positive against a check or a printed finding id           |
| `anvil skill`        | Install and verify bundled Agent Skills                                   |
| `anvil start`        | Activate anvil in this repository                                         |
| `anvil status`       | Show project status and health                                            |
| `anvil telemetry`    | Show or change anonymous usage telemetry consent                          |
| `anvil tutorial`     | Interactive guided tutorial                                               |
| `anvil uninstall`    | Remove project anvil state; use `--global` for user state and daemon      |
| `anvil update`       | Update anvil to the latest version                                        |
| `anvil validate`     | Validate a planning file written in APS format                            |
| `anvil version`      | Show install-method-aware version + upgrade guidance                      |
| `anvil watch`        | Watch files and report save-time findings after the baseline scan         |
| `anvil welcome`      | Show the welcome screen with quick-start options                          |
| `anvil wizard`       | Guided project setup wizard                                               |
| `anvil workspace`    | Control which project folders the local protection process may access     |

## Global flags

These flags are available on every command. They are not repeated in the
per-command tables.

| Flag                    | Purpose                                                                                                                                                                      |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--json`                | Output results as JSON: success stdout is exactly one JSON document (or a documented machine stream), on every command                                                       |
| `--no-tui`              | Disable TUI rendering; use plain text output                                                                                                                                 |
| `--verbose`             | Enable verbose logging                                                                                                                                                       |
| `--anvil-home`          | Re-root install-owned state (user state, daemon socket/PID, kernel cache/logs) under this prefix so a pre-release candidate can run side-by-side with the production install |
| `--touch-project-state` | Permit durable per-project mutations (baseline refresh, witness append, cutoff pinning) while running under a non-default `--anvil-home` / `ANVIL_HOME`                      |

## Command flags and subcommands

### `anvil start`

Activate anvil in this repository.

| Flag                | Purpose                                                                                             |
| ------------------- | --------------------------------------------------------------------------------------------------- |
| `--verify`          | Run a non-mutating activation probe — skip init, first-scan, and the MCP install step               |
| `--watch`           | After activation, run the save-time watch fallback when MCP cannot pre-write attach                 |
| `--format`          | Pick a config file format for first-run activation                                                  |
| `--new-identity`    | Mint a fresh project UUID and record the previous one as `forked_from`                              |
| `--why`             | Print per-tier activation evidence to stderr alongside the normal verdict on stdout                 |
| `--no-daemon`       | Skip auto-starting the per-user save-time daemon                                                    |
| `--no-mcp`          | Skip MCP config installation                                                                        |
| `--all-mcp-clients` | Non-interactive: wire every supported MCP client even when that client is not detected on this host |
| `--mcp-client`      | Explicitly configure one or more MCP clients from the full registry                                 |
| `--mcp-scope`       | Scope for clients selected with --mcp-client (and first-wave install)                               |

Interactive `anvil start` offers every installable MCP client (unticked by
default). Scripted multi-client install uses `--mcp-client <id>` (repeatable),
`--all-mcp-clients`, and `--mcp-scope global|project`. Discover client ids with
`anvil mcp install --help`.

### `anvil check`

Scan files for anti-patterns and hardcoded secrets (planless mode).

| Argument | Purpose                                                                     |
| -------- | --------------------------------------------------------------------------- |
| `FILES`  | Files to analyse (optional if using --changed, --staged, --since, or --all) |

| Flag               | Purpose                                                                                                    |
| ------------------ | ---------------------------------------------------------------------------------------------------------- |
| `--changed`        | Analyse git-changed files only (ignored if explicit file paths are given)                                  |
| `--staged`         | Analyse only staged files (implies --changed; ignored if explicit file paths are given)                    |
| `--since`          | Compare against a git ref, e.g. main, HEAD~3 (implies --changed; ignored if explicit file paths are given) |
| `--all`            | Analyse all source files in the project                                                                    |
| `--extensions`     | Comma-separated file extensions to analyse (e.g. .ts,.tsx,.html)                                           |
| `--severity`       | Minimum severity for blocking: error, warning, info (default: error)                                       |
| `--include-opt-in` | Include opt-in patterns                                                                                    |
| `--artifact`       | Artifact kind: source, pr-description, commit-message, agent-output                                        |
| `--format`         | Output format: auto (default), tui, plain, json, or sarif                                                  |

### `anvil gate`

Run gate checks against the current project.

| Argument | Purpose                                                      |
| -------- | ------------------------------------------------------------ |
| `PLAN`   | Plan file to run gates against (omit for full codebase scan) |

| Flag                 | Purpose                                                            |
| -------------------- | ------------------------------------------------------------------ |
| `--profile`          | Gate profile: dev, ci, production, ai                              |
| `--skip-checks`      | Comma-separated list of checks to skip                             |
| `--only-checks`      | Only run these checks (comma-separated canonical names or aliases) |
| `--fail-fast`        | Stop on first check failure                                        |
| `--fail-on-warnings` | Treat warning-severity findings as blocking (exit non-zero)        |
| `--progress`         | Show real-time progress                                            |
| `--list-profiles`    | List available gate profiles                                       |
| `--format`           | Output format: auto (default), tui, plain, json, or sarif          |

### `anvil config`

Show, set, and convert anvil project config.

| Subcommand             | Purpose                                                |
| ---------------------- | ------------------------------------------------------ |
| `anvil config show`    | Show the effective anvil config                        |
| `anvil config set`     | Set a rule mode in the project config                  |
| `anvil config convert` | Convert the project config to another canonical format |

#### `anvil config set`

| Argument | Purpose                                                                                                               |
| -------- | --------------------------------------------------------------------------------------------------------------------- |
| `RULE`   | Rule to set: `public-api-expansion`, `new-dependency-introduction`, `cross-layer-violation`, or `privilege-expansion` |
| `MODE`   | Mode to apply: `off`, `warn`, or `enforce`                                                                            |

#### `anvil config convert`

| Flag           | Purpose                                                         |
| -------------- | --------------------------------------------------------------- |
| `--to`         | Destination format: yaml, yml, json, or toml                    |
| `--stdout`     | Print the converted config instead of writing `.anvil.<ext>`    |
| `--force`      | Overwrite an existing destination file                          |
| `--remove-old` | Delete the source file when the destination is a different path |

### `anvil watch`

Watch files and report save-time findings after the baseline scan.

| Flag                 | Purpose                                                                                                                                   |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `--file`             | File or directory to scope the watcher (when a file is given, its parent directory is watched; other files there may also trigger events) |
| `--action`           | Action to run on each change: check (default), gate, or none for an architecture/dependency-only watch with no code-quality scan          |
| `--plans`            | Watch planning documents                                                                                                                  |
| `--source`           | Watch source files                                                                                                                        |
| `--all`              | Watch everything except built-in local-noise/generated/cache directories                                                                  |
| `--patterns`         | Glob patterns to watch (comma-separated, e.g. "src/**/\*.ts,lib/**/\*.ts")                                                                |
| `--exclude`          | Glob patterns to exclude (comma-separated, e.g. "vendor/**,**/\*.test.ts")                                                                |
| `--debounce`         | Debounce interval in milliseconds                                                                                                         |
| `--no-daemon`        | Skip starting (or offering to start) the per-user save-time daemon                                                                        |
| `--save-time-driver` | Internal: run as the headless save-time driver the intercept daemon's supervisor spawns per registered worktree                           |
| `--worktree`         | Canonical worktree root to drive (save-time driver mode only)                                                                             |

### `anvil doctor`

Run diagnostic checks on your environment.

| Flag    | Purpose                        |
| ------- | ------------------------------ |
| `--fix` | Auto-fix issues where possible |

### `anvil init`

Initialise anvil configuration for a project.

| Flag      | Purpose                                            |
| --------- | -------------------------------------------------- |
| `--force` | Overwrite existing configuration without prompting |

### `anvil policy`

Manage and evaluate policies.

| Subcommand                       | Purpose                                                                                |
| -------------------------------- | -------------------------------------------------------------------------------------- |
| `anvil policy eval`              | Evaluate a Rego policy against an input document                                       |
| `anvil policy eval-regression`   | Run trust-regression eval suites and report regressions against the persisted baseline |
| `anvil policy attack-regression` | Run a prompt-attack regression pack and gate on the fail-policy verdict                |
| `anvil policy probe-trends`      | Show adversarial probe pass/fail trends by category from the eval history              |
| `anvil policy list`              | List available policies                                                                |
| `anvil policy explain`           | Explain a specific policy                                                              |
| `anvil policy diff`              | Show policy differences                                                                |
| `anvil policy validate`          | Validate a policy pack: manifest, metadata, structure, and tests                       |
| `anvil policy install`           | Install a bundled starter policy pack into `.anvil/policies/`                          |
| `anvil policy show`              | Show a bundled starter policy pack without installing it                               |
| `anvil policy test`              | Run policy tests                                                                       |

#### `anvil policy eval`

| Argument | Purpose                                     |
| -------- | ------------------------------------------- |
| `POLICY` | Path to the `.rego` policy file to evaluate |

| Flag                 | Purpose                                                                                        |
| -------------------- | ---------------------------------------------------------------------------------------------- |
| `--input`            | Path to a JSON `PolicyInput` document                                                          |
| `--query`            | Rego query to evaluate                                                                         |
| `--explain`          | Render line coverage for the evaluation                                                        |
| `--why`              | Explain a finding by its 0-based index: render the evaluation trace and highlight that finding |
| `--fail-on-warnings` | Treat warnings as blocking: exit non-zero on any non-baselined warning                         |

#### `anvil policy eval-regression`

| Flag                   | Purpose                                                                                                             |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `--suites`             | Path to a JSON file defining the suites to run (an array of eval suites: `{ "name", "policy", "query", "input"? }`) |
| `--store`              | Directory holding the eval history                                                                                  |
| `--anvil-bin`          | The `anvil` executable used to run each suite                                                                       |
| `--update-baseline`    | Append each run to the history, updating the baseline future runs compare against                                   |
| `--fail-on-regression` | Block (exit non-zero) when any suite regressed                                                                      |

#### `anvil policy attack-regression`

| Flag           | Purpose                                                                                                                    |
| -------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `--pack`       | Path to an attack pack YAML file (a set of scenario fixtures)                                                              |
| `--fail-above` | Block (exit non-zero) when a failing scenario's severity is strictly above this band (`low`, `medium`, `high`, `critical`) |

#### `anvil policy probe-trends`

| Flag      | Purpose                            |
| --------- | ---------------------------------- |
| `--store` | Directory holding the eval history |

#### `anvil policy list`

| Flag         | Purpose                    |
| ------------ | -------------------------- |
| `--category` | Filter by category         |
| `--enabled`  | Show only enabled policies |

#### `anvil policy explain`

| Argument    | Purpose              |
| ----------- | -------------------- |
| `POLICY_ID` | Policy ID to explain |

#### `anvil policy diff`

| Argument | Purpose          |
| -------- | ---------------- |
| `BASE`   | Base policy file |
| `HEAD`   | Head policy file |

#### `anvil policy validate`

| Argument | Purpose                                                   |
| -------- | --------------------------------------------------------- |
| `PATH`   | Pack manifest file, or a directory containing `pack.yaml` |

#### `anvil policy install`

| Argument  | Purpose                                                  |
| --------- | -------------------------------------------------------- |
| `PACK_ID` | Identifier of the bundled pack to install (see `--list`) |

| Flag          | Purpose                                                            |
| ------------- | ------------------------------------------------------------------ |
| `--list`      | List the bundled starter packs and exit                            |
| `--force`     | Overwrite existing pack files instead of refusing                  |
| `--workspace` | Workspace root to install into (defaults to the current workspace) |

#### `anvil policy show`

| Argument  | Purpose                                                       |
| --------- | ------------------------------------------------------------- |
| `PACK_ID` | Identifier of the bundled pack to show (see `install --list`) |

#### `anvil policy test`

| Argument | Purpose                |
| -------- | ---------------------- |
| `PATH`   | Test file or directory |

| Flag           | Purpose                    |
| -------------- | -------------------------- |
| `--list-files` | List discovered test files |

## Exit codes

Stable process exit codes used by the CLI. Scripts should gate on these values
rather than parsing human-readable prose.

| Code | Meaning                                                                                                                                   |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `0`  | Success                                                                                                                                   |
| `1`  | General error (recoverable user-action condition)                                                                                         |
| `2`  | Gate failure — fail-fast for CI and scripted gates                                                                                        |
| `3`  | Authentication required on an action command or auth probe (not used by read-only `status`, which exits 0 with an informational envelope) |
| `4`  | Configuration error                                                                                                                       |
| `5`  | Surface and daemon on different OS instances, or cross-boundary mixed configuration (reserved / future emission)                          |
| `6`  | Daemon not running and embedded fallback unavailable (reserved / future emission)                                                         |
| `7`  | CLI or hook protocol version mismatch with the daemon (reserved / future emission)                                                        |
| `10` | Runtime discovery failed (reserved / future emission)                                                                                     |

### Authentication-required behaviour

- **Action commands** (`anvil start`, bare `anvil`, `anvil init`, `anvil gate`,
  `anvil check`, `anvil watch`, and other gated mutating surfaces) exit **`3`**
  when authentication is required, so `&&` chains and script preflights stop at
  an unauthenticated or unactivated repo.
- **Read-only status** (`anvil status`) exits **`0`** when authentication is
  required and reports an informational `authRequired` envelope under `--json`.
  Auth-required is the expected answer on that state probe, not a failure.
- Auth state probes such as `anvil auth whoami` exit **`3`** so scripts can
  detect a missing login without treating it as a generic error `1`.
- Read-only activation probes (`anvil start --verify`, `anvil status --verify`)
  bypass the pre-dispatch auth wall entirely.

When `--json` is set, action-command auth-required responses use an
informational envelope (`state: "authRequired"`, `next`, optional
`earlyAccessUrl`) on stdout while still exiting `3`.
