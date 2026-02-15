---
name: parallel-agents
description:
  Dispatching multiple agents concurrently, coordinating parallel work, managing
  subagent results
---

# Parallel Agents Skill

## Overview

Efficiently dispatch and coordinate multiple specialised agents to work
concurrently on independent tasks.

## When to Apply

- Large codebase exploration
- Multi-file refactoring
- Parallel testing strategies
- Comprehensive code review
- Research across multiple domains

## Coordination Patterns

### 1. Fan-Out Pattern

**Use when**: Tasks are independent and can run simultaneously.

```
Main Agent
    ├── Subagent A (task 1)
    ├── Subagent B (task 2)
    ├── Subagent C (task 3)
    └── Subagent D (task 4)
```

**Implementation**:

1. Spawn one agent per independent task with `spawn_agent`.
2. Keep task scope narrow (files, domain, expected output).
3. Wait for completion with `wait` over all agent IDs.

### 2. Pipeline Pattern

**Use when**: Tasks have dependencies but stages are parallelisable.

```
Stage 1: [Research A] [Research B] [Research C]
           ↓            ↓            ↓
Stage 2:      [Analysis of all research]
                        ↓
Stage 3:           [Implementation]
```

### 3. Specialist Pattern

**Use when**: Different expertise is needed.

```
Main Agent
    ├── Architect Agent → Design review
    ├── Security Agent → Vulnerability scan
    ├── Performance Agent → Bottleneck analysis
    └── Quality Agent → Code standards
```

## Best Practices

### Task Definition

Each parallel task should:

- Be **independent** (no shared mutable state)
- Have **clear scope** (specific files/domains)
- Include **success criteria** (how to know when done)
- Specify **output format** (what to return)

### Resource Management

- Start with 2-4 subagents, then scale up if useful.
- Balance thoroughness vs. speed.
- Prefer short, focused prompts over broad instructions.
- Use longer `wait` timeouts for deep analysis tasks.

### Result Aggregation

After parallel tasks complete:

1. Collect all results.
2. Identify conflicts or overlaps.
3. Synthesize into a unified view.
4. Resolve contradictions before acting.

## Example: Comprehensive Code Review

```markdown
## Launch Parallel Review Agents

### Agent 1: Functionality Review

Focus: Does the code work correctly? Files: All changed files Output: List of
functional issues

### Agent 2: Security Review

Focus: Are there security vulnerabilities? Files: All changed files Output:
Security findings with severity

### Agent 3: Performance Review

Focus: Are there performance concerns? Files: All changed files Output:
Performance issues and suggestions

### Agent 4: Style Review

Focus: Does code follow standards? Files: All changed files Output: Style
violations
```

## Coordination Commands

### Launching Agents

Use `spawn_agent` for each task, then `wait` on all IDs.

### Handling Results

```markdown
Results received:

- Agent 1: Found 3 issues in auth
- Agent 2: Found 2 issues in data layer
- Agent 3: Found 1 performance concern
- Agent 4: No issues found

Synthesised findings:

1. [Critical] Auth bypass in login.ts:45
2. [Major] SQL injection in query.ts:23
3. [Minor] N+1 query in users.ts:89
```

## Error Handling

### Agent Failure

If an agent fails:

1. Check if the task was too broad.
2. Retry with a tighter scope.
3. Fall back to sequential execution if needed.
4. Report partial results.

### Timeout Management

- Set timeouts based on task complexity.
- Poll with `wait` for long-running agents.
- Have a sequential fallback for stuck tasks.
