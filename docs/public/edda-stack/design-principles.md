---
id: design-principles
title: Memory System Design Principles
description:
  Principles for trustworthy, reusable development memory at team scale.
sidebar_position: 2
owner: DOCSYNC
upstream:
  - packages/edda-stack/README.md
verified_against: 0.9.4-beta
---

# Design Principles

These principles exist to protect one thing: memory quality over time.

## 1. Signal over volume

More notes are not automatically better memory.

- Capture broadly, curate narrowly.
- Only promote entries that are reusable and evidenced.
- Treat canonical memory as a scarce resource.

## 2. Human accountability is mandatory

AI can suggest; people decide.

- Promotion requires human rationale and attribution.
- Confidence is asserted by reviewers, not inferred by heuristics.
- Governance is explicit, not implied.

## 3. Provenance must be queryable

Every canonical entry needs a traceable chain.

- Where did this come from?
- Who approved it?
- What changed since last version?

If these questions cannot be answered quickly, trust degrades.

## 4. Retrieval is deterministic first

Teams need predictable behaviour before clever behaviour.

- Exact and structured retrieval come first.
- Ranking and semantic enrichment can be added later.
- Debuggable retrieval beats opaque retrieval.

## 5. Low-friction capture, high-rigour curation

Capture should be quick enough to do during active work, while promotion should
be deliberate enough to avoid polluting canonical memory.

This asymmetry is intentional.

## 6. Local-first and portable by default

Teams should keep control over their memory data.

- Works without central infrastructure
- Compatible with normal tooling (files, git, CLI)
- Portability by design, not afterthought

## 7. Version everything that matters

Canonical memory should evolve through explicit supersession, not silent edits.

- Preserve history
- Keep previous versions queryable
- Make organisational learning auditable

## 8. Compose incrementally

Teams can start small and still get value.

- Start with capture
- Add candidate review when ready
- Add canonical governance when the team needs higher trust

---

**Back to:** [Development Memory System Overview →](/edda-stack/overview)
