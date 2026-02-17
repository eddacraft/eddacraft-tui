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
| `codex-review.sh`      | PreToolUse (Bash)        | GPT review before commits        | `CLAUDE_CODEX_REVIEW_PRECOMMIT=false` (off) |
| `post-edit.sh`         | PostToolUse (Write/Edit) | Auto-format and lint after edits | `CLAUDE_POST_EDIT_LINT=false` (off)         |
| `codex-review-post.sh` | PostToolUse (Bash)       | GPT suggestions after commits    | `CLAUDE_CODEX_REVIEW=true` + async          |
| `tdd-guard.sh`         | PreToolUse               | Enforces test-first development  | `CLAUDE_TDD_STRICT=false` (off)             |
| `on-stop.sh`           | Stop                     | Desktop notifications            | Always                                      |
| `on-agent-stop.sh`     | SubagentStop             | Parses agent trigger lines       | `CLAUDE_AGENT_TRIGGERS=false` (off)         |
| `session-start.sh`     | SessionStart             | Environment check                | Always                                      |
| `kindling-capture.sh`  | PostToolUse              | Kindling integration             | Always                                      |

Current settings (from `settings.json`): pre-commit codex review is **off**,
post-edit lint is **off**, TDD strict is **off**, agent triggers are **off**,
auto-consult is **off**. Codex review runs async post-commit only.

## Environment Variable Toggles

| Variable                        | Current Value | What it controls                            |
| ------------------------------- | ------------- | ------------------------------------------- |
| `ANVIL_DEBUG`                   | `0`           | Enable debug logging across all Anvil       |
| `ANVIL_TRACE`                   | `0`           | Enable trace logging in shell hooks         |
| `ANVIL_LOG_FILE`                | (none)        | Append shell hook logs to this file         |
| `ANVIL_LOG_TS`                  | `0`           | Prefix shell hook log lines with timestamps |
| `CLAUDE_CODEX_REVIEW`           | `true`        | Master switch for all GPT code review       |
| `CLAUDE_CODEX_REVIEW_PRECOMMIT` | `false`       | Block commits on critical GPT findings      |
| `CLAUDE_CODEX_REVIEW_ASYNC`     | `true`        | Run post-commit review in background        |
| `CLAUDE_POST_EDIT_LINT`         | `false`       | Auto-lint after file edits                  |
| `CLAUDE_TDD_STRICT`             | `false`       | Require test file before editing source     |
| `CLAUDE_TDD_RUN_TESTS`          | `false`       | Run related tests before allowing edits     |
| `CLAUDE_AUTO_CONSULT`           | `false`       | Architect/planner auto-consult specialists  |
| `CLAUDE_AGENT_TRIGGERS`         | `false`       | Parse TRIGGER: lines from agent output      |
| `CLAUDE_NOTIFICATION_COOLDOWN`  | `60`          | Seconds between desktop notifications       |
| `CLAUDE_CODE_MAX_SUBAGENTS`     | `5`           | Max concurrent subagents                    |

## Debug & Trace Logging

Anvil has two complementary logging systems.

### Anvil Application Debug (TypeScript)

The primary debug system lives in `packages/anvil/core/src/utils/debug.ts` and
uses `createDebugger()` with namespaced loggers across all packages.

```bash
# Enable all Anvil debug output
ANVIL_DEBUG=1 anvil gate plan.md

# Enable specific namespace(s) via DEBUG
DEBUG=anvil:gate anvil gate plan.md
DEBUG=anvil:cache,anvil:check anvil gate plan.md
DEBUG=anvil:* anvil gate plan.md     # all namespaces
```

#### Debug Namespaces

| Namespace        | Package   | What it covers                               |
| ---------------- | --------- | -------------------------------------------- |
| `gate`           | runtime   | Gate runner orchestration, check dispatch    |
| `check`          | runtime   | Individual gate check execution & results    |
| `cache`          | runtime   | Cache hits/misses, key generation, file I/O  |
| `watch`          | runtime   | File watcher, debouncer, orchestrator        |
| `architecture`   | core      | Architecture analysis, baselines, layers     |
| `compiler`       | core      | YAML parsing, Rego/DC code generation        |
| `edge-detector`  | core      | Architecture edge detection                  |
| `entry-detector` | core      | Architecture entry point detection           |
| `drift`          | core      | Snapshot capture, comparison, reports        |
| `provenance`     | core      | Provenance collection and storage            |
| `validation`     | core      | APS document and warning validation          |
| `suppression`    | core      | Suppression parsing and application          |
| `explain`        | core      | Explain service and template loading         |
| `config`         | core      | Configuration loading                        |
| `secret`         | runtime   | Secret detection patterns                    |
| `git-ai-notes`   | core      | Git AI notes serialization                   |
| `policy`         | policy    | OPA execution, bundle management             |
| `agent`          | runtime   | Multi-agent registration and tracking        |
| `lock`           | runtime   | Distributed file-based locking               |
| `queue`          | runtime   | Sequential execution queue                   |
| `atomic`         | runtime   | Atomic file operations                       |
| `git-agent`      | runtime   | Git-aware agent coordination                 |
| `cli`            | anvil-cli | CLI command entry/exit, argument handling    |
| `service`        | anvil-cli | CLI services (project detection, hooks, etc) |
| `kindling`       | kindling  | Observation emission, queries, sessions      |
| `api`            | anvil-api | API routes, middleware, database             |
| `adapter`        | kindling  | Kindling adapter creation                    |
| `export`         | runtime   | Constraint export, LLMs.txt, MCP resources   |

#### Adding Debug Logging

```typescript
import { createDebugger } from '@eddacraft/anvil-core';
const log = createDebugger('gate');

// Usage
log('running check', { name: check.name, cached: false });
log('check failed', error); // Error objects get stack traces
```

Output (when enabled): `[2026-02-17T14:32:15.123Z] [anvil:gate] running check`

Secrets are automatically redacted (tokens, API keys, Bearer headers).

### Shell Hook Logging

Claude Code hooks (`.claude/hooks/`) use a separate bash logging library at
`.claude/hooks/lib/log.sh`, controlled by `ANVIL_TRACE` and `ANVIL_LOG_FILE`.
Logs auto-append to `.anvil/logs/hooks.log` when `ANVIL_DEBUG=1` or
`ANVIL_TRACE=1`. See the library header for details.

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

## Gotchas

- `mcp.json` and `settings.json` both define MCP servers — `settings.json` takes
  precedence and uses a different codex model
- The `--no-verify` flag on git commit bypasses the codex pre-commit hook
- `plans/` directory contains implementation specs — check there before planning
  new features
- This repo uses ESM (`"type": "module"` in package.json)
- TypeScript is a devDependency but there's no tsconfig or source code to
  compile
