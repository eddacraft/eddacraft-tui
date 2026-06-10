---
name: parallel-agents
description: Coordinate multiple parallel work streams when tasks are independent — using git worktrees, background processes, and separate opencode sessions
---

# Parallel Agents

## Overview

Run independent tasks concurrently to compress wall-clock time on large refactors, multi-domain audits, or comprehensive code reviews. opencode's primitives for parallelism are **separate sessions**, **git worktrees**, and **background shell processes** — not in-process subagent dispatch.

## When to apply

- Large codebase exploration spanning unrelated subsystems
- Multi-file refactoring where edits don't touch shared files
- Parallel testing across multiple test suites or matrices
- Comprehensive code review along independent dimensions (security, performance, style)
- Research that fans out across multiple sources

## When NOT to apply

- Tasks that share mutable state (same files, same DB schema migrations)
- Tasks with strict ordering dependencies
- Simple short tasks where coordination overhead exceeds the win

## Coordination patterns

### 1. Fan-out across worktrees

Use when tasks edit code on independent branches.

```
main repo
├── worktrees/feat-auth      ← session A: refactor auth
├── worktrees/feat-billing   ← session B: refactor billing
└── worktrees/feat-api       ← session C: refactor API
```

Setup:

```bash
git worktree add worktrees/feat-auth     -b feat/auth-refactor
git worktree add worktrees/feat-billing  -b feat/billing-refactor
git worktree add worktrees/feat-api      -b feat/api-refactor
```

Then start an opencode session in each worktree (e.g. open three terminals or use a multiplexer like tmux). Each session works independently.

When all are done, merge each branch to the integration branch sequentially and resolve any cross-cutting conflicts on the merge — not during parallel work.

See the `using-git-worktrees` skill for setup details.

### 2. Fan-out via background shell

Use when the parallel work is shell-driven (linters, scanners, builds, exploratory greps) rather than interactive editing.

```bash
# Run independent checks in parallel; capture output per-task
mkdir -p .parallel-out

(eslint . > .parallel-out/lint.log 2>&1) &
(tsc --noEmit > .parallel-out/typecheck.log 2>&1) &
(pnpm test --run > .parallel-out/tests.log 2>&1) &

wait

# Aggregate
grep -E "error|fail" .parallel-out/*.log
```

Use `wait` to block until all background jobs complete. Inspect logs after.

### 3. Pipeline pattern

Use when stages have ordering but each stage has independent units.

```
Stage 1 (parallel): research auth | research billing | research api
                          ↓                ↓               ↓
Stage 2 (sequential):       synthesize findings
                                 ↓
Stage 3 (sequential):       implement
```

Run Stage 1 with one of the patterns above, gather all output, then drive Stage 2 from a single session.

### 4. Specialist pattern (multiple sessions, one task each)

Use when different expertise is needed for different aspects of the same code.

Open separate sessions targeting the same target with different briefs:

- Session A: security review of `src/auth/` — output to `review/security.md`
- Session B: performance review of `src/auth/` — output to `review/performance.md`
- Session C: code-quality review of `src/auth/` — output to `review/quality.md`

Aggregate the markdown outputs from a coordinator session.

## Best practices

### Task definition

Each parallel task should be:

- **Independent** — no shared mutable state with other parallel tasks
- **Scoped** — explicit list of files or directories
- **Well-defined output** — agreed file path, log path, or branch name to read back from
- **Bounded** — a clear definition of "done" so the task can finish without waiting on a peer

### Resource management

- Cap concurrency to `nproc` or below for CPU-heavy tasks
- Watch RAM when running multiple typecheckers / language servers simultaneously
- Respect API rate limits if tasks call external services
- Use one git worktree per branch — never have two sessions editing the same working tree

### Result aggregation

After parallel tasks complete:

1. Collect all outputs (logs, branches, files)
2. Identify conflicts or overlaps
3. Synthesise into a unified view
4. Resolve contradictions explicitly — don't silently pick one

### Explicit handoff files

Use convention-driven file paths so a coordinator session can find the outputs without out-of-band coordination:

```
.parallel-out/<task-name>.log
review/<dimension>.md
plans/findings/<task-id>.json
```

## Example: comprehensive code review across worktrees

```bash
# 1. Create three worktrees pinned to the same SHA
SHA=$(git rev-parse HEAD)
for dim in security performance style; do
  git worktree add "worktrees/review-$dim" "$SHA"
done

# 2. Open an OpenCode session in each. Each session is told:
#    "Review src/ for <dimension>. Write findings to review/<dimension>.md"

# 3. Coordinator session reads back:
cat worktrees/review-security/review/security.md \
    worktrees/review-performance/review/performance.md \
    worktrees/review-style/review/style.md \
  > review/combined.md
```

## Error handling

### A parallel task fails

1. Check whether the task scope was too broad — split it
2. Retry with a tighter scope or after fixing the underlying issue
3. Fall back to sequential execution if a task isn't recoverable
4. Always report partial results — don't pretend a failed task didn't run

### Stuck or runaway processes

- Set timeouts on background commands: `timeout 600 <cmd> &`
- Track PIDs (`echo $! >> .parallel-out/pids`) so you can kill stragglers
- Use `wait -n` to react to whichever job finishes first when needed

## Rules

- Do not run two interactive editing sessions inside the same working tree
- Always capture per-task output to a known path so results survive after the session exits
- Never assume parallel tasks finished correctly — verify each output before aggregating
