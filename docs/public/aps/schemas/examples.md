---
id: examples
title: Schema Examples
description: Example APS documents with schema annotations.
sidebar_position: 2
---

# Schema Examples

Example APS documents showing various patterns and structures.

## Minimal Valid Plan

The smallest valid APS document:

```markdown
---
format: aps
version: 1.0
---

# My Project

## Task: TASK-001 — Setup

**Intent:** Project initialised

**Validation:** `echo "done"`
```

**JSON representation:**

```json
{
  "format": "aps",
  "version": "1.0",
  "title": "My Project",
  "modules": [
    {
      "id": "default",
      "tasks": [
        {
          "id": "TASK-001",
          "title": "Setup",
          "intent": "Project initialised",
          "validation": "echo \"done\""
        }
      ]
    }
  ]
}
```

## Full Featured Plan

All fields populated:

```markdown
---
format: aps
version: 1.0
hash: sha256:abc123...
---

# E-Commerce Platform

An online store with payments and notifications.

## Modules

- [auth](modules/auth.aps.md)
- [payments](modules/payments.aps.md)
- [notifications](modules/notifications.aps.md)
```

````markdown
---
format: aps
module: auth
depends_on:
  - core
files:
  - src/auth/**
  - src/types/auth.ts
---

# Authentication Module

User authentication and session management.

## Task: AUTH-001 — Login endpoint

**Status:** completed

**Intent:** Users can authenticate with email and password. The system returns a
JWT valid for 24 hours. Invalid credentials return a 401 response.

**Validation:**

```bash
pnpm test src/auth/login.test.ts
```
````

**Dependencies:**

- CORE-001

**Steps:**

1. [x] Endpoint accepts POST /auth/login
2. [x] Request body validated
3. [x] Password verified against hash
4. [x] JWT generated with claims
5. [x] Response includes token

## Task: AUTH-002 — Registration

**Status:** locked

**Intent:** New users can create accounts with email and password. Email
uniqueness is enforced. Password strength requirements applied.

**Validation:**

```bash
pnpm test src/auth/register.test.ts
```

**Dependencies:**

- AUTH-001

**Steps:**

1. [x] Endpoint accepts POST /auth/register
2. [x] Email format validated
3. [ ] Password strength checked
4. [ ] Email uniqueness verified
5. [ ] User record created
6. [ ] Welcome email sent

````

## Task Dependencies

Showing complex dependency chains:

```markdown
## Task: PAY-001 — Payment processor integration

**Dependencies:**
- AUTH-001 (user context required)
- CORE-002 (database transactions)

---

## Task: PAY-002 — Checkout flow

**Dependencies:**
- PAY-001 (payment processor)
- CART-001 (cart service)
- INV-001 (inventory check)
````

**Dependency graph:**

```
AUTH-001 ─┐
          ├──▶ PAY-001 ─┐
CORE-002 ─┘              │
                         ├──▶ PAY-002
CART-001 ────────────────┤
                         │
INV-001 ─────────────────┘
```

## Module Dependencies

Cross-module dependencies:

```markdown
---
format: aps
module: payments
depends_on:
  - auth
  - inventory
  - notifications
files:
  - src/payments/**
---

# Payments Module

Requires auth for user context, inventory for stock checks, and notifications
for receipts.
```

## Step Variations

### Simple Steps

```markdown
**Steps:**

1. [ ] Feature flag created
2. [ ] Feature implemented
3. [ ] Tests passing
4. [ ] Deployed to staging
```

### Detailed Steps

```markdown
**Steps:**

1. [ ] Endpoint responds to health check
   - GET /health returns 200
   - Response includes version

2. [ ] Database connection verified
   - Connection pool initialised
   - Read/write operations work

3. [ ] Cache layer operational
   - Redis connection established
   - TTL behaviour correct
```

## Validation Commands

### Single Command

```markdown
**Validation:** `pnpm test`
```

### Multiple Commands

````markdown
**Validation:**

```bash
pnpm build && pnpm test && pnpm lint
```
````

````

### With Environment

```markdown
**Validation:**
```bash
NODE_ENV=test pnpm test src/auth/
````

````

### Script Reference

```markdown
**Validation:** `pnpm run test:auth`
````

---

**Next:** [Minimal plan example →](/aps/examples/minimal-plan)
