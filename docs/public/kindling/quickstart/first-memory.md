---
id: first-memory
title: Your First Memory
description: Capture observations, group them into a capsule, and search your memory from the CLI.
sidebar_position: 2
---

# Your First Memory

This walkthrough uses the CLI to capture context, organise it into a capsule,
and retrieve it. It assumes you have [installed Kindling](/kindling/quickstart/install)
and run `kindling init` in your project.

## Capture an observation

The fastest way to record context is `kindling log`:

```bash
kindling log "The retry logic needs exponential backoff; linear fails under load"
```

Every observation has a **kind**. The default is `message`; pass `--kind` to be
explicit:

```bash
kindling log --kind error "segfault in auth middleware after the upgrade"
kindling log --kind command "pnpm test — 3 failures in auth.spec.ts"
```

The available kinds are fixed: `tool_call`, `command`, `file_diff`, `error`,
`message`, and the workflow kinds `node_start`, `node_end`, `node_output`,
`node_error`. See [Observations](/kindling/concepts/observations) for what each
means.

### Scope an observation

Observations can be tagged with scope identifiers so retrieval can be narrowed
later:

```bash
kindling log "rate limit is 100 req/min" --session session-123 --repo ./my-project
```

## Group work into a capsule

A **capsule** is a bounded session of related observations with an _intent_ and
an open/close lifecycle. Open one when you start a piece of work:

```bash
kindling capsule open --intent "investigating the memory leak" --repo ./my-project
```

```
Opened capsule cap_8a3f… (session)
  intent: investigating the memory leak
```

Attach observations to it with `--capsule`:

```bash
kindling log --capsule cap_8a3f… --kind error "unbounded cache in SessionStore"
```

Close it with a summary when you are done. The summary becomes part of the
[retrieval](/kindling/concepts/retrieval) current-summary tier:

```bash
kindling capsule close cap_8a3f… --summary "root cause: unbounded cache in SessionStore"
```

## Search your memory

```bash
kindling search "memory leak"
```

```
Search Results for: "memory leak"
==================================================

Current Summary:
  root cause: unbounded cache in SessionStore
  Confidence: 0.8

Candidates (1):

1. 9c20… (score: 2.41)
   Type: observation
   Content: unbounded cache in SessionStore
   Time: 2026-06-20 11:02:17
```

Narrow results by scope, and cap how many candidates come back:

```bash
kindling search "auth" --session session-123 --repo ./my-project --max 5
```

## Pin what matters

Pins are never evicted and always appear first in search results. Pin an
observation (or a summary) by ID, optionally with a note:

```bash
kindling pin observation 9c20… --note "root cause — keep handy"
```

Remove it later with `kindling unpin <pin-id>`.

## List and inspect

```bash
kindling list capsules                 # all capsules
kindling list capsules --status open   # only open ones
kindling list observations --kind error
kindling list pins
```

## Export and import

Memory is portable. Export the whole store (or a single session/repo) to JSON,
and import it elsewhere:

```bash
kindling export ./backup.json --pretty
kindling import ./backup.json
```

## Next steps

- [Let an adapter capture automatically →](/kindling/quickstart/automatic-capture)
- [How retrieval ranks results →](/kindling/concepts/retrieval)
- [Full CLI reference →](/kindling/reference/cli)
