---
id: edda
title: Canonical Memory (Edda)
description:
  Maintain trusted, attributable, versioned memory for teams and agents.
sidebar_position: 3
owner: DOCSYNC
---

# Canonical Memory (Edda)

Edda is the canonical memory capability. It stores team-approved knowledge that
should persist, stay auditable, and remain easy to query.

## Why this capability matters

- Faster decision-making with trusted prior context
- Fewer repeated incidents caused by forgotten lessons
- Better governance for AI-assisted and human-assisted changes

## What makes memory canonical

Canonical entries are:

- Human-approved
- Attributable (who, when, why)
- Provenance-linked to source observations
- Versioned with explicit supersession

## Operating contract

- **Inputs:** Human-approved candidate promotions
- **Decision point:** Promote into canonical memory or keep in review
- **Outputs:** Queryable canonical entries by memory type and status
- **Evidence:** Attribution metadata, provenance chain, and version history

## Memory types

Edda supports six practical categories:

- Decision
- Pattern
- Constraint
- Warning
- Doctrine
- Lesson

These types make retrieval and policy checks predictable.

## Operator outcomes

Teams using canonical memory can answer:

- What is the current approved approach?
- Which entry superseded this one?
- What evidence supports this rule?
- Who approved this and for what reason?

## Example

```
Memory: "Payment idempotency keys are mandatory for POST /payments"
Type: Constraint
Status: Active
Confidence: High
Attribution: promoted_by=payments-owner@example.com
Provenance: ember:prop_01H..., kindling:obs_01H...
```

## Positioning note

Externally, this is governed team memory. Internally, Edda is the storage and
service layer implementing that guarantee.

---

**Next:** [Roadmap →](/edda-stack/roadmap)
