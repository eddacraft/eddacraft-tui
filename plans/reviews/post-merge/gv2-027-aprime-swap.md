# Post-merge: gv2-027-aprime-swap

PR: #NNN
Branch: `feat/gv2-027-aprime-swap`
APS: GV2-027 (stacked: also lands GV2-029)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip GV2-029 `In Progress → Merged 2026-06-08 via PR #N` and GV2-027
      `In Progress → Merged 2026-06-08 via PR #N` in
      `plans/modules/graph-v2-foundation.aps.md`; reconcile Phase 3 (`1/7 → 3/7`)
      and the module + index total (`9/19 → 11/19`). Run
      `node scripts/aps/index-counts.mjs --check`. Own `chore(aps)` PR
      (#2421/#2432 pattern), to avoid colliding with any concurrent GV2 flips. (agent: yes)
- [ ] Confirm `cargo test -p eddacraft-anvil-intercept -- backing_parity` and the
      diagnostic-parity gate are green on `main`. (agent: yes)
- [x] **Owner/council decision (not auto-resolvable):** decide whether sub-phase
      A′ adopts the depth-capped `HotReadApi::reverse_impact` for the
      certifiability closure (would change the
      `ImpactSetOverflow`-vs-`ExportSurfaceChange` stale reason in graphs deeper
      than `MAX_REVERSE_IMPACT_DEPTH`). Reconciles ADR-061 §6 (cert closure
      default 1 hop) with ADR-063 (bounded hot reads). Currently
      verdict-preserving (unbounded `impact_closure` retained). (human required)
      **Resolved 2026-06-09:** owner chose path A — adopt the depth cap. Captured
      in [ADR-077](../../decisions/077-cert-closure-depth-cap.md) (Proposed,
      awaiting ratification); implemented by GV2-024. The relabel is between two
      `Partial` reasons, so the swap stays coverage-verdict-preserving.

## Notes

GV2-027 routes `validate_paths` certification through the resident GV2 hot-read
index (`HotReadApi::certify`, GV2-022) — the live A′ backing behind the unchanged
ADR-061 wire. Verdict-preserving: `certify`'s algorithm is unchanged, proven by
the `backing_parity` property test (warm incremental backing == cold rebuild,
verdict-identical over arbitrary delta sequences, across budgets 64 and 1 so both
the `ExportSurfaceChange` and `ImpactSetOverflow` branches are exercised, incl.
the GV2-029 `node:fs` privilege dimension).

GV2-029 (privilege containment on the daemon certify path) is included in this
stack via cherry-pick of the sibling's completed-but-unpublished commit
(`10991cc8f`, authorship preserved) — it is a hard prerequisite of the GV2-027
swap. If a separate GV2-029 PR lands first, this stack rebases cleanly (identical
change drops out).

**GV2-024 hand-off:** when the hot-read type split lands, its seal must either
replace `HotReadApi::certify`'s body with a depth-capped closure or explicitly
exclude it with the ADR rationale from the owner decision above — flagged in the
`HotReadApi::certify` doc comment.
