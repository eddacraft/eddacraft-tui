---
name: dev-loop
description: >-
  Compatibility/front-door skill for the canonical agentic delivery lifecycle.
  Use for APS work items, modules, goals, resume, or drain. Loads agentic-loop,
  then the tiny runtime router; native harness adapters own choreography.
---

# Development loop

Load and obey `agentic-loop` first. This skill preserves the familiar invocation
surface while retiring the old mandatory orchestrator→executor choreography.

## Invocation

```text
/dev-loop complete DASH-001
/dev-loop complete DASH
/dev-loop goal "Add tenant export"
/dev-loop resume DASH-001
/dev-loop drain [MODULE]
```

Treat `/dev-loop <TARGET>` as `complete <TARGET>`.

## Dispatch

1. Resolve repository `devLoop` policy and APS target truth.
2. Load `dev-loop-router` to select the uniquely named native harness adapter.
3. Item targets follow the canonical `agentic-loop` lifecycle directly.
4. Module targets load `dev-loop-module` for module topology, calibration, and
   integrated close verification.
5. `dev-loop-differential` may alter role/model routing only; it never changes
   scope, evidence, verification, merge authority, or terminal outcomes.

## Contracts and policy

Load repository policy before asking for mode; never infer merge authority from
silence. Shared contracts and machine-readable schemas live alongside this skill:

- [references/policy-contract.md](references/policy-contract.md) — policy search
  order, defaults, budgets, isolation, publication, merge authority.
- [references/contracts.md](references/contracts.md) — ReadyItem and the shared
  stage contracts.
- [references/coordination-module.md](references/coordination-module.md) — claims
  and child leases.
- [references/dev-loop.policy.template.yaml](references/dev-loop.policy.template.yaml)
  — starting point when a repository has no policy file yet.
- [references/differential-handoff.md](references/differential-handoff.md) —
  cross-harness hand-off shape used by `dev-loop-differential`.
- [references/](references/) — claim, evidence-bundle, run-checkpoint, and
  differential-routing schemas.

## Durable run state

Write one checkpoint per run and update it at every phase transition, evidence
gate, verifier decision, and landing step. The compatibility default path is
`.dev-loop/checkpoints/<runId>.json` against
[references/run-checkpoint.schema.json](references/run-checkpoint.schema.json);
a native runtime may own the store instead. Do not create a second checkpoint
location.

Budgets are finite and shared:

- `maxConcurrentWriters` bounds concurrent writers; `maxItemsPerWave` bounds
  items per wave. Both default to 2.
- Child runs share the parent deadline and repair budget. They never receive
  fresh budgets.
- Stop on a repeated fingerprint of progress. A run that is not moving escalates
  through `aps-escalation-queue`; it does not keep spending.

Evidence reuse requires an immutable exact match against
[references/evidence-bundle.schema.json](references/evidence-bundle.schema.json):
at least one command plus recorded repository state. An empty command list is
never reusable evidence.

## Pull request state

Poll live pull-request state rather than trusting a cached view. Request
`mergeable` and `mergeStateStatus` alongside checks and reviews.

- `CONFLICTING` or `DIRTY` routes to `address-reviews` for base-sync repair,
  then back through evidence and re-verification of the changed surface.
- Green CI and resolved reviews are not sufficient to merge on their own.
- Re-check live mergeability immediately before any merge attempt. A stale
  mergeability read is not a merge gate.

## Drain mode

`drain [MODULE]` is the outer loop: planning fills the Ready backlog, drain
empties it one item at a time. Drain grants no execution authority of its own;
every crossing item runs the full canonical lifecycle.

1. **Orient.** Load APS truth via `aps-planning`. Read `plans/execution/lessons/`
   and the tail of the run journal (`plans/execution/loop-journal.md`) when
   present. Resume interrupted cycles via `aps-resume`, re-verifying unless its
   trust window explicitly holds.
2. **Probe.** Establish the repository's real test, lint, and build commands via
   `aps-probe` before executing work items.
3. **Guard.** Check intent against `aps-safety-rails` before any destructive,
   irreversible, or authority-changing action.
4. **Run.** Execute the selected item through `agentic-loop`.
5. **Land.** Cross the merge boundary through `aps-landing`.
6. **Park.** Send blockers and checkpoint questions to `aps-escalation-queue` as
   closed questions with defaults, for the human to clear in one sitting.
7. **Journal.** Append the cycle outcome to `plans/execution/loop-journal.md` and
   capture lessons under `plans/execution/lessons/`. `aps-resume` and `loop-retro`
   both read these; a drain that writes nothing leaves them nothing to work with.
8. **Audit.** `loop-retro` may review the run window report-only. Findings become
   Proposed items or escalation entries, never direct fixes.

Unattended runs require a finite aggregate deadline. Park untouched work rather
than exceeding it.

## Resume

`resume <TARGET>` reconstructs state from APS, Git, pull-request state, and the
run checkpoint via `aps-resume`, then verifies before acting. Never replay
non-idempotent work on the strength of a checkpoint alone.

## Docs Freshness (code-changing runs)

When changed paths are declared docs Upstream, or `docs-owed` would fail, settle
Freshness in the same PR before CI with `pnpm docs:redate --since <base>` (add
`--write --note "..."` when applying). Details live in `agentic-loop` Evidence and
`docs-workflow`. Do not add a calendar grace window.

## Non-negotiable changes from the legacy loop

- The active native harness **lead implements directly by default**.
- A delegated implementation worker is optional, not a required executor stage.
- Independent `verify-loop` remains mandatory.
- Council is adaptive and bounded: use design/assurance modes at decision or
  risk boundaries; never review-until-no-one-can-think-of-anything-else.
- Large, coherent finding sets trigger `REWORK` or `REPLAN` rather than an
  unbounded sequence of finding-by-finding patches.
- Headless `-p`/`exec` transport is not a canonical default. An adapter may use
  supervised headless execution only when its native capabilities cannot meet a
  required boundary and policy permits that transport.

All claims, isolation, evidence, PR, landing, and reconciliation invariants come
from `agentic-loop` plus repository policy.
