# Symbol Call Graph

| ID    | Owner | Status   | Progress |
| ----- | ----- | -------- | -------- |
| GCALL | —     | In Progress | 2/7   |

**Last reviewed:** 2026-06-17 (created from the GCTX-014 `anvil_find_callers`
block. The warm graph carries **no symbol-level call edges**: the
`EdgeType::Calls` / `References` variants exist in `anvil-kernel-types`, but the
kernel symbol extractor never emits them and `FileSymbols` (the `apply_delta`
feed) carries only `symbols` / `imports` / `reexports`, so a true call graph
cannot be projected today. This module owns the **producer-side** substrate —
call-site extraction + resident call edges + a caller-traversal read API — that
GCTX consumes. It is **not** a GCTX item: GCTX projects egress DTOs over a graph
the daemon already holds, mirroring how GCTX consumes GV2. Module **In Progress,
2/7**: GCALL-001 design accepted as
[ADR-086](../decisions/086-symbol-call-graph-substrate.md) (Accepted, operator
2026-06-17; Merged via #2705) — the call-edge model, the `FileSymbols` `calls`
contract, the ADR-031 budget posture, and the PV-9 caller-egress posture.
**GCALL-002** (TS/JS call-site extraction) **Merged 2026-06-17 via #2707** — the
`CallSite` / `CalleeRef` / `LocalSymbolRef` types + the `calls` channel on
`FileSymbols` + the TS/JS extractor's call-site pass (alias reverse-map,
namespace member, module-scope/synthetic caller, unresolved drop). **GCALL-003**
(resident edges + read API) is the next pick — In Progress (#2708); GCALL-004/005
(Rust/Python extraction) also unblock on GCALL-001.)

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

- **Status:** In Progress — lifting `FileSymbols.calls` into resident
  `EdgeType::Calls` edges (`re_resolve_calls` + `all_calls` accumulator) and the
  bounded `callers_of` read API, on the GCALL-002 producer (merged via #2707;
  its module-status flip is pending post-merge cleanup).
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

- **Status:** Proposed
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

- **Status:** Proposed
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

- **Status:** Proposed
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

- **Status:** In Progress — verdict authored
  ([2026-06-17 caller-egress privacy review](../reviews/2026-06-17-gcall-caller-egress-privacy-review-verdict.md),
  APPROVE-WITH-CONDITIONS): caller egress is identity-only and equivalent-risk to
  the PV-9-approved `find_dependents`; conditions CALL-1..CALL-5 folded into
  GCTX-014. The one new condition (CALL-1, heuristic/partial honesty markers)
  carries a GCALL-003 substrate follow-up.
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
