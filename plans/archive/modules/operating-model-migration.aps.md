<!--
APS Module: Operating Model Migration
=====================================
Coordinates implementation of the target Plan / Build / Release operating model
across branching, release, documentation, APS lifecycle, review/council,
agent guidance, CI, and recovery. See: plans/aps-rules.md
-->

# Operating Model Migration

| ID      | Owner | Status   | Progress |
| ------- | ----- | -------- | -------- |
| OPMODEL | —     | Complete | 12/12    |

**Spec:** [2026-05-09 Plan / Build / Release Operating Model](../specs/2026-05-09-plan-build-release-operating-model.md)
**Execution architecture:** [2026-05-09 Agentic Execution Ecosystem Architecture](../specs/2026-05-09-agentic-execution-ecosystem-architecture.md)
**Review architecture:** [2026-05-09 Council Agent And Skill Change Proposal](../specs/2026-05-09-council-agent-skill-change-proposal.md)

## Purpose

Implement one coherent development process and workflow for humans and agents:

```text
APS -> branch/worktree -> deterministic guidance -> implementation
  -> local checks -> targeted review -> PR -> CI -> merge
  -> release candidate -> tag -> artefact verification -> release record
  -> APS shipped-state reconciliation
```

This module coordinates the migration from today's mixed `dev` integration,
manual documentation closeout, release-runbook, and prompt-driven review model to
the target operating model where:

- APS owns intent, readiness, scope, acceptance criteria, and release metadata.
- `main` becomes the only permanent product branch after migration.
- CI owns validation truth for commit SHAs.
- Tags own released source snapshots.
- GitHub Release assets own distributed artefacts.
- Release records own shipped-state reconciliation.
- Skills route to playbooks and deterministic commands.
- Agents provide judgement, synthesis, and critique.
- Hooks are deterministic guardrails only.

## Cross-Cutting Convention

This is a cross-cutting APS module and follows the rules in
[`plans/aps-rules.md#module-types-vertical-and-conductor`](../../aps-rules.md#module-types-vertical-and-conductor).
Task closeout must sweep `Coordinates with:`, `Blocks on:`, `Supersedes:`, and
`Superseded by:` callouts rather than carrying unresolved references into
archive.

OPMODEL owns coordination, sequencing, vocabulary, and migration boundaries. It
does not take implementation ownership away from specialist modules such as
RELORCH or DOCGOV.

## In Scope

- Current-state versus target-state migration map.
- APS lifecycle vocabulary and release metadata alignment.
- Branching and worktree policy migration to trunk-first `main`.
- Release-record schema and release-readiness integration boundaries.
- Review/council trigger rules and playbook entrypoints.
- Deterministic agent guidance shared by hooks, skills, agents, and CI.
- CI release-readiness and candidate artefact workflow coordination.
- Drift checks for APS/repo/release consistency.
- Rollback and incident playbooks for bad `main`, bad artefacts, and bad
  releases.

## Out of Scope

- Implementing the per-phase release commands — owned by RELORCH.
- Implementing documentation validators and generated indexes — owned by DOCGOV.
- Rewriting every stale document in one pass.
- Changing Anvil product scope, release contents, or cargo-dist asset topology.
- Building a general-purpose agent orchestration platform.

## Interfaces

**Depends on:**

- `plans/specs/2026-05-09-plan-build-release-operating-model.md` — target
  lifecycle and source-of-truth hierarchy.
- `plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md` — skill,
  agent, hook, session, event, and deterministic/probabilistic boundaries.
- `plans/specs/2026-05-09-council-agent-skill-change-proposal.md` — review tier
  and planning-council model.
- `plans/aps-rules.md` — APS structure and cross-cutting module convention.
- `docs/guides/branching-strategy.md` and `docs/guides/worktree-policy.md` —
  current branching and workspace guidance.
- `docs/guides/release-runbook.md` — current release procedure.

**Coordinates with:**

- RELORCH — release command surface, release tracking issue comments, release
  record generation, and release skill/runbook wiring.
- CICD — CI/CD cost controls, validation layering, shared path/risk
  classification, local-first validation commands, and release-readiness workflow
  implementation.
- DOCGOV — documentation taxonomy, metadata, closeout, generated indexes, and
  docs validation commands.
- CGBDG — council evidence bridge where review outputs become durable evidence.
- TRACE / ADR-035 — event, session, and provenance pipe allocation.
- CI workflows — release-readiness, candidate artefact, and drift-check gates.

**Exposes:**

- Canonical operating-model migration plan.
- Shared lifecycle and terminology for downstream docs and skills.
- Sequenced implementation backlog across docs, APS, review, guidance, CI, and
  release.
- Migration exit criteria for retiring normal `dev` workflow.

## Target Lifecycle

All downstream artefacts must use this lifecycle vocabulary:

```text
APS Draft -> APS Proposed -> APS Ready -> In Progress -> Merged -> Released/Shipped -> Complete/Archived
```

- `Ready` means execution is authorised.
- `In Progress` means work has started.
- `Merged` means code reached the integration target, but has not necessarily
  shipped.
- `Released` / `Shipped` means a release record proves inclusion in a verified
  release.
- `Complete` means the APS item or module has no remaining active closeout work
  and may be archived under APS rules.

`Committed` is legacy wording for `Merged` unless a specific module defines a
narrower transition. New operating-model artefacts should prefer `Merged` and
`Released/Shipped`.

## Migration Phases

1. **Declare authority:** make current-state and target-state boundaries explicit.
2. **Standardise vocabulary:** update APS rules, docs, skills, and PR templates.
3. **Add deterministic guidance:** introduce shared path/risk guidance in warning
   mode.
4. **Wire review and planning:** make targeted review and planning-council gates
   executable through playbooks and sessions.
5. **Implement release records and readiness:** add release candidate,
   release-readiness, candidate artefact, and release-record contracts.
6. **Migrate branching:** prepare cutover controls, freeze new `dev` PRs during
   the cutover window, promote current `dev` to `main`, retarget normal work to
   `main`, and protect or retire `dev`.
7. **Promote drift checks:** start warning-only, then require stable APS/repo/
   release consistency checks.

## Closure Criteria

This module is Complete when:

1. Branching, worktree, release, APS, review, and documentation guidance all cite
   the same lifecycle and source-of-truth hierarchy.
2. Normal work branches from `main` and normal PRs target `main`.
3. `dev` is either retired, protected against normal work, or explicitly retained
   as a dated compatibility branch.
4. Release readiness is recorded by CI against commit SHAs.
5. Release records are emitted and used for APS shipped-state reconciliation.
6. Agent guidance, review routing, and council escalation use one shared rule
   source.
7. Documentation validation and APS/repo/release drift checks are available in CI
   at least in warning mode.

## Tasks

### OPMODEL-001: Current-state to target-state migration map

- **Status:** Complete
- **Authorisation:** Operator explicitly approved executing OPMODEL-001 on
  2026-05-10; this approval treated the item as Ready for this slice before it
  moved to Complete.
- **Intent:** Produce the authoritative migration map from today's `dev`-based
  workflow to trunk-first `main`, including what remains current-state, what is
  target-state, and what must not be mixed.
- **Expected Outcome:** A migration section or companion spec lists each affected
  surface, its current behaviour, target behaviour, owner module, and cutover
  preconditions.
- **Validation:** `pnpm format:check`
- **Files:** `plans/specs/2026-05-09-plan-build-release-operating-model.md`,
  `plans/modules/operating-model-migration.aps.md`
- **Coordinates with:** RELORCH, DOCGOV
- **Completed:** 2026-05-10 — Added the authoritative current-state to
  target-state migration map to the operating model specification; validation
  passed with `pnpm format:check`, `pnpm lint:md`, and `git diff --check`.
- **Closeout follow-up:** PR #1378 merged before Copilot's post-merge comments;
  this follow-up clarifies the Minimum Viable Operating Model cutover caveat and
  records the documentation closeout in the PR body.
- **Confidence:** high

---

### OPMODEL-002: Branching and worktree policy target-state update

- **Status:** Complete
- **Authorisation:** Operator explicitly approved executing OPMODEL-002 on
  2026-05-10; this approval treated the item as Ready for this slice before it
  moved to Complete.
- **Intent:** Update human-facing branch and worktree guidance so it clearly
  separates current compatibility behaviour from the target `main`-first model.
- **Expected Outcome:** Branching and worktree docs describe one permanent
  product branch (`main`), disposable normal branches from `main`, exceptional
  `release/*` and `hotfix/*` use, and `dev` retirement rules.
- **Validation:** `pnpm format:check && pnpm lint:md`
- **Files:** `docs/guides/branching-strategy.md`, `docs/guides/worktree-policy.md`
- **Blocks on:** OPMODEL-001
- **Completed:** 2026-05-10 — Updated branch and worktree guidance to separate
  current `dev` compatibility execution from target `main`-first execution;
  validation passed with `pnpm format:check`, `pnpm lint:md`, and
  `git diff --check`.
- **Confidence:** high

---

### OPMODEL-003: APS lifecycle and release metadata rules

- **Status:** Complete
- **Authorisation:** Operator explicitly approved moving on through OPMODEL work
  on 2026-05-10; this approval treated OPMODEL-003 as Ready for this slice.
- **Intent:** Make APS rules describe the shared lifecycle and metadata required
  for target-state execution and release reconstruction.
- **Expected Outcome:** `plans/aps-rules.md` defines `Merged`,
  `Released/Shipped`, legacy `Committed` mapping, validation metadata,
  `changeType`, `releaseIntent`, `releaseScope`, and release-note fields.
- **Validation:** `pnpm format:check && pnpm lint:md`
- **Files:** `plans/aps-rules.md`, `docs/public/aps/**` if public APS docs need
  alignment
- **Coordinates with:** DOCGOV-003
- **Completed:** 2026-05-10 — Updated APS lifecycle rules, release metadata, and
  public APS docs so package schema truth stays distinct from Anvil repository
  operating-model extensions; validation passed with `pnpm format:check`,
  `pnpm lint:md`, and `git diff --check`.
- **Confidence:** medium

---

### OPMODEL-004: Release record schema and authority contract

- **Status:** Complete
- **Authorisation:** Operator explicitly approved moving on through OPMODEL work
  on 2026-05-10; this approval treated OPMODEL-004 as Ready for this slice.
- **Intent:** Define the release record as the canonical shipped-state artefact
  without conflicting with RELORCH's GitHub tracking issue operator log.
- **Expected Outcome:** A schema specifies release version, source SHA, previous
  tag, APS items, assets, checksums, private/public release URLs, verification
  timestamp, and policy decisions. The schema states where records are emitted
  and how APS reconciliation consumes them.
- **Validation:** `pnpm format:check && pnpm lint:md`
- **Files:** `plans/specs/2026-05-10-release-record-schema.md`,
  `plans/specs/2026-05-09-plan-build-release-operating-model.md`
- **Blocks on:** RELORCH-001 if the schema is folded into the release command
  surface design
- **Completed:** 2026-05-10 — Added the release-record schema and authority
  contract while preserving RELORCH ownership of command implementation;
  validation passed with `pnpm format:check`, `pnpm lint:md`, and
  `git diff --check`.
- **Confidence:** medium

---

### OPMODEL-005: Release readiness and candidate artefact workflow design

- **Status:** Complete
- **Authorisation:** Operator explicitly approved moving on through OPMODEL work
  on 2026-05-10; this approval treated OPMODEL-005 as Ready for this slice.
- **Intent:** Define the CI workflow that proves a selected `main` SHA is ready
  to tag and optionally builds non-publishing candidate artefacts.
- **Expected Outcome:** CI design covers trigger, inputs, required checks,
  release candidate metadata, candidate artefact retention, and failure handling.
- **Validation:** `pnpm format:check && pnpm lint:md`
- **Files:** `plans/specs/2026-05-10-release-readiness-workflow.md`,
  `.github/workflows/` when implemented
- **Coordinates with:** RELORCH, CICD, CI workflows
- **Completed:** 2026-05-10 — Added the release-readiness and candidate artefact
  workflow specification, defining trigger inputs, required checks, candidate
  metadata, artefact retention, failure classes, and release-record integration;
  workflow implementation remains with RELORCH/CI follow-up work. Validation
  passed with `pnpm format:check`, `pnpm lint:md`, and `git diff --check`.
- **Confidence:** medium

---

### OPMODEL-006: Release runbook and skill migration boundary

- **Status:** Complete
- **Authorisation:** Operator explicitly approved moving on through OPMODEL work
  on 2026-05-10; this approval treated OPMODEL-006 as Ready for this slice.
- **Intent:** Align the release runbook and release skill with current-state and
  target-state truth so agents do not execute non-existent commands or mix `dev`
  promotion with tag-from-main release semantics.
- **Expected Outcome:** Runbook and skill explicitly label compatibility paths,
  target paths, command availability, release-record usage, and emergency manual
  recovery boundaries.
- **Validation:** `pnpm format:check && pnpm lint:md`
- **Files:** `docs/guides/release-runbook.md`, `.claude/skills/release/SKILL.md`
- **Coordinates with:** RELORCH-011
- **Completed:** 2026-05-10 — Split the release runbook and release skill into
  current compatibility mode and target command mode so agents do not execute
  missing `scripts/release/*` commands or treat target `main` release semantics
  as executable before OPMODEL-012. Validation passed with `pnpm format:check`,
  `pnpm lint:md`, and `git diff --check`.
- **Confidence:** high

---

### OPMODEL-007: Deterministic agent guidance script

- **Status:** Complete
- **Authorisation:** Operator explicitly approved moving on through OPMODEL work
  on 2026-05-10; this approval treated OPMODEL-007 as Ready for this slice.
- **Intent:** Add one deterministic rules engine that maps changed paths, APS
  items, and risk classes to playbooks, review tiers, and required checks.
- **Expected Outcome:** `scripts/agent/guidance.sh` supports staged, branch, and
  PR modes; emits JSON for agents/CI and concise text for humans/hooks; starts in
  advisory mode.
- **Validation:** Shell tests or fixture tests for representative path/risk cases;
  `pnpm format:check`
- **Files:** `scripts/agent/guidance.sh`, `scripts/agent/_test/`, CI warning
  touchpoint when implemented
- **Coordinates with:** council/review specs, hooks, CI workflows
- **Completed:** 2026-05-10 — Added advisory `scripts/agent/guidance.sh` with
  staged, branch, PR, and fixture-file modes; text and JSON output; deterministic
  path rules for release, APS, docs, agent workflow, CI, TypeScript, and Rust
  changes; and shell fixture tests. CI integration remains a later warning-mode
  touchpoint. Validation passed with `bash scripts/agent/_test/guidance.test.sh`,
  `pnpm format:check`, `pnpm lint:md`, and `git diff --check`.
- **Confidence:** medium

---

### OPMODEL-008: Review and council entrypoint alignment

- **Status:** Complete
- **Authorisation:** Operator explicitly requested starting OPMODEL-008 after
  OPMODEL-007 on 2026-05-10; this approval treated the item as Ready for this
  slice.
- **Intent:** Make `/review`, `/council`, and planning-council playbooks use the
  same review tiers and transition points defined by the operating model.
- **Expected Outcome:** `/review` becomes targeted pre-PR review; `/council`
  supports quick, mini, full, status, and publish; planning-council playbooks
  cover creation, direction validation, pre-execution validation, and amendment.
- **Validation:** `pnpm format:check && pnpm lint:md`
- **Files:** `.claude/commands/review.md`, `.claude/commands/council.md`,
  `.claude/skills/planning-council/**` or equivalent skill/playbook paths
- **Coordinates with:** CGBDG
- **Completed:** 2026-05-10 — Updated repo-local `/review` to be targeted
  pre-PR review, updated `/council` to support quick, mini, full, status, and
  publish modes, and added local planning-council playbooks for plan creation,
  direction validation, pre-execution validation, and amendment. Validation
  passed with `pnpm format:check`, `pnpm lint:md`, and `git diff --check`.
- **Coordination closeout:** CGBDG remains the downstream owner for durable
  council evidence bridging. This slice aligns entrypoint vocabulary only;
  durable session/schema integration stays with OPMODEL-009 and CGBDG.
- **Confidence:** medium

---

### OPMODEL-009: Workflow session and event schema alignment

- **Status:** Complete
- **Authorisation:** Operator explicitly requested moving on to OPMODEL-009 on
  2026-05-10; this approval treated the item as Ready for this slice.
- **Intent:** Define the durable session and event records needed for agents,
  reviews, planning validation, release, and recovery without making chat
  history authoritative.
- **Expected Outcome:** Session/event schemas include workflow id, state
  transition, actor, tool invoked, inputs/outputs digest, APS items, branch/SHA,
  validation result, error class, and human approval events. Pipe allocation
  cites ADR-035.
- **Validation:** `pnpm format:check && pnpm lint:md`
- **Files:** `plans/specs/<date>-workflow-session-and-event-schema.md`,
  `schemas/workflow-session-event.schema.json`, `.claude/council/schema.json`
  when migrated
- **Coordinates with:** TRACE / ADR-035, CGBDG
- **Completed:** 2026-05-10 — Added the workflow session/event schema spec and
  tracked `schemas/workflow-session-event.schema.json` contract covering workflow id, state
  transitions, actors, tool invocations, input/output digests, APS items,
  branch/SHA anchors, validation results, error classes, and human approval
  events. Existing `.claude/council/schema.json` remains the shared council
  schema symlink until CGBDG migrates writers/readers. The spec cites ADR-035
  pipe allocation and keeps chat history non-authoritative. Validation passed
  with `pnpm format:check`, `pnpm lint:md`, and `git diff --check`.
- **Coordination closeout:** TRACE / ADR-035 owns the pipe allocation rule and
  future trace correlation; CGBDG remains the downstream owner for durable
  council evidence bridging. This slice defines the shared contract only.
- **Confidence:** medium

---

### OPMODEL-010: APS/repo/release drift checks

- **Status:** Complete
- **Authorisation:** Operator explicitly requested continuing with the next
  OPMODEL steps on 2026-05-10; this approval treated the item as Ready for this
  slice.
- **Intent:** Add warning-mode checks for inconsistent APS, repository, and
  release state before promoting stable rules to required gates.
- **Expected Outcome:** Checks detect changed files without APS references,
  inconsistent module counts, Complete items without validation evidence,
  candidate contents missing merged APS items, version/tag mismatch, missing
  release artefacts, and shipped APS state without release records.
- **Validation:** Fixture tests for each drift class; CI warning output on a
  controlled fixture
- **Files:** `scripts/aps/drift-check.mjs`, `scripts/aps/_test/drift-check.test.sh`,
  `.github/workflows/ci.yml`, `package.json`
- **Coordinates with:** DOCGOV-005, RELORCH, release-readiness workflow
- **Completed:** 2026-05-10 — Added warning-mode APS/repo/release drift checks
  that report changed files without APS references, inconsistent module/index
  counts, Complete items without validation evidence, candidate records missing
  merged APS items, version/tag mismatches, missing release artefact integrity,
  and shipped APS state without matching published release records. CI runs the
  checker as a non-blocking advisory job, and fixture tests cover each drift
  class. Validation passed with `pnpm test:aps-drift`, `pnpm format:check`,
  `pnpm lint:md`, and `git diff --check`.
- **Coordination closeout:** DOCGOV-005 remains the downstream owner for broader
  documentation validators and generated indexes. RELORCH and the
  release-readiness workflow remain owners of release-record emission and
  candidate inputs; this slice provides warning-only consistency checks and does
  not make them required gates.
- **Confidence:** medium

---

### OPMODEL-011: Rollback and incident playbooks

- **Status:** Complete
- **Authorisation:** Operator explicitly requested starting OPMODEL-011 on
  2026-05-11 after OPMODEL-010 completion; this approval treats the item as
  Ready for this slice.
- **Intent:** Make recovery paths executable for bad `main`, bad candidate
  artefacts, bad published releases, and emergency hotfixes.
- **Expected Outcome:** Playbooks define triggers, commands, success criteria,
  rollback/supersession rules, release-record updates, and APS/issue closeout.
- **Validation:** `pnpm format:check && pnpm lint:md`
- **Files:** `docs/runbooks/rollback-bad-main.md`,
  `docs/runbooks/rollback-bad-candidate-artefact.md`,
  `docs/runbooks/rollback-bad-published-release.md`,
  `docs/runbooks/emergency-hotfix.md`,
  `.claude/skills/release/SKILL.md`
- **Coordinates with:** RELORCH, DOCGOV-006
- **Completed:** 2026-05-11 — Added four operator-facing playbooks under
  `docs/runbooks/` for bad `main`, bad candidate artefact, bad published
  release, and emergency hotfix. Each defines triggers, decision tree,
  commands, success criteria, release-record updates per the
  [release-record schema](../specs/2026-05-10-release-record-schema.md), APS /
  issue closeout, and compatibility-vs-target mode notes. Wired the release
  skill's Emergency Recovery section to the playbooks while keeping mutating
  release commands operator-owned in compatibility mode. Validation passed
  with `pnpm format:check`, `pnpm lint:md`, `git diff --check`, and
  `pnpm aps:drift` showed no new findings attributable to this change.
- **Confidence:** medium

---

### OPMODEL-012: Main-first cutover and dev retirement

- **Status:** Complete
- **Authorisation:** Operator approved phased execution on 2026-05-11; Phase 0
  (audit + playbook + APS bump) landed in PR #1410; Phase 2 (operator-driven
  cutover) and Phase 3 (docs flip + APS close-out) followed in the same
  window.
- **Intent:** Execute the actual branch migration only after guidance, release
  readiness, and documentation are coherent enough to support normal work on
  `main`.
- **Expected Outcome:** Current `dev` is promoted to `main`; normal PRs target
  `main`; `dev` is protected, retired, or given an explicit compatibility
  expiry; runbooks and PR templates no longer describe normal `dev -> main`
  promotion.
- **Validation:** Branch protections and CI pass on `main`; docs and PR
  template cite the main-first model; no active runbook requires normal
  back-merge.
- **Blocks on:** OPMODEL-001, OPMODEL-002, OPMODEL-003, OPMODEL-004,
  OPMODEL-005, OPMODEL-006, OPMODEL-007, OPMODEL-008, OPMODEL-010,
  OPMODEL-011 — all Complete at cutover time; callouts resolved.
- **Action plan:**
  [`plans/execution/opmodel-012.steps.md`](../execution/opmodel-012.steps.md)
- **Phase 0 outputs (PR #1410):**
  [`plans/audits/2026-05-11-opmodel-012-workflow-audit.md`](../audits/2026-05-11-opmodel-012-workflow-audit.md),
  [`docs/runbooks/main-first-cutover.md`](../../docs/runbooks/main-first-cutover.md)
- **Files:** `docs/guides/branching-strategy.md`,
  `docs/guides/worktree-policy.md`, `docs/guides/release-runbook.md`,
  `.claude/skills/release/SKILL.md`, `.github/workflows/pr-base-guard.yml`
  (deleted in PR #1417), the 6 cleanup workflows (`ci`, `codeql`, `napi`,
  `release-harness`, `rust`, `security`), `.github/dependabot.yml`,
  `docs/runbooks/emergency-hotfix.md`.
- **Completed:** 2026-05-11 — Phase 0 audit + playbook merged in PR #1410.
  Phase 2 executed live with operator approval per step. Phase 3 docs flip in
  the present PR. Receipts:
  - **Cutover SHA (FF push):**
    `b6f236e90dbc03338f17767202acf93f1449f8d2` (operator log entry).
  - **Rollback boundary (`pr-base-guard.yml` retirement PR #1417 merge):**
    `62d85777c03ffe9a196befc9390a7d0a18ff0ee8`.
  - **Default branch:** `main` (flipped via `gh api -X PATCH`).
  - **`main` ruleset:** id 16217152, 7 required checks
    (APS Drift Check, Docs Lint, Lint & Format, Type Check, Unit Tests
    (Node 22.x, ubuntu-latest), Security Summary, Detect Changes), PR
    required with thread-resolution + stale-dismiss, non-FF + deletion
    blocked, copilot code review, admin bypass retained as deliberate
    operator escape hatch.
  - **`dev (retired)` ruleset:** id 16217300, blocks deletion/non-FF/
    creation/update on `refs/heads/dev`.
  - **`dev` disposition:** dated compatibility branch — tag
    `dev-retired-2026-05-11` at `b6f236e9`, deletion scheduled on or after
    2026-07-10 (follow-up issue #1419).
  - **Smoke verification:** PR #1418 (closed without merging) — all 7
    required checks passed against `main` under the new ruleset.
  - **Phase 0 → Phase 2 evidence:** Phase 0 audit found exactly one
    cutover-blocking workflow (`pr-base-guard.yml`, retired in PR #1417);
    6 post-cutover cleanup workflows (`ci`, `codeql`, `napi`,
    `release-harness`, `rust`, `security`) had `dev` triggers dropped in
    Phase 3.
  - Validation: `pnpm format:check`, `pnpm lint:md`, `pnpm aps:drift` no
    new findings, `git diff --check` clean.
- **Confidence:** high

## Cross-cutting callout closure summary

Per [`plans/aps-rules.md#module-types-vertical-and-conductor`](../../aps-rules.md#module-types-vertical-and-conductor)
rule 4, all open callouts in this module body are swept and resolved before
archive:

- **Module-level `Coordinates with:`** (RELORCH, CICD, DOCGOV, ADR-035,
  CGBDG): documented-and-closed. Each named coordinator delivered or accepted
  hand-off in its own module. RELORCH-001..-004 design and command surface
  partially shipped; CICD-001..-004, -009, -012 shipped during this window;
  DOCGOV continues separately on documentation-governance closeout. No
  remaining open coordination obligations live in this module.
- **Per-task `Coordinates with:`** (OPMODEL-001..-011): documented-and-closed
  inline in each work item's completion line. The coordinating modules
  accepted the design or guidance produced; further execution is owned by
  those modules.
- **Per-task `Blocks on:`** (OPMODEL-005 internal-to-OPMODEL dependencies,
  OPMODEL-012's dependency on -001..-008/-010/-011): all satisfied at time
  of cutover (each predecessor item is Complete). Resolved.

Provenance: see `plans/aps-rules.md` provenance section — OPMODEL was the
second cross-cutting trial after LAUNCH; this archive close exercises the
rule's archive-time sweep clause for the first time.
