# Symbol Call Graph

| ID    | Owner | Status   | Progress |
| ----- | ----- | -------- | -------- |
| GCALL | —     | In Progress | 7/7   |

**Last reviewed:** 2026-06-17 (created from the GCTX-014 `anvil_find_callers`
block. The warm graph carries **no symbol-level call edges**: the
`EdgeType::Calls` / `References` variants exist in `anvil-kernel-types`, but the
kernel symbol extractor never emits them and `FileSymbols` (the `apply_delta`
feed) carries only `symbols` / `imports` / `reexports`, so a true call graph
cannot be projected today. This module owns the **producer-side** substrate —
call-site extraction + resident call edges + a caller-traversal read API — that
GCTX consumes. It is **not** a GCTX item: GCTX projects egress DTOs over a graph
the daemon already holds, mirroring how GCTX consumes GV2.
Module **In Progress, 7/7** — all work items Merged; the module advances to
Complete on the release tag that includes the GCALL substrate.
**GCALL-001** design accepted as
[ADR-086](../decisions/086-symbol-call-graph-substrate.md) (Accepted, operator
2026-06-17; Merged via #2705) — the call-edge model, the `FileSymbols` `calls`
contract, the ADR-031 budget posture, and the PV-9 caller-egress posture.
**GCALL-002** (TS/JS extraction) **Merged via #2707**; **GCALL-003** (resident
`EdgeType::Calls` edges + `callers_of` read API) **Merged via #2708** (+ CALL-1
heuristic marker #2712); **GCALL-004** (Rust extraction) **Merged via #2711**;
**GCALL-005** (Python extraction) **Merged via #2733**; **GCALL-006** (save-time
call-lift latency gate) **Merged via #2735**; **GCALL-007** (caller-egress
privacy review verdict) **Merged via #2710**. The GCALL consumer **GCTX-014
`anvil_find_callers` Merged via #2715** over the GCALL-003 `callers_of` read API.)

## Purpose

Give Anvil a deterministic, save-time symbol-level **call graph** so consumers
can answer "who calls this symbol" without whole-repo rereads. The dependency
graph (file/import/reexport edges) is complete in GV2; call edges
(caller → callee at symbol granularity) were deliberately out of GV2 scope. This
module adds them on the same trusted substrate, within the ADR-031 save-time
latency budget, behind a caller-egress privacy review.

## Boundaries

**In scope:**

- Per-language call-site extraction (TS/JS first, then Rust, then Python) into
  the `FileSymbols` feed.
- Lifting `EdgeType::Calls` into the resident `SymbolGraph` via `apply_delta`.
- A bounded, depth-limited caller-traversal read API over the resident graph.
- Save-time hot-path budget validation and a caller-egress privacy review.

**Out of scope:**

- The assistant-facing tool surface (`anvil_find_callers`) — that is GCTX-014,
  which consumes this module's read API.
- Cross-process / cross-host call resolution.
- Dynamic-dispatch / runtime call resolution beyond static call-site analysis.

## Dependencies

GV2 (resident `SymbolGraph`, `apply_delta` feed, `anvil-graph-cache` per
ADR-064), `anvil-kernel-types` (`EdgeType::Calls` / `References`),
ADR-031 (save-time latency gate), the PV-9 context-egress privacy posture, and
the per-language scanners (TS/JS, Rust, `lang-python`).

## Work Items

#### GCALL-001: Call-graph substrate design + ADR

- **Status:** Merged 2026-06-17 via #2705 — design accepted as
  [ADR-086](../decisions/086-symbol-call-graph-substrate.md) (Accepted, operator
  2026-06-17); ratifies the call-edge model, the `FileSymbols` `calls` contract,
  the ADR-031 budget posture, and the PV-9 caller-egress posture that
  GCALL-002..007 build on.
- **Intent:** Settle the call-edge model and the contracts the rest of the
  module builds on before any extraction work begins.
- **Expected Outcome:** An accepted ADR (and any spec delta) defines the
  call-edge model, the `FileSymbols` extension contract for call sites, the
  save-time budget posture (ADR-031), and the caller-egress privacy posture
  (relationship to PV-9). Resolves overload/cycle/recursion semantics at the
  model level.
- **Validation:** ADR Accepted in `plans/decisions/DECISION-LOG.md`; design
  reviewed against ADR-031 and ADR-064 boundaries.
- **Files:** `plans/decisions/`, `docs/architecture/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** —

---

#### GCALL-002: TS/JS call-site extraction into `FileSymbols`

- **Status:** Merged 2026-06-17 via #2707 — landed the `CallSite` / `CalleeRef` /
  `LocalSymbolRef` types + the `serde(default) calls` channel on `FileSymbols` in
  `anvil-kernel-types`, and the TS/JS extractor's call-site pass (caller via
  `for_file_symbols` ordinals / `module_scope` synthetic node; callee export-name +
  `via_import` with alias reverse-map + namespace member; unresolved drop) per the
  GCALL-001 (ADR-086) v1 resolution contract. Consumed by GCALL-003 (#2708).
- **Intent:** Emit symbol-level call sites for the primary language so the feed
  carries call data alongside `symbols` / `imports` / `reexports`.
- **Expected Outcome:** The kernel extractor emits call-site edges for TS/JS into
  `FileSymbols`, with stable callee identity resolution for the cases the model
  (GCALL-001) admits.
- **Validation:** Fixture tests cover direct calls, method calls, imported-symbol
  calls, and unresolved call sites; extraction is deterministic.
- **Files:** `crates/anvil-kernel/`, `crates/anvil-kernel-types/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GCALL-001

---

#### GCALL-003: Resident call edges + caller read API

- **Status:** Merged 2026-06-17 via #2708 — lifts `FileSymbols.calls` into
  resident `EdgeType::Calls` edges (`re_resolve_calls` + `all_calls` accumulator)
  and the bounded `callers_of` read API on the GCALL-002 producer (#2707); the
  CALL-1 heuristic/partial honesty marker landed via #2712.
- **Intent:** Make extracted call sites queryable on the resident graph so a
  consumer can perform bounded caller traversal.
- **Expected Outcome:** `apply_delta` lifts `EdgeType::Calls` into the resident
  `SymbolGraph`; a read-only API returns bounded, depth-limited caller results
  (calling symbol identity, source file, distance, truncation metadata) with
  cycle/recursion handling. This is the substrate GCTX-014 consumes.
- **Validation:** Tests cover direct callers, transitive callers at bounded
  depth, recursion/cycles, and overload disambiguation over a fixture call graph.
- **Files:** `crates/anvil-graph-cache/`, `crates/anvil-intercept/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GCALL-001, GCALL-002

---

#### GCALL-004: Rust call-site extraction

- **Status:** Merged 2026-06-17 via #2711 — extends the GCALL-002 two-pass
  extractor to Rust in `rust.rs`: a parallel `spans` vec over Pass 1 symbols + a
  Pass 2 `extract_call_sites` walking `call_expression`s, with a `use`-derived
  binding table (alias reverse-map), `Self::`/`self.` → `Owner.method`
  resolution, and scoped paths as the namespace-member shape, per the ADR-086 v1
  contract.
- **Intent:** Extend call-site extraction to Rust.
- **Expected Outcome:** The Rust extractor emits call-site edges into
  `FileSymbols` consistent with the GCALL-001 model.
- **Validation:** Rust fixture tests cover direct, method/trait, and
  cross-module calls; deterministic.
- **Files:** `crates/anvil-kernel/`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** GCALL-001

---

#### GCALL-005: Python call-site extraction

- **Status:** Merged 2026-06-18 via #2733 — extends the PYLAN-002 Python
  extractor with a Pass 2 call-site walk in `python.rs`: a parallel `spans` vec
  over Pass 1 symbols + a `walk_calls` over `call` nodes, with an import-binding
  table
  (`from m import x [as y]` reverse-map, `import m` / `import a.b as c` module
  bindings), `self.`/`cls.` → `Owner.method` resolution, and bare-member fallback,
  per the ADR-086 v1 contract. Lift-side resolution is the language-agnostic
  GCALL-003 `re_resolve_calls`, so no graph-cache change is needed.
- **Intent:** Extend call-site extraction to Python.
- **Expected Outcome:** The Python extractor emits call-site edges into
  `FileSymbols` consistent with the GCALL-001 model.
- **Validation:** Python fixture tests cover direct, method, and imported-symbol
  calls; deterministic.
- **Files:** `crates/anvil-kernel/`
- **Confidence:** low
- **Priority:** Medium
- **Dependencies:** GCALL-001, lang-python

---

#### GCALL-006: Save-time hot-path budget validation

- **Status:** Merged 2026-06-18 via #2735 — added
  `crates/anvil-graph-cache/benches/call_lift.rs`
  (`harness=false`, mirroring the GV2-025 `hot_read` gate): an in-process bench
  timing the two call-graph save-time ops — `update_file` (the full per-save
  apply incl. `lift_calls_tracked`) and the daemon `re_resolve_calls`
  forward-reference pass — over a 100-function / ~300-call corpus that lifts real
  resident `Calls` edges, exiting non-zero on an 80 ms p95 breach. Wired into
  `resource-budget.yml` as a gate step + a `ANVIL_BENCH_CALLLIFT_STALL_MS`
  self-test proving the gate trips on a synthetic regression. The corpus is
  padded to ~50k resident nodes (representative of a mid-large workspace, since
  `resolve_import` scans the whole graph per cross-file callee); measured p95
  ~6 ms (~13× under budget), ~9 ms even at 100k nodes.
- **Intent:** Prove call extraction stays within the save-time latency budget.
- **Expected Outcome:** Call-site extraction + `apply_delta` lift run within the
  ADR-031 save-time p95 budget on the benchmark corpus; a CI latency gate guards
  it (mirroring the existing save-time gate pattern).
- **Validation:** `harness=false` bench + exit-code gate in the resource-budget
  workflow stays green at the ADR-031 threshold.
- **Files:** `crates/anvil-graph-cache/`, `.github/workflows/resource-budget.yml`
- **Confidence:** low
- **Priority:** High
- **Dependencies:** GCALL-002, GCALL-003

---

#### GCALL-007: Caller-egress privacy review

- **Status:** Merged 2026-06-17 via #2710 — verdict filed
  ([2026-06-17 caller-egress privacy review](../reviews/2026-06-17-gcall-caller-egress-privacy-review-verdict.md),
  APPROVE-WITH-CONDITIONS): caller egress is identity-only and equivalent-risk to
  the PV-9-approved `find_dependents`; conditions CALL-1..CALL-5 folded into
  GCTX-014 (Merged #2715). The one new substrate condition (CALL-1,
  heuristic/partial honesty markers) landed in GCALL-003 via #2712.
- **Intent:** Settle the privacy posture for exposing "who calls this" before any
  assistant-facing surface ships it.
- **Expected Outcome:** A council/privacy review (PV-style, modelled on PV-9)
  approves the caller-egress surface, confirming identity-only default and the
  redaction choke point; conditions folded into the GCTX-014 acceptance criteria.
- **Validation:** Verdict filed under `plans/reviews/`; conditions reflected in
  GCTX-014.
- **Files:** `plans/reviews/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GCALL-003
