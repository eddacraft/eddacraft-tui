---
id: agent-harness
title: Agent Harness Patterns
description: Using anvil as a harness for AI coding agents.
sidebar_position: 3
---

# Agent Harness Patterns

anvil can serve as a "harness" for AI coding agents—constraining their actions
and validating their outputs.

## The Problem

AI coding agents (Claude, GPT-Engineer, Aider, etc.) can:

- **Write code autonomously** — with varying quality
- **Execute commands** — potentially destructive
- **Make architectural decisions** — without understanding your constraints

Without guardrails, agents drift from your intended architecture.

## anvil as a Harness

A harness constrains an agent's behaviour:

```
┌────────────────────────────────────────────────┐
│                   Harness                       │
│                                                 │
│   Agent ──▶ Plan ──▶ Execute ──▶ Validate     │
│              │                      │          │
│              └──────── anvil ───────┘          │
│                                                 │
└────────────────────────────────────────────────┘
```

### Plan Constraint

Agent works within a defined plan:

```markdown
## Task: AUTH-001

Outcome: Users can log in with email/password

Allowed files:

- src/auth/\*\*
- src/types/auth.ts

Forbidden:

- src/payments/\*\*
- Any database migrations
```

### Execution Validation

Every change is validated before acceptance:

```
Agent: "I've created src/auth/login.ts"
anvil: ✓ File is within allowed scope
anvil: ✓ No architecture violations
anvil: ✓ No anti-patterns detected
Harness: Accept change
```

### Rejection Flow

```
Agent: "I've modified src/payments/processor.ts"
anvil: ✗ File outside task scope
Harness: Reject change
Agent: "Understood. I'll find another approach."
```

## Patterns That Work Today

### Pattern 1: Wrapper Script

Wrap your agent invocation:

```bash
#!/bin/bash
# run-agent.sh

# Start anvil in watch mode with JSON output (background).
# --json is a global flag and must precede the subcommand
anvil --json watch --source > anvil.ndjson 2> anvil.watch.log &
ANVIL_PID=$!
trap 'kill "$ANVIL_PID" 2>/dev/null || true' EXIT

# Run agent
your-agent-cli "$@"

# Check anvil results by parsing the NDJSON event stream.
if jq -e 'select(.event_type == "violation")' anvil.ndjson >/dev/null; then
  echo "Agent produced failing code"
  exit 1
fi

kill "$ANVIL_PID"
```

### Pattern 2: MCP Integration

Use anvil's MCP server to give agents real-time access to architecture rules and
validation:

```json
{
  "mcpServers": {
    "anvil": {
      "command": "anvil",
      "args": ["mcp", "serve", "--stdio"],
      "env": {}
    }
  }
}
```

The agent can then:

- Validate proposed writes before applying them via `anvil_validate_write`
- Use `anvil mcp-config` when you need editor-specific config generation or
  verification
- Use the legacy Node.js MCP server only if you need the broader legacy tool and
  resource surface today

When you run both patterns together (a background `anvil watch` plus the MCP
server), daemon-backed save-time routing is used automatically when the resident
daemon is live, so watch and `anvil_validate_write` share one warm verdict path
instead of each spinning up its own scan — useful when several agents are
editing concurrently. Set `ANVIL_WATCH_DAEMON=0` to opt out, or
`ANVIL_WATCH_DAEMON=1` to force the daemon path while diagnosing routing. The
[save-time validation guide](save-time-validation.md) covers the full routing
story, assurance states, and fallback behaviour.

In an agent or headless harness, bring the daemon up explicitly with
`anvil intercept start --foreground`. The upcoming `v0.9.0-beta` auto-start in
`anvil start` and the `anvil watch` offer are both deliberately suppressed in
headless, `--json`, CI, hook, and piped contexts, so neither starts a daemon
unattended — only an interactive (at-the-keyboard) `anvil start` auto-starts
one. Either way an unattended harness never blocks on a prompt and falls back
deterministically to the scoped check until a daemon is running.

See [MCP Integration](/anvil/integrations/mcp) for full configuration and tool
reference.

### Pattern 3: Pre-commit Hook

Catch issues before they're committed:

```bash
#!/bin/bash
# .git/hooks/pre-commit

anvil check --changed --staged
if [ $? -ne 0 ]; then
  echo "anvil validation failed. Commit blocked."
  exit 1
fi
```

Config-mode alternative on Git 2.54 or newer: `anvil hooks install --config`
manages the same Git pre-commit hook entry point through native `[hook.<name>]`
blocks instead of a file, but it installs Anvil's managed `anvil gate` hook
rather than the exact `anvil check --changed --staged` script shown above. File
mode remains the default; see [Git hook setup](/anvil/operations/git-hooks) for
both modes and coexistence rules.

## Telemetry and Learning

Track agent behaviour over time by reviewing validation results:

- **Violation rate** — how often does the agent drift?
- **Common violations** — what patterns recur?
- **Improvement over time** — is the agent learning from rejections?

Use `anvil --json check --all` to capture structured results for analysis.

## Coming Soon

The following features are planned to make agent harnesses more powerful:

- **Plan-first workflow** — guided APS creation and `anvil session start/end`
  commands for structured agent task scoping
- **`@eddacraft/anvil-client` SDK** — TypeScript client for programmatic session
  management, constraint queries, and validation
- **Evidence querying** — `anvil evidence list` for analysing agent behaviour
  patterns over time

`anvil mcp serve --stdio` shipped in 0.5.0-beta — see
[MCP Integration](/anvil/integrations/mcp) for the current tool surface.

---

**Next:** [GitHub integration →](/anvil/integrations/github)
