# anvil-kernel

The Anvil Rust kernel — file watcher, parser, semantic graph, and policy engine.

## Modules

- **`watcher/`** — file system event monitoring (notify-rs)
- **`parser/`** — source file parsing (tree-sitter)
- **`graph/`** — semantic dependency graph (symbol graph, trust levels,
  incremental updates)
- **`policy/`** — policy configuration, engine, and invariant checks
- **`protocol/`** — event protocol emitter
- **`embedded.rs`** — embedded mode for in-process usage
- **`watch.rs`** — watch loop orchestration
- **`engine_mode.rs`** — feature flag for engine selection

## Benchmarks

```bash
cargo bench -p eddacraft-anvil-kernel
```

## Tests

```bash
cargo test -p eddacraft-anvil-kernel
```

Includes architecture parity tests (`tests/architecture_parity.rs`) and dual-run
tests (`tests/dual_run.rs`).

## Part of

[eddacraft Anvil](../../README.md) monorepo (`crates/anvil-kernel`).
