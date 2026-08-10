# Benchmark Results

| Type  | Authority | Owner                                                                                                                                   | Status | Freshness                                                       |
| ----- | --------- | --------------------------------------------------------------------------------------------------------------------------------------- | ------ | --------------------------------------------------------------- |
| Guide | Advisory  | RLB ([`plans/modules/resource-load-benchmarking.aps.md`](../../plans/modules/resource-load-benchmarking.aps.md)); KFIT kindling section | Live   | Full-suite history 2026-08-10; Kindling history seed 2026-08-03 |

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

### Scheduled runs versus committed history

- [`bench.yml`](../../.github/workflows/bench.yml) runs the hosted Criterion and
  mid-edit suites daily at 19:00 UTC (03:00 AWST), plus manual dispatch.
- [`resource-budget.yml`](../../.github/workflows/resource-budget.yml) runs the
  per-process budgets daily at 17:15 UTC (01:15 AWST), plus integration,
  release-readiness, and manual triggers.
- [`bench-nightly.yml`](../../.github/workflows/bench-nightly.yml) is manual
  only while no self-hosted `bench` runner is online.

Those workflows upload expiring artefacts; they do not commit this ledger. A
reviewed quiet-box run must be promoted through the
[`benchmarks/history/` workflow](../../benchmarks/README.md#adding-a-run) to
become durable history.

Reference machine for the dev-box entries below: **AMD Ryzen 7 5800X (8c/16t),
Linux 6.17** — the intercept interactive pool is 4 threads (half-cores capped at
4).

---

## Full-suite run — 2026-08-10, dev box

Source: anvil `adabdb782`, Rust 1.97.1, AMD Ryzen 7 5800X (16 logical CPUs),
Linux 6.17. Machine-readable record:
[`benchmarks/history/2026-08-10.json`](../../benchmarks/history/2026-08-10.json).
The stable `pnpm bench` entrypoint hit the known non-TTY `cargo | tee` capture
failure before measurement; every declared command then ran directly and
sequentially using the documented fallback. Compile checks and 81 `anvil-bench`
tests passed.

All hard latency and resource budgets passed. The mid-edit comparator reported
**0 SLO breaches and 11 soft drift warnings across 14 rows**. The largest
warnings were the empty and 1 KiB service paths (+91.0% and +88.8% against the
committed developer-machine baseline); absolute p95 remained 2.112 ms and 2.184
ms against the 50 ms service SLO. Treat this as drift to watch, not an
unqualified no-change result.

### Latency gates

| Surface / worst budgeted case              | Observed p95 | Budget | Verdict                  |
| ------------------------------------------ | ------------ | ------ | ------------------------ |
| `validate_paths`, 4 agents + executor scan | 0.091 ms     | 80 ms  | pass                     |
| Mid-edit service, near-cap buffer          | 28.899 ms    | 50 ms  | pass, soft drift warning |
| Mid-edit roundtrip, near-cap buffer        | 37.751 ms    | 80 ms  | pass                     |
| Hot read, slowest certification case       | 0.035 ms     | 80 ms  | pass                     |
| Call lift, cap-ceiling file                | 5.230 ms     | 80 ms  | pass                     |

### CPU / RSS budgets

| Process / phase                | CPU (measured / budget) | RSS (measured / budget) | Status |
| ------------------------------ | ----------------------- | ----------------------- | ------ |
| `anvil watch` — churn path     | 0.95% / 50%             | 23.64 / 300 MiB         | pass   |
| intercept daemon — idle        | 0.0% / 5%               | 16.52 / 96 MiB          | pass   |
| intercept daemon — burst       | 100.03% / 250%          | 16.52 / 128 MiB         | pass   |
| MCP server                     | 5.57% / 200%            | 16.14 / 96 MiB          | pass   |
| concurrent all-three aggregate | 99.06% / 800%           | 54.43 / 700 MiB         | pass   |

### Same-host throughput signals

- Secret scanning: 6.201 s serial versus 0.842 s parallel (**7.37× speed-up**).
- Discovery walk: 85.78 ms sequential versus 13.87 ms parallel (**6.19×
  speed-up**).
- Call-lift p95 improved by 19–33% across the three common cases versus
  2026-06-26.
- The parallel mixed antipattern corpus was 0.637 ms, **39.2% slower** than
  2026-06-26. The TypeScript small/medium/large and clean-file cases were 30–52%
  slower. Post-run catalogue analysis found that the default regex rules the
  scanner executes had grown from 23 to 33, while default TypeScript-source
  rules grew from 11 to 19. Because the workload changed, these timings are a
  new workload baseline rather than evidence of a scanner-only regression.

This is the final `parallel_mixed_corpus` v1 result. Later runs use
`parallel_balanced_corpus_v2_fnv1a_<content-digest>`, with the source share
split evenly across TypeScript, Rust, and Python. The case identity changes when
fixture content changes; history also records default and enabled-regex
antipattern-catalogue fingerprints. Compare timings only when the complete case
name and applicable fingerprint match.

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
the `resource-budget-verdicts` CI artifact. CPU is scaled so 100% is one
fully-saturated core; RSS is peak resident-set MiB.

### 2026-06-04, dev box (release binary) — every budget `status: pass`

| Process / phase                      | CPU (measured / budget) | RSS (measured / budget) | Status |
| ------------------------------------ | ----------------------- | ----------------------- | ------ |
| `anvil watch` — churn path (RLB-002) | 7.0% / 50%              | 16.1 / 300 MiB          | pass   |
| intercept daemon — idle (RLB-003)    | 0.0% / 5%               | 10.9 / 96 MiB           | pass   |
| intercept daemon — burst (RLB-003)   | 100.9% / 250%           | 10.9 / 128 MiB          | pass   |
| MCP server (RLB-004)                 | 83.5% / 200%            | 9.7 / 96 MiB            | pass   |
| concurrent all-three (RLB-005)       | 189.6% / 800%           | 35.5 / 700 MiB          | pass   |

Every process sits far inside its ceiling — RSS is single-digit-to-low-tens of
MiB against 96–700 MiB budgets, and the intercept daemon idles at **0% CPU /
10.9 MiB** (the "must not idle hot" guard from GH #2156 holds). Burst CPU caps
at ~one core (intercept) / ~83% (MCP); the three-process aggregate peaks at ~1.9
cores against the 8-core (800%) ceiling. The multi-agent watch load ramp
(RLB-001, `scripts/bench/load-ramp.sh`) is a separate dispatch-only / quiet-box
harness, not run here.

## Kindling (bundled runtime — consumer evidence, KFIT)

Kindling performance that gates **anvil** adoption is filed **in this repo**,
not on the public kindling site. Machine-readable history:

- [`benchmarks/history/kindling/`](../../benchmarks/history/kindling/) — one
  JSON run per date (`schema_version: 1`, `suite: "kindling"`)
- [`benchmarks/history/kindling/raw/`](../../benchmarks/history/kindling/raw/) —
  compact profile + Criterion summary extracts
- Promote with `python3 scripts/bench/kindling-to-history.py` (see
  [`benchmarks/README.md`](../../benchmarks/README.md))
- Narrative assessment:
  [`docs/reviews/kindling-performance-and-integration-assessment.md`](../reviews/kindling-performance-and-integration-assessment.md)

Scratch Criterion HTML and full dumps stay under gitignored
`benchmark-results/manual-*-kindling/`.

Proposed budgets (KFIT-005 contract from the 2026-08-03 assessment): cold start
p95 ≤ 50 ms; direct append p95 ≤ 500 µs; warm daemon append p95 ≤ 1 ms;
concurrent daemon append p95 ≤ 5 ms; daemon page p95 ≤ 10 ms; ranked retrieve
p95 ≤ 50 ms at standard (~25k) scale; outage append p95 ≤ 5 ms; replay ≥ 2k
rows/s. Full scans are report-only. Prefer **`--isolated-process`**
RSS/thread/FD deltas for release resource series; shared-process runs stay
directional only.

### Run — 2026-08-03, 16-logical-CPU Linux reference box

Source: kindling `c15089df2` (+ uncommitted KINTEG-013/014 worktree), filed via
history
[`benchmarks/history/kindling/2026-08-03.json`](../../benchmarks/history/kindling/2026-08-03.json).
`partial: true` — not a published runtime floor.

#### Latency / throughput (standard + stress)

| Workload                         | Std p50  | Std p95  | Stress p50 | Stress p95 | Budget (p95)    | Verdict                     |
| -------------------------------- | -------- | -------- | ---------- | ---------- | --------------- | --------------------------- |
| cold-start / runtime-start       | 11.24 ms | 11.34 ms | 11.23 ms   | 11.33 ms   | ≤ 50 ms         | pass                        |
| direct-service / append          | 60.6 µs  | 111.8 µs | 63.0 µs    | 123.2 µs   | ≤ 500 µs        | pass                        |
| daemon / spooled-append-warm     | 163 µs   | 230 µs   | 168 µs     | 237 µs     | ≤ 1 ms          | pass                        |
| daemon / append-concurrent       | 817 µs   | 1.39 ms  | 1.69 ms    | 3.47 ms    | ≤ 5 ms          | pass                        |
| daemon / list-page               | 1.21 ms  | 1.28 ms  | 2.05 ms    | 2.49 ms    | ≤ 10 ms         | pass                        |
| direct-service / list-page       | 427 µs   | 590 µs   | 793 µs     | 1.12 ms    | report          | report                      |
| direct-service / list-full-scan  | 42.9 ms  | 43.3 ms  | 1.42 s     | 1.45 s     | report (export) | report                      |
| daemon / list-full-scan          | 94.9 ms  | 95.8 ms  | 2.14 s     | 2.21 s     | report (export) | report                      |
| direct-service / ranked-retrieve | 14.3 ms  | 15.2 ms  | 152.5 ms   | 156.5 ms   | ≤ 50 ms (std)   | pass / report               |
| daemon / ranked-retrieve         | 17.3 ms  | 19.2 ms  | 178.3 ms   | 185.7 ms   | ≤ 50 ms (std)   | pass / report               |
| outage / spool-append            | 5.19 µs  | 6.49 µs  | 5.16 µs    | 6.58 µs    | ≤ 5 ms          | pass                        |
| outage / spool-append-early      | 5.23 µs  | 6.59 µs  | 5.19 µs    | 6.67 µs    | ≤ 5 ms          | pass                        |
| outage / spool-append-late       | 5.15 µs  | 6.49 µs  | 5.15 µs    | 6.45 µs    | ≤ 5 ms          | pass                        |
| outage / spool-replay            | 168 ms   | 168 ms   | 18.3 s     | 18.3 s     | ≥ 2k rows/s     | pass (5.96k / 5.45k rows/s) |

#### Resources (directional, shared process)

| Profile  | Group           | Peak RSS (MiB) | Spool (bytes) | Storage (bytes) |
| -------- | --------------- | -------------- | ------------- | --------------- |
| standard | cold-start      | 6.4            | 0             | 0               |
| standard | direct-service  | 25.3           | 0             | ~17.4 M         |
| standard | daemon          | 46.4           | 0             | ~19.7 M         |
| standard | outage-recovery | 46.4           | ~285 K        | ~4.9 M          |
| stress   | cold-start      | 6.5            | 0             | 0               |
| stress   | direct-service  | 88.2           | 0             | ~137 M          |
| stress   | daemon          | 193.3          | 0             | ~155 M          |
| stress   | outage-recovery | 258.5          | ~28.7 M       | ~65.8 M         |

### Run — 2026-08-04, 16-logical-CPU Linux reference box (post-merge)

Source: kindling `f6dcd7d` (merged KINTEG-013/014 + isolated bench, PR
[#143](https://github.com/eddacraft/kindling/pull/143)),
`kindling-bench --isolated-process` standard + stress. History:
[`benchmarks/history/kindling/2026-08-04.json`](../../benchmarks/history/kindling/2026-08-04.json).
**Not partial** for source state (published `main`); still a single-host series.

#### Latency / throughput (standard + stress, isolated-process)

| Workload                         | Std p50  | Std p95  | Stress p50 | Stress p95 | Budget (p95)    | Verdict                     |
| -------------------------------- | -------- | -------- | ---------- | ---------- | --------------- | --------------------------- |
| cold-start / runtime-start       | 11.30 ms | 11.41 ms | 11.33 ms   | 11.44 ms   | ≤ 50 ms         | pass                        |
| direct-service / append          | 79.6 µs  | 142.5 µs | 99.1 µs    | 181.8 µs   | ≤ 500 µs        | pass                        |
| daemon / spooled-append-warm     | 221 µs   | 320 µs   | 212 µs     | 317 µs     | ≤ 1 ms          | pass                        |
| daemon / append-concurrent       | 1.15 ms  | 1.62 ms  | 2.37 ms    | 4.11 ms    | ≤ 5 ms          | pass                        |
| daemon / list-page               | 1.90 ms  | 2.35 ms  | 2.54 ms    | 3.58 ms    | ≤ 10 ms         | pass                        |
| direct-service / list-page       | 454 µs   | 747 µs   | 0.83 ms    | 1.40 ms    | report          | report                      |
| direct-service / list-full-scan  | 55.0 ms  | 61.4 ms  | 1.92 s     | 2.04 s     | report (export) | report                      |
| daemon / list-full-scan          | 141 ms   | 185 ms   | 2.68 s     | 2.92 s     | report (export) | report                      |
| direct-service / ranked-retrieve | 17.5 ms  | 19.6 ms  | 167 ms     | 181 ms     | ≤ 50 ms (std)   | pass / report               |
| daemon / ranked-retrieve         | 22.0 ms  | 23.3 ms  | 188 ms     | 197 ms     | ≤ 50 ms (std)   | pass / report               |
| outage / spool-append            | 4.7 µs   | 6.8 µs   | 6.7 µs     | 9.8 µs     | ≤ 5 ms          | pass                        |
| outage / spool-append-early      | 4.7 µs   | 6.8 µs   | 4.8 µs     | 6.7 µs     | ≤ 5 ms          | pass                        |
| outage / spool-append-late       | 4.7 µs   | 6.1 µs   | 6.7 µs     | 9.4 µs     | ≤ 5 ms          | pass                        |
| outage / spool-replay            | 231 ms   | 231 ms   | 25.3 s     | 25.3 s     | ≥ 2k rows/s     | pass (4.33k / 3.95k rows/s) |

#### Resources (isolated-child peak RSS)

| Profile  | Group           | Peak RSS (MiB) | Scope          |
| -------- | --------------- | -------------- | -------------- |
| standard | cold-start      | 6.5            | isolated-child |
| standard | direct-service  | 23.9           | isolated-child |
| standard | daemon          | 33.8           | isolated-child |
| standard | outage-recovery | 11.7           | isolated-child |
| stress   | cold-start      | 6.6            | isolated-child |
| stress   | direct-service  | 87.1           | isolated-child |
| stress   | daemon          | 116.1          | isolated-child |
| stress   | outage-recovery | 173.8          | isolated-child |

Physical `writeBytes` often stayed 0; use `logicalWriteBytes` in the history
JSON for syscall-level I/O. Concurrent daemon append stress p95 **4.11 ms**
remains the tightest write budget (5 ms).

### Kindling history index

| Date       | Kindling commit | Host class           | Notes                                  | File                                                                   |
| ---------- | --------------- | -------------------- | -------------------------------------- | ---------------------------------------------------------------------- |
| 2026-08-04 | `f6dcd7d`       | 16-logical-CPU Linux | post-merge, `--isolated-process`       | [`2026-08-04.json`](../../benchmarks/history/kindling/2026-08-04.json) |
| 2026-08-03 | `c15089df2`\*   | 16-logical-CPU Linux | pre-merge worktree; shared-process RSS | [`2026-08-03.json`](../../benchmarks/history/kindling/2026-08-03.json) |

\*Uncommitted KINTEG-013/014 worktree on that base — see run `caveats`.

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
- Kindling (consumer): local kindling harness →
  `benchmark-results/manual-*-kindling/` →
  `python3 scripts/bench/kindling-to-history.py` →
  `benchmarks/history/kindling/`.
