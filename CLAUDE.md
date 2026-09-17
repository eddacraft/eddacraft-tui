# anvil

> **Start here:** this repository uses the published `anvil` CLI by default;
> `anvil-main` remains an optional rolling dogfood channel. Before the final
> response on non-trivial work, **MUST** append the pending CI-log note required
> by `AGENTS.md`.

**Shared conventions:** `@AGENTS.md`  
**Repo map:** `@CONTEXT.md`  
**Skills / agents / commands:** `@docs/guides/agent-surface-inventory.md`

Claude Code adapter only. Do not restate shared workflow, validation,
architecture, or skill procedures — those live in the files above. The CI-log
bullet is an intentional reminder of the Operating Rules MUST (Claude sessions
still see it when `AGENTS.md` is truncated).

## Claude-only

- **CI-log closeout:** before the final response on non-trivial work, run
  `pnpm ci-log:append -- --agent claude --task "<summary>"` (pending).
  `Improvement: none` is valid. Not a named `agentic-loop` step. Do not skip
  because the note looks unrelated to the feature PR.
- **Hooks and event wiring** live in **user** settings
  (`~/.claude/settings.json`). Scripts under `.claude/hooks/` (some via
  `code-env`). Inspect settings and the hooks directory when a hook blocks or
  surprises you; this file is not a hook inventory.
- **Fable model:** prefer `f5`-prefixed skills when an equivalent exists;
  otherwise use the standard skill.
- Machine-specific notes: `CLAUDE.local.md` if present (gitignored; not shared).
