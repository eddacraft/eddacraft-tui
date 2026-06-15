# Post-merge: feat-gctx-010-search-symbols

PR: #2637
Branch: `feat/gctx-010-search-symbols`
APS: GCTX-010
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance GCTX-010 from Ready (#2635) → In Progress in
      `plans/modules/graph-context-delivery.aps.md` and the Phase-1 row in the
      Stats table; this PR deliberately did not touch `plans/` to avoid
      colliding with the in-flight readiness PR #2635 (agent: no — needs the
      #2635 text as the base; do after #2635 merges)
- [ ] Confirm the daemon dep-boundary still holds on `main` after merge:
      `cargo test -p eddacraft-anvil-intercept --test daemon_dep_boundary`
      (no tree-sitter pulled into the daemon via the new egress crate)
      (agent: yes)
- [ ] Confirm `cargo hakari verify` is clean on `main` (the two new crates add
      no new transitive features) (agent: yes)
- [ ] Smoke the live path on a box with the daemon running: start
      `anvil intercept` (daemon), then call `anvil_search_symbols` via
      `anvil mcp serve` against a warmed worktree and confirm a `ready` outcome
      with ordered identities; against a cold worktree confirm `not_ready`
      (human required — needs a running daemon + warmed graph; the daemon has no
      auto-start yet, DSV-021/DLIFE)

## Notes

This is PR 1 of 4 within GCTX-010 (the thin vertical slice). Do **not** mark
GCTX-010 Complete on this merge — it advances to In Progress only. The remaining
staged PRs (CE-6 cursors; CE-10/CE-11 telemetry+kill-switch; C1 warm-up) must
land, and GCTX terminates at Released/Shipped on a release tag, not at Merged.

Sequence the user-facing rollout against **DLIFE** (ADR-084): GCTX is
daemon-required and degrades to `unavailable` when the daemon is not running, so
it is only useful once daemon-backed protection is the normal user path.
