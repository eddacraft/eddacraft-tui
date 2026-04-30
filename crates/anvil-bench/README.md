# anvil-bench

Stress test and benchmark harness for `anvil-kernel`. Generates synthetic
repositories of configurable size and language mix, measures timing and memory,
and produces structured JSON reports.

## Status

Active

## Modules

| Module      | Description                                                                  |
| ----------- | ---------------------------------------------------------------------------- |
| `fixture`   | Synthetic repo generation with configurable file counts and language weights |
| `measure`   | Timing and memory measurement (RSS/VM via `/proc/self/status`)               |
| `report`    | Structured `ScenarioResult` output (JSON, human-readable)                    |
| `scenarios` | Stress test scenarios                                                        |

## Scenarios

- **watcher_saturation** -- File watcher throughput under high churn
- **graph_memory** -- Symbol graph memory scaling
- **incremental_throughput** -- Incremental re-analysis performance
- **policy_scaling** -- Policy evaluation scaling with rule count
- **cold_start_scaling** -- Cold start time vs repository size

## Benchmarks

- **stress** -- Kernel-level stress scenarios (graph, policy, cold start,
  incremental throughput).
- **antipattern_scan** -- Parallel anti-pattern scan throughput on a
  320-artifact synthetic corpus spanning every `ArtifactKind`. Exercises the
  rayon fan-out plus every enabled registry rule (including the SPG-003
  post-filter rules and the `flags:"i"` inline prefix). This is the CI guard for
  the parallel-scan claim in ADR-026.
- **secret_scan_parallel** -- Serial vs parallel throughput on the secret scan
  path. Validates the parallel rollout speedup claim.

Run a single bench:

```bash
cargo bench -p anvil-bench --bench antipattern_scan
```

### v0.5.0-beta baseline summary

The 0.5.0-beta release captures two new headline numbers in this harness:

- **SCAN parallel scan** — 7.39× wall-time improvement on a synthetic 3,000-file
  surface over the previous serial baseline. The benchmark exercises the shared
  gitignore-aware discovery walk plus the rayon scan pattern that `gate`,
  `audit`, `check`, `drift`, policy, architecture validation, and the watcher
  all consume.
- **RTAI-001 mid-edit secret-detection roundtrip** — measured at about 1.4 ms
  p95 over 1024 iterations, roughly 60× under the ADR-031 warm-path budget. The
  benchmark exercises a single `scan_buffer` method with a mode discriminator
  selecting save-time versus mid-edit validation.

The per-bench detail below is the dev-box baseline that produced these
release-level numbers; future releases extend the table the same way.

### antipattern_scan baseline

Collected on an Ubuntu 25.04 / Linux 6.17 / rayon default thread pool (recorded
2026-04-28, local dev machine, `dev` branch post-rmcp stdio tool protocol):

- Corpus: 320 synthetic artifacts, mixed kinds:
  - 60% source (`.ts`) cycling 6 content variants — exercises AP-\*,
    DD-001/002/003, GS-001.
  - 20% pr-description cycling 6 variants — exercises DD-004, RL-001/002/
    003/004/005 (and their `flags:"i"` handling + post-filters).
  - 10% commit-message cycling 3 variants — exercises RL-002/005.
  - 10% agent-output cycling 3 variants — exercises DD-004 and RL-\*.
- Throughput: ~39.9 K artifacts/sec (≈8.0 ms per full corpus pass).
- ~42% throughput improvement vs 2026-04-25 baseline (28.6 K/sec, 11.2 ms).
- Criterion-verified: −29.6% time, +42.1% throughput (p < 0.05).

Previous baseline (2026-04-25, `release/v0.4.0-beta` post-RUSTNX-008): ~28.6 K
artifacts/sec (≈11.2 ms). That measurement was ~23% faster than the
pre-RUSTNX-008 baseline (≈14.6 ms).

Corpus diversity exists specifically so the measurement reflects real match work
(PCRE post-filter path, `flags:"i"` inline prefix, multi-kind dispatch) rather
than cache-hit throughput on a few repeated strings.

Regressions of more than 2x on the same machine class should be treated as a
scan-path regression and investigated before release. Note: GitHub-hosted
runners have 2 cores and produce a materially lower absolute throughput — the 2x
guard must be evaluated against a baseline collected on the same machine class,
not the dev-box number above.

### stress baselines (kernel hot path)

Collected on the same dev machine (2026-04-28, `dev` branch):

- `graph_memory/small_graph` (100/500/1000 nodes, 3 edges/node) — 281 µs (within
  noise; no real change)
- `incremental_throughput/sustained_edits` (500 nodes, 0.1 batch fraction, 100
  ms sustain) — 100 ms (flat; matches sustain budget)
- `policy_scaling/rule_scaling` (200 symbols, 10/50/100 rules) — 115 µs (no
  change)
- `cold_start/scaling` (multi-step file-count scaling) — 3.54 ms (+3% vs prior;
  expected — SCAN-001 ignore::WalkBuilder overhead)

Previous baseline (2026-04-25, after LAUNCH-001 watch hot-path rewrite): 273 µs
/ 100 ms / 114 µs / 3.45 ms.

These are upper bounds for the README hero stats
(`10 µs save-time incremental file update`, `800 ns full policy evaluation`,
`14.5 ms cold graph build, 100-file codebase`). Per-step times within each
scenario are faster than the aggregate Criterion timing — the harness reports
the full multi-step `run()` so the published hero numbers stay conservative.

### secret_scan_parallel baseline

Collected on the same dev machine (2026-04-28, `dev` branch):

- `scan/serial_baseline` — ~3.5 s (~862 elem/s)
- `scan/parallel_rollout` — ~442 ms (~6.8 K elem/s)
- Parallel speedup: ~7.7x vs serial baseline.

## Usage

```bash
# Run Criterion benchmarks
cargo bench -p anvil-bench

# Use as a library in custom benchmarks
[dev-dependencies]
anvil-bench = { path = "../anvil-bench" }
```

## Development

```bash
cargo test -p anvil-bench
cargo bench -p anvil-bench
```
