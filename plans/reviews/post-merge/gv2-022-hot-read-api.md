# Post-merge: gv2-022-hot-read-api

PR: #NNN
Branch: `feat/gv2-022-hot-read-api`
APS: GV2-022
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip GV2-022 `In Progress → Merged` in
      `plans/modules/graph-v2-foundation.aps.md` and reconcile the Phase 2 Stats
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

Deferred, tracked as their own GV2-027-critical-path items (do **not** treat as
missing here):

- **GV2-024** — compile-time type split sealing `HotReadApi` so denylist ops are
  uncallable from the hot surface (depends on this item).
- **GV2-025** — Criterion `benches/hot_read.rs` + ADR-031 CI latency gate
  validating these reads meet the save-time budget on the canonical corpus
  (run on a quiet/CI box per the bench-harness flakiness note).
- **GV2-026** — wires `reverse_impact`'s `max_depth` to a `flags/manifest.json`
  runtime lever (default 1 hop); `MAX_REVERSE_IMPACT_DEPTH = 2` is the hard cap
  this item already enforces.

No production behaviour changes in this PR — the API has no callers yet, so the
hot path is unchanged until GV2-027.
