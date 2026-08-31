<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Intake-only module; it does not authorise implementation of filed issues. -->

# Clawpatch 30–31 August Intake

| ID     | Owner | Priority | Status | Progress |
| ------ | ----- | -------- | ------ | -------- |
| CLAW30 | —     | P2       | Done   | 1/1      |

**Completed:** 2026-08-31 — thirteen selected records were calibrated against
current source, seven coherent issues were filed as
[#4230](https://github.com/eddacraft/anvil-001/issues/4230)–[#4233](https://github.com/eddacraft/anvil-001/issues/4233) and
[#4280](https://github.com/eddacraft/anvil-001/issues/4280)–[#4282](https://github.com/eddacraft/anvil-001/issues/4282), eleven receipts remain
open with owners, one is fixed, and one is `wont-fix` at a structural boundary.

## Purpose

Provide APS authority for the bounded triage-and-filing workflow requested for
Clawpatch runs `20260830T054354-0927e9`, `20260830T165831-0f8d78`, and
`20260831T032953-703e11`, without granting implementation authority or writing
to the shared CIB module.

## Source truth

- [Intake triage evidence](../../../docs/reviews/2026-08-30-clawpatch-latest-run.md)
- Clawpatch persisted finding, run, and report stores
- Current source and tests at `80a2da1106a4b8813c1c9c36d433f43ea1f7d844`
- [Issue triage and APS authority](../../specs/2026-05-28-issue-triage-and-aps-authority.md)
- Private work-item claim [#4279](https://github.com/eddacraft/anvil-001/issues/4279)

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

### CLAW30-001: Triage and file the latest Clawpatch run

- **Status:** Done 2026-08-31; thirteen receipts persisted, seven finding issues
  read back, and claim #4279 recorded for the publishing PR.
- **Intent:** Every selected allegation has a current-source disposition and
  every actionable cluster has exactly one visible owner.
- **Expected Outcome:** The full store is inventoried; the selected records are
  deduplicated and calibrated; fixed or accepted-boundary work is closed;
  actionable records point to issues with explicit readiness and APS boundaries;
  the review is published without claiming implementation.
- **Files:** `docs/reviews/2026-08-30-clawpatch-latest-run.md`, this module,
  `plans/index.aps.md`
- **Validation:** `clawpatch status --json`; read back the thirteen finding
  receipts; `gh issue view 4230`; `gh issue view 4231`; `gh issue view 4232`;
  `gh issue view 4233`; `gh issue view 4279`; `gh issue view 4280`;
  `gh issue view 4281`; `gh issue view 4282`; `pnpm docs:check`;
  `pnpm aps:active-lint`; `pnpm aps:index:check`
- **Risk:** low
- **Decision:** done — implementation authority remains with each issue's
  labelled lane and any later APS promotion.
