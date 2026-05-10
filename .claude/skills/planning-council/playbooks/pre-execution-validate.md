# Planning Council: Pre-Execution Validate

Use before starting non-trivial Ready work.

## Inputs

- APS work item and module status
- current base branch and worktree status
- changed files since the plan was written, if known
- relevant specs, ADRs, runbooks, and source references

## Checks

- The item is still Ready or explicitly authorised for this slice.
- Dependencies and `Blocks on:` / `Coordinates with:` callouts are still true.
- Repo reality still matches the plan.
- The intended branch target follows current migration rules.
- Required validation commands are available and proportionate.
- As-built docs, runbooks, or public docs are identified when affected.

## Decision

Return one of:

- `proceed`: start work.
- `amend`: update APS or supporting docs before work.
- `split`: divide the item into smaller PRs.
- `replan`: direction changed enough to revisit design.
- `block`: do not start until the blocker is resolved.

If the decision is not `proceed`, stop implementation and update APS first.
