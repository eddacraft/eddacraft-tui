---
id: overview
title: Kindling Overview
description: Capture and store structured observations from development sessions.
sidebar_position: 1
---

# Kindling

Kindling is an open-source tool for capturing structured observations from development sessions.

## What is Kindling?

Kindling captures the *context* that matters—decisions made, problems solved, patterns discovered—without drowning in noise.

```
Session → Observations → Capsule → Retrieval
```

**Session:** You're working on a feature, debugging, or learning.

**Observations:** Kindling captures structured notes about what happened.

**Capsule:** Observations are stored in a searchable container.

**Retrieval:** Later, you (or your tools) can query that knowledge.

## Why Kindling?

### The Problem

Development sessions generate valuable context:
- "This approach didn't work because..."
- "The API expects this specific format..."
- "This error means you need to..."

But that context is lost:
- Chat history disappears
- Terminal scrollback clears
- Memory fades

### The Solution

Kindling captures observations *as you work*:

```bash
kindling observe "The retry logic needs exponential backoff; linear fails under load"
```

Later:

```bash
kindling search "retry"
```

```
[2024-01-15] The retry logic needs exponential backoff; linear fails under load
  Source: manual observation
  Capsule: payment-integration
  Tags: retry, performance
```

## Core Concepts

### Capsules

A capsule is a container for related observations:

```bash
kindling capsule create payment-integration
```

Think of it as a project folder for knowledge.

### Observations

An observation is a single piece of captured context:

```typescript
{
  content: "API rate limits at 100 req/min, need to batch",
  kind: "discovery",
  source: "manual",
  timestamp: "2024-01-15T10:30:00Z",
  tags: ["api", "rate-limit"]
}
```

Observations have:
- **Content** — the actual knowledge
- **Kind** — what type of observation
- **Source** — where it came from
- **Provenance** — origin metadata

### Storage

By default, Kindling stores observations in SQLite:

```
~/.kindling/
├── capsules/
│   ├── payment-integration.db
│   └── auth-refactor.db
└── config.json
```

Portable. No server required.

## How It Fits

Kindling is part of the **Edda Stack**:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Kindling   │ ──▶ │   Ember     │ ──▶ │    Edda    │
│  (Capture)  │     │ (Candidate) │     │  (Curated) │
└─────────────┘     └─────────────┘     └─────────────┘
     Now              Coming Soon         Coming Soon
```

Kindling captures raw observations. Ember (planned) promotes valuable ones. Edda (planned) curates verified knowledge.

## Quick Example

```bash
# Create a capsule for your current work
kindling capsule create api-migration

# Capture observations as you work
kindling observe "Legacy API uses XML, new API uses JSON"
kindling observe "Need to handle both formats during transition"

# Search later
kindling search "XML"

# Export for use in prompts
kindling export --capsule api-migration --format markdown
```

## What Kindling Doesn't Do

Kindling is focused:

- **Not a note-taking app** — structured observations, not freeform notes
- **Not a chat logger** — captures knowledge, not conversation
- **Not a database** — simple storage, not a query engine
- **Not AI-powered** — mechanical storage and retrieval (AI optional)

---

**Next:** [Install Kindling →](/docs/kindling/quickstart/install)
