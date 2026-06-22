---
id: terminology
title: Terminology
description:
  APS vocabulary — work items, action plans, actions, and checkpoints.
sidebar_position: 5
---

# APS Terminology

| Type        | Authority | Owner   | Status | Freshness                                               |
| ----------- | --------- | ------- | ------ | ------------------------------------------------------- |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream            |
| ------------------------------------------------------------------------- | --------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

This page defines the words APS uses and what they mean.

## Renamed concepts (v0.2.0+)

| Old term        | Current term | Meaning                                                            |
| --------------- | ------------ | ------------------------------------------------------------------ |
| Task            | Work Item    | A bounded unit of work with intent, outcome, scope, and validation |
| Step (file)     | Action Plan  | Execution breakdown for a work item                                |
| Step (numbered) | Action       | A coherent unit of execution within a plan                         |
| Checkpoint      | Checkpoint   | Observable proof that an action is complete                        |

New projects start with current terminology. Upgrading from pre-v0.2 projects:

- **Task** sections → **Work Items**
- **`.steps.md`** files → **`.actions.md`**
- Numbered **steps** → **Actions**

No content needs rewriting unless it violates checkpoint rules.

## Core principle

- **Specs describe intent.**
- **Work items define outcomes.**
- **Action plans decompose execution.**
- **Checkpoints verify state.**

Implementation emerges from patterns and agent judgement.

## Hierarchy

| Layer       | Purpose             | You write                          | You do not write      |
| ----------- | ------------------- | ---------------------------------- | --------------------- |
| Index       | Plan overview       | Modules, milestones, risks         | Implementation detail |
| Module      | Bounded work area   | Interfaces, boundaries, work items | Code snippets         |
| Work Item   | Outcome unit        | Intent, outcome, validation        | How to implement      |
| Action Plan | Execution breakdown | Actions, checkpoints               | Tutorials             |
| Action      | Unit of work        | Purpose, produces, checkpoint      | Code structure        |
| Checkpoint  | Verification        | Observable state                   | Implementation steps  |

## Work items

A work item is the contract: what will be achieved and how success is verified.

### Required fields

- **Intent** — One sentence describing the outcome
- **Expected Outcome** — Testable or observable result
- **Validation** — Command or condition to verify completion

### Optional fields

- Scope / Non-scope
- Dependencies
- Confidence (`low`, `medium`, `high`)
- Files (best-effort)
- Risks
- Deliverables

### Rules

- Work items describe **what must be true**, not how it is achieved
- Work items authorise change but do not prescribe implementation
- Validation must be deterministic where possible

## Action plans

An action plan decomposes a work item into executable actions.

### When to create one

- The work item is non-trivial
- Multiple artefacts are produced
- Ordering or dependencies matter

Simple work items may be executed without an action plan.

### File naming

```
plans/execution/{WORK-ITEM-ID}.actions.md
```

Example: `AUTH-001.actions.md`

## Actions and checkpoints

Each action represents a coherent unit of execution, not a tutorial.

```markdown
## Action 1 — Create authentication middleware

**Purpose** Validate requests and attach user context.

**Produces** Auth middleware module.

**Checkpoint** Auth middleware validates requests, attaches user to context

**Validate** `npm test -- auth.middleware.test.ts`
```

### Checkpoint rules

- Must be verifiable by inspection or command
- Must avoid implementation detail
- Must remain valid even if implementation changes

✅ **Good:** "Authenticated requests attach user context"

❌ **Bad:** "Create mapping.ts with switch statement"

## Module types

Most modules are **vertical** — they own a single domain end-to-end.

A **conductor** (crosscutting) module coordinates work _across_ vertical modules
— release cuts, security audits, perf budgets. Mark it with `Type: Conductor` in
the metadata table. See
[Conductor modules](https://github.com/EddaCraft/anvil-plan-spec/blob/main/docs/conductor-modules.md)
in the source repository.

---

**Next:** [Taxonomy →](./spec/taxonomy.md)
