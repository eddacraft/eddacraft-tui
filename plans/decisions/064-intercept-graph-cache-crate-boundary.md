# ADR-064: Extract `anvil-graph-cache` for the daemon save-time graph boundary

## Status

**Proposed** — 2026-06-02. Resolves council blocker **B5** (the hard predecessor
to B1) from the
[daemon-graph review verdict](../reviews/2026-06-01-daemon-graph-council-verdict.md);
gates the start of
[daemon save-time sub-phase A](../execution/2026-06-01-daemon-save-time-subphase-a.md)
Tasks 6/7/8.

## Date

2026-06-02

## Context

[ADR-061](061-save-time-daemon-delta-validation.md) makes the intercept daemon
the save-time validation authority: `validate_paths` certifies changed paths
against a warm per-`WorktreeKey` graph cache. The
[sub-phase A plan](../execution/2026-06-01-daemon-save-time-subphase-a.md) Tasks
6–8 need the daemon (`anvil-intercept`) to hold a `SymbolGraph` +
`DependencyGraph`, apply deltas to it, and run a net-new `certify` over its
reverse-impact closure.

A review council returned **do not start as written** and named this a
compile-blocking, unacknowledged architecture decision (B5, MAJOR; raised
independently by the architect and adversarial reviewers):

> `anvil-intercept` depends on `anvil-kernel-types` only; `anvil-kernel` arrives
> only via **dev-dependencies** (`anvil-checks`). Tasks 6/7/8 require
> `graph::incremental::{update_file, remove_file, re_resolve_imports}` and
> `DependencyGraph` from full `anvil-kernel`. The plan never acknowledges the
> boundary change, its build-weight cost, or the cycle risk.
> **This is a hard predecessor to B1** — the daemon cannot cache
> `DependencyGraph` until it can depend on the crate that defines it.

`crates/anvil-intercept/src/watcher.rs:28` documents the boundary as a
**deliberate refusal**, and states the reason explicitly — it is build-weight,
not a cycle:

> Pulling `eddacraft-anvil-kernel` into `anvil-intercept` would drag in
> `tree-sitter`, `petgraph`, and the parser surface — none of which the daemon
> needs at runtime.

### Cycle audit (done for this ADR)

- `anvil-kernel`'s `[dependencies]` are **`anvil-kernel-types` + `anvil-rayon-init`
  only** — it does **not** depend on `anvil-intercept`, `-proto`, or `-rules`.
- Therefore adding `anvil-kernel → anvil-intercept` would create **no dependency
  cycle**. The `watcher.rs:28` refusal is a build-weight / dependency-honesty
  argument, not a cycle constraint.

### What the daemon actually needs (read vs. write)

The graph algorithm layer is **already parse-free**:

- `update_file(graph: &mut SymbolGraph, new_symbols: FileSymbols) -> GraphDelta`
  and `re_resolve_imports(graph, imports: &[ImportEdge])` **take already-parsed
  input** — they never call the parser. Parsing (tree-sitter) lives only in the
  parser *functions* (`parser/extract/mod.rs`), which produce `FileSymbols`.
- `FileSymbols` and `ImportEdge` are **plain data structs** (`String`,
  `Vec<SymbolNode>`, `u32`). `SymbolNode` / `SymbolEdge` / `EdgeType` already
  live in the shared `anvil-kernel-types::graph` (pure: `serde` only). Only the
  *declaration site* of `ImportEdge` / `FileSymbols` couples them to the parser
  crate.
- The daemon **hot path only reads** the cache: `certify` walks
  `DependencyGraph::dependents_of` (a `petgraph` traversal). `dependents_of` has
  **zero non-test callers today** (council B1: the reverse index is net-new, not
  "existing/O(1)"). Cache **writes** (`update_file` with `FileSymbols`) require
  an upstream parse, which stays in `anvil-kernel`.

So the load-bearing question is narrow: **what crate hosts `SymbolGraph`,
`DependencyGraph`, the incremental apply-delta logic, and the net-new `certify`,
such that `anvil-intercept` can depend on it without inheriting the tree-sitter
parser surface?**

## Decision

**Extract a new internal crate `eddacraft-anvil-graph-cache` (path
`crates/anvil-graph-cache`)** that owns the graph state and algorithms, and have
both `anvil-kernel` and `anvil-intercept` depend on it. Parsing stays in
`anvil-kernel`.

1. **Relocate the two plain graph-data types** `ImportEdge` and `FileSymbols`
   from `anvil-kernel/src/parser/extract/mod.rs` into
   `anvil-kernel-types::graph` (joining `SymbolNode` / `SymbolEdge` / `EdgeType`).
   They carry no parser logic. The parser re-exports or imports them from
   `anvil-kernel-types` so existing `crate::parser::extract::{ImportEdge,
   FileSymbols}` paths keep resolving during migration.

2. **Move `crates/anvil-kernel/src/graph/` →
   `crates/anvil-graph-cache/src/`** (`symbol_graph.rs`, `dependency.rs`,
   `incremental.rs`, `trust.rs`, `mod.rs`). The new crate's `[dependencies]` are
   **`anvil-kernel-types` + `petgraph` + `serde` + `thiserror`** — **no
   `tree-sitter`, `notify`, `walkdir`, `ignore`, or `rayon`**.

3. **`anvil-kernel` depends on `anvil-graph-cache`** and re-exports it at the
   existing `anvil_kernel::graph` path (`pub use anvil_graph_cache as graph;` or
   targeted re-exports) so internal kernel call sites and the parser→graph
   feed (`update_file(graph, parsed_file_symbols)`) are unchanged. Tree-sitter,
   the watcher, and the directory walk remain kernel-only.

4. **`anvil-intercept` adds `anvil-graph-cache` to `[dependencies]`** —
   inheriting `petgraph` only (a pure-Rust, no-native-dep crate **already in the
   binary's tree** via `anvil-kernel`). It does **not** add `anvil-kernel`.
   `watcher.rs:28` is updated to record that the boundary is now held at
   `anvil-graph-cache` (read-only graph state), and that the parser surface is
   still deliberately excluded.

5. **`certify` (sub-phase A Task 6, net-new) lands in `anvil-graph-cache`** with
   the council B1 signature
   `certify(sym: &SymbolGraph, dep: &DependencyGraph, change, delta, budget)`,
   and the daemon's per-`WorktreeKey` cache (Task 7) holds the
   `(SymbolGraph, DependencyGraph)` pair (B1). This unblocks B1 directly.

This is the **`certify`/read** boundary only. Who drives cache **writes** (the
kernel-side warming path feeding `FileSymbols` deltas vs. an interim daemon-side
build) is a sub-phase A task detail, not fixed by this ADR — but the crate split
guarantees the write path that *does* parse stays in `anvil-kernel`, off the
daemon hot path, consistent with [ADR-063](063-gv2-hot-path-boundary.md) ("hot
path does no parse").

## Rationale

Both options are cycle-free, so the decision turns on **build weight** and
**architectural honesty**, and the evidence is one-sided:

- Option A pulls the entire parser surface — `tree-sitter` +
  `tree-sitter-typescript` + `tree-sitter-javascript` (native build steps),
  `notify`, `walkdir`, `ignore`, `rayon` — into the always-resident daemon
  binary, for code the daemon never runs. That directly contradicts the
  documented `watcher.rs:28` stance and bloats the process ADR-061 wants lean.
- Option B's cost is small and bounded **because the layering already exists**:
  the graph algorithms are parse-free and consume only plain data types, two of
  which (`SymbolNode`/`SymbolEdge`) are *already* in the shared crate. The
  extraction moves five files and two structs; it adds no new external
  dependency to the binary (`petgraph` is already present transitively).
- Option B also gives the **net-new `certify`** and the future **GV2 hot-read
  API** (sub-phase A′, [ADR-063](063-gv2-hot-path-boundary.md)) a natural
  parser-free home, instead of cementing a parser dependency into the daemon
  that A′ would then have to claw back.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **B — extract `anvil-graph-cache` (chosen)** | Keeps tree-sitter/parser/notify/walkdir out of the daemon; honours `watcher.rs:28`; parser-free home for `certify` + GV2 A′ hot-read API; no new external dep in the binary; layering already parse-free so the move is mechanical | One new crate + workspace wiring; relocate 2 structs + 5 files; repoint internal import paths; `cargo hakari` regen |
| **A — add `anvil-kernel` to `anvil-intercept/[dependencies]`** | One-line `Cargo.toml` change; no file moves | Drags 3 tree-sitter grammars (native builds) + `notify` + `walkdir` + `ignore` + `rayon` + the full parser surface into the resident daemon for code it never runs; reverses the documented deliberate boundary; bloats daemon cold-start and binary; A′ would have to undo it |
| **C — host the graph cache in `anvil-kernel-types`** | No new crate; both already depend on it | `anvil-kernel-types` is a deliberately minimal, dependency-light type crate (`serde` only); adding `petgraph` + the incremental-mutation + `certify` algorithm surface turns a shared "types" crate into a logic crate every consumer inherits — wrong home for stateful graph algorithms |

## Consequences

- **Positive:** B1 is unblocked — the daemon can hold `(SymbolGraph,
  DependencyGraph)` and run `certify` against a crate it legitimately depends on.
  The daemon binary stays free of the tree-sitter/parser surface. `certify` and
  the GV2 A′ hot-read API get a clean, parser-free home. The
  `anvil-kernel::graph` re-export keeps kernel call sites unchanged.
- **Negative:** One more workspace crate to maintain; a one-time mechanical
  refactor (move `graph/`, relocate two structs, repoint paths) touching
  `anvil-kernel`, `anvil-kernel-types`, and `anvil-intercept`.
- **Risks:** (1) A missed internal import path breaks the `anvil-kernel` build —
  mitigated by the `pub use` re-export and a workspace `cargo build`/`clippy`
  gate. (2) `cargo hakari` workspace-hack drift after adding a crate — regenerate
  and `cargo hakari verify` in the same PR. (3) `ACKNOWLEDGEMENTS` churn —
  expected **none** (no new *external* crate enters the shipped tree; `petgraph`
  is already present), but re-run the generator to confirm.
- **Out of scope:** This ADR does not resolve the sibling security major B5-notes
  (`confinement.rs` placed in `anvil-intercept` while the `ANVIL_HOME` resolver
  lives in `anvil-cli`) — that is a different wrong-direction dependency about
  confinement, not the graph cache, and is tracked separately in the verdict.

## Migration path

1. Add `ImportEdge` / `FileSymbols` to `anvil-kernel-types::graph`; re-export
   from `parser::extract` for source compatibility. Build + test `anvil-kernel`.
2. Create `crates/anvil-graph-cache`; move `graph/*.rs` into it; deps
   `anvil-kernel-types` + `petgraph` + `serde` + `thiserror`. Build + test in
   isolation (`cargo test -p eddacraft-anvil-graph-cache`).
3. `anvil-kernel`: depend on `anvil-graph-cache`, re-export as
   `anvil_kernel::graph`. Full `cargo test -p eddacraft-anvil-kernel`.
4. `anvil-intercept`: add `anvil-graph-cache`; update `watcher.rs:28` doc.
5. `cargo hakari generate && cargo hakari verify`; regenerate `ACKNOWLEDGEMENTS`
   to confirm no change; `cargo clippy --workspace --all-targets -- -D warnings`;
   `cargo fmt --all --check`.

## References

- Related ADRs: [ADR-061](061-save-time-daemon-delta-validation.md) (save-time
  daemon delta validation), [ADR-063](063-gv2-hot-path-boundary.md) (GV2
  hot-/non-hot-path boundary),
  [ADR-036](036-daemon-scope-discovery-and-boundaries.md) (daemon scope &
  boundaries)
- Council verdict: B5 + Action 1
  ([`plans/reviews/2026-06-01-daemon-graph-council-verdict.md`](../reviews/2026-06-01-daemon-graph-council-verdict.md))
- Execution plan:
  [`plans/execution/2026-06-01-daemon-save-time-subphase-a.md`](../execution/2026-06-01-daemon-save-time-subphase-a.md)
  (Tasks 6/7/8; correction §B5)
- APS modules: RLB (resource-load-benchmarking), GV2 (graph-v2-foundation)
- Evidence: `crates/anvil-intercept/src/watcher.rs:28`;
  `crates/anvil-intercept/Cargo.toml`; `crates/anvil-kernel/Cargo.toml`;
  `crates/anvil-kernel/src/graph/{symbol_graph,dependency,incremental}.rs`;
  `crates/anvil-kernel/src/parser/extract/mod.rs`;
  `crates/anvil-kernel-types/src/graph.rs`
