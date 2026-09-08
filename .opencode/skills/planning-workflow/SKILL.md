---
name: planning-workflow
description: Turn intent into approved, validated work by coordinating truth discovery, design Q&A, APS updates, readiness validation, and handoff to dev-loop.
---

# Planning Workflow

## Purpose

Use this skill before implementation when the user has a goal, feature, fix, or
workflow change that is not already a clearly valid ready work item.

This skill is the planning orchestrator. It does not replace specialist skills:

- Use `grill-design` for unclear scope, behaviour, architecture, ownership, or
  user experience.
- Use `aps-planning` for APS truth validation, status, drift, and reconciliation.
- Use `plan-ready` for implementation plans after design approval.
- For high-risk or cross-boundary design, obtain independent bounded specialist
  views through runtime-native read-only delegation when available, then
  synthesise them explicitly; otherwise run the same perspectives sequentially.

This workflow never writes code and never creates branches. All APS plan-file
edits go through `aps-planning`; this workflow does not edit plan files directly.

Planning is interactive by default. For loop-worthy projects, this workflow owns
the user-facing intent, discovery, design, and readiness decision; `dev-loop`
owns autonomous execution only after that decision is approved.

## Activation

Invoke this skill when:

- The user asks to plan work.
- The work does not map cleanly to one existing ready item.
- `dev-loop` or `aps-planning` returns `needs-plan-update`.
- The goal touches architecture, scope, CI, release, security, docs authority,
  feature flags, branch policy, or multiple modules.
- A plan exists but may be stale relative to current project truth.

If the task already has one validated ready item and no drift is found, return to
`dev-loop` instead of replanning.

## Workflow

### 1. Goal Intake

Capture the user's requested outcome, success criteria, constraints, urgency,
and known non-goals. Ask one concise clarifying question only when the next
planning step would otherwise be guesswork.

### 2. Project Truth Discovery

Check current source, tests, docs, ADRs, workflows, release state, and feature
flags relevant to the goal. Prefer current implementation truth over stale plan
prose, but preserve plan intent unless it conflicts with shipping behaviour or
accepted decisions.

### 3. Existing Work Match

Decide whether the goal maps to one existing item, updates an item, supersedes or
splits work, needs a new item/module, or is out of scope.

### 4. Design Gate

Invoke `grill-design` before plan writing when the goal changes behaviour,
architecture, UX, ownership, security posture, feature flags, release policy, or
system boundaries. For high-risk or cross-boundary work, gather and reconcile
multiple independent specialist perspectives using only capabilities the active
runtime actually provides.

### 5. Plan Synthesis

Use `plan-ready` for approved implementation plans. If the project uses APS,
use `aps-planning` and the project's APS agent/tooling to draft or update module
and work item text.

### 6. Readiness Validation

Run the project's readiness check. In APS projects, run `aps-planning` APS Truth
Validation. The plan is ready only when validation evidence is defined and
dependencies are closed or explicitly documented.

### 7. Handoff

Return this handoff block. This handoff is a planning summary, not a ReadyItem —
dev-loop's plan stage still produces the ReadyItem via `plan-ready`.

```markdown
## Planning Workflow Handoff

- Goal:
- Work item:
- Status:
- Design source:
- Dependencies:
- Files:
- Validation commands:
- Risks:
- Decision: ready-for-dev | needs-design | needs-plan-update | blocked | out-of-scope
- Next skill: dev-loop | dev-loop drain | grill-design | aps-planning | plan-ready | stop
```

`ready-for-dev` hands back to `dev-loop`. All other decisions stop before
implementation.

Use `dev-loop` drain mode as the next skill only when the user has approved autonomous
execution of the ready APS plan. Otherwise hand off to `dev-loop` for one
interactive implementation slice.

## Loop Readiness

Before handing to `dev-loop` drain, confirm these conditions explicitly:

- the user approved the design or readiness decision;
- the APS item or action plan is in scope;
- the expected outcome is testable;
- validation evidence is defined;
- dependencies are closed or documented;
- checkpoint boundaries are clear.
