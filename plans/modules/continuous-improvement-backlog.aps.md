<!--
APS Module: Continuous Improvement Backlog
==========================================
Standing intake for concrete improvement work identified across the project.
This module intentionally remains active while the project is active.
-->

# Continuous Improvement Backlog

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| CIB | —     | In Progress | 1/2      |

## Purpose

Capture concrete improvement opportunities identified anywhere in the project so
they are not lost between feature modules, reviews, releases, incidents, and
documentation closeout.

## Standing Module Policy

This is a standing APS module. It does not close merely because all currently
listed items are done. Keep it active while the project is active, append new
items as they are identified, and only archive it if a future APS decision
explicitly replaces the intake model.

Progress stays numeric for APS drift tooling. Use `0/0` while no items are
listed, then update it to `done/total` as `CIB-NNN` items are added.

## In Scope

- Cross-project cleanup and quality improvements that do not fit a more specific
  active module
- Follow-up work discovered during reviews, releases, debugging, docs closeout,
  or routine implementation
- Small operational, developer-experience, test, documentation, and maintenance
  improvements with clear expected outcomes
- Candidate work that needs triage before promotion into a specialist module

## Out of Scope

- Vague ideas without an observable outcome
- Product features large enough to need their own APS module
- Work already owned by a specific active module
- Duplicating issue trackers or PR review threads without distilling an
  executable APS item

## Intake Rules

Add a new `CIB-NNN` item when an improvement has:

- A one-sentence intent
- An expected outcome or observable acceptance condition
- A validation command, manual check, or explicit reason validation is not yet
  known
- Best-effort source context, such as the review, release, incident, file path,
  or module where it was identified

When a cluster becomes large or domain-specific, promote it into a dedicated APS
module and leave a short `Superseded by:` note on the original CIB item.

## Cross-Cutting Convention

This is a cross-cutting APS module and follows the rules in
[`plans/aps-rules.md#cross-cutting-modules`](../aps-rules.md#cross-cutting-modules).
Task closeout must sweep `Coordinates with:`, `Blocks on:`, `Supersedes:`, and
`Superseded by:` callouts rather than carrying unresolved references into
archive.

## Item Template

```markdown
### CIB-NNN: Short outcome-focused title

- **Status:** Draft
- **Intent:** One sentence describing the improvement outcome.
- **Expected Outcome:** Observable result or acceptance condition.
- **Validation:** `command` or manual check.
- **Identified From:** Review, release, incident, module, or file path.
- **Confidence:** low | medium | high
```

## Tasks

### CIB-001: Sweep global `dev-workflow` skill for post-cutover and current-council drift

- **Status:** Complete
- **Intent:** Bring the global `dev-workflow` routing skill into alignment with
  the main-first cutover and the current risk-tiered council architecture.
- **Expected Outcome:** `~/Projects/src/code-env/.claude/skills/dev-workflow/SKILL.md`
  no longer instructs branching from `dev`, the Stage Map references the current
  council and skill set (risk-tiered `council`, `local-review-council` for the
  streaming flow, `planning-council`), and adjacent stages
  (`addressing-pr-reviews`, `finishing-a-branch`, `release`) are linked where
  relevant. Skill is consistent with `AGENTS.md` and current APS lifecycle.
- **Validation:** Manual diff of the updated skill against the main-first
  cutover artefacts (`docs/guides/branching-strategy.md`,
  `docs/guides/worktree-policy.md`) and the current council skill; `pnpm
  format:check` if any in-repo doc is touched alongside.
- **Identified From:** Session review 2026-05-11 — OPMODEL-012 archive closed
  without sweeping `dev-workflow`; skill still says "Branch from `dev`" at line
  32 and points "Review" only at the legacy `code-review` skill plus `/council`
  command despite the newer streaming/batch council model.
- **Coordinates with:** DOCGOV-008 (stale entrypoints), CIB-002 (canonical skill
  list), `plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md`
  (skill authority boundaries).
- **Evidence:** Anvil PR #1443 (vendored repo-local copy at
  `.claude/skills/dev-workflow/SKILL.md`, merged 2026-05-11); follow-up review
  fixes in commit `ce4091cf` aligned the skill to the repo-local `quick|mini|full`
  council tiers and added a Surface Inventory section. Companion code-env PR
  `joshuaboys/code-env#20` covers the upstream global skill — open at closeout
  time; tracked separately.
- **Confidence:** high

### CIB-002: Establish definitive skill and agent list for the anvil repo

- **Status:** In Progress
- **Intent:** Produce a single authoritative inventory of the skills and agents
  this repository expects to be available, distinguishing repo-local from global
  surfaces and recording authority and source for each entry.
- **Expected Outcome:** A checked-in list (location decided during triage —
  candidates: `docs/guides/agent-surface-inventory.md` or a section inside
  `AGENTS.md`) names every skill and agent the anvil workflow depends on, marks
  repo-local versus global, identifies the canonical source for each global
  entry (e.g. `joshuaboys/code-env`), and is linked from `AGENTS.md`. Drift
  between this list and `.claude/` plus external skill repos is detectable by a
  documented manual check until automated validation is added.
- **Validation:** Manual inventory cross-check against `.claude/agents/`,
  `.claude/skills/` (where present), the global Claude skill directory, and
  current `AGENTS.md` references; `pnpm format:check` for any in-repo docs
  touched.
- **Identified From:** Session review 2026-05-11 — repeated drift between
  expected skills (e.g. `dev-workflow`, `council`, `release`) and what is
  current or correct, with no single source of truth available to detect it.
- **Coordinates with:** CIB-001 (drift sweep informs entries), DOCGOV-002
  (taxonomy and metadata),
  `plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md`
  (Phase 1: Inventory And Declare Authority).
- **Confidence:** medium
