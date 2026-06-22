---
id: minimal-plan
title: Minimal Plan
description: The simplest possible APS plan.
sidebar_position: 1
---

# Minimal Plan

| Type      | Authority | Owner   | Status | Freshness                                              |
| --------- | --------- | ------- | ------ | ------------------------------------------------------ |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream           |
| ------------------------------------------------------------------------- | -------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

This example shows the smallest useful APS plan.

## Single-file plan

For small projects, one file is enough:

```markdown
# Todo App

A simple todo list application.

## Problem

Users need to track tasks.

## Success Criteria

- [ ] Users can add todos
- [ ] Users can list todos

## Work Items

### TODO-001: Initial setup

- **Intent:** Project structure created with working build
- **Expected Outcome:** `npm run build` exits 0
- **Validation:** `npm run build`

### TODO-002: Add todo endpoint

- **Intent:** POST /todos creates a new todo item
- **Expected Outcome:** Endpoint returns 201 with created todo
- **Validation:** `npm test -- todos.create.test.ts`

### TODO-003: List todos endpoint

- **Intent:** GET /todos returns all todos for user
- **Expected Outcome:** Endpoint returns paginated list filtered by user
- **Validation:** `npm test -- todos.list.test.ts`
```

## Why this works

Even this minimal plan provides:

### Clear outcomes

Each work item has a single, measurable outcome. You know when it is done.

### Validation commands

Tests prove completion. No ambiguity.

### Observable checkpoints

When you need more granularity, add an action plan — but simple items do not
require one.

## When to grow

Upgrade to multi-file when:

- More than 5–7 work items
- Multiple distinct features or modules
- Multiple people working concurrently

See [Multi-module example →](./multi-module.md)

## Template

Copy this template for new projects:

```markdown
# {Project Name}

## Problem
{Brief problem statement}

## Success Criteria
- [ ] {Observable outcome}

## Work Items

### {PREFIX}-001: {Title}

- **Intent:** {What success looks like}
- **Expected Outcome:** {Testable result}
- **Validation:** `{command}`
```

---

**Next:** [Multi-module example →](./multi-module.md)
