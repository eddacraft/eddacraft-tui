# APS Planning Index

`plans/index.aps.md` is the single source of truth for all module statuses and
progress counts. Read it before starting any implementation work.

Full spec rules are in `plans/aps-rules.md` — read before writing or modifying
any `.aps.md` file.

## Rules that agents keep forgetting

- Do NOT create separate module lists or summary files — `index.aps.md` is the
  only index
- **Shared multi-writer modules (CIB today):** feature PRs do **not** edit the
  module `.aps.md` — no status, intake, or promotion. Reconcile on a
  bookkeeping branch only (`plans/project-context.md#keeping-plans-current`)
- **Exclusive modules:** before starting work, mark the work item **In Progress**
  where helpful; after completing it, update its status only — do **not** bump
  the module header or index `N/M` count in feature PRs (ADR-053)
- When starting a work item, search for an existing private GitHub issue or PR
  for that APS ID; if none, create one, assign it, and put `Fixes #N` on the PR.
  Grain is the work item, not the module. Do not create issues at APS-item
  creation. See `plans/project-context.md#work-item-claim-issues`.
- After all items done on an exclusive module, update module status to **Done**
- Reconcile stored `N/M` counts with `pnpm aps:index` when a refresh is needed
- Archive completed modules with `git mv` to `plans/archive/modules/`
