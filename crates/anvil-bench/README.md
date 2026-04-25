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

Run a single bench:

```bash
cargo bench -p anvil-bench --bench antipattern_scan
```

### antipattern_scan baseline

Collected on an Ubuntu 25.04 / Linux 6.17 / rayon default thread pool (recorded
2026-04-25, local dev machine, `release/v0.4.0-beta` post-RUSTNX-008
workspace-hack and SPG-001..006):

- Corpus: 320 synthetic artifacts, mixed kinds:
  - 60% source (`.ts`) cycling 6 content variants — exercises AP-\*,
    DD-001/002/003, GS-001.
  - 20% pr-description cycling 6 variants — exercises DD-004, RL-001/002/
    003/004/005 (and their `flags:"i"` handling + post-filters).
  - 10% commit-message cycling 3 variants — exercises RL-002/005.
  - 10% agent-output cycling 3 variants — exercises DD-004 and RL-\*.
- Throughput: ~28.6 K artifacts/sec (≈11.2 ms per full corpus pass).
- Confirms no scan-path regression from the workspace-hack `serde_json` feature
  unification (`preserve_order`) landed in `RUSTNX-008` — measurement is ~23%
  faster than the pre-RUSTNX-008 baseline (≈14.6 ms).

Corpus diversity exists specifically so the measurement reflects real match work
(PCRE post-filter path, `flags:"i"` inline prefix, multi-kind dispatch) rather
than cache-hit throughput on a few repeated strings.

Regressions of more than 2x on the same machine class should be treated as a
scan-path regression and investigated before release. Note: GitHub-hosted
runners have 2 cores and produce a materially lower absolute throughput — the 2x
guard must be evaluated against a baseline collected on the same machine class,
not the dev-box number above.

### stress baselines (kernel hot path)

Collected on the same dev machine, same date, after the LAUNCH-001 watch
hot-path rewrite (`next_id` → monotonic counter, `was_tracked` → HashSet):

- `graph_memory/small_graph` (100/500/1000 nodes, 3 edges/node) — 273 µs
- `incremental_throughput/sustained_edits` (500 nodes, 0.1 batch fraction, 100
  ms sustain) — 100 ms (matches sustain budget; demonstrates the watch loop
  sustained the configured edit cadence without falling behind)
- `policy_scaling/rule_scaling` (200 symbols, 10/50/100 rules) — 114 µs
- `cold_start/scaling` (multi-step file-count scaling) — 3.45 ms

These are upper bounds for the README hero stats
(`10 µs save-time incremental file update`, `800 ns full policy evaluation`,
`14.5 ms cold graph build, 100-file codebase`). Per-step times within each
scenario are faster than the aggregate Criterion timing — the harness reports
the full multi-step `run()` so the published hero numbers stay conservative.

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
