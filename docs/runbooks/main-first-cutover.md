# Main-First Cutover

| Type    | Authority     | Owner   | Status | Freshness                                                                                   |
| ------- | ------------- | ------- | ------ | ------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | OPMODEL | Live   | Last reviewed 2026-05-11 against OPMODEL-012 Phase 2 cutover and `.github/workflows/ci.yml` |

| Upstream                            | Downstream                                         |
| ----------------------------------- | -------------------------------------------------- |
| `.github/workflows/ci.yml`, ADR-049 | release council, on-call operators, OPMODEL module |

> **Owner:** Operator (Josh) + Release council **Scope:** One-shot cutover from
> the current `dev -> main` promotion model to the target main-first model.
> Implements OPMODEL-012. **Inputs:**
> [`plans/audits/2026-05-11-opmodel-012-workflow-audit.md`](../../plans/audits/2026-05-11-opmodel-012-workflow-audit.md),
> [`plans/execution/opmodel-012.steps.md`](../../plans/execution/opmodel-012.steps.md).
> **Companion playbooks:**
> [`branch-reconciliation.md`](./branch-reconciliation.md) (one-time divergence
> recovery — distinct from this cutover),
> [`rollback-bad-main.md`](./rollback-bad-main.md) (post-cutover recovery).

## Purpose

Promote `dev`'s HEAD to `main` via fast-forward, add branch protection on
`main`, delete the `pr-base-guard.yml` workflow that enforces the old model,
restrict or retire `dev`, and verify the new target model is executable. This
playbook is operator-driven; the agent runs commands only when the operator
approves each step in the moment.

## When to use

Once. After OPMODEL-012's Phase 0 PR has merged, when the operator has a cutover
window scheduled and has notified open-PR owners.

Do **not** use this playbook for:

- Routine recovery — see [`rollback-bad-main.md`](./rollback-bad-main.md) and
  the OPMODEL-011 family.
- Re-running the cutover after a partial failure — read the Rollback section
  first; partial-state recovery has different commands.

## Required access

- Repo admin on `eddacraft/anvil-001` (branch protection, default branch).
- Push access to `main` and `dev`.
- `gh` authenticated against `eddacraft/anvil-001`.

## Pre-flight

Before opening the cutover window:

1. Phase 0 PR (#1410) is merged and CICD-012
   (`Main-first cutover readiness for validation workflows`) is Complete.
   CICD-012 settles the validation surface so the cutover doesn't strand
   in-progress CICD work mid-stream.
2. Phase 0 audit is current — re-confirm fast-forward is still clean and the
   open-PR list:

   ```bash
   git fetch origin --prune
   git rev-list --count origin/dev..origin/main      # must be 0
   git rev-list --count origin/main..origin/dev      # records the FF distance
   gh pr list --base dev --state open --json number,title,headRefName
   gh pr list --base main --state open --json number,title,headRefName
   ```

3. Operator has notified each open-PR-against-`dev` owner with a deadline:
   merge-before-cutover or accept retarget-after.
4. Operator has decided the dev retirement disposition (see Step 7 below):
   protect-and-keep, dated compatibility branch (recommended), or delete.
5. **Dependabot awareness.** `.github/dependabot.yml` does not pin a
   `target-branch`, so it tracks the repo default. The moment Step 6 flips the
   default to `main`, all new Dependabot PRs target `main`. Plan the cutover
   window so a scheduled Dependabot run mid-window is acceptable (PRs queue
   against `main` while the channel is still frozen — that is fine, just be
   ready for them).

If `git rev-list --count origin/dev..origin/main` is not `0`, the fast-forward
window has closed. Stop. Decide whether to merge `main` into `dev` first
(restoring the FF window) or to abort and reschedule. Do not force-push `main`
to recover the window.

## Cutover commands

Run in order. Each step has an explicit verification before moving on.

### 1. Open the cutover window

Announce in the team channel that merges to `dev` are frozen. Confirm no
in-flight CI on `dev`:

```bash
gh run list --repo eddacraft/anvil-001 --branch dev --limit 5 \
  --json status,conclusion,workflowName,headSha
```

Wait for any `in_progress` runs to finish or cancel them with operator approval.
Proceed only when `dev` CI is settled.

### 2. Re-check the fast-forward window

```bash
git fetch origin --prune
git rev-list --count origin/dev..origin/main
```

Must print `0`. If not, abort per Pre-flight.

### 3. Fast-forward `main` to `dev`'s HEAD

The repository has no branch protection on `main` at cutover start (confirmed in
Phase 0 audit), so a direct push works. After protection lands in Step 5 this
exact command stops working for everyone except admins — admins retain
direct-push capability because Step 5 deliberately sets `enforce_admins: false`
(see Step 5's note for the recovery rationale). That admin path is the
operator's escape hatch if protection is later misconfigured; it is not a
routine path. All non-emergency changes go through PR review + required checks
per the protection rule.

```bash
DEV_SHA=$(git rev-parse origin/dev)
echo "Fast-forwarding main to ${DEV_SHA}"
git push origin "${DEV_SHA}:refs/heads/main"
```

Verify:

```bash
git fetch origin
git rev-parse origin/main          # must equal DEV_SHA
git rev-parse origin/dev           # must equal DEV_SHA
```

If either rev-parse disagrees with `DEV_SHA`, stop. Do not proceed to Step 4.
Investigate the mismatch — likely cause: someone pushed to `main` or `dev`
between Step 2 and Step 3 (no protection yet).

### 4. Delete `pr-base-guard.yml` and adjust workflows

The `pr-base-guard.yml` workflow rejects feat/fix/docs/chore branches targeting
`main`. After cutover those are exactly the PRs that _must_ target `main`.
Delete the workflow in a small PR (the same PR can also drop `dev` from the 6
cleanup workflows identified in the audit; or split them — operator choice).

```bash
git switch -c chore/opmodel-012-workflow-cleanup origin/main
git rm .github/workflows/pr-base-guard.yml
# Optional: also drop `dev` from ci.yml, codeql.yml, napi.yml,
# release-harness.yml, rust.yml, security.yml per the Phase 0 audit.
git commit -m "chore(ci): retire pr-base-guard after main-first cutover"
git push -u origin chore/opmodel-012-workflow-cleanup
gh pr create --repo eddacraft/anvil-001 --base main \
  --title "chore(ci): retire pr-base-guard after main-first cutover" \
  --body "Per OPMODEL-012 Phase 2; see docs/runbooks/main-first-cutover.md."
```

Expect the **`PR Base Guard` check to fail red** on this PR — the cleanup PR's
head branch is `chore/*`, which the guard rejects. That is intentional and does
not block the merge: no branch protection exists yet at this step, so
required-check enforcement is not on. The operator merges the PR despite the red
check; the next PR after this merges into a clean state.

Merge after one council reviewer approval. Then **record this merge SHA and
timestamp in the operator log — this is the rollback boundary.** After this
merge, the clean rollback path in the Rollback section closes; further failures
move to fix-forward.

Verify the workflow is gone:

```bash
gh api repos/eddacraft/anvil-001/contents/.github/workflows/pr-base-guard.yml \
  2>&1 | grep -q '"message": "Not Found"' && echo "deleted" || echo "still present"
```

Do this **before** Step 5 — if branch protection lands first and
`pr-base-guard.yml` is required, the cleanup PR itself fails the guard.

### 5. Add branch protection on `main`

Use the required-check list from the Phase 0 audit
([`workflow audit`](../../plans/audits/2026-05-11-opmodel-012-workflow-audit.md#required-ci-checks-for-main-branch-protection)).
Confirm the canonical list against a recent code PR before applying.

**Required checks rule:** only include checks that **always run** for the shape
of change you want to gate. A required check that is path-filtered out on a
given PR will block that PR indefinitely. The default list below is the
always-running subset for code PRs; do **not** add Build, Dependency Audit, E2E
Harness, Platform Smoke, Release Gate, SAST, Secret Scan, License Compliance, or
Dependency Audit (PR) to required checks unless you are willing to maintain a
separate "required for code" vs "required for docs" split via repo rulesets
rather than legacy branch protection.

```bash
# Replace REQUIRED_CHECKS with the operator-confirmed list.
REQUIRED_CHECKS='["APS Drift Check","Docs Lint","Lint & Format","Type Check","Unit Tests (Node 22.x, ubuntu-latest)","Security Summary","Detect Changes"]'

gh api -X PUT repos/eddacraft/anvil-001/branches/main/protection \
  -H "Accept: application/vnd.github+json" \
  --input - <<EOF
{
  "required_status_checks": {
    "strict": true,
    "contexts": ${REQUIRED_CHECKS}
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 1,
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF
```

`enforce_admins: false` is a **deliberate** small-team choice so the operator
can recover from misconfigured protection without being locked out. Revisit this
when the team grows beyond a single primary maintainer; flip to `true` once
protection settings are stable and a second maintainer can confirm recovery
paths exist.

Verify:

```bash
gh api repos/eddacraft/anvil-001/branches/main/protection \
  --jq '{required_checks: .required_status_checks.contexts, reviews: .required_pull_request_reviews.required_approving_review_count, force_push: .allow_force_pushes.enabled, deletions: .allow_deletions.enabled}'
```

Expect: required-check list matches `REQUIRED_CHECKS`,
`required_approving_review_count: 1`, `force_push: false`, `deletions: false`.

### 6. Set `main` as repo default branch

```bash
gh api -X PATCH repos/eddacraft/anvil-001 -f default_branch=main
gh api repos/eddacraft/anvil-001 --jq .default_branch     # must print "main"
```

Open a small **non-empty** test PR against `main` to confirm the default-branch
dropdown shows `main` and that the required checks actually pass on a real diff.
An empty commit (`--allow-empty`) does not exercise path-filtered jobs; a
one-line docs touch exercises Docs Lint at minimum:

```bash
git switch -c chore/cutover-smoke origin/main
date -u +"%Y-%m-%dT%H:%M:%SZ" >> docs/runbooks/main-first-cutover-smoke.log
git add docs/runbooks/main-first-cutover-smoke.log
git commit -m "chore: cutover smoke (will close)"
git push -u origin chore/cutover-smoke
gh pr create --base main --title "chore: cutover smoke (close immediately)" \
  --body "OPMODEL-012 Phase 2 verification PR. Close without merging."
# Verify CI runs and required checks pass:
gh pr checks <PR-NUM> --watch
# Then close (do not merge); the smoke log file goes with the deleted branch:
gh pr close <PR-NUM> --delete-branch
```

CI must run (proves workflows still trigger on `main` PRs), every required check
must pass (proves protection isn't going to block real PRs), and the PR base
must default to `main`. Close the PR; do not merge.

### 7. Restrict or retire `dev`

Three options. **Default recommendation: dated compatibility branch.**
Reasoning: the emergency-hotfix runbook still references `origin/dev` for the
compat-mode back-merge step, RELORCH-011 has not yet retired the compatibility
release path, `origin/HEAD` on existing local clones still points to `dev` until
each contributor runs `git remote set-head`, and any out-of-tree tooling that
fetches the `dev` ref by name will break silently on delete. A dated branch
costs one tag push and gives every consumer a forcing-function deadline rather
than a permanent stub.

- **Dated compatibility branch (recommended).** Tag `dev`'s tip with a
  retirement date, then apply restrictive protection so no further work lands on
  it. Pick an expiry date and post it to the team channel:

  ```bash
  RETIREMENT_TAG="dev-retired-$(date -u +%Y-%m-%d)"
  git tag "${RETIREMENT_TAG}" origin/dev
  git push origin "${RETIREMENT_TAG}"

  gh api -X PUT repos/eddacraft/anvil-001/branches/dev/protection \
    -H "Accept: application/vnd.github+json" \
    --input - <<'EOF'
  {
    "required_status_checks": null,
    "enforce_admins": true,
    "required_pull_request_reviews": null,
    "restrictions": { "users": [], "teams": [], "apps": [] },
    "allow_force_pushes": false,
    "allow_deletions": false
  }
  EOF
  ```

  Open a follow-up issue ("Delete `dev` branch on or after `<expiry-date>`")
  with the retirement-tag SHA in the body so the actual delete has an owner and
  a calendar anchor.

- **Protect-and-keep (no expiry).** Choose only if you have a concrete reason to
  keep `dev` indefinitely (regulatory, external contractual reference, or a
  non-trivial historical query workflow that depends on the ref). Same
  protection API call as the dated branch, minus the tag and the follow-up
  issue. Worse than dated by default because there is no forcing function to
  ever clean it up.

- **Delete.** Choose only after enumerating and confirming that no external
  consumer (CI in another repo, installer scripts, downstream clones, Dependabot
  configs in dependent repos, contributor muscle memory) fetches `dev` by name.
  Irreversible without `git reflog` access on the remote:

  ```bash
  git push origin --delete dev
  git ls-remote origin dev      # must print nothing
  ```

Document the choice, the rationale, and (for dated) the expiry date and
follow-up issue number in the OPMODEL-012 completion line and in the team
channel.

### 8. Open the channel back up

Announce: cutover complete; new branches branch from `main`; PRs target `main`;
the open `dev`-targeted PRs need retarget per the deadline set in Pre-flight.

Each contributor with an existing local clone needs to update their local view
of the repo's default branch. Include this snippet in the announcement:

```bash
git fetch origin --prune
git remote set-head origin --auto      # updates origin/HEAD to point at main
git symbolic-ref refs/remotes/origin/HEAD     # must print refs/remotes/origin/main
```

Without `git remote set-head origin --auto`, `gh pr create` may still default to
`dev` as the base on existing clones because it resolves the default from local
`origin/HEAD`.

For each previously-open PR against `dev` that did not merge before cutover:

```bash
gh pr edit <PR-NUM> --base main
```

Where the head branch was branched off `dev`, the PR author may need to rebase
onto `main` (which is now the same as the old `dev`, so a no-op merge in most
cases).

## Verification

The cutover is verified when all of the following hold:

- `git rev-parse origin/main` equals the SHA captured in Step 3.
- Default branch on `eddacraft/anvil-001` is `main`
  (`gh api repos/... --jq .default_branch`).
- Branch protection on `main` is active with the operator-confirmed required
  checks (`gh api repos/.../branches/main/protection`).
- `pr-base-guard.yml` no longer exists in `.github/workflows/`.
- The Step 6 smoke PR ran CI on `main`, **every required check passed on the
  smoke PR** (proving protection won't block real PRs), and the PR was closed
  cleanly.
- `dev` reflects the operator's chosen disposition (protected-and-kept, dated,
  or deleted).
- A new throwaway branch off `main` can open a PR against `main` without hitting
  any guard rejection.
- The team announcement included the `git remote set-head origin --auto` snippet
  so contributors' clones update.

**Phase 2 evidence to paste into the Phase 3 PR body** (the Phase 3 docs flip
will not be reviewed without these):

```bash
# Cutover SHA:
git rev-parse origin/main

# Branch protection summary:
gh api repos/eddacraft/anvil-001/branches/main/protection \
  --jq '{required_checks: .required_status_checks.contexts, reviews: .required_pull_request_reviews.required_approving_review_count, force_push: .allow_force_pushes.enabled, deletions: .allow_deletions.enabled}'

# Default branch:
gh api repos/eddacraft/anvil-001 --jq .default_branch
```

Record each verification in the OPMODEL-012 completion line on the module file
and in a comms post to the team channel.

## Rollback

### Boundary

The clean-rollback window closes when the **Step 4 cleanup PR merges**
(`pr-base-guard.yml` deleted, deletion logged to the operator log per the note
in Step 4). If you reach a failure after that merge has landed and any further
work has appeared on `main`, full revert is no longer clean and the only safe
path is fix-forward — open issues for any broken paths and address per
[`rollback-bad-main.md`](./rollback-bad-main.md).

If `dev` was already deleted in Step 7, you are also past the clean rollback
window: skip step 1 below and go straight to fix-forward. (Steps 7 and later all
happen after Step 4 has merged, so `dev`-deleted implies past-boundary.)

### Steps (within boundary)

1. Re-fetch origin and confirm `main` and `dev` are still equal:

   ```bash
   git fetch origin
   git rev-parse origin/main origin/dev    # must print the same SHA twice
   ```

   If this command errors with "unknown revision" on `origin/dev`, `dev` has
   been deleted — you are past the rollback boundary. Stop here and go to
   fix-forward.

2. Remove branch protection on `main`:

   ```bash
   gh api -X DELETE repos/eddacraft/anvil-001/branches/main/protection
   ```

3. Restore the default branch to `dev` if it was changed:

   ```bash
   gh api -X PATCH repos/eddacraft/anvil-001 -f default_branch=dev
   ```

4. Restore `pr-base-guard.yml` if it was deleted. Use a content-addressed
   restore from the last commit that still contained the file — `origin/main^`
   is the previous tip of `main` (the old release branch) and may be a
   completely unrelated commit:

   ```bash
   # Locate the SHA of the deletion commit and its parent:
   gh pr view <cleanup-pr-number> --json mergeCommit --jq .mergeCommit.oid
   git show <merge-sha>^:.github/workflows/pr-base-guard.yml \
     > .github/workflows/pr-base-guard.yml
   git add .github/workflows/pr-base-guard.yml
   git commit -m "revert: restore pr-base-guard.yml during OPMODEL-012 rollback"
   git push
   ```

   Or simply revert the cleanup PR via `gh pr revert`. Either path is
   acceptable; pick whichever produces the cleaner audit trail in the tracking
   issue.

5. Restore `dev` if it was deleted:

   ```bash
   # Locate the cutover SHA from the operator log; restore the branch:
   git push origin <cutover-sha>:refs/heads/dev
   ```

   If `dev` was protect-and-kept, just remove the restrictive protection:

   ```bash
   gh api -X DELETE repos/eddacraft/anvil-001/branches/dev/protection
   ```

6. Reset local `origin/HEAD` for everyone with a clone:

   ```bash
   git fetch origin --prune
   git remote set-head origin --auto
   ```

## Release-record updates

OPMODEL-012 does not produce a release record. The cutover is a process
transition, not a release. After cutover, all subsequent releases follow the
target operating model per the
[release-record schema](../../plans/specs/2026-05-10-release-record-schema.md).

## APS / issue closeout

Cutover completion triggers the Phase 3 PR (per
[`opmodel-012.steps.md`](../../plans/execution/opmodel-012.steps.md)). That PR
carries the APS state changes:

- Mark OPMODEL-012 `Complete` with a completion line citing: cutover SHA,
  default-branch change time, branch-protection settings applied, dev
  disposition.
- Bump OPMODEL header to `12/12` and module status to `Complete`.
- Bump `plans/index.aps.md` OPMODEL row.
- Sweep cross-cutting callouts per
  [`aps-rules.md#cross-cutting-modules`](../../plans/aps-rules.md#cross-cutting-modules)
  (resolve / downgrade / document-and-close).
- `git mv plans/modules/operating-model-migration.aps.md plans/archive/modules/`
  once the module is Complete and all callouts are resolved.

## Mode notes

This playbook **executes** the cutover; it does not describe a steady-state
flow. After successful cutover the playbook is preserved as historical evidence
of how the cutover happened (per the same pattern as
[`branch-reconciliation.md`](./branch-reconciliation.md)) but should not be
re-run.
