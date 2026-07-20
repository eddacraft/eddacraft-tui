---
id: multi-module
title: 'Example: dependent modules'
description: A multi-module APS plan with an explicit dependency boundary.
sidebar_position: 2
---

# Example: dependent modules

This example separates identity from sessions. The session module cannot begin
until the identity module is complete.

## Index

```markdown
# Account access

## Overview

Add account identity and session creation in dependency order.

## Problem & Success Criteria

**Problem:** The product has no authenticated account access.

**Success Criteria:**

- [ ] A registered user can create a session.
- [ ] Invalid credentials do not create a session.

## Modules

| Module                                | ID      | Status | Dependencies |
| ------------------------------------- | ------- | ------ | ------------ |
| [identity](./modules/identity.aps.md) | ID      | Ready  | —            |
| [sessions](./modules/sessions.aps.md) | SESSION | Draft  | ID           |
```

## Identity module

```markdown
# Identity

| ID  | Owner | Priority | Status |
| --- | ----- | -------- | ------ |
| ID  | @team | high     | Ready  |

## Purpose

Own account records and credential verification.

## In Scope

- Account creation.
- Credential verification.

**Last reviewed:** 2026-07-20

## Work Items

### ID-001: Verify credentials

- **Status:** Ready
- **Intent:** Verify a registered account's credentials.
- **Expected Outcome:** Correct credentials return an account ID; incorrect
  credentials fail safely.
- **Validation:** `npm test -- identity`
```

## Session module

```markdown
# Sessions

| ID      | Owner | Priority | Status | Dependencies |
| ------- | ----- | -------- | ------ | ------------ |
| SESSION | @team | high     | Draft  | ID           |

## Purpose

Create and revoke authenticated sessions after identity verification.

## In Scope

- Session creation and revocation.

## Work Items

### SESSION-001: Create a session

- **Status:** Draft
- **Intent:** Create a session for a verified account.
- **Expected Outcome:** A verified account receives a revocable session.
- **Validation:** `npm test -- sessions`
- **Dependencies:** ID-001
```

## How the queue behaves

```bash
aps next
aps graph
```

The queue selects `ID-001`. `SESSION-001` remains unavailable because both its
module and dependency are unfinished. After identity completes, review the
session scope and deliberately change its module and item to `Ready`.

This is the central APS boundary: dependency state, not prompt urgency, decides
what may execute next.
