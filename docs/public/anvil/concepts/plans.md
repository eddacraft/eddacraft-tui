---
id: plans
title: Plans
description:
  Understanding APS plans as the foundation for deterministic validation.
sidebar_position: 4
---

# Plans

Plans define _what_ should be built. anvil validates _how_ it's being built
against that definition.

## What is a Plan?

A plan is an APS document that describes:

- **Modules** — cohesive units of functionality
- **Tasks** — authorised work with validation criteria
- **Steps** — observable checkpoints within tasks

```
┌─────────────────────────────────────────┐
│                  Index                   │
│  (Project-level, lists all modules)     │
└───────────────────┬─────────────────────┘
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
    ┌───────┐   ┌───────┐   ┌───────┐
    │Module │   │Module │   │Module │
    │  A    │   │  B    │   │  C    │
    └───┬───┘   └───┬───┘   └───┬───┘
        │           │           │
    ┌───▼───┐   ┌───▼───┐   ┌───▼───┐
    │Tasks  │   │Tasks  │   │Tasks  │
    └───────┘   └───────┘   └───────┘
```

## Why Plans Matter

### Determinism

Plans are hash-stable. Given the same plan, anvil produces the same validation.
This enables:

- **Reproducible builds** — same inputs, same outputs
- **Audit trails** — prove what was validated
- **Caching** — skip unchanged validations

### Intent Declaration

Plans capture _intent_ before execution. The AI (or developer) works within the
plan's boundaries:

```markdown
## Task: AUTH-001 — Implement login endpoint

Outcome: Users can authenticate with email/password

Validation: `pnpm test src/auth/login.test.ts`
```

The plan doesn't say _how_ to implement login—that's up to the executor. It says
_what_ success looks like.

### Bounded Context

Each module is a bounded context. anvil can enforce that changes stay within
their module's boundaries:

```
Module: auth
Files: src/auth/**

Module: payments
Files: src/payments/**
```

A task in `auth` modifying files in `payments` triggers a boundary warning.

## Plan Structure

### Index (`index.aps.md`)

The root document:

```markdown
---
format: aps
version: 1.0
hash: sha256:abc123...
---

# Project Plan

## Modules

- [auth](modules/auth.aps.md)
- [payments](modules/payments.aps.md)
- [notifications](modules/notifications.aps.md)
```

### Module (`modules/auth.aps.md`)

A cohesive feature area:

```markdown
---
format: aps
module: auth
---

# Auth Module

## Tasks

### AUTH-001 — Login endpoint

### AUTH-002 — Registration endpoint

### AUTH-003 — Password reset
```

### Task

A unit of authorised work:

```markdown
### AUTH-001 — Login endpoint

**Outcome:** Users authenticate with email and password, receiving a JWT.

**Validation:** `pnpm test src/auth/login.test.ts`

**Steps:**

1. [ ] Endpoint accepts POST /auth/login
2. [ ] Invalid credentials return 401
3. [ ] Valid credentials return JWT
4. [ ] JWT contains user ID and expiry
```

## Plans in anvil

### Validation Against Plans

Use the CLI to validate APS document structure before sharing a plan:

```bash
anvil validate plans/index.aps.md
```

anvil checks the plan document itself:

- Does the Markdown use the expected APS sections?
- Are work item IDs and required fields well-formed?
- Are optional integrity hashes valid when present?

### Plan-less Mode

anvil works without plans too. In this mode, it only runs gate checks
(architecture, anti-patterns, etc.) without plan validation.

### Creating Plans

Plans can be created:

- **Manually** — write APS markdown
- **With agent/tooling support** — generate APS markdown, then validate it with
  `anvil validate`
- **From external formats** — anvil adapters convert SpecKit, BMAD, etc.

---

**Learn more:** [APS Specification →](/aps/overview)
