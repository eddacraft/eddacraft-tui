---
name: agent-negotiation
description:
  Multi-round dialogue between agents to reach consensus on technical decisions
---

# Agent Negotiation Skill

## Overview

Orchestrate structured dialogue between two specialist agents to reach consensus
on technical decisions. The main agent acts as facilitator, spawning agents in
rounds until agreement or max rounds is reached.

## When to Apply

- Architectural decisions requiring multiple perspectives
- Security vs usability tradeoffs
- Performance vs maintainability choices
- Design disagreements that benefit from structured debate
- Complex technical decisions with no obvious answer

## Negotiation Protocol

### Dialogue Structure

```
┌─────────────────────────────────────────────────────────┐
│                    Main Agent (Facilitator)             │
│                                                         │
│  Round 1: Spawn Agent A -> returns POSITION             │
│  Round 2: Spawn Agent B with A's position -> COUNTER    │
│  Round 3: Spawn Agent A with B's counter -> CONSENSUS?  │
│  ...                                                    │
│  Final: Synthesize outcome or declare deadlock          │
└─────────────────────────────────────────────────────────┘
```

### Response Format

Agents should end their response with one of:

```
CONSENSUS: [agreed approach in one sentence]
COUNTER: [disagreement and alternative in one sentence]
QUESTION: [clarification needed before deciding]
```

### Max Rounds

Default: 4 rounds (2 per agent) Configurable via `CODEX_NEGOTIATION_MAX_ROUNDS`

## Codex Implementation

### Step 1: Initialize Negotiation

Optionally create a signal file to track state:

```
.codex/agent-bus/signals/neg-{uuid}.json
```

```json
{
  "id": "{uuid}",
  "topic": "Should we use REST or GraphQL?",
  "participants": ["architect", "security-analyst"],
  "status": "in_progress",
  "round": 1,
  "history": []
}
```

### Step 2: First Agent Position

1. `spawn_agent` for agent A with negotiation prompt.
2. `wait` for completion.
3. Parse response tail for CONSENSUS/COUNTER/QUESTION.

### Step 3: Subsequent Rounds

1. `spawn_agent` or `send_input` to agent B with accumulated history.
2. Continue alternating roles.
3. Persist history after each round.

### Step 4: Resolution Logic

After each round:

1. Parse response for CONSENSUS/COUNTER/QUESTION.
2. If CONSENSUS: end negotiation and record outcome.
3. If QUESTION: answer it and continue.
4. If COUNTER at max rounds: declare deadlock.
5. Otherwise continue to next round.

### Step 5: Outcome to User

```markdown
## Negotiation Result

**Topic:** {topic} **Participants:** {agent_a}, {agent_b} **Rounds:** {n}
**Outcome:** {CONSENSUS|DEADLOCK}

### Summary

{synthesized_outcome_or_positions}

### Key Points

- {agent_a}: {final_position}
- {agent_b}: {final_position}

### Recommendation

{facilitator_recommendation_if_deadlock}
```

## Agent Selection Guidelines

| Decision Type      | Recommended Agents           |
| ------------------ | ---------------------------- |
| Architecture       | architect + security-analyst |
| Performance        | architect + debugger         |
| Testing strategy   | tdd-coach + code-reviewer    |
| Security tradeoffs | security-analyst + architect |
| Code quality       | code-reviewer + architect    |

## Configuration

| Variable                       | Default | Description                |
| ------------------------------ | ------- | -------------------------- |
| `CODEX_NEGOTIATION_MAX_ROUNDS` | 4       | Max rounds before deadlock |
| `CODEX_NEGOTIATION_TIMEOUT`    | 300000  | Per-agent timeout (ms)     |

## Error Handling

### Agent Doesn't Follow Protocol

If response lacks CONSENSUS/COUNTER/QUESTION:

1. Treat as implicit COUNTER.
2. Extract the best available position from response text.
3. Continue negotiation.

### Deadlock After Max Rounds

1. Summarise both positions.
2. Highlight points of agreement.
3. Present a recommendation based on context.
4. Let user make the final decision.

### Agent Timeout

1. Record partial response if any.
2. Offer retry or continuation with available evidence.
3. Log timeout for debugging.
