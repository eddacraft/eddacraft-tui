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
- **token_reduction** -- Graph-context delivery vs naive file-reading context
  cost for change-impact questions (GCTX-031)

## Benchmarks

- **single-command suite** -- `pnpm bench` from the repository root runs every
  routine benchmark surface consistently and writes logs under
  `benchmark-results/manual-<timestamp>/`. Add `-- --include-nightly-stress` to
  include the full stress runner used by the nightly workflow.
- **stress** -- Kernel-level stress scenarios (graph, policy, cold start,
  incremental throughput).
- **antipattern_scan** -- Parallel anti-pattern scan throughput on a
  320-artifact synthetic corpus spanning every `ArtifactKind`. Exercises the
  rayon fan-out plus every enabled registry rule (including the SPG-003
  post-filter rules and the `flags:"i"` inline prefix). This is the CI guard for
  the parallel-scan claim in ADR-026.
- **secret_scan_parallel** -- Serial vs parallel throughput on the secret scan
  path. Validates the parallel rollout speedup claim.
- **walk_discovery** -- Sequential `ignore::WalkBuilder` vs `WalkParallel` for
  the discovery _walk_ phase (traversal + per-entry `metadata()` stat). The
  SCAN-005 spike measuring whether parallelising the walk itself clears the ≥20%
  bar. Corpus size via `ANVIL_BENCH_WALK_FILES` (default 20,000).
- **validate_write_tier** -- `anvil_validate_write` full pipeline vs CIB-006
  risk-tier safelist wall time on a representative JSON metadata fixture (single
  string-value rename via `proposedContent` vs `patch`). Informational: reports
  per-tier mean latency and the safelist speedup.
- **watch_resource_budget** -- Release-binary `anvil watch` CPU/RSS budget
  check. Included in `pnpm bench`; skip with `-- --skip-resource-budget` when
  you only need Criterion micro-benchmarks.
- **anvil-bench-command** -- Manual finite-command runner for one `anvil` CLI
  command at a time. It executes only the resolved Anvil binary, defaults to a
  temporary `ANVIL_HOME` and fixture, disables usage/observation writes, records
  redacted argv shapes by default, and emits per-iteration JSON. This is not yet
  part of the curated `pnpm bench` routine/history suite; wire a small stable
  set only after the report shape has a comparable baseline.

Run a single Criterion bench:

```bash
cargo bench -p anvil-bench --bench antipattern_scan
```

Benchmark one finite Anvil CLI command end-to-end:

```bash
cargo build -p eddacraft-anvil --release --bin anvil
ANVIL_BENCH_ANVIL_BIN=target/release/anvil \
  cargo run -p anvil-bench --bin anvil-bench-command -- \
  --name status-verify --repeat 30 --warmup 5 --fixture empty -- \
  status --verify
```

If the worktree relocates Cargo output with `CARGO_TARGET_DIR`, the runner
resolves `target/release/anvil` through that target directory. Use
`--include-raw-argv` only for reviewed local diagnostics; reports otherwise
store redacted argument shapes so secrets are not persisted.

Run the full routine suite:

```bash
pnpm bench
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

### walk_discovery baseline

Collected 2026-06-02 on a 16-core dev box (Linux 6.17), default corpus
(`ANVIL_BENCH_WALK_FILES=20000`, 20,000 candidates), both strategies collecting
the identical candidate set:

- `walk_discovery/sequential_walkbuilder` — 101.11 ms median (~197.8 Kelem/s)
- `walk_discovery/parallel_walkparallel` — 17.67 ms median (~1.13 Melem/s)
- Parallel speedup: ~5.7x (~82.5% wall-time reduction).

SCAN-005 asked whether parallelising the _walk itself_ — directory traversal
plus the per-entry `metadata()` stat — buys ≥20% over the sequential
`ignore::WalkBuilder` already used for discovery. On this box it buys ~470%, so
the SCAN-006 follow-up to wire `WalkParallel` into the discovery path has real
headroom. The parallel timing deliberately includes the `mpsc` send/recv cost a
production refactor would pay (the source of the occasional high outlier). The
win scales with core count: expect it to compress toward the core-count ceiling
on 2–4 core machines, so evaluate regressions against a same-class baseline.

### watch load-ramp baseline (RLB-001 / RLB-007)

Not a Criterion micro-bench — `scripts/bench/load-ramp.sh` ramps concurrent
`anvil watch` agents against a synthetic repo with real file churn and reports
whole-process-tree CPU/RSS per agent level (the saturation tipping point the
idle-path `watch_resource_budget` bench cannot see). It drives whatever binary
it resolves and does **not** rebuild, so to measure current watch code build a
fresh release binary and pass `ANVIL_BIN` — the auto-picked
`target/release/anvil` is often stale.

Collected 2026-06-02 on the same 16-core box, default ramp (1,500 files, churn
200 ms, `--action check`), driving a freshly built release binary that includes
RLB-007 (`perf(watch): scope per-save check to changed paths`):

| agents | action | machine% | cores | RSS(parent)   |
| ------ | ------ | -------- | ----- | ------------- |
| 1      | check  | 0.0%     | 0.01  | 11.2 MiB      |
| 2      | check  | 0.1%     | 0.01  | 22.3 MiB      |
| 4      | check  | 0.4%     | 0.07  | 44.9 MiB      |
| 4      | none   | 0.0%     | 0.00  | 0.0 (control) |

The pre-RLB-007 `--all` rescan pattern (documented in the
`benchmarks/prototypes/anvil-load-probe.py` header on a comparable box) cost
~43.5% machine / 6.96 cores at 1 agent and ~88.1% / 14.09 cores at 4 agents —
the beta-tester high-CPU report. Scoping the per-save check to changed paths
drops 4-agent cost ~200x. The `none` row is the control: the bare watch loop is
effectively free, and the `check` rows scale above it (0.0 → 0.1 → 0.4), which
confirms the per-save check is firing rather than being skipped. For a
defensible before/after, build `af1524f5c^` as release and A/B in the same
session.

### token_reduction baseline (GCTX-031)

Makes the graph-context token-reduction claim reproducible. For each fixed
fixture the harness asks one change-impact question — _"what is impacted by
changing the most depended-on symbol?"_ — and counts the tokens three delivery
strategies cost, using the GCTX-020 estimator (`estimate_gctx_tokens`,
`gctx-simple-v1`) so the figures are measured with GCTX's own planning budget:

- **whole-repo** — a tool-less assistant reads every file in full. This is the
  pathological ceiling, not a realistic baseline (no assistant reads a whole
  repo for a one-symbol question); the **neighbourhood** column is the
  meaningful one.
- **neighbourhood** — a graph-less but savvy reader opens, in full, every file
  in the impacted set (the changed file plus the files holding its
  reverse-dependency closure).
- **graph context** — the **real** production
  `ImpactOutcome::Ready(ImpactReport)` (from `anvil-gctx-types`) serialised
  compact: the exact wire shape `anvil_impact_of_change` emits — `snake_case`,
  status-tagged, change-surface symbol identities + file-keyed `dependent_files`
  with hop distance, no source text. The scenario asserts the payload
  deserialises back into a real `ImpactOutcome`, so it cannot drift from
  production. The change-invariant response envelope (`workspace_assurance` +
  `workspace_root`) is excluded.

The impacted set is the **2-hop reverse-dependency closure** of the changed
file, matching the production cap (`MAX_REVERSE_IMPACT_DEPTH = 2` in
`anvil-graph-cache`). The neighbourhood baseline and the graph payload cover
that same set, so the comparison is apples-to-apples — the reduction reflects
only delivering identities instead of whole files. (Where the closure spans the
whole project — e.g. `small_lib` — the neighbourhood equals the whole-repo
column, honestly.) Each fixture's symbol graph is the single source of truth:
both the rendered source files and the payload derive from the same nodes and
edges. The token counts are deterministic (no RNG); the `generated_at_epoch`,
`duration_secs`, and RSS metrics in the JSON vary per run. Rerun with:

```bash
cargo run -p anvil-bench --release -- token_reduction
# results land in bench-results/stress-*.json (gitignored); rm to keep clean
```

`token_reduction` is part of the stress runner, so `pnpm bench` only includes it
with `-- --include-nightly-stress`, and the nightly CI run (`bench-nightly.yml`)
only exercises it when the self-hosted bench runner is online. The documented
local command above is the supported reproduction path; the golden test
(`bench.yml` runs `cargo test -p anvil-bench` on push) guards the published
numbers regardless.

The numbers below are asserted as golden values by
`default_fixture_token_counts_are_stable` in the scenario tests, so a code
change that moves them fails `cargo test -p anvil-bench` until this table is
updated in the same commit. Recorded 2026-06-26 (`gctx-simple-v1` estimator):

`affected` is the change-surface symbol count (`ImpactReport.affected_symbols`);
`deps` is the reverse-impact closure size (`dependent_files`).

| fixture       | files | affected | deps | whole-repo (tok) | neighbourhood (tok) | graph (tok) | ↓ vs whole-repo | ↓ vs neighbourhood |
| ------------- | ----- | -------- | ---- | ---------------- | ------------------- | ----------- | --------------- | ------------------ |
| `small_lib`   | 8     | 4        | 7    | 2,163            | 2,163               | 410         | 81.0%           | 81.0%              |
| `layered_app` | 20    | 5        | 10   | 8,614            | 4,738               | 520         | 94.0%           | 89.0%              |
| `wide_fanout` | 12    | 6        | 10   | 7,548            | 6,919               | 570         | 92.5%           | 91.8%              |
| **mean**      |       |          |      |                  |                     |             | **89.2%**       | **87.3%**          |

These figures bound the reduction for **identity-style change-impact queries**
on synthetic fixtures — the regime GCTX's identity surface targets — not a
universal claim about every assistant task. Two honesty caveats:

- **Estimator bias.** `gctx-simple-v1` takes `max(lexical_units, bytes/4)`.
  Source code is punctuation-dense and hits the `lexical_units` branch
  (~1.4–1.7× `bytes/4`); the sparse identity payload hits the `bytes/4` branch.
  Used symmetrically this over-counts the source baselines relative to the
  payload, so a real BPE tokenizer (cl100k/o200k) would likely show ratios a few
  points lower.
- **Identity-only.** The payload carries no source text, matching
  `impact_of_change`. Snippet-bearing modes (`anvil_symbol_context`,
  GCTX-021..023) trade a higher graph-token cost for richer context and would
  narrow these ratios.

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
