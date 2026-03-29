# Anvil

**For shared agent conventions (planning, commits, scope, code quality), see
`@AGENTS.md`.**

## Commands

```bash
npm run kindling:link   # Link @kindling/core, @kindling/cli, @kindling/store-sqlite
```

No build or test commands in this config layer — the monorepo uses `pnpm` and
`nx` for builds/tests, `cargo` for Rust crates.

## Key Paths

- `plans/` — APS implementation plans; check before planning new features
- `docs/vision/` — North star docs for validating feature alignment (not scope)
- `.claude/` — Claude Code configuration (agents, hooks, skills, MCP servers)
- `setup-claude-config.sh` — Copies/symlinks `.claude/` config into other
  projects (copy, `--symlink`, or `--update` modes)

## Active Hooks

| Hook                   | Trigger                  | What it does                     | Active?                                     |
| ---------------------- | ------------------------ | -------------------------------- | ------------------------------------------- |
| `security-guard.sh`    | PreToolUse (Bash)        | Blocks dangerous shell commands  | Always                                      |
| `forge.sh`             | PreToolUse (Bash)        | Pre-commit review negotiation    | `CLAUDE_FORGE_ENABLED=true` (on)            |
| `codex-review.sh`      | PreToolUse (Bash)        | GPT review before commits        | `CLAUDE_CODEX_REVIEW_PRECOMMIT=false` (off) |
| `post-edit.sh`         | PostToolUse (Write/Edit) | Auto-format and lint after edits | `CLAUDE_POST_EDIT_LINT=false` (off)         |
| `codex-review-post.sh` | PostToolUse (Bash)       | GPT suggestions after commits    | `CLAUDE_CODEX_REVIEW=true` + async          |
| `tdd-guard.sh`         | PreToolUse               | Enforces test-first development  | `CLAUDE_TDD_STRICT=false` (off)             |
| `on-stop.sh`           | Stop                     | Desktop notifications            | Always                                      |
| `on-agent-stop.sh`     | SubagentStop             | Parses agent trigger lines       | `CLAUDE_AGENT_TRIGGERS=false` (off)         |
| `session-start.sh`     | SessionStart             | Environment check                | Always                                      |
| `kindling-capture.sh`  | PostToolUse              | Kindling integration             | Always                                      |

## Environment Variable Toggles

| Variable                        | Current Value | What it controls                           |
| ------------------------------- | ------------- | ------------------------------------------ |
| `CLAUDE_CODEX_REVIEW`           | `true`        | Master switch for all GPT code review      |
| `CLAUDE_CODEX_REVIEW_PRECOMMIT` | `false`       | Block commits on critical GPT findings     |
| `CLAUDE_CODEX_REVIEW_ASYNC`     | `true`        | Run post-commit review in background       |
| `CLAUDE_POST_EDIT_LINT`         | `false`       | Auto-lint after file edits                 |
| `CLAUDE_TDD_STRICT`             | `false`       | Require test file before editing source    |
| `CLAUDE_TDD_RUN_TESTS`          | `false`       | Run related tests before allowing edits    |
| `CLAUDE_AUTO_CONSULT`           | `false`       | Architect/planner auto-consult specialists |
| `CLAUDE_AGENT_TRIGGERS`         | `false`       | Parse TRIGGER: lines from agent output     |
| `CLAUDE_NOTIFICATION_COOLDOWN`  | `60`          | Seconds between desktop notifications      |
| `CLAUDE_CODE_MAX_SUBAGENTS`     | `5`           | Max concurrent subagents                   |
| `CLAUDE_FORGE_ENABLED`          | `true`        | Pre-commit review via forge-reviewer agent |
| `CLAUDE_FORGE_MAX_ROUNDS`       | `3`           | Max negotiation rounds before auto-defer   |
| `CLAUDE_FORGE_AUTO_DEFER_NITS`  | `true`        | Auto-defer nit findings without arguing    |

## MCP Servers

- **codex** — GPT delegation via `codex` CLI (model: gpt-5.2-high)
- **memory** — Persistent context at `.claude/memory.json`
- **filesystem** — File operations scoped to project dir
- **Neon** — Neon database MCP
- brave-search, github, puppeteer — defined in `mcp.json` but disabled

## Agent Messaging

Agents communicate via `.claude/agent-bus/`:

- `send-message.sh --from X --to Y --type finding --payload '{...}'`
- `receive-messages.sh <agent> --format summary`
- `check-queue.sh` — manage trigger queue (`--pop`, `--mark-done <id>`)
- Trigger format in agent output: `TRIGGER:agent-name:context` (disabled by
  default)

## Council Review

Multi-perspective code review via `/council`. Spawns 5 specialist agents in
parallel (council-reviewer, kernel-maintainer, adversarial-reviewer,
operations-reviewer, pragmatic-lead). Findings are deduplicated, sorted by
severity, and synthesised into a unified verdict. Use for significant changes or
release prep. For quick reviews, use `/review` instead.

## Forge Pipeline

Pre-commit review via `forge.sh` hook. Intercepts `git commit`, spawns
`forge-reviewer` for cross-model review via codex MCP (max 3 rounds).
Critical/major findings must be fixed, minor is author's choice, nits
auto-deferred. Deferred findings filed as GH issues or APS work items.

## Gotchas

- `mcp.json` and `settings.json` both define MCP servers — `settings.json` wins
  and uses a different codex model
- `--no-verify` on git commit bypasses the forge/codex pre-commit hook
- This repo uses ESM (`"type": "module"` in package.json)
- TypeScript is a devDependency but there's no tsconfig or source code to
  compile
