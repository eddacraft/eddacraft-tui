# ADR-086: Save-time symbol-level call-graph substrate

## Status

Accepted (operator, 2026-06-17). The gating design decision for the
`symbol-call-graph` (GCALL) module — ratifies the call-edge model, the
`FileSymbols` `calls` contract, the ADR-031 budget posture, and the PV-9
caller-egress posture that the rest of the module (GCALL-002..007) builds on.

## Date

2026-06-17

## Context

`anvil_find_callers` (GCTX-014) — "who calls this symbol" — is **Blocked**: the
warm graph carries **no symbol-level call edges**. `EdgeType::Calls` and
`EdgeType::References` exist in `anvil-kernel-types`
(`crates/anvil-kernel-types/src/graph.rs:124`) but are never emitted; the kernel
extractor produces only `symbols` / `imports` / `reexports`, and `FileSymbols`
(the `apply_delta` producer feed, `graph.rs:188`) carries only those three
channels. So a true caller traversal cannot be projected today (this is exactly
why GCTX-011 split caller traversal out into GCTX-014, and GCTX-014 out to this
module).

The dependency graph already proves the pattern this substrate mirrors, end to
end:

- **Producer.** The per-language extractor (`crates/anvil-kernel/src/parser/extract/`)
  walks a tree-sitter tree once and emits `FileSymbols` (symbols + `ImportEdge`s
  + `ReexportEdge`s). It is the only place that parses (ADR-064/067 keep
  tree-sitter out of the daemon).
- **Resident lift.** `apply_delta` (`anvil-graph-cache` `update_file` +
  `anvil-intercept` `kernel_cache.rs`) lifts a file's imports into resident
  edges incrementally: it removes the file, re-adds its symbols, then resolves
  each import specifier against an `all_imports` accumulator + the resident file
  set (`re_resolve_imports`, `resolve_import`), so a forward reference resolves
  when its target later becomes resident. Cost is O(local neighbourhood), not a
  whole-graph re-derive (GV2-011).
- **Read.** `DependencyGraph::dependents_of` / `dependencies_of` answer
  file-level "who imports me" from a HashMap reverse index — **not** a petgraph
  traversal — which is what keeps it inside the hot certify budget (ADR-063/064).
  GCTX-011/012/013 then project bounded, identity-only, depth-clamped reverse
  walks over it.

A call graph is the same shape one level finer: **caller symbol → callee symbol**
instead of **file → file**. The forces:

- **Identity, not text.** Stable identity is `SymbolIdentity { file, kind, name,
  ordinal }` (`graph.rs:75`), where `ordinal` is the occurrence index among
  same-`(kind, name)` symbols in a file in parse order
  (`SymbolIdentity::for_file_symbols`, `graph.rs:104`); methods encode their
  owner as `Owner.method`. There is no type information and no signature in the
  identity (PV-1). Callee resolution must work from names + the import graph
  alone, statically — it cannot do type inference or dynamic-dispatch resolution.
- **Save-time budget.** Extraction runs on the interactive save path, which
  ADR-031 caps at **80 ms `validation.service` p95 / 120 ms roundtrip**. Adding
  call-site walking to the existing parse must stay inside that envelope, and the
  hot certify path must not grow (ADR-063: the hot path does no graph traversal).
- **Egress privacy.** The PV-9 context-egress review
  (`plans/reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md`) made
  GCTX egress **identity-only by default** through a single sealed-DTO choke
  point (CE-5), with source text a gated escalation (CE-1). "Who calls this"
  exposes a new relationship and needs the same posture before any assistant sees
  it.

A decision is needed now because GCALL-002 (extraction) and GCALL-003 (resident
edges + read API) cannot start until the call-edge model, the `FileSymbols`
contract, the budget posture, and the egress posture are fixed.

## Decision

Add a **save-time, static, identity-only symbol call graph** on the existing
substrate. Four contracts:

### 1. Call-edge model — caller `SymbolIdentity` → callee `SymbolIdentity`, best-effort static

A call edge is a directed `EdgeType::Calls` edge from the **enclosing caller
symbol** to the **resolved callee symbol**, both `SymbolIdentity`. The graph is
explicitly **heuristic and static**: it records the call relationships a
name-and-import analysis can prove, and nothing it cannot.

- **Caller attribution is the nearest enclosing named symbol.** A call site
  attributes to the nearest enclosing `SymbolNode` (function, method). A call site
  with **no** enclosing named symbol — module top-level, or inside an anonymous
  function/closure not bound to a named symbol — attributes to a **synthetic
  file-scope caller**: the `SymbolKind::Module` node `update_file` already
  materialises for a file (today only for side-effect-only files; this ADR makes
  the resolver reuse that one node for any file with top-level calls, never a
  second node). `LocalSymbolRef` carries a `module_scope: bool` discriminator for
  this case so the lift-time resolver binds to the file's synthetic node rather
  than a `(kind, name, ordinal)` lookup. This makes every call site attributable
  and decidable; it never silently drops a top-level call.
- **Resolution is best-effort.** A call site resolves to one of:
  - `Resolved(SymbolIdentity)` — the callee name binds unambiguously to a single
    resident symbol (same-file scope, or an imported binding whose module + name
    resolve to a resident definition).
  - `AmbiguousOverload(Vec<SymbolIdentity>)` — the name binds to several
    same-`(file, kind, name)` overloads the static analysis cannot disambiguate
    (no type info). The edge **fans out** to every candidate, **capped** at
    `MAX_OVERLOAD_FANOUT` (a name resolving to more candidates than the cap is
    treated as `Unresolved` + `partial`, so a pathological name cannot multiply
    edge count without bound). Every read result derived from a fan-out edge
    carries `heuristic: true` (the GCTX-013 marker) so a consumer never treats an
    over-included caller as exact. "Who calls X" over-includes rather than misses
    a caller — the conservative, heuristic-honest direction.
  - `Unresolved` — dynamic dispatch, an external/unknown callee, a default-export
    callee (v1 resolves **named** bindings only), or a target not (yet) resident.
    **No resident edge is produced**; the call contributes to no in-repo caller
    set. The read API marks results whose target file had unresolved call sites as
    `partial` so a consumer never reads "0 callers" as proof of none.
- **Cycles and recursion are first-class.** `a → b → a` and direct recursion
  `a → a` (self-edge) are valid edges. The graph may be cyclic; termination is a
  **read-side** guarantee (see the read API below), never enforced by dropping
  edges.
- **Read API shape (the substrate GCTX-014 consumes).** A read-only
  `callers_of(target: SymbolIdentity, depth)` over the resident **incoming**
  `Calls` edges, mirroring `collect_dependents` exactly: a breadth-first walk
  clamped by the GV2-026 `clamp_reverse_impact_depth` / `MAX_REVERSE_IMPACT_DEPTH`
  lever and a node budget, with a `seen` set (so cycles/recursion terminate),
  **sorted frontiers** (so an over-budget truncation keeps a deterministic
  path-ordered prefix), and truncation metadata. Returns calling symbol identity,
  source file, distance, and `partial`/`truncated` markers — never source text.
  This lives in `anvil-graph-cache` (it reads the resident `SymbolGraph`) and is
  surfaced over the daemon `GctxDispatch` RPC like `find_dependents`.

### 2. `FileSymbols` extension — a `calls` channel, resolved at lift time

Extend `FileSymbols` with a fourth channel, backward-compatible exactly like
`reexports` (GV2-010):

```rust
pub struct FileSymbols {
    pub file: String,
    pub symbols: Vec<SymbolNode>,
    pub imports: Vec<ImportEdge>,
    #[serde(default)]
    pub reexports: Vec<ReexportEdge>,
    #[serde(default)]                 // older snapshots deserialize unchanged
    pub calls: Vec<CallSite>,
}
```

`CallSite` carries **file-local, unresolved** call data — the extractor knows the
enclosing symbol and the callee name, but not cross-file callee identity:

```rust
pub struct CallSite {
    /// The enclosing caller, by file-local identity. `module_scope: true` marks a
    /// top-level / anonymous-closure caller that binds to the file's synthetic
    /// Module node (then kind/name/ordinal are ignored).
    pub from: LocalSymbolRef,        // { kind: SymbolKind, name: String, ordinal: u32, module_scope: bool }
    /// The callee as seen at the call site, to be resolved at lift time. `name` is
    /// the **target module's export name** (the extractor reverse-maps a local
    /// alias `import {foo as bar}` → `foo` using the file's own import table);
    /// `via_import` is the module specifier (the namespace specifier for
    /// `import * as ns; ns.foo()`); `None` means same-file scope.
    pub callee: CalleeRef,           // { name: String, via_import: Option<String> }
    pub line: u32,
}
```

- **Caller resolution.** The extractor MUST assign `from`'s `(kind, name,
  ordinal)` by calling `SymbolIdentity::for_file_symbols` over the **same**
  `symbols` slice it puts in `FileSymbols.symbols` — never an independently
  recomputed ordinal — so the extractor's ordinal and the lift-time
  `for_file_symbols` ordinal are identical by construction. At lift time the
  resolver maps `from` → the symbol's session-local `u64` id by matching
  `(kind, name, ordinal)` over the file's just-added symbols (or the synthetic
  Module node when `module_scope`).
- **Callee resolution** happens at **lift time, not in the extractor**, through a
  **distinct** `re_resolve_calls` resolver (not `re_resolve_imports`, whose
  file-level `first()`-symbol source is wrong for symbol-granular edges). It
  reuses the `resolve_import` **module** hop and the accumulator **pattern**, then
  resolves the export name to a target symbol id:
  - `via_import: None` → resolve `name` in same-file scope.
  - `via_import: Some(specifier)` → `resolve_import` the specifier to a resident
    file, then match the **export name** `name` to a defined symbol there. A
    barrel/re-export target (the name is `Reexports`-edged, not defined locally)
    is followed across resident `Reexports` edges up to `MAX_REEXPORT_HOPS`, then
    `Unresolved`. Default-export and dynamic targets are `Unresolved` in v1.
  - Unresolved-because-not-yet-resident calls re-resolve when the target later
    lands (forward references), via the `all_calls` accumulator.
- **Re-resolution scope (incremental, bounded).** When a **callee** file is saved,
  petgraph node removal in `remove_file` drops the incoming `Calls` edges that
  pointed at its old symbol ids, so they must be re-issued. This is scoped to the
  callee's importers — a call edge into a file can originate only from a file that
  imports it — so `re_resolve_calls` runs over `dependents_of(callee)` (the same
  `affected` set `apply_delta` already iterates for dependency refresh), **not**
  the whole workspace: O(importers × their call sites), preserving the
  local-neighbourhood budget. The `all_calls` accumulator is keyed by caller file
  for this lookup.
- The resolver emits resident `EdgeType::Calls` edges into the `SymbolGraph` and
  threads `added_edges` / `removed_edges` through `GraphDelta` like every other
  edge channel. Stale `all_calls` entries for a deleted callee persist until the
  caller is next saved and resolve to `Unresolved` (no edge) — inherited
  `all_imports` behaviour, not a new mechanism.

GCALL-002 fixes the TS/JS extraction details; GCALL-004/005 extend to Rust /
Python; the `CallSite` / `CalleeRef` shapes here are the cross-language contract.
The named patterns above (alias reverse-map, namespace member, barrel-follow with
cap, default-export = Unresolved) are the v1 resolution contract, so per-language
work does not invent undocumented behaviour.

### 3. Save-time budget posture (ADR-031 / ADR-063)

- **Extraction is additive to the existing parse.** Call-site walking is another
  node-kind match in the same single tree-sitter pass that already emits symbols
  (O(tree nodes)); it produces no extra parse.
- **The lift is incremental** (O(call sites in the changed file + import-graph
  resolution)), mirroring import resolution — never a whole-graph re-derive.
- **The hot certify path does not change.** Certify reads only the dependency
  reverse index (ADR-063/064); it never reads `Calls` edges. Call edges are a
  **consumer-only** surface (GCTX-014), off the enforcement hot path.
- **The combined save-time cost stays inside the ADR-031 interactive budget
  (80 ms `validation.service` p95 / 120 ms roundtrip)**, guarded by GCALL-006's
  CI latency gate using the established `harness=false` bench + exit-code pattern
  in `resource-budget.yml`.
- **Per-file call-site cap (no silent breach, no double parse).** Call extraction
  lives in the one parse pass that already emits symbols — there is no separate
  "calls-only" extraction path and the daemon never parses twice (ADR-064/067). A
  pathological, call-dense file is bounded by a per-file `MAX_CALL_SITES` cap: over
  the cap, the file's call sites are truncated and its call data is marked
  `partial`, so a single file can never blow the budget, and the truncation is
  surfaced rather than silent. The cap is the budget escape hatch — not a separate
  code path.
- **Cold warming rides the existing executor.** Call edges warm on a fresh /
  evicted graph exactly as symbols and imports do today: the ADR-085 full-scan
  executor feeds files through the same extractor on the background pool. Until a
  file's call data is warm the read API reports `partial` / not-ready (CE-7), never
  a whole-file fallback. This is the normal cold-warm path, not a budget fallback.

### 4. Caller-egress privacy posture (PV-9)

- **Identity-only by default.** The egress surface (GCTX-014) returns calling
  symbol `SymbolIdentity`, workspace-relative source file, traversal distance, and
  truncation/partial markers — **no call-site source text, no byte spans, no
  session-local `u64` node/edge ids**.
- **Same sealed-DTO choke point (CE-5).** Caller egress is projected through the
  single `GctxProjector` boundary as a sealed `anvil-gctx-types` DTO, structurally
  incapable of carrying source text (the GCTX no-leak test extends to it). The
  resident graph's internal ids are never egressed raw.
- **Snippets are a gated escalation.** Any future call-site or caller-body text
  egress requires the CE-1 opt-in + CE-2/CE-3 secret/path redaction, exactly like
  GCTX snippets; it is **out of scope** here.
- **GCALL-007 ratifies.** GCALL-007 runs a PV-style review modelled on PV-9 and
  folds its conditions into GCTX-014's acceptance criteria. This ADR fixes the
  posture (identity-only default, sealed DTO, single choke point, snippets gated);
  GCALL-007 confirms it before any assistant-facing surface ships.

## Rationale

The dependency graph already validated producer → incremental-lift →
bounded-identity-read for file-level edges; the call graph is the same machine at
symbol granularity, so the lowest-risk design is to **reuse every seam** —
`FileSymbols` channel + `serde(default)`, the `all_*` accumulator +
re-resolution, `EdgeType::Calls` (which already exists), and the
`collect_dependents`-shaped bounded read — rather than invent a parallel
substrate.

Resolving callees **at lift time** (not in the extractor) is the load-bearing
choice: the extractor is per-file and has no cross-file import graph, so
extractor-side resolution would either miss cross-file callees or duplicate the
import resolver. Lift-time resolution reuses the `resolve_import` module hop and
the forward-reference accumulator *pattern*, but through a **distinct**
symbol-granular resolver (`re_resolve_calls`) — the existing `re_resolve_imports`
takes a file's `first()` symbol as the edge source, which is correct for
file-level import edges but wrong for symbol-to-symbol call edges, so the call
resolver does its own per-symbol `(kind, name, ordinal)` → id lookup on both
ends.

Framing the graph as **explicitly heuristic/static** (over-include on overloads,
drop-and-mark on unresolved) keeps "who calls this" honest under the constraint
that there is no type information — and matches how GCTX already presents its
results (e.g. GCTX-013 `heuristic: true`). It also keeps the hard semantics
problems (overloads, dynamic dispatch) out of the budget-sensitive save path.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Lift-time callee resolution via a distinct symbol-granular `re_resolve_calls` (chosen)** | Reuses the `resolve_import` module hop + accumulator pattern + the `dependents_of` affected-set locality; handles forward references; incremental; extractor stays per-file and parse-only | A new resolver (not a reuse of `re_resolve_imports`, whose `first()`-symbol source is file-level); resolution logic lives in `anvil-graph-cache`, not co-located with extraction |
| Resolve callees in the extractor | Call edges fully formed in `FileSymbols` | Extractor has no cross-file import graph → misses cross-file callees or duplicates the resolver; breaks on forward references |
| Type-aware resolution (infer overloads / dynamic dispatch) | Precise edges | Not deterministic-cheap; needs a type system; blows the ADR-031 budget; not static |
| Separate `CallGraph` struct (not `SymbolGraph` edges) | Isolated | Duplicates symbol-node storage; `EdgeType::Calls` already exists on `SymbolGraph`; two graphs to keep consistent |
| Drop unresolved/ambiguous calls silently | Simpler API | "0 callers" becomes a false negative; hides heuristic incompleteness — rejected for `partial` marking + overload fan-out |

## Consequences

- **Positive:** Unblocks GCTX-014 with a deterministic, bounded, identity-only
  caller traversal. No new crate, no new graph store, no new egress spine — reuses
  the GV2 dependency-graph and GCTX-010 projector seams. The model is
  language-agnostic at the `CallSite`/`CalleeRef` contract; per-language work is
  additive (GCALL-002/004/005).
- **Negative:** `FileSymbols` grows a fourth channel (snapshot size + extraction
  cost rise with call density). Callee resolution adds import-resolution-shaped
  work to `apply_delta`. Overload fan-out can over-report callers.
- **Risks:** (a) Save-time budget — call-dense files could push extraction past
  ADR-031. (b) Resolution quality — alias / namespace / re-export / barrel chains
  and default exports may leave many calls `Unresolved`, weakening the answer.
  (c) Overload fan-out can attach a caller to a sibling overload (false caller).
  (d) Cyclic graphs must terminate every read. (e) Caller-ordinal skew between the
  extractor and the lift-time `for_file_symbols`.
- **Mitigations:** (a) GCALL-006 gates the budget with a CI latency bench; the
  per-file `MAX_CALL_SITES` cap bounds any single file and cold warming rides the
  existing executor (§3) — no double parse. (b) Resolution is explicitly
  best-effort and `partial`-marked, with the v1 resolution contract (§2) naming
  exactly which patterns resolve vs. fall to `Unresolved`; quality improves
  additively without a model change. (c) Fan-out is capped (`MAX_OVERLOAD_FANOUT`)
  and every fan-out-derived result carries `heuristic: true`, so over-inclusion is
  surfaced, not silent. (d) The read API's `seen` set + GV2-026 depth clamp + node
  budget + sorted frontiers guarantee bounded, deterministic termination on any
  graph (the proven GCTX-011/013 pattern). (e) The extractor MUST derive caller
  ordinals from `for_file_symbols` over the emitted `symbols` slice (§2), so the
  two passes agree by construction; GCALL-003 fixtures assert order-independent
  convergence (GV2-011) against a save-driven baseline.

## References

- Related ADRs: ADR-031 (save-time latency rubric — 80 ms p95), ADR-063 (hot path
  does no traversal), ADR-064 (`anvil-graph-cache` parse-free daemon boundary),
  ADR-067 (injected `SymbolParser` seam), ADR-084 (GCTX daemon-RPC + `GctxProjector`
  egress spine), ADR-085 (full-scan executor — the background-warm fallback path)
- APS modules: GCALL-001 (this ADR), GCALL-002..007 (`plans/modules/symbol-call-graph.aps.md`),
  GCTX-014 (`anvil_find_callers`, the consumer), GV2-026 (reverse-impact depth lever)
- Reviews: `plans/reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md`
  (PV-9 / CE-1..CE-12 — the caller-egress posture GCALL-007 ratifies)
- Substrate: `crates/anvil-kernel-types/src/graph.rs` (`FileSymbols`, `EdgeType`,
  `SymbolIdentity`), `crates/anvil-graph-cache/src/incremental.rs` (`update_file`,
  `resolve_import`, `re_resolve_imports`), `crates/anvil-graph-cache/src/dependency.rs`
  (the read API this mirrors)
