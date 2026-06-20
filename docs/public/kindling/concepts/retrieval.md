---
id: retrieval
title: Retrieval
description:
  Deterministic, explainable three-tier retrieval — pins, current summary, and
  ranked hits.
sidebar_position: 3
---

# Retrieval

Retrieval is where Kindling earns its keep: getting the right context back out,
predictably. It is **mechanical and explainable** — full-text search over what
you actually stored, with a fixed ranking order and provenance on every result.
There are no embeddings and no opaque relevance model.

## The three tiers

A single retrieval returns results in three tiers, always in this order:

1. **Pins** — user-marked priority items. Non-evictable; always first.
2. **Current summary** — the active session/capsule summary, when one applies.
3. **Candidates** — ranked full-text search hits over observations and
   summaries.

```
kindling search "authentication token"
        │
        ├─ Pins              (always returned, never evicted)
        ├─ Current Summary   (active context, if present)
        └─ Candidates        (ranked FTS hits, with scores + provenance)
```

### Pins

Pins are how you guarantee something is always surfaced. Pin an observation or a
summary by ID:

```bash
kindling pin observation <id> --note "root cause — keep handy"
kindling pin summary <id>
kindling unpin <pin-id>
```

A pin may carry a `--note` explaining why it matters and an optional `--ttl` (in
milliseconds) after which it expires.

### Current summary

When a relevant capsule has a summary (produced on
[close](/kindling/concepts/capsules#close)), it is returned as the current
summary — the distilled conclusion of an episode of work, surfaced ahead of raw
observations.

### Candidates

Everything else is ranked full-text search over observation and summary content,
backed by SQLite FTS5. Each candidate carries a relevance `score` and an
optional `matchContext` snippet showing _why_ it matched.

## Result shape

```typescript
interface RetrieveResult {
  pins: { pin: Pin; target: Observation | Summary }[];
  currentSummary?: Summary;
  candidates: {
    entity: Observation | Summary;
    score: number;
    matchContext?: string;
  }[];
  provenance: {
    query: string;
    scopeIds: ScopeIds;
    totalCandidates: number;
    returnedCandidates: number;
    truncatedDueToTokenBudget: boolean;
    providerUsed: string;
  };
}
```

The `provenance` block makes every retrieval auditable: the exact query and
scope used, how many candidates matched versus were returned, and which provider
served the request.

## Searching from the CLI

```bash
# Basic search
kindling search "authentication error"

# Narrow by scope
kindling search "auth" --session s-1 --repo ./my-project

# Cap the number of candidates (default 10)
kindling search "auth" --max 5
```

Human output groups results by tier:

```
Search Results for: "authentication error"
==================================================

Pins (1):

1. [PIN] 9c20…
   Type: observation
   Content: JWT validation failed: token expired
   Note: root cause — keep handy

Candidates (2):

1. 5f1c… (score: 1.83)
   Type: observation
   Content: auth middleware rejects valid tokens after the upgrade
   Time: 2026-06-20 10:31:04
```

Add `--json` for the full structured `RetrieveResult`, including scores and
provenance.

## Why mechanical retrieval

You should be able to find exactly what you stored, and understand why each
result came back. Deterministic ranking plus provenance means results are
reproducible and debuggable — important when an AI agent is consuming them as
context rather than a human eyeballing a list.

## Next

- [Storage — the SQLite layer behind retrieval →](/kindling/concepts/storage)
- [CLI reference: search →](/kindling/reference/cli#search)
