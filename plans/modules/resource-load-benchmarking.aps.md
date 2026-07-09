# Resource & Load Benchmarking

| ID  | Owner  | Priority | Status      | Progress |
| --- | ------ | -------- | ----------- | -------- |
| RLB | @aneki | high     | In Progress | 8/9      |

**Last reviewed:** 2026-05-31 — filed from the beta-tester high-CPU report;
tracked in GH [#2156](https://github.com/eddacraft/anvil-001/issues/2156).
RLB-001 + RLB-007 (the per-save CPU bugfix Tier-1 release freight) Merged
2026-05-31 via PR [#2184](https://github.com/eddacraft/anvil-001/pull/2184):
the load-ramp harness lands and the per-save `anvil check` now scopes to the
changed file (measured 1 agent 6.55 → 0.08 cores). The remaining items stay
**Proposed** pending the Tier-2 daemon / Graph V2 planning council
([ADR-061](../decisions/061-save-time-daemon-delta-validation.md)).

2026-06-12: RLB-002/-003/-004/-005/-008 confirmed in the v0.8.0-beta tag
(record: plans/releases/v0.8.0-beta.md) and advanced to Released/Shipped;
RLB-006 remains Proposed.

2026-07-07: RLB-009 added from the CLI command benchmark tool investigation
(`docs/reviews/cli-command-benchmark-tool-investigation.md`) to cover
finite-command process-level benchmarking inside `anvil-bench`; RLB-009 is
Ready and coordinates with TCOV-026 for routine benchmark/history-schema
alignment.

## Purpose

Give Anvil's long-running processes real CPU/RSS coverage and a manually-runnable
load harness that finds saturation tipping points. A beta tester reported high
CPU while the `watch_resource_budget` bench showed ~0%; the gap is structural,
not noise.

## Background

A prototype load probe (`benchmarks/prototypes/anvil-load-probe.py`) reproduced
the field report. On an 800-file repo, one file saved every 200 ms, measuring the
whole process tree (parent `utime+stime` plus reaped-children `cutime+cstime`):

| Agents (concurrent watch pipelines) | `--action check` | `--action none` (control) |
| ----------------------------------- | ---------------- | ------------------------- |
| 1                                   | 6.96 cores       | —                         |
| 2                                   | 12.61 cores      | —                         |
| 3                                   | 13.75 cores      | —                         |
| 4                                   | 14.09 cores      | 0.03 cores                |

Root cause: bare `anvil watch` defaults to `--action check`
(`crates/anvil-cli/src/commands/watch.rs`), which spawns a full-repo
`anvil check --all` subprocess on every debounced save (changed-path scoping is
not yet wired). One agent eats ~7 of 16 cores; ~2 agents saturate the box. With
`--action none` it is ~0%, so the per-save check subprocess is ~100% of the cost.

The existing `watch_resource_budget` bench cannot see this: it generates no file
events, samples only the parent pid, and uses 100 static files over a 3 s window.

## Coverage gaps this module closes

- Three long-running processes — `anvil watch`, the intercept daemon
  (`anvil intercept start`), and the MCP server (`anvil mcp serve --stdio`) — but
  only `watch` has any CPU/RSS bench, and it measures the idle path.
- No concurrent multi-process bench. Rayon caps at N/2 cores **per process**, so
  several concurrent Anvil processes scanning at once can oversubscribe cores.
- All resource/latency benches are Linux/Unix only; `notify` backends and IPC
  transports differ per OS, and the field rig may be macOS or Windows.

## In Scope

- A manually-runnable, multi-agent load harness that ramps concurrent agents and
  reports per-level process-tree CPU/RSS to find tipping points.
- CPU/RSS budgets for the watch default path, the intercept daemon, and the MCP
  server, plus a concurrent-all-three scenario.
- Cross-platform (macOS + Windows) perf coverage.
- Behavioural remediation of the per-save full-repo check cost.

## Out of Scope

- Throughput micro-benchmarks already covered by the kernel/checks criterion
  suites.
- Reviving the offline self-hosted nightly `bench` runner (tracked separately).

## Work Items

### RLB-001: Multi-agent load-ramp harness (tipping points)

- **Status:** Released/Shipped via v0.7.4-beta (2026-06-01; merge commit
  `72f2de98` confirmed in tag). Merged 2026-05-31 via PR #2184
- **Intent:** Promote the prototype into a committed, manually-runnable harness
  that ramps N concurrent agents and reports process-tree CPU/RSS per level.
- **Expected Outcome:** One command prints a saturation table across agent counts
  and watch actions; seeded by `benchmarks/prototypes/anvil-load-probe.py`.
- **Validation:** `bash scripts/bench/load-ramp.sh --smoke` emits a per-level table.

### RLB-002: Fix watch_resource_budget to measure the real default

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-02 via PR #2228 (ADR-061 sub-phase A freight)
- **Intent:** Make the budget bench generate sustained churn, run the default
  `check` action, and measure the whole process tree (not just the parent pid).
- **Expected Outcome:** The bench reflects production per-save cost and regresses
  if it grows.
- **Validation:** `cargo bench -p anvil-bench --bench watch_resource_budget`
- **Dependencies:** RLB-001

### RLB-003: Intercept daemon CPU/RSS budget + SLO

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-02 via PR #2228 (ADR-061 sub-phase A freight)
- **Intent:** Add a steady-state and burst CPU/RSS bench for the intercept daemon.
- **Expected Outcome:** Daemon footprint is gated by a budget, not just latency
  (burst = IPC connection-churn; `scan_buffer` load deferred — needs a
  peer-PID-authenticated session).
- **Validation:** `cargo bench -p eddacraft-anvil-intercept --bench intercept_resource_budget`

### RLB-004: MCP server CPU/RSS budget + SLO

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-02 via PR #2228 (ADR-061 sub-phase A freight)
- **Intent:** Spawn `anvil mcp serve --stdio`, drive sustained `tools/call`
  load, and measure CPU/RSS.
- **Expected Outcome:** MCP server gains its first resource budget.
- **Validation:** `cargo bench -p anvil-bench --bench mcp_resource_budget`

### RLB-005: Concurrent multi-process bench

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-02 via PR #2228 (ADR-061 sub-phase A freight)
- **Intent:** Run watch + MCP + intercept together under load to expose
  cross-process rayon oversubscription.
- **Expected Outcome:** Aggregate CPU/RSS under realistic concurrency is measured
  and budgeted.
- **Validation:** `cargo bench -p anvil-bench --bench concurrent_processes`
- **Dependencies:** RLB-002, RLB-003, RLB-004

### RLB-006: Cross-platform perf coverage (macOS + Windows)

- **Status:** Proposed
- **Intent:** Extend the resource benches and bench CI matrix to macOS and
  Windows so per-OS notify/IPC cost is visible.
- **Expected Outcome:** Resource benches run green on macos-latest and
  windows-latest in CI.
- **Validation:** `.github/workflows/bench.yml` matrix includes macOS + Windows resource jobs.
- **Dependencies:** RLB-002

### RLB-007: Remediate per-save check cost

- **Status:** Released/Shipped via v0.7.4-beta (2026-06-01; merge commit
  `72f2de98` confirmed in tag). Merged 2026-05-31 via PR #2184
- **Intent:** Stop spawning a full-repo `check --all` per save — scope the action
  to changed paths and/or coalesce and cap concurrency.
- **Expected Outcome:** The load harness shows watch steady-state CPU drop under
  the budget at the measured tipping points.
- **Validation:** `cargo test -p eddacraft-anvil -- watch_action_scope` and a before/after delta from the RLB-001 harness.
- **Dependencies:** RLB-001 (the harness proves the before/after; RLB-002's
  criterion-bench upgrade is a follow-on hardening, not a blocker for the fix)

### RLB-008: Define SLOs + wire the harness into CI

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-02 via PR #2228 (SLO + CI gate for sub-phase A)
- **Intent:** Set CPU/RSS SLOs for watch/daemon/MCP and make the load harness
  runnable via `workflow_dispatch` (and nightly once a runner returns).
- **Expected Outcome:** Each process has a documented budget; the load harness is
  reproducible in CI.
- **Validation:** `pnpm bench` (or a dispatchable workflow) runs the harness and the budgets gate.
- **Dependencies:** RLB-002, RLB-003, RLB-004, RLB-005

### RLB-009: Per-command CLI benchmark runner

- **Status:** Done 2026-07-07 — `anvil-bench-command` runner implemented in `crates/anvil-bench` with safe temp state, redacted argv JSON, direct Anvil argv execution, timeout cleanup, CARGO_TARGET_DIR-aware binary resolution, README guardrails, and `status --verify` smoke evidence. Routine-suite/history integration intentionally deferred until comparable baselines exist; manual workflow documented.
- **Intent:** Add an Anvil-specific runner that measures individual finite
  `anvil` CLI commands end-to-end with repeat/warmup controls, isolated state,
  safe argument redaction, and structured JSON reports.
- **Expected Outcome:** Operators can benchmark one CLI command without editing
  Criterion benches or the routine `pnpm bench` script; curated command
  benchmarks can be normalised into `benchmarks/history/` when the JSON shape is
  stable. The runner executes only the resolved `anvil` binary, defaults to
  temporary state/fixtures, records no raw argument values by default, and reuses
  `anvil-bench` process cleanup plus Linux process-tree CPU/RSS sampling.
- **Scope:** `crates/anvil-bench` command-runner binary/library, deterministic
  fixture selection for finite commands, safe benchmark environment defaults,
  per-iteration + aggregate JSON output, optional routine-suite/history-schema
  integration for a small curated command set.
- **Non-scope:** Generic shell benchmarking; long-running `watch`, intercept
  daemon, MCP, or concurrent process benchmarks already owned by RLB-002..005;
  new product CLI commands; automated CI gating on noisy local command timings.
- **Files:** `crates/anvil-bench/src/cli_command.rs`,
  `crates/anvil-bench/src/bin/anvil-bench-command.rs`,
  `crates/anvil-bench/tests/cli_command.rs`, `crates/anvil-bench/src/lib.rs`,
  `crates/anvil-bench/README.md`, optional `scripts/bench/run.sh`,
  optional `scripts/bench/to-history.py`, optional `benchmarks/README.md`.
- **Dependencies:** RLB-002 process-tree sampling and spawn helpers;
  investigation note `docs/reviews/cli-command-benchmark-tool-investigation.md`;
  coordinates with TCOV-026 when routine-suite/history surfaces are changed.
- **Validation:** `cargo test -p anvil-bench cli_command`;
  `cargo clippy -p anvil-bench --all-targets -- -D warnings`;
  `ANVIL_BENCH_ANVIL_BIN=target/release/anvil cargo run -p anvil-bench --bin anvil-bench-command -- --name status-verify --repeat 3 --warmup 1 --fixture empty -- status --verify`;
  `pnpm docs:check` if docs/history surfaces change.
- **Closeout:** Done 2026-07-07 — targeted tests and clippy passed; release `status --verify` smoke emitted 3 successful samples. Full `cargo test -p anvil-bench`, APS/docs/format validation run at branch closeout.
- **Confidence:** high
