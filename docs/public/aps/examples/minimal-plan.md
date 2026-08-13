---
id: minimal-plan
title: 'Example: one small module'
description: A complete small APS plan with one executable work item.
sidebar_position: 1
owner: DOCSYNC
verified_against: 0.6.0
---

# Example: one small module

This example is the smallest practical orchestration shape: one index and one
module. It is intentionally more complete than an illustrative one-file sketch.

## `plans/index.aps.md`

```markdown
# Status endpoint

## Overview

Add one machine-readable service status endpoint.

## Problem & Success Criteria

**Problem:** Operators cannot distinguish a running service from a healthy one.

**Success Criteria:**

- [ ] `GET /status` returns HTTP 200 and a stable JSON body.

## Modules

| Module                            | ID     | Status | Dependencies |
| --------------------------------- | ------ | ------ | ------------ |
| [status](./modules/status.aps.md) | STATUS | Ready  | —            |
```

## `plans/modules/status.aps.md`

```markdown
# Status endpoint

| ID     | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| STATUS | @team | medium   | Ready  |

## Purpose

Expose the service's current health in a stable machine-readable form.

## In Scope

- One unauthenticated status endpoint.
- A response-contract test.

## Out of Scope

- Metrics and tracing.

**Last reviewed:** 2026-07-20

## Work Items

### STATUS-001: Add the status endpoint

- **Status:** Ready
- **Intent:** Give operators a deterministic health probe.
- **Expected Outcome:** `GET /status` returns HTTP 200 with `{"status":"ok"}`.
- **Validation:** `npm test -- status`
```

## Try the example

Use a current review date and a validation command that exists in your project,
then run:

```bash
aps lint
aps next
```

`aps next` selects `STATUS-001`. Continue with the
[execution workflow](../workflow.md).
