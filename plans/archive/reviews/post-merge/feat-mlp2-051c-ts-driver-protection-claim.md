# Post-merge: feat/mlp2-051c-ts-driver-protection-claim

PR: #TBD
Branch: `feat/mlp2-051c-ts-driver-protection-claim`
APS: MLP2-051c
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Mark MLP2-051c `Merged` in `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Bump MLP2 progress counter in `plans/index.aps.md` from `52/76` to `53/76` and update the matching prose mentions (agent: yes)
- [ ] Re-check whether MLP2-051e (cross-surface parity test) can move out of `Blocked` — it depends on MLP2-051a + 051b + 051c. MLP2-051a is still `In Progress`; 051e stays `Blocked` until 051a merges (agent: yes)
- [ ] Run `wt remove` for the worktree after merge once `addressing-pr-reviews` reports no remaining review threads (human required)

## Notes

- TS side is now wire-compatible with the MLP2-051b producer: missing
  `protection_claim` is treated as pre-MLP2-051b parity (returns
  `undefined`); a present-but-malformed claim throws `TypeError` so a
  half-typed claim never propagates silently.
- Hand-rolled parser, no Zod dependency (third such mirror after
  MLP2-029 AgentTag and MLP2-030 mid-edit). If a fourth mirror lands
  (e.g., GH Action surface for MLP2-051d once the Marketplace track
  unblocks), reconsider extracting a shared parser helper.
- The exported `parseOptionalProtectionClaimFromValidateWrite` has no
  internal call-site in this package — by design. The MCP
  `validate_write` response is consumed by external editor drivers /
  MCP shim consumers; they import the parser as a library function.
  The MLP2-051e cross-surface parity test will be the first internal
  call-site (`packages/anvil-driver-client/__tests__/` per the spec).
- Hostile-input tests pin: (1) `__proto__` keys from `JSON.parse` do
  not escalate (own-data-property, not prototype), (2) `surfaces: null`
  errors with a `got null` diagnostic, (3) mixed-type entries in
  `surfaces` reject with `SurfaceClaim` context, (4) array-shaped
  envelope is rejected by the `Array.isArray` guard in `asObject`.
