---
id: cli-reference
title: CLI command reference
description: Discover every public top-level anvil command.
---

<!-- Generated from shipped product sources. Do not edit by hand. -->

# CLI command reference

This page is generated from the command definitions shipped with anvil
0.9.3-beta. Use `anvil <command> --help` for flags, examples, and subcommands
for your installed version.

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
| `anvil dashboard`    | Open a native read-only dashboard over local anvil state                  |
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
| `anvil report-fp`    | Report a false positive against a check                                   |
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

## `anvil start` flags

Activation entrypoint. Flags below are generated from the shipped CLI; confirm
with `anvil start --help` on your binary.

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
