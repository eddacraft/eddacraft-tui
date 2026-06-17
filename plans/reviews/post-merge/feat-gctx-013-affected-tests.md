# Post-merge: feat-gctx-013-affected-tests

PR: #NNN
Branch: `feat/gctx-013-affected-tests`
APS: GCTX (graph-context-delivery), item GCTX-013
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip GCTX-013 status `In Progress → Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/graph-context-delivery.aps.md` (if not carried in the merge
      commit); bump the GCTX module count `5/14 → 6/14` (module header + Stats
      table total + index row) and reconcile the index NBI table + narrative
      (GCTX-013 no longer Ready; next pick is GCTX-014 once GV2 call-edges land,
      or a Phase-1/2 Draft item). (agent: yes)
- [ ] Live daemon smoke: with `anvil watch` on a small TS workspace where
      `s.test.ts` imports `./s` and a second changed source `u.ts` has no test,
      call the `anvil_affected_tests` MCP tool with `changedFiles: ["s.ts",
      "u.ts"]` and confirm a `ready` outcome whose `tests` lists `s.test.ts` with
      an evidence edge to `s.ts`, `coverage_gaps` lists `u.ts`, and `heuristic` is
      `true`. (agent: no — needs a quiet box with a live daemon + inotify.)
- [ ] GCTX-013 → Released/Shipped only on the next release tag that includes this
      commit. (agent: yes — on tag evidence)

## Notes

- Ships `anvil_affected_tests` on the GCTX-010/011/012 spine — no new egress
  crate, no new graph substrate. Reuses GCTX-012's `is_test_file` heuristic + the
  reverse-impact walk, and adds the dependency graph's **forward** edges
  (`dependencies_of`) for the evidence link and transitive coverage.
- Daemon RPC takes **explicit changed file paths only** (≤200, CE-6 — never diff
  content). The MCP tool requires an explicit `changedFiles` array.
- Two bounded passes run under the cache lock: a reverse multi-source BFS finds
  the dependent tests (each tagged with `dependencies_of(test) ∩ changed_set`
  evidence + hop distance), and a forward multi-source BFS from the discovered
  tests (plus any changed test files) determines transitive coverage; a changed
  non-test file no test reaches within the bound is a coverage gap.
- Council (adversarial) run pre-PR. Fixed: the two passes now **share one
  aggregate `MAX_DEPENDENTS_WALK` budget** (was one per pass → 2× the ADR-031
  lock-held node budget); the forward coverage walk now **drops absolute-path
  dependency nodes** (was burning budget on unexpandable node_modules paths →
  earlier truncation / false gaps). Added a regression test for the forward-walk
  absolute-path drop.
- **Deferred follow-ups** (deliberate, shared with the merged GCTX surface — not
  regressions):
  - Relevance is an **import heuristic** (`heuristic: true`) — file-keyed, not
    execution-verified, not symbol-level. By contract (GCTX-001).
  - A changed test file is excluded from the `tests` output (it is part of the
    change set, consistent with the dependent-closure seed exclusion) but DOES
    contribute to coverage (so a changed source imported only by a co-changed
    test is not a false gap).
  - The MCP tool does no client-side per-path validation (daemon-side CE-6 is the
    gate) — consistent with the sibling tools.
  - `AffectedTestsOutcome` has no `#[serde(other)]` forward-compat fallback —
    matching the sibling GCTX outcome enums (same-version client/daemon). A
    cross-surface forward-compat pass, if wanted, should touch all four GCTX
    outcome enums together.
  - Git-diff path derivation in the MCP tool (the spec's optional "may derive
    client-side") is not implemented — the tool takes explicit `changedFiles`.
  - No dedicated ADR-031 bench for `affected_tests`; the shared-budget bound keeps
    its lock-held cost within the single-walk envelope the existing GCTX benches
    cover.
