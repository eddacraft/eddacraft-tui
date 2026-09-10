---
name: address-reviews
description: >-
  Fix CI and unresolved review threads on an open pull request: CI first,
  then reply and resolve threads, re-verify, push. Use for PR feedback, red
  checks, or responding to reviewers.
---

# Address reviews

Make an open PR mergeable: green CI, threads handled, base in sync.

## When

- User asks to address PR comments / review feedback.
- `dev-loop` autonomous land is chasing CI or review.
- After `land-branch` leaves a PR open.
- **`dev-loop` Resolve open-PR poll** finds CI red or unresolved review on
  an owned open PR before starting new work.
- `dev-loop` or `land-branch` finds `mergeable: CONFLICTING` or
  `mergeStateStatus: DIRTY`.
- `land-branch` pre-merge probe finds unresolved threads or red CI.
- `land-branch` does **not** invoke this skill for a clean probe, or for
  `BLOCKED` with no unresolved threads.

## Hard rules

1. **CI before comment archaeology.** Red build makes review comments secondary.
2. Fetch threads via **GraphQL** (REST lacks resolution state).
3. **Reply then resolve** every thread you handle.
4. Never `@`-mention bot reviewers (re-triggers reviews) — write the name plain.
5. Re-run **evidence-gate** (and `verify-loop` when policy requires) after code fixes.
6. "Pre-existing CI failure" requires proof the same check fails on the base branch.
7. Never say "tracked as follow-up" without actually tracking it.
8. Never force-push a rebased repair unless repository policy and explicit user
   authority allow it; otherwise merge the base into the head branch.
9. The parent run's repair budget and progress fingerprint bind this skill. If
   the fingerprint repeats or the budget is exhausted, stop and return to the
   loop; do not start another review round.

## Steps

### 1. Identify PR

```console
gh pr view --json number,baseRefName,headRefName,headRefOid,mergeable,mergeStateStatus,url
gh pr checkout <n>   # if not already on the branch
```

If GitHub reports `mergeable: UNKNOWN`, give it one bounded settle window and
query again. Do not infer a clean merge from absent conflict data.

### 2. Repair a conflicting base first

When `mergeable: CONFLICTING` or `mergeStateStatus: DIRTY`, repair the base
before CI: a conflicted PR may not have meaningful checks on the current merge
candidate.

1. Fetch the PR base and confirm the checked-out head SHA still matches GitHub.
2. Resolve the repository's documented merge/rebase preference. For a managed
   stack, use its declared stack tool and preserve dependency bases; never
   flatten the stack with an ordinary integration-branch merge. If policy is
   silent and the PR is not stacked, merge the base into the head branch
   because it preserves published history. Rebase only with explicit
   force-push authority, using `--force-with-lease` after rechecking the remote
   head.
3. Resolve every conflict against the ReadyItem, current base behaviour, and
   accepted specification. If resolution requires a product or ownership
   choice, stop `awaiting-human`; do not pick a side silently.
4. Prove `git ls-files -u` is empty, run `git diff --check`, then run the
   affected validation before committing and pushing the repair.
5. Query `mergeable` and `mergeStateStatus` again after GitHub recomputes them.
   If still conflicting after three focused repairs, return `blocked`.

### 3. CI first

```console
gh pr checks <n>
# on failure: gh run view <id> --log-failed
```

Fix root causes, commit, push, re-check. Cap at 3 focused attempts then escalate.

### 4. Unresolved threads (GraphQL)

Query `reviewThreads`; keep `isResolved: false` only. Copy-paste GraphQL query
and mutation templates live in [`references/graphql.md`](./references/graphql.md).
For each:

| Type              | Action                                   |
| ----------------- | ---------------------------------------- |
| Suggestion block  | Apply if sound                           |
| Change request    | Fix or push back with technical reason   |
| Question          | Answer in-thread                         |
| Nit               | Fix                                      |
| Outdated          | Confirm; reply that it no longer applies |
| Reviewer conflict | Ask the user; do not pick a side         |

### 5. Reply + resolve

- Reply with REST: `POST .../pulls/{n}/comments/{databaseId}/replies`
- Resolve with GraphQL: `resolveReviewThread`

### 6. Push and optional summary

Push commits. Post a summary comment **only if code changed**, separating fixes
from reply-only threads.

### 7. Keep the base current

Before final evidence, re-fetch the base. If it moved, repeat step 2; do not
declare the PR mergeable from an earlier mergeability result.

### 8. Final CI + evidence

```console
gh pr checks <n> --watch
```

Run `evidence-gate` on the new head. For high-risk repairs, `verify-loop` again.

## Exit

```markdown
## Exit

- Decision: review-addressed | blocked | awaiting-human
- Next: land-branch | dev-loop | stop
- Notes: <CI state; open human decisions>
```

## Non-goals

- Not initial implementation of a ReadyItem.
- Not opening the first PR (`land-branch`).
