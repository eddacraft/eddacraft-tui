# Anvil watch CPU — the core system problem (discussion brief)

> Scratch/discussion doc. Not committed. Grounds the "why is Anvil CPU-expensive"
> question in the actual code, with `file:line` refs you can click.
> Origin: beta-tester high-CPU report → reproduced 2026-05-30 (GH #2156).

---

## TL;DR

Anvil's value is *real-time governance* — check on every save. The way it does
that today is to **re-scan the entire repository, from a cold start, in a fresh
process, on every save**. That's cheap to *wait* for but expensive to *do*, and
the expense is invisible until you have a realistic concurrent (multi-agent)
workload — which is exactly the workload becoming normal.

Measured (prototype `benchmarks/prototypes/anvil-load-probe.py`, 800-file repo,
one save / 200 ms, whole process tree):

| Concurrent agents | default (`--action check`) | `--action none` (control) |
| ----------------- | -------------------------- | ------------------------- |
| 1                 | **6.96 cores**             | —                         |
| 2                 | 12.61 cores                | —                         |
| 4                 | 14.09 cores (≈ saturated)  | **0.03 cores**            |

One agent eats ~7 of 16 cores; ~2 agents saturate the box. With the action
disabled it's ~0%. **The per-save full-repo scan is ~100% of the cost.**

---

## 1. Primer: where CPU load actually comes from

- A CPU is a fixed set of **cores** (workers). Each does one thing at a time.
  "Load" = how many cores are busy. Ask for more simultaneous work than you have
  cores and work queues — everything (editor, compiler, agent) slows down.
- A **daemon/service** is a program that stays running and *waits* for events,
  then reacts. **Waiting is ~free.** Cost is entirely in the *reaction*.
- So: **load = (cost to react to one event) × (event rate).** Both halves matter.

The problem is not that Anvil is *running*. It's what it *does per save*, times
how often saves happen, times how many agents are saving at once.

---

## 2. Anvil's three long-lived processes

| Process | Entry | What it is |
| --- | --- | --- |
| `anvil watch` | `crates/anvil-cli/src/commands/watch.rs` | save-time file watcher (foreground) |
| MCP server | `crates/anvil-cli/src/commands/mcp.rs:77` (`run_serve`) | per-editor write-gate (`anvil_validate_write` / `anvil_apply_patch`, see `mcp.rs:17`) |
| intercept daemon | `crates/anvil-cli/src/commands/intercept.rs:4` (`start --foreground`) | mid-edit buffer validation over a socket |

They are **independent OS processes** and can all run at once. Nothing
coordinates their *combined* CPU footprint — each one individually decides how
much of the machine to use (see §5).

---

## 3. Idle is free — proof in the code

The kernel watch loop is a **blocking wait**, not a busy-loop:

- `crates/anvil-kernel/src/watcher/mod.rs:218` — `raw_rx.recv_timeout(tick_interval)`
- `:37` `tick_interval = 20ms`, `:35` `debounce_window = 50ms`
- On timeout it just checks the debouncer (`:264 debouncer.tick()`) and goes back
  to waiting.

So a watcher with nothing happening costs ~nothing. This is why the `--action
none` control above is 0.03 cores. **The daemon is not the problem; the reaction
is.**

---

## 4. The reaction: a cold, whole-repo scan per save

This is the heart of it. On a save, bare `anvil watch` runs an **action**, and
the default action is `check`:

- `crates/anvil-cli/src/commands/watch.rs:377` — `resolve_action`: `None | Some("check") => Ok(Some("check"))`
- confirmed by test `:1328` `resolve_action_defaults_to_check_when_absent`

That action is dispatched by **spawning a brand-new `anvil` process**:

- `crates/anvil-cli/src/commands/watch.rs:433` — `build_action_command`
- `:441` — `std::process::Command::new(exe)`
- `:455` — `cmd.arg("--all")`

And `--all` means **the whole repository**, not the changed file. The code says
so, and says the scoped version isn't built yet:

> `watch.rs:445-452` — *"Use `--all` rather than `--changed` … Scoping to exactly
> the watcher's changed paths … needs the kernel to surface changed paths to the
> dispatcher; tracked as [issue]."*

So every save = spawn a fresh process that **walks the filesystem and re-scans
every file** (secret scan, anti-pattern scan, policy/architecture eval). The
spawned process starts **cold** — it does not reuse the warm state the watch
daemon already has.

Two distinct costs hide here:

1. **Graph (re)construction** — building the symbol graph from scratch. The
   initial build is "embarrassingly parallel" and fans out across cores:
   `crates/anvil-kernel/src/watch.rs:214` ("parse all files in parallel"),
   `:228` `par_iter`. This is what makes a single save grab ~7 cores.
2. **Evaluation/scan** — secret + anti-pattern + policy passes over the file set.

The warm daemon path *is* incremental — it updates only the changed file:
`crates/anvil-kernel/src/watch.rs:10` imports `update_file`, `re_resolve_imports`,
`annotate_trust`; the per-event handler updates one file's symbols and
re-resolves. **But the spawned `check --all` subprocess throws that away and
rebuilds cold.** That gap is the single biggest lever.

(Aside: watch also does not honour `.gitignore` — it uses a hardcoded denylist,
`crates/anvil-kernel/src/watch.rs:141` `standard_filters(false)` +
`crates/anvil-kernel/src/watcher/filter.rs:16` `IGNORE_DIRS`. A build tool writing
to a dir not on that list — `.vite`, `out`, `.cache`, custom `gen/` — generates a
file-event storm that fires the check repeatedly. Second-order, but real.)

---

## 5. The concurrency cliff — rayon caps *per process*, not globally

Anvil parallel work uses a shared global **rayon** thread pool, deliberately
capped at half the cores so it leaves room for the editor:

- `crates/anvil-rayon-init/src/lib.rs:74` — `(available_cores / 2).max(1)`
- `:96-98` — `build_global().num_threads(threads)` with `num_cpus::get()`

The catch: that cap is **per process**. Each spawned `check` (and the MCP and
intercept daemons) is a *separate process* that initialises *its own* N/2 pool.
Two concurrent checks → 2 × (N/2) = N threads; three → 1.5N threads, all fighting
over N cores. There is **no machine-wide budget** for "how much CPU all of Anvil
may use." That's why the table in the TL;DR climbs 7 → 13 cores and then slams
into the ceiling: independent half-core pools stacking up.

(The per-save children are reaped — `watch.rs:623/733/777` `child.wait()` — which
is why the prototype can attribute their CPU via the parent's `cutime/cstime`.)

---

## 6. Why the current bench saw none of this

`crates/anvil-bench/src/watch_resource_budget.rs`:

- `:124` `watch_command_args` = `["--json","--no-tui","watch","--all","--debounce=100"]`
- `:135` `measure_process` just `sleep`s (`:139`, `:147`) and samples — it
  **writes no files**, so no save event fires and the `check` subprocess **never
  spawns**. It measures an idle watcher.
- It samples **only the parent pid** — `:161 read_process_cpu_ticks(pid)` — so
  even with churn it would miss the `check` children where the cost lives.
- 100 static files, ~3 s window. No scale, no sustained churn, no concurrency.

Three independent reasons it reports ~0%. The number is real — for a workload
nobody runs.

---

## 7. The core problem, stated

> Anvil wants to be **real-time** (every save), **cheap** (invisible CPU),
> **thorough** (full scan), and **safe under many concurrent agents** — and today
> it cannot be all four, because it does **whole-repo** work in **cold,
> uncoordinated** processes on **every** event.

Three levers (this is what a real rig must measure us against):

1. **Incrementality** — do work proportional to *what changed*, not the whole
   repo. Closing the warm-daemon-vs-cold-`check --all` gap (`watch.rs:455`) is the
   biggest single win. *Note:* a warm/cached **graph** removes the
   *construction* cost (§4.1); the *evaluation/scan* cost (§4.2) is separate and
   must also be scoped to the delta, or amplification persists.
2. **Warm & shared, not cold & many** — one long-lived service that holds state
   and answers queries, instead of spawning fresh full-scan processes. This also
   creates a single place to enforce a budget.
3. **A global budget with backpressure** — a machine-wide ceiling on Anvil's
   total CPU (vs the per-process N/2 cap in §5), so that when N agents pile on the
   system *coalesces / queues / sheds* work instead of each component grabbing
   cores blindly. Governance that steals the CPU the developer/agent needs won't
   get used — that's the existential version of the problem.

The rig's job: spawn realistic agents, drive the concurrent workload, find the
tipping point before a tester does, and prove a fix actually moved it.

---

## 8. How GV2 (the five graphs) relates to cold start

GV2 (`plans/archive/modules/graph-v2-foundation.aps.md`, Draft) is a deliberately
**"multiple joined graphs, not one mega-graph"** design (`:32`). It's the natural
home for the *warm substrate* that would kill cold-start construction (§4.1) —
but only graphs 1–2 (and partly 3) touch the per-save hot path; 4–5 are
attribution/provenance and do not reduce save-time CPU.

| # | Graph | Owns | Item | Per-save hot path? |
| - | ----- | ---- | ---- | ------------------ |
| 1 | Semantic code | symbols, imports, calls, refs, exports, spans, language metadata | GV2-010 (`:188`) | **Yes** — warm structural model the check reads |
| 2 | Dependency / impact + hot indexes | boundary membership, symbol ownership, known-edge existence, arch index checks | GV2-011 (`:206`) | **Yes — the hot-path graph** |
| 3 | Trust / policy | trust levels, side-effect surfaces, data classes, invariant guards, policy evidence | GV2-012 (`:224`) | **Partly** — consulted at check time |
| 4 | Control / session | hosts, drivers, sessions, leases, fences, worktrees, attribution | GV2-013 (`:243`) | Mostly no — "who did this" |
| 5 | Plan / provenance | APS items, commits, Edda provenance, graph deltas, trust posture | GV2-014 (`:262`) | No — explainability; *not a runtime prerequisite* |

The cold-start lever isn't the *count* of graphs — it's two substrate items
underneath them:

- **GV2-011's explicit hot/cold split** (`:209-213`): *"the warmed indexes the
  daemon may read on the hot path … boundary membership, symbol ownership,
  known-edge existence … exposed as bounded reads; transitive impact traversal
  remains explicitly non-hot-path."* This is the cold-start fix written down —
  bounded reads against a warm graph instead of a cold parallel rebuild. Its
  validation is already *"Criterion benchmark demonstrates the hot reads meet
  ADR-031 component budgets"* (`:214`) — a hot-read perf bench that dovetails with
  RLB.
- **GV2-021 persistence/snapshot** (`:301`): a snapshot *"derivable from source
  and safe to discard/rebuild"* (`:46`) that need not be rebuilt every time —
  warm start across process restarts, with a multi-process reader stance.

**The catch (same shape as §7).** GV2 builds the warm substrate; it does **not**
by itself rewire the per-save `check --all` subprocess (`watch.rs:455`) to
*consume* the hot reads. GV2's framing is "control and provenance primitive
first" (`:26`), not "make `anvil watch` cheap" — so there's a real risk it lands
the warm graphs while the watch path still cold-spawns, leaving the CPU win on
the table.

**Verdict:** graphs 1–2 + the GV2-011 hot-read split + the GV2-021 snapshot are
the right foundation and remove the dominant cold-start *construction* cost — but
the win only materialises when the per-save path is rewired to query them instead
of cold-rebuilding (the connective tissue is RLB-007 + a GV2-aware hot-read
bench). Graphs are **necessary, not sufficient**: evaluation-scoping (§4.2) and a
global CPU budget (§5) are still separate levers.

---

## Appendix — file reference index

| Concern | Location |
| --- | --- |
| default action = check | `crates/anvil-cli/src/commands/watch.rs:377`, test `:1328` |
| per-save subprocess spawn | `crates/anvil-cli/src/commands/watch.rs:433`, `:441`, `:455` |
| changed-path scoping not wired | `crates/anvil-cli/src/commands/watch.rs:445-452` |
| child reaping | `crates/anvil-cli/src/commands/watch.rs:623`, `:733`, `:777` |
| rayon N/2 cap (per process) | `crates/anvil-rayon-init/src/lib.rs:74`, `:96` |
| idle blocking loop | `crates/anvil-kernel/src/watcher/mod.rs:218`, `:35`, `:37` |
| incremental daemon update | `crates/anvil-kernel/src/watch.rs:10` |
| cold parallel full build | `crates/anvil-kernel/src/watch.rs:214`, `:228` |
| gitignore not applied / denylist | `crates/anvil-kernel/src/watch.rs:141`, `crates/anvil-kernel/src/watcher/filter.rs:16` |
| budget bench blind spots | `crates/anvil-bench/src/watch_resource_budget.rs:124`, `:135`, `:161` |
| MCP server | `crates/anvil-cli/src/commands/mcp.rs:77`, `:17` |
| intercept daemon | `crates/anvil-cli/src/commands/intercept.rs:4` |
| GV2 five-graph taxonomy | `plans/archive/modules/graph-v2-foundation.aps.md:32`, `:137` |
| GV2 hot/cold read split | `plans/archive/modules/graph-v2-foundation.aps.md:206` (GV2-011), `:209-214` |
| GV2 persistence/snapshot | `plans/archive/modules/graph-v2-foundation.aps.md:301` (GV2-021), `:46` |
| GV2 "provenance-first" framing | `plans/archive/modules/graph-v2-foundation.aps.md:26` |
| plan / tracking | `plans/modules/resource-load-benchmarking.aps.md`, GH #2156 |
