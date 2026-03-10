# APS vs GitHub Projects — Trial Decision Space

Date: 2026-03-09  
Owner: Josh / Anvil  
Status: Proposed (trial)

## Why this decision now

We currently split planning/execution context across two systems:

- **APS** (`plans/*.aps.md`) for deep planning (decomposition, dependencies,
  sequencing, architecture context).
- **GitHub Projects/Issues/PRs** for team-visible execution flow and delivery
  telemetry.

The goal is not to pick one and abandon the other. The goal is to define clean
ownership boundaries so both stay useful without drift chaos.

## Decision space

### Option A — APS-primary, GitHub mirrors delivery

APS is canonical. GH mirrors selected execution state.

### Option B — GitHub-primary, APS reduced to strategy docs

GitHub becomes canonical. APS is mostly long-form context.

### Option C — Dual-primary with domain boundaries (**recommended trial**)

Both are primary, but for different domains:

- **APS primary for planning semantics** (task model, readiness, dependency
  intent, acceptance criteria).
- **GitHub primary for execution telemetry** (who is doing what now, PR/review
  state, queuing in projects).

This is not strict dual-write parity. It is bounded dual-primary with explicit
ownership.

## Recommendation (trial)

Adopt **Option C (dual-primary with domain boundaries)** for a 2-week trial.

Rationale:

- Keeps APS planning depth.
- Gives GH Projects true day-to-day operational visibility.
- Avoids pretending one tool is good at everything.

## Trial policy (2 weeks)

### Scope

- Apply to one focused stream: **APS vs GH tracking trial**.
- Keep scope to 5–10 tasks.

### Canonical ownership rules

#### APS is canonical for

- Task definition and decomposition
- Dependency declarations
- Readiness semantics (canonical APS states: `Proposed`, `Ready`, `In Progress`,
  `Blocked`, `Done`; legacy aliases for backward compatibility: `Draft` →
  `Proposed`, `Complete` → `Done`)
- Acceptance criteria

#### GitHub is canonical for

- Active execution lane (`In Progress`, `In Review`, `Done` in Projects)
- Branch/PR linkage
- Review and merge lifecycle
- Assignment/throughput visibility

### Conflict-resolution rules

1. If **task intent/scope** is wrong → fix APS first.
2. If **execution state** is wrong → fix GH first.
3. If both drift, reconcile APS + GH in same working session before new work
   starts.

## Required linking contract

Every execution artifact must carry APS linkage:

- **Issue body** includes:
  - `APS Module:`
  - `APS Task:`
- **PR body** includes:
  - `APS Module:`
  - `APS Task(s):`
  - `GH Project Status:`
  - `Acceptance criteria checked:`
- **Commits** should include footer `Refs: <MODULE>-<TASK>` when practical.

## Status mapping (APS ↔ GH)

- `APS Proposed` (legacy `Draft`) → GH Issue not created or in `Backlog`
- `APS Ready` → GH Issue in `Ready`
- `APS In Progress` → GH Issue in `In Progress` with active branch/PR link
- `APS Blocked` → GH Issue labelled `blocked` and in `Blocked`
- `APS Done` (legacy `Complete`) → GH Issue `Done` and PR merged

## Branch/PR naming

- Branch: `chore/<module-task>-<slug>` where `<module-task>` is the APS task id
  normalised to lowercase and hyphenated (for example, `CRB-031` → `crb-031`).
- PR title: `[<MODULE>-<TASK>] <summary>`

## GitHub Projects tracking model

Use fields:

- **Status**: Backlog, Ready, In Progress, In Review, Blocked, Done
- **APS Module** (text/select)
- **APS Task** (text)
- **Priority** (High/Med/Low)
- **Risk** (Low/Med/High)
- **Target Milestone** (optional)

### Minimal automation

- PR opened → linked Issue `In Review`
- PR merged → linked Issue `Done`
- `blocked` label added → move Issue `Blocked`

## Operating cadence

### Daily (execution)

- Pull work from APS `Ready`.
- Ensure GH Issue exists before coding starts.
- Keep Project card + PR APS fields updated.

### Twice-weekly (reconciliation)

- 15-min APS↔GH drift check:
  - APS `In Progress` with no GH `In Progress`?
  - GH `In Progress` with no APS `In Progress`?
  - merged PRs missing APS status updates?

### End-of-trial review (week 2)

Evaluate:

- Lead time (issue open → merge)
- Drift incidents (APS/GH mismatch count)
- Admin overhead (low/med/high)
- Visibility quality in Projects

## Success criteria

- <=10% APS↔GH drift during trial
- No regression >10% in median PR cycle time
- team reports better visibility in GH Projects without APS quality drop

If unmet, either tighten contracts/automation or re-evaluate Option A/B.

## Immediate implementation checklist

- [ ] Create GH Project fields for APS Module / APS Task
- [x] Add PR template fields for APS + GH execution context
- [x] Add issue template fields for APS Module / APS Task
- [ ] Start trial with 5–10 scoped tasks
- [ ] Schedule twice-weekly reconciliation checkpoints
- [ ] Run end-of-trial retrospective
