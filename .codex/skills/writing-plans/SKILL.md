---
name: writing-plans
description: Use when you have an approved spec or requirements for a multi-step task, before touching code. Produces a detailed implementation plan with bite-sized TDD steps, exact file paths, and exact commands.
---

# Writing Plans

Produce a detailed, self-contained implementation plan from an approved spec.

## Plan Header (required)

Every plan must start with:

```markdown
# [Feature Name] Implementation Plan

**Goal:** [One sentence]
**Architecture:** [2–3 sentences]
**Tech Stack:** [Key technologies]

---
```

## File Map

Before tasks, list every file that will be created or modified and its single responsibility. This locks in decomposition decisions.

## Task Structure

Each task produces working, testable, committable code.

```markdown
### Task N: [Name]

**Files:**

- Create: `exact/path/to/file.ts`
- Modify: `exact/path/to/existing.ts`
- Test: `exact/path/to/file.test.ts`

- [ ] Write failing test
- [ ] Run test — verify it fails
- [ ] Write minimal implementation
- [ ] Run test — verify it passes
- [ ] Commit: `git commit -m "feat(scope): description"`
```

## Rules

- Exact file paths always — no vague references
- Complete code in the plan, not "add validation here"
- Exact commands with expected output
- DRY, YAGNI, TDD, frequent commits
- If scope spans multiple independent subsystems, split into separate plans

## Plan Location

Save to `plans/execution/YYYY-MM-DD-<feature>.md` (or the project's convention). Commit it.

## Execution Handoff

After saving the plan, offer two paths:

> "Plan saved. Execute in this session (inline) or split into independent worktrees for parallel execution?"

- **Inline** — work through tasks in this session, committing after each green test
- **Parallel** — if tasks are independent, see the `parallel-agents` skill for fanning out across worktrees / sessions
