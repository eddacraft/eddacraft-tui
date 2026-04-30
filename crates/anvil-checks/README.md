# anvil-checks

Quality gate checks ported from TypeScript to Rust for performance-critical
evaluation in the Anvil kernel.

## Modules

- **`secret`** — secret/credential detection in source files, with a
  `max_line_bytes` ReDoS guard (default 4096 bytes) that skips oversized lines
  before regex evaluation and reports skipped counts through
  `SecretCheckResult`.
- **`antipattern`** — registry-backed anti-pattern detection (unsafe code
  patterns, known bad practices). Every shipped rule flows through the compiled
  `.anvil` registry at `patterns/compiled/registry.json`, and rule provenance is
  attached to every finding.
- **`reasoning`** — AI-001 reasoning-category rule that flags appeal-to-
  authority comments at info severity, scoped to comment regions and honouring
  `// @anvil-ignore AI-001 -- <reason>`.
- **`surface`** — `.env`, `.env.*`, and `.envrc` parsing (SURFENV-001) that
  routes values through the existing secret patterns and reports findings with
  the variable name and source line; suppress with
  `# @anvil-ignore SURFENV-001`.
- **`command_safety`** — shell command safety analysis.

## Parallel Scanning

`gate`, `audit`, `check`, `drift`, policy, architecture validation, and the
watcher all share the gitignore-aware discovery walk plus the rayon scan
pattern. First-run scans honour the `ANVIL_SCAN_THREADS` environment variable
and default to `min(num_cpus, 4)` so the parallel walk does not starve TUI or
editor work. Raise the cap on dedicated CI runners; lower it on shared laptops
if you see contention.

The 0.5.0-beta SCAN benchmark recorded a 7.39× wall-time improvement on a
synthetic 3,000-file surface over the previous serial scan baseline.

## Benchmarks

```bash
cargo bench -p eddacraft-anvil-checks
```

Benchmarks live in `benches/checks.rs`.

## Part of

[eddacraft Anvil](../../README.md) monorepo (`crates/anvil-checks`).
