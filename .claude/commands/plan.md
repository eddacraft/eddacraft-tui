---
name: plan
description: Create a detailed implementation plan for a feature or task
---

# Create Implementation Plan

## Task to Plan

$ARGUMENTS

## Instructions

Create a detailed, actionable implementation plan following this structure:

### 1. Requirements Analysis

- Understand the goal completely
- Identify constraints and dependencies
- Surface any assumptions

### 2. Break Down into Tasks

Each task should:

- Take 2-5 minutes to complete
- Have a clear, verifiable deliverable
- Include the specific file(s) to modify
- Specify how to verify completion

### 3. Sequence Tasks

- Order by dependencies
- Identify what can be parallelized
- Note critical path items

### 4. Risk Assessment

- What could go wrong?
- How do we mitigate risks?
- What's the rollback plan?

## Output Format

```markdown
# Implementation Plan: [Title]

## Overview

Brief description

## Prerequisites

- [ ] Item 1
- [ ] Item 2

## Phase 1: [Name]

### Task 1.1: [Title]

- **File(s)**: path/to/file
- **Action**: What to do
- **Verification**: How to confirm done

### Task 1.2: [Title]

...

## Phase 2: [Name]

...

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |

## Success Criteria

- [ ] Criterion 1
- [ ] Criterion 2
```

Use the TodoWrite tool to track these tasks as you create them.
