# APS Always-On Skill

**Date:** 2026-03-01 **Status:** Approved **Replaces:**
`.claude/agents/anvil-plan-spec.md` (repurposed as background subagent)

## Problem

APS plan statuses go stale because updating them is a manual, separate step from
the work itself. Work gets done — commits land, PRs merge, branches complete —
but nobody invokes the APS agent to update the plan files. The result is modules
marked Draft that are actually Complete, progress counts that are wrong, and an
index that misleads rather than informs.

The root cause: **plan awareness is disconnected from development work.**

## Solution

An always-on APS skill that loads plan context at session start, maintains
awareness throughout development work, and delegates bookkeeping to a background
agent at session boundaries.

### Design Principles

1. **Non-intrusive** — never interrupt mid-flow; flag relevance at natural
   pauses
2. **Background-first** — heavy scanning and reconciliation runs async
3. **Confirmation required** — never auto-edit plan files without user approval
4. **No-op safe** — does nothing if `plans/index.aps.md` doesn't exist

## Architecture

```
User action
  -> Skill (always-on, lightweight, in main context)
    -> Recognises relevance or explicit command
    -> Spawns anvil-plan-spec agent via Task tool
      -> Background: reconciliation, status sync, drift detection
      -> Foreground: /plan-status reports, work item creation, execution
    -> Reports results back to user
```

### Components

| Component     | Location                                 | Purpose                                    |
| ------------- | ---------------------------------------- | ------------------------------------------ |
| APS skill     | `~/.claude/skills/aps-planning/SKILL.md` | Global, always-on awareness                |
| APS agent     | `.claude/agents/anvil-plan-spec.md`      | Background subagent for heavy lifting      |
| Project rules | `.claude/rules/aps-project.md`           | Per-repo module IDs, file map, conventions |

## Detailed Design

### 1. Skill Identity

- **Name:** `aps-planning`
- **Location:** `~/.claude/skills/aps-planning/SKILL.md` (global)
- **Trigger:** Session start on any project with `plans/index.aps.md`; user says
  `/plan`, `/plan-status`, or mentions APS/work items; user is doing
  implementation work in a project with active APS modules
- **No-op:** If `plans/index.aps.md` doesn't exist, passive awareness does
  nothing. Explicit `/plan` invocations still work and can bootstrap a new APS
  index.

### 2. Session Start — Context Loading

When the skill activates, it builds a compact APS Context Block:

1. Read `plans/index.aps.md` — extract active modules table (non-Complete,
   non-Archived)
2. For each active module, read the `.aps.md` file — extract work items with
   status, files, and validation commands
3. Build a file-to-item map (e.g., `forge-defer.sh -> [PBLU-020, PBLU-021]`)
4. Produce a summary injected into working memory:

```
## APS Context (auto-loaded)
Active modules: PBLU (27/57), CRB (3/25), ANVFMT (in progress), BMAD4 (0/8)

In-progress items:
  (none currently assigned)

File map (12 tracked paths):
  .github/workflows/claude-code-review.yml -> PBLU-002
  .claude/agent-bus/forge-defer.sh -> PBLU-020, PBLU-021
  ...

Next suggested: PBLU-005 (no dependencies, high priority)
```

This is read-only — no file edits at session start. The context stays in memory
and informs decisions throughout.

### 3. During Work — Natural Awareness

Three passive behaviours, no forced interruptions:

**Recognise relevance.** When editing or committing files that appear in the
file map, mentally note which work item(s) they relate to. Don't interrupt —
just hold the association.

**Flag completion opportunities.** When work is clearly done (tests pass, PR
merged, commit pushed) and it maps to a tracked work item, mention it naturally:

> "That commit touches `forge-defer.sh` which relates to PBLU-020 (jq for JSON
> output). Want me to update its status?"

Only flag — never auto-update without confirmation.

**Track unplanned work.** If implementation work doesn't map to any existing
work item but falls within an active module's scope, note it at the next natural
pause:

> "The refactoring you just did to the export formatter isn't tracked in any APS
> item. Want me to add it as a new work item to ANVFMT?"

**What the skill does NOT do:**

- Interrupt mid-flow to ask about plans
- Auto-edit any plan files without confirmation
- Run validation commands during active work
- Slow down the commit/PR workflow

### 4. Session Boundaries — Background Reconciliation

At natural boundaries (commit, PR creation, branch completion), the skill spawns
a background agent to do the heavy bookkeeping. This runs async — the user isn't
blocked. Explicit `/plan-status` invocations run in the foreground since the
user is waiting for the report (see section 6).

**The background agent does:**

1. **Validation scan** — for each work item that declares a `Validation:`
   command (including Complete items, to detect regressions), surface the
   command to the user for confirmation before executing. Only allowlisted safe
   commands (`pnpm test`, `pnpm lint`, `npm test`, etc.) may run without
   explicit approval. If the validated check passes and the item is not already
   Complete, propose marking Complete.
2. **Status sync** — update work item statuses in the module `.aps.md` files
   (with user-confirmed changes only, or auto-apply if the user pre-approved a
   batch).
3. **Progress count sync** — recalculate `X/Y` progress in `index.aps.md` by
   counting Complete vs total items per module.
4. **Drift detection** — flag inconsistencies: items marked Complete whose
   validation now fails, items marked Draft whose validation passes, module
   status that doesn't match its items.
5. **Report** — produce a short summary of what changed and what needs
   attention.

**Output format:**

```
APS Reconciliation (background)
  Updated: PBLU-020 Draft -> Complete (validation passed)
  Updated: index.aps.md PBLU progress 27/57 -> 28/57
  Drift: PBLU-002 marked Complete but validation fails (id-token still present)
  No action needed: CRB (3/25), ANVFMT, BMAD4
```

**Agent type:** Uses the existing `anvil-plan-spec` agent definition (repurposed
as a subagent rather than standalone), invoked via the `Task` tool with
`subagent_type: anvil-plan-spec` and `run_in_background: true`.

### 5. Project Rules File

Each repo can optionally have `.claude/rules/aps-project.md` to provide
project-specific context the global skill can't infer. Generated during the
first explicit reconciliation step (not on passive session-start activation) if
`plans/index.aps.md` exists but the rules file doesn't — the user is prompted
before any file is created.

**Contents:**

```markdown
# APS Project Rules

## Active Modules

<!-- Auto-generated from index.aps.md, refreshed by reconciliation agent -->

- PBLU: post-beta-launch-uplift (27/57)
- CRB: code-review-backlog (3/25)
- ANVFMT: anvil-file-format (in progress)
- BMAD4: bmad-v4-backward-compat (0/8)

## Conventions

- UK English spelling in all plan text
- Work item IDs: PREFIX-NNN (3-digit zero-padded)
- Module statuses: Proposed -> Ready -> In Progress -> Done -> Blocked (legacy
  aliases: Draft=Proposed, Complete=Done)
- Plans live in plans/modules/\*.aps.md
- Decisions live in plans/decisions/NNN-\*.md

## File Map

<!-- Extracted from work item Files: fields -->

.github/workflows/claude-code-review.yml: PBLU-002
.claude/agent-bus/forge-defer.sh: PBLU-020, PBLU-021
apps/anvil-cli/src/commands/plan.ts: PBLU-003
```

This file is checked into the repo (not gitignored). The reconciliation agent
keeps it fresh. Other developers (or future sessions) get immediate APS context
without needing to scan all modules.

### 6. Replacing the Agent

**What happens to `anvil-plan-spec.md` agent:**

- Kept as a file but repurposed as the background subagent the skill spawns
- No longer invoked directly by the user — the skill is the entry point
- The agent's heavy capabilities (create modules, draft work items, execute
  items, wave planning, validation) remain intact — the skill delegates to it

**Command integration:**

- `/plan` — invokes the skill, which delegates to the background agent for
  creation/execution tasks
- `/plan-status` — invokes the skill, triggers immediate reconciliation
  (foreground, not background, since the user is explicitly asking)

**What changes for the user:**

- No more manually remembering to invoke the APS agent after work is done
- Plan statuses stay current automatically
- `/plan` and `/plan-status` continue to work as before but route through the
  skill

## Success Criteria

1. After a session where work touches APS-tracked files, the relevant work item
   statuses are updated (with user confirmation)
2. Progress counts in `index.aps.md` are never more than one session stale
3. Drift between actual codebase state and plan statuses is detected and flagged
4. The skill adds zero latency to normal development work (all heavy ops are
   background)
5. Projects without `plans/index.aps.md` are completely unaffected
