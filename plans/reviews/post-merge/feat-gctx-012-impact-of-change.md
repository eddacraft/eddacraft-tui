# Post-merge: feat-gctx-012-impact-of-change

PR: #NNN
Branch: `feat/gctx-012-impact-of-change`
APS: GCTX (graph-context-delivery), item GCTX-012
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip GCTX-012 status `In Progress → Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/graph-context-delivery.aps.md` (if not carried in the merge
      commit); bump the GCTX module count `4/14 → 5/14` (module header + Stats
      table total + index row) and reconcile the index NBI table + narrative
      (GCTX-012 no longer Ready; next pick is GCTX-013). (agent: yes)
- [ ] Live daemon smoke: with `anvil watch` on a small TS workspace where `b.ts`
      and `a.test.ts` import `./a`, call the `anvil_impact_of_change` MCP tool
      with `changedFiles: ["a.ts"]` and confirm a `ready` outcome whose
      `affected_symbols` lists `a.ts`'s symbols, `dependent_files` lists `b.ts` +
      `a.test.ts`, and `known_tests` lists `a.test.ts`. (agent: no — needs a quiet
      box with a live daemon + inotify.)
- [ ] GCTX-012 → Released/Shipped only on the next release tag that includes this
      commit. (agent: yes — on tag evidence)

## Notes

- Ships `anvil_impact_of_change` on the GCTX-010/011 spine — no new egress crate,
  no new graph substrate. Composes `symbols_in_file` (affected surface) + a
  multi-source reverse-impact BFS (dependent closure, reusing the GCTX-011 walk
  shape) + a heuristic test-file filter.
- Daemon RPC takes **explicit changed file paths only** (≤200, CE-6 — never diff
  content). The MCP tool requires an explicit `changedFiles` array.
- Council (kernel/adversarial/operations) run pre-PR. Fixed: bounded the
  affected-symbol allocation under the lock (`MAX_AFFECTED_SYMBOLS`); sorted the
  hop-2 `next` frontier (in both `collect_impact` and `collect_dependents`) so an
  over-budget truncation keeps a path-ordered prefix; closed a CE-6 gap where a
  `\\server\share` UNC root passed validation but was dropped downstream
  (inflating the count) — now rejected and the count comes straight from
  `collect_impact`; dropped a no-op dedup; added the `InvalidQuery` wire
  round-trip + UNC-rejection + count-accuracy tests.
- **Deferred follow-ups** (deliberate, shared with the merged search/dependents
  surface — not regressions):
  - `known_tests` is **best-effort heuristic** (naming convention). GCTX-013
    `anvil_affected_tests` owns the rigorous evidence-edge + coverage-gap version.
  - The MCP tool does no client-side per-path validation (daemon-side CE-6 is the
    gate) — consistent with the sibling tools.
  - `ImpactOutcome` has no `#[serde(other)]` forward-compat fallback — matching
    the sibling `SearchSymbolsOutcome` / `FindDependentsOutcome` (the GCTX client
    and daemon ship same-version, unlike the cross-version driver protocol where
    `AssuranceState` needed it). A cross-surface forward-compat pass, if wanted,
    should touch all three GCTX outcome enums together.
  - Git-diff path derivation in the MCP tool (the spec's optional "may derive
    client-side") is not implemented — the tool takes explicit `changedFiles`.
