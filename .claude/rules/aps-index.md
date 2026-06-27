# APS Planning Index

`plans/index.aps.md` is the single source of truth for all module statuses and
progress counts. Read it before starting any implementation work.

Full spec rules are in `plans/aps-rules.md` — read before writing or modifying
any `.aps.md` file.

## Rules that agents keep forgetting

- Do NOT create separate module lists or summary files — `index.aps.md` is the
  only index
- Before starting work on a module, mark its status **In Progress**
- After completing a work item, update its status only — do **not** bump the
  module header or index `N/M` count in feature PRs (ADR-053 advisory counts)
- After all items done, update module status to **Done**
- Reconcile stored `N/M` counts with `pnpm aps:index` when a refresh is needed
- Archive completed modules with `git mv` to `plans/archive/modules/`
