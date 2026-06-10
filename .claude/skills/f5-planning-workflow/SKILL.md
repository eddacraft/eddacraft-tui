---
name: f5-planning-workflow
description: >-
  Fable 5-tuned planning orchestrator. Turns goals into validated, ready APS
  work with minimal ceremony — acts when enough is known, recommends instead
  of surveying, and treats the plan as a living artifact it may evolve.
  Use when running on Claude Fable 5 and work needs planning, replanning, or
  a readiness decision. Companion to f5-dev-workflow and f5-aps-loop;
  prior-model projects use planning-workflow instead.
---

# Fable 5 Planning Workflow

Turn intent into a validated, ready work item — or an honest decision that
the work is blocked or out of scope. Same handoff contract as
`planning-workflow`; leaner path to it. This workflow never writes code and
never creates branches.

## Operating stance

When you have enough information to plan, plan. Do not re-derive facts
already established in the conversation, re-litigate decisions the user has
already made, or survey options you will not pursue. If approaches genuinely
compete, give a recommendation with the deciding trade-off, not a catalogue.
Ask the user one question only when the next planning step would otherwise be
guesswork on something only they can answer — product intent, risk appetite,
scope boundaries. Everything else, resolve from project truth and record the
assumption.

## Path

1. **Ground in project truth.** Current source, tests, ADRs, release state,
   and feature flags outrank stale plan prose. Where the plan and the code
   disagree, the code is the fact and the plan carries the intent — preserve
   intent, correct facts.
2. **Match the goal to work.** One existing item, an update, a split, a new
   item, or out of scope. In APS projects, `aps-planning` is the engine for
   truth validation and item drafting; follow `plans/aps-rules.md` — items
   carry intent, outcome, and validation, never implementation steps.
3. **Design gate — only when genuinely contested.** Invoke `brainstorming`
   or `planning-council` when the approach changes behaviour, architecture,
   security posture, or ownership AND more than one credible approach exists.
   A clear-cut change with an obvious shape does not need a design ceremony;
   note the chosen shape in the work item and move on.
4. **Write the plan.** `writing-plans` for implementation plans;
   `aps-planning` for module and work item text. Lean actions: observable
   checkpoints with validation commands, not tutorials.
5. **Validate readiness.** A work item is ready when its outcome is testable,
   its validation command exists, and its dependencies are closed or
   explicitly documented. Run the APS truth validation when APS is present.

## Plan evolution

Plans are living artifacts. When invoked from `f5-aps-loop` or any
unattended run, apply plan changes within the loop's plan-evolution authority
(statuses, drift corrections, new Proposed items, action plans) directly and
record them; reserve user checkpoints for changes to agreed scope, success
criteria, or accepted decisions. When working interactively, show proposed
plan text before writing it.

## Handoff

Return the same block `planning-workflow` defines, so downstream skills
interoperate:

```markdown
## Planning Workflow Handoff

- Goal:
- Work item:
- Status:
- Design source:
- Dependencies:
- Files:
- Validation:
- Risks:
- Decision: ready-for-dev | needs-design | needs-plan-update | blocked | out-of-scope
- Next skill: f5-dev-workflow | brainstorming | aps-planning | writing-plans | planning-council
```

`ready-for-dev` hands to `f5-dev-workflow` (or back to the loop). Ground
the Decision in evidence you can point to — a validation command that exists,
a dependency item's actual status — not on plan prose alone.

## Cross-references

- `aps-planning` — APS truth validation, reconciliation, item drafting
- `writing-plans` — implementation plans with exact paths and commands
- `brainstorming` / `planning-council` — contested design decisions
- `f5-aps-loop` — the autonomous loop that calls this for replanning
- Tuning rationale: `docs/fable5-agent-tuning.md` (eddacraft-skills catalogue)
