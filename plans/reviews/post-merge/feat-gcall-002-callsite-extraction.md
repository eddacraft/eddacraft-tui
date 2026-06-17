# Post-merge: feat-gcall-002-callsite-extraction

PR: #NNN
Branch: `feat/gcall-002-callsite-extraction`
APS: GCALL (symbol-call-graph), item GCALL-002
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip GCALL-002 status `In Progress → Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/symbol-call-graph.aps.md`; bump the GCALL module count
      `1/7 → 2/7` (module header + Last-reviewed note + index module-table row)
      and refresh the NBI: GCALL-003 (resident edges + read API) is the next pick
      now that its `GCALL-002` dependency is satisfied. (agent: yes)
- [ ] GCALL-002 → Released/Shipped only on the next release tag that includes
      this commit. (agent: yes — on tag evidence)

## Notes

- Producer side only (ADR-086 §2): the TS/JS extractor emits **unresolved**
  `CallSite`s into `FileSymbols.calls`. Cross-file callee resolution into resident
  `EdgeType::Calls` edges is GCALL-003 (`re_resolve_calls`), and the
  `anvil_find_callers` egress is GCTX-014.
- Implemented as a **separate pass 2** so pass 1's symbol/import/reexport emission
  is byte-identical — the `ts_extractor_parity_snapshot` test is unchanged.
- **Caller attribution uses pass 1's actual emitted symbols**, not a re-derivation:
  pass 1 records each emitted symbol's defining-node byte range (a `spans` vec
  parallel to `symbols`), and pass 2 attributes a call to the innermost containing
  span, reading the caller identity straight from `for_file_symbols`. This was a
  deliberate redesign after an adversarial review found that an independent
  re-recognition (the first cut) minted **phantom callers** for nested
  function/class declarations pass 1 never emits (pass 1 does not recurse
  function/class bodies, but DOES recurse arrow-const bodies — an asymmetry the
  span model captures for free). Regression tests cover nested-in-function,
  nested-in-arrow-const, anonymous `export default function`, and class-field
  initialisers.
- **v1 callee resolution contract** (per ADR-086, deliberately best-effort/static):
  - same-file identifier → `{name, via_import: None}`;
  - imported identifier → export name (alias reverse-mapped) + specifier;
  - namespace member `ns.foo()` → `{foo, specifier}`;
  - default import → `{name: "default", specifier}` (Unresolved at lift);
  - `this.method()` in a class → `Owner.method`;
  - general member `obj.method()` → `{method, None}` (lift likely Unresolved);
  - `require(...)`, computed members `obj[x]()`, IIFEs → no `CallSite` (nothing
    statically nameable).
- **Deferred / known limitations** (accepted under the ADR's heuristic posture):
  - Shadowing (a local `const foo` shadowing `import {foo}`) is not detected —
    the call resolves to the import. Dataflow analysis is out of scope.
  - Duplicate local import names (last-write-wins in the binding table).
  - Getter/setter/computed method names follow pass 1's existing `Owner.method`
    fidelity limits (documented on `SymbolKind::Method`).
  - Rust / Python call-site extraction is GCALL-004 / GCALL-005 (their extractors
    emit no `calls` yet).
