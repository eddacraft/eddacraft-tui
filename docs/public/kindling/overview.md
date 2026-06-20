---
id: overview
title: Kindling Overview
description:
  Local-first memory and continuity for AI-assisted development — capture,
  organise, and deterministically retrieve development context.
sidebar_position: 1
---

# Kindling

**Local-first memory and continuity for AI-assisted development.**

Kindling gives AI coding tools — and you — a memory of what happened: tool
calls, file edits, commands, errors, decisions, summaries, and pinned findings.
It stores that context locally in SQLite, organises it into meaningful sessions,
and retrieves it later with deterministic, explainable results.

You can use Kindling from the command line, as a daemon-backed Rust SDK, or as
an embedded in-process library.

## The model

Kindling has a small, deliberate data model:

```
Events → Observations → Capsules → Retrieval
```

- **Observations** are atomic, immutable records of something that happened: a
  tool call, a shell command, a file diff, an error, or a message.
- **Capsules** are bounded groups of observations — a session, a workflow run —
  each with an _intent_ and an open/close lifecycle.
- **Retrieval** returns context in three predictable tiers: pins, the current
  summary, and ranked search hits, each with provenance.

## Why Kindling

Development sessions generate valuable context that is normally lost — chat
history disappears, terminal scrollback clears, and reasoning fades. Kindling
captures that context as it happens and makes it retrievable later, so you (or
your assistant) can resume work without re-explaining everything.

It is built around three principles:

- **Local-first.** Project memory lives on your machine in SQLite. No server, no
  account, no network round-trip.
- **Deterministic retrieval.** Pins, the current summary, and ranked provider
  hits are returned in a predictable, explainable order — you get back what you
  stored, not what an opaque model guessed.
- **Tool-agnostic.** Capture context manually with the CLI, automatically
  through an adapter (Claude Code, OpenCode, PocketFlow), or programmatically
  from your own Rust or TypeScript code.

## A quick taste

```bash
# Initialise memory for this project
kindling init

# Capture context as you work
kindling log "JWT tokens expire after 15 minutes, not 1 hour"
kindling log --kind error "segfault in auth middleware after the upgrade"

# Get it back later, ranked and explainable
kindling search "JWT"
```

```
Search Results for: "JWT"
==================================================

Candidates (1):

1. 5f1c… (score: 1.83)
   Type: observation
   Content: JWT tokens expire after 15 minutes, not 1 hour
   Time: 2026-06-20 10:31:04
```

## How it fits together

```
        You / AI coding tools
   ┌───────────┬───────────┬───────────────┐
   │   CLI     │  Adapters │   SDK crates  │
   │ kindling  │  (hooks)  │ kindling-*    │
   └─────┬─────┴─────┬─────┴───────┬───────┘
         └───────────┼─────────────┘
                     ▼
            kindling daemon / service
                     ▼
        SQLite (per project, WAL + FTS5)
```

The same store backs every entry point. A CLI invocation, a Claude Code hook,
and a Rust client all read and write the same per-project database.

## What Kindling is not

- **Not a note-taking app** — it captures structured observations, not freeform
  prose.
- **Not a chat logger** — it stores knowledge and events, not transcripts.
- **Not a semantic black box** — retrieval is mechanical full-text search with
  explainable ranking, not embeddings guessing at intent.

## Where to go next

- [Install Kindling →](/kindling/quickstart/install)
- [Your first memory →](/kindling/quickstart/first-memory)
- [Core concepts: observations →](/kindling/concepts/observations)
- [CLI reference →](/kindling/reference/cli)
