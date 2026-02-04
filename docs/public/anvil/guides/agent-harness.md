---
id: agent-harness
title: Agent Harness Patterns
description: Using Anvil as a harness for AI coding agents.
sidebar_position: 3
---

# Agent Harness Patterns

Anvil can serve as a "harness" for AI coding agents—constraining their actions
and validating their outputs.

## The Problem

AI coding agents (Claude, GPT-Engineer, Aider, etc.) can:

- **Write code autonomously** — with varying quality
- **Execute commands** — potentially destructive
- **Make architectural decisions** — without understanding your constraints

Without guardrails, agents drift from your intended architecture.

## Anvil as a Harness

A harness constrains an agent's behaviour:

```
┌────────────────────────────────────────────────┐
│                   Harness                       │
│                                                 │
│   Agent ──▶ Plan ──▶ Execute ──▶ Validate     │
│              │                      │          │
│              └──────── Anvil ───────┘          │
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
Anvil: ✓ File is within allowed scope
Anvil: ✓ No architecture violations
Anvil: ✓ No anti-patterns detected
Harness: Accept change
```

### Rejection Flow

```
Agent: "I've modified src/payments/processor.ts"
Anvil: ✗ File outside task scope
Harness: Reject change
Agent: "Understood. I'll find another approach."
```

## Implementation Patterns

### Pattern 1: Wrapper Script

Wrap your agent invocation:

```bash
#!/bin/bash
# run-agent.sh

# Start Anvil in watch mode (background)
anvil watch --json > anvil.log &
ANVIL_PID=$!

# Run agent
your-agent-cli "$@"

# Check Anvil results
if grep -q '"status":"fail"' anvil.log; then
  echo "Agent produced failing code"
  exit 1
fi

kill $ANVIL_PID
```

### Pattern 2: MCP Integration

Use Anvil via MCP (Model Context Protocol):

```json
{
  "mcpServers": {
    "anvil": {
      "command": "anvil",
      "args": ["mcp", "serve"]
    }
  }
}
```

The agent can then:

- Query allowed files for a task
- Validate changes before proposing
- Check current violations

### Pattern 3: Pre-commit Hook

Catch issues before they're committed:

```bash
#!/bin/bash
# .git/hooks/pre-commit

anvil run --staged
if [ $? -ne 0 ]; then
  echo "Anvil validation failed. Commit blocked."
  exit 1
fi
```

### Pattern 4: Plan-First Workflow

Require a plan before agent execution:

```bash
# 1. Create plan
anvil plan create --task "Add user authentication"

# 2. Review and approve plan
cat plans/execution/AUTH-001.steps.md
# Edit if needed

# 3. Run agent within plan
anvil session start --task AUTH-001
your-agent-cli "Implement AUTH-001"
anvil session end
```

## Example: Claude + Anvil

Using Claude with Anvil harness:

```typescript
import { AnvilClient } from '@eddacraft/anvil-client';
import { Anthropic } from '@anthropic-ai/sdk';

const anvil = new AnvilClient();
const claude = new Anthropic();

async function runWithHarness(task: string) {
  // Start Anvil session
  const session = await anvil.startSession({ task });

  // Get task constraints
  const constraints = await anvil.getTaskConstraints(task);

  // Run Claude with constraints in prompt
  const response = await claude.messages.create({
    model: 'claude-3-opus-20240229',
    messages: [
      {
        role: 'user',
        content: `
        Task: ${constraints.outcome}

        Allowed files: ${constraints.allowedFiles.join(', ')}
        Forbidden: ${constraints.forbiddenPatterns.join(', ')}

        Implement this task.
      `,
      },
    ],
  });

  // Validate Claude's output
  const validation = await anvil.validate(response.content);

  if (validation.status === 'fail') {
    // Rejection flow
    return await runWithHarness(task); // Retry with feedback
  }

  await anvil.endSession(session.id);
  return response;
}
```

## Telemetry and Learning

Track agent behaviour over time:

```bash
anvil evidence list --agent claude --since 30d
```

Analyse:

- **Violation rate** — how often does the agent drift?
- **Common violations** — what patterns recur?
- **Improvement over time** — is the agent learning from rejections?

---

**Next:** [GitHub integration →](/docs/anvil/integrations/github)
