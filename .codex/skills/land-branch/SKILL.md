---
name: land-branch
description: >-
  Land verified work: commit if needed, open or update a PR (including stacked
  bases when dependencies are unmerged), merge when authorised, or keep/discard
  with confirmation. Owns finish policy, APS reconcile on land, branch deletion,
  and integration-ancestor verification after merge.
---

# Land branch

Finish implementation only after evidence. Present clear options; execute the
chosen path safely. **Land is incomplete without APS reconcile when APS exists.**

**`MERGED` on GitHub is not enough.** A PR can merge into a **dead stack base**
and never reach the integration branch. Always prove commits are ancestors of
integration before APS `Merged` / outcome `integrated`.

## When

- `evidence-gate` (and `verify-loop` when required) supports the claim.
- Interactive finish after a feature branch.
- `dev-loop-core` Review and land step (interactive → review-ready; autonomous → merge if authorised).

## Hard rules

1. **No land with failing required gates** (unless inherited failures are documented and accepted).
2. **Never force-push** unless the user explicitly requests it.
3. **Never discard** without typed `discard` confirmation.
4. **Never implement on the default branch** to "finish faster" (except policy-blessed out-of-band bookkeeping — see policy).
5. Lockfiles and generated artefacts stay **atomic** with the change that caused them.
6. **Bookkeeping vs feature work:**
   - Unrelated APS/index bookkeeping ships as **standalone** PRs unless policy
     allows out-of-band bookkeeping on integration.
   - **Exception (required):** status/Files/evidence updates for the **current
     ReadyItem** after land ship with the feature PR or immediate same-branch
     follow-up before `review-ready` / `integrated`.
7. Autonomous merge only when `dev-loop-core` policy + branch protection + invocation authority allow it.
8. **Reconcile is mandatory** when `plans/index.aps.md` exists (step 6). Skipping it is a land failure.
9. **PR base is not always the integration branch.** Stacked work uses step 3b.
10. **Delete head branch on merge** (step 4d). Prefer repo setting
    "Automatically delete head branches"; when merging via `gh`, pass
    `--delete-branch`. Stack retargeting must not rely on hope.
11. **Integration-ancestor gate** (step 4e): before APS `Merged` or `integrated`,
    prove the landed commit is an ancestor of integration. Never trust PR state
    alone.
12. Process asks in PR bodies are advisory only. Prefer machine-enforced repo
    settings, CI gates, branch protection, and CLI flags for invariants.

## Steps

### 1. Pre-flight

Confirm evidence block is fresh. If missing or stale → `evidence-gate` first.

```bash
INTEGRATION="${INTEGRATION:-main}"  # from policy devLoop.integrationBranch
git fetch origin "$INTEGRATION" 2>/dev/null || true
```

Decide **PR base branch**:

| Situation                          | Base                             |
| ---------------------------------- | -------------------------------- |
| No unmerged dependencies           | Integration branch               |
| Depends on unmerged branch/PR      | That dependency branch (stacked) |
| Depends on multiple unmerged lines | Escalate                         |

### 2. Commit remaining work (if dirty)

```text
<type>(<scope>): <imperative summary>

[optional body]

APS: <ITEM-ID>
```

No secrets. No `--no-verify` unless explicitly requested.

### 3. Present options (interactive)

```text
Implementation verified. Next step?

1. Merge to <base> locally
2. Push and open/update Pull Request (base: <base-branch>)
3. Keep branch as-is
4. Discard this work (type discard)

Which option?
```

Show the **resolved base**. Autonomous: skip menu; open/update PR; merge only if
authorised; else `awaiting-merge-authority` / `review-ready`.

### 3b. Dependent work on an unmerged branch

| Path         | When                                         | Action                                                                      |
| ------------ | -------------------------------------------- | --------------------------------------------------------------------------- |
| **Stack**    | Dep open, CI green enough, stacks allowed    | Branch from dep tip; PR `--base <dep-branch>`; merge order + retarget notes |
| **Wait**     | Dep unstable / contested / stacks disallowed | Park this item; other Ready work                                            |
| **Escalate** | Multi-parent or product risk                 | Ask user                                                                    |

**Stack procedure:**

1. Confirm dependency PR, branch, head SHA.
2. Feature branch based on that head.
3. `gh pr create --base <dependency-branch>`.
4. PR body **Stack** section:

```markdown
## Stack

- Depends on: #<dep-pr> (`<dep-branch>` @ <sha>)
- Merge order: merge #<dep-pr> **into integration** first (not only into another stack base)
- After dep is on integration: retarget this PR base to `<integration>`, rebase, re-CI
- Enable **delete head branch on merge** for every PR in the stack
- Do not treat this PR as integrated until its commits are ancestors of `<integration>`
```

5. APS: this item `In Progress`; do not mark dep `Merged` until integration-ancestor check passes for the dep.
6. **After dep merges (or appears MERGED):** run step 4e on the **dependency**
   first. If dep is not on integration, **do not** retarget as if it were —
   repair the stack (recreate base, escalate).

### 4. Execute

**Merge locally into resolved base** only when that base is intentional. Local
merge of a stack into integration without the dep is forbidden.

**PR:**

```bash
git push -u origin <feature-branch>
gh pr create --base <base-branch> --title "<title>" --body "..."
```

After creating a PR, give GitHub checks a short settle window before watching.
Immediate `gh pr checks --watch` can race check discovery and return “no checks
reported”.

```bash
sleep 10
gh pr checks <n> --watch
```

PR body: Summary, Test plan (checked only after real runs), APS Work Items,
Stack section if needed, post-merge notes, reconcile disclosure if included.

### 4d. Delete branch on merge

When **you** merge:

```bash
gh pr merge <n> --merge|--squash|--rebase --delete-branch
```

(Use the project’s merge method.) If `--delete-branch` is unavailable, delete
explicitly after merge:

```bash
git push origin --delete <head-branch> 2>/dev/null || true
```

**Recommend once per repo** (durable fix): GitHub → Settings → General →
**Automatically delete head branches**. A PR-body request to delete branches is
not a control; use the setting or `--delete-branch`.

When a **human** merges without deleting the head branch, stacks that used that
branch as base can silently point at a zombie. On resume, step 4e + stack repair
are mandatory.

### 4e. Integration-ancestor verification (mandatory for `Merged` / `integrated`)

GitHub `MERGED` only means “merged into **its base**.” Prove presence on
**integration**:

```bash
INTEGRATION="${INTEGRATION:-main}"
git fetch origin "$INTEGRATION" "<head-sha-or-branch>" 2>/dev/null || true

# Prefer merge commit or PR head SHA from gh:
HEAD_SHA=$(gh pr view <n> --json mergeCommit,headRefOid -q '.mergeCommit.oid // .headRefOid')

git merge-base --is-ancestor "$HEAD_SHA" "origin/$INTEGRATION"
echo "ancestor_exit=$?"   # 0 = on integration
```

Optional file/content probe when SHAs were rewritten by squash/rebase-merge:

```bash
# After fetch: expect distinctive change from the PR to exist on integration
git log -1 --oneline "origin/$INTEGRATION" -- <path-touched-by-pr>
# or: git show origin/$INTEGRATION:<path> | head
```

| Result                                                     | Action                                                                                                                                           |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Ancestor of integration (or content proven on integration) | May set APS `Merged`; may return `integrated`                                                                                                    |
| PR `MERGED` but **not** ancestor of integration            | **Do not** set APS `Merged`. Treat as stack/base failure: retarget, re-merge into integration, or escalate. Notes: `merged-into-non-integration` |
| Squash/rebase rewrite, SHA check fails                     | Use content/file probe; if still missing, not integrated                                                                                         |

For **stacked children** after a dep “merged”: re-run 4e on the dep **before**
retargeting the child to integration.

### 5. Worktree cleanup

Only for merge-local and discard, and only for worktrees this session created.
`cd` to main repo root before `git worktree remove`.

**After rebase-merge / squash:** Worktrunk may still flag a branch as unmerged
(SHA-based). If GitHub says merged **and** 4e passes, remove the worktree/branch
with an explicit delete (`git branch -D` / worktrunk force) — do not reopen work.

**Local integration dirty/unpushed:** before basing new work on local `main`,
`git fetch` + reconcile with `origin/main` (stash/rebase/push as needed). Avoid
committing skill-pack or bookkeeping edits directly on local `main`; use a branch
or pull before committing. Never stack on a divergent local-only integration tip
without noting it.

### 6. APS reconcile (mandatory when APS exists)

1. **Only after step 4e passes** for merge-into-integration:
   set item status `Merged` (not `Complete`).
2. PR open only (incl. stacked, not yet on integration): `In Progress`; record PR
   URL, base branch, stack deps; **never** `Merged`.
3. Update `Files:`; attach validation evidence summary.
4. Commit reconcile on the feature branch when still open; after true integration
   merge, see **out-of-band reconcile** in policy if the loop did not own the merge.

**Hard gate:** no `review-ready` / `integrated` while item still shows pre-land
status — and no APS `Merged` without integration-ancestor proof.

### 7. Release claim

Release or transfer claim (`dev-loop-core/references/coordination-module.md`).

## Exit

```markdown
## Exit

- Decision: merged-local | pr-open | stacked-pr-open | kept | discarded | awaiting-merge-authority | blocked | merged-into-non-integration
- Next: address-reviews | dev-loop-core | stop
- Notes: <PR URL; base; stack; delete-branch; ancestor-check; APS; claim>
```

## Non-goals

- Not implementation or redesign.
- Not full GraphQL review workflow (`address-reviews`).
- Not independent verification (`verify-loop`).
