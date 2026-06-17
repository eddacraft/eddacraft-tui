# Post-merge: feat-gcall-003-resident-edges

PR: #NNN
Branch: `feat/gcall-003-resident-edges`
APS: GCALL (symbol-call-graph), item GCALL-003
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip GCALL-003 status `In Progress → Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/symbol-call-graph.aps.md`; bump the GCALL module count
      `2/7 → 3/7` (module header + Last-reviewed note + index module-table row)
      and refresh the NBI. With GCALL-003 merged, **GCTX-014 `anvil_find_callers`
      is unblocked on the substrate side** (still gated on GCALL-007, the
      caller-egress privacy review) — note that in the GCTX-014 entry. (agent: yes)
- [ ] GCALL-003 → Released/Shipped only on the next release tag that includes this
      commit. (agent: yes — on tag evidence)
- [ ] Live daemon smoke: with `anvil watch`, save `util.ts` (`export function
      helper(){}`) then `main.ts` (`import {helper} from './util'; function run(){
      helper(); }`), and confirm a future `anvil_find_callers helper` (GCTX-014)
      returns `run`. (agent: no — needs a live daemon + GCTX-014, which is not yet
      shipped.)

## Notes

- The consumer-facing half of the call graph (ADR-086): lift `FileSymbols.calls`
  into resident `EdgeType::Calls` edges + a bounded `callers_of` read API. The
  `anvil_find_callers` MCP egress is GCTX-014, which projects `callers_of`.
- **Lift** (`anvil-graph-cache` `incremental.rs`): `update_file` lifts a file's
  call sites eagerly (same-file + already-resident callees); `re_resolve_calls`
  over a `(caller_file, CallSite)` accumulator handles forward references and
  re-wires incoming edges dropped when a callee file is re-saved. Caller is
  resolved by `(kind, name, ordinal)` against `for_file_symbols` (NOT the
  file-level `first()`-symbol shortcut imports use); `module_scope` callers anchor
  to the file's synthetic `Module` node (reused, never duplicated). Callee
  resolution: same-file by name, or imported via `resolve_import` + export-name
  lookup; overloads fan out (capped `MAX_OVERLOAD_FANOUT = 8`); default imports
  and over-cap fan-outs are Unresolved (no edge). Edge add is idempotent (dedup on
  `(from, to, Calls)`).
- **Daemon** (`anvil-intercept` `kernel_cache.rs`): an `all_calls` accumulator on
  the warm `Entry`, maintained in lockstep with `all_imports`/`all_reexports`;
  `re_resolve_calls` runs over the **affected neighbourhood** (`dependents_of(file)
  ∪ {file}` ∪ re-resolved import sources), not the whole accumulator, so the
  lock-held cost stays neighbourhood-bounded (ADR-031).
- **Read API** (`call_graph.rs`): `callers_of(target, depth)` — a bounded reverse
  BFS over incoming `Calls` edges, GV2-026 depth-clamped + `MAX_CALLERS_WALK`
  node budget, identity-sorted frontiers (deterministic truncation), seen-set
  cycle termination. **Identity-only** output (`CallerResult { caller, distance }`).
  The target is excluded from its own caller set (the `collect_dependents`
  origin-exclusion), so a purely self-recursive symbol reports no callers.
- **`heuristic`/`partial` markers are GCTX-014's job (ADR-086 §1 implementation
  finding).** Adversarial review confirmed the resident `Calls` edge carries no
  provenance, so fan-out cannot be distinguished from two legitimate distinct
  overload calls at read time, and an unresolved caller produces no edge (invisible
  to the walk). The bare `callers_of` returns identity + distance + `truncated`
  (faithfully computable); GCTX-014 threads `heuristic`/`partial` from the lift's
  provenance + the `all_calls` accumulator. ADR-086 §1 updated to record this.
- **Deferred / known limitations** (accepted under the heuristic posture):
  - **Barrel / re-export follow** for callees is NOT implemented — the resident
    graph carries `Reexports` edges but not the per-name mapping, so a callee
    re-exported through a barrel resolves to Unresolved. A future refinement
    (needs the per-name reexport data on the resident edge or a side table).
  - Full-accumulator re-resolution was rejected in favour of the
    neighbourhood-scoped pass; if a bench ever shows the scoped pass missing an
    edge, widen the affected set rather than going full-accumulator.
  - No dedicated ADR-031 bench for the call-lift; the scoped re-resolution keeps
    it within the existing save-time envelope (GCALL-006 owns the explicit gate).
