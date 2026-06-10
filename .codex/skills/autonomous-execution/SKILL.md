---
name: autonomous-execution
description: Long-running autonomous tasks, checkpointing, error recovery, progress tracking
---

# Autonomous Execution

## Overview

Execute complex, multi-step tasks autonomously with checkpointing, error handling, and progress reporting.

## When to Apply

- Large-scale refactoring
- Codebase migrations
- Multi-file feature implementation
- Automated testing and fixing
- Batch operations

## Execution Framework

### 1. Planning Phase

Before autonomous execution:

```
1. Define a clear end state
2. Break into atomic tasks
3. Identify checkpoints
4. Plan rollback strategy
5. Set timeout limits
```

### 2. Execution Loop

```
while not complete and not blocked:
    task = get_next_task()
    log_start(task)
    create_checkpoint()

    try:
        result = execute(task)
        verify(result)
        mark_complete(task)
    except TransientError:
        retry_with_backoff(task)
    except CriticalError:
        rollback_to_checkpoint()
        escalate_to_user()

    report_status()
```

### 3. Checkpointing Strategy

**What to checkpoint:**

- Completed tasks
- File modifications
- State variables
- Configuration changes

**When to checkpoint:**

- After each significant change
- Before risky operations
- At phase boundaries
- Periodically (every N minutes)

A simple way to checkpoint code state is `git commit` after each verified step. Use a feature branch and revert if needed.

### 4. Progress Reporting

```
[STARTING] Task: Description
[PROGRESS] 5/20 tasks complete (25%)
[SUCCESS] Completed: What was done
[WARNING] Issue encountered: Description (continuing)
[ERROR] Failed: Description (retrying)
[BLOCKED] Need input: Question for user
[COMPLETE] All tasks finished successfully
```

## Error Handling Patterns

### Transient Errors

```
Retry strategy:
- Attempt 1: Immediate retry
- Attempt 2: Wait 2 seconds
- Attempt 3: Wait 4 seconds
- Attempt 4: Wait 8 seconds
- Give up: Escalate or skip
```

### Critical Errors

```
Response:
1. Stop execution
2. Roll back to last checkpoint (e.g. `git reset --hard <sha>`)
3. Log full error context
4. Notify user
5. Await instruction
```

### Partial Failures

```
Options:
- Continue with remaining tasks
- Mark task as skipped
- Attempt alternative approach
- Escalate with partial results
```

## State Management

Persist state between long-running steps so the work can be resumed.

### Example state file

```json
{
  "session_id": "uuid",
  "started_at": "ISO timestamp",
  "last_checkpoint": "ISO timestamp",
  "status": "running|paused|completed|failed",
  "tasks": [
    {
      "id": 1,
      "description": "Task description",
      "status": "pending|in_progress|completed|failed|skipped",
      "started_at": null,
      "completed_at": null,
      "result": null,
      "error": null
    }
  ],
  "checkpoints": [
    {
      "id": 1,
      "timestamp": "ISO timestamp",
      "task_id": 5,
      "git_sha": "abc1234"
    }
  ]
}
```

Save it somewhere durable — for example `.opencode/state/<session-id>.json` (add the directory to `.gitignore`).

### Resume from Checkpoint

```
1. Load state file
2. Find last successful checkpoint
3. Restore state (e.g. `git reset --hard <sha>` if appropriate)
4. Continue from next pending task
```

## Safety Guardrails

### Before destructive operations

```
1. Confirm operation is intended
2. Create backup/checkpoint
3. Verify rollback is possible
4. Log the operation
5. Execute with verification
```

### Maximum limits

```
- Max runtime per task: 10 minutes
- Max total runtime: 60 minutes
- Max consecutive failures: 3
- Max file deletions: 10 without confirmation
```

### Protected operations

Always require confirmation for:

- Deleting files
- Modifying configs
- Changing permissions
- External API calls
- Database modifications
