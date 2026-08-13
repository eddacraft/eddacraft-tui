---
id: examples
title: Copyable format fragments
description: Copy common APS fields and structures into an existing plan.
sidebar_position: 3
owner: DOCSYNC
verified_against: 0.6.0
---

# Copyable format fragments

These fragments are building blocks, not complete files. Use the
[small-module example](../examples/minimal-plan.md) when you need a complete
starting plan.

## Ready work item

```markdown
### AUTH-003: Add token renewal

- **Status:** Ready
- **Intent:** Keep an authenticated session active across short network
  interruptions.
- **Expected Outcome:** A valid renewal token produces a new session token.
- **Validation:** `npm test -- token-renewal`
- **Dependencies:** AUTH-001, AUTH-002
- **Confidence:** medium
- **Non-scope:** Changing initial login.
```

## Blocked work item

```markdown
### PAY-004: Capture payment

- **Status:** Blocked
- **Intent:** Capture an authorised payment.
- **Expected Outcome:** Successful capture records the provider reference.
- **Validation:** `npm test -- payment-capture`
- **Dependencies:** PAY-003
- **Risks:** Provider sandbox access is not yet approved.
```

Name the blocking condition in nearby prose or the risks field. Do not use
`Blocked` as a substitute for an undefined scope.

## Module interface

```markdown
## Interfaces

**Depends on:**

- identity — verified account IDs

**Exposes:**

- session service — create and revoke sessions
```

## Decision

```markdown
## Decisions

- **D-001:** Session tokens expire after 15 minutes — limits exposure while
  renewal preserves usability.
```

## Action checkpoint

```markdown
### Action 1 — Add the renewal endpoint

**Purpose** Expose the authorised renewal behaviour.

**Produces** A tested endpoint and response contract.

**Checkpoint** Valid renewal tokens create a new session token.

**Validate** `npm test -- token-renewal`
```

## Package scope

```markdown
- **Packages:** api, auth-core
```

An item-level package list overrides its module's package list for queue
filtering. See [plan a monorepo](../guides/monorepo.md).
