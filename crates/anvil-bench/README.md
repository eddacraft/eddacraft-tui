# anvil-bench

Stress test and benchmark harness for `anvil-kernel`. Generates synthetic
repositories of configurable size and language mix, measures timing and memory,
and produces structured JSON reports.

## Status

Active

## Modules

| Module | Description |
| --- | --- |
| `fixture` | Synthetic repo generation with configurable file counts and language weights |
| `measure` | Timing and memory measurement (RSS/VM via `/proc/self/status`) |
| `report` | Structured `ScenarioResult` output (JSON, human-readable) |
| `scenarios` | Stress test scenarios |

## Scenarios

- **watcher_saturation** -- File watcher throughput under high churn
- **graph_memory** -- Symbol graph memory scaling
- **incremental_throughput** -- Incremental re-analysis performance
- **policy_scaling** -- Policy evaluation scaling with rule count
- **cold_start_scaling** -- Cold start time vs repository size

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
