# Post-merge: feat-gctx-014-find-callers

PR: #NNN
Branch: `feat/gctx-014-find-callers`
APS: GCTX (graph-context-delivery), item GCTX-014
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip GCTX-014 status `In Progress → Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/graph-context-delivery.aps.md`; bump the GCTX module count
      `6/14 → 7/14` (module header + Last-reviewed note + index module-table row)
      and reconcile the index NBI table/narrative. With GCTX-014 merged, the GCTX
      Phase-1 tool surface (010/011/012/013/014) is complete; remaining GCTX items
      are Phase-1/2 Draft (021..023, 030). (agent: yes)
- [ ] GCTX-014 → Released/Shipped only on the next release tag that includes this
      commit. (agent: yes — on tag evidence)
- [ ] Live daemon smoke: with `anvil watch` on a TS workspace where `run()` calls
      `helper()` (same file or imported), call `anvil_find_callers` with the
      `helper` symbol identity and confirm a `ready` outcome listing `run` at
      distance 1, `heuristic: false`. (agent: no — needs a live daemon + inotify.)

## Notes

- The consumer tool for the symbol call graph: `anvil_find_callers` projects the
  GCALL-003 `callers_of` read API over the daemon, mirroring `anvil_find_dependents`
  end to end (sealed `anvil-gctx-types` DTOs, `GctxProjector` choke point,
  `anvil/gctx/find_callers` RPC on `GctxDispatch`, graph-free MCP consumer).
- **GCALL-007 CALL-1..CALL-5, all met:**
  - CALL-1 — `heuristic` per `CallerSummary` (from the substrate marker) +
    report-level `partial` (set when the walk was node-budget-truncated or the
    graph is `Stale`/`Bounded`); the tool description states best-effort static /
    not authoritative. Static under-inclusion (dynamic dispatch) is conveyed by
    the description, not the boolean (an unresolved call leaves no edge to count).
  - CALL-2 — sealed `CallerSummary` / `FindCallersProjection` DTO (identity +
    distance + heuristic only); CE-5 structural no-leak tests extend to it.
  - CALL-3 — node budget + GV2-026 depth clamp (substrate) + opaque keyset
    pagination (`MAX_PAGE_LIMIT`) + per-path query validation.
  - CALL-4 — `FindCallersOutcome` mirrors `FindDependentsOutcome`; enum-only
    telemetry (`telemetry_outcome → GctxOutcome`); `ANVIL_GCTX_EGRESS=0`
    kill-switch; no source fallback on warming/stale.
  - CALL-5 — identity-only; call-site source-text egress stays a CE-1-gated
    Phase-2 escalation, out of scope.
- **Deferred / known limitations:**
  - The Windows named-pipe GCTX client is a future item; `find_callers` degrades
    to `unavailable` on non-unix (mirrors the sibling tools).
  - Barrel/re-export callee resolution is the GCALL-003 documented deferral (a
    callee re-exported through a barrel is `Unresolved`, so its callers are
    `partial`-covered).
  - No dedicated ADR-031 bench for the read path; it is off the save-time hot
    path (ADR-063), bounded by `MAX_CALLERS_WALK` + depth clamp.
