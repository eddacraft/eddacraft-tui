# anvil graph-cache

| Type   | Authority     | Owner | Status | Freshness                                                                     |
| ------ | ------------- | ----- | ------ | ----------------------------------------------------------------------------- |
| README | Authoritative | GV2   | Live   | Last reviewed 2026-08-29 against `src/lib.rs`, ADR-064, and `ARCHITECTURE.md` |

| Upstream                                                                                                                 | Downstream                                                                                           |
| ------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| `crates/anvil-graph-cache/src/**`, `crates/anvil-kernel-types/src/graph.rs`, ADR-064, ADR-063, ADR-069, ADR-077, ADR-105 | Kernel watch and embedded paths, intercept daemon, GCTX projectors, component architecture consumers |

This crate is the parser-free semantic graph for anvil. It stores symbols and
edges, maintains a file-level dependency projection, annotates trust, certifies
save-time deltas, and persists warm snapshots. GV2 owns the component. The crate
consumes already-extracted `FileSymbols`; it never parses. `anvil-kernel`
re-exports it as `graph`.

It does not own tree-sitter extraction, architecture-layer YAML, MCP transport,
or GCTX DTO projection.

## Entry points

- [`src/lib.rs`](src/lib.rs) re-exports the public graph surfaces.
- [`src/symbol_graph.rs`](src/symbol_graph.rs) is the petgraph-backed symbol
  graph.
- [`src/incremental.rs`](src/incremental.rs) applies per-file updates and
  resolves imports, calls, and re-exports.
- [`src/dependency.rs`](src/dependency.rs) maintains the file-to-file
  projection.
- [`src/hot_index.rs`](src/hot_index.rs) and [`src/certify.rs`](src/certify.rs)
  are the save-time read and certifiability surfaces.

## Local validation

```bash
cargo test -p eddacraft-anvil-graph-cache --no-fail-fast
```

Run the `hot_read` and `call_lift` benches only when changing those
latency-gated paths.

## Architecture and authorities

Read the source-linked [local architecture](ARCHITECTURE.md) before changing
graph mutation, snapshots, or hot-path reads. Kernel orchestration lives in
[`../anvil-kernel/ARCHITECTURE.md`](../anvil-kernel/ARCHITECTURE.md). Save-time
use lives in
[`../anvil-intercept/ARCHITECTURE.md`](../anvil-intercept/ARCHITECTURE.md).
Assistant projection is the
[AI context delivery guide](../../docs/guides/ai-context-delivery.md). Crate
placement is
[ADR-064](../../plans/decisions/064-intercept-graph-cache-crate-boundary.md).
