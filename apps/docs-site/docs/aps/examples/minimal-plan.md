---
id: minimal-plan
title: Minimal Plan
description: The simplest possible APS plan.
sidebar_position: 1
---

# Minimal Plan

This example shows the smallest useful APS plan.

## Single File Plan

For small projects, one file is enough:

```markdown
---
format: aps
version: 1.0
---

# Todo App

A simple todo list application.

## Task: TODO-001 — Initial setup

**Outcome:** Project structure created with working build

**Validation:** `pnpm build`

**Steps:**
1. [ ] package.json created
2. [ ] TypeScript configured
3. [ ] Build produces dist/

---

## Task: TODO-002 — Add todo endpoint

**Outcome:** POST /todos creates a new todo item

**Validation:** `pnpm test src/todos/create.test.ts`

**Steps:**
1. [ ] Endpoint accepts POST /todos
2. [ ] Request body validated
3. [ ] Todo stored in database
4. [ ] Response includes created todo

---

## Task: TODO-003 — List todos endpoint

**Outcome:** GET /todos returns all todos for user

**Validation:** `pnpm test src/todos/list.test.ts`

**Steps:**
1. [ ] Endpoint accepts GET /todos
2. [ ] Todos filtered by user
3. [ ] Response paginated
```

## Why This Works

Even this minimal plan provides:

### Clear Outcomes

Each task has a single, measurable outcome. You know when it's done.

### Validation Commands

Tests prove completion. No ambiguity.

### Observable Steps

Each step describes what should be true, not how to achieve it.

## When to Grow

Upgrade to multi-file when:

- More than 5-7 tasks
- Multiple distinct features
- Multiple people working

See [Multi-module example →](/docs/aps/examples/multi-module)

## Template

Copy this template for new projects:

```markdown
---
format: aps
version: 1.0
---

# {Project Name}

{Brief description}

## Task: {PREFIX}-001 — {Title}

**Outcome:** {What success looks like}

**Validation:** `{command}`

**Steps:**
1. [ ] {Observable state}
2. [ ] {Observable state}
```

---

**Next:** [Multi-module example →](/docs/aps/examples/multi-module)
