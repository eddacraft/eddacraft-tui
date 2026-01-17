---
id: file-layout
title: File Layout
description: Conventions for organising APS files in a project.
sidebar_position: 2
---

# File Layout

Standard conventions for organising APS documents in your project.

## Recommended Structure

```
project/
├── plans/
│   ├── index.aps.md              # Root index
│   ├── modules/
│   │   ├── auth.aps.md           # Auth module
│   │   ├── payments.aps.md       # Payments module
│   │   └── notifications.aps.md  # Notifications module
│   ├── execution/
│   │   ├── AUTH-001.steps.md     # Detailed steps for AUTH-001
│   │   └── PAY-002.steps.md      # Detailed steps for PAY-002
│   └── decisions/
│       ├── ADR-001.md            # Architecture decision records
│       └── ADR-002.md
└── ...
```

## Directory Purposes

### `plans/`

Root directory for all planning documents. Keeps plans separate from code.

### `plans/modules/`

One file per module. Each file contains:

- Module metadata
- Task list
- Brief task descriptions

### `plans/execution/`

Detailed step breakdowns for complex tasks. Used when:

- Task has many steps
- Steps need extensive detail
- Multiple people working on same task

### `plans/decisions/`

Architecture Decision Records (ADRs) that inform the plan. Not strictly APS, but
useful context.

## File Naming

### Index

```
index.aps.md
```

Always named `index.aps.md` at the plans root.

### Modules

```
{module-name}.aps.md
```

Examples:

- `auth.aps.md`
- `payments.aps.md`
- `user-management.aps.md`

Use kebab-case for multi-word names.

### Execution Steps

```
{TASK-ID}.steps.md
```

Examples:

- `AUTH-001.steps.md`
- `PAY-002.steps.md`

Matches the task ID exactly.

## Minimal Layout

For small projects, a single file works:

```
project/
├── plans/
│   └── index.aps.md    # Contains everything
└── ...
```

Contents:

```markdown
---
format: aps
version: 1.0
---

# My Project

## Module: Core

### Task: CORE-001 — Initial setup

**Outcome:** Project builds successfully

**Validation:** `pnpm build`
```

## Growing the Structure

As projects grow, split into modules:

### Step 1: Extract Modules

```markdown
<!-- Before: inline module -->

## Module: Auth

### AUTH-001 — Login

### AUTH-002 — Registration

<!-- After: reference -->

## Modules

- [auth](modules/auth.aps.md)
```

### Step 2: Extract Steps

When tasks get complex:

```markdown
<!-- Before: inline steps -->

### Task: AUTH-001 — Login

**Steps:**

1. [ ] Endpoint exists
2. [ ] Validation works ... (20 more steps)

<!-- After: reference -->

### Task: AUTH-001 — Login

**Steps:** See [AUTH-001.steps.md](../execution/AUTH-001.steps.md)
```

## Version Control

### Commit Practices

```bash
# Good commit messages
git commit -m "plan(auth): add AUTH-003 password reset task"
git commit -m "plan(payments): mark PAY-001 complete"
git commit -m "plan: add notifications module"

# Convention: plan({module}): {description}
```

### Branching

For large planning changes:

```bash
git checkout -b plan/add-payments-module
```

### Code Review

Plans are code. Review them:

- Is the outcome clear?
- Is the validation correct?
- Are steps observable?

## Multi-Team Layouts

For large organisations:

```
plans/
├── index.aps.md
├── teams/
│   ├── platform/
│   │   ├── index.aps.md
│   │   └── modules/
│   ├── product/
│   │   ├── index.aps.md
│   │   └── modules/
│   └── infrastructure/
│       ├── index.aps.md
│       └── modules/
└── shared/
    └── modules/
        └── common.aps.md
```

---

**Next:** [Determinism rules →](/docs/aps/spec/determinism)
