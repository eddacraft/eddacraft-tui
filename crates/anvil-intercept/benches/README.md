# anvil-intercept benchmarks

Latency and resource benchmarks for the intercept daemon
(`anvil intercept start`). All three set `harness = false`, so cargo runs each
bench's own `main` (the gate logic + JSON/line output below);
`midedit_roundtrip` additionally carries a Criterion harness for local
profiling. `ipc_roundtrip` and `midedit_roundtrip` require the `bench-internals`
feature.

| Bench                       | Measures                                                                                 | Feature           |
| --------------------------- | ---------------------------------------------------------------------------------------- | ----------------- |
| `ipc_roundtrip`             | IPC + save-time `validate_paths` latency (`validation.service` / `validation.roundtrip`) | `bench-internals` |
| `midedit_roundtrip`         | `scan_buffer` mid-edit secret-detection roundtrip (RTAI-001)                             | `bench-internals` |
| `intercept_resource_budget` | Idle + burst CPU/RSS budget of the real daemon binary (RLB-003)                          | —                 |

Run one:

```bash
cargo bench -p eddacraft-anvil-intercept --bench ipc_roundtrip --features bench-internals
```

> Latency benches want a **quiet box**: a backgrounded agent shell or a loaded
> machine inflates the absolute numbers (the _shape_ still holds). Record only
> warm-daemon runs; cold-start is out of scope (ADR-031).

## `ipc_roundtrip` — the `validate_paths` SLO gate (DSV-006 / Task 16)

This bench is the ADR-061 §9 concurrency SLO gate. It exits non-zero when
interactive `validate_paths` p95 breaches the **ADR-031 interactive save-time
budget of 80 ms** (`validation.service`), so the CI step (`resource-budgets` job
of
[`.github/workflows/resource-budget.yml`](../../../.github/workflows/resource-budget.yml))
is the gate. Cases:

- **warm** — single-client `validate_paths` p95 (the headline SLO).
- **4 agents + 1 background scan** — the gated concurrency point: four agents,
  each on its own `WorktreeKey`, drive `validate_paths` while a background scan
  competes for cores. Measures interactive-pool contention.
- **scoped-fallback (RLB-002)** — the daemon-absent path `watch` runs (a scoped
  antipattern check over the changed bytes, never `--all`). Report-only.
- **agent sweep** — opt-in, report-only (see below).

### Result set — gate (dev box, 2026-06-03)

Ryzen 7 5800X (8c/16t, interactive pool = 4 threads), Linux 6.17, 200 samples.
Numbers vary run-to-run on a loaded box; the budget headroom is the point.

| Case                               | p50      | p95      | p99      | Budget | Verdict |
| ---------------------------------- | -------- | -------- | -------- | ------ | ------- |
| `validate_paths` warm              | 0.023 ms | 0.031 ms | 0.034 ms | 80 ms  | PASS    |
| `validate_paths` 4 agents + 1 scan | 0.031 ms | 0.047 ms | 0.071 ms | 80 ms  | PASS    |
| scoped-fallback (daemon-absent)    | 0.008 ms | 0.011 ms | 0.015 ms | report | —       |

Interactive p95 lands ~2000× inside the 80 ms budget even under the ramp.

### Agent-count sweep (opt-in)

The gate measures one fixed concurrency point. To chart the saturation curve,
set `ANVIL_BENCH_VALIDATE_AGENTS` to a comma-separated list of positive integers
(any counts). The sweep is **report-only** — it never changes the exit code, so
the default CI run stays the single gate point (mirrors how `load-ramp.sh`
separates `--smoke` from the full ramp). A malformed value (e.g. the wrong
separator `1;2;4`) prints a WARN and skips.

```bash
ANVIL_BENCH_VALIDATE_AGENTS=1,2,4,8,16,32,64 \
  cargo bench -p eddacraft-anvil-intercept --bench ipc_roundtrip --features bench-internals
```

### Result set — sweep (same box / run, 2026-06-03)

Each level is `N` agents + 1 background scan, 50 `validate_paths` calls per
agent.

| Agents | Calls | p50      | p95      | p99       |
| ------ | ----- | -------- | -------- | --------- |
| 1      | 50    | 0.032 ms | 0.038 ms | 0.045 ms  |
| 2      | 100   | 0.028 ms | 0.053 ms | 0.060 ms  |
| 4      | 200   | 0.026 ms | 0.040 ms | 0.050 ms  |
| 8      | 400   | 0.033 ms | 0.047 ms | 0.061 ms  |
| 16     | 800   | 0.053 ms | 0.120 ms | 0.188 ms  |
| 32     | 1600  | 0.082 ms | 0.282 ms | 0.410 ms  |
| 64     | 3200  | 0.124 ms | 0.690 ms | 12.407 ms |

Reading the curve: **flat through 8 agents**, **knee at ~16** (p95 triples as
agents start oversubscribing the 4-thread interactive pool), then roughly linear
degradation 16 → 32 → 64. At 64 agents (16× the gated scenario) interactive p95
is still ~115× inside budget; the p99 tail (12.4 ms) is the first sign of real
queueing stress and the number to watch if real-world concurrency ever climbs
that high.

### Synthetic-regression self-test

The CI gate is only credible if a regression fails it.
`ANVIL_BENCH_VALIDATE_STALL_MS` injects a per-parse stall to force a breach; the
workflow runs this and asserts a non-zero exit:

```bash
ANVIL_BENCH_VALIDATE_STALL_MS=300 \
  cargo bench -p eddacraft-anvil-intercept --bench ipc_roundtrip --features bench-internals
# → FAIL: validate_paths (warm) p95 ~200ms exceeds budget 80.000ms; exit 1
```

## Where the numbers are tracked

The consolidated, cross-benchmark ledger lives at
[`docs/testing/benchmark-results.md`](../../../docs/testing/benchmark-results.md).
Append new runs there; this README carries only the latest validate_paths
baseline.
