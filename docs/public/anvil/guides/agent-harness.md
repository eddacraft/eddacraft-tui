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

# Start anvil in watch mode with JSON output (background)
# --json is a global flag and must precede the subcommand
anvil --json watch --source > anvil.log &
ANVIL_PID=$!

# Run agent
your-agent-cli "$@"

# Check anvil results (WatchEvent uses snake_case fields)
if grep -q '"event_type":"violation"' anvil.log; then
  echo "Agent produced failing code"
  exit 1
fi

kill $ANVIL_PID
```

### Pattern 2: MCP Integration

Use anvil's MCP server to give agents real-time access to architecture rules and
validation:

```json
{
  "mcpServers": {
    "anvil": {
      "command": "npx",
      "args": ["@eddacraft/anvil-mcp-server"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

The agent can then:

- Query architecture layer and boundary rules via `anvil://boundaries`
- Validate changes before proposing via `anvil_check`
- Check current violations via `anvil_status`

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

On Git 2.54 or newer, native config-based hooks are planned as a future option,
but `anvil hooks install` writes a file-based hook today. Native config-mode
install is gated on `GHOOK-002`. See the
[Git hook setup](/anvil/operations/git-hooks) page for the rollout shape.

## Telemetry and Learning

Track agent behaviour over time by reviewing validation results:

- **Violation rate** — how often does the agent drift?
- **Common violations** — what patterns recur?
- **Improvement over time** — is the agent learning from rejections?

Use `anvil --json check --all` to capture structured results for analysis.

## Coming Soon

The following features are planned to make agent harnesses more powerful:

- **`anvil mcp serve`** — built-in MCP server in the Rust binary (currently
  requires the separate `@eddacraft/anvil-mcp-server` Node.js package)
- **Plan-first workflow** — `anvil plan create` and `anvil session start/end`
  commands for structured agent task scoping
- **`@eddacraft/anvil-client` SDK** — TypeScript client for programmatic session
  management, constraint queries, and validation
- **Evidence querying** — `anvil evidence list` for analysing agent behaviour
  patterns over time

---

**Next:** [GitHub integration →](/anvil/integrations/github)
