# anvil

**Shared conventions:** `@AGENTS.md`  
**Repo map:** `@CONTEXT.md`  
**Skills / agents / commands:** `@docs/guides/agent-surface-inventory.md`

This file is the **Claude Code adapter only**. Do not restate shared workflow,
validation commands, architecture, or skill procedures here — those live in
`AGENTS.md`, `CONTEXT.md`, and the inventory. The CI-log closeout bullet below
is an intentional Claude-adapter reminder of the Operating Rules MUST (so Claude
sessions still see it when the shared file is truncated).

## Claude-only

- **CI-log closeout:** before the final response on non-trivial work, run
  `pnpm ci-log:append -- --agent claude --task "<summary>"` (pending). Claude
  sessions skip this more often than Codex/Grok — it is not a named
  `agentic-loop` step. `Improvement: none` is valid. Do not skip because the
  note looks unrelated to the feature PR.
- **Hooks and event wiring** live in **user** settings
  (`~/.claude/settings.json`). Scripts under `.claude/hooks/` (some via
  `code-env`). Do not treat this file as a hook inventory — inspect settings and
  the hooks directory when a hook blocks or surprises you.
- Prefer repo validation scripts (`pnpm validate:*` and friends — see
  `AGENTS.md`) over inventing Claude-layer build or test commands.
- **Fable model:** prefer `f5`-prefixed skills when an equivalent exists;
  otherwise use the standard skill.
- Machine-specific notes (local toggles, personal hook maps): `CLAUDE.local.md`
  if present (gitignored; not shared).
