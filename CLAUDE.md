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

All hooks are symlinked from `code-env/.claude/hooks/`.

| Hook                        | Trigger           | What it does                           | Active?                                     |
| --------------------------- | ----------------- | -------------------------------------- | ------------------------------------------- |
| `git-safety.sh`             | PreToolUse (Bash) | Blocks dangerous shell commands        | Always                                      |
| `council-gate.sh`           | PreToolUse (Bash) | Checks for Council review on commit    | `CLAUDE_COUNCIL_GATE=false` (off)           |
| `local-review-precommit.sh` | PreToolUse (Bash) | Reminds to check Council before commit | `CLAUDE_LOCAL_REVIEW_PRECOMMIT=false` (off) |
| `kindling-capture.sh`       | PostToolUse       | Kindling integration                   | Always                                      |

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
