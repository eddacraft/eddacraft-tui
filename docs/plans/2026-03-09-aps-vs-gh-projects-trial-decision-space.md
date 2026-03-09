# APS vs GitHub Projects — Trial Decision Space

Date: 2026-03-09
Owner: Josh / Anvil
Status: Proposed (trial)

## Why this decision now

We currently split planning/execution context across two systems:

- **APS** (`plans/*.aps.md`) as the internal source of structured work items, dependencies, and module-level sequencing.
- **GitHub Projects/Issues/PRs** as the external delivery and collaboration surface.

This trial defines a practical operating model to reduce duplication while preserving APS planning quality.

## Decision space

### Option A — APS-primary, GitHub mirrors delivery

APS is the source of truth for planning state. GitHub tracks externally visible execution.

**How it works**
- Work decomposition, status semantics, and dependencies live in APS.
- GitHub Issues/PRs are created for execution units that need async collaboration/review.
- Every GH Issue/PR references APS IDs (module + task).

**Pros**
- Keeps rich architecture-aware planning where it already exists.
- Preserves APS dependency graph and module context.
- Minimizes disruption to current workflow.

**Cons**
- Requires discipline to keep APS↔GH links updated.
- Some duplicate status updates remain.

### Option B — GitHub Projects-primary, APS reduced to architecture docs

GitHub Projects becomes operational source of truth; APS becomes strategy/background only.

**Pros**
- Single execution board visible to all collaborators.
- Lower coordination overhead for non-APS users.

**Cons**
- Loses APS-native dependency and module rigor.
- Requires migration and re-training.
- Higher risk of planning quality regression.

### Option C — Dual-write parity (strict sync)

APS and GitHub Projects both fully maintained as peers.

**Pros**
- Maximum visibility in both systems.

**Cons**
- High overhead, high drift risk.
- More process than value for current team size.

## Recommendation (trial)

Adopt **Option A (APS-primary)** for a 2-week trial.

Rationale:
- Lowest risk path.
- Aligns with existing repo planning structure.
- Gives immediate GH visibility without sacrificing APS quality.

## Trial policy (2 weeks)

### Scope
- Apply to one focused stream: **APS vs GH tracking trial** across selected Ready items.
- Keep scope small: 5–10 tasks max during trial window.

### Canonical state rules
- **Planning truth:** APS task status (`Draft/Ready/In Progress/Done/...`).
- **Execution truth:** GH Issue/PR activity and review state.
- If conflict occurs, reconcile by updating APS first, then GH.

### Required linking contract

Every execution artifact must include APS references:

- **Issue title/body** includes `APS: <MODULE>-<TASK>`.
- **PR body** includes:
  - `APS Module:`
  - `APS Task(s):`
  - `Acceptance criteria checked:`
- **Commit message footer** includes `Refs: <MODULE>-<TASK>` where practical.

### Status mapping (APS ↔ GH)

- `APS Ready` → GH Issue in `Backlog/Ready`
- `APS In Progress` → GH Issue in `In Progress` (+ active branch)
- `APS Blocked` → GH Issue labeled `blocked`
- `APS Done` → GH Issue closed (or in `Done`) and PR merged

### Branch/PR naming convention

- Branch: `chore/<module-task>-<slug>`
- PR title prefix: `[<MODULE>-<TASK>] <summary>`

Example:
- Branch: `chore/crb-031-aps-gh-trial-mapping`
- PR: `[CRB-031] Define APS↔GH project tracking policy`

## Tracking model in GitHub Projects

Use a lightweight project field set:

- **Status** (Backlog, Ready, In Progress, In Review, Blocked, Done)
- **APS Module** (text/select)
- **APS Task** (text)
- **Priority** (High/Med/Low)
- **Risk** (Low/Med/High)
- **Target Milestone** (optional)

### Minimal automation

- PR opened → move linked Issue to `In Review`
- PR merged → close linked Issue and move to `Done`
- `blocked` label added → move Issue to `Blocked`

## Operating cadence

### Daily (execution)
- Work from APS `Ready` items.
- Ensure GH Issue exists before coding starts.
- Keep PR body APS references current.

### Twice-weekly (reconciliation)
- 15-min APS↔GH drift check:
  - any APS `In Progress` without GH Issue?
  - any GH `In Progress` without APS `In Progress`?
  - any merged PR without APS status update?

### End-of-trial review (week 2)
Evaluate:
- Lead time (issue open → merge)
- Drift incidents (APS/GH mismatch count)
- Admin overhead (subjective: low/med/high)
- Team clarity (did project board improve visibility?)

## Success criteria for adopting permanently

- ≤10% APS↔GH drift during trial.
- No increase in median PR cycle time.
- Team reports improved visibility in GH Projects.

If unmet, revisit Option B for broader GH-primary shift.

## Immediate implementation checklist

- [ ] Create GH Project fields for APS Module / APS Task
- [ ] Add PR template section for APS references
- [ ] Add issue template field for APS Task ID
- [ ] Start trial with 5–10 scoped tasks
- [ ] Schedule twice-weekly reconciliation checkpoints
- [ ] Run end-of-trial retrospective and decide keep/change
