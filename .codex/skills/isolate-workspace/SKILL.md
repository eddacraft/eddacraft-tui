---
name: isolate-workspace
description: >-
  Create or confirm an owned non-default branch and workspace before writes.
  Prefer Worktrunk or native worktree tools; fall back to git worktree. Always
  run dependency install and a baseline after create or switch — native tools
  do not replace setup. Use at the start of implementation in the development loop.
---

# Isolate workspace

Establish a safe write surface. Never implement on the protected/default branch.

## When

- About to implement a ReadyItem.
- `dev-loop-core` Isolate step.
- Parallel writers, module claims, or autonomous runs (isolation required).

## Hard rules

1. **Never write on the integration/default branch.**
2. Prefer **native tools** (Worktrunk / harness worktree) over raw `git worktree add`.
3. Detect **existing isolation** before creating another worktree.
4. Project-local worktree dirs must be **gitignored**.
5. **Always run Setup (step 3) after create or switch** — including after
   `wt switch --create`, `wt add`, harness worktree tools, or `git worktree add`.
   Native tools do **not** install dependencies unless a project hook does.
6. Establish a **baseline** (tests or documented inherited failures) before build.
7. Do not skip Setup because "the main tree already has node_modules".
8. If the harness or MCP write gate trusts only the main checkout, record
   `write-gate: degraded` and use content-mode validation. Do not pretend
   patch-mode edits are available from an untrusted worktree.

## Steps

### 1. Detect current state

```bash
GIT_DIR=$(cd "$(git rev-parse --git-dir)" 2>/dev/null && pwd -P)
GIT_COMMON=$(cd "$(git rev-parse --git-common-dir)" 2>/dev/null && pwd -P)
BRANCH=$(git branch --show-current)
git rev-parse --show-superproject-working-tree 2>/dev/null  # submodule guard
```

| State                                                                          | Action                                                    |
| ------------------------------------------------------------------------------ | --------------------------------------------------------- |
| Already on owned feature branch, clean tree, no parallel writer, policy allows | Reuse path; still run Setup if deps missing               |
| Linked worktree (`GIT_DIR != GIT_COMMON`, not submodule)                       | Reuse; do not nest; run Setup if first entry this session |
| Default/protected branch or dirty unrelated work                               | Create isolation                                          |
| Module / autonomous / parallel (per policy)                                    | Always dedicated worktree + branch                        |

### 2. Create isolation (priority)

**Start point:** integration branch tip **unless** Resolve recorded a **stack
base** (unmerged dependency branch). When stacking, create/branch from
`<dependency-branch>` tip and record `base: <dependency-branch>` for `land-branch`.

1. **Worktrunk** if available (`wt` / project Worktrunk config), e.g.
   `wt switch --create <branch>` or equivalent (from the correct start point).
2. **Harness native** worktree tool if present.
3. **Git worktree fallback:**

Directory priority: `.worktrees/` → `worktrees/` → project docs preference → ask.

```bash
git check-ignore -q .worktrees || git check-ignore -q worktrees
# If not ignored: add to .gitignore and commit that bookkeeping first

git worktree add <path>/<branch-name> -b <branch-name>
cd <path>/<branch-name>
```

Branch names follow project convention (`feat/`, `fix/`, `docs/`, `chore/`).

Optional: project Worktrunk `post-start` hooks may automate Setup. **Do not
assume they exist** — always verify Setup yourself (step 3).

### 3. Setup (mandatory after create or first use of a worktree)

From the **worktree cwd** (not the main checkout):

```bash
# Node / JS
if [ -f package.json ]; then
  if [ -f pnpm-lock.yaml ]; then pnpm install
  elif [ -f yarn.lock ]; then yarn install
  elif [ -f package-lock.json ]; then npm ci || npm install
  else npm install
  fi
fi

# Rust
[ -f Cargo.toml ] && cargo fetch

# Python
[ -f requirements.txt ] && pip install -r requirements.txt
[ -f pyproject.toml ] && (poetry install 2>/dev/null || pip install -e . 2>/dev/null || true)

# Go
[ -f go.mod ] && go mod download
```

**Verification:** for JS worktrees, `test -d node_modules` (or project equivalent)
must pass before baseline. If typecheck/tests fail with "cannot find module",
re-run Setup — do not treat as product defects.

### 4. Baseline

Run the project test suite, the ReadyItem smoke subset, or policy-declared gates.

- Pass → proceed.
- Fail → report; ask whether to proceed with **inherited failures** recorded, or stop.

### 4b. Write-gate probe

If the runtime exposes a write gate or patch validator (for example an MCP tool
bound to a trusted workspace root), confirm it accepts the selected worktree path
before relying on patch-mode validation.

| Result                                   | Action                                                                  |
| ---------------------------------------- | ----------------------------------------------------------------------- |
| Worktree accepted                        | Record `write-gate: patch-mode` and continue                            |
| Worktree rejected but content reads work | Record `write-gate: content-mode`; validate by reading files + commands |
| Worktree rejected and validation blocked | Stop; ask to register the worktree path or choose another workspace     |

Do not abandon isolation just to satisfy a main-checkout-only write gate. The
degradation belongs in the report and checkpoint.

### 5. Report

```text
Workspace: <path>
Branch: <name>
Base: <integration-branch @ sha>
Setup: ran | skipped-reuse-with-deps
Baseline: green | inherited-failures (<summary>)
Write gate: patch-mode | content-mode | blocked
ReadyItem: <id>
```

## Exit

```markdown
## Exit

- Decision: isolated | reused | blocked
- Next: build-tdd | stop
- Notes: <path, branch, setup, baseline>
- Notes: <path, branch, setup, baseline, write-gate>
```

## Non-goals

- Not implementation, commit, or PR open.
- Not claim protocol (orchestrator / `dev-loop-core` owns claims).
- Cleanup after land is `land-branch`.
