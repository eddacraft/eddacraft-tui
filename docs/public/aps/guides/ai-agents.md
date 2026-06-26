---
id: ai-agents
title: AI Agents
description: Guide for AI agents working with APS specs.
sidebar_position: 2
---

# AI Agent Guide

| Type        | Authority | Owner   | Status | Freshness                                               |
| ----------- | --------- | ------- | ------ | ------------------------------------------------------- |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream            |
| ------------------------------------------------------------------------- | --------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

This guide is for LLMs and AI agents working with APS. For human-readable
onboarding, see [Getting Started →](../getting-started.md).

## Quick decision tree

```text
Is there a plans/ directory?
├─ NO  → Initialize APS (aps init or scaffold/install)
├─ YES → Does plans/index.aps.md exist?
   ├─ NO  → Create index
   ├─ YES → Read index, identify request type:
      ├─ Planning request → Generate/update specs
      ├─ Implementation request → Execute work items
      └─ Question → Read relevant specs, answer from context
```

## Core principles

1. **Never implement without a work item** — Create one first or ask
2. **Always read before you write** — Check existing specs and code patterns
3. **Work items are permission** — Status must be `Ready` before execution
4. **Checkpoints over instructions** — Write what should exist, not how to
   create it
5. **One work item at a time** — Complete and validate before moving to next

## Planning workflow

When asked to plan a feature:

1. Read `plans/index.aps.md`, `plans/aps-rules.md`, and existing modules
2. Determine scope:
   - Small (1–3 items) → `simple.template.md`
   - Medium (4–8 items) → `module.template.md`
   - Large (multiple modules) → update index, create leaf modules
3. Create/update module spec with Purpose, Scope, Interfaces
4. **Do not write work items** until module is Ready and scope is clear
5. Add work items with Intent, Expected Outcome, Validation
6. Run `aps lint plans/` to validate

## Execution workflow

When asked to implement:

1. Find the authorised work item — status must be `Ready`
2. If the bash fallback/vendored runtime is available, run `aps start <ID>` to
   claim it and get the context package; otherwise hand-edit the status to
   `In Progress`
3. Read `.aps/context/<ID>.md` for focused brief when generated
4. If complex, open or create `{ID}.actions.md`
5. Execute one action at a time; validate each checkpoint
6. Run the work item's Validation command
7. Mark the item `Complete: YYYY-MM-DD`; with the bash fallback/vendored
   runtime, prefer `aps complete <ID> --learning "..."`

## Prompting a work item

```text
You have a work item authorised at plans/modules/auth.aps.md (AUTH-003).
Context package: .aps/context/AUTH-003.md.
Implement it, run the validation step, and report back.
```

This prompt works in Claude Code, Cursor, Copilot, Codex, OpenCode, Gemini, or
pasted into ChatGPT.

## Agent definitions

APS ships first-class agent definitions for:

- Claude Code (`.claude/agents/`)
- GitHub Copilot (`.github/agents/`)
- OpenCode (`.opencode/agents/`)
- Codex (`.codex/config.toml`)
- Gemini (`.gemini/`)

Install with `aps setup <tool>` or select during `aps init`.

## Prompts

Tool-agnostic prompts ship in
[anvil-plan-spec `docs/ai/prompting/`](https://github.com/EddaCraft/anvil-plan-spec/tree/main/docs/ai/prompting):

| Task               | Prompt file           |
| ------------------ | --------------------- |
| Planning           | `index.prompt.md`     |
| Module design      | `module.prompt.md`    |
| Work item creation | `work-item.prompt.md` |
| Execution          | `actions.prompt.md`   |

## aps-rules.md

`aps init` scaffolds `plans/aps-rules.md` — a portable guide that travels with
your specs. Point your agent at it and APS conventions are followed by default.

The canonical source is
[`scaffold/plans/aps-rules.md`](https://github.com/EddaCraft/anvil-plan-spec/blob/main/scaffold/plans/aps-rules.md)
in anvil-plan-spec.

## MCP integration

For MCP-capable agents, register the APS MCP server:

```json
{
  "mcpServers": {
    "aps": {
      "command": "node",
      "args": ["/path/to/anvil-plan-spec/mcp/src/index.ts"],
      "env": { "APS_PLANS": "/path/to/your/project/plans" }
    }
  }
}
```

See [CLI Reference →](../tooling/validation.md) for details.

---

**Back to:** [APS Overview →](../overview.md)
