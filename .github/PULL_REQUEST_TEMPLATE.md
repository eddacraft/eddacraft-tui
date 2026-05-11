## Summary

<!-- 1-3 bullet points: what changed and why -->

-

<!--
Target base reminder (CICD-012, OPMODEL-012):

- Migration mode (today): normal `feat/*` / `fix/*` / `docs/*` /
  `chore/*` PRs target `dev`. PRs to `main` are release sync
  (`dev`) or release/hotfix (`release/*` / `hotfix/*`) only — the
  `PR Base Guard` workflow enforces this.
- Target mode (after `OPMODEL-012`): normal PRs target `main`. The
  release gate fires on `release/*` / `hotfix/*` heads, not on every
  PR. See `docs/guides/branching-strategy.md` for the full table.

Check that this PR's base branch matches the operating mode in
effect right now.
-->

## APS + GH execution context

APS Module: <!-- e.g. INTD, RTAI, TRACE --> APS Task(s):

<!-- e.g. INTD-013, TRACE-001 --> GH Project Status:
<!-- Backlog | Ready | In Progress | In Review | Blocked | Done -->

Acceptance criteria checked:

- [ ] Criteria met
- [ ] Follow-up required

## Test plan

<!--
How was this tested? Check all that apply.

For "Manual testing", link to the APS work item `Validation:` field or a
verification issue where the steps live. Ephemeral PR checklists rot; put the
steps somewhere durable.
-->

- [ ] Unit tests added/updated
- [ ] Manual testing (link to durable steps — see comment above)
- [ ] CI passes (lint, typecheck, test)
- [ ] N/A (docs/config only)
