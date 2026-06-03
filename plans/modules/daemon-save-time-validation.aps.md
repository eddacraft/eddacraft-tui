# Daemon Save-time Validation

| ID  | Owner | Status      |
| --- | ----- | ----------- |
| DSV | Josh  | In Progress |

**Last reviewed:** 2026-06-03

## Purpose

Deliver Anvil's save-time validation as a daemon-mediated service across its
planned sub-phases, behind one frozen `validate_paths` wire. The intercept daemon
certifies a set of changed paths against a warm per-`WorktreeKey` graph cache and
returns a verdict-shaped envelope; `watch` and the MCP `anvil_validate_write` tool
become thin daemon clients with a scoped (never `--all`) fallback.

**Why:** this work has grown past a single work item. It spans ~6 crates, a new
crate extraction (ADR-064), three ADRs (061/063/064), and three sequenced
sub-phases. It was previously tracked only as the interim-backing item
[MLP2-067](multilayer-protection-v2.aps.md) plus an execution plan, which
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
  [holistic re-review verdict](../reviews/2026-06-02-b-corrections-holistic-verdict.md)).
  Action plan:
  [`execution/2026-06-01-daemon-save-time-subphase-a.md`](../execution/2026-06-01-daemon-save-time-subphase-a.md)
  (Tasks 0–17).
- **Sub-phase A′ — GV2 hot-read swap.** Replace the interim cache with the GV2
  resident warm-index slice (GV2-010/011/020/022) under the unchanged wire.
  Blocked on the GV2 hot-/non-hot-path boundary gate.
- **Sub-phase B — warm-start persistence.** Add a default-off, per-uid,
  owner-only snapshot that restores graph indexes (never the verdict) on daemon
  restart, per the validation contract §9. Blocked on the GV2-021 persistence ADR.

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

- `anvil-graph-cache` (net-new, [ADR-064](../decisions/064-intercept-graph-cache-crate-boundary.md))
  — `SymbolGraph`, `DependencyGraph`, incremental apply-delta, `certify`
- `anvil-intercept` / INTD — daemon transport, SO_PEERCRED handshake, IPC
  dispatch; [`intercept-daemon`](../archive/modules/intercept-daemon.aps.md) is
  archived Complete, so daemon integration debt lives here and in MLP2
- `anvil-intercept-proto` — the shared `DiagnosticEnvelope` (B3, landed) and the
  frozen method constants
- `anvil-checks` — `run_antipattern_check`; B7 adds a guarded-bytes + injected-pool
  entrypoint
- [GV2](graph-v2-foundation.aps.md) — GV2-010/011/020/022 (hot-read slice, A′) and
  GV2-021 (persistence ADR, B)
- [MLP2-067](multilayer-protection-v2.aps.md) — the originating interim-backing
  item, now delivered here as Sub-phase A
- [RLB](resource-load-benchmarking.aps.md) — RLB-002/-005/-008 resource model + SLO
- [DRVR](../archive/modules/surface-drivers.aps.md) — MCP `anvil_validate_write` re-point (archived Complete)
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
  [`2026-06-02-b-corrections-holistic-verdict.md`](../reviews/2026-06-02-b-corrections-holistic-verdict.md))
- For A′: the GV2 hot-/non-hot-path boundary gate agreed with INTD/DRVR owners
- For B: the GV2-021 persistence ADR accepted

## Ready Checklist

Sub-phase A is **Ready** (execution authorised, GO-WITH-CONDITIONS). A′ and B stay
**Blocked** until their GV2 gates clear.

- [x] Architecture decided and ADRs accepted (061/063/064)
- [x] Council review passed (do-not-start blockers resolved; holistic re-review GO-WITH-CONDITIONS)
- [x] Sub-phase A action plan exists with concrete validation commands per task
- [x] Crate-boundary predecessor identified and scoped (DSV-001 / Task 0)
- [ ] (A′) GV2 hot-/non-hot-path boundary agreed with INTD and DRVR owners
- [ ] (B) GV2-021 persistence ADR accepted

## Work Items

### Sub-phase A — interim-cache `validate_paths`

#### DSV-001: Extract `eddacraft-anvil-graph-cache` (ADR-064 / B5)

- **Status:** Merged 2026-06-03 via PR #2254
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

- **Status:** Merged 2026-06-03 via PR #2252
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

- **Status:** Merged 2026-06-03 via PR #2264
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

- **Status:** Merged 2026-06-03 via PR #2273
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

- **Status:** Merged 2026-06-03 via PR #2282
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
     [ADR-067](../decisions/067-daemon-symbol-feed-parse-hook.md)): the daemon
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

- **Status:** Merged 2026-06-03 via PR #2283
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

- **Status:** Merged 2026-06-03 via PR #2284
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
  `unavailable{daemon-absent}`). Wired into `watch.rs`'s `run_one_action`, **opt-in via
  `ANVIL_WATCH_DAEMON`** (default-off so the not-yet-auto-started daemon does not change
  default watch behaviour — trunk-releasable, per the release-gating model).
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

- **Status:** Merged 2026-06-03 via PR #2275
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

- **Status:** Merged 2026-06-04 via PR #2294
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
closed in PR #2291). Windows is the real gap: the save-time verbs are not served
and the clients are `cfg(not(unix))` stubs. DSV-010/011 (Proposed 2026-06-04,
drafted as a DSV-007 follow-up) bring Windows to parity; DSV-010 carries an open
read-safety design risk that likely needs an ADR before it goes Ready.

#### DSV-010: Windows named-pipe save-time daemon

- **Status:** Proposed
- **Intent:** Serve the frozen save-time verbs on Windows so a Windows project gets
  the same daemon-mediated save-time validation as Unix. ADR-015 mandates Windows
  support, and the IPC transport already speaks named pipes (MLP2-075 wired the MCP
  `scan_buffer` / protection-claim Windows client).
- **Expected Outcome:** `validate_paths` / `workspace_status` / `request_full_scan`
  are dispatched and answered on Windows over the per-user named pipe; `save_time.rs`
  (today `#![cfg(unix)]`) and its read path are lifted to a cross-platform boundary;
  same-user peer authorisation via the named-pipe ACL / `pipe_name_for_current_user`
  (the SO_PEERCRED equivalent); the frozen wire and verdict semantics are unchanged.
- **Open design risk (resolve before Ready):** the verdict's read-safety guard
  (`path_safety.rs` — an `openat2` + `RESOLVE_BENEATH` held dirfd) has **no Windows
  analogue**. A Windows guarded read that preserves the "daemon reads the exact bytes
  it certifies, no symlink/junction escape, no TOCTOU" contract (B2 / security C2/C3)
  needs a design decision — likely an ADR (`NtCreateFile` with reparse-point controls
  / handle-based reads, or a documented weaker guarantee). This is the gating unknown;
  the rest is mechanical parity.
- **Validation:** a Windows IPC fixture round-trip (mirroring the MLP2-075 `windows_*`
  tests) proving the three verbs answer over a per-PID pipe; the cross-path parity gate
  (DSV-009) extended to a Windows path.
- **Files:** `crates/anvil-intercept/src/{save_time,path_safety,ipc}.rs`,
  `crates/anvil-intercept-win32/`.
- **Confidence:** low — gated on the read-safety design decision above.
- **Priority:** High (short-term-supported target).
- **Dependencies:** DSV-005; an accepted Windows read-safety ADR.
- **Source:** DSV-007 follow-up (macOS + Windows are short-term save-time targets);
  ADR-015; brainstorms `2026-05-01-hearth-rearchitecture.md` /
  `2026-05-07-daemon-sessions-surfaces-boundaries.md`.

#### DSV-011: Windows `watch` + `status` save-time clients

- **Status:** Proposed
- **Intent:** Make the Windows user-facing surfaces thin save-time-daemon clients,
  matching the Unix `watch` / `status` wiring shipped in DSV-007.
- **Expected Outcome:** a `WindowsPipeSaveTimeTransport` (parallel to the MCP
  `WindowsPipeDaemonValidationClient`) backs the `cfg(not(unix))` stubs in
  `watch_save_time.rs` (`query_workspace_status`, `build_save_time_client`), so
  `watch` routes save-time validation and `anvil status` renders the assurance
  surface on Windows — under the same opt-in (`ANVIL_WATCH_DAEMON`) gate and scoped
  fallback as Unix.
- **Validation:** the watch socket round-trip + status render tests extended to a
  Windows named-pipe fixture (mirroring the MLP2-075 Windows test pattern).
- **Files:** `crates/anvil-cli/src/commands/{watch_save_time,watch,status}.rs`.
- **Confidence:** medium — mechanical once DSV-010 serves the verbs on Windows.
- **Priority:** High (short-term-supported target).
- **Dependencies:** DSV-010.
- **Source:** DSV-007 follow-up; ADR-015.

---

### Sub-phase A′ — GV2 hot-read swap

#### DSV-020: Swap the GV2 hot-read slice under the frozen wire

- **Status:** Blocked
- **Intent:** Replace the interim `SymbolGraph` cache with GV2 resident warm indexes
  behind the unchanged `validate_paths` wire, so the verdict reads GV2's hot-path API
  instead of the rebuild-on-restart interim cache.
- **Expected Outcome:** The daemon backing is the GV2 hot-read slice; the wire,
  `check_families` scoping, and parity gate are unchanged; latency stays within the
  ADR-031 budget.
- **Validation:** the Sub-phase A parity + SLO gates stay green with the GV2 backing;
  criterion hot-read benchmark meets ADR-031.
- **Confidence:** low
- **Priority:** Medium
- **Dependencies:** GV2-010, GV2-011, GV2-020, GV2-022; the GV2 hot-/non-hot-path
  boundary gate; DSV-005
- **Blocked reason:** the GV2 hot-/non-hot-path boundary gate is not yet agreed with
  INTD/DRVR owners (GV2 Ready Checklist), and GV2-010/011/020/022 are not done.
- **Source:** ADR-061 sub-phase A′; ADR-063.

---

### Sub-phase B — warm-start persistence

#### DSV-030: Warm-start persistence for the daemon graph cache

- **Status:** Blocked
- **Intent:** Let the daemon restore graph indexes (not verdicts) on restart so a fresh
  connection is not `Stale` until a full scan completes.
- **Expected Outcome:** A default-off, per-uid, owner-only snapshot location;
  warm-start restores **indexes only, never the verdict**; structural-identity-only
  privacy line per the validation contract §9; crash-safe.
- **Validation:** restart-restores-indexes test; default-off assertion; the verdict is
  still re-derived from bytes after warm-start.
- **Confidence:** low
- **Priority:** Low
- **Dependencies:** GV2-021; DSV-020
- **Blocked reason:** the GV2-021 persistence/snapshot ADR is not yet accepted.
- **Source:** ADR-061 §9; validation contract §9; GV2-021.

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
| A′ — GV2 hot-read swap | 1 | 0/1 done | Blocked |
| B — Warm-start persistence | 1 | 0/1 done | Blocked |
| **Total** | **11** | **9/11 done** | **In Progress** |
