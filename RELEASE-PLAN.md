# Anvil Release Plan

**Last updated:** 2026-05-11 (post-OPMODEL-012 cutover; current delivery:
RELORCH + remaining CICD)

> Companion: [ROADMAP.md](./ROADMAP.md) for thematic horizons. Execution source
> of truth: [`plans/index.aps.md`](./plans/index.aps.md) and the linked APS
> modules. This file selects the release slate and shows what can run in
> parallel; it does not duplicate every APS work item.

---

## Current State

**Latest verified tag in repo:** `v0.6.1-beta`

`v0.6.0-beta` and `v0.6.1-beta` shipped wow-start activation and daemon-backed
validation. The active work since `v0.6.1-beta` has been the operational
substrate that makes future releases repeatable:

- **`OPMODEL` — Complete, 12/12, archived 2026-05-11.** Main-first operating
  model migration finished. Cutover SHA `b6f236e9`; `pr-base-guard.yml` retired
  in PR #1417; `main` ruleset id 16217152 enforces 7 required checks, PR review,
  non-FF, and no-delete. `dev` retired, with cutover tagged as
  `dev-retired-2026-05-11`; deletion follow-up #1419 (~2026-07-10).
- **CI/CD readiness (`CICD`) — In Progress, 8/12.** Path/risk classifier
  (CICD-002), local validation commands (CICD-003), fast PR validation
  (CICD-004), coverage cost controls (CICD-006), security/dependency targeting
  (CICD-007), release-readiness workflow (CICD-009), and cutover readiness
  (CICD-012) all shipped. Remaining: CICD-005 (integration SHA validation
  redesign), CICD-008 (matrix targeting), CICD-010 (workflow decomposition),
  CICD-011 (APS/repo/release drift checks in CI).
- **Release orchestration (`RELORCH`) — Unblocked 2026-05-11, 3/12.** Paused
  during the OPMODEL-012 / CICD-012 cutover window; resumes now with the main
  branch as the validated integration target. Phase 1 nucleus remaining:
  RELORCH-002 harness fixture matrix, RELORCH-010 closeout, RELORCH-011
  wire-up + decommission. Phase 2 (`prepare` / `promote` / `tag` / `monitor` /
  `verify`) follows. RELORCH-012 closes the release-record schema gap (yank
  state + policyDecisions enum) surfaced by the OPMODEL-011 rollback playbooks.

The next tagged release is still an **operational** release, not a
product-surface bundle. Its claim shifts from "the operating model is designed"
to "the operating model is executable end-to-end."

---

## Current Candidate: `v0.6.2-beta`

**Claim:** _The release operating model is executable end-to-end._ A selected
`main` SHA can be assessed, prepared, tagged, monitored, and verified through
deterministic commands and CI evidence rather than prose-only runbooks.

**Hard gates:**

- Main-first cutover complete with branch protection on `main`. **Met** —
  ruleset 16217152.
- Release-readiness workflow validates an exact `main` SHA without publishing
  credentials. **Met** — `.github/workflows/release-readiness.yml`.
- Rollback and incident playbooks executable. **Met** — OPMODEL-011.
- RELORCH command surface deterministic enough to replay against `v0.6.1-beta`
  from scratch. **Not yet met** — Phase 1/2 RELORCH work in progress.

**Release type:** beta patch/minor operational release. Do not market it as the
daemon-working product release; that remains the next window.

---

## Remaining Lanes

These lanes are all post-cutover and can run in parallel.

| Lane                                   | Owner                                                                                                                                                                                       | Status                 | Main outputs                                                                                                                                     |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| L1: Release commands — Phase 1 nucleus | [`RELORCH-002`](./plans/modules/release-orchestration.aps.md), [`RELORCH-010`](./plans/modules/release-orchestration.aps.md), [`RELORCH-011`](./plans/modules/release-orchestration.aps.md) | Unblocked              | Harness fixture matrix, closeout, wire-up + decommission.                                                                                        |
| L2: Release commands — Phase 2         | [`RELORCH-005..009`](./plans/modules/release-orchestration.aps.md)                                                                                                                          | After L1 harness lands | `scripts/release/{prepare,promote,tag,monitor,verify}.sh`.                                                                                       |
| L3: Release-record schema              | [`RELORCH-012`](./plans/modules/release-orchestration.aps.md)                                                                                                                               | Unblocked              | Yank state + policyDecisions enum; closes gap surfaced by OPMODEL-011 playbooks.                                                                 |
| L4: Integration SHA validation         | [`CICD-005`](./plans/modules/ci-cd-validation.aps.md)                                                                                                                                       | Unblocked              | Separate merged-SHA validation from PR feedback; distinct readiness contract for `main` pushes.                                                  |
| L5: Matrix targeting                   | [`CICD-008`](./plans/modules/ci-cd-validation.aps.md)                                                                                                                                       | Unblocked              | Reserve macOS/Windows/cross-compile/NAPI/bench matrices for changes that need platform evidence.                                                 |
| L6: Workflow decomposition             | [`CICD-010`](./plans/modules/ci-cd-validation.aps.md)                                                                                                                                       | Unblocked              | Map existing workflows onto PR / integration / assurance / candidate / publish contracts.                                                        |
| L7: APS/repo/release drift in CI       | [`CICD-011`](./plans/modules/ci-cd-validation.aps.md)                                                                                                                                       | Unblocked              | Warning-mode CI drift checks for missing APS references, inconsistent counts, stale validation metadata, release-note gaps, shipped-state drift. |

---

## What Can Be Done In Parallel Now

Start as separate worktrees branched from `main`:

| Track                      | First work item | Why safe in parallel                                                                                                            |
| -------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Release harness completion | `RELORCH-002`   | Fixture matrix grows existing harness; isolated from CICD lanes.                                                                |
| Release-record schema gap  | `RELORCH-012`   | Schema-only change; closes a known correctness gap.                                                                             |
| Matrix targeting           | `CICD-008`      | Touches workflow `if:` conditions in non-release jobs; pattern follows shipped CICD-004 / -006 / -007.                          |
| Workflow decomposition     | `CICD-010`      | Mostly `.github/workflows/README.md` and trigger consolidation; conflicts only with CICD-008 if both rewrite the same workflow. |
| Drift checks in CI         | `CICD-011`      | Extends already-shipped OPMODEL-010 drift script; touches `scripts/ci/` + a workflow job.                                       |

Avoid parallelising too early:

- RELORCH-005..-009 before RELORCH-002 lands the broader fixture matrix —
  command contracts should fail in CI from day one.
- CICD-010 workflow decomposition while CICD-008 matrix targeting is mid-edit on
  the same workflow — sequence them or carve up files explicitly.

---

## Suggested `v0.6.2-beta` Cut

| Pick                      | Include                                                                            | Exclude                                          |
| ------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------ |
| Minimum                   | RELORCH-002 harness fixture matrix; RELORCH-012 schema gap; CICD-011 drift checks. | RELORCH-005..-011; CICD-005/-008/-010.           |
| Strong                    | Minimum + RELORCH-005..-009 + RELORCH-010 closeout; CICD-008 matrix targeting.     | RELORCH-011 wire-up; CICD-005, CICD-010.         |
| Full operating-model beta | Strong + RELORCH-011 wire-up + decommission; CICD-005, CICD-010.                   | Daemon-working product slate (separate release). |

**Recommendation:** take the **Strong** cut. It gives operators a real release
loop end-to-end (assess → preflight → prepare → promote → tag → monitor →
verify) replayable against `v0.6.1-beta`, with matrix costs already targeted.
Save the wire-up step and the integration/decomposition work for a follow-on
release.

---

## Later: Daemon-Working Product Release

The daemon-working product slate remains valuable; it should not compete with
the remaining RELORCH/CICD work in the same release. Promote once the release
machinery is trustworthy end-to-end.

| Future slice                                 | Source                                                | Gate before promotion                                                                           |
| -------------------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Multi-Layer Protection v1                    | [`MLP`](./plans/modules/multilayer-protection.aps.md) | Release readiness can prove an exact `main` SHA replayably (RELORCH-009 against `v0.6.1-beta`). |
| Intercept Launcher v1                        | [`INTL`](./plans/modules/intercept-launcher.aps.md)   | Shared `AgentTag` schema coordinated with MLP-014.                                              |
| Enterprise / compliance / language expansion | Queued APS modules                                    | Promote only after daemon-working and release machinery are stable.                             |
