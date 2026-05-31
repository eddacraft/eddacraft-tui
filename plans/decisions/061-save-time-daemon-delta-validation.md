# ADR-061: Save-time governance is daemon-mediated delta validation

## Status

Proposed

## Date

2026-05-31

## Context

Anvil's product claim is real-time governance: as an agent edits a repo, Anvil
validates the change at the point of change. Today `anvil watch` delivers that
in the most expensive way possible. On every debounced save it builds and spawns
a **cold `anvil check --all` child process**
(`crates/anvil-cli/src/commands/watch.rs:377` default action →
`:433` `build_action_command` → `:441` `Command::new` → `:455` `--all`). That
child cold-rebuilds the whole-repo graph and rayon-scans every file, then exits —
discarding all warm state.

A beta tester reported Anvil being CPU-expensive on their machine. A load probe
(`benchmarks/prototypes/anvil-load-probe.py`, measuring process-tree CPU under
sustained file churn across a ramp of concurrent agents) reproduced it: a single
agent's watch consumes ~7 cores during save storms, and ~2 concurrent agents
saturate a 16-core box. An `--action none` control measured 0.03 cores — proving
the cost is the per-save reaction (the whole-repo scan), not the watcher, the
file events, or idle daemon overhead. The existing `watch_resource_budget`
benchmark reports ~0% CPU because it measures the idle path (no file events, parent
pid only, static corpus) — it cannot see this failure mode.

Two structural multipliers make it worse:

- **No warm state is reused.** The kernel maintains incremental
  `SymbolGraph`/`DependencyGraph` state, but it lives in the watch process and is
  thrown away; `check --all` rebuilds cold each save.
- **Rayon is capped per-process, not per-host**
  (`crates/anvil-rayon-init/src/lib.rs:74`, `(cores/2).max(1)`), so N Anvil
  processes oversubscribe the machine.

Crucially, the daemon needed to fix this **already exists**. `anvil intercept`
is a permissioned, concurrent, warm-state daemon
(`crates/anvil-intercept/src/lib.rs:915` `run_foreground`,
`crates/anvil-intercept/src/ipc.rs:780` serve loop with semaphore-gated
admission at `:831`) that already validates in-flight buffers via the
`anvil/scan_buffer` JSON-RPC method
(`crates/anvil-intercept-proto/src/protocol.rs:83`, dispatch
`crates/anvil-intercept/src/ipc.rs:2208`). And the check logic is already a
scoped, in-process library call —
`run_antipattern_check(files, config, workspace_root)`
(`crates/anvil-checks/src/antipattern/check.rs:95`) takes an arbitrary file list
and returns diagnostics synchronously, no subprocess required. The gap is that
`watch` (and `anvil mcp serve`) do not use any of it.

ADR-015 already proposes the intercept daemon as the enforcement control plane,
and ADR-031 defines the validation-latency rubric. This ADR locks the *product
contract* for the save-time path so the daemon, watch, MCP, and Graph V2 work
(modules INTD, DRVR, RLB, GV2) sequence behind one decision rather than each
inventing a save-time model. A decision is needed now because the CPU report is a
live field issue and the cheap tactical fix and the architectural fix must not be
conflated.

## Decision

**Save-time governance is delta validation mediated by the intercept daemon over
warm graph state. A whole-repo scan is never the default reaction to a single
save.** The daemon is scoped per ADR-036 — one daemon per `(uid, os)` execution
scope, serving every workspace under it; "warm model", work budget, and
assurance state below are **per-workspace state held inside that one daemon**,
not one daemon per repo. Specifically:

1. **Remove `check --all` from the per-save hot path.** The default save-time
   action in `watch.rs` stops spawning a cold whole-repo child. Whole-repo scans
   become explicit (`anvil check`), background, or CI-driven — not per-save.

2. **The intercept daemon is the save-time validation authority.** It gains three
   JSON-RPC methods alongside the existing `anvil/scan_buffer`, on the same
   transport, framing, and diagnostic envelope:
   - `anvil/validate_paths` — validate a set of changed on-disk paths against
     warm state; returns the existing diagnostic envelope plus a
     `workspace_assurance` marker.
   - `anvil/workspace_status` — report per-workspace assurance state
     (`clean | stale | pending | running`).
   - `anvil/request_full_scan` — enqueue an explicit/background full scan.
   `validate_paths` calls `run_antipattern_check` (and sibling check libraries)
   with the **changed paths only**, against warm graph indexes.

3. **Watch, MCP, and intercept are thin clients of the one daemon** in that
   execution scope. `anvil watch` sends changed paths to `validate_paths`;
   `anvil mcp serve`'s `anvil_validate_write` re-points from its own in-process
   scan to the daemon so a repo running watch + MCP keeps one warm model
   (per-workspace) and never double-scans. This
   keeps MCP a *projection/client* of the authority, consistent with the Graph V2
   rule that MCP is not the control plane.

4. **The daemon owns the warm Graph V2 hot-read slice and the work budget.** The
   first slice is GV2-010 (per-file semantic extract) + GV2-011 (warm
   boundary/known-edge/dependency indexes) + GV2-020 (registry) + GV2-022
   (bounded hot-read API), hosted in the daemon. Rayon becomes one per-host pool,
   configurable. A blunt initial budget — one full scan per workspace at a time
   plus a bounded delta-validation pool — coalesces save storms; queued
   validations for the same path collapse to latest-state.

5. **The daemon is optional; absence degrades, never blocks.** If no daemon is
   reachable, watch falls back to a **scoped** `check` on the changed paths (never
   `--all`) or skip-with-warning; MCP falls back to its current in-process scan.
   This preserves ADR-001 (planless-first) and ADR-002 (warnings over blocks,
   exit 0).

6. **Workspace assurance is daemon state with a defined lifecycle.**
   `Clean → Stale` on a delta the warm indexes can't fully certify (config
   change, boundary-config edit, or path-set overflow); `Stale → Pending →
   Running → Clean` via the background scheduler; surfaced through
   `anvil/workspace_status` and rendered by `anvil status` / TUI / `--json`.

Sequencing (each phase ships and is provable independently):

- **Phase 1 (tactical, no daemon/GV2 dependency):** scope the per-save check to
  changed paths in `watch.rs` (RLB-007). Immediate CPU relief.
- **Phase 2:** `validate_paths` + watch client (INTD + RLB-001..005).
- **Phase 3:** warm Graph V2 hot-read slice in the daemon (GV2-010/011/020/022).
- **Phase 4:** daemon work budget + per-host rayon (RLB-002).
- **Phase 5:** workspace-assurance state + background full-scan scheduler +
  persistence/warm-start (GV2-021).

Every phase is gated by a process-tree CPU benchmark under sustained churn across
a concurrent-agent ramp (RLB-001/008), plus a `validate_paths` warm-read latency
case on the existing `ipc_roundtrip` criterion bench tied to ADR-031.

## Rationale

The save-time path is doing whole-repo, cold, uncoordinated work per save. The
fix is not "make `check --all` faster" — it is "stop doing it on every save."
Because the daemon, the warm graph maintainer, and a scoped in-process check
library already exist, the architectural fix is wiring and a warm-state slice,
not new runtime machinery. Decoupling the tactical changed-path scoping (Phase 1)
from the daemon work means the live CPU issue is relieved without waiting on the
architecture.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Daemon-mediated delta validation (chosen)** | Reuses the existing daemon, protocol, and in-process check library; one warm model and one work budget across watch/MCP/intercept; fixes per-process rayon oversubscription; aligns with ADR-015/030/031 and GV2 | Requires warm graph state in the daemon (GV2 slice) and re-pointing MCP; assurance state is new surface |
| **Only scope the per-save check to changed paths** | Tiny, this-week, no daemon/GV2 dependency; large immediate CPU drop | Still cold-spawns per save; no shared warm state; doesn't fix cross-process oversubscription or multi-agent coordination. Adopted as Phase 1, not the end state |
| **Make `check --all` incremental/faster in-process** | No protocol/daemon changes | Still whole-repo per save; doesn't address concurrency, oversubscription, or the buffer/MCP paths; optimises the wrong axis |
| **OS-level CPU throttling (nice/cgroups) of the watch child** | No code change to the scan path | Treats the symptom; makes governance laggy under load; doesn't reduce total work or coordinate agents; brittle cross-platform |
| **New standalone validation daemon** | Clean-slate design | Re-implements transport, framing, concurrency, and warm state the intercept daemon already ships and we already trust; two daemons to run |

## Consequences

- **Positive:** Per-save CPU drops from whole-repo to proportional-to-change;
  concurrent agents share one warm model and a bounded budget instead of N cold
  scans; one diagnostic envelope across watch/MCP/intercept; per-host rayon
  removes cross-process oversubscription; the product contract ("save-time =
  delta; full-repo = explicit/background") is locked for INTD/DRVR/RLB/GV2.
- **Negative:** The daemon gains warm graph state and a new method surface to
  maintain; MCP's validation path changes; a new workspace-assurance concept must
  be surfaced in CLI/TUI; full-repo assurance is no longer implicit on every save
  and must be scheduled/requested.
- **Risks:** (a) pressure to answer richer questions per save pulls expensive
  traversal onto the hot path, violating GV2's constraints; (b) making the daemon
  a hard requirement would break planless-first; (c) a delta that should
  invalidate the workspace (e.g. a boundary-config change) being mis-classified
  as `clean`.
- **Mitigations:** (a) the process-tree CPU bench + the `ipc_roundtrip`
  warm-read latency budget (ADR-031) keep the hot path honest and fail on
  regression; (b) mandatory daemon-absent fallback to scoped subprocess /
  in-process scan, exit 0; (c) explicit `Stale` transition on config/boundary
  edits and path-set overflow, with the background scheduler reconciling to
  `Clean`.

## References

- Related ADRs: ADR-015 (intercept-loop enforcement), ADR-031 (validation
  latency rubric), ADR-036 (daemon scope and boundaries), ADR-030 (surface
  drivers on the daemon), ADR-001 (planless-first), ADR-002 (warnings over
  blocks)
- APS modules: RLB-001/002/005/007/008 (resource-load-benchmarking),
  GV2-010/011/020/021/022 (graph-v2-foundation), INTD (intercept daemon), DRVR
  (surface-drivers)
- Evidence: process-tree load probe `benchmarks/prototypes/anvil-load-probe.py`
  (in-repo); CPU field report and tester-diagnostics tracking in issue #2156
