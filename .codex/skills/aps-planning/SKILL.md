---
name: aps-planning
description: >-
  APS awareness, truth validation, and reconciliation. Loads plan context,
  validates selected work against current project truth before implementation,
  and reconciles plan state at task boundaries.
---

# APS Planning Skill

## Activation Guard

If `plans/index.aps.md` does not exist in the current project, do nothing.
This skill is inert in projects without APS plans. Do not mention APS, do not
offer to create plans, do not load any context. Silently skip all sections
below.

## Session Start -- Load Context

When a session begins and `plans/index.aps.md` exists, build APS awareness in
working memory. Do this quietly -- do not print output to the user unless they
ask about plan status.

### Step 1: Read the index

Read `plans/index.aps.md`. Extract the Modules table. Identify active modules
-- those whose status is NOT `Done`, `Complete`, `Merged`, `Released`, `Shipped`,
`Released/Shipped`, or `Archived`. Note the plan title and any current-window
notes.

### Step 2: Read active modules

For each active module, read its `.aps.md` file from `plans/modules/`. Extract
all work items with their fields:

- **ID** (e.g., `AUTH-001`)
- **Title**
- **Status** (canonical module statuses are `Proposed`, `Ready`, `In Progress`,
  `Merged`, `Released/Shipped`, `Complete`, `Blocked`; recognise legacy `Draft`
  -> `Proposed`, `Done` -> `Complete`, and `Released` / `Shipped` ->
  `Released/Shipped`)
- **Files** (from the `Files:` field, if present)
- **Validation** (from the `Validation:` field)
- **Priority** (if specified)
- **Dependencies** (if specified)

Skip work items with inactive/completed statuses: `Done`, `Complete`, `Merged`,
`Released`, `Shipped`, `Released/Shipped`, or `Archived`.

### Step 3: Build the file-to-item map

From all non-Complete work items that have a `Files:` field, build a mapping of
file paths to work item IDs. This map is used during passive awareness to
recognise when edited files relate to planned work.

Example:

```
src/auth/login.ts -> AUTH-001, AUTH-003
src/db/migrations/ -> DB-002
packages/core/src/policy.ts -> POL-001
```

### Step 4: Check for cached context

Look for `.opencode/rules/aps-project.md` in the project. If it exists, read it
for additional project-specific APS context (module relationships, conventions,
recent decisions). If missing, note its absence but do not block on it.

### Step 5: Store APS Context Block

Hold this compact summary in working memory (not displayed to user):

```
## APS Context
Active: [MODULE_ID (X/Y items done), ...]
In-progress: [ITEM-NNN: title, ...] or (none)
File map: [N tracked paths]
Next suggested: [ITEM-NNN (reason)]
```

Where:

- **Active** lists each active module with completion ratio
- **In-progress** lists work items currently being worked on
- **File map** is the count of tracked file paths
- **Next suggested** is the highest-priority Ready item with no unmet
  dependencies, or the most impactful Proposed item if nothing is Ready

## APS Truth Validation

Run this mode when `dev-workflow` asks for an APS gate, when
`planning-workflow` needs a readiness decision, when the user asks if a plan is
current, or when scope appears stale, ambiguous, or cross-cutting.

Steps:

1. Confirm the user goal maps to exactly one primary APS work item. If not,
   return `needs-plan-update` and hand off to `planning-workflow`.
2. If ownership, scope, behaviour, or architecture is unclear, hand off to
   `planning-workflow`.
3. Confirm the module and work item status allow implementation.
4. Check dependencies and cross-reference callouts.
5. Read referenced files from `Files:` plus directly related tests, schemas,
   docs, ADRs, workflows, and feature flag definitions.
6. Compare expected outcome and validation commands against current project
   truth.
7. Identify drift: already-completed work, stale assumptions, moved files,
   changed APIs, invalid commands, missing dependencies, release-state mismatch,
   documentation authority conflicts, or scope conflicts.

Return this report before branch or code:

```markdown
## APS Truth Validation

- Module:
- Work item:
- Status:
- Project truth checked:
- Drift found:
- Decision: valid | needs-plan-update | blocked
- Required APS updates:
- Implementation notes:
```

Implementation MUST NOT begin from a stale, ambiguous, unauthorised, or blocked
APS item.

## During Work -- Passive Awareness

While the user works on code, maintain quiet awareness of APS relevance
without interrupting their flow.

### Recognise Relevance

When the user edits, reads, or discusses a file that appears in the
file-to-item map:

- Note the matching work item(s) internally
- Do NOT announce the match unprompted
- If the user asks "what item is this for?" or similar, cite the work item ID,
  title, and status
- If the user's changes clearly advance a work item's Expected Outcome, note
  this in working memory for later reconciliation

### Flag Completion Opportunities

At natural pauses -- after a commit, after completing a logical chunk of work,
or when the user asks "what's next?" -- you may surface a brief suggestion if
a work item appears to be complete:

> It looks like AUTH-001 (Add login endpoint) may be done -- the validation
> command is `pnpm vitest run src/auth/__tests__/login.test.ts`. Want me to
> run it and mark it complete?

Rules:

- **Never auto-update** work item status. Always ask first.
- Keep suggestions to one sentence plus the validation command.
- Do not repeat a suggestion the user has already declined or deferred.
- Maximum one suggestion per natural pause.

### Track Unplanned Work

If the user creates or modifies files within an active module's scope but those
files are not tracked by any work item:

- Note the untracked work internally
- At a natural pause, offer to add it as a new work item:

> You've added `src/auth/mfa.ts` which is in the AUTH module scope but isn't
> covered by any work item. Want me to draft a work item for MFA support?

Rules:

- Only offer for files clearly within an active module's scope
- Do not offer for test files, config files, or minor refactors
- One offer per untracked cluster of changes, not per file

## Commit and PR Integration

When APS context is active, enrich commits and PRs with work item references.
This is passive -- only add references when the file-to-item map produces
matches.

### Commit Messages

When creating a commit, look up the staged files against the file-to-item map.
If any staged files match one or more work items:

1. Resolve the list of matching work item IDs (deduplicated, sorted)
2. Add an `APS:` trailer to the commit message footer

Format:

```text
feat(auth): add login endpoint

Implement JWT-based login with bcrypt password hashing.

Fixes #42

APS: AUTH-001
```

Multiple items:

```text
APS: AUTH-001, AUTH-003, DB-002
```

Rules:

- Only add the trailer when the file-to-item map produces matches
- If no staged files match any work item, omit the trailer entirely
- Do not replace or interfere with the scope -- scope remains the code area
- The `APS:` trailer goes on its own line in the footer block
- If more than 5 items match, list the primary and note "and N others"

### Pull Requests

When creating a PR and APS context is active:

1. Collect all work item IDs from commits in the PR (scan for `APS:` trailers)
   or look up changed files against the file-to-item map
2. Add an **APS Work Items** section to the PR body, after Summary and before
   Test Plan:

```markdown
## APS Work Items

- **AUTH-001**: Add login endpoint (In Progress)
- **AUTH-003**: Add password hashing (Ready)
```

Rules:

- Include item ID, title, and current status
- If only one item, still use the section for consistency
- Do not add APS references to the PR title -- keep titles concise
- If no work items match, omit the section entirely

## Session Boundaries -- Reconciliation

Reconciliation syncs the APS plan files with actual project state.

### Triggers

Run reconciliation when any of these occur:

- A commit touches files in the file-to-item map
- A PR is created from a branch with tracked changes
- A branch is completed (merged or closed)
- The user explicitly requests it ("reconcile plan", "update plan status", "plan status")

### Reconciliation Steps

Perform reconciliation inline (foreground). This means:

1. For each recently changed file, check if its work item's Expected Outcome
   and Validation are now satisfied.
2. For work items that appear complete, verify by running the Validation
   command if one exists.
3. Draft status updates -- do NOT write them until the user approves, unless a
   currently active loop skill (`dev-loop-core`, `land-branch`, or `aps-loop`)
   has already granted explicit authority to reconcile the current work item.
4. Identify any new files that should be added to existing work items' Files fields.
5. Identify any unplanned work that warrants new Draft work items.
6. Check for dependency changes -- are any Blocked items now unblocked?

Output a reconciliation report in this format:

```
## APS Reconciliation Report

### Status Changes (proposed)
- ITEM-NNN: [current status] -> [proposed status] (reason)

### Files to Add
- ITEM-NNN: add [file path] to Files field

### New Work Items (proposed)
- [MODULE]-NNN: [title] — [rationale]

### Unblocked
- ITEM-NNN: dependency [DEP-ID] now complete

### Validation Results
- ITEM-NNN: [pass/fail] — [command output summary]

### No Changes Needed
- [list items reviewed but unchanged]
```

After presenting the report, ask if the user wants to apply the proposed changes.

> APS reconciliation found: 2 items potentially complete (AUTH-001, AUTH-003),
> 1 new file to track. Want me to apply the updates?

Exception: when called by `dev-loop-core` / `land-branch` for the current
ReadyItem after verified PR/merge evidence, apply only that item's status,
`Files:`, and evidence updates under the loop's authority. Broader plan, module,
ADR, dependency, or new-work changes still require a user checkpoint.

### Background Reconciliation (Optional)

For large plans where reconciliation is slow, you may run it as a background
shell process. Start it with `nohup` or in a separate terminal session and
redirect output to a temp file, then read the results when the user asks:

```bash
nohup bash -c 'cd /path/to/project && <reconciliation-steps> > /tmp/aps-reconcile.log 2>&1' &
```

When done, read `/tmp/aps-reconcile.log` and present the report.

## Plan Status Query

When the user asks for plan status ("what's the plan?", "plan status", "what's next?", "show plan"):

1. Run reconciliation inline (see above)
2. Produce the full APS status report
3. Ask if they want to apply proposed changes

## Plan Creation / Modification

When the user asks to create or modify a plan:

1. If `plans/index.aps.md` does not exist, help bootstrap APS structure:
   - Create `plans/` directory
   - Create `plans/index.aps.md` with plan metadata and empty Modules table
   - Create `plans/modules/` directory
2. If plans exist, help the user add modules, work items, or update existing entries
3. Always ask before writing — show proposed content first

## What This Skill Does NOT Do

- **Interrupt mid-flow** -- never break the user's concentration with APS info
- **Auto-edit plan files** -- always ask before writing status changes, except
  current-item reconciliation explicitly delegated by an active loop skill
- **Run validation during active work** -- validation runs only at session
  boundaries or on explicit request
- **Slow down the commit/PR workflow** -- reconciliation never gates commits or pushes
- **Activate on projects without `plans/index.aps.md`** -- the activation
  guard ensures complete silence in non-APS projects
