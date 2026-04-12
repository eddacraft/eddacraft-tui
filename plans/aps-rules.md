# APS Rules for AI Agents

> This file guides AI agents working with APS specs in this repository.
> Keep it in `plans/` so agents discover it when exploring the planning directory.
>
> **Specification:** [github.com/EddaCraft/anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec)

## Core Principle

**Specs describe intent. Tasks authorise execution. Steps are checkpoints, not tutorials.**

## Hierarchy

| Layer | Purpose | You Write | You DON'T Write |
|-------|---------|-----------|-----------------|
| Index | Plan overview | Modules, milestones, risks | Implementation details |
| Module | Bounded work area | Interfaces, tasks, boundaries | Code snippets |
| Task | Execution authority | Outcome, validation command | How to implement |
| Action | Checkpoint | Observable state | Implementation steps |

## Module Statuses
Modules progress through these statuses:

| Status | Meaning | Tasks Executable? |
|--------|---------|-------------------|
| Proposed / Draft | Work in progress, not ready | No |
| Ready | Scope clear, dependencies identified, tasks defined | Yes |
| In Progress | Actively being worked on | Yes |
| Done / Complete | All tasks done | N/A |
| Blocked | Cannot proceed (document reason) | No |

> Note: "Proposed" and "Done" are the current spec values; "Draft" and "Complete" are supported for backwards compatibility.

## Actions: The Lean Rule

Actions translate task intent into **observable checkpoints**. They are NOT implementation guides.

### Format

```markdown
### 1. [Action verb] [target]

- **Purpose:** [Why this action is needed]
- **Produces:** [What this action creates or changes]
- **Checkpoint:** [Observable state — max 12 words]
- **Validate:** `[command]` (optional)
```

### What Goes WHERE

| Write in Action | Write NOWHERE (emerges from patterns) |
|-----------------|---------------------------------------|
| "Auth middleware exists" | Which library to use |
| "Tests pass" | Test implementation details |
| "Migration applied" | SQL schema definition |
| "Function handles errors" | Try/catch structure |

### Anti-Patterns (NEVER do this)

```markdown
# ❌ BAD: Implementation tutorial disguised as action
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
# ✅ GOOD: Observable checkpoint only
### 1. Create authentication middleware

- **Checkpoint:** Auth middleware validates requests, attaches user to context
- **Validate:** `npm test -- auth.middleware.test.ts`
```

### Why Lean Actions?

1. **Implementation emerges** from existing patterns + agent judgment
2. **Specs don't rot** — checkpoints stay valid even when code changes
3. **Agents stay autonomous** — they figure out HOW, you verify WHAT
4. **Review stays fast** — humans scan checkpoints, not implementation plans

## Task Rules

Tasks are **execution authority** — permission to make changes.

### Required Fields

- **Intent:** One sentence — what outcome this achieves

### Recommended Fields

- **Expected Outcome:** Testable/observable result
- **Validation:** Command to verify completion (also accepts **Test:**)
- **Confidence:** low/medium/high

### Optional Fields

- **Scopes:** What can be changed (LLM file access constraints)
- **Non-scope:** What will NOT change
- **Files:** Best-effort list of files (not exhaustive)
- **Tags:** Labels for filtering and search
- **Dependencies:** Other task IDs that must complete first
- **Inputs:** Required inputs or context (as a list)
- **Risks:** Potential risks associated with this task
- **Packages:** Affected packages (monorepo support)
- **Link:** External reference (e.g., Jira ticket)

### Task Anti-Patterns

| ❌ Don't | ✅ Do |
|----------|-------|
| "Implement JWT auth using jsonwebtoken" | "Add token-based authentication" |
| "Create UserService class with methods..." | "User operations are encapsulated" |
| "Add try/catch blocks to all handlers" | "API errors return consistent format" |

## Naming Conventions

### Module Files

Name module files with a numeric prefix based on dependency order:

```text
modules/
├── 01-core.aps.md      # Foundation, no dependencies
├── 02-auth.aps.md      # Depends on core
├── 03-payments.aps.md  # Depends on auth
└── 04-ui.aps.md        # Depends on all above
```

- Use zero-padded numbers (`01-`, `02-`, not `1-`, `2-`)
- Order matches dependency flow (foundational → dependent)
- Order should reflect the Modules table in `index.aps.md`

### Task IDs

Tasks use the module's ID prefix: `AUTH-001`, `AUTH-002`, `CORE-001`, etc.

## Creating APS Documents

### When Asked to Plan

1. Read existing `plans/index.aps.md` if present (active/planned work)
2. Check `plans/completed-index.aps.md` for completed work context
3. Identify which template fits (index, module, simple)
4. Fill sections with **intent**, not implementation
5. Mark assumptions explicitly
6. Leave tasks empty until module is Ready

### When Asked to Execute

1. Find the task in the relevant `.aps.md` file
2. Check module has **Ready** or **In Progress** status
3. Create action plan file in `plans/execution/` if complex
4. Execute one action at a time, validate checkpoint
5. Mark task complete when validation passes

## File Locations

```text
plans/
├── aps-rules.md              # This file (agent guidance)
├── index.aps.md              # Root plan (active/planned work)
├── completed-index.aps.md    # Completed work archive
├── modules/                  # Module specs (numbered by dependency order)
│   ├── 01-core.aps.md
│   └── 02-auth.aps.md
├── execution/                # Action plan files
│   ├── [TASK-ID].steps.md    # Per-task (complex projects)
│   └── [MODULE].steps.md     # Per-module (simple projects)
└── decisions/                # ADRs (optional)
    └── [NNN]-[title].md
```

## Feature Flag Rules

When a task or module introduces a feature flag into the manifest:

1. **`createdFor` is mandatory** — every flag must reference the APS work item
   that introduced it (e.g. `FLAGS-008`).
2. **Sunset metadata** — `rollout` class flags must have an
   `expiryOrReviewDate`. Other classes should have one.
3. **Retirement task** — when a rollout reaches 100% and stabilises, the owning
   module must include a task to retire the flag (set status to `retiring` →
   `retired` → delete).
4. **Review checkpoint** — flag creation and class changes require review.
   Council review should verify retirement steps are followed before manifest
   entries are deleted.
5. **Governance guide** — see `docs/guides/feature-flag-governance.md` for the
   full lifecycle, rollout policy, and kill switch procedures.

## Quick Reference

| If agent is... | Check for... |
|----------------|--------------|
| Writing actions | Max 12 words per checkpoint? No implementation detail? |
| Writing tasks | Outcome-focused? Has validation command? |
| Planning module | Boundaries clear? Status set? No premature tasks? |
| Executing | Module status is Ready/In Progress? Prerequisites met? |
| Starting work | Read index.aps.md (active) + completed-index.aps.md (context)? |
| Finishing / committing | Module set to Committed? Post-merge test plan extracted to plans/reviews/post-merge/? |
| Cleanup agent | Committed items merged + CI green → advance to Complete? Post-merge plans verified? |
