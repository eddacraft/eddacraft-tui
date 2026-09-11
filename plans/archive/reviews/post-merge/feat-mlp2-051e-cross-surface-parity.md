# Post-merge: feat/mlp2-051e-cross-surface-parity

PR: #TBD
Branch: `feat/mlp2-051e-cross-surface-parity`
APS: MLP2-051e
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance MLP2-051e from `In Progress` to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Verify the MLP2-051 umbrella status — with 051a/b/c/e all `Merged`,
      mark the umbrella `Merged` too. 051d remains Blocked on the Marketplace
      gate (per memory: `project_marketplace_track_gate`); do not include
      it in the umbrella closure (agent: yes)
- [ ] Update the `J. Protection-claim render conformance` group counter in
      `multilayer-protection-v2.aps.md` (currently `4/10`) — increment by one
      for 051e merging. Re-derive the count from items in `Merged` /
      `Released/Shipped` / `Complete` to be safe (agent: yes)
- [ ] Update the MLP2 row in `plans/index.aps.md` done/total count if 051e's
      merge crosses a tally line (agent: yes — pure arithmetic from the
      module file)
- [ ] Confirm `MLP2-051e` advances to `Released/Shipped → Complete` only when
      the `v0.7.0-beta` release record lands (agent: no — release-trigger
      decision; cleanup agent should flag rather than auto-advance)
- [ ] Re-check whether 051d (`apps/action/*` parity) can move out of `Blocked`.
      It stays blocked while the Marketplace track gate (MLP2-042..045) is
      paused on the licensing/pricing model lock — no change expected unless
      the commercial decision flips (agent: yes — read the
      `project_marketplace_track_gate` memory, confirm "no change", record it
      in cleanup-log)

## Notes

- Cross-surface fixture set lives at
  `crates/anvil-cli/tests/fixtures/status_v1/cross_surface/`. Six pinned
  cases. Regenerate intentional changes with
  `ANVIL_UPDATE_FIXTURES=1 cargo test --test protection_claim_cross_surface`.
- The TS leg of the parity test reads the Rust-emitted fixtures via a
  workspace-relative path
  (`packages/anvil-driver-client/src/protection_claim/cross_surface.test.ts`).
  If the TS package ever moves, that relative path needs updating in the
  same commit.
- HARD-GATE close: with 051a (PR #1655), 051b (PR #1668), 051c (PR #1675),
  and 051e all `Merged`, the §14 closed-set protection-claim contract has
  byte-pinned conformance across every shipping render surface — the
  HARD-GATE the MLP-009 umbrella block hinged on.
