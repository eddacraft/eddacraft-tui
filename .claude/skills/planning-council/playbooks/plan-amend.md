# Planning Council: Plan Amend

Use when implementation, review, CI, or repo reality shows that an APS plan is
stale or incomplete.

## Inputs

- existing APS item or module
- evidence that triggered the amendment
- changed files, failed checks, review findings, or release/recovery context
- related ADRs, specs, and governance docs

## Steps

1. Identify what changed: scope, dependency, validation, sequencing, or authority.
2. Decide whether to amend the current item, split it, add a follow-up, or block
   execution.
3. Update the APS module and `plans/index.aps.md` when counts or status change.
4. Add ADR/spec/runbook follow-ups only when durable decisions or operational
   procedures changed.
5. Record validation evidence or the remaining evidence gap.

## Decision

Return `proceed` only after APS reflects the new reality. Otherwise return
`amend`, `split`, `replan`, or `block` with the exact files to update.
