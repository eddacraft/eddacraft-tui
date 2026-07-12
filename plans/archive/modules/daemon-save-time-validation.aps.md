# Daemon Save-time Validation

| ID  | Owner | Status      |
| --- | ----- | ----------- |
| DSV | Josh  | Complete |

**Last reviewed:** 2026-07-06 (Sub-phase C implementation merged: DSV-048
Merged via PR #3186, DSV-047 Merged via PR #3191, DSV-049 Merged on `main` with
status wire validation evidence, DSV-050 Merged via PR #3200, and DSV-051 Merged
via PR #3205. DSV now awaits release evidence before Released/Shipped and
Complete lifecycle advancement.)

2026-07-13: all Merged items confirmed in the v0.9.0-beta tag (record:
plans/releases/v0.9.0-beta.md) and advanced to Released/Shipped; module
ready to archive per the archive cascade.

2026-06-12: the shipped sub-phase A/A-W/A′ arc (incl. DSV-021 default-on
routing) confirmed in the v0.8.0-beta tag (record:
plans/releases/v0.8.0-beta.md); Merged items advanced to Released/Shipped.

2026-06-24: stale Sub-phase B blocker reconciled. ADR-069 (graph persistence
and snapshot strategy) is Accepted; its GV2-021 work item is Released/Shipped;
and DSV-030 (warm-start persistence) Merged 2026-06-17 via PR #2688. Sub-phase B
is no longer Blocked and now awaits the v0.9 release tag for Released/Shipped/
Complete lifecycle advancement.

2026-06-16: added **DSV-045 (full-scan executor, Ready)** — the loop that drives
`request_full_scan` `Pending → Running → Clean` and populates the warm graph
cache without a save, so a fresh session is not cold. Architecture decided by
[ADR-085](../../decisions/085-daemon-full-scan-executor.md) (planning council
`plan-898d9222`); merges before the GCTX-010 warm-up triggers. Module total
19 → 20.

2026-07-04: **DSV-046** shaped and promoted — planning-council direction validate
on the headless save-time driver; [ADR-101](../../decisions/101-headless-save-time-driver.md)
Accepted (operator); design
[`specs/2026-07-04-headless-save-time-driver-design.md`](../../specs/2026-07-04-headless-save-time-driver-design.md).
DSV-046 marked **Done** (design); splits into **DSV-047..051** (Ready, cut-line).
Module total 21 → 26. Closes the RELEASE-PLAN usefulness addendum implementation
gap alongside merged ACTMO-014..020.

## Purpose

Deliver Anvil's save-time validation as a daemon-mediated service across its
planned sub-phases, behind one frozen `validate_paths` wire. The intercept daemon
certifies a set of changed paths against a warm per-`WorktreeKey` graph cache and
returns a verdict-shaped envelope; `watch` and the MCP `anvil_validate_write` tool
become thin daemon clients with a scoped (never `--all`) fallback.

**Why:** this work has grown past a single work item. It spans ~6 crates, a new
crate extraction (ADR-064), three ADRs (061/063/064), and three sequenced
sub-phases. It was previously tracked only as the interim-backing item
[MLP2-067](../../modules/multilayer-protection-v2.aps.md) plus an execution plan, which
under-represented its scope in `plans/index.aps.md`. This module is the durable
home that makes progress trackable and keeps the sub-phases coherent.

**North star:** the wire is frozen once; the *backing* swaps underneath it across
sub-phases (interim `SymbolGraph` cache → GV2 resident warm indexes → warm-start
persistence) without consumers re-integrating.

## Sub-phases

- **Sub-phase A — interim-cache `validate_paths`.** Ship the frozen wire + watch
  client + MCP re-point, backed by an interim per-`WorktreeKey` `SymbolGraph`
  cache (rebuild-on-restart, no persistence). Authorised to start
  (GO-WITH-CONDITIONS — see the
  [holistic re-review verdict](../../reviews/2026-06-02-b-corrections-holistic-verdict.md)).
  Action plan:
  [`execution/2026-06-01-daemon-save-time-subphase-a.md`](../../execution/2026-06-01-daemon-save-time-subphase-a.md)
  (Tasks 0–17).
- **Sub-phase A-W — Windows + cross-platform parity.** Bring the Sub-phase A
  save-time surface (daemon verbs + `watch`/`status` clients) to the other
  short-term-supported targets. macOS already works (`cfg(unix)`); Windows was the
  gap (verbs unserved, clients were `cfg(not(unix))` stubs). DSV-010 Merged (the
  verbs are served on Windows behind the ADR-068/070 read-safety guard + peer-SID
  auth + owner-only config trust); DSV-011 Merged (Windows `watch`/`status`
  clients; closed 2/2 on green cross-matrix evidence, run 27102943706). Same
  frozen wire and interim backing as Sub-phase A — a *platform* axis, not a
  *backing* swap, so it is orthogonal to A′/B.
- **Sub-phase A′ — GV2 hot-read swap + default-on routing.** Replace the interim
  cache with the GV2 resident warm-index slice under the unchanged wire, then
  make daemon routing the safe default for `check` watches. **Done** — the
  hot-/non-hot-path boundary gate is closed by
  [ADR-063](../../decisions/063-gv2-hot-path-boundary.md) (Accepted 2026-06-01), and
  the swap is the **headline v0.8.0-beta payload** per
  [ADR-075](../../decisions/075-v080-graph-product-scope.md) (Accepted via council).
  GV2-027 merged the backing swap with verdict parity, GV2-025 supplied the
  ADR-031 Criterion gate, and DSV-021 flipped `ANVIL_WATCH_DAEMON` default-on
  with the rollout controls required by ADR-075.
- **Sub-phase B — warm-start persistence.** Add a default-off, per-uid,
  owner-only snapshot that restores graph indexes (never the verdict) on daemon
  restart, per the validation contract §9. **Done/Merged:** GV2-021 is
  Released/Shipped and DSV-030 Merged 2026-06-17 via PR #2688; the sub-phase now
  waits only for release-tag lifecycle advancement.
- **Sub-phase C — headless background driver.** Unattended filesystem observation
  for durable registered worktrees without a visible `anvil watch` terminal.
  **Design Done:** DSV-046 + [ADR-101](../../decisions/101-headless-save-time-driver.md)
  (2026-07-04). **Implementation Ready:** DSV-047..051 — daemon-supervised detached
  `--save-time-driver` sidecars consuming the ACTMO-014 membership hook. This is
  the `v0.9.0-beta` usefulness cut-line gate alongside merged ACTMO registration UX.

## In Scope

- The frozen verdict-shaped `validate_paths` / `workspace_status` /
  `request_full_scan` JSON-RPC wire and its forward-compatible envelope
- Daemon-side authorisation (net-new `validate_workspace_roots` wiring),
  openat2 read-safety, inode-based change classification, and default-deny
  invalidation taxonomy
- The bounded reverse-impact certifiability decision and the interim
  per-`WorktreeKey` `(SymbolGraph, DependencyGraph)` cache it reads
- The two cooperating rayon pools, per-workspace DoS caps, and the concurrency
  SLO bench + CI gate
- `watch` and MCP daemon clients, the scoped daemon-absent fallback, and the
  `anvil status` assurance surface
- Cross-platform parity for that save-time surface: the daemon + `watch`/`status`
  clients on **macOS** (shipped — `cfg(unix)`) and **Windows** (named pipe — the
  short-term gap tracked as DSV-010/011), not just Linux
- Opt-in workspace confinement mode and the `anvil workspace` CLI
- The cross-path diagnostic parity gate
- The GV2 hot-read swap (A′) and warm-start persistence (B) as sequenced
  successor sub-phases under the same wire

## Out of Scope

- The Graph v2 foundation substrate itself (schema, identity, hot indexes,
  persistence ADR) — owned by [GV2](graph-v2-foundation.aps.md); this module
  *consumes* it
- Running the structural policy invariants on the save-time hot path (ADR-061
  deliberately keeps them on whole-repo `anvil gate`)
- Cross-uid trust boundaries (the SO_PEERCRED same-uid boundary, and its Windows
  per-user named-pipe equivalent, are the only ones claimed)
- Windows GA *hardening* beyond same-user functional parity (Job Object
  containment, service-mode autostart, code-signing) — the parity itself is now in
  scope (DSV-010/011), this is the layer above it
- Replacing the embedded in-process fallback path

## Interfaces

**Depends on:**

- `anvil-graph-cache` (net-new, [ADR-064](../../decisions/064-intercept-graph-cache-crate-boundary.md))
  — `SymbolGraph`, `DependencyGraph`, incremental apply-delta, `certify`
- `anvil-intercept` / INTD — daemon transport, SO_PEERCRED handshake, IPC
  dispatch; [`intercept-daemon`](./intercept-daemon.aps.md) is
  archived Complete, so daemon integration debt lives here and in MLP2
- `anvil-intercept-proto` — the shared `DiagnosticEnvelope` (B3, landed) and the
  frozen method constants
- `anvil-checks` — `run_antipattern_check`; B7 adds a guarded-bytes + injected-pool
  entrypoint
- [GV2](graph-v2-foundation.aps.md) — GV2-010/011/020/022 (hot-read slice, A′) and
  GV2-021 (persistence ADR, B)
- [MLP2-067](../../modules/multilayer-protection-v2.aps.md) — the originating interim-backing
  item, now delivered here as Sub-phase A
- [RLB](../../modules/resource-load-benchmarking.aps.md) — RLB-002/-005/-008 resource model + SLO
- [DRVR](./surface-drivers.aps.md) — MCP `anvil_validate_write` re-point (archived Complete)
- ADR-061 (save-time daemon delta validation), ADR-063 (GV2 hot-path boundary),
  ADR-064 (graph-cache crate boundary), ADR-031 (latency rubric), ADR-035
  (notification envelope)

**Exposes:**

- The frozen `validate_paths` verdict wire + `check_families` scoping
- `watch` and MCP daemon clients with scoped fallback
- The `anvil status` / `--json` workspace-assurance surface and `anvil workspace`
  confinement CLI

## Constraints

- UK English spelling in all plan text and user-facing docs
- Warnings over blocks; deterministic same-input-same-output verdicts
- The hot path never re-reads files outside the openat2/`RESOLVE_BENEATH` guard,
  and the antipattern check runs on the interactive pool, not the global one (B7)
- `coverage: certified` attests **only** `check_families: ["antipattern"]` — never
  an unscoped structural-safety claim (B2)
- The daemon links no tree-sitter; the cache write-path receives already-parsed
  `FileSymbols` from the kernel feed (ADR-064 §4)
- The wire is frozen across sub-phases; only the backing swaps

## Prerequisites

- ADR-061, ADR-063, ADR-064 accepted (done 2026-06-01/-02)
- The B-corrections holistic re-review applied (done — verdict
  [`2026-06-02-b-corrections-holistic-verdict.md`](../../reviews/2026-06-02-b-corrections-holistic-verdict.md))
- For A′: the GV2 hot-/non-hot-path boundary gate agreed with INTD/DRVR owners
- For B: the GV2-021 persistence ADR accepted (**done**; ADR-069 Accepted,
  GV2-021 Released/Shipped, DSV-030 Merged)

## Ready Checklist

Sub-phase A is **Ready** (execution authorised, GO-WITH-CONDITIONS). A′ and B
have both cleared their GV2 gates; B's implementation (DSV-030) is Merged and
awaits release-tag lifecycle advancement.

- [x] Architecture decided and ADRs accepted (061/063/064)
- [x] Council review passed (do-not-start blockers resolved; holistic re-review GO-WITH-CONDITIONS)
- [x] Sub-phase A action plan exists with concrete validation commands per task
- [x] Crate-boundary predecessor identified and scoped (DSV-001 / Task 0)
- [x] (A′) GV2 hot-/non-hot-path boundary agreed with INTD and DRVR owners
- [x] (B) GV2-021 persistence ADR accepted — ADR-069 Accepted, GV2-021 Released/Shipped, DSV-030 Merged

## Work Items

### Sub-phase A — interim-cache `validate_paths`

#### DSV-001: Extract `eddacraft-anvil-graph-cache` (ADR-064 / B5)

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-03 via PR #2254
- **Intent:** Give the daemon a parser-free crate that owns the graph state and
  algorithms so the certify/cache work can compile without dragging tree-sitter
  into the resident daemon.
- **Expected Outcome:** `anvil-graph-cache` exists (`petgraph`-only — no
  tree-sitter/notify/walkdir/ignore/rayon); `anvil-kernel` re-exports it via the
  module alias `pub use anvil_graph_cache as graph;`; `anvil-intercept` depends on
  it; `certify` has a home there; the daemon links no tree-sitter.
- **Validation:** `cargo test -p eddacraft-anvil-graph-cache`;
  `cargo test -p eddacraft-anvil-kernel` (incl. `architecture_parity.rs`);
  workspace build proves `anvil-intercept` compiles against the crate; guard test
  `daemon_does_not_link_tree_sitter`.
- **Files:** `crates/anvil-graph-cache/`, `crates/anvil-kernel-types/src/graph.rs`,
  `crates/anvil-kernel/Cargo.toml`, `crates/anvil-intercept/Cargo.toml`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None
- **Source:** subphase-a Task 0; ADR-064; council blocker B5.

---

#### DSV-002: Freeze the `validate_paths` verdict wire

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-03 via PR #2252
- **Intent:** Pin the forward-compatible verdict-shaped wire (method constants,
  request/response types, `coverage`, `check_families`, `StaleReason`,
  `WorkspaceAssurance`) so all four surfaces integrate against a stable contract.
- **Expected Outcome:** The proto crate carries the frozen types; responses serialise
  `check_families: ["antipattern"]`; the shared `DiagnosticEnvelope` types the
  diagnostics field; unknown additive fields deserialise OK; `ALL_ANVIL_METHODS` is
  two-directionally pinned.
- **Validation:** `cargo test -p eddacraft-anvil-intercept-proto` (roundtrip,
  forward-compat, kebab wire strings, check-families, two-directional method pin).
- **Files:** `crates/anvil-intercept-proto/src/protocol.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None (B3 `DiagnosticEnvelope` already landed)
- **Source:** subphase-a Task 1; INTD `validate_paths` method; council B2/B3.

---

#### DSV-003: Daemon ingest spine — auth, read-safety, change classification, taxonomy

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-03 via PR #2264
- **Scope note (2026-06-03):** the ingest-spine *components* landed in PR #2264 —
  `path_safety` (Task 3, openat2 dirfd read-safety), `change_class` (Task 4,
  inode classification), `assurance` taxonomy half (Task 5), and a standalone,
  unit-tested `workspace_admission::AdmittedRoots` (Task 2's per-connection
  admitted-root set). Task 2's `ipc.rs` per-connection threading + the live
  `validate_workspace_roots`/`authorise` caller (B7) were intentionally deferred
  to **DSV-005**: `validate_paths` is the admitted-root set's only real consumer,
  so the wiring landed with that dispatch arm (DSV-005, PR #2282) rather than as
  inert plumbing here. With that wiring merged, the item is complete.
- **Intent:** Establish the load-bearing trust + ingest path the verdict depends on:
  net-new workspace-root authorisation, openat2 read-safety, inode-based change
  classification, and a default-deny invalidation taxonomy.
- **Expected Outcome:** A growable per-connection admitted-root set (canonical path
  paired with a once-opened `O_PATH` dirfd; no `/proc/<pid>/cwd`); `read_under` via
  `openat2(RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH)` for `path` and `renamed.from`;
  inode-flip-aware classification (atomic-save ⇒ ContentModify); every non-certifiable
  class maps to a `StaleReason` (unknown ⇒ stale, never clean).
- **Validation:** `cargo test -p eddacraft-anvil-intercept auth path_safety change_class assurance::taxonomy`.
- **Files:** `crates/anvil-intercept/src/{auth,path_safety,change_class,assurance}.rs`,
  `crates/anvil-intercept/src/ipc.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** DSV-002
- **Source:** subphase-a Tasks 2–5; council B7 (net-new auth + read-safety).

---

#### DSV-004: Certifiability + interim graph cache + `FileSymbols` feed

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-03 via PR #2273
- **Progress:** Task 6 (certifiability) delivered — `certify` + `export_surface_changed`
  in `anvil-graph-cache` (`certify.rs`), conservative export-surface default (B4),
  importer discovery via `dependents_of` only (B1), `errors`-guard + precondition doc
  from Council. Two crate-boundary deviations from the spec wording, both forced by
  the dep graph: the `Partial` reason is a graph-cache-local `CertifyStale` (not the
  wire `StaleReason`, which would add `anvil-intercept-proto` to ADR-064 §2's frozen
  set) and the change descriptor is a local `ChangeKind` (not `anvil-intercept`'s
  `CanonicalChange`, which would cycle) — the daemon maps both at the boundary
  (DSV-005). Task 7 (interim graph cache) delivered — `KernelGraphCache` in
  `anvil-intercept` (`kernel_cache.rs`): bounded-LRU + per-key generation guard +
  `invalidate` for the registry unregister hook, parse-free `apply_delta` consuming
  already-parsed `FileSymbols` (the `daemon_dep_boundary` guard still holds — no
  tree-sitter in the daemon), cold key → `CrossFileResolutionNeeded`, eviction →
  generation bump (→ `WarmStateEvicted` at the DSV-005 state machine), and the
  B1-named `reverse_index_consistent_after_delta` test. The `DependencyGraph` is
  re-derived from the in-place-mutated `SymbolGraph` each delta (interim Sub-phase A
  backing; the GV2 A′ hot-read swap replaces it with a resident incremental index).
  Remaining for DSV-005, not DSV-004: the kernel→daemon `FileSymbols` feed-**producer**
  wiring (`watcher.rs` `WatcherChangeBatch` carrying parsed symbols) — ADR-064's
  "Task 7/8 must nail" detail, which lands with the `validate_paths` orchestration
  that actually calls `apply_delta` with fed symbols. DSV-004's consuming contract
  (parse-free `apply_delta(FileSymbols)`) is complete.
- **Intent:** Decide certified-vs-stale via a bounded reverse-impact closure over a
  warm per-`WorktreeKey` `(SymbolGraph, DependencyGraph)` cache the daemon mutates
  from kernel-fed parsed symbols.
- **Expected Outcome:** `certify(sym, dep, change, delta, budget)` returns
  Certified/Partial with the conservative export-surface default; the cache holds the
  pair behind LRU + generation-guard, cold-key state is `Stale(CrossFileResolutionNeeded)`;
  `apply_delta` consumes already-parsed `FileSymbols` from the kernel feed (the daemon
  never parses); the reverse index matches a cold rebuild after multi-step deltas.
- **Validation:** `cargo test -p eddacraft-anvil-graph-cache certify`;
  `cargo test -p eddacraft-anvil-intercept kernel_cache`;
  `apply_delta_consumes_fed_file_symbols_not_a_daemon_parse`.
- **Files:** `crates/anvil-graph-cache/src/certify.rs`,
  `crates/anvil-intercept/src/kernel_cache.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** DSV-001, DSV-003 (and DSV-002 conceptually — the cache backs the frozen wire, though Task 7 is parse-free with no compile-time dep on the wire types)
- **Source:** subphase-a Tasks 6–7; MLP2-067 (interim backing); council B1/B4; ADR-064 §4 feed.

---

#### DSV-005: `validate_paths` orchestration + assurance lifecycle

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-03 via PR #2282
- **Progress (2026-06-03):** delivered across the DSV-005 PR set (#2276 B7
  predecessor, #2278 Task 9 state machine, #2279/#2280 Task 8, #2282 the
  capstone ipc wiring + envelopes + symbol feed) as three council-reviewed
  change-sets:
  1. **ipc wiring + per-connection admission.** `save_time::{SaveTimeState,
     SaveTimeConn}` route the three verbs through the `ipc.rs` special-method
     dispatch (mirroring `scan_buffer`), threaded cross-platform as
     `Option<&mut dyn SaveTimeDispatch>`. This lands DSV-003 Task 2's deferred
     `ipc.rs` admission threading: each verb authorises `workspace_root` against
     the per-connection `AdmittedRoots` before any read, and every byte is read
     through the held openat2 dirfd (B7 / security C2). Verdict keyed on the
     **canonical** root; per-key `Arc<Mutex<AssuranceMachine>>` so cross-worktree
     verdicts don't serialise. `request_full_scan` queues a scan (the executor is
     DSV-006); client hashes never trusted.
  2. **Task 9 notification envelopes.** `telemetry::envelope_for_assurance_transition`
     mirrors the fence envelope (class `FenceState`, priority high for
     →stale/→unavailable, `grouping.key=intercept:assurance:<root>`); each
     transition emits it + a tracing mirror carrying the machine fields, which
     stay **off** the wire `NotificationContext` (Cond A). Production `Fanout::route`
     delivery is the shared Phase E producer wire-up (as for fence transitions).
  3. **Kernel symbol feed (Task 7) as a dependency-inverted parse hook.** Modelled
     as the EIP **Content Enricher behind a Messaging Gateway** (see
     [ADR-067](../../decisions/067-daemon-symbol-feed-parse-hook.md)): the daemon
     defines the `SymbolParser` trait (no tree-sitter — ADR-064 holds), and
     `anvil-cli` injects a kernel-backed impl via `ForegroundOpts`. `validate_paths`
     hands the parser the **exact** guarded bytes it hashed (no second read → no
     B2 race), unblocking real `Certified` verdicts. The async watcher feed is
     reframed as a future advisory cache-warmer, never the verdict source.
- **Deferred (tracked):** `Fanout::route` subscriber delivery (Phase E);
  the interactive-pool **offload** of the synchronous parse + the `4 agents + 1
  scan` **SLO bench** (DSV-006 Task 16); the registry unregister-hook wiring for
  warm-state reclamation; an operator antipattern-config surface.
- **Expected Outcome:** Orchestration runs auth → classify → guarded-bytes read →
  apply-delta → certify → antipattern check (on guarded bytes + the interactive pool)
  → coalesce → assurance; the lifecycle emits ADR-035 notification envelopes (machine
  fields on the tracing mirror only); client-supplied hashes are never trusted for a
  verdict.
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib -- save_time validate_paths telemetry`;
  `cargo test -p eddacraft-anvil intercept_symbol_parser`;
  `cargo test -p eddacraft-anvil-intercept --test daemon_dep_boundary --test save_time_wired`.
- **Files:** `crates/anvil-intercept/src/{validate_paths,save_time,telemetry,ipc,lib}.rs`,
  `crates/anvil-cli/src/intercept_symbol_parser.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** DSV-004, DSV-006
- **Source:** subphase-a Tasks 8–9; council B6/B7/item 8; ADR-067.

---

#### DSV-006: Resource model — two-pool scheduler, DoS caps, SLO gate

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-03 via PR #2283
- **Intent:** Keep the interactive verdict path responsive under concurrent agents +
  background scans, and gate latency in CI.
- **Progress:** 10a (spine) delivered via PR #2253 — `crates/anvil-intercept/src/workspace_pool.rs`
  builds the two cooperating rayon pools (small interactive + background) from one
  per-host budget and adds the per-`WorktreeKey` in-flight admission token (the Task 8
  predecessor). 10b delivered via PR #2272 — the chunked-yield background-scan loop
  (`run_chunked_scan` + `ScanCancel`/`ScanOutcome`): the background scan checks a cancel
  flag at every chunk boundary so it hands cores back to interactive work within one
  chunk, with `processed` doubling as a resume offset. **Task 11 (DoS caps) delivered**
  — `workspace_pool::DosCaps` (parse-size + walk-depth) plus the symlink-skipping
  `walk_capped` primitive the background-scan executor consumes; `validate_paths`
  enforces the parse-size cap per file, skipping an oversized file before any
  parse/scan/hash and emitting a `Warning` coverage diagnostic (`intercept-parse-size-cap`,
  category `Other`) while marking the path `Partial`. **Task 16 (SLO bench + CI gate)
  delivered** — `benches/ipc_roundtrip.rs` gains the in-process `validation.service`
  warm `validate_paths` p95 case, the `4 agents + 1 background scan` ramp (each agent on
  its own `WorktreeKey` → measures interactive-pool contention), the RLB-008 >80 ms
  pre-service queue-wait WARN, and the RLB-002 daemon-absent scoped-fallback comparison;
  the bench exits non-zero when interactive p95 breaches the ADR-031 80 ms save-time
  budget. The gate runs on the **per-PR/push `resource-budgets` job** of
  `resource-budget.yml` (ADR-061 §9 "not optional"): the bench is in-process (no
  daemon/inotify, unlike the dispatch-only load-ramp job) and the measured warm p95
  sits ~3 orders of magnitude under the 80 ms budget, so it gates on the standard
  runner without the flake risk a tight latency SLO would carry. A
  synthetic-regression self-test step (build-first, so a compile failure is not
  mistaken for a gate trip) proves the gate is live — verified locally: a 300 ms
  injected stall fails the gate, exit 1. A second, defence-in-depth memory-DoS guard
  was added to the guarded read itself (`path_safety::MAX_GUARDED_READ_BYTES`, 64 MiB):
  a file beyond the hard ceiling is refused at the read before its buffer grows, so the
  parse-size cap is a parse/scan guard layered above a real read-allocation bound.
- **Deferred:** the transport (`validation.roundtrip`) harness for a real `watch`/MCP
  driver lands with those clients in DSV-007. (The loaded dev box cannot produce a
  clean absolute p95 — warm p95 here is ~0.03 ms — but the CI runner is the authority
  for the gate, not the dev box.)
- **Expected Outcome:** A small interactive `rayon::ThreadPool` (10a, spine — a Task 8
  predecessor) + a chunked-yield background pool (10b); per-workspace in-flight token;
  parse-size + walk-depth caps; a `validate_paths` warm-read + `4 agents + 1 scan`
  bench with a p95 SLO and a >80 ms queue-wait WARN wired as a CI gate.
- **Validation:** `cargo test -p eddacraft-anvil-intercept workspace_pool`;
  `cargo bench -p eddacraft-anvil-intercept ipc_roundtrip` (quiet box); CI gate fails
  on a synthetic regression.
- **Files:** `crates/anvil-intercept/src/workspace_pool.rs`,
  `crates/anvil-intercept/src/validate_paths.rs`,
  `crates/anvil-intercept/src/save_time.rs`,
  `crates/anvil-intercept/src/path_safety.rs`,
  `crates/anvil-intercept/benches/ipc_roundtrip.rs`,
  `.github/workflows/resource-budget.yml`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** None for the interactive-pool construction (10a — itself a DSV-005/Task-8 predecessor); **DSV-005 for the DoS caps (Task 11 also modifies `validate_paths.rs`) and the SLO bench (Task 16 depends on 8+10)**. The SLO gate is an ADR-061 §9 Phase-2 merge dependency.
- **Source:** subphase-a Tasks 10/11/16; RLB-002/-005/-008.

---

#### DSV-007: `watch` + MCP clients + status surface

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-03 via PR #2284
- **Intent:** Make the user-facing surfaces thin daemon clients with a safe fallback.
- **Expected Outcome:** `watch` routes save-time validation to the daemon and falls
  back to a *scoped* (never `--all`) check on daemon absence/mid-session death,
  surfacing `unavailable{daemon-absent}` (never a truncated `clean`) and warning once
  per disconnect; MCP `anvil_validate_write` re-points with a byte-identical in-process
  fallback; `anvil status` renders `clean|stale|pending|running|unavailable` (+ reason,
  + `confined: N`).
- **Progress (2026-06-03):** Task 12 (`watch` client) delivered —
  `crates/anvil-cli/src/commands/watch_save_time.rs` is the daemon `validate_paths`
  client + connection-lifecycle state machine (warn-once-per-disconnect, reconnect
  re-issues `request_full_scan`, mid-session death ⇒ scoped fallback +
  `unavailable{daemon-absent}`). Initially wired into `watch.rs`'s `run_one_action`
  as an **opt-in via `ANVIL_WATCH_DAEMON`** (default-off so the
  not-yet-auto-started daemon did not change default watch behaviour —
  trunk-releasable, per the release-gating model).
- **Progress (2026-06-09):** the DSV-021 rollout flip reuses this client path as
  the default when a live daemon serves `workspace_status`; explicit
  `ANVIL_WATCH_DAEMON=1` preserves the old forced-on diagnostics path, and
  `ANVIL_WATCH_DAEMON=0` opts out.
- **Task 13 reconciliation (council-confirmed 2026-06-03):** the execution-plan wording
  "re-point the in-process scan to daemon `validate_paths`" is corrected to **the daemon's
  `scan_buffer` verb**. MCP `anvil_validate_write` is a *pre-write* gate over *proposed
  content not yet on disk*; `validate_paths` has a frozen content-free wire and reads the
  exact openat2-guarded bytes from disk (ADR-061 §2/§7), so routing proposed content
  through it would be a false attestation. ADR-061 §3 only says MCP "re-points… to the
  daemon" — never names `validate_paths`. The existing `scan_buffer` + byte-identical
  embedded fallback already satisfies §3; Task 13 = the named tests + this note. Consequence
  for DSV-009 / Task 15: the **"MCP+daemon" parity leg uses `scan_buffer`, not
  `validate_paths`** (both route to the same `run_antipattern_check`).
- **Validation:** `cargo test -p eddacraft-anvil -- watch`; the MCP tool tests; status
  render tests.
- **Files:** `crates/anvil-cli/src/commands/{watch,watch_save_time,status}.rs`,
  `crates/anvil-cli/src/mcp/{validation.rs,tools/validate_write.rs}`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** DSV-005
- **Source:** subphase-a Tasks 12/13/17; DRVR; council item 8.

---

#### DSV-008: Confinement mode + `anvil workspace` CLI

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-03 via PR #2275
- **Intent:** Give operators an opt-in confinement boundary above the same-uid trust
  model.
- **Expected Outcome:** Operator-level config (`ANVIL_HOME`/XDG, owner-only) with
  `admission = open|allowlist`; allowlist mode refuses non-admitted roots with
  `workspace-not-admitted`, never reads the allowlist from a repo `.anvil.yaml`, and
  fails closed + loud on config load failure; the loader resolves config via the
  daemon's own `anvil_home_prefix()` (no `anvil-cli` dep); `anvil workspace allow|deny|list|mode`.
- **Validation:** `cargo test -p eddacraft-anvil-intercept confinement`;
  `cargo test -p eddacraft-anvil -- workspace`.
- **Files:** `crates/anvil-intercept/src/confinement.rs`,
  `crates/anvil-cli/src/commands/workspace.rs`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** DSV-003
- **Source:** subphase-a Task 14; council item 8(a).

---

#### DSV-009: Cross-path diagnostic parity gate

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-04 via PR #2294
- **Intent:** Prove the antipattern-family delivery paths return identical finding sets.
  **Scope reconciled (2026-06-04):** the antipattern family runs on the save-time
  `validate_paths` surfaces — **`watch+daemon` and `watch+fallback`** — so the gate
  covers those two. MCP `anvil_validate_write` deliberately stays on the `scan_buffer`
  verb (secret/launch-reasoning family, `default_rule_registry()`) per DSV-007 and
  produces no antipattern findings; its `daemon`↔`embedded` parity is gated separately
  on that family. The original "four-path antipattern parity" framing pre-dated that
  DSV-007 decision and is corrected in contract §7.2.
- **Expected Outcome:** An order-normalised golden parity test over a fixed corpus
  proving `watch+daemon` (`validate_paths`, guarded bytes) ≡ `watch+fallback`
  (`anvil check`, disk) antipattern finding sets, byte-identical under a shared
  `sort_diagnostics` normalisation fed in opposite orders; `workspace_assurance` and
  daemon-only `DoS` notices carved out; run as a gate. Backed by a production
  sort-before-envelope normalisation in `validate_paths`.
- **Validation:** `cargo test -p eddacraft-anvil-intercept --test diagnostic_parity`.
- **Files:** `crates/anvil-intercept/tests/diagnostic_parity.rs`,
  `crates/anvil-intercept/tests/fixtures/parity-corpus/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** DSV-005, DSV-007
- **Source:** subphase-a Task 15; ADR-061 §8.

---

### Sub-phase A-W — Windows + cross-platform parity

Parity for the Sub-phase A save-time surface on the other short-term-supported
targets. macOS already works (the daemon and `watch`/`status` clients are
`cfg(unix)`, so they run on Darwin — the only macOS gap was a Linux-gated test,
closed in PR #2291). Windows was the real gap: the save-time verbs were not served
and the clients were `cfg(not(unix))` stubs. **DSV-010 (Merged 2026-06-05 via
PR #2328, hardened via #2340)** closes it — the three verbs are served over the
per-user named pipe behind the ADR-068/070 read-safety guard, peer-SID auth, and
owner-only config trust. **DSV-011 (Merged 2026-06-05 via PRs #2317 + #2327)**
completes the pair — the Windows `watch`/`status` clients speak the served
verbs end-to-end; closure evidence is the green full cross matrix on dispatch
run 27102943706 (2026-06-07, after PR #2365 fixed an unrelated bench-test
flake on the Windows leg). Sub-phase A-W is closed 2/2.

#### DSV-010: Windows named-pipe save-time daemon

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-05 via PR #2328
- **Progress (2026-06-04):** increment 1 — the ADR-068 read-safety **guard** —
  delivered as `crates/anvil-intercept-win32/src/read_safety.rs`: `WorkspaceDir`
  held-handle anchor (C2), the per-component `NtCreateFile` + `OBJ_DONT_REPARSE`
  ladder `read_under` (reparse/junction refused as `ERROR_CANT_RESOLVE_FILENAME`,
  the Unix-`ELOOP` analogue), the Windows-hardened `normalise_rel`
  (backslash/drive/UNC/device/ADS/trailing-dot/reserved-name), and the 64 MiB
  refuse-don't-truncate cap (B2), with fixture tests (incl. a privilege-free
  `mklink /J` junction test). **Verified by cross-compile-check + clippy `-D
  warnings` on `x86_64-pc-windows-gnu`** — the project's current Windows bar
  (PR #2182 precedent). The fixture tests do **not** run in CI yet (see the
  finding below), so the FFI is type-verified, not runtime-verified.
- **Finding (2026-06-04) — Windows workspace build is pre-existing-red,
  expanding DSV-010 scope.** `cargo build/test --workspace` on `windows-msvc`
  fails before reaching this crate: `crates/anvil-intercept/src/change_class.rs`
  is unix-gated yet consumed by non-gated `assurance.rs`/`validate_paths.rs`
  (`unresolved import crate::change_class`). So making the **daemon crate
  Windows-buildable** (cfg-cleaning `change_class`, `save_time`, `path_safety`,
  `workspace_admission`, the assurance/validate spine) is a prerequisite of the
  remaining DSV-010 work, not just "lift `save_time.rs`". Only once the
  workspace builds on Windows can the guard's fixture tests run on the `rust.yml`
  windows matrix (`cargo test --workspace`).
- **Progress — DSV-010a (compile-clean, ADR-069 Stage 1, 2026-06-04):** the only
  actual daemon build-breaker was `change_class` being Unix-gated while its
  **neutral** `CanonicalChange` enum is imported by `assurance`/`validate_paths`
  (the other survey "blockers" — `workspace_admission`, `confinement`,
  `workspace_pool`/`registry` doc-links — are already gated or doc-only). Fix:
  un-gate `change_class`, keep `CanonicalChange` neutral, `#[cfg(unix)]`-gate the
  inode `PathIdentity`/`classify`/`IdentityTable`/probe. Verbs still reply
  `not enabled` on Windows (ipc.rs). Unix build + tests unchanged (verified);
  Windows verified via the `rust.yml` matrix (local cross-check of the full crate
  is blocked by `aws-lc-sys`' C build needing an msvc/mingw toolchain).
- **Progress — DSV-010b (functional, ADR-070 Stage 2, 2026-06-05):** the verbs
  are now served on Windows. Delivered: the platform-neutral
  `crate::workspace_anchor::WorkspaceAnchor` (Unix `O_PATH` dirfd via
  `path_safety` / the Windows ADR-068 `read_safety::WorkspaceDir` guard, behind
  one `read_rel`); `AdmittedRoots`, `save_time`, and `confinement::to_admitted_roots`
  lifted off `#[cfg(unix)]` onto the anchor (`save_time`/`workspace_admission` →
  `#[cfg(any(unix, windows))]`); the `ipc.rs` listener `save_time` field +
  `with_save_time_state` + `handle_connection` save-time arg made cross-platform
  and wired through the Windows named-pipe serve loop; `run_foreground` builds
  `SaveTimeState` + the unregister hook on Windows too; and the ADR-070 step-4
  peer-SID belt-and-suspenders (`anvil_intercept_win32::named_pipe_client_is_owner`
  — `GetNamedPipeClientProcessId → token SID` compare, fail-closed) in the accept
  loop beside the owner-only pipe DACL. Tests: a Windows IPC round-trip
  (`tests/save_time_wired_windows.rs`, per-PID pipe, MLP2-075 pattern) proving the
  three verbs answer over the pipe via `run_foreground`, and the DSV-009 parity
  gate extended to the Windows anchor read path (`diagnostic_parity.rs`). Verified
  locally: Unix build + full suite (586 + integration, green), `anvil-intercept-win32`
  `windows-gnu` clippy `-D warnings`, fmt + workspace clippy. The Windows daemon
  crate is not locally compilable (`aws-lc-sys` C build needs an msvc/mingw
  toolchain), so the Windows arms + Windows tests are runtime-verified on the
  `rust.yml` `windows-msvc` matrix (the DSV-010a / DSV-011 precedent).
- **Scoped out of DSV-010b (deliberate):**
  - **Windows `PathIdentity` (`FILE_ID_INFO`)** — the inode `PathIdentity` /
    `IdentityTable` (`change_class`) has **zero callers** outside its own module
    on either platform: the verbs read-and-certify, and change classification
    arrives pre-formed on the wire as the neutral `CanonicalChange`. A Windows
    `FILE_ID_INFO` identity would be unused FFI weakening nothing real; defer it
    to whenever `IdentityTable` is activated (a cross-platform follow-up).
  - **Windows tree-sitter parser injection** — the daemon's `symbol_parser`
    plumbing is symmetric on Windows, but the `anvil-cli` tree-sitter injection
    (`intercept_symbol_parser`) stays Unix-only for now, so the Windows daemon
    runs parser-less (safe `Partial` verdicts — the documented degraded mode Unix
    also uses without a parser; the antipattern diagnostics + DSV-009 parity hold
    regardless). Lifting the injection (→ `Certified` on Windows) is a small
    follow-up. Then DSV-011 clients (already in fallback mode) light up.
- **Windows-GA hardening follow-ups (Council 2026-06-05 → done 2026-06-07 via
  PR #2340):**
  - **Peer-SID check off the accept-loop thread — done.**
    `named_pipe_client_is_owner` (`OpenProcess` + `GetTokenInformation` ×2) now
    runs via `tokio::task::spawn_blocking` in the named-pipe accept loop (handle
    passed as `usize` — a Win32 `HANDLE` is not `Send`; `connected_server` held
    alive across the await), so a pathologically slow same-uid peer no longer
    stalls the reactor thread. Fail-closed on non-owner / error / join failure.
  - **Windows trusted-config ownership check — done.** `confinement::read_trusted`
    has a real Windows impl (`anvil_intercept_win32::read_trusted_config`):
    refuses a reparse point (symlink/junction → `SymlinkedConfig`) and a
    foreign-owned file (`GetSecurityInfo` owner-SID mismatch → new
    `NotOwnerSid`), reading the verified handle otherwise — the analogue of the
    Unix `O_NOFOLLOW` + owner-uid check. `create_owner_only_dir` creates the
    config dir with an owner-only DACL (`CreateDirectoryW` + SDDL), the analogue
    of the Unix 0700 dir. The interim "unverified on Windows" startup `warn` is
    removed. Unsafe FFI quarantined in `anvil-intercept-win32` (cross-checked on
    `x86_64-pc-windows-gnu`; runtime-verified on the `windows-msvc` matrix).
- **Intent:** Serve the frozen save-time verbs on Windows so a Windows project gets
  the same daemon-mediated save-time validation as Unix. ADR-015 mandates Windows
  support, and the IPC transport already speaks named pipes (MLP2-075 wired the MCP
  `scan_buffer` / protection-claim Windows client).
- **Expected Outcome:** `validate_paths` / `workspace_status` / `request_full_scan`
  are dispatched and answered on Windows over the per-user named pipe; `save_time.rs`
  (today `#![cfg(unix)]`) and its read path are lifted to a cross-platform boundary;
  same-user peer authorisation via the named-pipe ACL / `pipe_name_for_current_user`
  (the SO_PEERCRED equivalent); the frozen wire and verdict semantics are unchanged.
- **Read-safety design (was the gating unknown — resolved by
  [ADR-068](../../decisions/068-windows-save-time-read-safety.md), Accepted; staged
  by [ADR-070](../../decisions/070-daemon-windows-buildability.md), Accepted):** the
  verdict's Unix guard (`path_safety.rs` — `openat2(RESOLVE_NO_SYMLINKS |
  RESOLVE_BENEATH)` against a held `O_PATH` dirfd) has no Windows analogue. ADR-068
  mirrors it with `NtCreateFile` anchored at a held workspace directory handle +
  `OBJ_DONT_REPARSE` (per-component `FILE_OPEN_REPARSE_POINT` ladder fallback),
  preserving C2 (held-handle identity), no-reparse traversal (symlinks + junctions),
  beneath-root, and B2 (read-then-certify; refuse oversized, never truncate), in
  `anvil-intercept-win32` so the daemon stays `forbid(unsafe_code)`. **Shipped in
  PR #2328** as `read_safety.rs` (the `WorkspaceAnchor` Windows arm); the design
  gate is closed.
- **Validation:** a Windows IPC fixture round-trip (mirroring the MLP2-075 `windows_*`
  tests) proving the three verbs answer over the named pipe — the *test* binds a
  per-PID pipe name so it never collides with a real per-user daemon on the same
  runner (the MLP2-075 rationale), while production uses the per-user pipe from the
  Expected Outcome; the cross-path parity gate (DSV-009) extended to a Windows path.
- **Files:** `crates/anvil-intercept/src/{save_time,path_safety,ipc}.rs`,
  `crates/anvil-intercept-win32/`.
- **Confidence:** high — delivered; the read-safety design gate is closed (was:
  low, gated on the design decision above; ADR-068/070 Accepted and shipped).
- **Priority:** High (short-term-supported target).
- **Dependencies:** DSV-005; [ADR-068](../../decisions/068-windows-save-time-read-safety.md) Accepted (Windows read-safety); [ADR-070](../../decisions/070-daemon-windows-buildability.md) Accepted (buildability staging).
- **Source:** DSV-007 follow-up (macOS + Windows are short-term save-time targets);
  ADR-015; brainstorms `2026-05-01-hearth-rearchitecture.md` /
  `2026-05-07-daemon-sessions-surfaces-boundaries.md`.

#### DSV-011: Windows `watch` + `status` save-time clients

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-05 via PRs #2317 (client) + #2327 (hardening)
- **Progress (2026-06-07, closure):** with DSV-010b serving the verbs (PR
  #2328), the full `rust.yml` cross matrix ran green on dispatch run
  [27102943706](https://github.com/eddacraft/anvil-001/actions/runs/27102943706)
  — both `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` legs pass,
  including the named-pipe fixture test. The one red blocking that evidence
  was an unrelated latent timing flake in `anvil-bench`'s spawn liveness
  test (fixed via PR #2365 — deterministic reap instead of a 100 ms sleep).
  All Expected-Outcome surfaces verified live on `main`; item advances to
  Merged. Sub-phase A-W closes 2/2.
- **Progress (2026-06-04):** brought forward alongside DSV-010a (it resolves the
  same windows dead-code: the save-time client machinery had no Windows
  constructor). `WindowsPipeSaveTimeTransport` (`watch_save_time.rs`) speaks
  JSON-RPC over the per-user named pipe via `anvil-intercept-win32`'s
  `connect_owner_only_pipe_client` (owner-only DACL = the SO_PEERCRED analogue);
  the JSON-RPC framing is now shared with the Unix socket transport via a neutral
  `framing` module. `query_workspace_status` + `build_save_time_client` gain
  Windows arms. Until DSV-010b serves the verbs on Windows the daemon replies
  `Method not found` → folds to fallback (same as daemon-absent), so the Windows
  client is functional in fallback mode now. Cross-platform framing + unix path
  verified locally; the Windows path is verified green on the `rust.yml`
  `windows-msvc` matrix (the full suite passes — including the ADR-068 guard's
  reparse/junction-rejection, C2, and B2 tests, which runtime-caught a wrong
  `STATUS_REPARSE_POINT_ENCOUNTERED` constant cross-check could not).
- **Progress (DSV-011 Hardening):** added `REQUEST_TIMEOUT` + worker-thread +
  `recv_timeout` wrapper to `WindowsPipeSaveTimeTransport::round_trip` (and the
  fire-and-forget full-scan path) so a wedged pipe cannot stall `watch`/`status`
  (matches the established pattern from `query_daemon_status_windows_at_with_timeout`).
  Added `with_pipe_name` test seam (parity with MCP). Extended the round-trip
  tests with `windows_pipe_transport_maps_unserved_daemon_to_unavailable` (per-PID
  pipe fixture, NoopDispatcher → Unavailable, mirroring the socket test and
  MLP2-075). Removed the "known gap" note; the client is now hardened. (Work in
  `feat/dsv-011-hardening`.)
- **Intent:** Make the Windows user-facing surfaces thin save-time-daemon clients,
  matching the Unix `watch` / `status` wiring shipped in DSV-007.
- **Expected Outcome:** a `WindowsPipeSaveTimeTransport` (parallel to the MCP
  `WindowsPipeDaemonValidationClient`) backs the `cfg(not(unix))` stubs in
  `watch_save_time.rs` (`query_workspace_status`, `build_save_time_client`), so
  `watch` routes save-time validation and `anvil status` renders the assurance
  surface on Windows — under the same default-on/opt-out
  (`ANVIL_WATCH_DAEMON`) gate and scoped fallback as Unix.
- **Validation:** the watch socket round-trip + status render tests extended to a
  Windows named-pipe fixture (mirroring the MLP2-075 Windows test pattern).
- **Files:** `crates/anvil-cli/src/commands/{watch_save_time,watch,status}.rs`.
- **Confidence:** medium — mechanical once DSV-010 serves the verbs on Windows.
- **Priority:** High (short-term-supported target).
- **Dependencies:** DSV-010.
- **Source:** DSV-007 follow-up; ADR-015.

---

### Sub-phase A — deferred follow-ups (post-merge debt)

The DSV-005/006/007 capstone PRs each merged with a small set of follow-ups
explicitly deferred to keep the verdict path focused. These items track that
debt as first-class work rather than buried `Deferred:` notes. None re-opens a
Merged item; each is an additive improvement under the frozen wire.

#### DSV-040: Registry unregister-hook warm-state reclamation

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-04 via PR #2296
- **Intent:** Reclaim a worktree's warm `SaveTimeState` (graph cache + assurance
  machine) when its last session leaves the registry, so an unregister/evict
  does not leave warm state resident until LRU pressure or process exit.
- **Expected Outcome:** `SessionRegistry::set_unregister_hook` installs the hook
  post-construction (the warm cache is built after the registry in
  `run_foreground`); the daemon's single composed closure calls
  `SaveTimeState::invalidate` keyed on the canonical worktree path (matching the
  key `validate_paths` warms under); `RuleSetCache::invalidate` joins the same
  closure when MLP2-014 lands. Memory-promptness, not a correctness fix (the
  cache is already LRU + generation-guarded).
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib -- set_unregister_hook invalidate_reclaims`;
  `cargo test -p eddacraft-anvil-intercept --test daemon_config_wired -- run_foreground_reclaims_warm_state_on_unregister`
  (a negative check confirms the socket test reads `pending` — fails — when the
  hook body is neutered).
- **Files:** `crates/anvil-intercept/src/{registry,save_time,lib}.rs`,
  `crates/anvil-intercept/tests/daemon_config_wired.rs`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** DSV-005
- **Source:** DSV-005 deferred note ("registry unregister-hook wiring for
  warm-state reclamation"); MLP2-057.

---

#### DSV-041: Operator antipattern-config surface

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-04 via PR #2296
- **Progress (2026-06-04):** delivered — `crates/anvil-intercept/src/antipattern_config.rs`
  loads an owner-only `antipattern.yaml` (a local `deny_unknown_fields` file
  struct overlays the named fields onto `AntipatternCheckConfig::default`),
  reusing confinement's one audited `read_trusted` owner-only/`O_NOFOLLOW`
  reader (exposed `pub(crate)`) and the shared `anvil_config_dir` resolver — no
  duplicated security code, errors mapped to a neutral
  `AntipatternConfigError` at the boundary. `run_foreground` now calls
  `antipattern_config::load_or_fail_safe()` in place of the hardcoded default.
  Fail-safe posture: missing ⇒ full default set; untrusted/malformed ⇒ full
  default set + `error` log (a broken config never silently disables checks, the
  opposite-and-safe direction from confinement's fail-*closed*). 6 unit tests
  (missing/partial-overlay/unknown-key/group-writable/symlink/no-dir). Wiring is
  a single visible consumption beside the trusted `confinement::load_or_fail_closed()`
  (loaders are value-consumption, not the inert-builder #1671 class the
  `daemon_config_wired` socket tests guard). **Deferred:** an `anvil antipattern`
  CLI + an operator docs page (a behavioural end-to-end test needs an `ANVIL_HOME`
  test seam to avoid env races).
- **Intent:** Give operators a config surface to select/tune the save-time
  antipattern check set, instead of the hardcoded `AntipatternCheckConfig::default()`
  the daemon constructs at startup.
- **Expected Outcome:** A loader mirroring `confinement::load_or_fail_closed`
  (ANVIL_HOME/XDG, owner-only, `deny_unknown_fields`): missing file ⇒ default
  (permissive, the configured set); malformed/untrusted ⇒ fail-closed + loud,
  never a silent degrade-to-default; the loader propagates `Result::Err`.
  `run_foreground` calls it in place of the hardcoded default. Never read from a
  repo `.anvil.yaml`.
- **Validation:** `cargo test -p eddacraft-anvil-intercept antipattern_config`;
  a `daemon_config_wired`-style test proving a configured pattern set reaches the
  save-time verdict.
- **Files:** `crates/anvil-intercept/src/antipattern_config.rs` (new),
  `crates/anvil-intercept/src/lib.rs`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** DSV-005, DSV-008 (confinement loader is the pattern)
- **Source:** DSV-005 deferred note ("operator antipattern-config surface").

---

#### DSV-042: Interactive-pool offload of the synchronous parse

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-04 via PR #2296
- **Progress (2026-06-04):** delivered — the `fed_symbols` parse closure in
  `save_time.rs` now runs `p.parse(..)` via `state.scheduler.interactive().install(..)`
  instead of inline on the IPC connection thread, so the verdict's CPU work
  (parse + the antipattern scan, already on `env.pool`) is bounded by the one
  interactive pool and N concurrent agents cannot oversubscribe cores. The pure
  `validate_paths` core is unchanged. Correctness-neutral: the parser is handed
  the SAME guarded bytes (no second read, B2 preserved) and the
  `parser_receives_the_exact_guarded_bytes` + `validate_certifies_when_parser_feeds_matching_surface`
  tests stay green — the verdict is byte-identical, just computed on a pool
  thread. **Perf is CI-gated, not locally measured:** the `resource-budgets`
  warm-p95 gate (DSV-006) is the authority; this loaded dev box cannot produce a
  clean absolute p95 (it sits ~3 orders under the 80 ms budget regardless).
- **Intent:** Move the synchronous symbol parse off the IPC dispatch thread onto
  the interactive rayon pool so a large-file parse cannot block the dispatch
  thread for the duration of a tree-sitter parse.
- **Expected Outcome:** The `fed_symbols` parse runs via the interactive
  `WorkScheduler` pool (`pool.install`) rather than inline; verdict determinism
  and the guarded-bytes contract (parse the EXACT bytes the daemon hashed, no
  second read) are preserved; tree-sitter `Parser` `!Sync` is respected (built
  per call). The `4 agents + 1 scan` warm p95 stays under the ADR-031 80 ms
  budget on the `resource-budgets` CI gate.
- **Validation:** `cargo test -p eddacraft-anvil-intercept -- save_time validate_paths`;
  `cargo bench -p eddacraft-anvil-intercept ipc_roundtrip` (quiet box; CI gate is authority).
- **Files:** `crates/anvil-intercept/src/{validate_paths,save_time}.rs`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** DSV-005, DSV-006
- **Source:** DSV-005/006 deferred note ("interactive-pool offload of the
  synchronous parse"); ADR-067 ("later optimisation").

---

#### DSV-043: `validation.roundtrip` transport bench for `validate_paths`

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-04 via PR #2296
- **Progress (2026-06-04):** delivered — `benches/ipc_roundtrip.rs` gains
  `roundtrip_validate_paths()`, a report-only transport case that drives a warm
  `validate_paths` over the daemon socket (a listener with `with_save_time_state`
  + bench parser, one persistent connection so admission is paid once). It
  reports `validation.roundtrip:validate_paths` beside the in-process
  `validation.service:validate_paths` gate, so transport overhead = roundtrip
  p95 − service p95 is visible (locally ~0.066 ms vs ~0.040 ms ⇒ ~0.026 ms of
  socket framing/serialise). Report-only by design (the SLO gate stays the
  in-process service case; transport latency on a loaded box is noisy and not
  what ADR-031 governs). Verified locally: the verdict round-trips and the
  `workspace_assurance` envelope assertion passes all 200 samples; the SLO gate
  still PASSes. The earlier `validation.roundtrip` (session.list) case stays as
  the generic transport baseline.
- **Intent:** Add the documented-deferred transport-level bench that drives a
  real client through the daemon socket against `validate_paths` (the existing
  `validation.roundtrip` case only drives `session.list`; `midedit_roundtrip`
  only drives `scan_buffer`).
- **Expected Outcome:** A bench case that opens a daemon socket connection and
  measures a warm `validate_paths` round-trip (write + dispatch + serialise +
  read), reported alongside the in-process `validation.service` case so the
  transport overhead is visible. Local run is constrained by daemon/socket setup
  on a loaded box; the CI `resource-budgets` runner is the authority.
- **Validation:** `cargo bench -p eddacraft-anvil-intercept ipc_roundtrip`
  (quiet box / CI).
- **Files:** `crates/anvil-intercept/benches/ipc_roundtrip.rs`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** DSV-005, DSV-007
- **Source:** DSV-006 deferred note ("transport (`validation.roundtrip`) harness
  … lands with those clients in DSV-007"); confirmed NOT landed in #2284.

---

#### DSV-044: Emit assurance/fence transitions through the production fanout (Phase E)

- **Status:** Done 2026-06-08
- **Progress (2026-06-08):** delivered — `SaveTimeState` now holds the live
  `TelemetryBroadcaster` built over the daemon's production `Fanout`, and
  `SaveTimeConn` carries the registered session id as the load-bearing
  `originating_session_id` after a successful `RegisterSession` on that
  connection; mismatched roots suppress the broadcast rather than reusing the
  last session id. Assurance transitions still emit the local tracing mirror,
  then build `telemetry::envelope_for_assurance_transition` and call
  `TelemetryBroadcaster::broadcast`, so own-session subscribers receive the
  event through the same route/redaction path as MLP2-071 Phase 2. `FenceStore`
  now has an optional production telemetry attachment and emits
  `ActiveToFenced` / `FencedToActive` envelopes only for sessions registered on
  the exact canonical worktree. `RegistryOwnershipResolver` reads live
  `FenceStore` state for spoof-degraded origins (fail-closed on load error), so
  newly written spoof fences affect fanout routing immediately. The DSV
  regressions `save_time::tests::assurance_transition_emits_through_fanout`,
  `save_time::tests::assurance_transition_does_not_reuse_session_for_other_worktree`,
  `fence::tests::fence_worktree_emits_active_to_fenced_through_fanout`, and
  `fence::tests::unblock_worktree_emits_fenced_to_active_through_fanout` pin the
  production broadcaster path and session/worktree correlation. The Phase 2
  Council follow-ups tied to producer activation are also closed: live telemetry
  `subscriber_count` / `dropped_envelopes` are surfaced as additive optional
  `query_status` fields, and the first slow-subscriber drop logs at `warn`
  while per-drop detail remains `debug` and non-blocking. Verified locally with
  `cargo test -p eddacraft-anvil-intercept --lib` (603 passed) and
  `cargo test -p eddacraft-anvil-intercept-proto` (71 passed).
- **Unblocked 2026-06-08:** the subscriber surface this depended on —
  [MLP2-071](../../modules/multilayer-protection-v2.aps.md) **Phase 2** (the IPC
  `subscribe-telemetry` accept-loop multiplex → `Fanout::register`, and the
  `TelemetryBroadcaster` that calls `Fanout::route` and delivers per-subscriber
  envelopes) — landed via PR
  [#2414](https://github.com/eddacraft/anvil-001/pull/2414) (Merged 2026-06-08;
  closes the subscriber-surface half of #1722). The `Fanout::route` reader now
  exists, so DSV's emission call sites are no longer "an emit with no reader"
  (dead code) nor a scoping-leak risk — they route through the live broadcaster
  (`crate::broadcaster::TelemetryBroadcaster::broadcast`), which enforces the
  INTD-015 redaction + D6 spoofed-origin denial.
- **First step on pickup:** this slice is security-sensitive (per-session
  telemetry scoping + cross-session redaction), so it still warrants the
  design/Council pass this entry calls for, coordinated with the MLP2-071 owner.
  The producer/consumer ownership boundary is recorded in
  [`plans/specs/2026-05-21-intd-015-cross-session-attribution-design-pass.md`](../../specs/2026-05-21-intd-015-cross-session-attribution-design-pass.md)
  (2026-06-08 addendum, "Phase 2 Council follow-ups"): DSV owns the emit call
  sites in `save_time`/`telemetry`/`fence.rs`; the broadcaster handle they call
  is MLP2-071's, shipped. The addendum also flags the open identity assumption
  (registrant==subscriber peer) the design pass should confirm before wiring
  real producers.
- **Intent:** Route the DSV-built assurance- (and fence-) transition notification
  envelopes through the production `Fanout::route` so subscribers actually
  receive them, instead of the envelopes being constructed only on the tracing
  mirror / in tests.
- **Expected Outcome:** When an assurance transition occurs, the production code
  path builds the envelope (`telemetry::envelope_for_assurance_transition`) and
  routes it through `Fanout::route` to authorised subscribers. **Cross-module:**
  the subscribe-handler (`IpcCommand::SubscribeTelemetry` per-connection JSON-RPC
  handler → `Fanout::register`) and the producer broadcaster are
  [MLP2-071](../../modules/multilayer-protection-v2.aps.md) Phase 2 / Phase E, *not* DSV; DSV
  owns only the assurance-transition emission call sites. Security-sensitive
  (per-session telemetry scoping + cross-session redaction) — warrants its own
  design/Council pass coordinated with the MLP2-071 owner; do not land the
  emission sites before the MLP2-071 fanout reader exists (an emit with no reader
  is dead code; an emit that bypasses `Fanout::route` is a scoping-leak bug).
- **Validation:** the MLP2-071 Phase 2 subscribe/broadcast tests plus a DSV
  assurance-transition-emits-through-fanout test.
- **Files:** `crates/anvil-intercept/src/{save_time,telemetry,fence}.rs` (DSV
  slice); `crates/anvil-intercept/src/{ipc,lib}.rs` (MLP2-071 slice)
- **Confidence:** low
- **Priority:** Low
- **Dependencies:** MLP2-071 Phase 2 (subscribe handler + broadcaster) —
  **satisfied** (Merged 2026-06-08 via PR
  [#2414](https://github.com/eddacraft/anvil-001/pull/2414)); DSV-005 (Merged).
- **Source:** DSV-005 deferred note ("`Fanout::route` subscriber delivery (Phase
  E)"); MLP2-071 Phase 2.

---

### Sub-phase A′ — GV2 hot-read swap

#### DSV-020: Swap the GV2 hot-read slice under the frozen wire

- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-08 via PR #2446
- **Progress (2026-06-09 closeout):** the stale `Blocked` state is reconciled to
  the already-merged GV2-027 stack: `validate_paths` now reads the resident GV2
  hot-read index under the frozen DSV wire, with verdict parity and privilege
  containment wired in the same stack. The user-facing default-on flip is tracked
  separately as DSV-021.
- **Intent:** Replace the interim `SymbolGraph` cache with GV2 resident warm indexes
  behind the unchanged `validate_paths` wire, so the verdict reads GV2's hot-path API
  instead of the rebuild-on-restart interim cache.
- **Expected Outcome:** The daemon backing is the GV2 hot-read slice; the wire,
  `check_families` scoping, and parity gate are unchanged; latency stays within the
  ADR-031 budget.
- **Validation:** GV2-027 verdict-parity property test and the Sub-phase A parity
  gates; GV2-025 Criterion hot-read benchmark meets ADR-031.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** GV2-010, GV2-011, GV2-022, GV2-028, GV2-029; the GV2
  hot-/non-hot-path boundary gate; DSV-005
- **Source:** ADR-061 sub-phase A′; ADR-063.

---

#### DSV-021: Default-on daemon routing with rollout controls

- **Status:** Done 2026-06-09
- **Intent:** Flip save-time daemon routing from preview opt-in to safe default-on
  for `check` watches once the A′ backing is live, without producing
  daemon-absent warning storms for users who do not run the daemon.
- **Expected Outcome:** With `ANVIL_WATCH_DAEMON` unset, `anvil watch --action
  check` builds the save-time client only after the daemon answers the served
  `workspace_status` verb; no live daemon keeps the subprocess-only path and
  default status output unchanged. `ANVIL_WATCH_DAEMON=0` / `false` / `off` /
  `no` opts out. `ANVIL_WATCH_DAEMON=1` / `true` / `on` / `yes` preserves the
  previous forced-on diagnostic mode, including scoped daemon-absent fallback.
  Public docs describe all three states.
- **Validation:** `cargo test -p eddacraft-anvil daemon_routing`; targeted watch
  and status tests.
- **Files:** `crates/anvil-cli/src/commands/{watch_save_time,watch,status}.rs`,
  `docs/public/anvil/operations/config.md`,
  `docs/public/anvil/guides/agent-harness.md`,
  `docs/public/anvil/integrations/{watch-output,mcp}.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** DSV-020; GV2-025; ADR-075 rollout controls.
- **Source:** ADR-075 default-on save-time daemon routing; v0.8.0-beta release
  cut criteria.

---

### Sub-phase B — warm-start persistence

#### DSV-030: Warm-start persistence for the daemon graph cache

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-06-17 via PR #2688
- **Intent:** Let the daemon restore graph indexes (not verdicts) on restart so a fresh
  connection is not `Stale` until a full scan completes.
- **Expected Outcome:** A default-off, per-uid, owner-only snapshot location;
  warm-start restores **indexes only, never the verdict**; structural-identity-only
  privacy line per the validation contract §9; crash-safe. The serialization core
  (sealed allowlist `SnapshotPayload` DTO + `postcard` codec + magic/version/CRC
  integrity gate + golden round-trip + relative-path no-leak test) already shipped
  in `anvil-graph-cache/src/snapshot.rs` (with the ADR); this item adds the
  **daemon-side disk I/O + timing**: atomic durable symlink-safe `write_snapshot` /
  `load_snapshot` / GC sweep, wired to write-after-scan + load-on-first-contact +
  start-GC + graceful-shutdown, behind `ANVIL_PERSIST_GRAPH` (default-off,
  fail-closed). A restored entry serves stale reads; a full scan is
  disk-authoritative (restored entry invalidated before rebuild) so a
  deleted-while-down file never survives into a `Clean` graph.
- **Validation:** restart-restores-indexes test; default-off assertion; the verdict is
  still re-derived from bytes after warm-start.
- **Confidence:** low
- **Priority:** Low
- **Dependencies:** [ADR-069](../../decisions/069-graph-v2-persistence.md) (Accepted);
  GV2-021 (Released/Shipped via #2301); DSV-020; DSV-045 (the reconcile scan).
- **Follow-up (not this item; tracked for the default-on graduation, ADR-069 §7):**
  (a) the content-hash **fast**-reconcile (ADR-069 §3 — skip re-parse of files whose
  content hash is unchanged) needs the snapshot DTO to carry per-file content hashes;
  (b) **orphan-by-key GC** (ADR-069 §10 — sweep `*.snap` for worktrees with no
  registered worktree) — the filename is a one-way SHA-256 hash so this needs a
  registry cross-ref once the registry is populated, not a start-time sweep (which
  has no registered worktrees yet); the `*.tmp` sweep and unregister-time removal +
  the "deleting `graph-cache/` is safe" escape hatch cover the rest; (c) the
  `snapshot_load_result`/`snapshot_write_result` **fleet counters** + the ADR-035
  write-failure **Notification** (§10) — tracing logs at the §10 severities ship now;
  (d) the openat/`O_PATH`-anchored **directory** create (§4) — per-file `O_EXCL` +
  `O_NOFOLLOW` ship now and bound it under the same-uid boundary; (e) a **write
  debounce** (§4 — currently writes once per successful scan, never per-save, which
  the DSV-045 scan coalescing already bounds); (f) **Windows** persistence (Unix-only
  here, mirroring the DSV-010/011 parity split).
- **Source:** ADR-061 §9; validation contract §9;
  [ADR-069](../../decisions/069-graph-v2-persistence.md) (GV2-021).

---

### Full-scan executor

The `request_full_scan` verb shipped (DSV-002/005) and the scan primitives
shipped (DSV-006: `walk_capped`, `run_chunked_scan`, `ScanCancel`, `DosCaps`, the
two-pool scheduler), but the verb only sets the assurance machine to `Pending` —
nothing drives `Pending → Running → Clean` or populates the warm
`KernelGraphCache`. Population today is **only** via save-time `validate_paths` →
`apply_delta`, so a fresh session's graph stays cold until a file is saved. This
item builds the executor loop that closes that gap (and the
[ADR-084](../../decisions/084-gctx-graph-handle-access.md) C1 cold-start
requirement). Architecture decided by
[ADR-085](../../decisions/085-daemon-full-scan-executor.md).

#### DSV-045: Full-scan executor

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-06-16 via PR #2674
- **Intent:** Drive a queued `request_full_scan` to completion on the background
  pool — walk, parse, apply, complete — so the warm graph cache is populated
  without a save, with cancellation, a timeout watchdog, eviction-rewarm, a
  scan↔save race guard, truncation handling, and DoS coalescing.
- **Expected Outcome:** A `Pending` assurance state spawns a background-pool job
  that opens its own `WorkspaceAnchor` on the admitted canonical root, walks the
  worktree (gitignore pre-filtered, `DosCaps`-bounded), parses each file with the
  injected `SymbolParser` on `scheduler.background()` (the daemon links no
  parser), feeds each file as `ChangeKind::Create` through `apply_delta`, and
  transitions `start_scan → complete_scan` holding the per-key machine lock only
  for those brief calls (ADR-084 C2). An interactive `validate_paths` preempts a
  mid-chunk scan via `ScanCancel`; on yield, applied deltas are kept, the
  worktree goes `Stale`, and a continuation re-queues from the processed offset.
  `complete_scan` **reads-and-clears a per-key dirty-during-scan flag atomically
  within the same per-key machine-lock critical section** as the `Clean`
  transition (compare-and-clear under lock) → `Clean` only if never dirtied, else
  `Stale(CrossFileResolutionNeeded)` + re-queue. The dirty flag is set by **ANY**
  `apply_delta` for the key during a `Running` scan, regardless of call origin —
  interactive `validate_paths` *and* a GCTX on-demand re-warm. A no-parser daemon
  `mark_stale`s, never producing a phantom empty `Clean`. A worktree still over
  `max_walk_files` after the gitignore pre-filter resolves to the new
  `AssuranceState::Bounded` (warm-but-bounded; wire `"bounded"`, distinct from the
  unrelated `Coverage::Partial`), carrying a `scan_coverage:
  Option<ScanCoverage { scanned_files, total_files }>` field on
  `WorkspaceAssurance` and `reason = None` (a lifecycle state, like `Clean`),
  never `Clean`. The same proto change adds `#[serde(other)] Unknown` to
  `AssuranceState` so the new variant is genuinely additive/forward-compatible
  (consumers treat `Unknown` fail-safe as `Stale`); `status`/`watch`/GCTX each
  explicitly handle `Bounded` (no wildcard-to-`Clean`). A per-key `scan-enqueued`
  CAS coalesces repeated `request_full_scan` so N calls drive one scan; the CAS
  flag resets via an RAII/drop guard on **any** job exit (completion, panic, or
  cancellation), so a panicked/cancelled scan never wedges `request_full_scan`
  inert for that key. The executor is reactive to `Pending` and the daemon
  auto-enqueues on first contact (`validate_paths` / `workspace_status`) against a
  cold key. A `WarmStateEvicted` event re-queues a `Pending` scan.
- **Validation:**
  `cargo test -p eddacraft-anvil-intercept -- full_scan_executor`. Named tests:
  - **Order-independent convergence** —
    `scan_driven_graph_equivalent_to_save_driven_baseline`, run against a
    minimum corpus that includes an import cycle (A→B→A), a diamond
    (A→B,C→D), and ≥10 files with cross-file imports (so order-independence is
    actually exercised, not asserted on a trivial graph).
  - **Dirty flag, origin-agnostic + compare-and-clear** —
    `apply_delta_during_running_scan_marks_stale_not_clean`, plus
    `non_validate_paths_apply_delta_during_running_also_sets_dirty` (a GCTX-origin
    re-warm during `Running` sets the flag), and a case asserting the flag is
    read-and-cleared under the same lock as the `Clean` transition (set/check
    cannot interleave).
  - **Truncation** —
    `over_walk_cap_after_gitignore_resolves_bounded_not_clean` (resolves to
    `AssuranceState::Bounded` with a populated `scan_coverage` and `reason =
    None`).
  - **No-parser** — `no_parser_marks_stale_never_starts_scan`.
  - **Coalescing liveness** — `repeated_request_full_scan_drives_one_scan` and
    `executor_panic_resets_scan_enqueued_flag` (the CAS flag clears on a
    panicking/cancelled job so a subsequent `request_full_scan` still enqueues).
  - **Yield/cancel** —
    `yield_keeps_applied_deltas_and_resumes_from_processed_offset`.
  - **Eviction rewarm** — `evicted_warm_state_requeues_and_rewarms`.
  - **C2 concurrency** — a `validate_paths` completes within the ADR-031 budget
    *while* a scan is `Running`, mechanically proving the per-key lock is not held
    across walk+parse+apply.
  - **`Bounded` exhaustiveness** — `status`/`watch`/GCTX each explicitly handle
    `Bounded` (no wildcard-to-`Clean`), verified by an exhaustiveness (compile) or
    per-consumer test.
  - **ADR-031 bench + self-test** — `cargo bench -p eddacraft-anvil-intercept
    ipc_roundtrip`: the scan-in-flight case holds interactive p95 under 80 ms, and
    a synthetic regression self-test mirroring DSV-006 injects an artificial scan
    stall and asserts the gate exits **non-zero** (proving the gate catches a
    regression). CI `resource-budgets` gate is the authority.
- **Files:** `crates/anvil-intercept/src/save_time.rs`,
  `crates/anvil-intercept/src/full_scan_executor.rs` (new),
  `crates/anvil-intercept/src/assurance.rs`,
  `crates/anvil-intercept-proto/src/protocol.rs`
  (`AssuranceState::Bounded` + `#[serde(other)] Unknown` fallback + new
  `ScanCoverage { scanned_files, total_files }` struct + `scan_coverage`
  field on `WorkspaceAssurance` + updated `reason`-invariant doc-comment),
  `crates/anvil-intercept/src/workspace_pool.rs`,
  `crates/anvil-intercept/benches/ipc_roundtrip.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** DSV-006 (scan primitives);
  [ADR-085](../../decisions/085-daemon-full-scan-executor.md). Soft dependency on
  DLIFE-002 (UX docs only — code ships independently). Unblocks
  [GCTX-010](graph-context-delivery.aps.md) C1 (session-init + on-demand warm-up
  triggers rebase onto this).
- **Source:** ADR-084 C1 cold-start warm-up; ADR-085; planning council
  `plan-898d9222`.

---

#### DSV-046: Headless background save-time driver contract

- **Status:** Done 2026-07-04 — design deliverable (planning-council direction
  validate). [ADR-101](../../decisions/101-headless-save-time-driver.md) Accepted
  (operator); hardened spec at
  [`specs/2026-07-04-headless-save-time-driver-design.md`](../../specs/2026-07-04-headless-save-time-driver-design.md).
  **Keystone:** daemon-supervised detached `anvil watch --save-time-driver`
  sidecars (one per durable registration), not in-daemon `notify` (ADR-064). The
  registry `membership_hook` (ACTMO-014) is the spawn/stop seam. Splits into
  **DSV-047..051** (Ready, cut-line).
- **Source:** Operator usefulness review 2026-06-29 + RELEASE-PLAN addendum; ACTMO
  registration UX merged without an unattended filesystem observer.
- **Intent:** Decide the non-MCP, non-visible save-time driver model so
  `anvil start` can honestly mean background daemon-backed validation is active.
- **Expected Outcome:** ADR + spec + Ready implementation items (delivered as
  DSV-047..051).
- **Validation:** Planning council direction validate recorded in the design spec
  §Planning council notes; implementation validation lives on DSV-051.
- **Dependencies:** ACTMO-014 (membership hook), DSV-007, ADR-094 decision 7.
- **Confidence:** high — architecture pinned; implementation is a bounded split.

---

#### DSV-047: Daemon `SaveTimeDriverSupervisor`

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-07-04 via PR #3191
- **Progress (2026-07-04, delivery):** `save_time_driver.rs` lands the
  supervisor honouring all four review pins (enqueue-only hook; no
  auto-respawn; spawn-failure ⇒ `failed` never panic; `DaemonLauncher` seam
  reused — `spawn_detached` now returns the child PID and
  `DetachedCommandLauncher` gained `with_env` for the
  `ANVIL_SAVE_TIME_DRIVER_LOG` handoff). Hardening added during review:
  `stop_all` latches a shutdown flag checked under the drivers lock (held
  across spawn→insert) so an in-flight spawn can never orphan its child;
  `stop_all`/`reconcile_on_start` run via `spawn_blocking` on both daemon
  exit paths; on start-time-capable platforms a driver is never *tracked*
  without its PID-reuse discriminator (unreadable start time at spawn ⇒ the
  just-spawned child is stopped immediately and reported `failed`), and the
  startup reconcile never signals a bare-PID record. The artefact directory
  is resolved by `save_time_driver::default_driver_dir()` with the child's
  own precedence (`{ANVIL_HOME}/runtime/save-time-drivers/`) — deliberately
  NOT the PID-file parent, which skips `runtime/` under `ANVIL_HOME`. Host
  opt-in is `ForegroundOpts::with_save_time_drivers`, set only by the
  production `anvil intercept start --foreground` entry (test/embedded hosts
  must never re-exec their own binary); `ANVIL_NO_SAVE_TIME_DRIVER` opt-out
  honoured on top. DSV-049 consumes `driver_status`/`status_snapshot`.
- **Source:** [ADR-101](../../decisions/101-headless-save-time-driver.md) decision 1;
  design spec §Driver contract.
- **Intent:** Consume durable membership changes and manage detached driver
  children without duplicating registration semantics.
- **Expected Outcome:** `SaveTimeDriverSupervisor` in `anvil-intercept` wires
  `SessionRegistry::set_membership_hook`: on `Registered` spawn one detached
  `anvil watch --save-time-driver --worktree <canonical>` via the DLIFE launcher
  pattern; on `Unregistered`/`Reaped` terminate the child; on daemon startup
  reconcile drivers for all reloaded durable registrations; maintain PID registry
  under `{ANVIL_HOME}/runtime/save-time-drivers/`; honour non-empty
  `ANVIL_NO_SAVE_TIME_DRIVER`; stop all drivers on daemon shutdown.
  **Review pins (2026-07-04):** (a) the membership hook only **enqueues** — the
  supervisor consumes events on its own task, so spawn/PID-file I/O never runs
  on the registry call path (`signal_membership` fires synchronously inside
  `session.register` handling); (b) child death while the daemon lives does
  **not** auto-respawn at cut-line — status reports `failed` honestly and the
  respawn/backoff policy is an explicit follow-up decision, not an accident;
  (c) spawn failure (including a stale `current_exe` path after a binary
  upgrade) marks the driver `failed` and never panics the supervisor. Reuse the
  `DaemonLauncher` trait + `DetachedCommandLauncher` in
  `crates/anvil-intercept/src/ensure.rs` (in-crate; the trait is the
  fake-launcher test seam).
- **Validation:** `cargo test -p eddacraft-anvil-intercept save_time_driver` (new
  module tests with fake launcher); supervisor unit tests for spawn/stop/reconcile
  without a live filesystem.
- **Files:** `crates/anvil-intercept/src/save_time_driver.rs` (new),
  `crates/anvil-intercept/src/lib.rs` (hook wiring in `run_foreground`),
  `crates/anvil-intercept/src/registry.rs` (hook consumer only if needed)
- **Dependencies:** DSV-046 (design), ACTMO-014 (Merged), DSV-048 (spawn argv
  contract)
- **Confidence:** medium
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

---

#### DSV-048: CLI `anvil watch --save-time-driver` headless mode

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-07-04 via PR #3186
- **Progress (2026-07-04, delivery):** shipped as designed plus review
  hardening: new `crates/anvil-cli/src/commands/watch_driver.rs` (child-owned
  `DriverLog` — append + Windows-safe remove-then-rename rotation to `.1` at
  1 MiB; log-path resolution `ANVIL_SAVE_TIME_DRIVER_LOG` → runtime-dir
  default mirroring the daemon PID-file precedence; leaf+SHA-256-prefix log
  naming), `WatchOutputMode::Driver` (headless regardless of TTY),
  `watch_daemon_offer_applies` gate (driver never offers/ensures a daemon),
  `DispatcherInner.driver_log` verdict sink with one-line stderr breadcrumb.
  Argv contract enforced at the clap layer (`--worktree` mutually required;
  `--action`/`--file`/`--patterns`/`--exclude`/`--plans`/`--source`/`--all`
  conflict). Worktree canonicalised before the log id is derived. 10 new
  `watch_save_time_driver*` tests; live-daemon E2E deferred to DSV-051 (local
  smoke blocked by the beta licence wall, exit 3). Post-merge plan:
  [`reviews/post-merge/feat-dsv-048-save-time-driver.md`](../../reviews/post-merge/feat-dsv-048-save-time-driver.md).
- **Source:** [ADR-101](../../decisions/101-headless-save-time-driver.md) decision 2;
  design spec §Spawn shape.
- **Intent:** Provide a stable, headless entrypoint the supervisor can spawn without
  a visible terminal or daemon lifecycle prompts.
- **Expected Outcome:** `WatchArgs` gains `--save-time-driver` and required
  `--worktree <PATH>`; driver mode forces plain/headless output (no TUI, no
  `[watching]` stdout banners), skips daemon offer/spawn, reuses
  `watch_save_time` daemon routing (`DefaultOnWhenLive`), appends findings to the
  supervisor-provided log path (env `ANVIL_SAVE_TIME_DRIVER_LOG` or default under
  runtime), exits non-zero only on unrecoverable setup failure (not on findings).
  **Review pin (2026-07-04) — log ownership:** the **child owns the findings
  log** end-to-end (opens, appends, rotates/truncates at 1 MiB via the env
  path); the supervisor redirects the child's stdout/stderr to a **separate**
  crash-capture file (`<worktree-id>.spawn.log`), never to the findings log —
  two writers plus rotate-under-a-held-redirect-fd is the failure mode this
  split avoids.
- **Validation:** `cargo test -p eddacraft-anvil -- watch_save_time_driver`; driver
  mode does not call `ensure_daemon` (mock/spy test).
- **Files:** `crates/anvil-cli/src/commands/watch.rs`,
  `crates/anvil-cli/src/commands/watch_save_time.rs`
- **Dependencies:** DSV-007 (Merged)
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

---

#### DSV-049: `save_time_driver` status wire + derivation

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-07-05 — implementation present on `origin/main` at
  `370167883` (`feat(daemon): surface save-time driver in status (DSV-049)`) and
  `de3721a67` (`test(daemon): pin Failed driver overlay through the provider
  (DSV-049)`) when validated in `feat/dsv-049-status-wire`; closeout reconciles
  stale APS state.
  Delivered `SaveTimeDriverStatusV1` with `#[serde(other)] Unknown`, the
  `WorktreeStatusV1.save_time_driver` additive field, daemon provider overlay from
  `SaveTimeDriverSupervisor::status_snapshot`, plain/JSON status rendering, and
  activation evidence helpers that treat unknown/absent fail-safe.
- **Validation evidence (2026-07-05):**
  `cargo test -p eddacraft-anvil -- status_save_time_driver`;
  `cargo test -p eddacraft-anvil-intercept-proto save_time_driver`;
  `cargo test -p eddacraft-anvil -- driver_attached`.
- **Source:** [ADR-101](../../decisions/101-headless-save-time-driver.md) decisions 4–5;
  ADR-094 decision 6 assurance axis; ACTMO-017 soft-dep.
- **Intent:** Make driver attachment observable so `watching` is evidence-backed.
- **Expected Outcome:** `WorktreeStatusV1` gains `save_time_driver: attached | absent
  | failed` (additive wire field); supervisor updates driver state; `anvil status`
  plain + `--json` render per-worktree driver state; activation diagnostic uses
  `registered ∧ driver_attached` for save-time-active `watching` copy (distinct from
  membership-only `watching`). **Review pin (2026-07-04) — forward compat:** the
  new enum ships with `#[serde(other)] Unknown` (consumers treat it fail-safe as
  `absent`) from day one — the `AssuranceState::Bounded` lesson: without the
  fallback, the next variant is a breaking wire change.
- **Validation:** `cargo test -p eddacraft-anvil -- status_save_time_driver`;
  `apps/e2e` status JSON contract extended if applicable.
- **Files:** `crates/anvil-intercept-proto/src/status.rs`,
  `crates/anvil-intercept/src/save_time_driver.rs`,
  `crates/anvil-cli/src/commands/status.rs`,
  `crates/anvil-cli/src/activation/{diagnostic,render}.rs`
- **Dependencies:** DSV-047 (Merged 2026-07-04 via PR #3191), ACTMO-017 (Merged)
- **Confidence:** medium
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

---

#### DSV-050: Activation copy — honest armed posture without `anvil watch`

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-07-05 via PR #3200
- **Source:** ACTMO-006; design spec; CIB-162..166 class (misleading next-step).
- **Intent:** When the supervisor attaches a driver for the current worktree,
  `anvil start` must not recommend a foreground `anvil watch`.
- **Expected Outcome:** `start.rs` next-step arbitration: driver attached ⇒ point
  to `anvil intercept status` / log path, not `anvil watch`; driver absent but
  registered ⇒ honest "save-time driver is not attached" guidance; `anvil
  intercept status` reports active driver count;
  activation render tests updated.
- **Validation:** `cargo test -p eddacraft-anvil -- start_save_time_driver_copy`;
  `cargo test -p eddacraft-anvil -- cli_status_renders_save_time_driver_count_and_failed_count`;
  `cargo test -p eddacraft-anvil -- cli_status_deduplicates_save_time_driver_counts_by_worktree`;
  `cargo test -p eddacraft-anvil -- first_run_recipe_layer_lines_reflect_diagnostic`;
  `cargo test -p eddacraft-anvil -- daemon_backed_watching_without_driver_says_driver_is_not_attached`
- **Files:** `crates/anvil-cli/src/commands/{start,intercept}.rs`,
  `crates/anvil-cli/src/activation/render.rs`
- **Dependencies:** DSV-049, ACTMO-006 (Merged)
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

---

#### DSV-051: Runbook + E2E regression matrix

- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged
- **Source:** RELEASE-PLAN cut criteria; design spec §Validation matrix.
- **Intent:** Prove the usefulness addendum end-to-end and document operator
  recovery paths.
- **Expected Outcome:** Runbook at `docs/runbooks/save-time-background-driver.md`
  (opt-outs, log locations, inotify guidance, Windows notes); E2E cases in
  `apps/e2e/` for: `start --no-mcp` headless driver, planted finding in log,
  second worktree register, daemon restart reconciliation, `intercept stop`,
  `ANVIL_NO_SAVE_TIME_DRIVER`; docs index updated.
  **Review pins (2026-07-04):** (a) the Windows daemon runs **parser-less**
  (tree-sitter injection is Unix-only, DSV-010 scope-out), so the Windows
  planted-finding assertion must plant an **antipattern-family** finding (works
  parser-less) — never expect `Certified` coverage on that leg; (b) multi-driver
  E2E cases use small fixture worktrees and serialise on the Linux leg (known
  inotify-exhaustion risk on shared runners); (c) the Windows operator
  verification checklist lives at
  [`execution/DSV-051.windows.actions.md`](../../execution/DSV-051.windows.actions.md).
- **Validation evidence (2026-07-06):**
  `pnpm --filter @eddacraft/anvil-e2e exec vitest run smoke/save-time-driver.e2e.test.ts`
  passes (3/3 DSV-051 driver tests); `pnpm run aps:index:check`,
  `pnpm run aps:active-lint`, `pnpm run docs:check`, `pnpm run lint:md`,
  `pnpm run format:check`, and `git diff --check` pass. Broader
  `pnpm --filter @eddacraft/anvil-e2e test:smoke` remains blocked by the
  existing `smoke/smoke.e2e.test.ts` Rust MCP launch shim expectation
  (`gateUnavailable` vs `allow`), reproduced standalone after rebuilding
  `target/debug/anvil`; the new DSV-051 smoke file passes in that run.
- **Merged:** 2026-07-06 via PR #3205 (`717299e37`).
- **Files:** `docs/runbooks/save-time-background-driver.md`,
  `apps/e2e/src/**/*save*time*driver*.e2e.test.ts`, public activation docs cross-links
- **Dependencies:** DSV-047, DSV-048, DSV-049, DSV-050
- **Confidence:** medium
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** minor

---

## Decisions

1. **Delivery module, not foundation** — DSV owns the daemon save-time *delivery*
   across sub-phases; GV2 owns the graph *substrate*. DSV work items depend on GV2
   items; they do not duplicate them.
2. **Freeze the wire once, swap the backing** — the interim cache (A), GV2 hot-read
   slice (A′), and warm-start persistence (B) all sit behind the same frozen
   `validate_paths` wire so consumers never re-integrate.
3. **Narrow the attestation** — `coverage: certified` attests the antipattern family
   only; structural policy stays on whole-repo `anvil gate` (ADR-061, B2).
4. **No tree-sitter in the daemon** — the cache write-path receives kernel-parsed
   `FileSymbols`; the daemon depends on `anvil-graph-cache` (petgraph) only (ADR-064).

## Stats

| Sub-phase | Items | Completion | Status |
| --------- | ----- | ---------- | ------ |
| A — Interim-cache `validate_paths` | 9 | 9/9 done | Done (all Merged; awaiting release) |
| A-W — Windows + cross-platform parity | 2 | 2/2 done (DSV-010 Merged — verbs served on Windows + hardening; DSV-011 Merged — clients verified on the green cross matrix, run 27102943706) | Done (all Merged; awaiting release) |
| A — deferred follow-ups | 5 | 5/5 done | Done |
| A′ — GV2 hot-read swap + default-on routing | 2 | 2/2 done | Done |
| Full-scan executor | 1 | 1/1 done (DSV-045 Merged 2026-06-16 via #2674 — ADR-085) | Done (Merged; awaiting release) |
| B — Warm-start persistence | 1 | 1/1 done (DSV-030 Merged 2026-06-17 via #2688 — ADR-069) | Done (Merged; awaiting release) |
| C — Headless background driver | 6 | 6/6 done (DSV-046 design Done 2026-07-04 — ADR-101; DSV-048 Merged 2026-07-04 via PR #3186; DSV-047 Merged 2026-07-04 via PR #3191; DSV-049 Merged 2026-07-05; DSV-050 Merged 2026-07-05 via PR #3200; DSV-051 Merged 2026-07-06 via PR #3205) | Done (all Merged; awaiting release) |
| **Total** | **26** | **26/26 done** | **Done (awaiting release evidence before Complete)** |
