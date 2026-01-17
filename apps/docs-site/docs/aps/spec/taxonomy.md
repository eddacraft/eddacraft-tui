---
id: taxonomy
title: Taxonomy
description: The four-level structure of APS documents.
sidebar_position: 1
---

# APS Taxonomy

APS documents follow a four-level hierarchy: **Index → Module → Task → Step**.

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
    │ Tasks │       │ Tasks │       │ Tasks │
    └───┬───┘       └───┬───┘       └───┬───┘
        │               │               │
    ┌───▼───┐       ┌───▼───┐       ┌───▼───┐
    │ Steps │       │ Steps │       │ Steps │
    └───────┘       └───────┘       └───────┘
```

## Level 1: Index

The root document that ties everything together.

### Purpose

- Lists all modules in the project
- Provides project-level metadata
- Serves as entry point for validation

### Format

```markdown
---
format: aps
version: 1.0
hash: sha256:abc123...
---

# Project Name

Brief description of the project.

## Modules

- [auth](modules/auth.aps.md)
- [payments](modules/payments.aps.md)
- [notifications](modules/notifications.aps.md)
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| `format` | Yes | Must be `aps` |
| `version` | Yes | APS version (e.g., `1.0`) |
| `hash` | Auto | Content hash (generated) |

## Level 2: Module

A cohesive unit of functionality.

### Purpose

- Groups related tasks
- Defines a bounded context
- Maps to a feature or component

### Format

```markdown
---
format: aps
module: auth
depends_on:
  - core
files:
  - src/auth/**
---

# Authentication Module

Handles user authentication and authorisation.

## Overview

This module provides:
- Login/logout
- Registration
- Password reset
- Session management

## Tasks

### AUTH-001 — Login endpoint
### AUTH-002 — Registration endpoint
### AUTH-003 — Password reset
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| `format` | Yes | Must be `aps` |
| `module` | Yes | Module identifier |
| `depends_on` | No | Other modules this depends on |
| `files` | No | Glob patterns for module scope |

## Level 3: Task

A unit of authorised work.

### Purpose

- Defines a discrete piece of work
- Specifies success criteria
- Provides validation command

### Format

```markdown
### Task: AUTH-001 — Login endpoint

**Status:** pending | in_progress | complete | blocked

**Outcome:**
Users can authenticate with email and password, receiving a JWT
that can be used for subsequent authenticated requests.

**Validation:**
```bash
pnpm test src/auth/login.test.ts
```

**Dependencies:**
- CORE-001 (database setup)

**Steps:**
1. [ ] Endpoint accepts POST /auth/login
2. [ ] Invalid credentials return 401
3. [ ] Valid credentials return JWT
4. [ ] JWT contains user ID and expiry
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| ID | Yes | Unique identifier (e.g., `AUTH-001`) |
| Title | Yes | Short description |
| Status | No | Current state |
| Outcome | Yes | What success looks like |
| Validation | Yes | Command to verify completion |
| Dependencies | No | Other tasks that must complete first |
| Steps | No | Observable checkpoints |

### Task IDs

Task IDs follow the pattern: `{MODULE}-{NUMBER}`

- `AUTH-001`, `AUTH-002` — Authentication tasks
- `PAY-001`, `PAY-002` — Payment tasks
- `CORE-001` — Core/shared tasks

## Level 4: Step

An observable checkpoint within a task.

### Purpose

- Breaks down complex tasks
- Provides progress visibility
- Enables partial completion tracking

### Format

```markdown
**Steps:**
1. [ ] Endpoint responds to POST request
2. [ ] Request body validated (email, password required)
3. [ ] Password compared against stored hash
4. [x] JWT generation working
5. [ ] Response includes token and expiry
```

### Rules

**Do:**
- Describe observable state
- Use verifiable conditions
- Keep under 12 words

**Don't:**
- Include implementation details
- Reference specific code
- Use subjective criteria

### Examples

```markdown
❌ "Refactor the authentication code"
   (Not observable, subjective)

❌ "Import bcrypt and hash with 10 rounds"
   (Implementation detail)

✓ "Passwords are hashed before storage"
   (Observable outcome)

✓ "Login returns 401 for wrong password"
   (Verifiable behaviour)
```

## Relationships

### Dependencies

Tasks can depend on other tasks:

```markdown
### Task: AUTH-002 — Registration

**Dependencies:**
- CORE-001 (database)
- AUTH-001 (shares JWT logic)
```

### Module Dependencies

Modules can depend on other modules:

```yaml
---
module: payments
depends_on:
  - auth
  - core
---
```

### File Scope

Modules can declare their file scope:

```yaml
---
module: auth
files:
  - src/auth/**
  - src/types/auth.ts
---
```

This enables tooling to validate that changes stay within scope.

---

**Next:** [File layout conventions →](/docs/aps/spec/file-layout)
