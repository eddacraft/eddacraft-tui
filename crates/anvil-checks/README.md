# anvil-checks

Quality gate checks ported from TypeScript to Rust for performance-critical
evaluation in the Anvil kernel.

## Modules

- **`secret`** — secret/credential detection in source files
- **`antipattern`** — anti-pattern detection (unsafe code patterns, known bad
  practices)
- **`command_safety`** — shell command safety analysis

## Benchmarks

```bash
cargo bench -p eddacraft-anvil-checks
```

Benchmarks live in `benches/checks.rs`.

## Part of

[EddaCraft Anvil](../../README.md) monorepo (`crates/anvil-checks`).
