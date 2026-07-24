# anvil

**Shared conventions:** `@AGENTS.md`  
**Repo map:** `@CONTEXT.md`  
**Skills / agents / commands:** `@docs/guides/agent-surface-inventory.md`

This file is the **Claude Code adapter only**. Do not restate shared workflow,
validation commands, architecture, or skill procedures here — those live in
`AGENTS.md`, `CONTEXT.md`, and the inventory.

## Claude-only

- **Hooks and event wiring** live in **user** settings
  (`~/.claude/settings.json`). Scripts under `.claude/hooks/` (some via
  `code-env`). Do not treat this file as a hook inventory — inspect settings
  and the hooks directory when a hook blocks or surprises you.
- Prefer repo validation scripts (`pnpm validate:*` and friends — see
  `AGENTS.md`) over inventing Claude-layer build or test commands.
- **Fable model:** prefer `f5`-prefixed skills when an equivalent exists;
  otherwise use the standard skill.
- Machine-specific notes (local toggles, personal hook maps):
  `CLAUDE.local.md` if present (gitignored; not shared).
