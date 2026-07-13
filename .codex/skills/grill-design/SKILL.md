---
name: grill-design
description: >-
  Stress-test intent and design with one-at-a-time Q&A before plan authorisation.
  Use when scope, behaviour, architecture, UX, or approach is unclear, or when
  plan-ready / dev-loop raises needs-design.
---

# Grill design

Turn fuzzy intent into an approved approach. No code. No branches.

Borrowed shape: relentless single-question grilling (mattpocock `grilling` /
brainstorming Q&A). Stripped of hard-gates that force a specific plan skill.

## When

- Behaviour, architecture, ownership, UX, security posture, or system boundary is unclear.
- `plan-ready` or `dev-loop` returns `needs-design`.
- User asks to design, explore options, or "grill me on this".

## Hard rules

1. **One question per message.** Multiple questions at once are forbidden.
2. Prefer **multiple choice** with a **recommended answer** and one-line why.
3. If the codebase can answer, **read the code** instead of asking.
4. Walk the design tree branch by branch; resolve dependencies in order.
5. Do **not** write implementation code, scaffolds, or commits.
6. Stop when the user approves a design summary — then hand off to `plan-ready`.

## Steps

### 1. Context pass

Read relevant code, docs, ADRs, and any existing APS item. Note established
patterns. If the request spans independent subsystems, say so and help
decompose before deep Q&A.

### 2. Facts vs decisions

Separate:

- **Facts** — discoverable from repo or tools (do not grill these).
- **Decisions** — require human choice (grill these).

### 3. Question loop

For each open decision:

1. State the decision in one line.
2. Offer 2–3 options; mark your recommendation.
3. Wait for the answer.
4. Record the decision; only then move to the next dependency.

### 4. Approaches (when still open)

If more than one viable architecture remains, present 2–3 approaches with
trade-offs and a recommendation. Get explicit pick.

### 5. Design summary

Present a short design the user can approve:

```markdown
## Design summary

- Goal:
- Approach (chosen):
- Key decisions:
- Interfaces / boundaries:
- Risks / non-goals:
- Open questions: none | <list>
```

Scale length to risk: a few bullets for small work; sections for high-risk.

### 6. Persist when useful

For non-trivial work, write `plans/specs/YYYY-MM-DD-<topic>.md` (or the
project's spec path) and commit if the user wants it durable. For trivial
work, the summary in chat is enough if the user approves.

## Exit

```markdown
## Exit

- Decision: design-approved | needs-more-grilling | blocked | out-of-scope
- Next: plan-ready | stop
- Notes: <path to design doc if written>
```

## Non-goals

- Not a planning skill (no ReadyItem, no APS edits) — that is `plan-ready`.
- Not multi-persona formal design — use `planning-council` when the project requires it.
- Not implementation, TDD, or branches.
