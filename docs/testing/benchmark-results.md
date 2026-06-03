# Benchmark Results

| Type  | Authority | Owner                                                                                                            | Status | Freshness                                                                 |
| ----- | --------- | ---------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------- |
| Guide | Advisory  | RLB ([`plans/modules/resource-load-benchmarking.aps.md`](../../plans/modules/resource-load-benchmarking.aps.md)) | Live   | Created 2026-06-03 from `crates/anvil-intercept/benches/ipc_roundtrip.rs` |

| Upstream                                                                                                             | Downstream                                                                                                    |
| -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-bench`, `crates/anvil-intercept/benches`, `crates/anvil-checks/benches`, `crates/anvil-kernel/benches` | [`docs/policies/resource-budget.md`](../policies/resource-budget.md), `.github/workflows/resource-budget.yml` |

A consolidated, append-only ledger of benchmark runs across every Anvil
benchmark surface. This page is the **history**; the authoritative pass/fail
ceilings live in code (`ResourceBudget` in `crates/anvil-bench/src/budget.rs`,
the per-bench SLO consts) and in
[`resource-budget.md`](../policies/resource-budget.md), and the **live** numbers
are the CI artifacts produced by the `.github/workflows/resource-budget.yml` and
`.github/workflows/bench.yml` runs.

## How to use this page

- **Append, don't overwrite.** Each entry records a dated run with its machine
  class. A regression of more than ~2× on the same machine class is a real
  regression — investigate before release.
- **Machine class matters.** Dev-box numbers and GitHub-hosted-runner numbers (2
  cores) are not comparable in absolute terms; compare like with like.
- **Latency benches want a quiet box** (ADR-031: warm daemon, quiet host). A
  loaded box or a backgrounded agent shell inflates absolute numbers; the curve
  _shape_ still holds.

Reference machine for the dev-box entries below: **AMD Ryzen 7 5800X (8c/16t),
Linux 6.17** — the intercept interactive pool is 4 threads (half-cores capped at
4).

---

## Latency — save-time `validate_paths` (intercept `ipc_roundtrip`)

The ADR-061 §9 concurrency SLO. Budget: interactive save-time
`validation.service` p95 **≤ 80 ms** (ADR-031). Gated in the `resource-budgets`
CI job; the bench exits non-zero on a breach. Run:
`cargo bench -p eddacraft-anvil-intercept --bench ipc_roundtrip --features bench-internals`.

### Gate — 2026-06-03, dev box, 200 samples

| Case                               | p50      | p95      | p99      | Budget | Verdict |
| ---------------------------------- | -------- | -------- | -------- | ------ | ------- |
| `validate_paths` warm              | 0.023 ms | 0.031 ms | 0.034 ms | 80 ms  | PASS    |
| `validate_paths` 4 agents + 1 scan | 0.031 ms | 0.047 ms | 0.071 ms | 80 ms  | PASS    |
| scoped-fallback (daemon-absent)    | 0.008 ms | 0.011 ms | 0.015 ms | report | —       |

### Agent-count sweep — 2026-06-03, dev box (opt-in, report-only)

`ANVIL_BENCH_VALIDATE_AGENTS=1,2,4,8,16,32,64`; each level is N agents + 1
background scan, 50 calls per agent. "Headroom" is p95 against the 80 ms budget.

| Agents | Calls | p50      | p95      | p99       | Headroom |
| ------ | ----- | -------- | -------- | --------- | -------- |
| 1      | 50    | 0.032 ms | 0.038 ms | 0.045 ms  | ~2100×   |
| 2      | 100   | 0.028 ms | 0.053 ms | 0.060 ms  | ~1500×   |
| 4      | 200   | 0.026 ms | 0.040 ms | 0.050 ms  | ~2000×   |
| 8      | 400   | 0.033 ms | 0.047 ms | 0.061 ms  | ~1700×   |
| 16     | 800   | 0.053 ms | 0.120 ms | 0.188 ms  | ~670×    |
| 32     | 1600  | 0.082 ms | 0.282 ms | 0.410 ms  | ~280×    |
| 64     | 3200  | 0.124 ms | 0.690 ms | 12.407 ms | ~115×    |

Flat through 8 agents; **knee at ~16** (4× the 4-thread interactive pool); then
roughly linear p95 degradation. The p99 tail at 64 (12.4 ms) is the first sign
of real queueing stress — the number to watch if production concurrency climbs.

## Latency — `scan_buffer` mid-edit roundtrip (intercept `midedit_roundtrip`, RTAI-001)

Budget: interactive buffer `validation.roundtrip` p95 **≤ 80 ms** (ADR-031).
Last recorded baseline: **~1.4 ms p95** over 1024 iterations (~60× under budget)
— see [`crates/anvil-bench/README.md`](../../crates/anvil-bench/README.md) for
the release-level summary; the `midedit_roundtrip` JSON baseline lives at
`crates/anvil-intercept/benches/baselines/midedit_roundtrip.json`.

## Throughput — kernel / checks (`anvil-bench`, `bench.yml`)

Authoritative detail + history is in
[`crates/anvil-bench/README.md`](../../crates/anvil-bench/README.md); latest
recorded dev-box baselines (2026-04-28 unless noted):

- **`antipattern_scan`** — ~39.9 K artifacts/sec (≈8.0 ms / 320-artifact corpus
  pass); +42% vs the 2026-04-25 baseline. CI guard for the ADR-026 parallel-scan
  claim.
- **`secret_scan_parallel`** — serial vs parallel throughput on the secret scan
  path (validates the parallel-rollout speedup claim).
- **`walk_discovery` (SCAN-005)** — sequential vs `WalkParallel` discovery walk;
  corpus via `ANVIL_BENCH_WALK_FILES` (default 20,000).
- **`stress` (kernel hot path)** — `graph_memory/small_graph` ~281 µs;
  `incremental_throughput/sustained_edits` ~100 ms (matches sustain budget);
  `policy_scaling/rule_scaling` ~115 µs; `cold_start/scaling` ~3.54 ms.
- **SCAN parallel scan** — 7.39× wall-time improvement on a synthetic 3,000-file
  surface vs the previous serial baseline (v0.5.0-beta headline).

## Resource budgets — CPU / RSS (`resource-budget.yml`, RLB-002/003/004/005)

Pass/fail ceilings are pinned in `crates/anvil-bench/src/budget.rs` and
documented with rationale in
[`resource-budget.md`](../policies/resource-budget.md); the live verdicts are
the `resource-budget-verdicts` CI artifact. Summary of the pinned ceilings (not
a measured run — see the artifact for measurements):

- **`anvil watch` idle** — steady-state CPU ≤ 5%, peak RSS ≤ 200 MiB.
- **intercept daemon (RLB-003)** — idle steady-state + burst ceilings (the
  daemon must not idle hot — GH #2156).
- **MCP server (RLB-004)** — idle + burst ceilings for
  `anvil mcp serve --stdio`.
- **concurrent all-three (RLB-005)** + the multi-agent watch load ramp (RLB-001,
  `scripts/bench/load-ramp.sh`) — dispatch-only / quiet-box tier.

## Run index

- Latency (intercept):
  `cargo bench -p eddacraft-anvil-intercept --bench ipc_roundtrip --features bench-internals`
  (+ `--bench midedit_roundtrip`). See
  [`crates/anvil-intercept/benches/README.md`](../../crates/anvil-intercept/benches/README.md).
- Throughput + stress: `pnpm bench` (writes
  `benchmark-results/manual-<timestamp>/`); single bench
  `cargo bench -p anvil-bench --bench <name>`.
- Resource budgets: the `Resource Budget` workflow (push/PR for per-process;
  `workflow_dispatch` for the concurrent + load-ramp tier).
