---
id: monorepo
title: Plan a monorepo
description: Choose between one tagged plan and independently owned child plans.
sidebar_position: 2
owner: DOCSYNC
verified_against: 0.6.0
---

# Plan a monorepo

APS supports two monorepo shapes. Start with the lighter one and adopt
federation only when ownership demands it.

## Choose a tier

| Signal                                       | Tagged plan              | Federated child plans            |
| -------------------------------------------- | ------------------------ | -------------------------------- |
| Packages share one backlog                   | Best fit                 | Unnecessary overhead             |
| Work commonly spans packages                 | Best fit                 | Useful only with separate owners |
| Packages have independent owners or releases | Can become crowded       | Best fit                         |
| A package may move to another repository     | Plan must be split later | Child plan moves with it         |

## Tier 1: one tagged plan

Keep one `plans/` directory at the repository root. Add a `Packages` column to
module metadata:

```markdown
| ID   | Owner | Priority | Status | Packages  |
| ---- | ----- | -------- | ------ | --------- |
| AUTH | @team | high     | Ready  | core, api |
```

A work item can narrow that default:

```markdown
### AUTH-002: Add the API login endpoint

- **Status:** Ready
- **Packages:** api
- **Intent:** Expose login through the API.
- **Expected Outcome:** Valid credentials create a session.
- **Validation:** `pnpm test --filter api`
```

Use package-aware queue and roll-up views:

```bash
aps next --package api
aps next --by-package
aps rollup --by-package
```

`W022` warns when a package tag does not resolve under the repository's package
or application directories.

## Tier 2: federated child plans

Use child plans when packages own separate backlogs and lifecycles:

```text
monorepo/
├── plans/index.aps.md
└── packages/
    ├── catalog/plans/index.aps.md
    └── storefront/plans/index.aps.md
```

The root index links children in a `Child Plans` section and holds a roll-up.
Each child remains a complete plan that can lint and execute independently.

Create a federated starting shape with:

```bash
aps init --non-interactive --shape monorepo --templates index-nested
```

Within one child, IDs stay short, such as `PROD-001`. Across children, qualify
the dependency with the child name, such as `catalog:PROD-001`.

```bash
aps lint plans
aps next --plans plans
aps next --child catalog --plans plans
aps start catalog:PROD-001 --plans plans
aps graph --plans plans
aps rollup --plans plans
```

The CLI rejects an ambiguous bare ID instead of changing the wrong child file.

## Keep generated views current

`aps rollup` prints Markdown derived from current module state. Paste the result
into the root roll-up or package view when plan state changes. The index stays
reviewable Markdown while the command supplies current rows.

## Migration path

Move one package at a time:

1. Create a standalone child plan beside the package.
2. Move that package's modules into the child.
3. Qualify dependencies that now cross child boundaries.
4. Link the child from the root and remove its old root module rows.
5. Run `aps lint` from the federation root and from the child.

Do not federate every package merely for symmetry. Mixed adoption is valid while
some packages still share the root backlog.
