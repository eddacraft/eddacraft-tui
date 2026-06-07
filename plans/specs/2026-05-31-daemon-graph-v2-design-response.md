# Anvil Daemon + Graph V2 — design response

Companion to the save-time CPU field report (GH
[#2156](https://github.com/eddacraft/anvil-001/issues/2156)),
[ADR-061](../decisions/061-save-time-daemon-delta-validation.md), and the
[resource-load-benchmarking module](../modules/resource-load-benchmarking.aps.md)
— all grounded in the same load-probe data. Working doc written to be handed to
other engineers. Every architectural claim is grounded in a `path:line`
reference you can open.

This responds to the _Anvil Daemon and Graph V2 Acceleration Brief_. It accepts
the brief's product contract ("save-time Anvil is delta governance; full-repo
assurance is separate") and answers the nine design questions. It also pushes
back on one load-bearing assumption, because doing so makes the plan smaller,
cheaper, and lower-risk.

---

## 0. The one thing that changes the whole plan

**The brief reads as "build a daemon." We do not need to build a daemon — we
already have one, and it already does buffer validation.** The work is to route
the save-time path through it and give it warm state, not to stand up new
runtime machinery.

What `anvil intercept` is _today_ (not aspirationally):

| Capability the brief asks for                    | Already exists? | Evidence                                                                                                                                                                 |
| ------------------------------------------------ | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| One daemon process per workspace                 | **Yes**         | `anvil intercept start --foreground` → `crates/anvil-intercept/src/lib.rs:915` (`run_foreground`)                                                                        |
| Permissioned local transport                     | **Yes**         | Unix socket `…/anvil/intercept.sock` (0700 dir / 0600 sock, uid-checked) `crates/anvil-intercept/src/ipc.rs:273`, `:685`; Windows named pipe `:430`                      |
| Versioned, extensible wire protocol              | **Yes**         | JSON-RPC 2.0 + serde-tagged NDJSON envelope, `crates/anvil-intercept-proto/src/protocol.rs:62`, `:246`                                                                   |
| Async serve loop with admission control          | **Yes**         | per-connection tasks in a `JoinSet`, semaphore-gated `max_concurrent_connections` `crates/anvil-intercept/src/ipc.rs:780`, `:831`                                        |
| Warm in-process state across requests            | **Yes**         | `DaemonState` holds `SessionRegistry`, `FenceStore`, `Fanout` `crates/anvil-intercept/src/lib.rs:161`                                                                    |
| **Save-time validation method**                  | **Partly**      | `anvil/scan_buffer` already validates an in-flight _buffer_ `protocol.rs:83`, dispatch `ipc.rs:2208`; this is mid-edit, not changed-path-on-disk                         |
| Warm-daemon validation latency bench             | **Yes**         | `ipc_roundtrip` measures `validation.service` (in-proc) + `validation.roundtrip` (over socket), `daemonState=warm` `crates/anvil-intercept/benches/ipc_roundtrip.rs`     |
| In-process, scoped check library (no subprocess) | **Yes**         | `run_antipattern_check(files, config, workspace_root)` takes an arbitrary file list, returns diagnostics synchronously `crates/anvil-checks/src/antipattern/check.rs:95` |

What is genuinely missing (this is the actual project):

1. **`anvil watch` doesn't talk to the daemon.** On every debounced save it
   builds and spawns a cold `anvil check` child
   (`crates/anvil-cli/src/commands/watch.rs:433` `build_action_command` → `:441`
   `Command::new`). RLB-007 has since scoped that child to the **changed paths**
   (`scoped_check`, `:461`; `--all` only when the path set is empty, `:462`),
   which removed most of the per-save cost the CPU brief measured — but the
   child is still **cold-spawned with no warm graph state**, and the daemon is
   never consulted. The remaining win (no cold rebuild at all) needs the daemon,
   not another `watch.rs` tweak.
2. **No warm graph state in the daemon.** `SessionRegistry` is a session-id map,
   not a repo model. There is no cached `SymbolGraph`/`DependencyGraph` in the
   daemon — the kernel's incremental graph lives in the _watch_ process's memory
   and is thrown away; `check --all` rebuilds cold each time. (`RuleSetCache`
   exists but is explicitly _not wired_ — `lib.rs:204` notes it waits on
   MLP2-014.)
3. **MCP only reaches the daemon on Unix, and only for mid-edit buffers.**
   `anvil mcp serve --stdio` (`crates/anvil-cli/src/commands/mcp.rs:185`)
   already routes `anvil_validate_write` through `LocalDaemonValidationClient`,
   which attempts the daemon's `scan_buffer` over the Unix socket and only
   demotes to the in-process `anvil-checks`/`EnforcementPipeline` when the
   daemon is `Unavailable` (`crates/anvil-cli/src/mcp/validation.rs:206`,
   `:545`). Two gaps remain: it has no changed-paths (`validate_paths`) call,
   and on non-Unix it returns `Unavailable` and always runs embedded
   (`validation.rs:222`). So a repo running watch and MCP can still end up with
   two warm models — but the MCP→daemon edge already exists on Unix; it needs
   **extending, not building** (reinforcing §0's thesis).
4. **Rayon is capped per-process, not per-host.** Each Anvil process caps its
   pool at `(cores/2).max(1)` (`crates/anvil-rayon-init/src/lib.rs:74`), so N
   processes oversubscribe the box. One daemon = one pool to size.

So the corrected framing:

> Anvil already has a warm, permissioned, concurrent validation daemon that
> validates buffers. The save-time CPU fire was that `watch` cold-spawned a
> whole-repo scan per save; RLB-007 scoped that to changed paths, but `watch`
> still refuses to use the daemon and re-spawns a cold child with no warm graph.
> The project is **(a) stop the cold spawn, (b) give the daemon a changed-paths
> method backed by warm graph state, (c) make watch/MCP/intercept thin clients
> of it.**

This is the difference between "build a runtime" and "wire three things into a
runtime we shipped." It also means the **this-week CPU win needs neither the
daemon nor Graph V2** (see Q8).

---

## 1. Minimal daemon API for save-time validation (brief Q1)

Do **not** invent a new protocol or socket. Add methods next to the existing
`anvil/scan_buffer` on the same JSON-RPC dispatch (`ipc.rs:1949`
`handle_jsonrpc_request`), reusing the same diagnostic envelope `scan_buffer`
already returns (`crates/anvil-intercept/src/midedit.rs`). Same envelope for
every caller is the brief's "deterministic diagnostic envelope regardless of
caller" — for free.

Three new methods, smallest useful surface:

```
anvil/validate_paths        // the normal watch path
  req:  { workspace_root, paths: [ { path, content_hash?, mtime? } ], mode? }
  res:  { diagnostics: <same envelope as scan_buffer>,
          workspace_assurance: { state, reason? } }   // may flag "needs full scan"

anvil/workspace_status       // for CLI/TUI/JSON surfacing  (brief Q5)
  req:  { workspace_root }
  res:  { state: clean | stale | pending | running, last_full_scan?, graph_version? }

anvil/request_full_scan      // explicit / background assurance  (brief Phase 5)
  req:  { workspace_root, priority }
  res:  { job_id, state }
```

`validate_buffer` already exists as `anvil/scan_buffer` — leave it. That's the
editor/intercept/MCP mid-edit path the brief lists; we just add the
changed-paths sibling for watch and the workspace-assurance pair.

Implementation cost: three `IpcCommand`/JSON-RPC variants (`protocol.rs:62`),
three handler arms (`ipc.rs:1949`), one new field on `DaemonState` (the
per-workspace assurance state, §5). No transport, framing, or concurrency rework
— the brief's "build the smallest useful daemon contract" is literally three
methods on an existing dispatcher.

---

## 2. Graph V2 state for the first hot path (brief Q2)

`validate_paths` for a single changed file does **not** need the whole-repo
graph. It needs:

| Need                                                | Graph V2 item                                                                                | Hot-path read                             |
| --------------------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------- |
| Re-parse the changed file's symbols/imports/exports | GV2-010 semantic code graph (`crates/anvil-kernel/src/parser/extract.rs`)                    | per-file extract — already incremental    |
| "Do this file's new imports cross a boundary?"      | GV2-011 dependency/impact + **hot indexes** (`graph/dependency.rs`, `graph/symbol_graph.rs`) | boundary-membership lookup                |
| "Does this introduce a known-bad edge?"             | GV2-011 known-edge existence                                                                 | set lookup                                |
| "Did trust posture change?"                         | GV2-012 trust/policy lookup (`graph/trust.rs`)                                               | bounded read, no interprocedural analysis |

Everything else GV2 defines — plan/provenance (GV2-014), transitive impact,
explanation, context projection for agents — is **explicitly off the hot path**
and not in the first slice. This isn't me trimming; it's GV2's own Decision 3,
"hot indexes over hot traversal," and its constraint that "expensive traversal …
must stay off the daemon hot path" (`plans/modules/graph-v2-foundation.aps.md`
Decisions / Constraints).

So the first useful Graph V2 slice = **GV2-010 (per-file semantic extract) +
GV2-011 (warm dependency/boundary/known-edge indexes) + GV2-020 (a registry to
hold them in the daemon) + GV2-022 (the bounded hot-read API the daemon
calls).** GV2-021 (persistence/snapshot ADR) is the lever that lets the daemon
_warm-start_ from a snapshot instead of cold-rebuilding on restart — important,
but it does not block the first slice (rebuild-on-start is acceptable for v1).

The honest gap: the kernel already maintains these structures incrementally —
but **in the watch process, discarded each run.** The GV2 slice is mostly
_relocating_ that warm model into the daemon and exposing bounded reads, not
inventing a graph from scratch. That's why GV2's first slice and this daemon
work are the same work viewed from two modules.

---

## 3. Reuse vs bypass (brief Q3)

**Reuse (it already does the right thing):**

- `run_antipattern_check(files, config, workspace_root)` — takes a _scoped file
  list_, runs in-process, returns diagnostics. The daemon calls this with the
  changed paths. (`crates/anvil-checks/src/antipattern/check.rs:95`) This is the
  single most important reuse: the "scan a subset of files" capability already
  exists; `check --all` just never uses it on the hot path.
- The `scan_buffer` diagnostic envelope + JSON-RPC framing — reuse verbatim so
  all surfaces emit identical output (`midedit.rs`, `protocol.rs:83`).
- The kernel's incremental update pipeline + watcher debounce/coalesce
  (`crates/anvil-kernel/src/watcher/mod.rs:35` debounce, `:37` tick) — keep as
  the warm-graph maintainer, hosted in the daemon.
- The watcher's `notify` file-event source — keep; only its _reaction_ changes.

**Bypass / retire from the hot path:**

- `build_action_command` → `Command::new(anvil)` → `--all`
  (`watch.rs:433`/`:441`/`:455`). This child spawn is the thing that dies on the
  save path. The cold whole-repo rayon scan it triggers
  (`crates/anvil-kernel/src/watch.rs:214`, `:228`) only runs under explicit /
  background full-scan after this.
- The default action resolution at `watch.rs:377` flips: default save-time
  action becomes "validate changed paths (via daemon, or scoped fallback)," not
  "spawn `check --all`."

---

## 4. Preventing watch / MCP / intercept from duplicating heavy work (brief Q4)

One daemon per workspace owns the warm model and the work budget; the three
surfaces become thin clients of `validate_paths` / `scan_buffer`:

- **watch** → `validate_paths(changed_paths)` instead of spawning a child.
- **MCP** `anvil_validate_write` already attempts the daemon's `scan_buffer` on
  Unix via `LocalDaemonValidationClient`
  (`crates/anvil-cli/src/mcp/validation.rs:206`), demoting to in-process
  `anvil-checks` only when the daemon is `Unavailable`. The remaining work is to
  extend it to `validate_paths` and cover non-Unix, so a repo with watch + MCP
  keeps **one** warm graph and never double-scans. This is consistent with GV2's
  rule "MCP/agent query surfaces are projections, not the control plane" — MCP
  becomes a client of the authority, it does not _become_ the authority.
- **intercept** is the daemon; it gains the method.

The daemon's existing admission control (semaphore over connections,
`ipc.rs:831`) plus a work budget (§6) is the _single_ place coalescing and
backpressure live. N clients cannot create N heavy scans.

---

## 5. Representing full-scan state: clean / stale / pending / running (brief Q5)

New per-workspace value on `DaemonState` (`lib.rs:161`, beside `SessionRegistry`
/ `FenceStore`):

```
WorkspaceAssurance =
  Clean   { scanned_at, graph_version }   // last full scan passed, nothing changed since
  Stale   { reason }                      // delta arrived that warm indexes couldn't fully resolve,
                                          //   or config changed, or path-set overflowed delta budget
  Pending                                 // full scan queued
  Running { progress }                    // background full scan in flight
```

Transitions:

- `validate_paths` may push `Clean → Stale` and enqueue `→ Pending` when a
  change exceeds what a bounded delta can certify (e.g. a
  `tsconfig`/boundary-config edit, or > N changed paths).
- the background scheduler drives `Pending → Running → Clean`.

Surfaced through `anvil/workspace_status` and rendered by `anvil status` / TUI /
`--json`. This is exactly the brief's "save-time check vs workspace assurance"
UX split, made into daemon state rather than per-process guesswork.

---

## 6. Concurrent agent save storms (brief Q6)

The original melt: N agents × per-save cold `check --all` = N cold whole-repo
scans (load-probe: 1 agent ≈ 7 cores, 2 agents saturate a 16-core box). With one
daemon:

- N agents' saves become N cheap `validate_paths` calls against **one** shared
  warm model.
- **Daemon work budget** (blunt v1, the brief explicitly accepts blunt): _one
  heavy full-scan per workspace at a time_ + a small bounded pool for delta
  validations. Queued `validate_paths` for the same path coalesce to the latest
  state — superseded intermediate requests are dropped ("prefer latest-state
  validation over redundant intermediate-state"). This is RLB-002 (watch budget)
  made real.
- **Rayon: per-process → per-host.** Today every process caps at `(cores/2)` and
  they oversubscribe (`anvil-rayon-init/src/lib.rs:74`). One daemon = one pool,
  sized and **configurable** (the brief asks). This removes the cross-process
  cliff the CPU brief documents.

---

## 7. Daemon-unavailable fallback (brief Q7)

Every client degrades safely — daemon-down is a _degraded mode_, never a hard
block. This is the existing planless-first / warnings-over-blocks / exit-0
posture (`.claude/rules/architecture.md`):

- **watch**: socket absent / connect fails → fall back to spawning `check` on
  the **changed paths only** (never `--all`), or skip-with-warning, behind a
  flag. Even the fallback is bounded.
- **MCP**: falls back to its _current_ in-process `anvil-checks` call — i.e.
  MCP's fallback is literally today's behaviour, so re-pointing it at the daemon
  is strictly additive and reversible.
- Optional later nicety: watch auto-starts `intercept` if absent. Not required
  for v1; the scoped-subprocess fallback is the safe default.

---

## 8. Smallest migration step that cuts CPU _this week_ (brief Q8)

Two options, in order of size. **A is already shipped; B is next** — A needed
neither the daemon nor Graph V2, so it shipped independently and de-risks
everything after it.

**Option A — scope the per-save check. = RLB-007 — already shipped (Merged
2026-05-31 via PR [#2184](https://github.com/eddacraft/anvil-001/pull/2184)).**
`build_action_command` now passes the debounced **changed paths** to
`anvil check <paths>` (`scoped_check`, `watch.rs:461`) and only uses `--all`
when the changed-path set is empty (`watch.rs:462`). This turned a whole-repo
cold scan per save into a scan proportional to what changed — the RLB load-probe
measured the single-file-save number collapse from ~6.55 cores to ~0.08
(`plans/modules/resource-load-benchmarking.aps.md`). A few lines + tests in one
crate; no protocol, no daemon, no GV2. **This was the bleeding-stopper — it is
now the baseline Option B must beat.**

**Option B — route watch through the daemon (next). = RLB-001..005 + the new
`validate_paths` method.** Add `validate_paths` to the daemon (calls
`run_antipattern_check(changed_paths, …)` against warm state), and make watch a
client. This is the first _real_ step toward the target architecture and where
the warm-graph win (no cold rebuild at all) actually lands.

Both are gated by the benchmark in §9 so we can _prove_ the CPU drop rather than
assert it.

---

## 9. Acceptance tests / benchmarks that prove the real fix (brief Q9)

The existing `watch_resource_budget` bench reports ~0% CPU for three structural
reasons (all confirmed earlier): it generates no file events (so `check` never
fires), measures the parent pid only (misses reaped children), and uses 100
static files over 3s. It measures the **idle path** — exactly the thing that
looks cheap while save-time melts the box.

The prototype `benchmarks/prototypes/anvil-load-probe.py` already does it
correctly and reproduced the field report:

- process-tree CPU = `utime+stime+cutime+cstime` (captures reaped `check`
  children),
- sustained file churn (one writer per agent),
- an **agent ramp** (1→N) to find the tipping point,
- an `--action none` **control** proving the cost is the reaction, not the
  watcher (`none` = 0.03 cores vs `check` = 6.96 at 1 agent).

Promote it to a maintained harness (RLB-001) and gate on it (RLB-008):

- **Regression gate:** after Option A/B, single-file-change save-time CPU must
  drop from ~7 cores toward < ~1 core; the 4-agent ramp must not saturate.
- **Process-tree, churn, concurrency** are mandatory bench properties — the gate
  must catch "idle looks cheap, save-time melts."
- **Warm-read latency budget:** extend the existing `ipc_roundtrip` criterion
  bench with a `validate_paths` case, tied to ADR-031 component budgets — this
  is also GV2-011 / GV2-022's own validation criterion, so the two modules share
  one gate.
- **Behavioural assertions:** normal save spawns **no** `anvil check --all`
  child (assert against the process tree); changed-path validation is used;
  repeated saves coalesce; concurrent storm stays bounded.

---

## 10. How this maps to tracked work (no new module — one ADR)

This design is **not** a new APS module. It's a spine across modules that
already exist; the value is sequencing them against the CPU forcing-function:

| Brief phase                            | Lands as                                                 | Module                              |
| -------------------------------------- | -------------------------------------------------------- | ----------------------------------- |
| Phase 1 — stop the bleeding            | scope per-save check (Q8-A) — **shipped, PR #2184**      | **RLB-007**                         |
| Phase 2 — daemon validation API        | `validate_paths` + watch client (Q1, Q8-B)               | INTD / intercept + **RLB-001..005** |
| Phase 3 — warm Graph V2 slice          | per-file extract + hot indexes + registry + hot-read API | **GV2-010 / 011 / 020 / 022**       |
| Phase 4 — global CPU/backpressure      | daemon work budget + per-host rayon                      | **RLB-002**, `anvil-rayon-init`     |
| Phase 5 — full assurance as background | `workspace_status` + `request_full_scan` + scheduler     | INTD + **RLB-008** SLOs             |
| Persistence / warm-start               | snapshot ADR                                             | **GV2-021**                         |

The genuinely new artifact is **one ADR**: _"save-time governance is
daemon-mediated delta validation; full-repo scan is explicit/background."_ Per
`.claude/rules/architecture.md` an architectural decision of this weight should
go through `docs/guides/adr-process.md` and land in
`plans/decisions/DECISION-LOG.md`. That ADR is the thing to write first because
it locks the product contract the brief states — everything else is sequencing
existing items behind it.

---

## 11. Where I'd push back on the brief

1. **"Build the daemon" overstates the work.** The daemon exists and validates
   buffers; framing this as greenfield risks re-implementing transport,
   protocol, and concurrency we already ship and trust. Frame it as _wiring + a
   changed-paths method + warm state._
2. **Don't couple the CPU win to the daemon or GV2.** Option A (RLB-007) was a
   few lines in `watch.rs` and delivered most of the relief; shipping it first
   (done, PR #2184) kept the architecture work off the critical path for
   stopping the fire.
3. **Keep the daemon optional, hard.** The biggest way to get this wrong is to
   make the daemon a _requirement_ and break planless-first. Fallbacks (Q7) are
   not a nicety — they're load-bearing for the product's "value without config"
   principle.
4. **Resist growing GV2 on the hot path.** The pressure during implementation
   will be to answer richer questions per save (transitive impact, provenance).
   GV2's own constraints forbid it; the bench (§9) is what keeps the hot path
   honest.
