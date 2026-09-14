# CIB-028 Mini Council — Worktree Cleanup Sweep

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

CIB-028 adds a conservative operator assistant for reviewing and removing safe
Worktrunk cleanup candidates after merge batches.

Changed surfaces:

- `scripts/dev/wt-cleanup-sweep.py`
- `scripts/dev/wt-cleanup-sweep.sh`
- `scripts/dev/wt-cleanup-sweep.test.sh`
- `docs/guides/worktree-policy.md`
- `plans/modules/continuous-improvement-backlog.aps.md`
- `plans/index.aps.md`

## Council constraints applied

Mini Council returned WARN from operations and adversarial reviewers. The helper
implements their shared constraints:

- dry-run/list-only by default;
- no bare `wt remove` — apply requires explicit branch names;
- current, main, dev, detached, release/hotfix, non-disposable-prefix, and
  branch/path mismatch entries are skipped;
- status is rechecked immediately before removal;
- remote-gone alone is not enough: a branch must be proven safe against refreshed
  `origin/main` by ancestry or patch equivalence;
- dirty, untracked, and non-allowlisted ignored files are skipped;
- `wt remove --foreground --format json <branch>` is the only removal path;
- no force removal/deletion flags, no remote branch deletion, no raw path-template
  deletion.

## Implemented behaviour

- `scripts/dev/wt-cleanup-sweep.sh --dry-run` prints every worktree with either
  an `ELIGIBLE` marker or a skip reason.
- `--apply <branch>...` only considers explicitly named eligible branches,
  revalidates each branch, prompts for the exact branch name, then delegates to
  Worktrunk.
- `--json` returns structured candidate records for tests or operator review.
- Hidden `--confirm-for-test` is only for the fixture test and does not bypass
  Worktrunk's own safety checks.

## Validation evidence

Fixture validation passed:

```text
python3 -m py_compile scripts/dev/wt-cleanup-sweep.py
bash -n scripts/dev/wt-cleanup-sweep.sh
bash -n scripts/dev/wt-cleanup-sweep.test.sh
scripts/dev/wt-cleanup-sweep.test.sh
```

The fixture creates a temporary repo with:

- one merged clean branch with deleted upstream — eligible and removed only in
  apply mode;
- one unmerged remote-gone branch — skipped and preserved;
- one merged dirty branch — skipped and preserved;
- one detached worktree — skipped and preserved;
- the current/main worktree — skipped.

The fake `wt` recorder asserts the helper invokes `wt remove --foreground
--format json <branch>` and never passes Worktrunk force/no-hook/global-confirm
flags.

## Notes

A live `wt list --format json` in this sandbox emitted merge-tree warnings because
Git could not create temporary files outside the writable root. The helper avoids
using `wt list` status as proof and computes its own conservative eligibility
against `origin/main`; any failed proof becomes a skip/manual-review reason.
