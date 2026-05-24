# APS Rules for AI Agents

> This file guides AI agents working with APS specs in this repository.
> Keep it in `plans/` so agents discover it when exploring the planning
> directory.
>
> **APS-managed intent:** this file stays close to the canonical
> [`anvil-plan-spec`](https://github.com/eddacraft/anvil-plan-spec) scaffold so
> it is easy to refresh. Anvil-specific Worktrunk, Council, release, validation,
> and documentation-governance rules live in
> [`plans/project-context.md`](project-context.md).

## Core Principle

**Specs describe intent. Work items authorise execution. Actions are checkpoints,
not tutorials.**

## Hierarchy

| Layer | Purpose | You Write | You DON'T Write |
| ----- | ------- | --------- | ---------------- |
| Index | Plan overview | Modules, milestones, risks | Implementation details |
| Module | Bounded work area | Interfaces, work items, boundaries | Code snippets |
| Work Item | Execution authority | Outcome, validation command | How to implement |
| Action | Checkpoint | Observable state | Implementation steps |

## Lifecycle Statuses

Canonical APS treats schema status and project lifecycle prose as separate
vocabularies.

### Module Schema Status Values

The parser and validator accept these module status values:

| Status | Meaning | Work Items Executable? |
| ------ | ------- | ---------------------- |
| Proposed | Reviewed direction exists, but execution is not yet authorised | No |
| Ready | Scope clear, dependencies identified, validation known, execution authorised | Yes |
| In Progress | Actively being worked on | Yes |
| Done | Substantive work is finished | No new execution |
| Blocked | Cannot proceed; document the reason | No |

The local parser currently normalises legacy `Draft` to `Proposed` and legacy
`Complete` to `Done`. New APS text should write canonical status values.

### Work Item Status

Work item status describes execution authority and progress. Prefer canonical
planning values in APS text:

- `Proposed`
- `Ready`
- `In Progress`
- `Done`
- `Blocked`

Historical Anvil plans also contain lifecycle narrative labels such as `Merged`,
`Released/Shipped`, and `Archived`. Treat those as project context, not portable
APS schema vocabulary. See
[`plans/project-context.md#anvil-lifecycle-narrative`](project-context.md#anvil-lifecycle-narrative).

### Status Rules

1. Do not execute `Proposed` work unless the operator explicitly approves the
   item as urgent authorised work; record that authorisation inline.
2. Mark work `In Progress` before making substantive changes for that item.
3. Mark work `Done` only when the expected outcome is satisfied and validation
   evidence exists.
4. Archive completed modules by moving them to `plans/archive/modules/` and
   updating `plans/index.aps.md` in the same change.

## Actions: The Lean Rule

Actions translate work item intent into **observable checkpoints**. They are NOT
implementation guides.

### Format

```markdown
### 1. [Action verb] [target]

- **Checkpoint:** [Observable state — max 12 words]
- **Validate:** `[command]` (optional)
```

### What Goes Where

| Write in Action | Write NOWHERE (emerges from patterns) |
| --------------- | ------------------------------------- |
| "Auth middleware exists" | Which library to use |
| "Tests pass" | Test implementation details |
| "Migration applied" | SQL schema definition |
| "Function handles errors" | Try/catch structure |

### Anti-Patterns

```markdown
# BAD: Implementation tutorial disguised as action

### 1. Create authentication middleware

- **Checkpoint:** Middleware created in src/middleware/auth.ts that:
  - Extracts JWT from Authorization header
  - Validates token using jsonwebtoken library
  - Decodes payload and extracts user ID
  - Attaches user object to request context
  - Returns 401 if token invalid or expired
- **Validate:** `npm test -- auth.middleware.test.ts`
```

```markdown
# GOOD: Observable checkpoint only

### 1. Create authentication middleware

- **Checkpoint:** Auth middleware validates requests, attaches user to context
- **Validate:** `npm test -- auth.middleware.test.ts`
```

### Why Lean Actions?

1. **Implementation emerges** from existing patterns and agent judgement.
2. **Specs don't rot** because checkpoints stay valid when code changes.
3. **Agents stay autonomous** because they figure out how; humans verify what.
4. **Review stays fast** because readers scan checkpoints, not tutorials.

## Work Item Rules

Work items are **execution authority**: permission to make changes.

### Required Fields

- **Status:** Canonical planning status for the item.
- **Intent:** One sentence describing what outcome this achieves.
- **Expected Outcome:** Testable or observable result.
- **Validation:** Command or review procedure that verifies completion.

### Recommended Fields

- **Files:** Best-effort list of files or directories.
- **Dependencies:** Other work item IDs that must complete first.
- **Confidence:** `low`, `medium`, or `high`.

### Optional Fields

- **Scopes:** What can be changed.
- **Non-scope:** What will not change.
- **Inputs:** Required inputs or context.
- **Risks:** Potential risks associated with the item.
- **Packages:** Affected packages in monorepos.
- **Tags:** Labels for filtering and search.
- **Link:** External reference.

### Work Item Anti-Patterns

| Don't | Do |
| ----- | -- |
| "Implement JWT auth using jsonwebtoken" | "Add token-based authentication" |
| "Create UserService class with methods..." | "User operations are encapsulated" |
| "Add try/catch blocks to all handlers" | "API errors return consistent format" |

## Action Plans: Waves and Parallel Execution

Action plans can optionally group actions into **waves** for parallel execution.
Actions in the same wave are independent; each wave completes before the next
begins.

### Wave Table Format

```markdown
## Waves

| Wave | Actions | Gate |
| ---- | ------- | ---- |
| 1 | 1, 2 | Both checkpoints pass |
| 2 | 3 | Checkpoint passes |
```

### Action-Level Fields

Actions support optional execution metadata:

- **Wave** N — which wave this action belongs to.
- **Depends on** 1, 2 — action numbers that must complete first.
- **Agent** type — agent type for dispatch, such as `general` or `tdd-coach`.

### When to Use Waves

| Use Waves | Stay Sequential |
| --------- | --------------- |
| Three or more actions with independent work | Each action depends on the previous |
| Multi-agent dispatch needed | Single-agent linear execution |
| Work item has natural parallel boundaries | Actions share mutable state |

### When Not to Use Waves

- Simple work items with fewer than four actions.
- All actions modify the same files.
- Actions are inherently sequential.

## Naming Conventions

### Module Files

Canonical APS module files are short kebab-case slugs with an `.aps.md` suffix.
Numeric prefixes are also allowed when dependency order benefits from them.

```text
modules/
├── core.aps.md
├── auth.aps.md
├── 01-foundation.aps.md
└── 02-api.aps.md
```

- Use kebab-case and the `.aps.md` suffix.
- Keep filenames stable; dependency order lives in `plans/index.aps.md`.
- If numeric prefixes are used, use zero-padded numbers such as `01-`.

### Work Item IDs

Work items use the module's ID prefix and a zero-padded sequence:
`AUTH-001`, `AUTH-002`, `CORE-001`.

## Creating APS Documents

### When Asked to Plan

1. Read existing `plans/index.aps.md` if present.
2. Check completed work context if the project has a completed index.
3. Identify which template fits: index, module, action plan, or design.
4. Fill sections with **intent**, not implementation detail.
5. Mark assumptions explicitly.
6. Leave work items unready until execution authority is clear.

### When Asked to Execute

1. Find the relevant work item in a module file.
2. Confirm the work item is `Ready` or `In Progress`.
3. Create an action plan file in `plans/execution/` if the work is complex.
4. Execute one action at a time and validate checkpoints.
5. Mark work complete only after validation passes.

## File Locations

```text
plans/
├── aps-rules.md                  # APS-managed agent guidance
├── project-context.md            # Anvil-owned project context
├── index.aps.md                  # Root plan
├── issues.md                     # Development-time discoveries, optional
├── modules/                      # Active module specs
│   ├── core.aps.md
│   └── auth.aps.md
├── archive/modules/              # Historical completed modules
├── execution/                    # Action plans
│   ├── [WORK-ITEM-ID].actions.md # Per-work-item action plan
│   └── [MODULE].actions.md       # Per-module action plan
├── decisions/                    # ADRs, optional
│   └── [NNN]-[title].md
└── designs/                      # Technical designs, optional
    └── YYYY-MM-DD-slug.design.md
```

## Design Documents

Design docs live in `plans/designs/` when a project uses them. They capture
architectural thinking **before** committing to modules and work items.

### When to Create

- Multi-module work with non-obvious architecture.
- Multiple viable approaches that need comparison.
- Work that needs review before defining work items.
- Cross-cutting concerns that span several modules.

### When to Skip

- Straightforward single-module features.
- Bug fixes or small enhancements.
- Work where the approach is already established.

### Naming

`plans/designs/YYYY-MM-DD-slug.design.md` — date-prefixed descriptive slug.

### Linking

Reference designs from the index or module metadata:

```markdown
## Designs

- [Auth Architecture](designs/2025-01-05-auth-architecture.design.md)
```

### Accept-Then-Normalise

If a design doc already exists in free form, accept it. Do not reject it for
missing sections. Instead:

1. Add the minimum fields: `## Problem`, `## Design`, and status metadata.
2. Do not rewrite the author's content.
3. Append missing sections or infer them from existing content.

## Issues and Questions Tracker

Projects may use `plans/issues.md` to log development-time discoveries:

- **Issues (`ISS-NNN`)** — bugs, limitations, or edge cases noticed during
  development.
- **Questions (`Q-NNN`)** — unknowns that need answers or deferred decisions.

This tracker is for planning visibility, not routine bug reports or production
incidents.

## Project-Specific Context

Before applying these rules in Anvil, read
[`plans/project-context.md`](project-context.md). It defines local lifecycle,
branching, review, release, feature flag, and documentation-governance rules
that deliberately sit outside portable APS guidance.

## Release Metadata

Release metadata is an Anvil project extension, not portable APS schema. The
anchor remains here so existing docs can link to the concept without pulling the
full local workflow into this APS-managed file. See
[`plans/project-context.md#release-metadata-extensions`](project-context.md#release-metadata-extensions).

## Cross-Cutting Modules

Cross-cutting module closeout rules are an Anvil project extension. The anchor
remains here so active and archived plans keep stable links while the detailed
local convention lives in
[`plans/project-context.md#cross-cutting-modules`](project-context.md#cross-cutting-modules).

## Quick Reference

| If agent is... | Check for... |
| -------------- | ------------ |
| Writing a design | Problem and design sections present? No implementation prescriptions? |
| Writing actions | Max 12 words per checkpoint? No implementation detail? |
| Writing work items | Outcome-focused? Has validation command? |
| Planning module | Boundaries clear? No premature execution authority? |
| Executing | Work item status is Ready/In Progress? Prerequisites met? |
| Found issue/question | Logged in issues.md if project uses the tracker? |
| In Anvil | Project context read before branch/review/release work? |
