# Post-merge: fix-cib-005-007-validate-write

PR: #NNN
Branch: `fix/cib-005-007-validate-write`
APS: CIB-005, CIB-007
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Mark CIB-005 status `Merged` in `plans/modules/continuous-improvement-backlog.aps.md` (agent: yes)
- [ ] Mark CIB-007 status `Merged` in `plans/modules/continuous-improvement-backlog.aps.md` (agent: yes)
- [ ] Bump CIB progress counter from `3/7` to `5/7` in module header and `plans/index.aps.md` (agent: yes)
- [ ] When the next hotfix tag cuts, record release evidence on both items and advance to `Released/Shipped` (agent: yes, on tag detection)
- [ ] Remove `wt remove fix/cib-005-007-validate-write` worktree after operator confirms no live debugging is anchored there (human required)

## Notes

This PR is intended to ship in a hotfix tag bundled with the next
`anvil_validate_write` friction sweep. The release-record schema entry will
list both CIB-005 and CIB-007 under `aps.items[]` with `releaseScope: patch`
once the tag cuts.

The cleanup agent should not advance either item past `Merged` until the
hotfix tag is recorded in `plans/releases/`. The dev-workflow lifecycle is:

```
Merged → Released/Shipped (on tag) → Complete (after operator sign-off)
```

Option (a) of CIB-007 (worktree-aware accept) was deferred — if option (b)'s
recoverability proves insufficient in practice, open a follow-up CIB rather
than re-opening this item.
