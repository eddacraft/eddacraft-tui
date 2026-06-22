# Post-merge: feat-gctx-011-find-dependents

PR: #NNN
Branch: `feat/gctx-011-find-dependents`
APS: GCTX (graph-context-delivery), item GCTX-011
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip GCTX-011 status `In Progress → Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/graph-context-delivery.aps.md` (agent: yes — if not already
      carried in the merge commit). Bump the GCTX module count `3/13 → 4/13` and
      reconcile the index narrative in `plans/index.aps.md` (GCTX-011 no longer
      Draft). (agent: yes)
- [ ] Live daemon smoke: with `anvil watch` running on a small TS workspace where
      `b.ts` imports `./a`, call the `anvil_find_dependents` MCP tool for
      `file: a.ts` and confirm a `ready` outcome listing `b.ts` at distance 1.
      (agent: no — needs a quiet box with a live daemon + inotify; watch benches
      do not run in the agent shell.)
- [ ] Confirm `ANVIL_GCTX_EGRESS=0` yields a `disabled` outcome and unset yields
      identity results, against the same live daemon. (human required)
- [ ] GCTX-011 → Released/Shipped only on the next release tag that includes this
      commit (do not advance to Complete before a tag). (agent: yes — on tag
      evidence)

## Notes

- This ships `anvil_find_dependents` only (the dependents half of the original
  GCTX-011). Symbol-level *caller* traversal is split to **GCTX-014** (Blocked on
  GV2 call-edge support) and is out of scope here.
- The tool mirrors the GCTX-010 `anvil_search_symbols` spine end to end; no new
  egress crate, no manifest flag (the C4 `gctx.egress` manifest flag is Phase-2).
- Council follow-ups shared with the merged GCTX Phase-1 surface are tracked as
  **CIB-099** (`continuous-improvement-backlog.aps.md`). N5 (`SaveTimeError::Io`
  wire leak) closed in PR #2852.
