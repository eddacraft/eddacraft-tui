---
id: taxonomy
title: How APS documents fit together
description: Understand indexes, modules, work items, and optional action plans.
sidebar_position: 1
---

# How APS documents fit together

APS separates strategic intent from execution authority so a large plan can stay
readable without making every idea immediately actionable.

```text
index
└── module
    └── work item
        └── action plan (optional)
```

## Index

The index is the plan entry point. It explains the problem, success criteria,
constraints, and module map.

An index does not authorise implementation. Its module links help people and
tools find the files that do.

## Module

A module groups work that shares one boundary. It states why that area exists,
what is in and out of scope, which interfaces matter, and whether the module is
ready for execution.

A module should be large enough to have a meaningful boundary and small enough
that one owner can keep its intent coherent.

## Work item

A work item is the unit of execution authority. An active item needs:

- a `PREFIX-NNN` identifier;
- a status;
- an intent;
- an expected outcome; and
- a validation method.

Optional fields can name dependencies, affected files or packages, non-scope,
confidence, and risks.

One work item should produce one reviewable outcome. Split items that can fail,
ship, or be reviewed independently.

## Action plan

An action plan is an optional execution breakdown. Use one when the work needs
several checkpoints, ordered waves, or coordination between people or agents.

Each action states what it produces and how its checkpoint can be observed. It
does not need to prescribe every implementation keystroke.

## Execution direction

Author plans from broad to specific:

```text
problem → index → modules → ready work items
```

Execute from specific to broad:

```text
work item → validation → module progress → plan outcome
```

## Choosing the smallest useful shape

| Situation                            | Start with                         |
| ------------------------------------ | ---------------------------------- |
| One small outcome                    | One index and one module           |
| Several outcomes in one bounded area | One module with several work items |
| Several bounded areas                | An index with multiple modules     |
| Complex execution inside one item    | Add an action plan                 |
| One shared monorepo backlog          | Add package tags                   |
| Independently owned package backlogs | Add child plans                    |

See the [file layout](file-layout.md) for where each document lives.
