---
id: observations
title: Observations
description: The atomic, immutable unit of captured context in Kindling.
sidebar_position: 1
---

# Observations

An **observation** is the atomic unit of memory in Kindling: a single, immutable
record of something that happened during development.

## Structure

Every observation has the same shape:

```typescript
interface Observation {
  id: string; // unique identifier (UUID)
  kind: ObservationKind; // what happened
  content: string; // the captured text
  provenance: object; // source-specific metadata (free-form JSON)
  ts: number; // epoch milliseconds
  scopeIds: ScopeIds; // session / repo / agent / user / task
  redacted: boolean; // whether the content has been forgotten
}
```

Observations are **immutable**. The only field that ever changes is `redacted` —
set via [forgetting](#forgetting) — which masks the content without deleting the
record.

## Kinds

The set of kinds is fixed. There are no free-form tags or custom kinds; this
keeps retrieval and adapters predictable across every tool.

| Kind          | Captures                                              |
| ------------- | ----------------------------------------------------- |
| `tool_call`   | An AI tool invocation (Read, Edit, Bash, …).          |
| `command`     | A shell command, typically with its exit code/output. |
| `file_diff`   | A file change, with the affected path(s).             |
| `error`       | An error, often with a stack trace.                   |
| `message`     | A user or assistant message. The CLI default.         |
| `node_start`  | A workflow node began executing.                      |
| `node_output` | A workflow node produced output.                      |
| `node_error`  | A workflow node failed.                               |
| `node_end`    | A workflow node finished (success or failure).        |

The first five are produced both manually (via `kindling log`) and by the
[Claude Code](/kindling/adapters/claude-code) and
[OpenCode](/kindling/adapters/opencode) adapters. The `node_*` kinds are
produced by the [PocketFlow](/kindling/adapters/pocketflow) adapter.

## Content

`content` is the human-readable text of the observation — the thing you will
search for later. When logging manually, be specific and self-contained:

```bash
# Vague
kindling log "API issue"

# Specific
kindling log --kind error "API returns 500 when payload exceeds 1MB"
```

## Provenance

`provenance` is a free-form JSON object carrying source-specific metadata —
whatever the producer wants to attach. Adapters populate it richly; for example,
a `command` observation may carry its exit code, a `tool_call` may carry the
tool name and arguments, and an `error` may carry a stack trace.

Provenance is also where retrieval explainability comes from: it travels with
the observation so you can always answer "where did this come from?".

## Scope

Every observation carries a `ScopeIds` record. All fields are optional, so an
observation can be globally visible or narrowly scoped:

| Field       | Meaning                                                                        |
| ----------- | ------------------------------------------------------------------------------ |
| `sessionId` | The session that produced it.                                                  |
| `repoId`    | The repository it belongs to.                                                  |
| `agentId`   | The agent that produced it.                                                    |
| `userId`    | The user it belongs to.                                                        |
| `taskId`    | The task it belongs to (carried for provenance; not a retrieval filter today). |

Scope is how [search](/kindling/concepts/retrieval) is narrowed:

```bash
kindling log "rate limit is 100 req/min" --session s-1 --repo ./my-project
kindling search "rate limit" --session s-1 --repo ./my-project
```

## Capturing observations

```bash
# Manually
kindling log --kind error "JWT validation failed: token expired"

# Attached to an open capsule
kindling log --capsule cap_8a3f… "found the root cause"
```

Adapters capture them automatically — see
[Automatic Capture](/kindling/quickstart/automatic-capture).

The `kindling-service` layer masks common secret patterns in observation content
before persistence. Adapter-level filters (for example in the OpenCode package)
add a second line of defence.

## Forgetting

To redact an observation, use its exact ID:

```bash
kindling forget <observation-id>
```

This sets `redacted: true` and masks the content. The record remains (so history
stays consistent), but it is excluded from results unless redacted items are
explicitly requested.

## Next

- [Capsules — grouping observations →](/kindling/concepts/capsules)
- [Retrieval — getting them back →](/kindling/concepts/retrieval)
