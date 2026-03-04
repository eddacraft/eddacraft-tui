# Code-Env

Enhanced Claude Code environment with custom agents, hooks, skills, and agent
communication. This is a config-only repo — it provides `.claude/` configuration
that can be copied into other projects via `./setup-claude-config.sh`.

## Commands

```bash
npm run kindling:link   # Link @kindling/core, @kindling/cli, @kindling/store-sqlite
```

No build or test commands — this project is shell scripts and markdown
configuration.

## Project Structure

- `CLAUDE.md` — Claude Code instructions (this file)
- `README.md` — Human documentation
- `setup-claude-config.sh` — Sets up .claude/ config in another project (copy,
  symlink, or update)
- `check-version.sh` — Version check utility
- `plans/` — Implementation plans and specs
- `docs/vision/` — North star documents describing Anvil's long-term direction.
  Use these to validate whether new features align with the project's vision,
  but they are not scope documents — do not treat them as committed work items
- `.claude/` — All Claude Code configuration (agents, hooks, commands, skills,
  MCP servers)

## Setup Modes

```bash
./setup-claude-config.sh /path/to/project              # Copy mode (default)
./setup-claude-config.sh /path/to/project --symlink     # Granular symlink mode
./setup-claude-config.sh /path/to/project --update      # Re-sync from source
```

**Copy mode** — copies everything; project is fully independent.

**Symlink mode** — granular per-file symlinks for shared infrastructure (hooks,
skills, prompts, rules, settings), while copying extensible content (agents,
commands) so each project can add its own. Plugins writing to `.claude/agents/`
modify local copies, not the source repo.

**Update mode** — re-syncs shared files from source without touching
project-specific content. Adds new extensible files, updates unchanged ones,
skips user-modified files (detected via checksums in `.claude/.setup-meta`).
Cleans dangling symlinks from removed source files.

Old whole-directory symlinks (pre-granular) are auto-detected and migration is
offered.

## Active Hook Behavior

These hooks run automatically and affect how Claude operates:

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

Current settings (from `settings.json`): Forge is **on**, pre-commit codex
review is **off**, post-edit lint is **off**, TDD strict is **off**, agent
triggers are **off**, auto-consult is **off**. Codex review runs async
post-commit only.

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

- **codex** — GPT delegation via `codex` CLI (model: gpt-5.2-high in
  settings.json)
- **memory** — Persistent context at `.claude/memory.json`
- **filesystem** — File operations scoped to project dir
- **Neon** — Neon database MCP (in settings.json)
- brave-search, github, puppeteer — defined in `mcp.json` but disabled

## Agent Messaging

Agents communicate via `.claude/agent-bus/`:

- `send-message.sh --from X --to Y --type finding --payload '{...}'`
- `receive-messages.sh <agent> --format summary`
- `check-queue.sh` — manage trigger queue (`--pop`, `--mark-done <id>`)
- Trigger format in agent output: `TRIGGER:agent-name:context` (disabled by
  default)

## Forge & Temper Pipeline

Autonomous code review pipeline with two complementary phases:

- **Forge** (pre-commit, local): `forge.sh` hook intercepts `git commit`, spawns
  `forge-reviewer` agent for cross-model review via codex MCP, runs structured
  negotiation (max 3 rounds). Findings categorized by severity — critical/major
  must be fixed, minor is author's choice, nits auto-deferred. Deferred findings
  filed as GH issues (`forge:deferred` label) or APS work items.
- **Temper** (post-push, GitHub Actions): `temper.yml` workflow auto-addresses
  CI review comments. Max 2 cycles — cycle 1 addresses all fixable findings,
  cycle 2 is scoped to lines changed by cycle-1 fixes. Remaining findings
  deferred to issues. Triggered automatically when `forge:tempered` label is
  present, or manually via `workflow_dispatch`.

| Scenario               | Forge | Temper | What happens                                    |
| ---------------------- | ----- | ------ | ----------------------------------------------- |
| Full autonomous        | on    | on     | Pre-commit review + auto self-healing post-push |
| Local review only      | on    | off    | Pre-commit review, manual PR handling           |
| Auto self-healing only | off   | on     | No pre-commit, but PR reviews auto-addressed    |
| Everything off         | off   | off    | Current manual workflow (unchanged)             |

GitHub repo variables for Temper: `CLAUDE_TEMPER_ENABLED` (default: `false`),
`CLAUDE_TEMPER_MAX_CYCLES` (default: `2`). Manual dispatch always works
regardless of toggle.

See: `docs/plans/2026-02-24-forge-temper-review-pipeline.md`

## Gotchas

- `mcp.json` and `settings.json` both define MCP servers — `settings.json` takes
  precedence and uses a different codex model
- The `--no-verify` flag on git commit bypasses the codex pre-commit hook
- `plans/` directory contains implementation specs — check there before planning
  new features
- This repo uses ESM (`"type": "module"` in package.json)
- TypeScript is a devDependency but there's no tsconfig or source code to
  compile
