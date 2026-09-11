# Post-merge: feat/mlp2-051b-mcp-protection-claim

PR: #TBD
Branch: `feat/mlp2-051b-mcp-protection-claim`
APS: MLP2-051b
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Mark MLP2-051b `Merged` in `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Bump MLP2 progress counter in `plans/index.aps.md` from `50/76` to `51/76` and update the matching prose mentions (agent: yes)
- [ ] Unblock MLP2-051c (TS driver-client `ProtectionClaim` mirror) in the module file — its `Dependencies` line names MLP2-051b directly (agent: yes)
- [ ] Confirm `MLP2-051e` (cross-surface parity test) remains `Blocked` until MLP2-051a and MLP2-051c also land (agent: yes)
- [ ] Run `wt remove` for the worktree after merge once `addressing-pr-reviews` reports no remaining review threads (human required)

## Notes

- The `protection_claim` field is wire-additive and opt-in; no downstream
  consumer is broken by the addition. MLP2-051c picks up the field on the
  TS driver-client side.
- The 4 s worst-case wall-clock on `validate_write` (2 s scan + 2 s claim
  fetch when the daemon accepts-but-hangs) is documented inline; if
  operator telemetry surfaces unhealthy daemons stretching this, the
  follow-up is folding the claim into `scan_buffer`'s reply so the
  second hop disappears.
- The Windows path returns `None` for the claim today; lifting that gap
  needs a `query_daemon_status_windows_at` extraction and is tracked
  alongside the rest of the Windows MCP-shim catch-up.
