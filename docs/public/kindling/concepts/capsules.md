---
id: capsules
title: Capsules
description: Bounded groups of observations with an intent and an open/close lifecycle.
sidebar_position: 2
---

# Capsules

A **capsule** is a bounded unit of meaning: a group of related observations with
an _intent_ and a lifecycle. Where an [observation](/kindling/concepts/observations)
records a single event, a capsule captures a whole episode of work — a session,
or a single workflow node run.

## Structure

```typescript
interface Capsule {
  id: string;             // unique identifier
  type: CapsuleType;      // "session" | "pocketflow_node"
  intent: string;         // why this capsule exists
  status: CapsuleStatus;  // "open" | "closed"
  openedAt: number;       // epoch milliseconds
  closedAt?: number;      // set when closed
  scopeIds: ScopeIds;     // session / repo / agent / user / task
  observationIds: string[]; // members, in order
  summaryId?: string;     // summary produced on close
}
```

## Types

| Type              | Created by                                                      |
| ----------------- | --------------------------------------------------------------- |
| `session`         | Interactive development sessions — the CLI default, and what the [Claude Code](/kindling/adapters/claude-code) and [OpenCode](/kindling/adapters/opencode) adapters open. |
| `pocketflow_node` | A single [PocketFlow](/kindling/adapters/pocketflow) workflow node execution. |

## Intent

Every capsule has an `intent` — a short statement of _why_ it exists
("investigating the memory leak", "implement token refresh"). Intent is
required when opening a capsule and helps organise and rank retrieved context.

## Lifecycle

Capsules move through two states: **open** → **closed**.

### Open

```bash
kindling capsule open --intent "debug authentication issue" --repo ./my-project
```

`--type` defaults to `session`. While a capsule is open, observations can be
attached to it with `kindling log --capsule <id>`.

### Close

```bash
kindling capsule close <id> --summary "Fixed JWT expiration check in middleware"
```

Closing records `closedAt` and, when a summary is provided, attaches it as the
capsule's summary. That summary feeds the **current summary** tier of
[retrieval](/kindling/concepts/retrieval), so a closed capsule's conclusion
surfaces ahead of raw observations.

## Scope

Like observations, capsules carry a `ScopeIds` record (`sessionId`, `repoId`,
`agentId`, `userId`, `taskId`). Scope set on a capsule is how its work is later
isolated during search.

## Listing capsules

```bash
kindling list capsules                 # all
kindling list capsules --status open   # only open
kindling list capsules --repo ./my-project
```

## Next

- [Retrieval — pins, summaries, and ranked hits →](/kindling/concepts/retrieval)
- [Storage — where capsules live →](/kindling/concepts/storage)
