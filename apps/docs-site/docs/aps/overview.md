---
id: overview
title: APS Overview
description: The Anvil Plan Specification — a deterministic, hash-stable format for development plans.
sidebar_position: 1
---

# Anvil Plan Specification (APS)

APS is an open-source specification for defining deterministic, hash-stable development plans.

## What is APS?

APS is a document format (Markdown with YAML frontmatter) that describes:

- **What** should be built (not how)
- **Success criteria** for each piece of work
- **Validation commands** to verify completion

```markdown
---
format: aps
version: 1.0
---

# Feature: User Authentication

## Task: AUTH-001 — Login Endpoint

**Outcome:** Users authenticate with email/password, receiving a JWT.

**Validation:** `pnpm test src/auth/login.test.ts`
```

## Why APS?

### Determinism

Given the same plan, tools produce the same validation. The plan's hash doesn't change unless content changes.

**Enables:**
- Reproducible builds
- Reliable caching
- Audit trails

### Separation of Concerns

Plans describe *intent*, not *implementation*:

```markdown
❌ "Create middleware that extracts JWT, validates signature using
    jsonwebtoken library, checks expiry..."

✓ "Auth middleware validates requests, attaches user to context"
```

The executor (human or AI) determines implementation. The plan defines success.

### Tool Interoperability

APS is a common format. Different tools can:
- Generate APS from their native formats
- Consume APS for validation
- Exchange plans without lock-in

## Core Concepts

### Index

The root document listing all modules:

```markdown
---
format: aps
version: 1.0
---

# Project Plan

## Modules
- [auth](modules/auth.aps.md)
- [payments](modules/payments.aps.md)
```

### Module

A cohesive unit of functionality:

```markdown
---
format: aps
module: auth
---

# Authentication Module

## Tasks
- AUTH-001: Login endpoint
- AUTH-002: Registration endpoint
```

### Task

A unit of authorised work:

```markdown
## Task: AUTH-001 — Login endpoint

**Outcome:** Users authenticate with credentials

**Validation:** `pnpm test src/auth/login.test.ts`

**Steps:**
1. [ ] Endpoint accepts POST /auth/login
2. [ ] Returns 401 for invalid credentials
3. [ ] Returns JWT for valid credentials
```

### Step

An observable checkpoint (what should be true, not how):

```markdown
**Steps:**
1. [ ] API responds to health check
2. [ ] Database connection established
3. [ ] Cache warmed on startup
```

## Design Principles

### Intent Over Implementation

Plans describe outcomes, not code:

```markdown
❌ "Import bcrypt, hash password with 10 rounds..."
✓ "Passwords stored securely (not plaintext)"
```

### Observable State Only

Steps describe what can be verified:

```markdown
❌ "Refactor the auth module" (not observable)
✓ "Auth module has <100 lines per file" (observable)
```

### Hash Stability

Content changes = hash changes. Formatting changes don't:

```markdown
# These produce the same hash:

## Task: AUTH-001
Outcome: Users can log in

## Task: AUTH-001
Outcome:  Users   can   log in
```

## Getting Started

### 1. Create a Plan

```bash
# Manual
touch plans/index.aps.md

# Or via CLI
anvil plan create
```

### 2. Define Structure

```markdown
---
format: aps
version: 1.0
---

# My Project

## Modules
- [core](modules/core.aps.md)
```

### 3. Add Tasks

```markdown
---
format: aps
module: core
---

# Core Module

## Task: CORE-001 — Initial setup

**Outcome:** Project builds and tests pass

**Validation:** `pnpm build && pnpm test`
```

### 4. Validate

```bash
anvil plan validate plans/index.aps.md
```

---

**Next:** [Specification details →](/docs/aps/spec/taxonomy)
