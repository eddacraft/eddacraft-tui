---
name: isolate-workspace
description: >-
  Create or confirm an owned non-default branch and workspace before writes.
  Use when starting feature work that needs isolation or whenever the user
  asks for a worktree.
---

# Isolate workspace

Establish a safe write surface. Never implement on the protected/default branch.

## When

- About to implement a ReadyItem.
- `dev-loop` Isolate step.
- Parallel writers, module claims, or autonomous runs (isolation required).

## Hard rules

1. **Never write on the integration/default branch.**
2. Prefer **native tools** (Worktrunk / harness worktree) over raw `git worktree add`.
3. Detect **existing isolation** before creating another worktree.
4. Project-local worktree dirs must be **gitignored**.
5. **Always run Setup (step 3) after create or switch** — including after
   Worktrunk switch/add, harness worktree tools, or `git worktree add`.
   Native tools do **not** install dependencies unless a project hook does.
6. Establish a **baseline** (tests or documented inherited failures) before build.
7. Do not skip Setup because "the main tree already has node_modules".
8. If the harness or MCP write gate trusts only the main checkout, treat that as
   expected trust-boundary behaviour: record `write-gate: degraded` and use
   content-mode validation. Do not pretend patch-mode edits are available from an
   untrusted worktree.
9. Before invoking native workspace tools, prove the target path is under an
   allowed writable root and that read-only/orientation commands do not mutate
   the filesystem. If a tool writes during orientation, mark it unsafe and use a
   non-mutating Git/content fallback.
10. Large monorepos require capacity-aware setup: check free space before copy,
    install, and compile; avoid shared read-only targets; keep caches and temp
    dirs inside writable roots.
11. **Resolve Worktrunk once** before any Worktrunk call. On Windows Winget the
    binary is often `git-wt` because `wt` is Windows Terminal.

## Steps

### 1. Detect current state

```console
git rev-parse --absolute-git-dir
git rev-parse --path-format=absolute --git-common-dir
git branch --show-current
git rev-parse --show-superproject-working-tree
```

Capture the results through the harness. A non-zero final command normally
means the checkout is not a submodule; do not require shell assignment syntax.

| State                                                                          | Action                                                    |
| ------------------------------------------------------------------------------ | --------------------------------------------------------- |
| Already on owned feature branch, clean tree, no parallel writer, policy allows | Reuse path; still run Setup if deps missing               |
| Linked worktree (`GIT_DIR != GIT_COMMON`, not submodule)                       | Reuse; do not nest; run Setup if first entry this session |
| Default/protected branch or dirty unrelated work                               | Create isolation                                          |
| Module / autonomous / parallel (per policy)                                    | Always dedicated worktree + branch                        |

### 2. Create isolation (priority)

**Start point:** integration branch tip **unless** an upstream stage
(dev-loop Resolve) or the ReadyItem records a **stack base**
(`Stack depends on:` — an unmerged dependency branch). When stacking, create/branch from
`<dependency-branch>` tip and record `base: <dependency-branch>` for `land-branch`.

Honour repository policy `devLoop.isolation` when present:

| `provider`     | Behaviour                                                                         |
| -------------- | --------------------------------------------------------------------------------- |
| `worktrunk`    | Resolve Worktrunk below; if unresolved, stop (or fall back only if policy allows) |
| `git-worktree` | Skip Worktrunk; use git worktree (or harness if that is the only option)          |
| `harness`      | Prefer harness-native worktree tools                                              |
| unset          | Auto: Worktrunk if it resolves, else harness, else git worktree                   |

#### 2a. Resolve Worktrunk CLI

Resolve the executable once per isolate run in this order:

1. Policy `devLoop.isolation.command` when set — explicit policy wins; do not
   silently substitute another name if it is missing.
2. Environment `WORKTRUNK_BIN` when set and executable.
3. PATH probe, `git-wt` before `wt`.

Probe each candidate with `--version` or `--help` through the harness and inspect
the output for `worktrunk`. Treat `git-wt` as Worktrunk by name. Reject `wt` or
`wt.exe` when its help identifies Windows Terminal. Record and reuse the exact
resolved executable for create, switch, remove, and later landing; do not rely
on a session-only shell variable. Prefer invoking the executable directly over
`git wt` when directory switching is required.

#### 2b. Create path

1. **Worktrunk** when policy allows and its executable resolved, e.g.
   `<worktrunk> switch --create <branch>` or `<worktrunk> add …`.
2. **Harness native** worktree tool if present.
3. **Git worktree fallback:**

Directory priority: `.worktrees/` → `worktrees/` → project docs preference → ask.

Before creating or accepting a workspace, verify:

- the path is inside a permitted writable root for the harness sandbox;
- the parent filesystem has enough free space for expected dependency install,
  build artefacts, and copy strategy;
- native tooling will not copy the full repository without warning on large trees.

If the tool would copy a large tree, report estimated file count/bytes and ask or
choose a linked worktree strategy. Do not start a 10+ GiB copy silently.

```console
git check-ignore -q .worktrees
git check-ignore -q worktrees
git worktree add <path>/<branch-name> -b <branch-name>
```

At least one ignore probe must succeed before using a project-local worktree
directory. Change the harness working directory to the new path after creation.

Branch names follow project convention (`feat/`, `fix/`, `docs/`, `chore/`).

Optional: project Worktrunk `post-start` hooks may automate Setup. **Do not
assume they exist** — always verify Setup yourself (step 3).

### 3. Setup (mandatory after create or first use of a worktree)

From the **worktree cwd** (not the main checkout):

1. Read repository setup instructions (`AGENTS.md`, `README`, contributor docs,
   or the repo's declared automation) and use `aps-probe` evidence when present.
2. Select one evidenced setup command. Do not infer a package manager when the
   repository has conflicting manifests or lockfiles.
3. Confirm the task, repository policy, or user has authorised dependency
   installation and any required network access. If not, report the exact
   command and request authority before running it.
4. Run the selected command without suppressing failures. A setup failure blocks
   the baseline until repaired or explicitly recorded as an inherited
   environment failure.

Read `references/sandbox-and-setup.md` before selecting a setup command, and
before running in a sandboxed or autonomous harness — it holds the
per-ecosystem detection examples and the hermetic environment defaults.

**Verification:** for JS worktrees, `test -d node_modules` (or project equivalent)
must pass before baseline. If typecheck/tests fail with "cannot find module",
re-run Setup — do not treat as product defects.

### 4. Baseline

Run the project test suite, the ReadyItem smoke subset, or policy-declared gates.

- Pass → proceed.
- Fail → report; ask whether to proceed with **inherited failures** recorded, or stop.

### 4b. Write-gate probe

Before relying on patch-mode validation, run the write-gate probe in
`references/sandbox-and-setup.md` and record the result
(`patch-mode` | `content-mode` | blocked).

### 4c. Read-only tool probe

Before orientation commands in a sandboxed or autonomous harness, apply the
read-only tool probe in `references/sandbox-and-setup.md`.

### 5. Report

```text
Workspace: <path>
Branch: <name>
Base: <integration-branch @ sha>
Isolation provider: worktrunk | git-worktree | harness | auto
Worktrunk CLI: <resolved executable or none>
Setup: ran | skipped-reuse-with-deps
Baseline: green | inherited-failures (<summary>)
Write gate: patch-mode | content-mode | blocked
Environment: hermetic | inherited (<exceptions>)
ReadyItem: <id>
```

## Exit

```markdown
## Exit

- Decision: isolated | reused | blocked
- Next: loop-build-tdd | stop
- Notes: <path, branch, setup, baseline, write-gate, Worktrunk executable>
```

## Non-goals

- Not implementation, commit, or PR open.
- Not claim protocol (the lead / `dev-loop` owns claims).
- Cleanup after land is `land-branch`.
