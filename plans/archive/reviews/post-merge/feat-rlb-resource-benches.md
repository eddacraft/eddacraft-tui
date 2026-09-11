# Post-merge test plan — feat/rlb-resource-benches (RLB-002/-003/-004/-005/-008)

Resource & load benchmarking suite: process-tree sampler + per-process CPU/RSS
budgets for `anvil watch`, the intercept daemon, and the MCP server, a
concurrent all-three bench, and the SLO docs + CI wiring.

## What could not be verified locally

The dev box's global inotify watch count is saturated by sibling worktree
watchers, so `anvil watch` refuses to start. The **watch** (RLB-002) and
**concurrent** (RLB-005) benches therefore could not produce a live measurement
here — both were validated to (a) compile + clippy clean, (b) drive the full
orchestration, and (c) fail *loud* on the watcher startup error rather than
report a false pass. The shared sampler/churn/spawn logic they rely on is
unit-tested and was validated end-to-end via the intercept + MCP live runs.

## Verify after merge (quiet box or CI with inotify headroom)

1. **CI gate** — confirm the `resource-budgets` job (Resource Budget workflow)
   is green on the merge commit: watch + intercept + MCP budgets each `pass`.
   Check the `Report inotify limits` step shows ample `max_user_watches`.
2. **Watch churn bench** (RLB-002) on a quiet box:
   `ANVIL_BENCH_ANVIL_BIN=target/release/anvil cargo bench -p anvil-bench --bench watch_resource_budget`
   — expect a non-zero CPU sample (churn drove per-save checks) under the
   `ANVIL_WATCH_CHURN_V1` ceiling, not ~0%.
3. **Concurrent bench** (RLB-005) via the dispatch-only job
   (`workflow_dispatch` → `concurrent-resource-budget`) or locally:
   `cargo bench -p anvil-bench --bench concurrent_processes` — expect an
   aggregate sample under `ANVIL_CONCURRENT_ALL_V1` and the `load-ramp.sh
   --smoke` step green.

## RLB-008 follow-up (not blocking this PR)

The intercept-burst, MCP, watch-churn, and concurrent ceilings are documented
`[placeholder]` gross-regression gates with wide headroom. Calibrate them on a
dedicated/quiet runner and tighten toward the measured floors, recording the new
values in `plans/decisions/DECISION-LOG.md` + a release note (per
`docs/policies/resource-budget.md`).

## Deferred / out of scope (tracked)

- `scan_buffer`-driven daemon burst load — needs a peer-PID-authenticated
  session; mid-edit scan latency stays covered by `midedit_roundtrip`.
- Cross-platform (macOS + Windows) resource coverage — RLB-006.
