---
id: terminology
title: APS glossary
description: Plain-language definitions for the terms used in APS documentation.
sidebar_position: 5
owner: DOCSYNC
---

# APS glossary

You can start using APS without memorising these terms. Return here when a guide
or command introduces an unfamiliar word.

| Term                   | Meaning                                                                                                         |
| ---------------------- | --------------------------------------------------------------------------------------------------------------- |
| **Index**              | The entry point that explains the whole plan and links its modules. It is descriptive, not execution authority. |
| **Module**             | A bounded area of responsibility containing related work items.                                                 |
| **Work item**          | One authorised, observable outcome with a validation method.                                                    |
| **Action plan**        | An optional breakdown of complex work into actions and checkpoints.                                             |
| **Action**             | One coherent part of an action plan.                                                                            |
| **Checkpoint**         | Observable evidence that an action reached its intended state.                                                  |
| **Intent**             | Why a work item exists and what it aims to achieve.                                                             |
| **Expected outcome**   | The result that must be observable when the item is complete.                                                   |
| **Validation**         | The command or check that proves the expected outcome.                                                          |
| **Dependency**         | Another item or module that must be complete first.                                                             |
| **Context package**    | A focused, generated brief written when `aps start` claims an item.                                             |
| **Learning**           | A durable observation recorded at completion for downstream work.                                               |
| **Conductor module**   | A module that coordinates work owned by several other modules without taking over their implementation.         |
| **Tagged monorepo**    | One plan tree whose modules or items name the packages they affect.                                             |
| **Federated monorepo** | A root plan that links independently owned child plan trees.                                                    |
| **Toolchain pin**      | The expected APS CLI version stored in `.aps/config.yml`.                                                       |

## Statuses

| Status        | Meaning                                                         |
| ------------- | --------------------------------------------------------------- |
| `Draft`       | The item or module is still being shaped and is not executable. |
| `Ready`       | Its boundaries and validation are clear enough to execute.      |
| `In Progress` | Someone has claimed the work.                                   |
| `Blocked`     | Execution cannot continue until a named condition changes.      |
| `Complete`    | The outcome and validation have been recorded as finished.      |

`Proposed` is accepted as a draft alias and `Done` as a completion alias. New
portable plans should prefer the canonical statuses above.

## Next step

Return to [what APS does](overview.md) or review the
[document hierarchy](spec/taxonomy.md).
