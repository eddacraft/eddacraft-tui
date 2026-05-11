# Anvil Release Plan

**Last updated:** 2026-05-11 (post-OPMODEL-012 cutover; current delivery:
RELORCH complete + remaining CICD)

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
- **Release orchestration (`RELORCH`) — Complete, 12/12, archived.** The
  deterministic command surface now covers assess, preflight, prepare, promote,
  tag, monitor, verify, closeout, the release-command harness, release-record
  yank/discard schema closure, and skill/runbook wire-up with the legacy runner
  removed. Live CI readiness authority remains tracked under CICD.

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
  from scratch. **Met** — command surface complete and archived.

**Release type:** beta patch/minor operational release. Do not market it as the
daemon-working product release; that remains the next window.

---

## Remaining Lanes

These lanes are all post-cutover and can run in parallel.

| Lane                        | Owner module                                                                                                                                       | Can run now?                                  | Main outputs                                                                         | Blocks                                   |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------- |
| L3: Release CI/CD readiness | [`OPMODEL-005`](./plans/modules/operating-model-migration.aps.md#opmodel-005-release-readiness-and-candidate-artefact-workflow-design), RELORCH/CI | Yes, after L1 contract shape is stable enough | `.github/workflows/release-readiness.yml`, candidate metadata, exact-SHA validation. | L6, release tags that claim CI readiness |
| L5: Release commands        | [`RELORCH-001..012`](./plans/archive/modules/release-orchestration.aps.md)                                                                         | Complete                                      | `scripts/release/{assess,preflight,prepare,promote,tag,monitor,verify,closeout}.sh`. | Full target `/release` mode              |
| L7: Main-first cutover      | [`OPMODEL-012`](./plans/archive/modules/operating-model-migration.aps.md#opmodel-012-main-first-cutover-and-dev-retirement)                        | Complete                                      | `main` protected as the integration branch; `dev` retired.                           | —                                        |

---

## Parallel Delivery Shape

### Wave 0: Lock Contracts

Run these first. They unblock parallel execution without requiring branch
cutover.

| Work                            | Parallel? | Notes                                                               |
| ------------------------------- | --------- | ------------------------------------------------------------------- |
| `OPMODEL-007` guidance contract | Complete  | Defines the path/risk/check JSON consumed by CI, hooks, and agents. |
| `OPMODEL-008` review tiers      | Complete  | Council/review entrypoints aligned to one tier model.               |
| `RELORCH-001` command design    | Complete  | Command contract complete and archived.                             |

### Wave 1: Build Advisory Controls

Once Wave 0 contracts exist, split into independent implementation lanes.

| Work                           | Parallel? | Notes                                                          |
| ------------------------------ | --------- | -------------------------------------------------------------- |
| `OPMODEL-010` drift checks     | Complete  | Warning-mode fixtures shipped; CICD-011 tracks CI enforcement. |
| Release-readiness workflow     | Complete  | Exact-SHA validation and candidate metadata shipped.           |
| `RELORCH-002` harness          | Complete  | Harness encodes the command JSON and exit-code schema.         |
| `RELORCH-003` assess           | Complete  | Assessment command shipped.                                    |
| `RELORCH-004` preflight        | Complete  | Preflight command shipped.                                     |
| `RELORCH-010` closeout dry-run | Complete  | Closeout command shipped.                                      |

### Wave 2: Prove Failure Handling

This wave turns advisory controls into something operators can trust.

| Work                             | Parallel? | Notes                                                                       |
| -------------------------------- | --------- | --------------------------------------------------------------------------- |
| `OPMODEL-011` rollback playbooks | Complete  | Named failure classes and runbooks shipped.                                 |
| `RELORCH-005` prepare            | Complete  | Prepare command shipped.                                                    |
| `RELORCH-006` promote            | Complete  | Promote command shipped for direct and stabilisation strategies.            |
| `RELORCH-008` monitor            | Complete  | Monitor command shipped with explicit run evidence and polling modes.       |
| `RELORCH-009` verify             | Complete  | Verify command shipped with blocked live checks until evidence is supplied. |

### Wave 3: Cutover Decision

Do not start until Waves 1 and 2 have produced usable evidence.

| Work                               | Parallel? | Notes                                                                        |
| ---------------------------------- | --------- | ---------------------------------------------------------------------------- |
| `RELORCH-007` tag                  | Complete  | Tagging remains irreversible and is guarded by exact-SHA readiness evidence. |
| `RELORCH-011` wire-up/decommission | Complete  | Legacy runner removed; skill/runbook wired to the command surface.           |
| `OPMODEL-012` main-first cutover   | Complete  | `main` is the protected integration branch; `dev` retired.                   |

---

## What Can Be Done In Parallel Now

Start as separate worktrees branched from `main`:

| Track                  | First work item | Why safe in parallel                                                                                                            |
| ---------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Matrix targeting       | `CICD-008`      | Touches workflow `if:` conditions in non-release jobs; pattern follows shipped CICD-004 / -006 / -007.                          |
| Workflow decomposition | `CICD-010`      | Mostly `.github/workflows/README.md` and trigger consolidation; conflicts only with CICD-008 if both rewrite the same workflow. |
| Drift checks in CI     | `CICD-011`      | Extends already-shipped OPMODEL-010 drift script; touches `scripts/ci/` + a workflow job.                                       |
| Integration SHA split  | `CICD-005`      | Separates merged-SHA release authority from PR feedback after main-first cutover.                                               |

Avoid parallelising too early:

- CICD-010 workflow decomposition while CICD-008 matrix targeting is mid-edit on
  the same workflow — sequence them or carve up files explicitly.

---

## Suggested `v0.6.2-beta` Cut

| Pick                      | Include                                                                  | Exclude                                          |
| ------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------ |
| Minimum                   | RELORCH command surface plus CICD-011 drift checks.                      | CICD-005/-008/-010.                              |
| Strong                    | Minimum + CICD-008 matrix targeting.                                     | CICD-005, CICD-010.                              |
| Full operating-model beta | Strong + CICD-005 integration SHA split + CICD-010 workflow composition. | Daemon-working product slate (separate release). |

**Recommendation:** take the **Minimum** cut once PR #1433 lands and drift
checks are wired into CI. It gives operators a replayable command-driven release
loop while keeping broader CI architecture changes in follow-on work.

---

## Later: Daemon-Working Product Release

The daemon-working product slate remains valuable; it should not compete with
the remaining CICD work in the same release. Promote once the release machinery
is trustworthy end-to-end.

| Future slice                                 | Source                                                | Gate before promotion                                                                           |
| -------------------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Multi-Layer Protection v1                    | [`MLP`](./plans/modules/multilayer-protection.aps.md) | Release readiness can prove an exact `main` SHA replayably (RELORCH-009 against `v0.6.1-beta`). |
| Intercept Launcher v1                        | [`INTL`](./plans/modules/intercept-launcher.aps.md)   | Shared `AgentTag` schema coordinated with MLP-014.                                              |
| Enterprise / compliance / language expansion | Queued APS modules                                    | Promote only after daemon-working and release machinery are stable.                             |
