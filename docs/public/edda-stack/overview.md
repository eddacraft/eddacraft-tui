---
id: overview
title: Development Memory System Overview
description:
  Capture signals, promote reusable knowledge, and preserve trusted decisions.
sidebar_position: 1
owner: DOCSYNC
---

# Development Memory System

Most teams do not lose velocity because they cannot write code. They lose
velocity because they keep relearning the same lessons.

This is a three-stage capability system to capture signals, review candidates,
and preserve trusted decisions for reuse.

Internally we call these layers Kindling, Ember, and Edda.

## What people care about

- Fewer repeated incidents and regressions
- Faster onboarding for new engineers and agents
- Clear decision history: what changed, why, and by whom
- High-trust memory that does not fill with noise

## Capability model (first)

The stack is intentionally capability-first. Internal names are useful, but the
value is in the outcomes.

| Capability                | Internal layer | Status      | Outcome                                                                |
| ------------------------- | -------------- | ----------- | ---------------------------------------------------------------------- |
| Capture signals           | Kindling       | Available   | Useful observations are recorded with context while work is happening  |
| Evaluate candidates       | Ember          | Coming soon | Potentially reusable knowledge is reviewed instead of blindly promoted |
| Preserve canonical memory | Edda           | Coming soon | Team-approved truths stay queryable, attributable, and versioned       |

## Trust and governance contract

Canonical memory entries in this system are expected to be:

- Human-approved, not AI-autopublished
- Attributed with owner and rationale
- Linked to source observations and promotion history
- Versioned through explicit supersession

This is how memory stays trustworthy as teams and agents scale.

## What this does not do by default

- It does not auto-promote all captured notes.
- It does not treat AI suggestions as canonical truth.
- It does not optimise for memory volume over memory quality.

## How it works in practice

1. Capture raw observations during implementation, debugging, and reviews.
2. Promote only high-signal observations for review.
3. Approve canonical memory with human attribution and rationale.
4. Reuse these memories in future decisions, planning, and coding tasks.

## Why the three-layer design exists

The system separates speed from trust:

- Capture stays low-friction so teams do not skip it.
- Curation stays deliberate so memory quality does not collapse.
- Canonical memory stays durable so governance and audits remain possible.

## What to adopt first

Start with the capture layer and one team workflow:

- Capture discoveries and gotchas daily.
- Review candidates weekly.
- Promote only high-confidence entries to canonical memory.

This gives immediate value without forcing full process change on day one.

## 2-week pilot (low-friction rollout)

Week 1:

- Capture high-signal observations during active work.
- Tag by component or domain to improve retrieval.

Week 2:

- Run one review session for candidates.
- Promote only 1-3 entries that pass quality checks.

Pilot success signals:

- Lower repeated incident frequency
- Faster onboarding for common tasks
- Quicker decision turnaround on known issues

## Internal names vs external message

Externally, lead with capability language: capture, review, preserve, reuse.
Internally, Kindling/Ember/Edda provide precise terms for architecture and APIs.

---

**Next:** [Capabilities and components →](/edda-stack/components/kindling)
