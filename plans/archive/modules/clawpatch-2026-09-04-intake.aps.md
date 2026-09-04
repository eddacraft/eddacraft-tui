<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Intake-only module; it does not authorise implementation of filed issues. -->

# Clawpatch 4 September Intake

| ID     | Owner | Priority | Status | Progress |
| ------ | ----- | -------- | ------ | -------- |
| CLAW04 | —     | P2       | Done   | 1/1      |

**Completed:** 2026-09-04 — eight selected records from run
`20260904T074935-2c02a1` were calibrated against current source, five coherent
issues were filed as
[#4387](https://github.com/eddacraft/anvil-001/issues/4387)–[#4391](https://github.com/eddacraft/anvil-001/issues/4391),
and the two remaining 2 September open records stay with
[#4342](https://github.com/eddacraft/anvil-001/issues/4342).

## Purpose

Provide APS authority for the bounded triage-and-filing workflow requested for
Clawpatch run `20260904T074935-2c02a1`, including inventory of the intervening
runs `20260831T052858-acdf65` and `20260902T115230-44b5da`, without granting
implementation authority or writing to the shared CIB module.

## Source truth

- [Intake triage evidence](../../../docs/reviews/2026-09-04-clawpatch-latest-run.md)
- Clawpatch persisted finding, run, and report stores
- Current source and tests at `99a49b5975bd8e7b5a535365c28e4b18b7bc7d16`
- [Issue triage and APS authority](../../specs/2026-05-28-issue-triage-and-aps-authority.md)
- Private work-item claim [#4386](https://github.com/eddacraft/anvil-001/issues/4386)

## In scope

- Inventory the complete persisted store and confirm no live review lock.
- Calibrate the selected findings and explicit report companions against
  current source, existing plans, and GitHub tracking.
- Refresh the selection when newer completed runs appear before publication.
- Persist evidence-based Clawpatch dispositions.
- File coherent issues with explicit priority, readiness, and authority lanes.
- Publish the evidence and intake receipt through a docs-only PR.

## Out of scope

- Implementing or designing the filed repairs.
- Extending the 2026-08-28 CLAWOPEN source set.
- Editing the shared CIB module.
- Release, deployment, or administrator-policy claims.

## Work Items

### CLAW04-001: Triage and file the latest Clawpatch run

- **Status:** Done 2026-09-04; eight 4 September receipts persisted, five
  finding issues read back, claim #4386 recorded for the publishing PR, and
  #4342 confirmed as the 2 September Nx owner.
- **Intent:** Every selected allegation has a current-source disposition and
  every actionable cluster has exactly one visible owner.
- **Expected Outcome:** The full store is inventoried; the selected records are
  deduplicated and calibrated; fixed or accepted-boundary work is closed;
  actionable records point to issues with explicit readiness and APS boundaries;
  the review is published without claiming implementation.
- **Files:** `docs/reviews/2026-09-04-clawpatch-latest-run.md`, this module,
  `plans/index.aps.md`
- **Validation:** `clawpatch status --json`; read back the eight finding
  receipts; `gh issue view 4386`; `gh issue view 4387`; `gh issue view 4388`;
  `gh issue view 4389`; `gh issue view 4390`; `gh issue view 4391`;
  `gh issue view 4342`; `pnpm docs:check`; `pnpm aps:active-lint`;
  `pnpm aps:index:check`
- **Risk:** low
- **Decision:** done — implementation authority remains with each issue's
  labelled lane and any later APS promotion.
