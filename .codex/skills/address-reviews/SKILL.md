---
name: address-reviews
description: >-
  Fix CI and unresolved PR review threads on an open pull request: CI first,
  then GraphQL threads, reply and resolve, re-verify, push. Use when handling
  review feedback or red checks after land-branch.
---

# Address reviews

Make an open PR mergeable: green CI, threads handled, base in sync.

## When

- User asks to address PR comments / review feedback.
- `dev-loop-core` autonomous land is chasing CI or review.
- After `land-branch` leaves a PR open.
- **`dev-loop-core` Resolve open-PR poll** finds CI red or unresolved review on
  an owned open PR before starting new work.

## Hard rules

1. **CI before comment archaeology.** Red build makes review comments secondary.
2. Fetch threads via **GraphQL** (REST lacks resolution state).
3. **Reply then resolve** every thread you handle.
4. Never `@`-mention bot reviewers (re-triggers reviews) — write the name plain.
5. Re-run **evidence-gate** (and `verify-loop` when policy requires) after code fixes.
6. "Pre-existing CI failure" requires proof the same check fails on the base branch.
7. Never say "tracked as follow-up" without actually tracking it.

## Steps

### 1. Identify PR

```bash
gh pr view --json number,baseRefName,url
gh pr checkout <n>   # if not already on the branch
```

### 2. CI first

```bash
gh pr checks <n>
# on failure: gh run view <id> --log-failed
```

Fix root causes, commit, push, re-check. Cap at 3 focused attempts then escalate.

### 3. Unresolved threads (GraphQL)

Query `reviewThreads`; keep `isResolved: false` only. For each:

| Type              | Action                                   |
| ----------------- | ---------------------------------------- |
| Suggestion block  | Apply if sound                           |
| Change request    | Fix or push back with technical reason   |
| Question          | Answer in-thread                         |
| Nit               | Fix                                      |
| Outdated          | Confirm; reply that it no longer applies |
| Reviewer conflict | Ask the user; do not pick a side         |

### 4. Reply + resolve

- Reply with REST: `POST .../pulls/{n}/comments/{databaseId}/replies`
- Resolve with GraphQL: `resolveReviewThread`

### 5. Push and optional summary

Push commits. Post a summary comment **only if code changed**, separating fixes
from reply-only threads.

### 6. Sync base

Fetch base; rebase or merge per project preference; resolve conflicts fully
(no leftover markers); push.

### 7. Final CI + evidence

```bash
gh pr checks <n> --watch
```

Run `evidence-gate` on the new head. For high-risk repairs, `verify-loop` again.

## Exit

```markdown
## Exit

- Decision: review-addressed | blocked | awaiting-human
- Next: land-branch | dev-loop-core | stop
- Notes: <CI state; open human decisions>
```

## Non-goals

- Not initial implementation of a ReadyItem.
- Not opening the first PR (`land-branch`).
- Deep GraphQL templates live in the older `addressing-pr-reviews` skill if you
  need a full copy-paste reference until this pack is promoted.
