---
name: finishing-a-branch
description: Use when implementation is complete and tests pass — guides the final step of integrating work via merge, PR, or cleanup. Presents structured options and handles chosen workflow including worktree cleanup.
---

# Finishing a Branch

Drive the completion of feature work — merge locally, open a PR, keep the branch, or discard.

## Step 1: Verify Tests

Run the project test suite. If tests fail, stop — fix before proceeding. See the `verification-before-completion` skill if uncertain.

## Step 2: Present Options

```
Implementation complete. What would you like to do?

1. Merge to <base-branch> locally
2. Push and create a Pull Request
3. Keep branch as-is (handle later)
4. Discard this work

Which option?
```

## Step 3: Execute

### Option 1 — Merge locally

```bash
git checkout <base-branch> && git pull
git merge <feature-branch>
# run tests on merged result
git branch -d <feature-branch>
```

Then remove the worktree if applicable.

### Option 2 — Push and PR

```bash
git push -u origin <feature-branch>
gh pr create --title "<title>" --body "..."
```

PR body must include:

- Summary (2–3 bullets)
- Test plan with checkboxes

If the test plan has post-merge steps → extract them to `plans/reviews/post-merge/<branch-slug>.md` so they aren't lost when the PR description archives.

Keep the worktree until the PR merges.

### Option 3 — Keep as-is

Report branch and worktree location. Do nothing.

### Option 4 — Discard

Require typed `discard` confirmation. Then:

```bash
git checkout <base-branch>
git branch -D <feature-branch>
```

Then remove the worktree.

## Worktree Cleanup

Remove for Options 1 and 4. Keep for Options 2 and 3.

```bash
wt remove <feature-branch>
```

## Rules

- Never proceed past Step 1 with failing tests
- Never delete work without typed `discard` confirmation
- Never force-push without explicit request
- Always extract post-merge test plans to `plans/reviews/post-merge/` — do not leave them only in the PR description
