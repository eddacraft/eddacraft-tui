---
id: ember
title: Candidate Review (Ember)
description:
  Review and refine high-signal observations before canonical promotion.
sidebar_position: 2
owner: DOCSYNC
upstream:
  - packages/edda-stack/src/ember/README.md
verified_against: 0.9.4-beta
---

# Candidate Review (Ember)

Ember is the review capability in the memory system. It sits between raw capture
and canonical memory to keep quality high.

## Why this capability matters

- Prevents raw notes from polluting long-term memory
- Gives teams a clear promotion workflow
- Creates shared ownership of what becomes reusable guidance

## What Ember does

- Surfaces candidate observations for review
- Supports accept, reject, or refine decisions
- Preserves links to source observations and sessions
- Requires explicit human decision before promotion

## Operating contract

- **Inputs:** Candidate observations promoted from capture
- **Decision point:** Accept, reject, or refine promotion candidates
- **Outputs:** Reviewed candidates with decision, rationale, and confidence
- **Evidence:** Review trail that links candidate outcomes to source
  observations

## Review criteria teams care about

Use these checks before promotion:

- Is this reusable across multiple tasks or engineers?
- Is it still correct under current architecture?
- Does it include enough context to avoid misuse?
- Is it already captured elsewhere in canonical memory?

## Example promotion decision

```
Candidate: "Retries hide payment provider 429 limits"

Decision: Promote
Reason: Repeated production issue across two services
Confidence: High
Owner: Platform team
```

## Positioning note

Externally, this is your quality gate for memory. Internally, Ember is the
component name that implements the workflow.

---

**Next:** [Canonical Memory (Edda) →](/edda-stack/components/edda)
