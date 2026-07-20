---
id: ai-agents
title: Use APS with an AI agent
description:
  Give an AI coding agent bounded, validated work without tying the plan to one
  tool.
sidebar_position: 1
---

# Use APS with an AI agent

APS works without an AI tool. When you do use one, the plan stays portable: the
agent reads the same Markdown that a person reviews.

## Set up an integration

Choose integrations during `aps init`, or add one later:

```bash
aps setup codex
```

Available tool keys include `claude-code`, `copilot`, `codex`, `opencode`,
`grok`, and `generic`. Run `aps setup` for the guided picker.

Tool setup adds guidance for that client; it does not move the plan into the
client or make the client the source of truth.

## Prepare work before prompting

1. Run `aps lint`.
2. Use `aps next` to find an item whose dependencies are complete.
3. Review its scope, non-scope, expected outcome, and validation.
4. Run `aps start <ID>` to claim it and generate focused context.
5. Give the agent the work-item ID and context-package path.

Example prompt:

```text
Implement work item AUTH-003.
Read .aps/context/AUTH-003.md and the source module it names.
Stay within the stated scope and non-scope.
Run the work item's validation command and report the evidence.
Do not mark the item complete if validation fails.
```

## Keep authority clear

- A chat request is not a substitute for a ready work item.
- The plan defines the outcome; the repository defines implementation truth.
- The agent may propose new work, but new scope starts as draft.
- Validation evidence comes before completion.
- One agent should own one in-progress item unless the plan explicitly
  coordinates several owners.

## Close the item

After reviewing the implementation and fresh validation evidence:

```bash
aps complete AUTH-003 --learning "The session boundary owns token renewal"
```

The learning becomes durable planning context for dependent work.

## Optional hooks

APS can install hooks that remind supported clients to read or update plans.
Treat them as workflow assistance, not as security controls. Repository
permissions, review, tests, and CI still enforce the real boundaries.

## When not to automate

Keep a person in the loop when the work changes architecture, security or data
boundaries, public contracts, destructive operations, or unclear product
behaviour. Clarify the plan before expanding agent authority.

See the [workflow](../workflow.md) for the full lifecycle and the
[glossary](../terminology.md) for planning terms.
