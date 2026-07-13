---
id: monorepo
title: Monorepo
description: Using APS in monorepos with multiple packages and applications.
sidebar_position: 1
docgov:
  type: 'Public docs'
  authority: 'Derived'
  owner: 'DOCSYNC'
  status: 'Live'
  freshness: 'Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0'
  upstream:
    '[anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**`'
  downstream: 'APS docs-site section'
---

# APS in Monorepos

This guide explains how to use APS effectively in monorepos with multiple
packages and applications.

## The problem

Standard APS assumes a single `plans/` directory for one project. In a monorepo:

```text
monorepo/
├── apps/
│   ├── api/
│   ├── web/
│   └── cli/
├── packages/
│   ├── core/
│   └── shared/
└── plans/
```

You will hit these pain points:

1. **Scope ambiguity** — Does AUTH-001 affect `api`, `web`, both, or `core`?
2. **Navigation difficulty** — Hard to find which module owns a work item
3. **Cross-cutting concerns** — Work spans multiple packages
4. **Prioritisation gaps** — No clear "what's next" view across the monorepo

## The solution

Keep a single `plans/` at the monorepo root. Add:

- **Explicit package tags** on modules and work items
- **"What's Next" view** in the index for a prioritised work queue
- **"By Package" view** in the index for navigation
- **Session rituals** to keep docs in sync

Use the
[`index-monorepo.template.md`](https://github.com/EddaCraft/anvil-plan-spec/blob/main/templates/index-monorepo.template.md)
template.

## Index structure

### What's Next section

```markdown
## What's Next

| #   | Work Item | Module | Packages    | Owner | Status |
| --- | --------- | ------ | ----------- | ----- | ------ |
| 1   | AUTH-002  | auth   | core, api   | @josh | Ready  |
| 2   | CLI-001   | cli    | cli, shared | @josh | Ready  |
```

### Modules by package section

```markdown
## Modules by Package

### apps/api

- [auth](./modules/01-auth.aps.md) — AUTH-002 ready

### packages/core

- [auth](./modules/01-auth.aps.md) — AUTH-002 ready
- [data](./modules/04-data.aps.md) — no ready items
```

## Module package tags

```markdown
| ID   | Owner | Priority | Status | Packages  |
| ---- | ----- | -------- | ------ | --------- |
| AUTH | @josh | high     | Ready  | core, api |
```

Work items can override the module default:

```markdown
### AUTH-002: API login endpoint

- **Packages:** api
- **Intent:** POST /auth/login returns JWT
```

## Custom plans directory

Monorepos with per-package plans can set `plans_dir` in `.aps/config.yml`:

```yaml
cli_version: 0.4.0
plans_dir: packages/foo/plans/
```

Or pass the plan root explicitly to commands that accept a target:

```bash
aps lint packages/foo/plans/
APS_PLANS=packages/foo/plans/ aps next
```

## Session rituals

At the start of each session:

1. Run `aps next` to find the highest-priority ready item
2. Check the "What's Next" table in the index
3. Review any open issues in `plans/issues.md`

At the end of each session:

1. Update work item statuses
2. Log discoveries in `plans/issues.md`
3. Run `aps lint plans/` before committing

---

**Next:** [AI Agents →](./ai-agents.md)
