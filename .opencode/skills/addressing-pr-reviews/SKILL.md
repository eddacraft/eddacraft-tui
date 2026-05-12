---
name: addressing-pr-reviews
description:
  Use after opening an Anvil PR, or when asked to address PR comments, CI
  failures, Copilot feedback, or merge conflicts. Always run after PR creation
  even when no review comments exist, because it also catches late CI failures.
---

# Addressing PR Reviews

Project-local workflow for `anvil-001` PR remediation. This skill handles CI,
automated review comments, human review comments, base-branch sync, and final PR
readiness.

## When To Use

- Always after opening a PR, after waiting up to 10 minutes for Copilot and
  automated reviewers to complete or time out.
- When the user asks to address PR comments, fix review feedback, resolve
  conflicts, or handle failing PR checks.
- When CI fails late, even if there are no review comments.

## Anvil Rules

1. Use `gh` for all GitHub operations.
2. Fix failing CI first, then automated review comments, then human review
   comments.
3. Use the PR's base branch from `gh pr view`; Anvil PRs normally target `main`.
4. Prefer `git rebase origin/$BASE_BRANCH` for branch catch-up and conflict
   resolution; push rebased PR branches with `git push --force-with-lease`.
5. Never `@`-mention Copilot or other bots in replies, summaries, commits, or PR
   comments. Write `copilot`, not `@copilot`.
6. Do not claim a CI failure is unrelated or pre-existing without proving the
   same check fails on the PR's base branch.
7. If remediation changes `docs/**`, `plans/**`, README files, skill docs, or
   runbooks, perform the docs closeout required by `AGENTS.md`.
8. If post-merge verification is needed, extract it to
   `plans/reviews/post-merge/<branch-slug>.md`; do not leave it only in the PR
   body.

## Workflow

### 1. Identify The PR

Use the PR number from the user when provided. Otherwise detect it from the
current branch:

```bash
PR_NUMBER=$(gh pr view --json number -q '.number')
BASE_BRANCH=$(gh pr view "$PR_NUMBER" --json baseRefName -q '.baseRefName')
OWNER=$(gh repo view --json owner -q '.owner.login')
REPO=$(gh repo view --json name -q '.name')
```

If not already on the PR branch, check it out:

```bash
gh pr checkout "$PR_NUMBER"
```

### 2. Check CI First

Run:

```bash
gh pr checks "$PR_NUMBER"
```

If checks are pending, wait unless the user asked for a quick status only:

```bash
gh pr checks "$PR_NUMBER" --watch --fail-fast
```

For failures, inspect logs before editing:

```bash
gh run view <RUN_ID> --log-failed
```

Fix the root cause, run targeted local validation, commit, push, and re-check.
Repeat until CI passes or you have evidence that the failure also occurs on
`main`.

### 3. Fetch Review Threads

Use GraphQL so unresolved/resolved state and thread IDs are available. REST does
not provide enough data to resolve threads.

```bash
gh api graphql -f query='
query($owner: String!, $repo: String!, $pr: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $pr) {
      reviewThreads(first: 100) {
        nodes {
          id
          isResolved
          isOutdated
          path
          line
          comments(first: 50) {
            nodes {
              id
              databaseId
              body
              author { login }
              createdAt
            }
          }
        }
      }
    }
  }
}' -f owner="$OWNER" -f repo="$REPO" -F pr="$PR_NUMBER"
```

Only act on threads where `isResolved` is `false`.

### 4. Address Comments

Classify each unresolved thread:

| Type                  | Action                                              |
| --------------------- | --------------------------------------------------- |
| CI failure            | Fix before review comments                          |
| Suggested change      | Apply if correct, otherwise explain why not         |
| Change request        | Fix or push back with concrete technical reasoning  |
| Question              | Reply with a direct explanation and code references |
| Nitpick               | Usually fix without debate                          |
| Outdated              | Verify current code, reply with what changed        |
| Conflicting reviewers | Stop and ask the user which direction to take       |

When code changes are needed, make the smallest correct change and run targeted
validation before committing.

### 5. Reply And Resolve Threads

Reply to the top-level review comment using its `databaseId`:

```bash
gh api \
  --method POST \
  "repos/$OWNER/$REPO/pulls/$PR_NUMBER/comments/$COMMENT_DATABASE_ID/replies" \
  -f body="Fixed in the latest push."
```

Resolve the GraphQL thread after replying:

```bash
gh api graphql -f query='
mutation($threadId: ID!) {
  resolveReviewThread(input: { threadId: $threadId }) {
    thread { isResolved }
  }
}' -f threadId="$THREAD_NODE_ID"
```

Resolve every addressed thread, whether the response was a code change or an
explanation.

### 6. Sync With Base By Rebase

Anvil prefers rebasing PR branches onto the latest base branch before merge.

```bash
git fetch origin "$BASE_BRANCH"
git rebase "origin/$BASE_BRANCH"
```

If conflicts occur:

- Inspect with `git status --short` and `git diff --name-only --diff-filter=U`.
- Resolve conflict markers in each file.
- Check for remaining markers with
  `rg -n '^(<<<<<<<|=======|>>>>>>>|\|\|\|\|\|\|\|)' --glob '!.git/**' .`.
- Run relevant validation.
- Continue with `GIT_EDITOR=true git rebase --continue` when the commit message
  editor cannot open in a headless session.

After a successful rebase:

```bash
git push --force-with-lease
```

### 7. Final CI And Review State

Run the final checks:

```bash
gh pr checks "$PR_NUMBER" --watch --fail-fast
gh pr view "$PR_NUMBER" --json mergeable,mergeStateStatus,reviewDecision,reviews,comments,url
```

Do not finish while checks are pending or red unless the user explicitly accepts
that state after you present the evidence.

### 8. Summary Comment

Post a PR summary comment only when code or docs changed during remediation.
Keep replies and fixes separate. Do not tag bots.

```bash
gh pr comment "$PR_NUMBER" --body "$(cat <<'EOF'
## Review Remediation
- Fixed failing CI in <area>.
- Addressed review feedback in <files>.

## Validation
- <command>: passed
EOF
)"
```

## Final Response

Report:

- PR URL
- Commits pushed
- CI status and evidence
- Threads resolved or outstanding
- Any residual risks or user decisions needed
- Docs Closeout note when documentation changed
