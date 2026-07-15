---
name: dev-loop-core
description: >-
  Development loop orchestrator wired to the consistent stage pack (plan-ready,
  grill-design, isolate-workspace, build-tdd, debug, evidence-gate, verify-loop,
  land-branch, address-reviews). Use for complete/goal/resume of APS items or
  natural-language goals. Prefer this over legacy dev-workflow when the pack is
  installed. Claude binding notes included for subagent isolation.
---

# Development loop (core pack)

Clone of `dev-loop` with stage skills pinned to the **dev-loop-core** pack.
Scope comes from the target; mode controls checkpoints and authority.

Shared contracts: [references/contracts.md](references/contracts.md).
Policy / claims / evidence schemas: [references/](references/).

Run checkpoint path: `.dev-loop/checkpoints/<runId>.json`. The orchestrator owns
this file: create it after Resolve, update it after every phase transition,
evidence gate, verifier decision, repair, PR state change, and final outcome.
If `.dev-loop/` is missing, create it on the feature branch, not on integration.

## Invocation

```text
/dev-loop-core complete DASH-001
/dev-loop-core complete DASH
/dev-loop-core goal "Add tenant export"
/dev-loop-core resume DASH-001
```

Treat `/dev-loop-core DASH-001` as shorthand for `complete`.

**Policy:** load repository `devLoop` policy first (see
`references/policy-contract.md` — search `dev-loop.policy.yaml` and variants).
If no file exists, default to **`interactive`** (terminal `review-ready`; never
self-merge). Only ask the user for mode when they requested unattended work and
policy is still missing. **Never infer merge authority.**

| Intent              | Scope                                                               |
| ------------------- | ------------------------------------------------------------------- |
| `complete <ITEM>`   | One APS work item                                                   |
| `complete <MODULE>` | Module; may delegate child items                                    |
| `goal <text>`       | `plan-ready` → ReadyItem, then continue                             |
| `resume <TARGET>`   | Reconstruct from APS, Git, PR, run checkpoint; verify before acting |

## Stage wiring (authoritative)

| Phase                 | Skill                                             |
| --------------------- | ------------------------------------------------- |
| Design Q&A            | `grill-design`                                    |
| Plan / ReadyItem      | `plan-ready`                                      |
| APS truth / reconcile | `aps-planning` (when `plans/index.aps.md` exists) |
| Isolate               | `isolate-workspace`                               |
| Build                 | `build-tdd`                                       |
| Debug                 | `debug`                                           |
| Executor evidence     | `evidence-gate`                                   |
| Independent verify    | `verify-loop`                                     |
| Land                  | `land-branch`                                     |
| PR feedback           | `address-reviews`                                 |
| Multi-persona review  | `council` / `local-review-council` when available |

Do **not** route through `planning-workflow`, `brainstorming`, `writing-plans`,
`test-driven-development`, `verification-before-completion`,
`using-git-worktrees`, or `finishing-a-branch` while this skill is active.

## Non-negotiable invariants

1. Never implement on the protected or default branch.
2. Prefer Worktrunk-managed worktree via `isolate-workspace`; require isolation for modules, autonomous runs, and parallel writers.
3. Acquire the target claim before writes (see `references/coordination-module.md` — v1 is **degraded git-ref**, advisory). Module claims may issue child leases.
4. APS is planning truth, Git is implementation truth, PR/checks are review truth, run checkpoint is resumable orchestration state.
5. Give verifiers the governing specification, APS scope, base/head or bounded diff, acceptance criteria, and gates — **never** the executor's reasoning transcript.
6. Fresh adversarial verification via `verify-loop`. Verifier is read-only. Prefer a different model/harness for high-risk or disputed findings when available.
7. Evidence gates every transition. No advance from confidence or another agent's success claim.
8. Repository policy and branch protection outrank the mode flag. Session-scoped human overrides must name scope, authority, expiry, and be recorded in evidence.
9. During `land-branch`, APS status/Files/evidence reconciliation is explicit
   loop authority for the current ReadyItem. This overrides passive
   `aps-planning` advice to always ask before edits; broader scope changes still
   require a checkpoint.

## Roles

- **Orchestrator** (this session): scope, claims, state, repair routing, terminal decision.
- **Executor**: implement a bounded ReadyItem in an owned workspace (`build-tdd`).
- **Advisor**: investigate/critique without implementation writes (`grill-design` / design passes).
- **Specialist**: domain-bounded executor or advisor.
- **Verifier**: independent `verify-loop`; read-only.

### Claude Code binding

- Lead session = orchestrator; sole owner of claims, APS reconcile, repair routing, PR state, merge authority.
- Use subagents for executors, advisors, specialists, and blind verification.
- Every writing teammate gets its own Worktrunk worktree + child lease — never one shared write tree.
- Blind verify: fresh subagent with only contract + bounded diff; no executor transcripts.
- If subagents/teams unavailable, degrade to sequential fresh contexts and record the limitation.

## Loop

### 1. Resolve

Resolve intent, target, policy, mode, APS scope, dependencies, risk, integration
branch, required gates. Risk = max(APS metadata, policy, assessment). Raise freely;
lower only via recorded human override.

#### 1a. Open-PR and recent-merge poll (before new work)

Before claiming a **new** target (or when choosing "what's next"):

```bash
gh pr list --author "@me" --state open --json number,title,baseRefName,headRefName,url,reviewDecision,statusCheckRollup
# Also scan recently closed/merged PRs that are stack deps or same module:
gh pr list --author "@me" --state merged --limit 15 --json number,title,baseRefName,headRefOid,mergeCommit,url
```

For each relevant **open** PR:

1. CI red → **`address-reviews`** first.
2. Unresolved threads or `CHANGES_REQUESTED` → **`address-reviews`**.
3. Clean + mergeable + policy allows → may merge if autonomous+authorised.
4. If next work **depends on** an open PR branch, record it as **stack base**
   (`land-branch` 3b). Do not assume integration base.

For each relevant **merged** PR (deps, prior cycle, same module):

5. Run **integration-ancestor check** (`land-branch` step 4e).
   `MERGED` into a deleted or non-integration base is **not** integrated.
6. If not an ancestor of integration: do **not** treat APS as `Merged`; repair
   stack / re-merge / escalate before building dependents on that work.
7. If ancestor check passes but head branch still exists: delete it when safe
   (policy `deleteBranchOnMerge`) so stacks do not attach to zombies.

Skip the poll only when the user named a single already-isolated resume target
and explicitly said to ignore other PRs.

#### 1b. Target truth

- Natural language → invoke **`plan-ready`** (which may call **`grill-design`**).
- APS id → load **`aps-planning` from the repo path** when present
  (`.claude/skills/aps-planning/SKILL.md` or `.codex/skills/aps-planning/SKILL.md`)
  to avoid global shadowing; run truth validation; on failure route to `plan-ready`.
- Do not implement stale, ambiguous, blocked, or unauthorised work.
- Resolve specification precedence before build. Binding order is: accepted ADRs
  and repository policy, then module specification, then action plan / ReadyItem.
  An action plan may narrow parent scope only when it records the narrowing and
  rationale explicitly. If the action plan is narrower but silent, implement the
  ReadyItem scope conservatively and ask `verify-loop` to flag parent-spec drift.
- Map dependencies: if required upstream items are only on unmerged PRs, decide
  **stack / wait / escalate** before isolate (default: stack when CI green and
  single parent; wait when dep is red or contested; escalate when multi-parent).

Exit of plan stage must be a **ReadyItem** (`references/contracts.md`) with
Decision `ready` before isolate. Note `prBase` / stack dependency on the ReadyItem
or Resolve notes when not integration.

### 2. Claim and isolate

Acquire hierarchical claim before any write; refuse conflicts. Record operator,
session, executor, target, parent claim, branch, workspace, base revision, lease.

Invoke **`isolate-workspace`**. If stacking, create the branch **from the
dependency tip** (not from stale integration). Reuse current worktree only when
clean, already on the owned feature branch, and no parallel writer. Otherwise
Worktrunk / worktree + fresh PR branch. Green baseline or documented inherited
failures. Always run Setup after create/switch.

Checkpoint: write `.dev-loop/checkpoints/<runId>.json` with `phase: claimed`,
branch, workspace, base revision, mode, target, and next action. Update the same
file after each later phase; do not create multiple checkpoint locations.

### 3. Design when required

If ReadyItem risk or policy demands design and design source is missing:

- Standard path: **`grill-design`** until design-approved, then refresh ReadyItem via `plan-ready`.
- High-risk / architectural / irreversible: obtain two independent design proposals
  (separate contexts; do not cross-show), synthesise, record choice, then ReadyItem.

**Do not skip `grill-design` as "mechanical migration"** when any of these are
open: product framing, vendor/tool choice beyond a pin, scope of optional
components, UX copy that implies capability, or “probably not needed” features.
Mechanical means: rote mechanical edits with **already-recorded** approach and
no framing ambiguity. When in doubt, one short grill pass is cheaper than doc
reframing after the executor finishes.

Mid-run scope steers from the user: pause BUILD, update ReadyItem / design
notes, then continue — do not silently keep the old framing.

### 4. Execute

Invoke **`build-tdd`** against the ReadyItem. On unexpected failure → **`debug`**,
then resume build. Keep APS `In Progress`. Checkpoint phases: `implemented`,
`verified`, `review-ready` — do not invent new APS lifecycle states.

Parallelise only dependency-independent work with explicit ownership and child leases.

### 5. Executor evidence

Before any success claim, invoke **`evidence-gate`**. Result must be `supported`
(or accepted inherited failures named in Notes).

### 6. Verify

Invoke **`verify-loop`** with governing contract and bounded change set. Objective
gate failures and high-confidence critical/major findings are binding. Subjective
findings → orchestrator judgement; material disputes → differential or Council.

Do not skip blind verification for “easy” modules. Cheap work still gets a fresh
read-only verifier because the common findings are small but real: documentation
drift, missing guard tests, stale plan wording, and baseline gaps.

### 7. Repair

Choose original executor, fresh executor, or specialist. Verifier does not repair
by default. Continue while evidence improves and repair budget remains. On
no-progress or budget exhaust → structured blocker, not infinite loop. Ambiguity
that changes intent → `needs-plan-update` / `plan-ready`.

After each repair: `build-tdd` → `evidence-gate` → `verify-loop`.

### 8. Review and land

Default PR boundary = invocation target. Split early when size/risk warrants; record.

Invoke **`land-branch`** (mandatory APS reconcile when APS exists; **delete
branch on merge**; **integration-ancestor check** before `Merged` /
`integrated`):

- **Interactive:** stop at review-ready (PR open, CI green, agent findings resolved,
  APS status/Files updated for this item — still `In Progress` until on integration).
- **Autonomous:** chase CI/review via **`address-reviews`**, re-verify every repair,
  merge only when authorised, prove ancestor of integration, set `Merged`, release claim.

After true integration merge: update base before dependents. APS:
`Ready → In Progress → Merged → Released/Shipped → Complete`.

### 9. Reconcile and release

`land-branch` owns per-item APS reconcile on land. Orchestrator double-check:

1. **Git truth over PR labels:** for every item claimed merged this session or
   in the open-PR poll, re-run integration-ancestor check.
2. APS statuses match Git/PR **and** ancestor proof.
3. Evidence, checkpoint, claims, journal.
4. **Human / out-of-band merges:** if someone merged outside the loop, follow
   policy `aps.outOfBandMergeReconcile` (bookkeeping branch vs blessed direct
   commit on integration vs forbid). Do not invent a path — policy decides.

Report outcomes first: integrated work (with ancestor proof), verification
evidence, APS changes, open risks, overrides, next eligible target.

### Resume friction (expect and handle)

- **Unpushed local integration:** fetch `origin/<integration>`; stash/rebase/push
  local commits before creating stack bases from a divergent local tip.
- **Worktrunk “unmerged” after squash/rebase-merge:** if 4e passes, force-remove
  the stale worktree/branch marker; SHA identity will not match pre-merge heads.

## Stop outcomes

Return exactly one:

| Outcome                    | Meaning                                              |
| -------------------------- | ---------------------------------------------------- |
| `review-ready`             | Interactive terminal; PR ready for human             |
| `integrated`               | Merged; post-merge requirements passed               |
| `blocked`                  | External dependency, authority, safety, or ambiguity |
| `repair-budget-exhausted`  | No material progress within policy                   |
| `needs-plan-update`        | Replan via `plan-ready`                              |
| `claim-conflict`           | Another operator owns the target                     |
| `awaiting-merge-authority` | Review-ready but merge not authorised                |

Never call work complete when the strongest state is only implemented, verified,
or review-ready.

## Required sibling skills

Install these next to this skill (flat `.claude/skills/` or `.codex/skills/`):

`grill-design`, `plan-ready`, `isolate-workspace`, `build-tdd`, `debug`,
`evidence-gate`, `land-branch`, `address-reviews`, `verify-loop`, and
`aps-planning` when the project uses APS.

## Skill load / pinning caveat

Claude Code (and some harnesses) may resolve a skill **name** from the
user/global skill dir (`~/.claude/skills`) instead of the repo copy when names
collide. Pack-unique names (`plan-ready`, `grill-design`, `build-tdd`, …) avoid
this. Shared names (`aps-planning`, `dev-workflow`) can load the global copy.

**Required procedure for shared names while this pack is active:**

1. Prefer the Skill tool only for **pack-unique** stage names.
2. For `aps-planning` (and any other colliding name), **Read** the project file
   first:
   - `.claude/skills/aps-planning/SKILL.md` or
   - `.codex/skills/aps-planning/SKILL.md`
     then follow that content. Do not assume the Skill tool bound the repo copy.
3. If the repo file is missing, say so and fall back to global — record
   `skillSource: global` in Notes.
4. Long-term fix (catalogue): rename shared deps to pack-unique ids
   (e.g. `aps-planning-core`) at promotion time.

## Install hygiene

Never rsync with an unquoted space-separated path list as a single destination.
Each skill is its own directory under `.claude/skills/<name>/`.
