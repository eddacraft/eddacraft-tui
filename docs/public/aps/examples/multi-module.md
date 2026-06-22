---
id: multi-module
title: Multi-Module Plan
description: A realistic multi-module APS plan based on the user-auth example.
sidebar_position: 2
---

# Multi-Module Plan

| Type      | Authority | Owner   | Status | Freshness                                              |
| --------- | --------- | ------- | ------ | ------------------------------------------------------ |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                                  | Downstream           |
| ----------------------------------------------------------------------------------------- | -------------------- |
| [anvil-plan-spec `examples/user-auth`](https://github.com/EddaCraft/anvil-plan-spec/tree/main/examples/user-auth) | APS docs-site section |

A realistic example showing a multi-module APS structure for adding user
authentication to an existing application. Based on the
[user-auth example](https://github.com/EddaCraft/anvil-plan-spec/tree/main/examples/user-auth)
in anvil-plan-spec.

## Directory structure

```text
plans/
├── index.aps.md
├── modules/
│   ├── auth.aps.md
│   └── session.aps.md
├── designs/
│   └── 2025-01-05-auth-architecture.design.md
└── execution/
    └── AUTH-001.actions.md
```

## Index

```markdown
# User Authentication

## Overview

Add user authentication to an existing web application.

## Problem & Success Criteria

**Problem:** The application has no user authentication.

**Success Criteria:**

- [ ] Users can register with email and password
- [ ] Users can log in and receive a session token
- [ ] Protected routes reject unauthenticated requests

## Designs

- [Auth Architecture](./designs/2025-01-05-auth-architecture.design.md)

## Modules

| Module                              | ID      | Owner | Status | Priority | Dependencies |
| ----------------------------------- | ------- | ----- | ------ | -------- | ------------ |
| [auth](./modules/auth.aps.md)       | AUTH    | @josh | Ready  | high     | —            |
| [session](./modules/session.aps.md) | SESSION | @josh | Draft  | high     | auth         |
```

## Auth module

```markdown
# Authentication Module

| ID   | Owner | Priority | Status |
| ---- | ----- | -------- | ------ |
| AUTH | @josh | high     | Ready  |

## Purpose

Handle user registration and credential verification.

## In Scope

- User registration (email + password)
- Password hashing and verification
- Login credential validation

## Out of Scope

- Session management (see SESSION module)
- OAuth/social login (future work)

## Interfaces

**Depends on:**

- Database — user table with email, password_hash columns

**Exposes:**

- `registerUser(email, password)` → User
- `verifyCredentials(email, password)` → User | null

## Work Items

### AUTH-001: Create user registration function

- **Intent:** Allow new users to register with email and password
- **Expected Outcome:** `registerUser()` creates user record with hashed password
- **Validation:** `npm test -- auth.test.ts`
- **Confidence:** high

### AUTH-002: Create credential verification function

- **Intent:** Verify email/password combinations for login
- **Expected Outcome:** `verifyCredentials()` returns user if valid, null if not
- **Validation:** `npm test -- auth.test.ts`
- **Dependencies:** AUTH-001
- **Confidence:** high
```

## Session module

```markdown
# Session Module

| ID      | Owner | Priority | Status |
| ------- | ----- | -------- | ------ |
| SESSION | @josh | high     | Draft  |

## Purpose

Manage user sessions via JWT tokens.

## Interfaces

**Depends on:**

- auth — credential verification

**Exposes:**

- `createSession(user)` → token
- `validateSession(token)` → user | null

## Work Items

### SESSION-001: JWT token generation

- **Intent:** Generate signed JWT tokens on successful login
- **Expected Outcome:** Token contains user ID and expiry; verifiable with secret
- **Validation:** `npm test -- session.test.ts`
- **Dependencies:** AUTH-002
```

## Action plan

`plans/execution/AUTH-001.actions.md`:

```markdown
# Action Plan: AUTH-001

| Field     | Value                                    |
| --------- | ---------------------------------------- |
| Work Item | AUTH-001 — Create user registration function |
| Status    | In Progress                              |

## Actions

### Action 1 — Create user table migration

**Checkpoint**
Migration file exists with email, password_hash, created_at columns

**Validate**
`npm run migrate`

### Action 2 — Implement registerUser function

**Checkpoint**
Function hashes password and inserts user record

**Validate**
`npm test -- auth.test.ts`

### Action 3 — Reject duplicate emails

**Checkpoint**
Duplicate email registration returns error

**Validate**
`npm test -- auth.test.ts`
```

## Dependency graph

```text
AUTH-001 ──▶ AUTH-002 ──▶ SESSION-001
```

## Driving with the CLI

```bash
aps next                    # AUTH-001 (first Ready item)
aps start AUTH-001          # claim it
# ...implement via action plan...
aps complete AUTH-001 --learning "bcrypt cost 12 sufficient for v1"
aps next                    # AUTH-002 (AUTH-001 now Complete)
```

---

**Next:** [CLI Reference →](../tooling/validation.md)
