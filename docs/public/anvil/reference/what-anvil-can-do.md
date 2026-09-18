---
id: what-anvil-can-do
title: What anvil can do
description:
  A 12-row index of shipped anvil capabilities, not the full check catalogue.
owner: DOCDEF
upstream:
  - crates/anvil-cli/src/commands/check.rs
  - crates/anvil-cli/src/commands/gate.rs
  - crates/anvil-cli/src/commands/check_catalog.rs
verified_against: 0.11.1-beta
---

# What anvil can do

This is a short index of what anvil does today. It is not the check catalogue
and it does not grow past these 12 rows.

| #   | Capability                                                                                                                                                                                                                                                                                                                                                                                                           | Status                   |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| 1   | anvil evaluates a project with **checks** and decides merge-readiness with a **gate**.                                                                                                                                                                                                                                                                                                                               | shipped                  |
| 2   | **When it runs:** pre-write/save-time is regex + secrets (no AST). Installed packs also evaluate on MCP pre-write, and on save-time when the CLI-started daemon injected the policy hook. Save-time does not interrupt. `anvil check` / MCP `anvil_check` add AST. Watch may print a non-blocking AST follow-up after allow. `anvil gate` / hooks / CI is the merge judgement. Default watch is `check`, not `gate`. | shipped                  |
| 3   | `anvil check` needs no plan file and runs only `secret-detection` and `antipattern-scan` (regex **and** AST). A green save is not that pass.                                                                                                                                                                                                                                                                         | shipped                  |
| 4   | `anvil gate` is the merge judgement and runs the full gate set. Do not treat default `anvil watch` as this.                                                                                                                                                                                                                                                                                                          | shipped                  |
| 5   | **Init-default checks:** `secret-detection`, `import-boundaries`, `antipattern-scan`.                                                                                                                                                                                                                                                                                                                                | shipped                  |
| 6   | Other catalogue engines (`policy`, `lint`, `test`, `coverage`, `dependency`, `command-safety`, `import-boundaries`) run under **gate**, not `check`.                                                                                                                                                                                                                                                                 | shipped                  |
| 7   | Four **surface checks** (`sql-migrations`, `github-actions`, `dockerfile`, `shell-scripts`) run in gate by default, are selected by flags not `checks:`, and warn unless `--fail-on-warnings`.                                                                                                                                                                                                                       | shipped-with-flag-status |
| 8   | Project file is `.anvil.yaml` (also `.yml` / `.json` / `.toml`).                                                                                                                                                                                                                                                                                                                                                     | shipped                  |
| 9   | **Gate profiles:** `dev`, `ci`, `production`, `ai`.                                                                                                                                                                                                                                                                                                                                                                  | shipped                  |
| 10  | Policy is a **gate check**; the same packs also emit MCP pre-write findings, and save-time diagnostics when the CLI-started daemon has the policy hook ([policy model](../concepts/policy-model.md)). Start with a bundled pack (`anvil-baseline` or `anvil-control-examples`). Authoring is the installed skill, not a public manual.                                                                               | shipped                  |
| 11  | Full command list: [CLI command reference](cli.md).                                                                                                                                                                                                                                                                                                                                                                  | shipped                  |
| 12  | Model: [How anvil evaluates a project](../concepts/evaluation-model.md).                                                                                                                                                                                                                                                                                                                                             | shipped                  |

For the model behind these rows, read
[how anvil evaluates a project](../concepts/evaluation-model.md). For every
public command, read the [CLI command reference](cli.md).

## Try it

- [Install and get first value](../quickstart.md)
- [Ten-minute protection tutorial](../first-gate.md)
