---
name: plan-ready
description: >-
  Turn a goal or stale plan into a single ReadyItem with validation commands.
  Use for natural-language goals, needs-plan-update, or before isolate/build in
  the development loop. Coordinates design grilling and APS when present.
---

# Plan ready

Produce one authorised **ReadyItem** (see `references/contracts.md`). Never write
product code. Never create feature branches.

## When

- User has a goal/feature/fix that is not already a validated Ready item.
- `dev-loop-core` receives `goal <text>` or `needs-plan-update`.
- APS truth validation fails or maps to zero/many items.
- Scope, validation, or dependencies are unclear.

If one Ready item is already valid and drift-free, exit immediately with that
item and next `isolate-workspace` (or back to `dev-loop-core`).

## Hard rules

1. Prefer **project truth** (code, tests, CI, ADRs) over stale plan prose.
2. One primary work item per ReadyItem. Split multi-subsystem goals first.
3. `ready` requires **exact validation commands** — no TBD.
4. Interactive by default: user owns the Ready membrane unless `dev-loop-core`
   policy already granted authority for this target.
5. In APS projects, load and truth-check via `aps-planning`; do not invent
   parallel plan stores.

## Steps

### 1. Goal intake

Capture outcome, success criteria, constraints, urgency, non-goals. Ask one
clarifying question only when the next step would otherwise be guesswork.

### 2. Project truth

Read relevant source, tests, docs, ADRs, workflows, flags, release state.

### 3. Existing work match

Decide: maps to one item | update item | supersede/split | new item | out of scope.

If APS exists (`plans/index.aps.md`):

1. Invoke `aps-planning` truth validation for the candidate item.
2. On drift or ambiguity → correct plan text (propose, get approval) or return
   `needs-plan-update` / `needs-design`.

### 4. Design gate

Invoke `grill-design` when the work changes behaviour, architecture, UX,
ownership, security posture, release policy, boundaries, **product framing**,
or **optional scope** (“do we need X?”) — or when risk is high/critical.

Skip only when the approach is **already decided and recorded** (design doc,
ADR, or explicit user decision in-thread). **Do not** skip as “mechanical
migration” if framing, vendor choice beyond a version pin, or optional
components are still fuzzy — a short grill beats post-hoc doc reframing.

### 5. Synthesise ReadyItem

Fill the ReadyItem block from `references/contracts.md`.

For multi-step work, list **lean actions** under Expected behaviour or as APS
action lines — each action should be testable. Do **not** write a separate
essay-length execution plan unless the user asks; the ReadyItem + APS item are
the plan of record.

### 6. Readiness check

Ready only when:

- [ ] Expected behaviour is observable
- [ ] Validation commands are exact and runnable
- [ ] Dependencies closed or explicitly documented
- [ ] Design approved if the design gate fired
- [ ] User (or policy) authorises Ready

Propose APS status `Ready` only after the above; never silent auto-promote
without authority.

## Exit

Return the ReadyItem plus:

```markdown
## Exit

- Decision: ready | needs-design | needs-plan-update | blocked | out-of-scope
- Next: isolate-workspace | grill-design | aps-planning | dev-loop-core | stop
- Notes:
```

For `dev-loop-core goal` flows: `ready` continues the loop; other decisions stop
implementation.

## Non-goals

- Not isolation, TDD, verify, or land.
- Not autonomous backlog draining (`dev-loop-core` / older `aps-loop`).
- Not formal multi-persona design (`planning-council` when required).
