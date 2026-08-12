---
id: workflow
title: Run the APS workflow
description: Select, start, validate, and complete one bounded work item.
sidebar_position: 4
owner: DOCSYNC
---

# Run the APS workflow

**For:** people or agents working from an existing APS plan

**Time:** the duration of one work item

**Outcome:** one validated outcome recorded as complete, with the next ready
item visible

## Before you begin

Run `aps lint` and resolve errors. The work item you intend to execute must be
`Ready`, its module must be active, and its dependencies must be complete.

## 1. Find ready work

```bash
aps next
```

This command is read-only. It chooses the first ready item whose dependencies
are satisfied. Use a module name to narrow the queue:

```bash
aps next auth
```

## 2. Claim one item

```bash
aps start AUTH-003
```

On success APS:

- changes the item from `Ready` to `In Progress`;
- reports the file it changed;
- suggests a branch name without creating it; and
- writes `.aps/context/AUTH-003.md` with the focused planning context.

Commit the status change with the implementation so the plan and code travel
together.

## 3. Implement only the authorised outcome

Read the work item's intent, expected outcome, non-scope, dependencies, and
validation. An action plan is optional; use one when the work needs several
independently verifiable checkpoints.

If you discover separate work, record a new draft item instead of silently
expanding the active item.

## 4. Run the declared validation

Execute the exact command in the work item's `Validation` field. A passing
unrelated test is not completion evidence.

## 5. Record completion

```bash
aps complete AUTH-003 --learning "Retry behaviour belongs at the client boundary"
```

APS requires the item to be `In Progress`, stamps the completion date, and
records the optional learning beside the validation field. Downstream work can
then receive that learning in its context package.

## 6. Continue the queue

```bash
aps lint
aps next
```

The loop is deliberately small:

```text
lint → next → start → implement → validate → complete → lint
```

## When the loop stops

- **No ready item:** a dependency, decision, or module status still blocks the
  queue.
- **Start is rejected:** read the named state or dependency and repair the plan;
  do not force the transition.
- **Validation fails:** leave the item in progress and fix the result.
- **The outcome changed:** return the item to planning through a deliberate edit
  and review.

Use [validation and audit](spec/determinism.md) for CI and plan-versus-project
checks.
