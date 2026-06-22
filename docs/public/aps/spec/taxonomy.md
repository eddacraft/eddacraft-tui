---
id: taxonomy
title: Taxonomy
description: The APS document hierarchy — Index, Module, Work Item, Action Plan.
sidebar_position: 1
---

# APS Taxonomy

| Type        | Authority | Owner   | Status | Freshness                                               |
| ----------- | --------- | ------- | ------ | ------------------------------------------------------- |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream            |
| ------------------------------------------------------------------------- | --------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

APS documents follow a hierarchy: **Index → Module → Work Item → Action Plan**.

## Hierarchy

```
┌─────────────────────────────────────────────────┐
│                     Index                        │
│  (Project-level, lists all modules)             │
└───────────────────────┬─────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
    ┌───────┐       ┌───────┐       ┌───────┐
    │Module │       │Module │       │Module │
    │   A   │       │   B   │       │   C   │
    └───┬───┘       └───┬───┘       └───┬───┘
        │               │               │
    ┌───▼───┐       ┌───▼───┐       ┌───▼───┐
    │Work   │       │Work   │       │Work   │
    │Items  │       │Items  │       │Items  │
    └───┬───┘       └───┬───┘       └───┬───┘
        │               │               │
    ┌───▼───┐       ┌───▼───┐       ┌───▼───┐
    │Action │       │Action │       │Action │
    │Plans  │       │Plans  │       │Plans  │
    └───────┘       └───────┘       └───────┘
```

## Level 1: Index

The root document that ties everything together.

### Purpose

- Lists all modules in the project
- Provides project-level metadata (problem, success criteria, milestones)
- Serves as entry point for validation

### Format

```markdown
# Project Name

## Overview

Brief description of the project.

## Problem & Success Criteria

**Problem:** [What problem are we solving?]

**Success Criteria:**

- [ ] [Measurable outcome]

## Modules

| Module                                | ID   | Owner | Status | Priority |
| ------------------------------------- | ---- | ----- | ------ | -------- |
| [auth](./modules/auth.aps.md)         | AUTH | @you  | Draft  | high     |
| [payments](./modules/payments.aps.md) | PAY  | @you  | Draft  | medium   |
```

The Index is **non-executable** — it describes intent, not implementation.

## Level 2: Module

A cohesive unit of functionality with bounded scope.

### Purpose

- Groups related work items
- Defines a bounded context with interfaces
- Maps to a feature, component, or crosscutting concern

### Format

```markdown
# Authentication Module

| ID   | Owner | Priority | Status |
| ---- | ----- | -------- | ------ |
| AUTH | @you  | high     | Draft  |

## Purpose

Handles user authentication and authorisation.

## In Scope

- Login/logout
- Registration

## Work Items

### AUTH-001: Login endpoint

- **Intent:** Users authenticate with email and password
- **Expected Outcome:** POST /auth/login returns JWT for valid credentials
- **Validation:** `npm test -- auth.test.ts`
```

### Module metadata fields

| Field    | Required | Description                                            |
| -------- | -------- | ------------------------------------------------------ |
| ID       | Yes      | 2–6 uppercase chars (AUTH, PAY, CORE)                  |
| Owner    | No       | Person or team responsible                             |
| Priority | No       | `low`, `medium`, `high`                                |
| Status   | Yes      | `Draft`, `Ready`, `In Progress`, `Complete`, `Blocked` |
| Packages | No       | Affected packages (monorepo only)                      |
| Type     | No       | `Conductor` for crosscutting modules                   |

## Level 3: Work Item

A unit of authorised work — execution authority.

### Required fields

| Field            | Description                        |
| ---------------- | ---------------------------------- |
| ID               | `PREFIX-NNN` (e.g., `AUTH-001`)    |
| Intent           | What the work item aims to achieve |
| Expected Outcome | Testable or observable result      |
| Validation       | Command to verify completion       |

### Optional fields

| Field        | Description                                  |
| ------------ | -------------------------------------------- |
| Status       | `Draft`, `Ready`, `In Progress`, `Complete`  |
| Dependencies | Other work item IDs that must complete first |
| Confidence   | `low`, `medium` (default), `high`            |
| Scope        | What will change                             |
| Non-scope    | What will not change                         |
| Files        | Best-effort list of affected files           |
| Risks        | Potential risks                              |

### Work item ID format

Pattern: `{PREFIX}-{NNN}`

- `AUTH-001`, `AUTH-002` — Authentication
- `PAY-001` — Payments
- `CORE-001` — Core/shared

PREFIX is 1–10 uppercase alphanumeric characters. NNN is a 3-digit zero-padded
number.

## Level 4: Action Plan

An optional execution breakdown for complex work items.

### Purpose

- Breaks down complex work items into actions
- Provides progress visibility with checkpoints
- Enables wave-based parallel execution

### Format

```markdown
# Action Plan: AUTH-001

| Field     | Value                     |
| --------- | ------------------------- |
| Work Item | AUTH-001 — Login endpoint |
| Status    | Draft                     |

## Actions

### Action 1 — Create login endpoint

**Checkpoint** Endpoint accepts POST /auth/login

**Validate** `npm test -- auth.login.test.ts`
```

### Action rules

**Do:**

- Describe observable state in checkpoints
- Use verifiable conditions
- Keep checkpoints under ~12 words

**Do not:**

- Include implementation details
- Reference specific libraries or code structure
- Use subjective criteria

## Relationships

### Work item dependencies

```markdown
### AUTH-002: Registration

- **Dependencies:** AUTH-001, CORE-001
```

### Module dependencies

Declared in the Index module table or module interfaces:

```markdown
## Interfaces

**Depends on:**

- session — token management
```

---

**Next:** [File layout →](./file-layout.md)
