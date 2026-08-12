---
id: validation
title: CLI command reference
description: Look up the public commands in the APS 0.6 native CLI.
sidebar_position: 1
owner: DOCSYNC
---

# CLI command reference

This reference describes the native APS 0.6 command surface audited from the
released CLI definitions. Run `aps <command> --help` for the flags supported by
your installed version.

## Command index

| Command        | Purpose                                                                 | Changes project files?                         |
| -------------- | ----------------------------------------------------------------------- | ---------------------------------------------- |
| `aps init`     | Create an APS project through the wizard, flags, or saved configuration | Yes                                            |
| `aps setup`    | Add optional hooks, agent support, or tool integrations                 | Usually                                        |
| `aps update`   | Reconcile APS-managed templates and installed skills                    | Yes                                            |
| `aps migrate`  | Preview or apply migration from an older vendored runtime               | Only with `--apply`                            |
| `aps lint`     | Validate APS documents                                                  | No                                             |
| `aps next`     | Select ready work whose dependencies are complete                       | No                                             |
| `aps start`    | Mark one ready item in progress and write its context package           | Yes                                            |
| `aps complete` | Mark one in-progress item complete and optionally record a learning     | Yes                                            |
| `aps graph`    | Print work items and dependency arrows                                  | No                                             |
| `aps rollup`   | Print a current monorepo roll-up table                                  | No                                             |
| `aps audit`    | Compare plan state with project state                                   | No plan edits; may execute validation commands |
| `aps export`   | Emit an `aps-export/v1` JSON snapshot                                   | No                                             |
| `aps doctor`   | Diagnose the installed binary, project pin, and old runtime files       | No                                             |

## Global options

```bash
aps --version
aps --help
aps --strict lint
```

`--strict` turns a project CLI-version mismatch into a non-zero exit for
project-scoped commands.

## Initialisation and maintenance

```bash
aps init
aps init --non-interactive --profile team --shape monorepo
aps setup codex
aps update
aps doctor
aps migrate --dry-run
aps migrate --apply
```

`aps init --from <config.yml>` replays a previous initialisation selection. Use
`aps init --help` for selectable tools, templates, paths, components, hook
verbosity, and model preferences.

## Authoring

```bash
aps lint
aps lint plans/modules/auth.aps.md
aps lint plans --json
```

An explicit lint target wins over project discovery. Without one, APS finds the
nearest `.aps/config.yml` and reads its `plans_dir`.

## Orchestration

```bash
aps next
aps next auth
aps start AUTH-003
aps complete AUTH-003 --learning "A durable observation"
aps graph auth
```

`next`, `start`, `complete`, and `graph` accept `--plans <dir>`. Federated plans
also accept `--child <name>`; start and complete accept a qualified ID such as
`catalog:PROD-001`.

## Monorepo views

```bash
aps next --package core
aps next --by-package
aps rollup --by-package
aps rollup --plans packages
```

Package filters use a work item's `Packages` field, falling back to its module
metadata. Federated roll-up reads child plans linked from the root.

## Audit

```bash
aps audit --no-run
aps audit auth --json --no-run
aps audit --stale-days 30 --no-run
```

Omitting `--no-run` executes validation commands stored in completed work items.
Only do that for trusted plan content.

## Export

```bash
aps export --json
aps export --plans packages
```

JSON is the only export format; `--json` is accepted to make the intent
explicit.

## Plan-root resolution

For commands that operate on a plan tree, the order is:

1. an explicit target or `--plans` value;
2. the `APS_PLANS` environment variable;
3. `plans_dir` from the nearest `.aps/config.yml`; then
4. `plans/`.

See [run the APS workflow](../workflow.md) for the end-to-end sequence.
