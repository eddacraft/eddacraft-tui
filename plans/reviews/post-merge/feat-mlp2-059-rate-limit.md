# Post-merge: feat-mlp2-059-rate-limit

PR: #1606
Branch: `feat/mlp2-059-rate-limit`
APS: MLP2-059 (Group L production hardening — Council #C-023)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance MLP2-059 status to **Merged** in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Update Group L count from `3/4` to `4/4 (Complete)` in the
      module summary table (agent: yes)
- [ ] Bump module total (currently `35/66` on module, `36/66` on
      index — see Notes) by +1 to reflect MLP2-059 closure
      (agent: yes)
- [ ] Update `plans/index.aps.md` MLP2 row to reflect MLP2-059
      closure + name PR #1606 at the merge commit SHA in the
      wave-history blurb (agent: yes)
- [ ] Verify cleanup agent ran `wt remove` for the worktree
      (agent: yes)
- [ ] Manual: confirm the new
      `cache_invalidations_rate_limited` field shows on a live
      `anvil intercept status --json` after a dogfood storm —
      this is the operator-facing signal MLP2-059 added, and
      automated tests only cover the synthetic in-process case
      (human required)

## Notes

MLP2-059 was the last open item in Group L (Council production-
hardening follow-ons from session `council-e2fdfc0c`). With this
PR, Group L closes 4/4 alongside Groups F (2/2), H (5/5), and K
(4/4).

Pre-existing index/module count discrepancy (`36/66` on the
index vs `35/66` on the module) was inherited from prior PRs
and not reconciled in this PR to keep scope single-purpose.
The cleanup agent should pick the module's count as the
canonical source when advancing past merge.

Wire-shape note for downstream consumers (driver-client, MCP
shim, future audit-chain tooling): the new
`cache_invalidations_rate_limited` field on `DaemonStatusV1` is
additive-optional with `#[serde(skip_serializing_if = "Option::is_none")]`,
matching the MLP2-058 pattern. Pre-MLP2-059 daemons round-trip
into post-MLP2-059 consumers with `None` (pinned by
`pre_mlp2_059_payload_round_trips_with_rate_limit_field_absent`).
