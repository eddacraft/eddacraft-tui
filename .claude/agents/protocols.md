# Shared Agent Protocols

This file defines the standard protocols used across all agents. Agents should reference this file rather than defining these protocols inline.

## Trigger Protocol

When your work reveals issues that another specialist should address, emit a trigger line:

```
TRIGGER:agent-name:context
```

For urgent issues, prefix the context with `!`:

```
TRIGGER:agent-name:!Urgent context here
```

**Trigger format is exactly:** `TRIGGER:<target-agent>:<context>`

### Standard Trigger Targets

| Target | When to trigger |
|--------|----------------|
| `security-analyst` | Security concerns, vulnerability findings, compliance questions |
| `architect` | System design issues, scalability concerns, technology decisions |
| `debugger` | Bug investigation, performance analysis, root cause analysis |
| `tdd-coach` | Missing tests, test strategy, regression test needs |
| `code-reviewer` | Code quality issues, review needed, fix implementation |
| `planner` | Planning needed, task breakdown, roadmap updates |

### Trigger Actions

| Action | Format | Use when |
|--------|--------|----------|
| Request fix | `TRIGGER:code-reviewer:Fix [description] in [file]` | Straightforward code fix needed |
| Request review | `TRIGGER:code-reviewer:Review [files] for [concern]` | Code needs specialist review |
| Negotiate | `TRIGGER:negotiate:<agent>:![topic] in [file]` | Technical tradeoff needs discussion |
| Escalate | `TRIGGER:<agent>:![urgent context]` | Blocking issue found |

## Negotiation Protocol

When participating in a negotiation (via `TRIGGER:negotiate` or direct discussion), follow this structure:

1. **Read the topic and any previous positions** from other agents
2. **State your position clearly** with domain-specific reasoning
3. **End your response** with exactly one of:
   - `CONSENSUS: [agreed approach]` — you agree with the other agent's position
   - `COUNTER: [your position]` — you have a different recommendation
   - `QUESTION: [clarification needed]` — you need more information before deciding

### Negotiation Rules

- Stay in your domain — argue from your expertise, not general preferences
- Be willing to update your position if the other agent raises valid points
- Accept tradeoffs when risks are properly mitigated
- Maximum 3 rounds before escalating to the user

## Severity Levels

All agents use the same severity scale:

| Level | Meaning | Commit gate? |
|-------|---------|-------------|
| **CRITICAL** | Must fix before commit. Data loss, security breach, crash, or broken build. | Blocks commit |
| **MAJOR** | Significant issue that should be fixed or negotiated before merge. | Should block |
| **MINOR** | Real issue but low impact or unlikely to trigger. | Advisory |
| **NIT** | Style or preference, not a bug. | Optional |

## Auto-Consultation

When `CLAUDE_AUTO_CONSULT` is enabled, agents may spawn specialists for second opinions on significant decisions. Skip consultation when:

- `CLAUDE_AUTO_CONSULT=false`
- Minor or small-scope changes
- User explicitly requests speed
- Already in a negotiation (avoid recursion)
- Follow-up to already-reviewed work

### Consultation Response Format

Specialists respond with one of:
- `APPROVE` — no concerns
- `SUGGEST` — minor improvements recommended
- `CONCERN` — significant issue to address
- `BLOCK` — cannot proceed without resolution
