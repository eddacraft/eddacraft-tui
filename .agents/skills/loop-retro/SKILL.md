---
name: loop-retro
description: >-
  Audit the development workflow itself — not the code it produced — across
  the five loop dimensions, using run checkpoints, the loop journal, lessons,
  and evidence artefacts already emitted by dev-loop. Use after a drain run,
  when the Ready backlog is empty, on a recurring cadence, or when loop
  behaviour feels off (repeated repairs, stale lessons, skipped gates).
  Report-only: findings become Proposed items or escalation-queue entries,
  never direct fixes.
---

# Loop retrospective

**Audit the loop, not the code.** Code review and verification already exist
(`verify-loop`, `council`). This skill asks whether the _workflow that
produced the code_ is understood, controlled, validated, delivered, and
learning — and it answers only from artefacts.

## When not to run

- Verifying a specific change or claim → `verify-loop`.
- Executor self-check before a success claim → `evidence-gate`.
- Dependency or code-quality hygiene → `security-and-quality`.
- Mid-item: never interrupt an in-flight `dev-loop` item for a retro.

## Evidence domains

Collect from three domains. Analyse each on its own before synthesis; keep
every finding traceable to a named artefact.

1. **Run evidence** — `.dev-loop/checkpoints/*.json`, emitted evidence
   blocks, verifier evidence bundles, `plans/execution/loop-journal.md`,
   `plans/execution/lessons/`, escalation-queue entries.
2. **Project evidence** — `dev-loop.policy.yaml` (and variants), APS plans
   and statuses, repository gates (tests, linters, hooks, CI config),
   `.dev-loop/capability-manifest.json`.
3. **Harness evidence** — installed dev-loop bundle and sibling skills,
   active harness adapters, project instruction files (`AGENTS.md`,
   `CLAUDE.md`).

A domain with no artefacts is recorded as **unobserved**. Unobserved never
becomes a score, a pass, or a fail.

## Dimensions

Assess each dimension against the artefacts that should witness it:

| Dimension            | Question                                              | Witnessed by                                                            |
| -------------------- | ----------------------------------------------------- | ----------------------------------------------------------------------- |
| Task understanding   | Did work start from bounded, testable intent?         | ReadyItems, grill-design records, acceptance criteria                   |
| Controlled execution | Did work stay on claimed, isolated, repeatable paths? | Claims, worktree records, run checkpoints, policy file                  |
| Change validation    | Did evidence gate every success claim?                | Evidence blocks, verifier bundles, gate freshness                       |
| Reliable delivery    | Did speed bypass review, CI, or integration proof?    | Land outcomes, ancestor checks, PR review handling, out-of-band merges  |
| Learning capture     | Did later tasks benefit from earlier ones?            | Journal entries, lessons, and whether lessons carry comparable measures |

## Loop

1. **Bound the window.** Name the runs, date range, or module under review.
   No unbounded repository archaeology.
2. **Collect.** Gather the three evidence domains for the window. Record
   gaps explicitly as you go.
3. **Assess each dimension** using only collected artefacts. For every
   weakness, cite the artefact (or the absence) that shows it.
4. **Check improvement claims.** For each lesson or loop change in the
   window that claims effect: passing a current check proves the
   intervention was exercised; **only a comparable later result proves the
   loop improved**. Reclassify unproven claims as `unmeasured`.
5. **Write findings.** Each finding names: dimension, severity, artefact
   evidence, the loop element implicated (policy, stage, gate, skill,
   habit), a scoped proposed repair, and an acceptance measure a future
   retro could check.
6. **File, never fix.** Route findings as `Proposed` APS items within
   approved scope, or park them in `aps-escalation-queue`. This skill has
   no authority to change policy, plans, gates, or skills.

## Hard rules

1. No numeric or graded score without named artefact evidence; prefer
   `strong | adequate | weak | unobserved` per dimension.
2. Never collapse absence of evidence into evidence of absence — an
   unobserved dimension is a visibility finding, not a failure.
3. One retro produces one report; do not iterate the loop mid-retro.
4. Report-only. Repairs travel through planning, not through this skill.

## Report

```markdown
## Loop retrospective

- Window:
- Runs reviewed:
- Dimensions:
  - Task understanding: strong | adequate | weak | unobserved — evidence
  - Controlled execution: … — evidence
  - Change validation: … — evidence
  - Reliable delivery: … — evidence
  - Learning capture: … — evidence
- Evidence gaps:
- Improvement claims checked: proven | unmeasured (per claim)
- Findings: (dimension, severity, evidence, proposed repair, acceptance measure)
- Filed as: Proposed items | escalation-queue entries | none
```

## Exit

```markdown
## Exit

- Decision: report-complete | insufficient-evidence
- Next: plan-ready | aps-escalation-queue | stop
- Notes:
```

## Non-goals

- Not code or change verification (`verify-loop`).
- Not an executor evidence gate (`evidence-gate`).
- Not dependency or code hygiene (`security-and-quality`).
- Not authorised to edit policy, plans, skills, or gates.
