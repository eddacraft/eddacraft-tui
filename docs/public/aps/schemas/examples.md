---
id: examples
title: Schema Examples
description: Example APS documents showing various patterns and structures.
sidebar_position: 2
---

# Schema Examples

| Type      | Authority | Owner   | Status | Freshness                                              |
| --------- | --------- | ------- | ------ | ------------------------------------------------------ |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream           |
| ------------------------------------------------------------------------- | -------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

Example APS documents showing various patterns and structures.

## Minimal valid plan

The smallest useful APS document:

```markdown
# Todo App

## Problem

Users need a simple task list.

## Success Criteria

- [ ] Users can add and list todos

## Work Items

### TODO-001: Initial setup

- **Intent:** Project builds and tests pass
- **Expected Outcome:** `npm run build` exits 0
- **Validation:** `npm run build`
```

## Full-featured index

```markdown
# E-Commerce Platform

## Overview

An online store with payments and notifications.

## Problem & Success Criteria

**Problem:** No e-commerce capability exists.

**Success Criteria:**

- [ ] Users can browse and purchase products
- [ ] Payments process securely

## Modules

| Module                                        | ID   | Owner | Status | Priority |
| --------------------------------------------- | ---- | ----- | ------ | -------- |
| [auth](./modules/auth.aps.md)                 | AUTH | @you  | Ready  | high     |
| [payments](./modules/payments.aps.md)         | PAY  | @you  | Draft  | high     |
| [notifications](./modules/notifications.aps.md) | NOTIF | @you | Draft  | medium   |
```

## Module with work items

```markdown
# Authentication Module

| ID   | Owner | Priority | Status |
| ---- | ----- | -------- | ------ |
| AUTH | @you  | high     | Ready  |

## Purpose

User authentication and session management.

## In Scope

- Login/logout
- Registration

## Work Items

### AUTH-001: Login endpoint

- **Status:** Complete: 2026-05-12
- **Intent:** Users authenticate with email and password
- **Expected Outcome:** POST /auth/login returns JWT for valid credentials; 401 for invalid
- **Validation:** `npm test -- auth.login.test.ts`

### AUTH-002: Registration

- **Status:** Ready
- **Intent:** New users create accounts with email and password
- **Expected Outcome:** POST /auth/register creates user; rejects duplicate email
- **Validation:** `npm test -- auth.register.test.ts`
- **Dependencies:** AUTH-001
```

## Work item dependencies

```markdown
### PAY-001: Payment processor integration

- **Dependencies:** AUTH-001, CORE-002

### PAY-002: Checkout flow

- **Dependencies:** PAY-001, CART-001, INV-001
```

Dependency graph:

```text
AUTH-001 ─┐
          ├──▶ PAY-001 ─┐
CORE-002 ─┘              │
                         ├──▶ PAY-002
CART-001 ────────────────┤
INV-001 ─────────────────┘
```

## Action plan with waves

```markdown
# Action Plan: AUTH-001

| Field     | Value                    |
| --------- | ------------------------ |
| Work Item | AUTH-001 — Login endpoint |
| Status    | In Progress              |

## Waves

| Wave | Actions | Gate                  |
| ---- | ------- | --------------------- |
| 1    | 1, 2    | Both checkpoints pass |
| 2    | 3       | Checkpoint passes     |

## Actions

### Action 1 — Create login endpoint

**Checkpoint**
Endpoint accepts POST /auth/login

**Validate**
`npm test -- auth.login.test.ts`

**Wave** 1

### Action 2 — Validate request body

**Checkpoint**
Invalid requests return 400 with field errors

**Wave** 1

### Action 3 — Verify credentials

**Checkpoint**
Valid credentials return JWT; invalid return 401

**Wave** 2
```

## Issue tracking

```markdown
### ISS-001: OAuth credentials pending

| Field      | Value    |
| ---------- | -------- |
| Status     | Open     |
| Severity   | medium   |
| Discovered | AUTH-002 |
| Module     | AUTH     |

**Context:** Need Google API client ID/secret to finish AUTH-002.
```

## Validation command patterns

```markdown
# Single command
- **Validation:** `npm test`

# Multiple commands
- **Validation:** `npm run build && npm test && npm run lint`

# With environment
- **Validation:** `NODE_ENV=test npm test src/auth/`

# Script reference
- **Validation:** `npm run test:auth`
```

---

**Next:** [Minimal plan example →](../examples/minimal-plan.md)
