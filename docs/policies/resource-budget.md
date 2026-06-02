# Resource Budgets — long-running Anvil processes

| Type  | Authority     | Owner                                                                                                            | Status | Freshness                                                                                                       |
| ----- | ------------- | ---------------------------------------------------------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | RLB ([`plans/modules/resource-load-benchmarking.aps.md`](../../plans/modules/resource-load-benchmarking.aps.md)) | Live   | Last reviewed 2026-06-02 against `crates/anvil-bench/src/budget.rs` and `.github/workflows/resource-budget.yml` |

| Upstream                                                                                     | Downstream                                                                                    |
| -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `crates/anvil-bench` (`budget`, `proc_sampler`, `spawn`, `fixture`, `watch_resource_budget`) | `.github/workflows/resource-budget.yml`, `scripts/bench/run.sh`, `scripts/bench/load-ramp.sh` |

Pinned CPU/RSS ceilings for Anvil's three long-running processes —
`anvil watch`, the intercept daemon (`anvil intercept start`), and the MCP
server (`anvil mcp serve --stdio`) — plus a concurrent all-three aggregate.
Anvil's adoption test is that senior users do not notice it on their battery or
CPU graph during sustained daily use; these budgets are the hard line.

The ceilings live in source as `ResourceBudget` constants in
`crates/anvil-bench/src/budget.rs`. Bumping any field requires:

1. An entry in `plans/decisions/DECISION-LOG.md` recording the new value and why
   the previous ceiling is no longer reachable, and
2. A user-facing release note in the next candidate.

Silent drift defeats the point of the budget. The pinned test
`anvil_watch_v1_ceiling_is_pinned` makes the watch constant a visible diff, but
a reviewer can change both the constant and the test in one commit and the test
alone will go green. The DECISION-LOG + release note steps are the human gate
against an intentional bump landing without review.

## Unit

`steady_state_cpu_pct` is scaled so `100.0` is one fully-saturated core; a
process pinning four cores reads `400.0`, the concurrent aggregate of three busy
processes reads up to `300.0`+. `peak_rss_mib` is mebibytes of resident set. CPU
for a process _tree_ sums the root's `utime+stime` plus the reaped children's
`cutime+cstime`, so the per-save `anvil check` a watcher spawns is counted (the
original idle-path bench missed this — GH #2156).

## Ceilings

### `anvil watch` — idle (`ANVIL_WATCH_V1`)

| Axis             | Ceiling | Rationale                                                                                                                               |
| ---------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Steady-state CPU | 5%      | Below the per-core bar most laptop battery dashboards show; persistent background draw is the failure mode senior users complain about. |
| Peak RSS         | 200 MiB | Comparable to a single VS Code window's resident set on a quiet repo.                                                                   |

### `anvil watch` — default path under churn (`ANVIL_WATCH_CHURN_V1`)

The production default since GH #1913: every debounced save spawns a per-save
`anvil check`. This is the cost the idle ceiling cannot see (RLB-002).

| Axis             | Ceiling | Rationale                                                                                                                                                                             |
| ---------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Steady-state CPU | 50%     | RLB-007 scoped the per-save check to the changed path (~0.08 cores/agent, down from ~6.55); the ceiling carries headroom while tripping on a regression back toward whole-repo scans. |
| Peak RSS         | 300 MiB | Watcher plus a transient per-save check child.                                                                                                                                        |

### Intercept daemon — idle (`ANVIL_INTERCEPT_IDLE_V1`)

| Axis             | Ceiling | Measured floor (dev box, release) |
| ---------------- | ------- | --------------------------------- |
| Steady-state CPU | 5%      | 0% — daemon idles cold            |
| Peak RSS         | 96 MiB  | ~23 MiB                           |

### Intercept daemon — burst (`ANVIL_INTERCEPT_BURST_V1`)

Many short-lived connections each driving one JSON-RPC request through the full
accept → auth → parse → dispatch → serialise pipeline (RLB-003).

| Axis             | Ceiling | Measured floor (dev box, release, 4 workers)    |
| ---------------- | ------- | ----------------------------------------------- |
| Steady-state CPU | 200%    | ~101% (≈1 core)                                 |
| Peak RSS         | 128 MiB | ~23 MiB (flat vs idle — no per-connection leak) |

### MCP server — busy (`ANVIL_MCP_BUSY_V1`)

A single driver hammering `anvil_validate_write` so the server runs the real
embedded scan per buffer; MCP stdio is single-threaded and 1:1 (RLB-004).

| Axis             | Ceiling | Measured floor (dev box, release)             |
| ---------------- | ------- | --------------------------------------------- |
| Steady-state CPU | 150%    | ~94% (single-threaded ≈1 core), ~6.4k calls/s |
| Peak RSS         | 96 MiB  | ~24 MiB                                       |

### Concurrent all-three (`ANVIL_CONCURRENT_ALL_V1`)

watch + intercept daemon + MCP server under load at once, exposing cross-process
rayon oversubscription (each caps its pool at N/2 cores, so three at once can
oversubscribe the box) (RLB-005).

| Axis             | Ceiling | Rationale                                                                                                                               |
| ---------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Steady-state CPU | 800%    | Whole-box aggregate; the gate value is catching an aggregate regression, with the per-process budgets above catching per-process drift. |
| Peak RSS         | 700 MiB | Sum of all three trees' resident sets.                                                                                                  |

> **Calibration.** Every ceiling except the watch idle pair is a conservative
> quiet-box placeholder with headroom over the measured floor, not a tightened
> SLO. Final calibration on a dedicated runner is RLB-008's remaining work;
> until then the gates are sized to catch gross regressions without false
> failures on slower hardware.

## Reference fixture

The benches run against the synthetic repository produced by
`crates/anvil-bench::fixture` from a deterministic seed, so the same source
revision produces the same file tree on every run — that is what makes the
ceilings meaningful across machines and runners.

## Measurement protocol

Each bench spawns the real shipped `anvil` binary (set `ANVIL_BENCH_ANVIL_BIN`
to point at it; unset falls back to `target/debug/anvil` then
`target/release/anvil`), settles past the cold start, then samples the process
tree from `/proc` across the measurement window and feeds a `MeasurementSample`
to `anvil_bench::budget::evaluate`. A process that dies during startup or
mid-window is reported as a loud error, never measured as a frozen zombie (a
false "0% pass"). `evaluate` treats "exactly at the ceiling" as a Pass and emits
the raw derived values so CI logs show headroom without hiding slow drift.

**Host requirement:** the `watch` and concurrent benches need inotify headroom —
`anvil watch` refuses to start once the host's `fs.inotify` watch limit is
exhausted (common on a developer box running many worktree watchers). Run them
on a quiet box or a fresh CI runner.

## CI assertion

`.github/workflows/resource-budget.yml` builds a release `anvil` binary and runs
the budgets; each bench exits non-zero when `status != "pass"`, so the step is
the gate, and the JSON verdicts are uploaded as artifacts. The
`resource-budgets` job (push/PR + dispatch) runs the watch, intercept, and MCP
budgets; the heavier `concurrent-resource-budget` job (manual dispatch, nightly
once a dedicated runner returns) runs the concurrent budget and the
`load-ramp.sh --smoke` harness. `pnpm bench` runs the same budgets locally (skip
with `--skip-resource-budget`). The verdict shape is:

```jsonc
{
  "schema_version": 1,
  "status": "pass" | "fail_cpu" | "fail_rss" | "fail_both",
  "budget": { "steady_state_cpu_pct": 5.0, "peak_rss_mib": 200.0 },
  "sample": { "steady_state_cpu_pct": 0.8, "peak_rss_mib": 142.3 },
  "cpu_over_pct": -4.2,   // negative = headroom
  "rss_over_mib": -57.7
}
```

`schema_version` (pinned in Rust as `BUDGET_VERDICT_SCHEMA_VERSION`) is bumped
whenever a field is added or renamed. CI scripts should read it before parsing —
an unknown version is itself a failure mode.

## Out of scope (filed separately)

- Cold-start RAM footprint of `anvil start` (separate budget — file if/when
  there is a complaint).
- Watcher CPU during very large recursive scans (`audit` covers that surface).
- Cross-platform (macOS + Windows) resource coverage — RLB-006.
- `scan_buffer`-driven daemon load — needs a peer-PID-authenticated session;
  mid-edit scan latency is covered by `midedit_roundtrip`.

## Cross-references

- Budget constants — `crates/anvil-bench/src/budget.rs`
- Process-tree sampler — `crates/anvil-bench/src/proc_sampler.rs`
- Spawn helpers — `crates/anvil-bench/src/spawn.rs`
- Watch bench — `crates/anvil-bench/benches/watch_resource_budget.rs`
- Intercept bench —
  `crates/anvil-intercept/benches/intercept_resource_budget.rs`
- MCP bench — `crates/anvil-bench/benches/mcp_resource_budget.rs`
- Concurrent bench — `crates/anvil-bench/benches/concurrent_processes.rs`
- CI gate — `.github/workflows/resource-budget.yml`
- APS — `plans/modules/resource-load-benchmarking.aps.md` (RLB)
