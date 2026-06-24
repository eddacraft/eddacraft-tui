---
name: systematic-debugging
description: Scientific debugging methodology, root cause analysis, evidence-based problem solving
---

# Systematic Debugging Skill

## Overview

Debug issues using a scientific, evidence-based approach rather than guessing.

## When to Apply

- Production incidents
- Test failures
- Performance issues
- Intermittent bugs
- Integration problems

## The Debugging Process

### 1. REPRODUCE

**Goal**: Reliably trigger the issue

```
- Document exact steps to reproduce
- Note environment (OS, versions, config)
- Determine consistency (always vs. sometimes)
- Create minimal reproduction case
```

**Questions to ask:**

- What are the exact steps?
- Does it happen every time?
- When did it start?
- What changed recently?

### 2. ISOLATE

**Goal**: Narrow down the problem location

```
- Use binary search through code/time
- Remove components systematically
- Test in isolation
- Identify the boundary
```

**Techniques:**

- Git bisect for regression
- Comment out code sections
- Add strategic logging
- Use debugger breakpoints

### 3. ANALYZE

**Goal**: Understand what's actually happening

```
- Read the relevant code carefully
- Trace the execution flow
- Check logs and stack traces
- Form multiple hypotheses
```

**Evidence sources:**

- Error messages and stack traces
- Log files
- Metrics and monitoring
- Memory dumps
- Network traces

### 4. HYPOTHESIZE

**Goal**: Generate possible explanations

```
For each hypothesis:
1. What would cause this behaviour?
2. What evidence supports it?
3. What evidence contradicts it?
4. How can we test it?
```

**Common root causes:**

- State management issues
- Race conditions
- Resource exhaustion
- Configuration errors
- External dependencies
- Data corruption

### 5. TEST

**Goal**: Verify or refute hypotheses

```
- Test ONE hypothesis at a time
- Make predictions before testing
- Record all observations
- Update understanding based on results
```

**Testing methods:**

- Add targeted logging
- Use debugger
- Write test cases
- Modify inputs
- Check boundaries

### 6. FIX

**Goal**: Address the root cause

```
- Fix the actual problem, not symptoms
- Consider side effects
- Add regression test
- Document the fix
```

**Verification:**

- Original issue resolved
- No regressions introduced
- Test passes
- Edge cases handled

## Debugging Toolkit

### Logging Strategies

```python
# Structured logging for debugging
logger.debug("Function called", extra={
    "function": "process_data",
    "input_size": len(data),
    "timestamp": datetime.now().isoformat()
})
```

### Binary Search Template

```
1. Find a known good state (commit, date, config)
2. Find the known bad state
3. Test the midpoint
4. Narrow to half with the bug
5. Repeat until found
```

### Common Commands

```bash
# Search logs for errors
grep -r "ERROR\|Exception\|Failed" logs/

# Check recent changes
git log --oneline --since="1 week ago"

# Find what changed
git diff HEAD~10..HEAD -- path/to/file

# Check process state
ps aux | grep process_name
lsof -p PID
```

## Anti-Patterns to Avoid

1. **Shotgun debugging**: Making random changes hoping something works
2. **Print statement chaos**: Adding logs everywhere without strategy
3. **Blame shifting**: Assuming it's someone else's code
4. **Ignoring evidence**: Dismissing data that contradicts assumptions
5. **Fixing symptoms**: Addressing surface issues not root cause

## Post-Mortem Template

```markdown
## Incident: [Title]

### Summary

Brief description of what happened

### Timeline

- HH:MM: Event 1
- HH:MM: Event 2

### Root Cause

Why it happened

### Resolution

How it was fixed

### Lessons Learned

- What we learned
- What to do differently

### Action Items

- [ ] Prevent recurrence
- [ ] Improve detection
- [ ] Update documentation
```
