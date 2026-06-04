# Graph v2 Foundation

| ID  | Owner | Status |
| --- | ----- | ------ |
| GV2 | —     | Draft  |

**Last reviewed:** 2026-06-04

## Purpose

Give Anvil a persistent, joined structural model of code, trust, control, and
provenance so it can make deterministic safety decisions at the point of
change. Assistant context delivery is an optional projection over the same
model, not the reason the model exists.

**Why:** Anvil's current Rust kernel graph is a strong H1 seed: it tracks
symbols, imports, trust metadata, and incremental file updates in memory. The
next product claims need more than that. The intercept daemon and driver
framework need warmed constant-time graph reads; provenance needs stable
identity across commits and plans; behavioural diff needs comparable graph
snapshots; agents need reliable graph context without becoming the primary
architecture driver. Graph v2 is the shared substrate that prevents INTD, DRVR,
WEAVE, GCTX, trust policy, and provenance work from each inventing partial graph
models.

**North star:** Graph v2 is an Anvil control and provenance primitive first. MCP
adoption and better agent results are valuable secondary effects because agents
can tap the same deterministic model Anvil already trusts.

## In Scope

- Multi-graph taxonomy with explicit joins, not one mega-graph
- Stable identity for files, symbols, edges, sessions, plans, and provenance
  anchors
- Complete delta/event contract for graph updates and downstream consumers
- Semantic code graph v2 schema for symbols, imports, calls, references,
  exports, source spans, and language metadata
- Dependency/impact graph and hot-path indexes for boundary membership, symbol
  ownership, known-edge existence, and architectural index checks
- Trust/policy graph contract for trust levels, side-effect surfaces, data
  classifications, invariant guards, and policy evidence
- Control/session graph contract for hosts, drivers, sessions, leases, fences,
  worktrees, and attribution
- Plan/provenance graph contract joining APS work items, commits, memory
  provenance, and trust posture changes
- Persistence and snapshot strategy for graph state that remains derivable from
  source and safe to discard/rebuild
- Typed query traits usable by the daemon, drivers, MCP server, and future
  `anvil-weave` harness

## Out of Scope

- Generic graph database product work
- Community detection, clustering, or embedding search
- Visual graph UI surfaces; dashboard modules own visualisation
- Cross-repo graph registry beyond stable per-repo identity hooks
- Expensive transitive analysis on the daemon hot path
- Replacing APS, Edda, or Kindling data stores
- Making MCP the primary control plane
- Full interprocedural data-flow analysis in this module

## Interfaces

**Depends on:**

- `anvil-kernel` — current `SymbolGraph`, `DependencyGraph`, watcher,
  incremental update pipeline, and trust annotation pass
- `anvil-kernel-types` — current `SymbolNode`, `SymbolEdge`, `EdgeType`, and
  public type boundary
- `anvil-intercept` / INTD — daemon control authority and change provenance
  contracts when implementation lands
- `surface-drivers` / DRVR — editor and MCP driver contracts
- `edda-stack` — provenance and evolution graph concepts already delivered in
  EDDA/STACK
- APS modules — plan/work-item metadata for plan/provenance joins
- ADR-015, ADR-030, ADR-031 — intercept, driver, and latency constraints

**Exposes:**

- `docs/architecture/graph-v2-foundation-spec.md` — canonical Graph v2
  taxonomy, joins, and query boundaries
- `anvil-kernel-types` Graph v2 schema additions
- `anvil-kernel::graph` registry/query traits
- Hot-path read API for daemon/driver enforcement use
- Snapshot and delta contract consumed by downstream projection layers
- Query contract for GCTX, DRVR, and WEAVE consumers

## Constraints

- UK English spelling in all plan text and user-facing docs
- Graph state is derivable cache state unless an ADR explicitly marks a field as
  authoritative
- Hot-path queries must be constant-time or near-constant-time and cite
  ADR-031 boundaries when claiming latency
- Expensive traversal, enrichment, explanation, and slicing must stay off the
  daemon hot path
- Graph updates must be deterministic for identical source/config inputs
- Stored graph snapshots must version their schema and rebuild safely on
  mismatch
- MCP/agent query surfaces must not define Graph v2 schema requirements; they
  consume projections
- Privacy-sensitive provenance fields must be minimised and documented before
  persistence

## Prerequisites

- KERN H1 graph, trust metadata, and incremental update work complete
- ADR-015 intercept-loop enforcement accepted as the control-plane direction
- ADR-030 surface-driver pivot accepted as the VSCode/MCP migration direction
- ADR-031 latency rubric available for hot-path budget references
- EDDA provenance and evolution schemas complete for join alignment

## Ready Checklist

Change status to **Ready** when:

- [ ] Graph taxonomy accepted by architecture review
- [x] Hot-path/non-hot-path boundary agreed with INTD and DRVR owners —
      ratified in [ADR-063](../decisions/063-gv2-hot-path-boundary.md) (Accepted
      2026-06-01, Josh as sole owner of the INTD, DRVR, and GV2 surfaces)
- [ ] Stable identity model reviewed against git rename and symbol rename cases
- [x] Persistence strategy ADR drafted or explicitly assigned to GV2-021 —
      [ADR-069](../decisions/069-graph-v2-persistence.md) **Accepted 2026-06-04**,
      formalising the
      `plans/specs/2026-06-01-daemon-save-time-validation-contract.md` §9
      requirements (full council + design council, SOUND-WITH-FIXES, folded in)
- [ ] Privacy review completed for persisted provenance/session fields
- [x] GCTX module updated to depend on GV2 rather than owning foundation work —
      [graph-context-delivery](graph-context-delivery.aps.md) declares GV2 as a
      dependency and lists schemas, stable IDs, deltas, hot indexes, and
      persistence as GV2-owned (out of GCTX scope)
- [ ] Validation commands for the first implementation slice are concrete

> **ADR-061 (Accepted 2026-06-01, council `plan-5768ae0c`) is the consuming
> save-time contract** for the GV2 hot-read slice (GV2-010/011/020/022). ADR-061
> deliberately left the "Hot-path/non-hot-path boundary agreed with INTD and DRVR
> owners" gate open; **[ADR-063](../decisions/063-gv2-hot-path-boundary.md)
> (Accepted 2026-06-01) now closes it** — it pins the hot-path admission
> invariant, the read allowlist, the denylist, and the miss/stale policy across
> the INTD, DRVR, and GV2 surfaces, ratifies the boundary checkbox above, and
> clears GV2-022 to freeze. Sub-phase A′ is therefore no longer decision-blocked;
> it now depends on **implementing** the GV2 hot-read slice (GV2-010/011/020/022,
> all still Draft) under the frozen `validate_paths` wire. GV2-021's
> persistence/privacy/crash-safety content is specified concretely in
> `plans/specs/2026-06-01-daemon-save-time-validation-contract.md` §9 (warm-start
> restores indexes, never the verdict; default-off; per-uid owner-only snapshot
> location; structural-identity-only privacy line); sub-phase B remains blocked
> on the GV2-021 ADR itself being drafted.

---

## Work Items

### Phase 0 — Architecture and Contracts

#### GV2-001: Graph v2 architecture spec and taxonomy

- **Status:** Draft
- **Intent:** Define the joined-graph model Anvil will use for enforcement,
  provenance, driver attribution, behavioural diff, and assistant projections.
- **Expected Outcome:** Architecture spec describes the semantic code graph,
  dependency/impact graph, trust/policy graph, control/session graph, and
  plan/provenance graph, including what each graph owns and how joins work.
- **Validation:** Council review pass; spec cross-references ADR-015, ADR-030,
  ADR-031, and GCTX
- **Files:** `docs/architecture/graph-v2-foundation-spec.md`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

#### GV2-002: Stable graph identity model

- **Status:** Draft
- **Intent:** Provide stable identifiers for files, symbols, edges, sessions,
  plans, and provenance anchors so graph snapshots and deltas remain comparable
  across edits, renames, commits, and daemon restarts.
- **Expected Outcome:** Identity contract covers content hashes, path identity,
  symbol identity, edge identity, session/worktree identity, and APS/provenance
  references, with documented rename behaviour.
- **Validation:** Unit tests for file rename, symbol rename, delete/recreate, and
  same-name symbols in different scopes
- **Files:** `crates/anvil-kernel-types/src/graph.rs`,
  `docs/architecture/graph-v2-foundation-spec.md`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-001

---

#### GV2-003: Complete graph delta and event contract

- **Status:** Draft
- **Intent:** Make graph changes observable as complete, deterministic deltas
  that downstream enforcement, provenance, persistence, and projection consumers
  can trust.
- **Expected Outcome:** Delta contract includes added/removed/changed nodes,
  added/removed/changed edges, affected files, identity anchors, content hashes,
  provenance metadata, and schema version.
- **Validation:** Property test confirms full rebuild and replayed deltas produce
  equivalent observable graph state
- **Files:** `crates/anvil-kernel/src/graph/incremental.rs`,
  `crates/anvil-kernel-types/src/graph.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-002

---

### Phase 1 — Graph Schemas

#### GV2-010: Semantic code graph v2 schema

- **Status:** Draft
- **Intent:** Expand the current symbol/import graph into the canonical semantic
  code graph used by Anvil's safety checks and downstream graph projections.
- **Expected Outcome:** Schema models files, modules, symbols, imports, exports,
  calls, references, source spans, language metadata, visibility, and stable
  identity without requiring full AST persistence.
- **Validation:** Snapshot fixtures for TS/JS plus one future-language fixture
  show deterministic node and edge output
- **Files:** `crates/anvil-kernel-types/src/graph.rs`,
  `crates/anvil-kernel/src/parser/extract.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-002, GV2-003

---

#### GV2-011: Dependency/impact graph and hot indexes

- **Status:** Draft
- **Intent:** Define derived file/module dependency state and the warmed indexes
  the daemon may read on the hot path.
- **Expected Outcome:** Boundary membership lookup, symbol ownership lookup,
  known-edge existence, and precomputed architectural index checks are exposed as
  bounded reads; transitive impact traversal remains explicitly non-hot-path.
- **Validation:** Criterion benchmark demonstrates the hot reads meet ADR-031
  component budgets on the canonical fixture corpus
- **Files:** `crates/anvil-kernel/src/graph/dependency.rs`,
  `crates/anvil-kernel/src/graph/symbol_graph.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GV2-010

---

#### GV2-012: Trust and policy graph contract

- **Status:** Draft
- **Intent:** Separate trust/policy semantics from the raw semantic graph while
  preserving deterministic joins back to code evidence.
- **Expected Outcome:** Contract represents trust level, side-effect surfaces,
  data classifications, invariant guards, policy evidence, and override sources
  without forcing full interprocedural data-flow analysis.
- **Validation:** Fixtures show trust posture changes are emitted as graph deltas
  and policy evidence resolves back to source spans
- **Files:** `crates/anvil-kernel/src/graph/trust.rs`,
  `crates/anvil-kernel-types/src/graph.rs`,
  `docs/architecture/graph-v2-foundation-spec.md`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-010

---

#### GV2-013: Control and session graph contract

- **Status:** Draft
- **Intent:** Model execution hosts, drivers, sessions, leases, fences,
  worktrees, and attribution as a graph that can join to code changes without
  making MCP the control plane.
- **Expected Outcome:** Contract aligns with INTD and DRVR session/driver
  models and identifies which fields are hot-path, telemetry-only, or
  persisted-for-provenance.
- **Validation:** Design review against INTD and DRVR specs; contract covers
  shell, editor, and MCP driver cases
- **Files:** `docs/architecture/graph-v2-foundation-spec.md`,
  `plans/specs/anvil-driver-framework/anvil-driver-framework-design-spec.md`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GV2-001

---

#### GV2-014: Plan and provenance graph contract

- **Status:** Draft
- **Intent:** Join APS intent, git history, Edda provenance, graph deltas, and
  trust posture changes so Anvil can explain why a structural change was allowed
  or challenged.
- **Expected Outcome:** Contract maps work items, commits, change events,
  memories, policy decisions, and graph-state changes without making APS a
  runtime prerequisite.
- **Validation:** Fixture trace links one code change to an APS item, commit,
  graph delta, policy result, and provenance record
- **Files:** `docs/architecture/graph-v2-foundation-spec.md`,
  `packages/edda-stack/src/contracts/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GV2-003

---

### Phase 2 — Runtime Substrate

#### GV2-020: Multi-graph registry and typed query traits

- **Status:** Draft
- **Intent:** Provide one typed in-process entry point for querying joined graph
  state without coupling consumers to storage or `petgraph` internals.
- **Expected Outcome:** Registry exposes graph handles and join queries for
  semantic, dependency, trust, control, and provenance graphs; consumers depend
  on traits rather than concrete storage.
- **Validation:** Kernel unit tests exercise each graph handle and one join query
  across code/trust/provenance state
- **Files:** `crates/anvil-kernel/src/graph/mod.rs`,
  `crates/anvil-kernel/src/graph/registry.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-010, GV2-011, GV2-012, GV2-013, GV2-014

---

#### GV2-021: ADR — graph persistence and snapshot strategy

- **Status:** In Progress
- **Intent:** Record the persistence decision for Graph v2, reconciling the
  current GCTX rkyv/SQLite assumptions with daemon, hot-read, privacy, and
  schema-version requirements.
- **Expected Outcome:** ADR defines default snapshot format, rebuild behaviour,
  crash-safety expectations, multi-process reader stance, privacy boundaries,
  and migration/versioning rules.
- **Validation:** ADR reviewed by council-reviewer, kernel-maintainer, and
  security reviewer
- **Files:** `plans/decisions/069-graph-v2-persistence.md`
- **Progress:** [ADR-069](../decisions/069-graph-v2-persistence.md) **Accepted
  2026-06-04** (Josh) — sealed canonical-DTO load-once snapshot (`postcard`, not
  rkyv/SQLite/CBOR), default-off, restore-indexes-never-verdict, discard-and-rebuild
  versioning, structural-identity-only privacy line; reviewed by full council +
  design council (SOUND-WITH-FIXES, all folded in). Item completes (→ Merged) on
  PR [#2301](https://github.com/eddacraft/anvil-001/pull/2301) merge.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-001, GV2-003

---

#### GV2-022: Hot-path read API and latency guardrails

- **Status:** Draft
- **Intent:** Expose the specific warmed reads the daemon and drivers may use
  during save-time or mid-edit enforcement without allowing expensive graph work
  onto the hot path.
- **Expected Outcome:** API offers boundary membership, symbol ownership,
  known-edge existence, and architectural index checks with explicit stale/warm
  states and fallback behaviour.
- **Validation:** Benchmarks cite ADR-031 boundaries and fail when p95 exceeds
  the accepted budget
- **Files:** `crates/anvil-kernel/src/graph/hot_index.rs`,
  `crates/anvil-kernel/src/graph/mod.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GV2-011, GV2-020

---

#### GV2-023: Consumer query contract for daemon, drivers, MCP, and weave

- **Status:** Draft
- **Intent:** Define the graph query boundary that downstream consumers use so
  GCTX, DRVR, INTD, and WEAVE do not grow incompatible graph adapters.
- **Expected Outcome:** Query contract separates enforcement reads, diagnostic
  reads, provenance reads, and context projection reads; MCP/assistant queries
  are explicitly projections over the same trusted substrate.
- **Validation:** Contract review with GCTX, DRVR, INTD, and WEAVE plan owners;
  each consumer has at least one mapped query scenario
- **Files:** `docs/architecture/graph-v2-foundation-spec.md`,
  `plans/modules/graph-context-delivery.aps.md`,
  `plans/modules/surface-drivers.aps.md`,
  `plans/modules/weave.aps.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** GV2-020, GV2-022

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Graph v2 becomes a generic graph database project | Medium | High | Keep scope tied to enforcement, provenance, hot reads, and deterministic safety decisions |
| MCP/context needs over-shape the foundation | Medium | High | GCTX depends on GV2 and consumes projections; schema authority stays with GV2 |
| Hot path accidentally includes expensive traversal | Medium | High | GV2-011/GV2-022 explicitly split hot indexes from non-hot-path traversal and cite ADR-031 |
| Stable identity is wrong for rename-heavy work | Medium | Medium | GV2-002 includes rename/delete/recreate validation cases before implementation |
| Persisted session/provenance data captures too much private context | Medium | High | Privacy review in Ready checklist; minimise persisted fields and mark derivable cache state |
| Multiple graph layers feel too complex for contributors | Medium | Medium | GV2-001 owns taxonomy and examples; query traits hide storage/layout details from consumers |

## Decisions (Initial)

1. **Multiple joined graphs, not a mega-graph** — semantic code, dependency,
   trust/policy, control/session, and plan/provenance state have different
   lifecycles, privacy concerns, and latency needs.
2. **Anvil-first** — Graph v2 is justified by prevention, enforcement,
   provenance, and trust. Assistant context is a projection and a product
   accelerant, not the foundation's reason to exist.
3. **Hot indexes over hot traversal** — daemon/driver enforcement may read warm
   indexes, but full recompute, transitive analysis, explanation, and context
   slicing stay outside the hot path.
4. **Derivable by default** — persisted graph snapshots are cache state unless a
   future ADR explicitly makes a field authoritative.
5. **Planless-first preserved** — plan/provenance joins enrich Anvil when APS is
   present, but Graph v2 must still work from source/config alone.

## Stats

| Phase | Items | Completion | Status |
| ----- | ----- | ---------- | ------ |
| 0 — Architecture and Contracts | 3 | 0/3 done | Draft |
| 1 — Graph Schemas | 5 | 0/5 done | Draft |
| 2 — Runtime Substrate | 4 | 0/4 done | Draft |
| **Total** | **12** | **0/12 done** | **Draft** |
