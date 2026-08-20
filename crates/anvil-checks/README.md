# anvil-checks

| Type   | Authority     | Owner | Status | Freshness                                                                                           |
| ------ | ------------- | ----- | ------ | --------------------------------------------------------------------------------------------------- |
| README | Authoritative | SCAN  | Live   | Last reviewed 2026-08-20 against `f0f834b39`, `src/lib.rs`, `src/surface/**`, and `ARCHITECTURE.md` |

| Upstream                                                  | Downstream                                              |
| --------------------------------------------------------- | ------------------------------------------------------- |
| `src/**`, compiled pattern registry, ADR-029, and ADR-087 | CLI, intercept rules, activation, MCP, and contributors |

Reusable quality checks for performance-sensitive evaluation across anvil's CLI,
kernel, and interception surfaces. Read the source-linked
[local architecture](ARCHITECTURE.md) before changing family boundaries,
suppression, finding shapes, or guarded-byte evaluation. The former central
[checks as-built](../../docs/architecture/checks-as-built.md) is a dated
compatibility and history record.

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
  `# @anvil-ignore SURFENV-001`, plus SQL, Dockerfile, GitHub Actions, and shell
  source-specific checks.
- **`command_safety`** — shell command safety analysis.

## Parallel Scanning

`gate`, `audit`, `check`, `drift`, policy, architecture validation, and the
watcher all share the gitignore-aware discovery walk plus the rayon scan
pattern. First-run scans honour the `ANVIL_SCAN_THREADS` environment variable
and default to `min(num_cpus, 4)` so the parallel walk does not starve TUI or
editor work. Raise the cap on dedicated CI runners; lower it on shared laptops
if you see contention.

## Benchmarks

```bash
cargo bench -p eddacraft-anvil-checks
```

Benchmarks live in `benches/checks.rs`.

## Part of

[eddacraft Anvil](../../README.md) monorepo (`crates/anvil-checks`).
