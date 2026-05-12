# APS Rules for AI Agents

> This file guides AI agents working with APS specs in this repository.
> Keep it in `plans/` so agents discover it when exploring the planning directory.
>
> **Specification:** [github.com/eddacraft/anvil-plan-spec](https://github.com/eddacraft/anvil-plan-spec)

## Core Principle

**Specs describe intent. Tasks authorise execution. Steps are checkpoints, not tutorials.**

## Hierarchy

| Layer | Purpose | You Write | You DON'T Write |
|-------|---------|-----------|-----------------|
| Index | Plan overview | Modules, milestones, risks | Implementation details |
| Module | Bounded work area | Interfaces, tasks, boundaries | Code snippets |
| Task | Execution authority | Outcome, validation command | How to implement |
| Action | Checkpoint | Observable state | Implementation steps |

## Lifecycle Statuses

APS uses three related but separate vocabularies, and the schema treats them
differently:

1. **Module schema status** — the value parsed from a module's header
   `Status` (or a task body's `Status:` line in a module spec). This is the
   *planning* state the validator/`ModuleStatusSchema` cares about.
2. **Task execution status** — a separate, narrower vocabulary used by the
   external state file (`.anvil/state.json` per `TaskStatusSchema`) for
   in-flight execution locking. Authors do not write this in `.aps.md`
   files; it is managed by the state APIs.
3. **Lifecycle narrative** — prose labels used in index commentary, release
   tables, and operator-facing summaries. These are not parsed; they describe
   where work sits in the broader plan/build/release pipeline.

### Module Schema Status Values

The parser and validator accept exactly these five values for the module
status field (`ModuleStatusSchema` in `packages/aps/src/types/index.ts`):

| Status | Meaning | Tasks Executable? |
|--------|---------|-------------------|
| Proposed | Reviewed direction exists, but execution is not yet authorised | No |
| Ready | Scope clear, dependencies identified, validation known, execution authorised | Yes |
| In Progress | Actively being worked on | Yes |
| Done | Substantive work is finished | No new execution |
| Blocked | Cannot proceed (document reason) | No |

The parser normalises two legacy values written in older specs:

- `Draft` → `Proposed`
- `Complete` → `Done`

New APS text should write the canonical form directly. Any other module
status value is ignored by the parser (status is left unset).

### Task Execution Status (state, not text)

Tasks may carry a `Status:` line in their body. The parser maps the prose
value to one of four execution-state tokens defined by `TaskStatusSchema`:
`open`, `locked`, `completed`, `cancelled`. These describe *execution
state*, not planning state, and are normally managed by `state.json` rather
than written by hand.

The parser is deliberately lenient about prose: it normalises common module
status words (`In Progress`, `Done`, `Draft`, `Ready`, `Blocked`,
`Complete`) onto the four execution tokens, and **defaults to `open`** for
any value it doesn't recognise rather than leaving the field unset. See
`parseStatus()` in `packages/aps/src/parser/parse-task.ts` for the exact
alias table; document the canonical execution tokens (`open` / `locked` /
`completed` / `cancelled`) in new text.

### Lifecycle Narrative Labels

The following prose labels are used in `plans/index.aps.md` current-window
tables, release commentary, and module narrative — they are **not** valid
values for the schema `Status` field:

```text
APS Draft -> APS Proposed -> APS Ready -> In Progress -> Merged -> Released/Shipped -> Complete/Archived
```

- `Merged` — code or docs reached the integration target but have not
  necessarily shipped.
- `Released` / `Shipped` — a release record proves inclusion in a verified
  release.
- `Complete` (in prose) — no remaining active closeout work; module may be
  archived. When used as a schema `Status` value it is normalised to `Done`.
- `Archived` — historical record only; the file has been moved to
  `plans/archive/modules/`.
- `Committed` is legacy wording for `Merged` unless a specific module defines a
  narrower transition. New text should prefer `Merged`.

### Status Rules

1. Do not execute `Proposed` work (or legacy `Draft`, normalised to
   `Proposed`) unless the operator explicitly approves the item as urgent
   authorised work; record that authorisation inline.
2. Mark work `In Progress` before making substantive changes for that item.
3. In schema fields, advance the module/task to `Done` when substantive work is
   finished. In index narrative, additionally distinguish `Merged` vs
   `Released/Shipped` based on release-record evidence — do not infer shipped
   state from memory, a PR merge, or release notes prose.
4. Mark work `Complete` (narrative) only when validation, closeout, and
   cross-reference sweeps are done.
5. Archive completed modules with `git mv` into `plans/archive/modules/` and
   update `plans/index.aps.md` in the same change.

## Release Metadata

Target-state work items should carry enough metadata to reconstruct release
intent without reading PR prose. Add these fields where relevant:

```yaml
changeType: fix | feature | docs | internal | breaking
releaseIntent: candidate | hold | never
holdCondition: required when releaseIntent is hold
releaseScope: patch | minor | major | none
releaseNote:
  audience: user | operator | developer | none
  type: added | fixed | changed | removed | security
  text: optional one-sentence release note
validation:
  - command to prove the item
```

These fields are a **prose convention**: they are read by humans and the
release tooling that scans plan text, but they are NOT extracted into the
parser's typed `Task` schema. Write them as plain `**Field:** value` lines
in the task body alongside `Validation:` etc.

Rules:

1. `changeType` describes the change shape, not the git commit type.
2. `releaseIntent: candidate` means the item is eligible for release candidate
   contents once merged.
3. `releaseIntent: hold` means merged work should not ship until `holdCondition`
   is satisfied.
4. `releaseIntent: never` is for docs/internal work that should not drive a
   product release by itself.
5. `releaseScope` is `none` for non-releasable work.
6. `releaseNote.audience: none` means no user/operator/developer-facing note is
   expected.
7. `validation` records commands that prove the item; CI remains the validation
   authority for release readiness.

## Cross-Cutting Modules

A cross-cutting module coordinates work that touches multiple domains without
owning a single product surface. Such modules MUST follow these rules:

1. **Owns its own work items** — every cross-cutting task is owned and counted
   by the cross-cutting module, never by the surfaces it touches.
2. **Cross-references via prose callouts** — use `Coordinates with:`,
   `Blocks on:`, `Supersedes:`, and `Superseded by:` in task bodies. Use
   `Supersedes:` when the current task replaces an older item; use
   `Superseded by:` when the current task is replaced by a newer item. No
   typed relations, no separate dependency graph. (`Blocks on:` is currently
   provisional — to be hardened once exercised in a completed task.)
3. **Closer sweeps callouts on task completion** — whoever closes a task with
   cross-ref callouts MUST read each one in the body and either resolve it
   (reference is now correct), downgrade it (e.g. `Blocks on:` →
   `Coordinates with:`), or document the rationale and **close the callout
   in the same edit**. Documenting MUST NOT defer the callout into the
   archive. If the reference cannot be resolved or downgraded at close
   time, document the rationale inline and mark the callout as closed in
   the same edit ("document-and-close"). This is distinct from deferring —
   the callout is resolved by being explicitly closed, not carried forward.
4. **Closer sweeps all open callouts at archive time** — when a cross-cutting
   module is archived (via `git mv` to `plans/archive/modules/`), the closer
   sweeps every remaining open callout in the module body and
   resolves/downgrades/documents-and-closes each. None may carry into archive
   unresolved.

**Anti-drift hook:** Changes to this section update
`plans/modules/launch-flow-readiness.aps.md`,
`plans/modules/tracing-foundation.aps.md`, and
`plans/modules/usage-analytics.aps.md` headers in the same PR. New
cross-cutting modules cite this section by anchor link.

> Provenance: this section was promoted from the LAUNCH module's local
> convention block (the first trial) under
> [ADR-034](decisions/034-cross-cutting-modules-as-aps-primitive.md). The
> second trial is [`tracing-foundation`](modules/tracing-foundation.aps.md);
> the third trial is [`usage-analytics`](modules/usage-analytics.aps.md)
> (founder-requested 2026-05-10, durable usage observations on Kindling).
> When LAUNCH archives, sweep its remaining callouts per rule 4 and revisit
> the still-provisional `Blocks on:` clause based on whatever close cycles
> have happened in the meantime.

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
- **Validation:** Command to verify completion. The parser also accepts the
  legacy alias `**Test:**`; new tasks should write `Validation:`.
- **Confidence:** low/medium/high
- **changeType:** `fix`, `feature`, `docs`, `internal`, or `breaking`
- **releaseIntent:** `candidate`, `hold`, or `never`
- **holdCondition:** Required when `releaseIntent` is `hold`
- **releaseScope:** `patch`, `minor`, `major`, or `none`
- **releaseNote:** Audience, type, and one-sentence text for release notes

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

Name module files after the bounded work area they describe — a short
kebab-case slug that matches the module ID in `index.aps.md`:

```text
modules/
├── anvil-file-format.aps.md
├── anvil-rust-scanner.aps.md
└── compliance-policy-packs.aps.md
```

- Use kebab-case, `.aps.md` suffix
- The filename slug should match the module ID row in `index.aps.md`
- Dependency order lives in `index.aps.md` (the Modules table), not in the
  filename — this keeps filenames stable when ordering changes and avoids
  rename churn across docs, decisions, and cross-references

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
├── modules/                  # Active module specs
│   ├── anvil-file-format.aps.md
│   └── anvil-rust-scanner.aps.md
├── archive/modules/          # Completed modules (git mv from modules/)
├── execution/                # Action plan files
│   ├── [TASK-ID].steps.md    # Per-task (complex projects)
│   └── [MODULE].steps.md     # Per-module (simple projects)
└── decisions/                # ADRs (see DECISION-LOG.md for index)
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

## Project Conventions

- UK English spelling in all plan text
- Work item IDs are zero-padded to 3 digits: `PREFIX-001`, not `PREFIX-1`
- Plans live in `plans/modules/*.aps.md`
- Decisions live in `plans/decisions/NNN-*.md` (or `NNN[a-z]-*.md` for variants)

## Quick Reference

| If agent is... | Check for... |
|----------------|--------------|
| Writing actions | Max 12 words per checkpoint? No implementation detail? |
| Writing tasks | Outcome-focused? Has validation command? |
| Planning module | Boundaries clear? Status set? No premature tasks? |
| Executing | Module status is Ready/In Progress? Prerequisites met? |
| Starting work | Read index.aps.md (active) + completed-index.aps.md (context)? |
| Finishing / committing | Schema status set to Done? Narrative status (Merged) reflects integration? Post-merge test plan extracted to plans/reviews/post-merge/? |
| Cleanup agent | Done items merged + CI green → advance narrative to Complete? Post-merge plans verified? |
