---
id: file-layout
title: Project files and ownership
description:
  Know which APS files are generated, editable, optional, or temporary.
sidebar_position: 2
---

# Project files and ownership

`aps init` creates a small project contract and the planning structure selected
in the wizard or command flags. The exact optional directories vary by profile.

```text
project/
├── .aps/
│   ├── config.yml
│   └── context/
└── plans/
    ├── index.aps.md
    ├── aps-rules.md
    ├── project-context.md
    ├── modules/
    ├── execution/
    ├── designs/
    ├── decisions/
    └── releases/
```

## Files you own

| Path                           | Purpose                                           |
| ------------------------------ | ------------------------------------------------- |
| `plans/index.aps.md`           | Overall problem, success criteria, and module map |
| `plans/modules/*.aps.md`       | Module boundaries and work items                  |
| `plans/execution/*.actions.md` | Optional action plans                             |
| `plans/project-context.md`     | Project-specific workflow and technical context   |
| `plans/designs/*.design.md`    | Optional technical designs                        |
| `plans/decisions/*.md`         | Optional durable decision records                 |
| `plans/releases/v*.md`         | Optional release narratives                       |

Commit these files with the code they govern.

## APS-managed guidance

`plans/aps-rules.md` gives an AI assistant portable APS behaviour. Templates may
also be installed as hidden files so they do not look like active plan content.
`aps update` can refresh APS-managed templates and skills without rewriting your
plan documents.

## Project contract

`.aps/config.yml` records:

- the expected CLI version;
- the profile and project shape used by initialisation;
- the plan, documentation, and tooling paths;
- selected templates and components; and
- optional tool-integration choices.

Project-scoped commands discover this file by walking up from the current
directory.

## Temporary context

`aps start` writes `.aps/context/<WORK-ITEM-ID>.md`. The file is regenerated
from the current plan and is normally ignored by source control. Do not treat it
as a second source of truth.

## Naming conventions

| Document    | Convention                                     | Example                                   |
| ----------- | ---------------------------------------------- | ----------------------------------------- |
| Index       | `index.aps.md`                                 | `plans/index.aps.md`                      |
| Module      | kebab-case ending in `.aps.md`                 | `plans/modules/auth.aps.md`               |
| Action plan | work-item or module ID ending in `.actions.md` | `plans/execution/AUTH-003.actions.md`     |
| Design      | date and slug ending in `.design.md`           | `plans/designs/2026-07-20-auth.design.md` |
| Release     | version beginning with `v`                     | `plans/releases/v1.2.0.md`                |

## Custom plan locations

Set `plans_dir` in `.aps/config.yml`, or pass `--plans` to commands that expose
that option. `aps lint` accepts a file or directory as its positional target:

```bash
aps lint packages/catalog/plans
aps next --plans packages/catalog/plans
```

Use the [monorepo guide](../guides/monorepo.md) before creating several plan
roots; a single tagged plan is simpler when packages share one backlog.
