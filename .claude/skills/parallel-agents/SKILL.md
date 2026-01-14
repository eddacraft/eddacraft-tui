---
name: parallel-agents
description:
  Dispatching multiple agents concurrently, coordinating parallel work, managing
  subagent results
---

# Parallel Agents Skill

## Overview

Efficiently dispatch and coordinate multiple specialized agents to work
concurrently on independent tasks.

## When to Apply

- Large codebase exploration
- Multi-file refactoring
- Parallel testing strategies
- Comprehensive code review
- Research across multiple domains

## Coordination Patterns

### 1. Fan-Out Pattern

**Use when**: Tasks are independent and can run simultaneously

```
Main Agent
    ├── Subagent A (task 1)
    ├── Subagent B (task 2)
    ├── Subagent C (task 3)
    └── Subagent D (task 4)
```

**Implementation**:

```markdown
Launch multiple Task tools in a single message:

- Task 1: Explore auth module
- Task 2: Explore data layer
- Task 3: Explore API routes
- Task 4: Explore utilities
```

### 2. Pipeline Pattern

**Use when**: Tasks have dependencies but stages are parallelizable

```
Stage 1: [Research A] [Research B] [Research C]
           ↓            ↓            ↓
Stage 2:      [Analysis of all research]
                        ↓
Stage 3:           [Implementation]
```

### 3. Specialist Pattern

**Use when**: Different expertise needed for different aspects

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

- Don't exceed `CLAUDE_CODE_MAX_SUBAGENTS` limit
- Consider API rate limits
- Balance thoroughness vs. speed
- Use appropriate model for task complexity

### Result Aggregation

After parallel tasks complete:

1. Collect all results
2. Identify conflicts or overlaps
3. Synthesize into unified view
4. Resolve any contradictions

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

```markdown
Use the Task tool with multiple invocations:

<Task 1> subagent_type: Explore prompt: "Research the authentication system..."
</Task 1>

<Task 2> subagent_type: Explore prompt: "Research the data access layer..."
</Task 2>
```

### Handling Results

```markdown
Results received:

- Agent 1: Found 3 issues in auth
- Agent 2: Found 2 issues in data layer
- Agent 3: Found 1 performance concern
- Agent 4: No issues found

Synthesized findings:

1. [Critical] Auth bypass in login.ts:45
2. [Major] SQL injection in query.ts:23
3. [Minor] N+1 query in users.ts:89
```

## Error Handling

### Agent Failure

If an agent fails:

1. Check if the task was too broad
2. Retry with more specific scope
3. Fall back to sequential execution
4. Report partial results

### Timeout Management

- Set appropriate timeouts per task complexity
- Use background agents for long-running tasks
- Check progress periodically
- Have fallback for stuck agents
