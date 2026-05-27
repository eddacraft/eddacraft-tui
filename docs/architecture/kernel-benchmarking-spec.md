# Kernel Benchmarking Specification

| Type | Authority | Owner | Status   | Freshness                                        |
| ---- | --------- | ----- | -------- | ------------------------------------------------ |
| Spec | Derived   | KERN  | Proposed | Metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream                                                                                  | Downstream                                            |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `docs/architecture/rust-kernel-spec.md`, `crates/anvil-kernel/benches/kernel.rs`, ADR-031 | Kernel performance validation, benchmark harness work |

**Status:** Proposed

**Relationship to other documents:**

- The [Rust Kernel Spec](rust-kernel-spec.md) defines performance targets this
  harness validates
- The [Architecture Evolution](anvil-architecture-evolution.md) document defines
  H1/H2 scope — benchmarks track readiness for each phase
- KERN-043 (performance benchmarks) in the
  [KERN module](../../plans/modules/rust-kernel.aps.md) is the parent work item

---

## Purpose

Define a benchmarking strategy for the Rust Watcher Kernel that answers two
distinct questions:

1. **Regression detection** — "Did this commit make anything slower?" (micro-
   benchmarks via Criterion, run on every PR)
2. **Capacity discovery** — "What are the actual limits?" (stress tests via
   `anvil-bench` binary, run on demand)

The existing Criterion benchmarks in `crates/anvil-kernel/benches/kernel.rs`
cover question 1 at small scale. This spec extends coverage to realistic scales
and adds question 2.

---

## 1. Micro-Benchmarks (Criterion — existing + extensions)

Micro-benchmarks measure **latency of individual operations** under controlled
conditions. They run in CI on every PR to detect regressions.

### 1.1 Existing Groups

| Group                | What it measures                      | Current scale         |
| -------------------- | ------------------------------------- | --------------------- |
| `cold_graph_build`   | Full scan → parse → graph             | 10, 50, 100 files     |
| `incremental_update` | Reparse + graph delta for single file | 1 file                |
| `policy_evaluation`  | All H1 invariants on one delta        | 1 delta, 4 invariants |
| `event_emission`     | 1000 progress events through mpsc     | 1000 events           |

### 1.1.1 0.5.0-beta Scanner & Real-Time Validation Results

Two new benchmark surfaces landed during the 0.5.0-beta cycle:

- **SCAN parallel scan** — measured a **7.39× wall-time improvement** on a
  synthetic 3,000-file surface over the previous serial scan baseline. The
  benchmark exercises the shared gitignore-aware discovery walk plus the rayon
  scan pattern that `gate`, `audit`, `check`, `drift`, policy, architecture
  validation, and the watcher all consume. First-run scans cap their pool via
  `ANVIL_SCAN_THREADS` (default `min(num_cpus, 4)`) to keep the speedup without
  starving TUI or editor work.
- **RTAI-001 mid-edit secret-detection** — the phase-0 spike measured the
  mid-edit secret-detection loop at **about 1.4 ms p95** over 1024 iterations,
  roughly 60× under the ADR-031 warm-path budget. The benchmark exercises a
  single `scan_buffer` method with a mode discriminator selecting save-time
  versus mid-edit validation, and is wired into the standard Criterion harness
  so regressions are visible on every PR that touches `anvil-checks` secret
  scanning.

ADR-031 pins the latency budgets these benchmarks gate against (save-time,
mid-edit, gate paths) so future real-time validation work has an explicit
performance envelope.

### 1.2 Extensions Needed

| Group                         | What it measures                           | Scale                            |
| ----------------------------- | ------------------------------------------ | -------------------------------- |
| `cold_graph_build` (extended) | Same, at realistic scale                   | 500, 1k, 5k, 10k files           |
| `incremental_update_varied`   | Reparse files of varying complexity        | 10 LOC, 100 LOC, 500 LOC, 1k LOC |
| `symbol_extraction`           | Extract symbols from parsed AST            | Per-language, varied complexity  |
| `import_resolution`           | Resolve imports against known file set     | 100, 1k, 10k known files         |
| `trust_annotation`            | Annotate trust levels for a file's symbols | Varied import patterns           |
| `debouncer_throughput`        | Record + tick cycle under burst            | 100, 500, 1k pending changes     |
| `filter_throughput`           | `should_process` calls per second          | 10k paths with varied patterns   |
| `graph_query`                 | `symbols_in_file`, `outgoing_edges`        | 1k, 10k, 50k node graphs         |

### 1.3 Fixture Generator

The current `generate_fixture(n)` produces files with identical structure. This
is insufficient for capacity testing — real repos have:

- Varying file sizes (10–2000 LOC)
- Cross-module imports creating graph edges
- Different ratios of public/internal symbols
- Deep directory nesting

The extended generator accepts a `RepoSpec`:

```rust
struct RepoSpec {
    /// Total number of source files to generate.
    file_count: usize,
    /// Average lines of code per file (normal distribution, σ = avg/3).
    avg_loc: usize,
    /// Fraction of files that import from other generated files (0.0–1.0).
    import_density: f64,
    /// Fraction of imports that intentionally cross layer boundaries (0.0–1.0).
    cross_layer_ratio: f64,
    /// Maximum directory nesting depth.
    nesting_depth: usize,
    /// Architecture layer names and their relative sizes.
    layers: Vec<(String, f64)>,
}
```

The generator:

1. Creates a directory tree with `nesting_depth` levels
2. Distributes files across layers according to relative sizes
3. Generates TypeScript files with `avg_loc` ± variance
4. Wires `import_density × file_count` import edges between files
5. Makes `cross_layer_ratio` of those imports cross layer boundaries
6. Outputs a manifest describing the generated structure (for validation)

---

## 2. Stress Tests (`anvil-bench` binary — new)

Stress tests discover **capacity limits and degradation curves**. They are run
on demand (not in CI) and produce JSON reports.

### 2.1 Crate Structure

```
crates/anvil-bench/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI: anvil-bench <scenario> [options]
│   ├── fixture.rs           # RepoSpec + generator
│   ├── measure.rs           # RSS, timing, event counting
│   ├── report.rs            # JSON output + summary table
│   └── scenarios/
│       ├── mod.rs
│       ├── watcher_saturation.rs
│       ├── graph_memory.rs
│       ├── incremental_throughput.rs
│       ├── policy_scaling.rs
│       └── cold_start_scaling.rs
```

### 2.2 Scenarios

#### 2.2.1 Watcher Saturation

**Question:** How many concurrent file changes can the watcher handle before
events are dropped or latency degrades?

**Method:**

1. Generate repo with N files (parameterised: 1k, 5k, 10k, 50k)
2. Start `start_watcher()` on the repo root
3. Write to M files simultaneously (burst: 10, 50, 100, 500, 1000)
4. Measure:
   - Events received vs events expected (drop rate)
   - Time from write to `ChangeBatch` receipt (latency p50/p95/p99)
   - Peak RSS during burst
   - Debouncer backpressure flush count
5. Binary search for the M where drop rate exceeds 1%

**Targets (from kernel spec):**

- File detection latency p99 < 20ms
- No event drops at M ≤ 500 (current `max_pending` default)
- Graceful degradation (backpressure flush, not crash) above limit

#### 2.2.2 Graph Memory Ceiling

**Question:** At what repo size does the kernel exceed the 500MB memory budget?

**Method:**

1. Generate repos at 1k, 5k, 10k, 25k, 50k, 100k files
2. Run `run_embedded()` for each
3. Measure peak RSS at each tier (via `/proc/self/statm` on Linux)
4. Record: node count, edge count, AST cache size, total RSS
5. Plot memory vs file count, extrapolate ceiling

**Targets:**

- < 500MB RSS for 2000 files (~100k LOC)
- Linear or sub-linear memory growth with file count
- AST cache should dominate (not graph structure)

#### 2.2.3 Incremental Throughput Under Sustained Load

**Question:** At what change rate does the kernel fall behind?

**Method:**

1. Generate 5k-file repo, run `run_embedded()` to build initial graph
2. Write files at increasing rates: 1/sec, 5/sec, 10/sec, 50/sec, 100/sec
3. For each rate, sustain for 60 seconds
4. Measure:
   - Parse + graph update latency per file (p50/p95/p99)
   - Queue depth over time (is it growing?)
   - Whether the kernel ever falls behind (queue depth monotonically increasing)
5. Find the rate where queue depth starts growing unboundedly

**Targets:**

- Incremental update < 100ms per file (from kernel spec)
- Sustain 10 changes/sec without queue growth on medium hardware
- Graceful degradation above limit (bounded queue, not OOM)

#### 2.2.4 Policy Evaluation Scaling

**Question:** How many invariants can we evaluate per delta within the 100ms
budget?

**Method:**

1. Build a 5k-file graph
2. Generate synthetic `GraphDelta` with N added symbols/edges
3. Register M invariants (4 real H1 + synthetic no-op invariants)
4. Measure evaluation time for M = 4, 10, 25, 50, 100, 200
5. Also vary delta size: 1 symbol, 10 symbols, 50 symbols

**Targets:**

- 4 invariants (H1) < 1ms per delta
- 50 invariants < 10ms per delta
- Find the M where evaluation exceeds 100ms

#### 2.2.5 Cold Start Scaling

**Question:** At what repo size does cold start exceed 3 seconds?

**Method:**

1. Generate repos at 500, 1k, 2k, 5k, 10k, 25k, 50k files
2. Run `run_embedded()` for each, measure wall-clock duration
3. Break down: walk time, parse time, graph build time, policy time
4. Find the file count where total exceeds 3 seconds

**Targets:**

- < 3 seconds for 100k LOC (from kernel spec, ~2000 files at 50 LOC avg)
- Linear scaling with file count (not quadratic)

### 2.3 Output Format

Each scenario produces a JSON report:

```json
{
  "scenario": "watcher_saturation",
  "timestamp": "2026-03-17T14:30:00Z",
  "machine": {
    "os": "linux",
    "arch": "x86_64",
    "cpus": 8,
    "ram_gb": 32
  },
  "runs": [
    {
      "params": { "file_count": 10000, "burst_size": 500 },
      "results": {
        "events_expected": 500,
        "events_received": 487,
        "drop_rate": 0.026,
        "latency_p50_ms": 12,
        "latency_p95_ms": 45,
        "latency_p99_ms": 89,
        "peak_rss_mb": 210,
        "backpressure_flushes": 3
      }
    }
  ]
}
```

Reports are written to `bench-results/` (gitignored) and optionally to stdout as
a formatted table for quick review.

### 2.4 Memory Measurement

On Linux, read `/proc/self/statm` before and after each operation. Fields:

- `VmRSS` — resident set size (physical memory)
- `VmHWM` — high-water mark RSS

For more precise allocation tracking, optionally compile with
`tikv-jemallocator` and use `jemalloc_ctl` to query:

- `stats.allocated` — bytes currently allocated
- `stats.resident` — bytes in physical memory
- `stats.mapped` — bytes mapped (includes unused pages)

The default (no jemalloc) uses `/proc/self/statm` which is sufficient for
capacity discovery. Jemalloc is opt-in via a cargo feature flag.

---

## 3. CI Integration

### 3.1 Criterion Benchmarks (every PR)

The existing `cargo bench` run should be added to the Rust CI pipeline.
Criterion produces HTML reports in `target/criterion/` — these can be uploaded
as CI artefacts.

**Regression detection:** Criterion compares against the previous baseline
automatically. If a benchmark regresses by > 5%, the report flags it. CI should
not gate on this (benchmarks are noisy on shared runners) but should surface the
report.

### 3.2 Stress Tests (nightly / on demand)

Stress tests are too slow for per-PR CI (some scenarios take minutes). Run them:

- Nightly on a dedicated runner (consistent hardware)
- On demand via `workflow_dispatch` for specific scenarios
- Before releases to validate performance targets

Store results in `bench-results/` as JSON for trend analysis.

---

## 4. README Integration

The root `README.md` should include a **Rust Kernel Benchmarks** section
showing:

- Current Criterion results (cold build, incremental, policy, events)
- Performance targets from the kernel spec
- How to run benchmarks locally
- Link to this spec for methodology

This section is manually updated after significant benchmark runs (not
automated).

---

## 5. What Can Be Done Today

With the existing `crates/anvil-kernel/benches/kernel.rs` infrastructure:

### Immediately runnable

```bash
# Run all Criterion micro-benchmarks
cargo bench --bench kernel

# Run specific group
cargo bench --bench kernel -- cold_graph_build
cargo bench --bench kernel -- incremental_update
cargo bench --bench kernel -- policy_evaluation
cargo bench --bench kernel -- event_emission
```

### Quick wins (extend existing benchmarks, no new crate)

1. **Scale up `cold_graph_build`** — add 500, 1000, 5000 to the file count array
2. **Add `graph_query` group** — benchmark `symbols_in_file` and
   `outgoing_edges` on pre-built graphs
3. **Add `debouncer_throughput` group** — benchmark `record` + `tick` cycle
4. **Add `filter_throughput` group** — benchmark `should_process` on 10k paths

These require only adding functions to `kernel.rs` and extending
`generate_fixture`.

### Requires new crate (`anvil-bench`)

- Watcher saturation (needs real file I/O + timing)
- Graph memory ceiling (needs RSS measurement)
- Incremental throughput under sustained load (needs long-running process)
- Policy evaluation scaling (needs synthetic invariants)
- Cold start scaling at large scale (needs large fixture generation)

---

## 6. Non-Goals

- Benchmarking the TypeScript engine (that's the dual-run harness, KERN-042)
- Benchmarking TUI rendering (that's RATS scope)
- Benchmarking network I/O (daemon mode is post-H1)
- Continuous benchmark tracking service (CI artefacts are sufficient for now)
