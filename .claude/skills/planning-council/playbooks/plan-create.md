# Planning Council: Plan Create

Use when creating a new APS module, cross-cutting plan, or multi-item execution
slice.

## Inputs

- user goal and constraints
- relevant specs, ADRs, and scope guard documents
- existing `plans/index.aps.md` and related modules
- expected validation commands

## Steps

1. Confirm the work belongs in APS and identify the owning module.
2. Check existing decisions before proposing architecture or boundary changes.
3. Split work into small APS items with Intent, Expected Outcome, Validation,
   Files, and coordination callouts.
4. Identify dependencies, parallel waves, and release/readiness implications.
5. Draft the APS edits and any required ADR/spec follow-ups for normal APS
   review and closeout.

## Decision

Return `proceed` when the plan can move toward Proposed/Ready. Return `replan`
or `block` when scope, ownership, or authority is unclear.
