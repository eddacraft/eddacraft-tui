---
name: planning-workflow
description: >-
  Anvil planning workflow for turning intent into approved APS-backed work.
  Coordinates project truth discovery, brainstorming, APS updates, readiness
  validation, and handoff to dev-workflow.
---

# Planning Workflow

## Source And Variant

This is the Anvil vendored variant of the neutral EddaCraft skill at
`eddacraft-skills/skills/eddacraft/planning-workflow`. Keep the neutral
intent-to-ready contract aligned, but preserve Anvil-specific APS authorities,
ADR/scope checks, release metadata, and documentation closeout here.

## OpenCode Surface

Use OpenCode `skill` routing for specialist skills and the `task` tool with the
vendored `.opencode/agents/anvil-plan-spec.md` agent when APS module/task edits
need a dedicated planner. Do not rely on Claude slash commands from this copy.

## Purpose

Use this skill before implementation when the user has a goal, feature, fix, or
workflow change that is not already a clearly valid `Ready` or `In Progress` APS
work item.

This skill is the planning orchestrator. It does not replace specialist skills:

- Use `brainstorming` for unclear scope, behaviour, architecture, ownership, or
  user experience.
- Use `aps-planning` for APS truth validation, status, drift, and reconciliation.
- Use `writing-plans` for implementation plans after design approval.
- Use the vendored `anvil-plan-spec` agent for APS module/task drafting,
  validation, status updates, and plan hygiene.
- Use `planning-council` for high-risk, cross-boundary, or multi-persona design
  decisions.

## Activation

Invoke this skill when:

- The user asks to plan work.
- The work does not map cleanly to one existing APS item.
- `dev-workflow` or `aps-planning` returns `needs-plan-update`.
- The goal touches architecture, scope, CI, release, security, docs authority,
  feature flags, branch policy, or multiple modules.
- A plan exists but may be stale relative to current project truth.
- The user asks for a new module, roadmap change, or readiness review.

If the task already has one validated `Ready` or `In Progress` APS item and no
drift is found, return to `dev-workflow` instead of replanning.

This workflow never writes code and never creates branches.

## Authorities

Read these before making planning decisions:

- `AGENTS.md`
- `plans/aps-rules.md`
- `plans/index.aps.md`
- Relevant `plans/modules/<module>.aps.md`
- `plans/decisions/DECISION-LOG.md` for architectural or durable decisions
- `docs/vision/anvil-scope-guard.md` for scope questions
- `docs/architecture/overview.md` for architecture fit
- `docs/guides/documentation-governance.md` for docs changes
- `docs/guides/feature-flag-governance.md` for feature flags
- `docs/guides/branching-strategy.md` and `docs/guides/worktree-policy.md` for
  lifecycle routing

Read specific ADRs, runbooks, schemas, tests, or source files when the goal
depends on them.

## Workflow

### 1. Goal Intake

Capture the user's requested outcome, success criteria, constraints, urgency,
and known non-goals. Ask one concise clarifying question only when the next
planning step would otherwise be guesswork.

### 2. Project Truth Discovery

Use `aps-planning` to load current APS context. Check current source, tests,
docs, ADRs, workflows, release state, and feature flags relevant to the goal.
Prefer current implementation truth over stale plan prose, but preserve plan
intent unless it clearly conflicts with shipping behaviour or accepted ADRs.

### 3. Existing Work Match

Decide whether the goal:

- maps to one existing APS item,
- updates an existing APS item,
- supersedes or splits existing work,
- needs a new item in an existing module,
- needs a new module,
- is out of Anvil scope.

Do not create shadow indexes or summary files. `plans/index.aps.md` remains the
canonical module index.

If the work is already done, route to `aps-planning` reconciliation rather than
creating a new plan.

### 4. Design Gate

Invoke `brainstorming` before plan writing when the goal changes behaviour,
architecture, UX, ownership, security posture, feature flags, release policy, or
system boundaries.

If the design is high-risk or cross-boundary, invoke `planning-council` before
finalising the plan.

### 5. Plan Synthesis

Use `writing-plans` for approved implementation plans and the `anvil-plan-spec`
agent to draft or update APS content. New APS text must follow
`plans/aps-rules.md`:

- Module statuses: `Proposed`, `Ready`, `In Progress`, `Done`, `Blocked`
- Legacy aliases are recognised but new text should use canonical statuses
- Tasks authorise execution and state outcomes, validation, dependencies, and
  expected files without implementation tutorials
- Actions are observable checkpoints, not step-by-step code instructions
- Cross-cutting modules own their own work and close callouts when completed
- Release metadata is included when relevant

If plan edits touch docs, ADRs, feature flags, release state, or workflow rules,
include the required closeout/cross-link updates in the plan rather than leaving
them implicit.

### 6. Readiness Validation

Run `aps-planning` APS Truth Validation on the selected or newly updated item.
The plan is ready for development only when the decision is `valid`, the item is
`Ready` or `In Progress`, validation evidence is defined, and dependencies are
closed or explicitly documented.

### 7. Handoff

Return this handoff block:

```markdown
## Planning Workflow Handoff

- Goal:
- APS module:
- APS work item:
- Status:
- Design source:
- Dependencies:
- Files:
- Validation:
- Risks:
- Decision: ready-for-dev | needs-design | needs-plan-update | blocked | out-of-scope
- Next skill: dev-workflow | brainstorming | aps-planning | writing-plans | planning-council
```

`ready-for-dev` hands back to `dev-workflow`. All other decisions stop before
implementation.

## Stop Conditions

Stop and ask the user, or hand off to the named skill, when:

- The goal is out of Anvil scope.
- The work needs a durable architectural decision.
- The plan would execute `Proposed` work without explicit authorisation.
- Current project truth contradicts the requested plan.
- The next step is implementation.

## Writing Rules

- Ask before writing plan files unless the user explicitly requested plan edits.
- Keep plan edits minimal and tied to the discovered truth.
- Use UK English in plan text and documentation.
- Do not add backward-compatibility or scope expansion unless the plan or user
  explicitly requires it.
- If documentation changes are made, complete documentation closeout before the
  final response.
