# Resource & Load Benchmarking

| ID  | Owner  | Priority | Status   | Progress |
| --- | ------ | -------- | -------- | -------- |
| RLB | @aneki | high     | Proposed | 0/8      |

**Last reviewed:** 2026-05-30 — filed from the beta-tester high-CPU report;
tracked in GH [#2156](https://github.com/eddacraft/anvil-001/issues/2156).

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

- **Status:** Proposed
- **Intent:** Promote the prototype into a committed, manually-runnable harness
  that ramps N concurrent agents and reports process-tree CPU/RSS per level.
- **Expected Outcome:** One command prints a saturation table across agent counts
  and watch actions; seeded by `benchmarks/prototypes/anvil-load-probe.py`.
- **Validation:** `bash scripts/bench/load-ramp.sh --smoke` emits a per-level table.

### RLB-002: Fix watch_resource_budget to measure the real default

- **Status:** Proposed
- **Intent:** Make the budget bench generate sustained churn, run the default
  `check` action, and measure the whole process tree (not just the parent pid).
- **Expected Outcome:** The bench reflects production per-save cost and regresses
  if it grows.
- **Validation:** `cargo bench -p anvil-bench --bench watch_resource_budget`
- **Dependencies:** RLB-001

### RLB-003: Intercept daemon CPU/RSS budget + SLO

- **Status:** Proposed
- **Intent:** Add a steady-state and burst CPU/RSS bench for the intercept daemon.
- **Expected Outcome:** Daemon footprint is gated by a budget, not just latency.
- **Validation:** `cargo bench -p eddacraft-anvil-intercept --bench intercept_resource_budget`

### RLB-004: MCP server CPU/RSS budget + SLO

- **Status:** Proposed
- **Intent:** Spawn `anvil mcp serve --stdio`, drive sustained `tools/call`
  load, and measure CPU/RSS.
- **Expected Outcome:** MCP server gains its first resource budget.
- **Validation:** `cargo bench -p anvil-bench --bench mcp_resource_budget`

### RLB-005: Concurrent multi-process bench

- **Status:** Proposed
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

- **Status:** Proposed
- **Intent:** Stop spawning a full-repo `check --all` per save — scope the action
  to changed paths and/or coalesce and cap concurrency.
- **Expected Outcome:** The load harness shows watch steady-state CPU drop under
  the budget at the measured tipping points.
- **Validation:** `cargo test -p eddacraft-anvil-cli -- watch_action_scope` and a before/after delta from the RLB-001 harness.
- **Dependencies:** RLB-001, RLB-002

### RLB-008: Define SLOs + wire the harness into CI

- **Status:** Proposed
- **Intent:** Set CPU/RSS SLOs for watch/daemon/MCP and make the load harness
  runnable via `workflow_dispatch` (and nightly once a runner returns).
- **Expected Outcome:** Each process has a documented budget; the load harness is
  reproducible in CI.
- **Validation:** `pnpm bench` (or a dispatchable workflow) runs the harness and the budgets gate.
- **Dependencies:** RLB-002, RLB-003, RLB-004, RLB-005
