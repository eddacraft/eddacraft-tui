<!--
APS Module: Kernel Benchmarking
================================
Regression detection via Criterion micro-benchmarks and capacity discovery via
stress-test harness. Covers watcher saturation, graph memory, incremental
throughput, policy scaling, and cold start limits.

Scopes: BENCH (main)
-->

# Kernel Benchmarking

| ID    | Owner | Status      |
| ----- | ----- | ----------- |
| BENCH | —     | Complete |

## Purpose

Build a benchmarking harness that answers two questions: "did this commit make
anything slower?" (regression detection) and "what are the actual limits?"
(capacity discovery). Publishes results to the root README and produces JSON
reports for trend analysis.

**Why:** The kernel spec defines performance targets (cold build <3s, incremental
<100ms, memory <500MB, detection <20ms) but we have no way to validate them at
realistic scale. The existing Criterion benchmarks cover micro-operations at
10–100 files — they don't test what happens at 10k files or under sustained
change load. Without capacity data, we can't prioritise optimisation work or
confidently ship.

**Spec:** [Kernel Benchmarking Specification](../../docs/architecture/kernel-benchmarking-spec.md)

## In Scope

- Extended Criterion micro-benchmarks (scale up existing groups, add new groups)
- Parameterised synthetic repo generator (`RepoSpec`)
- `anvil-bench` stress-test binary with five scenarios
- JSON report output format
- RSS memory measurement (Linux `/proc/self/statm`, optional jemalloc)
- README section with current benchmark results and run instructions
- CI integration for Criterion on PRs

## Out of Scope

- Benchmarking the TypeScript engine (see KERN-042 dual-run harness)
- TUI rendering benchmarks (see RATS module)
- Daemon mode / network benchmarks (post-H1)
- Continuous benchmark tracking service
- Automated CI gating on benchmark results (too noisy on shared runners)

## Interfaces

**Depends on:**

- `anvil-kernel` — all public APIs (embedded, watcher, parser, graph, policy)
- `anvil-kernel-types` — event types, graph types
- Criterion 0.5 (existing dev-dependency)

**Exposes:**

- `anvil-bench` binary (`cargo run -p anvil-bench -- <scenario>`)
- JSON reports in `bench-results/` (gitignored)
- Criterion HTML reports in `target/criterion/`
- README benchmark section

## Constraints

- Stress tests must not run in per-PR CI (too slow, too noisy)
- Criterion benchmarks should run on the existing Rust CI pipeline
- Memory measurement must work without jemalloc by default
- Fixture generation must be deterministic (seeded RNG) for reproducibility
- Reports must include machine metadata for cross-machine comparison

## Ready Checklist

Change status to **Ready** when:

- [x] Existing Criterion benchmarks exist and run (`cargo bench --bench kernel`)
- [x] Kernel public API is stable enough to benchmark (Phases 1-2 done)
- [x] Performance targets defined in kernel spec
- [x] Benchmarking spec written and reviewed

---

## Phase 1 — Micro-Benchmark Extensions

> Extend the existing `crates/anvil-kernel/benches/kernel.rs` with larger scale
> and new benchmark groups. No new crate needed.

### BENCH-001: Scale cold graph build benchmarks to 500/1k/5k files

- **Status:** Complete
- **Intent:** Validate cold graph build time scales linearly (not quadratically)
  with file count, catching O(n²) regressions in the parse → extract → graph
  pipeline
- **Expected Outcome:** Criterion results for 500, 1000, and 5000 files showing
  linear scaling. Results published to README.
- **Validation:** `cargo bench --bench kernel -- cold_graph_build` completes and
  produces HTML report
- **Files:** `crates/anvil-kernel/benches/kernel.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### BENCH-002: Add graph query benchmarks

- **Status:** Complete
- **Intent:** Measure `symbols_in_file` and `outgoing_edges` query performance
  on pre-built graphs of varying size, establishing baseline for graph lookups
  that the policy engine depends on
- **Expected Outcome:** Criterion group `graph_query` with benchmarks at 1k, 5k,
  and 10k node graphs
- **Validation:** `cargo bench --bench kernel -- graph_query` completes
- **Files:** `crates/anvil-kernel/benches/kernel.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### BENCH-003: Add debouncer throughput benchmarks

- **Status:** Complete
- **Intent:** Measure debouncer `record` + `tick` cycle throughput under burst
  conditions to establish backpressure limits
- **Expected Outcome:** Criterion group `debouncer_throughput` with benchmarks
  at 100, 500, and 1000 pending changes
- **Validation:** `cargo bench --bench kernel -- debouncer_throughput` completes
- **Files:** `crates/anvil-kernel/benches/kernel.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### BENCH-004: Add incremental update benchmarks with varied file complexity

- **Status:** Complete
- **Intent:** Measure how parse + graph update latency scales with file
  complexity (LOC), not just file count — catching regressions in symbol
  extraction for large files
- **Expected Outcome:** Criterion group `incremental_update_varied` with
  benchmarks for 10, 100, 500, and 1000 LOC files
- **Validation:** `cargo bench --bench kernel -- incremental_update_varied`
  completes
- **Files:** `crates/anvil-kernel/benches/kernel.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### BENCH-005: Add policy evaluation scaling benchmarks

- **Status:** Complete
- **Intent:** Measure how policy evaluation time scales with number of registered
  invariants and delta size, finding the invariant count where evaluation exceeds
  the 100ms budget
- **Expected Outcome:** Criterion group `policy_scaling` with benchmarks for
  4, 10, 25, 50 invariants × 1, 10, 50 symbol deltas
- **Validation:** `cargo bench --bench kernel -- policy_scaling` completes
- **Files:** `crates/anvil-kernel/benches/kernel.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### BENCH-006: Publish benchmark results to root README

- **Status:** Complete
- **Intent:** Add a Rust Kernel Benchmarks section to `README.md` showing
  current Criterion results, performance targets, and instructions for running
  benchmarks locally
- **Expected Outcome:** README contains benchmark table with cold build,
  incremental, policy, and event emission numbers from a representative run
- **Validation:** Section visible in README, numbers correspond to a recent
  `cargo bench` run
- **Files:** `README.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** BENCH-001

---

## Phase 2 — Stress Test Harness

> New `anvil-bench` crate with parameterised repo generator, RSS measurement,
> and capacity-finding scenarios.

### BENCH-010: Parameterised repo generator (`RepoSpec`)

- **Status:** Complete (PR #681)
- **Intent:** Build a fixture generator that produces realistic synthetic repos
  with configurable file count, LOC distribution, import density, cross-layer
  violations, and nesting depth — replacing the simple `generate_fixture(n)`
- **Expected Outcome:** `RepoSpec` struct and `generate_repo(spec)` function
  that produces deterministic (seeded) temp directories with varied file content
  and cross-module imports
- **Validation:** Generate a 5k-file repo, verify import edges match requested
  density, verify layer distribution matches spec
- **Files:** `crates/anvil-bench/src/fixture.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### BENCH-011: RSS memory measurement utilities

- **Status:** Complete (PR #681)
- **Intent:** Provide cross-platform memory measurement (Linux: `/proc/self/statm`,
  optional jemalloc via feature flag) for tracking peak RSS during scenarios
- **Expected Outcome:** `measure_rss()` function returning current RSS in bytes,
  `MemoryTracker` struct that records high-water mark
- **Validation:** Unit test confirms RSS increases after allocating a large vec
- **Files:** `crates/anvil-bench/src/measure.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

### BENCH-012: JSON report output + summary table

- **Status:** Complete (PR #681)
- **Intent:** Produce machine-readable JSON reports with scenario results and
  machine metadata, plus a human-readable summary table on stdout
- **Expected Outcome:** `Report` struct serialised to `bench-results/<scenario>-<timestamp>.json`,
  summary table printed to stdout
- **Validation:** Run any scenario, verify JSON is valid and contains all fields
  from the spec
- **Files:** `crates/anvil-bench/src/report.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### BENCH-013: Watcher saturation scenario

- **Status:** Complete (PR #681)
- **Intent:** Discover the maximum burst size the watcher handles without
  dropping events — answering "how many simultaneous file changes before events
  are lost?"
- **Expected Outcome:** JSON report showing drop rate, latency percentiles, and
  peak RSS at each burst size. Binary search identifies the burst size where drop
  rate exceeds 1%.
- **Validation:** Run scenario, verify reported event counts match actual writes
  at small burst sizes (0% drop rate)
- **Files:** `crates/anvil-bench/src/scenarios/watcher_saturation.rs`
- **Confidence:** medium (real I/O timing is inherently variable)
- **Priority:** Critical
- **Dependencies:** BENCH-010, BENCH-011, BENCH-012

---

### BENCH-014: Graph memory ceiling scenario

- **Status:** Complete (PR #681)
- **Intent:** Find the repo size where the kernel exceeds the 500MB memory
  budget — answering "how many files can we handle?"
- **Expected Outcome:** JSON report showing RSS at each tier (1k, 5k, 10k, 25k,
  50k, 100k files) with node/edge/cache breakdown
- **Validation:** Run scenario, verify RSS measurements are monotonically
  increasing with file count
- **Files:** `crates/anvil-bench/src/scenarios/graph_memory.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** BENCH-010, BENCH-011, BENCH-012

---

### BENCH-015: Incremental throughput under sustained load scenario

- **Status:** Complete (PR #681)
- **Intent:** Find the change rate where the kernel falls behind — answering
  "how many file saves per second can it process without accumulating backlog?"
- **Expected Outcome:** JSON report showing per-rate metrics (1/sec, 5/sec,
  10/sec, 50/sec, 100/sec) over 60-second windows: latency percentiles, queue
  depth trend, and whether queue is bounded
- **Validation:** At 1/sec rate, queue depth should remain at 0 throughout
- **Files:** `crates/anvil-bench/src/scenarios/incremental_throughput.rs`
- **Confidence:** medium (depends on hardware and OS scheduler)
- **Priority:** High
- **Dependencies:** BENCH-010, BENCH-011, BENCH-012

---

### BENCH-016: Policy evaluation scaling scenario

- **Status:** Complete (PR #681)
- **Intent:** Find the invariant count where per-delta evaluation exceeds the
  100ms budget — answering "how many policy rules can we run per change?"
- **Expected Outcome:** JSON report showing evaluation time for 4, 10, 25, 50,
  100, 200 invariants × varied delta sizes
- **Validation:** At 4 invariants (H1), evaluation should be < 1ms per delta
- **Files:** `crates/anvil-bench/src/scenarios/policy_scaling.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** BENCH-010, BENCH-012

---

### BENCH-017: Cold start scaling scenario

- **Status:** Complete (PR #681)
- **Intent:** Find the repo size where cold start exceeds the 3-second target —
  answering "how big a repo can we handle within the latency budget?"
- **Expected Outcome:** JSON report showing wall-clock time broken down by phase
  (walk, parse, graph, policy) at each repo size tier
- **Validation:** At 2000 files, total should be < 3 seconds
- **Files:** `crates/anvil-bench/src/scenarios/cold_start_scaling.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** BENCH-010, BENCH-011, BENCH-012

---

## Phase 3 — CI Integration

### BENCH-020: Add Criterion benchmarks to Rust CI pipeline

- **Status:** Complete
- **Intent:** Run Criterion benchmarks on PRs touching kernel, checks, bench, or
  kernel-types crates, or changing `Cargo.toml`/`Cargo.lock`, uploading HTML
  reports as CI artefacts for regression detection
- **Expected Outcome:** CI workflow runs `cargo bench` for kernel, checks, and
  stress harnesses, uploading HTML reports as artefacts
- **Validation:** Open a PR changing kernel or checks code, verify benchmark
  artefacts appear in CI run for all targeted harnesses
- **Files:** `.github/workflows/bench.yml`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** BENCH-001

---

### BENCH-021: Nightly stress test workflow

- **Status:** Draft
- **Intent:** Run stress test scenarios nightly on dedicated hardware, storing
  JSON reports for trend analysis
- **Expected Outcome:** Nightly workflow runs all scenarios, uploads JSON reports,
  posts summary to a tracking issue or artefact
- **Validation:** Nightly run produces reports for all five scenarios
- **Files:** `.github/workflows/`
- **Confidence:** medium (requires dedicated runner)
- **Priority:** Low
- **Dependencies:** BENCH-013 through BENCH-017

---

## Performance Targets (from Kernel Spec)

| Metric | Target | Benchmark |
| ------ | ------ | --------- |
| Cold graph build (100k LOC) | < 3 seconds | BENCH-017 |
| Incremental update (single file) | < 100ms | BENCH-004, BENCH-015 |
| Event emission overhead | < 10ms | Existing (`event_emission`) |
| Memory footprint (medium repo) | < 500MB | BENCH-014 |
| File detection latency (p99) | < 20ms | BENCH-013 |
| tree-sitter parse (single file) | < 1ms | Existing (`incremental_update`) |

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Benchmark noise on shared CI runners | High | Low | Don't gate on results, use artefacts only |
| Fixture generation too slow at 50k+ files | Medium | Medium | Pre-generate and cache fixtures, parallel I/O |
| RSS measurement inaccurate due to OS caching | Medium | Low | Use jemalloc for precise allocation tracking |
| Watcher saturation varies by OS/filesystem | High | Medium | Document platform, run on consistent hardware |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Micro-Benchmark Extensions | 6 | Complete (6/6) |
| 2 — Stress Test Harness | 8 | Complete (code landed via PR #681; needs validation runs) |
| 3 — CI Integration | 2 | 1/2 complete (1 deferred — requires dedicated runner) |
| **Total** | **16** | **15/16 done (1 deferred)** |
