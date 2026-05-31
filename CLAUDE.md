# Anvil

**For shared agent conventions (planning, commits, scope, code quality), see
`@AGENTS.md`.**

## Commands

No build or test commands in this config layer — the monorepo uses `pnpm` and
`nx` for builds/tests, `cargo` for Rust crates.

## Key Paths

- `plans/` — APS implementation plans; check before planning new features
- `docs/vision/` — North star docs for validating feature alignment (not scope)
- `.claude/` — Claude Code configuration (agents, hooks, skills, MCP servers)

## Active Hooks

Hook scripts live in `.claude/hooks/` (several symlinked from
`code-env/.claude/hooks/`). The event wiring is in **user** settings
(`~/.claude/settings.json`), not project settings — so the set below reflects
this machine's config. Command hooks receive the tool payload as JSON on
**stdin**.

| Hook                        | Trigger                   | What it does                                  | Gating                                             |
| --------------------------- | ------------------------- | --------------------------------------------- | -------------------------------------------------- |
| `security-guard.sh`         | PreToolUse (Bash)         | Blocks dangerous commands (`rm -rf`, dev/...) | Always                                             |
| `git-safety.sh`             | PreToolUse (Bash)         | Guards risky git operations                   | Always                                             |
| `local-review-precommit.sh` | PreToolUse (Bash)         | Reminds to check local review before commit   | `CLAUDE_LOCAL_REVIEW_PRECOMMIT` (default off)      |
| `council-gate.sh`           | PreToolUse (Bash)         | Blocks commit without Council review          | `CLAUDE_COUNCIL_GATE` (default off)                |
| `tdd-guard.sh`              | PreToolUse (Write\|Edit)  | TDD enforcement on edits                      | `CLAUDE_TDD_STRICT` / `CLAUDE_TDD_RUN_TESTS` (off) |
| `post-edit.sh`              | PostToolUse (Write\|Edit) | Post-edit lint                                | `CLAUDE_POST_EDIT_LINT` (default off)              |
| `codex-review-post.sh`      | PostToolUse (Bash)        | Async Codex review after commit               | `CLAUDE_CODEX_REVIEW` (on by default)              |
| `on-stop.sh`                | Stop                      | Desktop notification when the turn ends       | `CLAUDE_NOTIFICATION_COOLDOWN`                     |
| `on-agent-stop.sh`          | SubagentStop              | Subagent-stop triggers                        | `CLAUDE_AGENT_TRIGGERS`                            |
| `session-start.sh`          | SessionStart              | Tooling / git readiness banner at startup     | Always                                             |

> `kindling-capture.sh` is present in `.claude/hooks/` but is **not wired to any
> event** in the current settings — it does not run. Wire it under a
> `PostToolUse` matcher to activate.

## Council Review

Multi-perspective code review via `/council`. Spawns 5 specialist agents in
parallel (council-reviewer, kernel-maintainer, adversarial-reviewer,
operations-reviewer, pragmatic-lead). Findings are deduplicated, sorted by
severity, and synthesised into a unified verdict. Use for significant changes or
release prep. For quick reviews, use `/review` instead.

## Gotchas

- This repo uses ESM (`"type": "module"` in package.json)
- TypeScript is a devDependency but there's no tsconfig or source code to
  compile
