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
- Cross-uid trust boundaries (the SO_PEERCRED same-uid boundary is the only one
  claimed)
- Windows named-pipe `validate_paths` GA (parity tracked separately)
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

- **Status:** In Progress
- **Scope note (2026-06-03):** the ingest-spine *components* land in this PR —
  `path_safety` (Task 3, openat2 dirfd read-safety), `change_class` (Task 4,
  inode classification), `assurance` taxonomy half (Task 5), and a standalone,
  unit-tested `workspace_admission::AdmittedRoots` (Task 2's per-connection
  admitted-root set). Task 2's `ipc.rs` per-connection threading + the live
  `validate_workspace_roots`/`authorise` caller (B7) are intentionally deferred
  to **DSV-005**: `validate_paths` is the admitted-root set's only real consumer,
  so wiring lands with that dispatch arm rather than as inert plumbing here. The
  item stays **In Progress** until that wiring merges.
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

- **Status:** In Progress
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

- **Status:** Ready
- **Intent:** Wire the verdict end to end and run the workspace assurance state
  machine + `workspace_status` / `request_full_scan` verbs.
- **Expected Outcome:** Orchestration runs auth → classify → guarded-bytes read →
  apply-delta → certify → antipattern check (on guarded bytes + the interactive pool)
  → coalesce → assurance; the lifecycle emits ADR-035 notification envelopes (machine
  fields on the tracing mirror only); client-supplied hashes are never trusted for a
  verdict.
- **Validation:** `cargo test -p eddacraft-anvil-intercept validate_paths assurance`.
- **Files:** `crates/anvil-intercept/src/{validate_paths,assurance}.rs`,
  `crates/anvil-intercept/src/ipc.rs`,
  `crates/anvil-checks/src/antipattern/check.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** DSV-004, DSV-006
- **Source:** subphase-a Tasks 8–9; council B6/B7/item 8.

---

#### DSV-006: Resource model — two-pool scheduler, DoS caps, SLO gate

- **Status:** In Progress
- **Intent:** Keep the interactive verdict path responsive under concurrent agents +
  background scans, and gate latency in CI.
- **Progress:** 10a (spine) delivered via PR #2253 — `crates/anvil-intercept/src/workspace_pool.rs`
  builds the two cooperating rayon pools (small interactive + background) from one
  per-host budget and adds the per-`WorktreeKey` in-flight admission token (the Task 8
  predecessor). 10b delivered via PR #2272 — the chunked-yield background-scan loop
  (`run_chunked_scan` + `ScanCancel`/`ScanOutcome`): the background scan checks a cancel
  flag at every chunk boundary so it hands cores back to interactive work within one
  chunk, with `processed` doubling as a resume offset. Remaining (Task 11/Task 16, gated
  on DSV-005): the parse-size + walk-depth DoS caps, and the `4 agents + 1 scan` SLO
  bench + CI gate.
- **Expected Outcome:** A small interactive `rayon::ThreadPool` (10a, spine — a Task 8
  predecessor) + a chunked-yield background pool (10b); per-workspace in-flight token;
  parse-size + walk-depth caps; a `validate_paths` warm-read + `4 agents + 1 scan`
  bench with a p95 SLO and a >80 ms queue-wait WARN wired as a CI gate.
- **Validation:** `cargo test -p eddacraft-anvil-intercept workspace_pool`;
  `cargo bench -p eddacraft-anvil-intercept ipc_roundtrip` (quiet box); CI gate fails
  on a synthetic regression.
- **Files:** `crates/anvil-intercept/src/workspace_pool.rs`,
  `crates/anvil-intercept/benches/ipc_roundtrip.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** None for the interactive-pool construction (10a — itself a DSV-005/Task-8 predecessor); **DSV-005 for the DoS caps (Task 11 also modifies `validate_paths.rs`) and the SLO bench (Task 16 depends on 8+10)**. The SLO gate is an ADR-061 §9 Phase-2 merge dependency.
- **Source:** subphase-a Tasks 10/11/16; RLB-002/-005/-008.

---

#### DSV-007: `watch` + MCP clients + status surface

- **Status:** Ready
- **Intent:** Make the user-facing surfaces thin daemon clients with a safe fallback.
- **Expected Outcome:** `watch` routes save-time validation to the daemon and falls
  back to a *scoped* (never `--all`) check on daemon absence/mid-session death,
  surfacing `unavailable{daemon-absent}` (never a truncated `clean`) and warning once
  per disconnect; MCP `anvil_validate_write` re-points with a byte-identical in-process
  fallback; `anvil status` renders `clean|stale|pending|running|unavailable` (+ reason,
  + `confined: N`).
- **Validation:** `cargo test -p eddacraft-anvil -- watch`; the MCP tool tests; status
  render tests.
- **Files:** `crates/anvil-cli/src/commands/{watch,status}.rs`,
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

- **Status:** Ready
- **Intent:** Prove the four delivery paths (watch+daemon, watch+fallback, MCP+daemon,
  MCP+fallback) return identical finding sets.
- **Expected Outcome:** An order-normalised golden parity test over a fixed corpus,
  `workspace_assurance` carved out, run as a gate.
- **Validation:** `cargo test -p eddacraft-anvil-intercept --test diagnostic_parity`.
- **Files:** `crates/anvil-intercept/tests/diagnostic_parity.rs`,
  `crates/anvil-intercept/tests/fixtures/parity-corpus/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** DSV-005, DSV-007
- **Source:** subphase-a Task 15; ADR-061 §8.

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
| A — Interim-cache `validate_paths` | 9 | 3/9 done | In Progress |
| A′ — GV2 hot-read swap | 1 | 0/1 done | Blocked |
| B — Warm-start persistence | 1 | 0/1 done | Blocked |
| **Total** | **11** | **3/11 done** | **In Progress** |
