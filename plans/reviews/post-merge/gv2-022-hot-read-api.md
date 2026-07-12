# Post-merge: gv2-022-hot-read-api

PR: #NNN
Branch: `feat/gv2-022-hot-read-api`
APS: GV2-022
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip GV2-022 `In Progress → Merged` in
      `plans/archive/modules/graph-v2-foundation.aps.md` and reconcile the Phase 2 Stats
      count (1/4 → 2/4 done) + the index total (5/19 → 6/19) — as its own
      `chore(aps)` PR (the #2421/#2432 pattern), to avoid colliding with the
      concurrent GV2-012/-028/-029 sibling flips on the same hot files. (agent: yes)
- [ ] Confirm `cargo test -p eddacraft-anvil-graph-cache -- hot_read` is green on
      `main` post-merge (14 tests). (agent: yes)

## Notes

GV2-022 ships the **read API + warm/stale contract only** (`HotRead`,
`HotReadMiss`, `HotReadApi` over the four ADR-063 allowlist reads, with a
hard-capped `reverse_impact`). It is not yet wired into `validate_paths` — that
backing swap is **GV2-027**, which also depends on GV2-024/-028/-029.

Downstream items **GV2-024 / -025 / -026 / -027** have since Merged (GV2 module
20/20 Complete). This post-merge doc is historical verification only.
