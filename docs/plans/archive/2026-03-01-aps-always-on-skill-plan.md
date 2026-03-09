# APS Always-On Skill Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to
> implement this plan task-by-task.

**Goal:** Create a global APS skill that loads plan context at session start,
maintains awareness during development work, and delegates bookkeeping to a
background agent at session boundaries.

**Architecture:** Global skill at `~/.claude/skills/aps-planning/SKILL.md` with
per-repo project rules at `.claude/rules/aps-project.md`. The existing
`anvil-plan-spec.md` agent is repurposed as the background subagent for heavy
reconciliation tasks. The `/plan` and `/plan-status` commands are updated to
route through the skill.

**Tech Stack:** Claude Code skills (markdown SKILL.md), Claude Code agents
(markdown agent definition), Claude Code rules (markdown rules file), shell
scripting for any helper scripts.

**Design doc:** `docs/plans/2026-03-01-aps-always-on-skill.md`

---

## Task 1: Create the global APS skill

**Files:**

- Create: `~/.claude/skills/aps-planning/SKILL.md`

**Step 1: Create the skill directory**

Run: `mkdir -p ~/.claude/skills/aps-planning`

**Step 2: Write the SKILL.md file**

The skill needs:

- Frontmatter with `name: aps-planning` and a description covering all trigger
  conditions (session start with APS, `/plan`, `/plan-status`, work items, plan
  mentions)
- Session start context loading instructions — read `plans/index.aps.md`,
  extract active modules, read each module's work items, build file-to-item map,
  produce compact APS Context Block
- During-work awareness instructions — recognise relevance from file map, flag
  completion opportunities at natural pauses, track unplanned work
- Session boundary reconciliation instructions — when to spawn the background
  agent (commit, PR, branch completion, `/plan-status`), what to tell it
- No-op guard — if `plans/index.aps.md` doesn't exist, do nothing
- Clear "what NOT to do" section — don't interrupt mid-flow, don't auto-edit,
  don't run validation during active work

```markdown
---
name: aps-planning
description: >-
  Always-on APS awareness. Loads plan context at session start, tracks relevance
  during work, delegates reconciliation to a background agent. Triggers on:
  session start with plans/index.aps.md, /plan, /plan-status, mentions of APS
  work items, plan updates, or plan status queries.
---

# APS Planning

## Overview

Always-on awareness layer for projects using the Anvil Plan Spec (APS). Loads
plan context at session start, maintains passive awareness during development
work, and delegates bookkeeping to a background agent at session boundaries.

## Activation Guard

If `plans/index.aps.md` does not exist in the current project **and** this is a
passive activation (session start, background reconciliation), the skill does
nothing. Stop here.

If this is an explicit `/plan` invocation, the missing index is expected — the
skill should offer to bootstrap APS for the project.

## Session Start — Load Context

When this skill activates, build the APS Context Block:

1. Read `plans/index.aps.md` — extract the modules table. Identify active
   modules (status is not Complete or Archived).
2. For each active module, read its `.aps.md` file. Extract all work items with
   their ID, title, status, files, validation command, and priority.
3. Build a file-to-item map from the `Files:` fields of all non-Complete work
   items.
4. Check `.claude/rules/aps-project.md` — if it exists, read it for cached
   context. If it doesn't, note that it should be generated during the first
   reconciliation.
5. Present a compact summary to working memory (not to the user unless they
   ask):
```

## APS Context

Active: [MODULE_ID (X/Y), ...] In-progress: [ITEM-NNN: title, ...] or (none)
File map: [N tracked paths] Next suggested: [ITEM-NNN (reason)]

```

This is read-only — no file edits at session start.

## During Work — Passive Awareness

Maintain three behaviours without interrupting the user's flow:

### Recognise Relevance

When editing or committing files that appear in the file map, mentally note
which work item(s) they relate to. Do not interrupt — just hold the
association.

### Flag Completion Opportunities

When work is clearly done (tests pass, commit pushed, PR created) and it maps
to a tracked work item, mention it at the next natural pause:

> "That commit touches `path/to/file` which relates to ITEM-NNN (title). Want
> me to update its status?"

Only flag — never auto-update without confirmation. If the user confirms,
update the work item status in the module file and the progress count in the
index.

### Track Unplanned Work

If implementation work does not map to any existing work item but falls within
an active module's scope, note it:

> "The work you just did isn't tracked in any APS item. Want me to add it as a
> new work item to MODULE_ID?"

## Session Boundaries — Background Reconciliation

At natural boundaries, spawn a background agent to reconcile plan state with
reality. Trigger on:

- After a commit that touches APS-tracked files
- Before or after creating a PR
- When completing a branch
- At explicit user request

Note: `/plan-status` runs reconciliation in the **foreground** (the user is
waiting for results), not as a background task. It uses the same reconciliation
logic but returns output directly.

### How to spawn the reconciliation agent

Use the Task tool with subagent_type `anvil-plan-spec` and
`run_in_background: true`:

```

Task( subagent_type: "anvil-plan-spec", run_in_background: true, prompt: "Run
APS reconciliation for this project. For each work item (including Complete
ones) with a Validation command, run the command. Report which items pass
(propose marking Complete), which fail (keep current status), and any drift
(items already marked Complete whose validation now fails — propose reverting to
Draft). Update progress counts in index.aps.md. Generate
.claude/rules/aps-project.md if it doesn't exist. Output a short reconciliation
report." )

```

For `/plan-status`, run in the foreground (not background) since the user is
explicitly asking for the result.

### Reconciliation report format

```

APS Reconciliation Updated: ITEM-NNN Draft -> Complete (validation passed)
Updated: index.aps.md MODULE progress X/Y -> X+1/Y Drift: ITEM-NNN marked
Complete but validation fails No action needed: MODULE_ID (X/Y)

```

## Command Integration

### /plan

When the user runs `/plan`, this skill activates. Follow the existing
`/plan` command instructions (creating modules, work items, executing ready
items) but with the benefit of the loaded APS context.

### /plan-status

When the user runs `/plan-status`, trigger a foreground reconciliation scan
and present the status report. Follow the existing `/plan-status` command
format.

## What This Skill Does NOT Do

- Interrupt mid-flow to ask about plans
- Auto-edit any plan files without user confirmation
- Run validation commands during active work (only at boundaries)
- Slow down the commit or PR workflow
- Activate on projects without `plans/index.aps.md`
```

**Step 3: Verify the skill is detected**

Run: `ls -la ~/.claude/skills/aps-planning/SKILL.md` Expected: file exists with
the content above

**Step 4: Commit**

This file lives outside the repo (global `~/.claude/skills/`), so there is
nothing to commit for this task. The skill is immediately active.

---

## Task 2: Update the anvil-plan-spec agent for subagent use

The existing agent at `.claude/agents/anvil-plan-spec.md` already has all the
capabilities needed. It just needs minor updates to work well as a background
subagent spawned by the skill.

**Files:**

- Modify: `.claude/agents/anvil-plan-spec.md`

**Step 1: Read the current agent file**

Run: `cat .claude/agents/anvil-plan-spec.md` Understand the current structure
before modifying.

**Step 2: Add reconciliation mode to the agent**

Add a new section `### 12. Reconciliation Mode` after section 11 (Validate
Plans). This section tells the agent how to behave when spawned by the APS skill
for reconciliation:

```markdown
### 12. Reconciliation Mode

When spawned by the APS planning skill for reconciliation, follow this workflow:

1. Read `plans/index.aps.md` and identify all active modules
2. For each active module, read the `.aps.md` file and extract all work items
   (including Complete items, to detect regressions)
3. For each work item with a `Validation:` command:
   - Display the command and request user confirmation before executing (or
     auto-approve if the command matches the allowlist: `pnpm test`,
     `pnpm lint`, `npm test`, `npm run lint`, `cargo test`, `cargo clippy`,
     `go test`)
   - Run the validation command only after approval
   - If it passes and status is Draft/Ready, propose marking Complete
   - If it fails and status is Complete, flag as drift (regression detected)
4. Count Complete vs total items per module and update the Progress column in
   `index.aps.md`
5. Generate or update `.claude/rules/aps-project.md` with:
   - Active modules list with progress
   - File-to-item map extracted from work item `Files:` fields
   - Project conventions (read from existing rules or plans/aps-rules.md)
6. Output a reconciliation report summarising all changes and findings

When proposing status changes, make the edits directly if running in background
mode with pre-approval. Otherwise, list proposed changes and wait for
confirmation.
```

**Step 3: Verify the agent file is valid**

Run: `head -20 .claude/agents/anvil-plan-spec.md` Expected: frontmatter and
title are intact

**Step 4: Commit**

```bash
git add .claude/agents/anvil-plan-spec.md
git commit -m "feat(agents): add reconciliation mode to anvil-plan-spec agent"
```

---

## Task 3: Create the project rules template

The reconciliation agent generates `.claude/rules/aps-project.md` per-repo, but
we need to seed the template and create the initial version for this project.

**Files:**

- Create: `.claude/rules/aps-project.md`

**Step 1: Generate the initial rules file**

Read `plans/index.aps.md` and all active module files to extract:

- Active module IDs with progress counts
- File-to-item map from all non-Complete work items' `Files:` fields
- Project conventions from `plans/aps-rules.md` or `CLAUDE.md`

Write the file:

```markdown
# APS Project Rules

<!-- Auto-generated by APS reconciliation agent. Manual edits will be
     overwritten on next reconciliation. Add persistent conventions to
     plans/aps-rules.md instead. -->

## Active Modules

- PBLU: post-beta-launch-uplift (27/57)
- CRB: code-review-backlog (3/25)
- ANVFMT: anvil-file-format (in progress)
- BMAD4: bmad-v4-backward-compat (0/8)
- RENG: rust-core-engine (0/24)

## Conventions

- UK English spelling in all plan text
- Work item IDs: PREFIX-NNN (3-digit zero-padded)
- Module statuses: Draft -> Proposed -> Ready -> In Progress -> Complete
- Plans live in plans/modules/\*.aps.md
- Decisions live in plans/decisions/NNN-\*.md

## File Map

<!-- Extracted from non-Complete work items' Files: fields -->

[Generate from actual work items — scan each active module's .aps.md for Files:
lines in non-Complete work items and list them as path: ITEM-NNN, ITEM-NNN]
```

**Step 2: Verify the rules file**

Run: `cat .claude/rules/aps-project.md` Expected: populated with current module
data and file map

**Step 3: Commit**

```bash
git add .claude/rules/aps-project.md
git commit -m "feat(rules): add auto-generated APS project rules"
```

---

## Task 4: Update /plan and /plan-status commands

The existing commands work but should reference the skill so they benefit from
the always-on context.

**Files:**

- Modify: `.claude/commands/plan.md`
- Modify: `.claude/commands/plan-status.md`

**Step 1: Read current command files**

Read both files to understand current content.

**Step 2: Add skill reference to /plan**

Add a note at the top of the Instructions section:

```markdown
> **Note:** If the `aps-planning` skill is active, it has already loaded APS
> context for this session. Use that context rather than re-scanning from
> scratch.
```

**Step 3: Add skill reference to /plan-status**

Add to the Instructions section:

```markdown
> **Note:** If the `aps-planning` skill is active, trigger a foreground
> reconciliation scan using the anvil-plan-spec agent. This runs validation
> commands and detects drift, producing a more accurate status than a simple
> file scan.
```

**Step 4: Verify both files**

Run: `head -15 .claude/commands/plan.md .claude/commands/plan-status.md`
Expected: both have the new notes

**Step 5: Commit**

```bash
git add .claude/commands/plan.md .claude/commands/plan-status.md
git commit -m "feat(commands): reference aps-planning skill in plan commands"
```

---

## Task 5: Test the full flow manually

No automated tests — this is a skill/agent/rules integration. Validate by
running through the flow manually.

**Step 1: Verify skill loads**

Start a new Claude Code session in the eddacraft project. The skill should
detect `plans/index.aps.md` and load APS context. Look for the APS Context Block
in the skill's output.

**Step 2: Verify file map awareness**

Edit a file that appears in the file map (e.g.,
`.claude/agent-bus/forge-defer.sh`). Confirm the skill flags the relevance to
PBLU-020/PBLU-021.

**Step 3: Verify /plan-status triggers reconciliation**

Run `/plan-status`. Confirm it spawns the anvil-plan-spec agent and produces a
reconciliation report with validation scan results.

**Step 4: Verify project rules generation**

Check that `.claude/rules/aps-project.md` was created or updated by the
reconciliation.

**Step 5: Verify no-op on non-APS projects**

Open a project without `plans/index.aps.md`. Confirm the skill does nothing.

---

## Task 6: Final commit and cleanup

**Step 1: Review all changes**

Run: `git diff --stat main` Expected: changes to agent, commands, rules, and
design doc

**Step 2: Push branch**

```bash
git push
```

**Step 3: Open PR or update existing**

If a PR already exists for this branch, push updates it. Otherwise create one.
