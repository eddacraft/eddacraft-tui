# Post-merge: feat-gv2-024-hot-read-seal

PR: #NNN
Branch: `feat/gv2-024-hot-read-seal`
APS: GV2-024
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Confirm the merge commit is on `origin/main` and the `Test` +
      `Clippy` + `Format` checks were green for it. (agent: yes — `gh run list`)
- [ ] Step 2 — Flip GV2-024 `In Progress → Merged` in
      `plans/archive/modules/graph-v2-foundation.aps.md` (`Merged YYYY-MM-DD via PR #N`)
      and bump the GV2 module count in `plans/index.aps.md` (Merged is counted;
      11/19 → 12/19 — regenerate via `pnpm aps:index` rather than by hand).
      (agent: yes)
- [ ] Step 3 — GV2-025 was merged (PR #2459) but its status may still read
      `In Progress` on `main`; flip it to `Merged` in the same index pass if the
      cleanup agent has not already done so. (agent: yes)

## Notes

- **What this delivers:** ADR-077 (path A) depth-cap of `certify`'s reverse-impact
  closure + the GV2-024 hot-read seal (`BackgroundReadApi` denylist home,
  `HotPathSurface` sealed marker, `debug_assert` depth guard, two `compile_fail`
  doctests).
- **Verdict-neutrality** of the cap is proven by `certify.rs` unit tests
  (`*_adr077`); `backing_parity` independently proves warm == cold (it cannot
  form >2-hop chains, so it does NOT cover the cap — see the corrected
  `HotReadApi::certify` doc).
- **Council review** (adversarial + kernel) ran pre-PR. Addressed: removed a dead
  `reverse_impact` debug_assert, inlined the thin `impact_closure` wrapper, fixed
  the false `backing_parity` attribution, documented the intentional origin-on-cycle
  behavior (preserved from the pre-cap walk — not changed, to stay within ADR-077's
  "depth-cap only" mandate). `BackgroundReadApi` is `pub` by design (its consumer is
  the sibling `anvil-intercept` background pool).
- **Follow-up (not in this PR):** the unbounded `BackgroundReadApi::impact_closure_unbounded`
  has no production caller yet — it lands its consumer when the background
  full-validation pool is built. The runtime 1→2-hop depth lever is GV2-026.
- Local verification: 94 graph-cache unit + 2 compile_fail doctests, 607
  anvil-intercept lib tests, `backing_parity` (warm==cold over 400-step
  sequences) all green; clippy `-D warnings` + fmt clean across graph-cache,
  anvil-kernel, anvil-intercept.
