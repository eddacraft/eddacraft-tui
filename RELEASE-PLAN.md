# Anvil Release Plan

**Last updated:** 2026-05-12 (operational substrate complete: OPMODEL +
RELORCH + CICD all 12/12. `v0.6.2-beta` is the next operational tag.)

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
- **CI/CD readiness (`CICD`) — Complete, 12/12.** Cost reporting (CICD-001),
  path/risk classifier (CICD-002), local validation commands (CICD-003), fast PR
  validation (CICD-004), integration SHA validation split (CICD-005), coverage
  cost controls (CICD-006), security/dependency targeting (CICD-007), matrix
  targeting (CICD-008), release-readiness workflow (CICD-009), workflow contract
  map + authority audit (CICD-010), APS/repo/release drift checks in CI
  (CICD-011), and cutover readiness (CICD-012) all shipped. Council follow-ups
  closed via PR #1442 (issue #1438).
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

| Lane                        | Owner module                                                                                                                                               | Can run now?                                  | Main outputs                                                                         | Blocks                                   |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------- |
| L3: Release CI/CD readiness | [`OPMODEL-005`](./plans/archive/modules/operating-model-migration.aps.md#opmodel-005-release-readiness-and-candidate-artefact-workflow-design), RELORCH/CI | Yes, after L1 contract shape is stable enough | `.github/workflows/release-readiness.yml`, candidate metadata, exact-SHA validation. | L6, release tags that claim CI readiness |
| L5: Release commands        | [`RELORCH-001..012`](./plans/archive/modules/release-orchestration.aps.md)                                                                                 | Complete                                      | `scripts/release/{assess,preflight,prepare,promote,tag,monitor,verify,closeout}.sh`. | Full target `/release` mode              |
| L7: Main-first cutover      | [`OPMODEL-012`](./plans/archive/modules/operating-model-migration.aps.md#opmodel-012-main-first-cutover-and-dev-retirement)                                | Complete                                      | `main` protected as the integration branch; `dev` retired.                           | —                                        |

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

| Work                           | Parallel? | Notes                                                                                           |
| ------------------------------ | --------- | ----------------------------------------------------------------------------------------------- |
| `OPMODEL-010` drift checks     | Complete  | Warning-mode fixtures shipped.                                                                  |
| `CICD-011` drift checks in CI  | Complete  | Drift script wired into `ci.yml` `aps-drift` (warning-mode, PR-metadata aware); fixture locked. |
| Release-readiness workflow     | Complete  | Exact-SHA validation and candidate metadata shipped.                                            |
| `RELORCH-002` harness          | Complete  | Harness encodes the command JSON and exit-code schema.                                          |
| `RELORCH-003` assess           | Complete  | Assessment command shipped.                                                                     |
| `RELORCH-004` preflight        | Complete  | Preflight command shipped.                                                                      |
| `RELORCH-010` closeout dry-run | Complete  | Closeout command shipped.                                                                       |

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

## `v0.6.2-beta` Cut: Ready to Tag

All operational-substrate work scoped to this release has merged. The cut is the
**Full operating-model beta** that earlier revisions of this plan described as
the target — every CICD targeting item, every RELORCH command, and every OPMODEL
cutover task is now on `main`.

| Component                             | Status                                                                                                                           |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| OPMODEL main-first cutover            | Shipped — `b6f236e9` (2026-05-11), `main` ruleset 16217152 active.                                                               |
| RELORCH deterministic command surface | Shipped — `scripts/release/{assess,preflight,prepare,promote,tag,monitor,verify,closeout}.sh` + harness + skill/runbook wire-up. |
| CICD integration SHA validation split | Shipped — CICD-005, push contract distinct from PR commentary.                                                                   |
| CICD platform-matrix targeting        | Shipped — CICD-008, `rust.yml` cross-compile gated; saves ~6k-9k billed runner-min/mo.                                           |
| CICD workflow contract map            | Shipped — CICD-010, `.github/workflows/README.md` + fixture-enforced.                                                            |
| CICD APS/repo/release drift in CI     | Shipped — CICD-011 + PR-metadata extension + `::warning::` surfacing.                                                            |

**Next operator action:** run `bash scripts/release/assess.sh` against the
current `main` SHA to confirm replayability. The RELORCH-009 verify gate (the
last hard gate in the "Hard gates" list above) is satisfied by a successful
end-to-end replay against `v0.6.1-beta`; once `v0.6.2-beta` ships, the same
machinery is the replay target for any subsequent operational release.

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
