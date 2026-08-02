---
id: cli-reference
title: CLI command reference
description: Discover every public top-level anvil command.
---

<!-- Generated from shipped product sources. Do not edit by hand. -->

# CLI command reference

This page is generated from the command definitions shipped with anvil
0.9.1-beta. Use `anvil <command> --help` for flags, examples, and subcommands
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
