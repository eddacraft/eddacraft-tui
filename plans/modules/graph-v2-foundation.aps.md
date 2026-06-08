# Graph v2 Foundation

| ID  | Owner | Status |
| --- | ----- | ------ |
| GV2 | —     | In Progress |

**Last reviewed:** 2026-06-08

> **A′ slice in the `v0.8.0-beta` window (2026-06-08, [ADR-075](../decisions/075-v080-graph-product-scope.md),
> Accepted via council).** In-window = the **GV2-027 critical-path closure**:
> GV2-010, 011, 012, 022, 024, 025, 028, 029, then the A→A′ swap (027).
> **GV2-010 Merged 2026-06-08 (PR #2419)**, **GV2-011 Merged 2026-06-08
> (PR #2428)**, and **GV2-028 Merged 2026-06-08 (parser feed — shipped under
> DSV-005 PR #2282, ratified + watch-path proof added here)**; the now-unblocked
> frontier is **GV2-012** (dep GV2-010 ✓) plus **GV2-022, GV2-024** (dep
> GV2-011 ✓) — promote at pickup. The rest stay dep-blocked along the chain.
> **Deferred to v0.9** (council, off the critical path): GV2-013, 014, 020, 023,
> 026 (registry/contracts) and GV2-030 (sealed-DTO snapshot, with Sub-phase B
> persistence). 013/014 are dep-unblocked but stay Draft as v0.9 scope. Count is
> **7/19** (001/002/003/010/011/021/028 Merged).

> **Reshaped 2026-06-08** around the now-landed spine spec
> [`docs/architecture/graph-v2-foundation-spec.md`](../../docs/architecture/graph-v2-foundation-spec.md)
> (GV2-001, merged via PR #2350) and the
> [2026-06-05 wave planning-council verdict](../reviews/2026-06-05-gv2-wave-planning-council-verdict.md).
> Three owner decisions are folded in: **grow the wave** (GV2-012/013/014 pulled
> in so GV2-020 is the full multi-graph registry), **graduate GV2-002** (real
> stable identity + export-diff, in-wave critical), and **claim privilege
> containment** (wire `annotate_trust` on the daemon certify path — GV2-029,
> which blocks the A′ swap). All `Files:` paths are re-grounded onto
> `crates/anvil-graph-cache/` per [ADR-064](../decisions/064-intercept-graph-cache-crate-boundary.md);
> the old `crates/anvil-kernel/src/graph/` tree no longer exists. Several Phase 0/1
> items are reframed from "build" to "ratify what shipped under Sub-phase A +
> close the named residual gaps".

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

- Multi-graph taxonomy with explicit joins, not one mega-graph — pinned in the
  spine spec
- Stable, cross-restart identity for files, symbols, edges, sessions, plans, and
  provenance anchors, with an export-diff primitive precise enough to graduate
  the conservative save-time `partial` default
- Complete delta/event contract for graph updates and downstream consumers
- Semantic code graph v2 schema for symbols, imports, exports, calls,
  references, source spans, and language metadata
- Dependency/impact graph and hot-path indexes for boundary membership, symbol
  ownership, known-edge existence, and architectural index checks, maintained
  incrementally
- Trust/policy graph contract for trust levels, side-effect surfaces, data
  classifications, invariant guards, and policy evidence — and wiring the trust
  annotation pass onto the daemon certify path
- Control/session graph contract for hosts, drivers, sessions, leases, fences,
  worktrees, and attribution
- Plan/provenance graph contract joining APS work items, commits, memory
  provenance, and trust posture changes
- Multi-graph registry + typed query traits, and the consumer query contract
- Hot-read enforcement: a type-split admissible API, an ADR-031 Criterion CI
  gate, a hard-capped reverse-impact depth lever, and the A→A′ backing swap with
  a verdict-parity proof
- Persistence-snapshot enforcement (sealed-DTO + structural no-leak guard) for
  the warm-start path

## Out of Scope

- Generic graph database product work
- Community detection, clustering, or embedding search
- Visual graph UI surfaces; dashboard modules own visualisation
- Cross-repo graph registry beyond stable per-repo identity hooks
- Expensive transitive analysis on the daemon hot path
- Replacing APS, Edda, or Kindling data stores
- Making MCP the primary control plane
- Full interprocedural data-flow analysis in this module
- The Rust-side Edda/Kindling provenance surface itself (EDDA-SEAL owns it);
  GV2-014 only defines the join contract to it

## Interfaces

**Depends on:**

- `crates/anvil-graph-cache` — `SymbolGraph`, `DependencyGraph` (+ reverse
  index), `GraphDelta`, `certify`, the incremental apply pipeline, and the trust
  annotation pass; extracted per [ADR-064](../decisions/064-intercept-graph-cache-crate-boundary.md)
- `crates/anvil-kernel-types` — `SymbolNode`, `SymbolEdge`, `EdgeType`,
  `TrustLevel`, `FileSymbols`, `ImportEdge`
- `crates/anvil-kernel` — the tree-sitter parser that produces `FileSymbols`
- `anvil-intercept` / INTD — daemon control authority, `validate_paths` wire,
  `SessionRecord`/`Attribution` session model ([`intercept-as-built.md`](../../docs/architecture/intercept-as-built.md) §10)
- `surface-drivers` / DRVR — editor and MCP driver contracts
- `edda-stack` — provenance and evolution concepts (TS today; Rust counterpart
  proposed — the open seam for GV2-014)
- APS modules — plan/work-item metadata for plan/provenance joins
- ADR-031 (latency), ADR-061 (save-time wire), ADR-063 (hot-path boundary),
  ADR-064 (crate boundary), ADR-067 (parse feed), ADR-069 (persistence)

**Exposes:**

- [`docs/architecture/graph-v2-foundation-spec.md`](../../docs/architecture/graph-v2-foundation-spec.md)
  — canonical Graph v2 taxonomy, cross-graph identity, joins, and query
  boundaries (the spine)
- `anvil-kernel-types` Graph v2 schema additions
- `anvil-graph-cache` registry/query traits and the typed hot-read API
- Snapshot and delta contract consumed by downstream projection layers
- Query contract for GCTX, DRVR, INTD, and WEAVE consumers

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
  persistence; the privacy line is a sealed-DTO + no-leak test, not a convention

## Prerequisites

- KERN H1 graph, trust metadata, and incremental update work complete
- ADR-015 intercept-loop enforcement accepted as the control-plane direction
- ADR-030 surface-driver pivot accepted as the VSCode/MCP migration direction
- ADR-031 latency rubric available for hot-path budget references
- EDDA provenance and evolution schemas complete for join alignment

## Ready Checklist

Change status to **Ready** when:

- [x] Graph taxonomy ratified by a formal architecture-review council —
      **ratified 2026-06-08** (council `plan-ec495f8b`, RATIFY-WITH-FIXES;
      conditions C-1..C-6 folded into the spine spec, now `Live`). Verdict:
      [2026-06-08-gv2-taxonomy-ratification-verdict](../reviews/2026-06-08-gv2-taxonomy-ratification-verdict.md)
- [x] Hot-path/non-hot-path boundary agreed with INTD and DRVR owners —
      ratified in [ADR-063](../decisions/063-gv2-hot-path-boundary.md) (Accepted
      2026-06-01)
- [x] Stable identity model reviewed against git rename and symbol rename cases
      — delivered by GV2-002: file rename = delete + create (identities are
      path-scoped), symbol rename classified by the export-diff with no
      retained history (PV-4); contract tests in
      `crates/anvil-graph-cache/tests/identity.rs`
- [x] Persistence strategy ADR — [ADR-069](../decisions/069-graph-v2-persistence.md)
      **Accepted 2026-06-04** (GV2-021 Merged via PR #2301)
- [x] Privacy review completed for persisted provenance/session fields —
      **completed 2026-06-08** (council `gv2-privacy-20260608`,
      APPROVE-WITH-CONDITIONS, unanimous; conditions PV-1..PV-12 folded into
      GV2-002/GV2-030 below). The completed answer: **no provenance/session
      field persists in v1** — the `SnapshotPayload` covers semantic +
      dependency only, and the per-field-class table feeds the GV2-030
      allowlist. Verdict:
      [2026-06-08-gv2-privacy-review-verdict](../reviews/2026-06-08-gv2-privacy-review-verdict.md)
- [x] GCTX module updated to depend on GV2 rather than owning foundation work —
      [graph-context-delivery](graph-context-delivery.aps.md) declares GV2 as a
      dependency and lists schemas, stable IDs, deltas, hot indexes, and
      persistence as GV2-owned
- [x] Validation commands for the first implementation slice are concrete — each
      reshaped work item below carries a runnable validation command grounded on
      `crates/anvil-graph-cache`

---

## Work Items

### Phase 0 — Architecture and Contracts

#### GV2-001: Graph v2 architecture spine spec and taxonomy

- **Status:** Merged — spec shipped 2026-06-07 via PR #2350; **taxonomy ratified
  2026-06-08** (council `plan-ec495f8b`, RATIFY-WITH-FIXES; conditions C-1..C-6
  folded, spec now `Live`).
- **Intent:** State the joined-graph model once: the five graphs, cross-graph
  identity, the join model, the query/registry API shape, and the subsystem
  seams — synthesising the ratified ADRs rather than re-deciding them.
- **Expected Outcome:** Spine spec on `main` describing the semantic code,
  dependency/impact, trust/policy, control/session, and plan/provenance graphs,
  what each owns, and how joins work; registered in the generated docs indexes.
- **Validation:** `pnpm docs:check && pnpm docs:index:check`; formal council
  ratification flips the taxonomy Ready gate.
- **Files:** `docs/architecture/graph-v2-foundation-spec.md`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

#### GV2-002: Stable graph identity + export-diff primitive

- **Status:** Merged 2026-06-08 via PR #2387 — privacy-review gate cleared the
  same day ([verdict](../reviews/2026-06-08-gv2-privacy-review-verdict.md));
  full-pack council `council-f33ee5ef` converged pre-merge
- **Intent:** Replace the position-conflated `symbol_baseline_key`
  (`file::kind::name`) and the session-local `SymbolNode.id` counter with a
  stable, cross-restart symbol identity, and deliver the export-diff primitive
  that graduates the save-time fast path from "any touched public symbol →
  `partial`" to a real added/removed/renamed-public-symbol diff.
- **Expected Outcome:** Identity contract covers content hashes, path identity,
  position-independent symbol identity (overload-disambiguated), edge identity,
  session/worktree identity, and APS/provenance references, with documented
  rename behaviour; snapshots and deltas stay comparable across restart.
  Per-row delivery: symbol identity + rename stance + edge-identity derivation
  ship in code/spec here; content hashes already exist (`validate_paths`);
  session/worktree identity and APS/provenance references are pinned as
  join-time-only contract rows (privacy verdict PV-3) and implement at
  GV2-013/GV2-014.
- **Validation:** Unit tests for file rename, symbol rename, delete/recreate,
  same-`(kind,name)` overload added to an already-public symbol (red today), and
  same-name symbols in different scopes — `cargo test -p eddacraft-anvil-graph-cache -- identity`
- **Files:** `crates/anvil-graph-cache/src/incremental.rs`,
  `crates/anvil-graph-cache/src/certify.rs`,
  `crates/anvil-kernel-types/src/graph.rs`,
  `docs/architecture/graph-v2-foundation-spec.md`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-001
- **Note:** Privacy review completed 2026-06-08 — implement under conditions
  PV-1..PV-5 of the
  [privacy verdict](../reviews/2026-06-08-gv2-privacy-review-verdict.md):
  (PV-1) overload disambiguator is structural only (kind/arity/type-identity
  refs or fixed-width hash, and/or source-offset ordinal) — never parameter
  source text or default-value expressions; (PV-2) the stable-id hash is a
  named deterministic content hash (SHA-256/Blake3/fixed-seed FxHash), never
  `std::hash::Hash` over the default random-seeded hasher; (PV-3)
  session/worktree identity and APS/provenance references in the identity
  contract are join-time-only — absent from any persisted payload in v1;
  (PV-4) no persisted rename history — rename = delete-old-id + create-new-id;
  (PV-5) every new hash field declares its input domain (ALLOW-class data or
  whole-file content; no truncated hashes over literals).

---

#### GV2-003: Complete graph delta and event contract

- **Status:** Merged 2026-06-08 via PR #2391 — full-pack council `council-be812df9`
  converged pre-merge; dependency GV2-002 Merged (PR #2387)
- **Intent:** Make graph changes observable as complete, deterministic deltas.
  Today `GraphDelta.removed_edges` is permanently empty and a modify is modelled
  as full churn; fix the incremental pipeline to populate removed edges and a
  changed-node channel, and carry a schema version.
- **Expected Outcome:** Delta contract includes added/removed/changed nodes,
  added/removed/changed edges, affected files, identity anchors, content hashes,
  provenance metadata, and `schema_version` — with no field that lies about its
  capability. Per-row delivery (the "no lying field" principle decides what
  ships as a field vs a documented reference): `removed_edges` is now
  **populated** (was permanently empty); a `node_changes` channel anchors
  added/changed/removed nodes to stable `SymbolIdentity` (the identity
  anchors); `schema_version` is carried on every delta. "Changed edges" need
  no separate channel — an edge's identity derives from its endpoint
  identities, so an endpoint change is reported via `node_changes` and the edge
  is re-created under the new ids. **Content hashes** ride
  the `FileSymbols` parser feed (hashed at the `validate_paths` boundary — not
  recomputable in graph-cache without file bytes) and **provenance/session
  metadata** is join-time-only (privacy verdict PV-3); neither is added as a
  `GraphDelta` field because this layer cannot populate them truthfully —
  documented on the type rather than shipped as an empty lying field.
- **Validation:** Property test confirms full rebuild and replayed delta
  sequences (incl. atomic-save inode flip, rename = delete+create,
  delete/recreate) produce equivalent observable `(SymbolGraph, DependencyGraph)`
  state — `cargo test -p eddacraft-anvil-graph-cache -- delta_replay_equivalence`
- **Files:** `crates/anvil-graph-cache/src/incremental.rs`,
  `crates/anvil-kernel-types/src/graph.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-002

---

### Phase 1 — Graph Schemas

#### GV2-010: Semantic code graph v2 schema

- **Status:** Merged 2026-06-08 via PR #2419 — shipped the `Reexports`
  first-class edge + `ReexportEdge` carrier + extraction (TS/JS + Rust, A′-critical)
  and the GCTX-projection **schema type** (no-text `ByteRange`), with
  `schema_fixtures`. Council `council-66cb4833` (accept-with-changes applied:
  namespace/type-only/alias/glob re-export name correctness). Per the ADR-075
  decision, call/reference/language-metadata *population* and span-field
  *attachment* (onto nodes/edges) are deferred to GCTX/v0.9.
- **Intent:** Expand the current symbol/import graph into the canonical semantic
  code graph, separating the A′-critical subset from the GCTX-projection subset.
- **Expected Outcome:** **A′-critical subset** — stable identity wiring
  (GV2-002), visibility, import/dependency edges, and a `Reexports` edge type
  (today re-export recursion rides file-level `dependents_of`). **GCTX-projection
  subset** — source spans (no-text `ByteRange` only), calls, references, and
  language metadata, added as additive fields that never persist raw bodies.
- **Validation:** Snapshot fixtures for TS/JS plus one future-language fixture
  show deterministic node/edge output, including a `Reexports` edge case —
  `cargo test -p eddacraft-anvil-graph-cache -- schema_fixtures`
- **Files:** `crates/anvil-kernel-types/src/graph.rs`,
  `crates/anvil-kernel/src/parser/extract/mod.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-002, GV2-003
- **Note:** Privacy review completed 2026-06-08
  ([verdict](../reviews/2026-06-08-gv2-privacy-review-verdict.md)) — the
  persistable schema subset is bounded by the verdict's per-field-class table;
  source spans stay no-text `ByteRange` (structurally incapable of holding
  spanned text) and the GCTX-projection fields never persist raw bodies.

---

#### GV2-011: Dependency/impact graph and incremental hot indexes

- **Status:** Merged 2026-06-08 via PR #2428 — incremental dependency-graph
  maintenance landed (the O(edges) `derive_dependency_graph` re-derive retired on
  the save-time hot path); the Criterion budget gate stays GV2-025.
- **Intent:** Maintain the dependency/impact indexes incrementally so the daemon
  retires the O(edges) `derive_dependency_graph` full re-derive
  (`crates/anvil-intercept/src/kernel_cache.rs`) and reads warm, resident state.
- **Expected Outcome:** Boundary membership, symbol ownership, known-edge
  existence, and precomputed architectural-index checks are bounded reads,
  maintained in `apply_delta` with no full re-derive; transitive impact stays
  explicitly non-hot-path; the precomputed-vs-background read set is enumerated.
- **Validation:** A property test that the incrementally-maintained indexes equal
  a cold rebuild after an arbitrary delta sequence; Criterion benchmark (GV2-025)
  shows the hot reads meet the ADR-031 **save-time** budget on the canonical
  corpus — `cargo test -p eddacraft-anvil-graph-cache -- index_consistency`
- **Files:** `crates/anvil-graph-cache/src/dependency.rs`,
  `crates/anvil-graph-cache/src/symbol_graph.rs`,
  `crates/anvil-intercept/src/kernel_cache.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GV2-010
- **Progress:** Incremental dependency-graph maintenance implemented — the
  per-save full re-derive (`derive_dependency_graph` on the `apply_delta` hot
  path) is retired in favour of `DependencyGraph::set_dependencies`
  (`dependency.rs`). On each non-delete save the affected files are refreshed
  individually: the changed file, its **prior dependents** (`update_file` drops
  every `importer → file` edge incident to the file's old symbols, so each is
  reconciled — re-added if still resolved, dropped if stale), and the **sources
  of edges re-resolution adds** (`re_resolve_imports_tracked` — forward
  references that just resolved, and surviving imports of *other* files that
  re-bind to a new target after a deletion); a delete drops the file in both
  directions. All bounded by the changed file's local neighbourhood, never the
  whole graph. `derive_dependency_graph` survives only as the `#[cfg(test)]`
  cold-rebuild oracle. Cold-rebuild equivalence is proven by a seeded-LCG
  property test at the dep-graph layer
  (`index_consistency_under_arbitrary_delta_sequence`, `eddacraft-anvil-graph-cache`)
  and a multi-seed end-to-end one over arbitrary create/modify/delete/recreate +
  forward-reference + ambiguous-rebind sequences
  (`warm_dep_graph_matches_cold_rebuild_over_arbitrary_sequence`,
  `eddacraft-anvil-intercept`), plus a focused regression for the re-resolution
  re-bind case a Council CRITICAL surfaced
  (`re_resolution_rebinds_surviving_import_after_target_delete`). The
  **precomputed-vs-background read set** is
  fixed by [ADR-063](../decisions/063-gv2-hot-path-boundary.md): the dependency
  index serves the hot-path allowlist reads (#2 known-edge existence, #3 bounded
  reverse impact via `dependents_of` at the hard-capped depth, #4 precomputed
  architectural-index check) as bounded resident reads; everything on the ADR-063
  denylist (cross-file resolution, transitive impact beyond the configured depth,
  full-graph scans, index rebuilds) stays in the background pool. The typed
  hot-read API + warm/stale markers (GV2-022), the depth lever (GV2-026), and the
  Criterion budget gate (GV2-025) remain their own items.

---

#### GV2-012: Trust and policy graph contract

- **Status:** Draft (grown into the wave)
- **Intent:** Separate trust/policy semantics from the raw semantic graph while
  preserving deterministic joins back to code evidence.
- **Expected Outcome:** Contract represents trust level, side-effect surfaces,
  data classifications, invariant guards, policy evidence, and override sources
  without forcing full interprocedural data-flow analysis. Intersects GV2-029
  (the daemon trust-annotation wiring).
- **Validation:** Fixtures show trust posture changes are emitted as graph deltas
  and policy evidence resolves back to source spans —
  `cargo test -p eddacraft-anvil-graph-cache -- trust_graph`
- **Files:** `crates/anvil-graph-cache/src/trust.rs`,
  `crates/anvil-kernel-types/src/trust.rs`,
  `docs/architecture/graph-v2-foundation-spec.md`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-010

---

#### GV2-013: Control and session graph contract

- **Status:** Draft — dep GV2-001 Merged (unblocked), but **v0.9 scope** per ADR-075 (non-critical-path)
- **Intent:** Model execution hosts, drivers, sessions, leases, fences,
  worktrees, and attribution as a graph that joins to code changes without
  making MCP the control plane.
- **Expected Outcome:** Contract **cites** the shipped INTD session model
  (`SessionRecord`, `SessionId`, `Attribution::Owned`,
  [`intercept-as-built.md`](../../docs/architecture/intercept-as-built.md) §10)
  rather than inventing one, and identifies which fields are hot-path,
  telemetry-only, or persisted-for-provenance. **Per ratification condition C-1
  (spec G-05):** defines the shared worktree-root→file relativisation type in
  `anvil-kernel-types` so the control/session→semantic join is followable
  without depending on `anvil-intercept` (which would invert ADR-064).
- **Validation:** Design review against INTD and DRVR specs; contract covers
  shell, editor, and MCP driver cases.
- **Files:** `docs/architecture/graph-v2-foundation-spec.md`,
  `plans/specs/anvil-driver-framework/anvil-driver-framework-design-spec.md`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GV2-001

---

#### GV2-014: Plan and provenance graph contract

- **Status:** Draft — deps GV2-002/003 Merged (unblocked), but **v0.9 scope** per ADR-075 (non-critical-path)
- **Intent:** Join APS intent, git history, Edda provenance, graph deltas, and
  trust posture changes so Anvil can explain why a structural change was allowed
  or challenged.
- **Expected Outcome:** Contract maps work items, commits, change events,
  memories, policy decisions, and graph-state changes without making APS a
  runtime prerequisite. **Explicitly resolves the Rust↔TS provenance boundary**
  (the spine spec G-02 open seam) and shares one provenance contract with
  EDDA-SEAL rather than designing a second.
- **Validation:** Fixture trace links one code change to an APS item, commit,
  graph delta, policy result, and provenance record (the spine spec's worked
  join trace).
- **Files:** `docs/architecture/graph-v2-foundation-spec.md`,
  `packages/edda-stack/src/contracts/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GV2-002, GV2-003

---

### Phase 2 — Runtime Substrate

#### GV2-020: Multi-graph registry and typed query traits

- **Status:** Draft — now buildable (012/013/014 are in-wave).
- **Intent:** Provide one typed in-process entry point for querying joined graph
  state without coupling consumers to storage or `petgraph` internals.
- **Expected Outcome:** Registry exposes graph handles and join queries for
  semantic, dependency, trust, control, and provenance graphs; consumers depend
  on traits rather than concrete storage.
- **Validation:** Kernel unit tests exercise each graph handle and one join query
  across code/trust/provenance state; a **negative test** that a non-admissible
  (denylist) op is not reachable from the hot-read API type; and one
  **end-to-end** test driving `validate_paths` through the registry path with a
  non-vacuous verdict (no zero-callers) —
  `cargo test -p eddacraft-anvil-graph-cache -- registry`
- **Files:** `crates/anvil-graph-cache/src/lib.rs`,
  `crates/anvil-graph-cache/src/registry.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-010, GV2-011, GV2-012, GV2-013, GV2-014

---

#### GV2-021: ADR — graph persistence and snapshot strategy

- **Status:** Merged 2026-06-04 via PR #2301
- **Intent:** Record the persistence decision for Graph v2, reconciling the
  current GCTX rkyv/SQLite assumptions with daemon, hot-read, privacy, and
  schema-version requirements.
- **Expected Outcome:** ADR defines default snapshot format, rebuild behaviour,
  crash-safety expectations, multi-process reader stance, privacy boundaries,
  and migration/versioning rules.
- **Validation:** ADR reviewed by council-reviewer, kernel-maintainer, and
  security reviewer.
- **Files:** `plans/decisions/069-graph-v2-persistence.md`
- **Progress:** [ADR-069](../decisions/069-graph-v2-persistence.md) **Accepted
  2026-06-04** (Josh) — sealed canonical-DTO load-once snapshot (`postcard`),
  default-off, restore-indexes-never-verdict, discard-and-rebuild versioning,
  structural-identity-only privacy line. **Merged 2026-06-04 via PR
  [#2301](https://github.com/eddacraft/anvil-001/pull/2301)** (`d8caed47`). The
  enforcement of the privacy line is GV2-030.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-001, GV2-003

---

#### GV2-022: Hot-path read API and latency guardrails

- **Status:** Draft
- **Intent:** Expose the specific warmed reads the daemon and drivers may use
  during save-time or mid-edit enforcement, with explicit warm/stale markers and
  a miss-degrades-to-fallback rule (never escalate to parse/rebuild/IO on the hot
  path).
- **Expected Outcome:** API offers boundary membership, symbol ownership,
  known-edge existence, and architectural index checks; each read returns a
  `warm`/`stale` marker; the reverse-impact depth is a hard-capped lever
  (GV2-026). Type-split enforcement is GV2-024; the CI latency gate is GV2-025.
- **Validation:** A warm-miss test proves a typed `StaleReason` + degrade with no
  filesystem read; benchmarks (GV2-025) fail when p95 exceeds the ADR-031
  budget — `cargo test -p eddacraft-anvil-graph-cache -- hot_read`
- **Files:** `crates/anvil-graph-cache/src/hot_index.rs`,
  `crates/anvil-graph-cache/src/lib.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GV2-011

---

#### GV2-023: Consumer query contract for daemon, drivers, MCP, and weave

- **Status:** Draft
- **Intent:** Define the graph query boundary downstream consumers use so GCTX,
  DRVR, INTD, and WEAVE do not grow incompatible graph adapters.
- **Expected Outcome:** Query contract separates enforcement reads, diagnostic
  reads, provenance reads, and context projection reads; MCP/assistant queries
  are explicitly projections over the same trusted substrate.
- **Validation:** Contract review with GCTX, DRVR, INTD, and WEAVE plan owners;
  each consumer has at least one mapped query scenario.
- **Files:** `docs/architecture/graph-v2-foundation-spec.md`,
  `plans/modules/graph-context-delivery.aps.md`,
  `plans/modules/surface-drivers.aps.md`,
  `plans/modules/weave.aps.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** GV2-020, GV2-022

---

### Phase 3 — Enforcement, Wiring, and the A′ Swap

#### GV2-024: Hot-read type split + hot-path debug assertion

- **Status:** Draft
- **Intent:** Make ADR-063 admissibility "enforced, not aspirational" — a sealed
  `HotReadApi` exposing only the four allowlist ops, with denylist ops reachable
  only via a separate `BackgroundReadApi`, plus a debug assertion that trips on
  any parse/resolve/traversal/IO inside a hot call.
- **Expected Outcome:** Non-admissible ops do not compile when called from the
  hot type; the assertion fires under test when a hot call performs disallowed
  work.
- **Validation:** A compile-fail test (e.g. `trybuild`) for a denylist call from
  the hot type; a unit test that the debug assertion trips —
  `cargo test -p eddacraft-anvil-graph-cache -- admission`
- **Files:** `crates/anvil-graph-cache/src/hot_index.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-022

---

#### GV2-025: Criterion hot-read benchmark + ADR-031 CI gate

- **Status:** Draft
- **Intent:** Land the missing latency gate ADR-063 names — a Criterion harness
  that fails CI when hot-read p95 exceeds the ADR-031 save-time budget.
- **Expected Outcome:** `benches/hot_read.rs` measuring per-file lookup,
  `dependents_of`, and `impact_closure` at depth 1 and at the hard cap on the
  latency corpus, wired as a CI check; declares its quiet/CI-box requirement.
- **Validation:** `cargo bench -p eddacraft-anvil-graph-cache` on a quiet box;
  CI asserts p95 within the ADR-031 interactive budget.
- **Files:** `crates/anvil-graph-cache/benches/hot_read.rs`,
  `.github/workflows/ci.yml`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-011, GV2-022

---

#### GV2-026: Reverse-impact hop-depth lever

- **Status:** Draft
- **Intent:** Implement the ADR-063 configurable, hard-capped reverse-impact
  depth (default 1 hop) — today `impact_closure` has only a file-count budget
  with unbounded depth.
- **Expected Outcome:** `impact_closure` gains a `max_depth` distinct from the
  file-count budget, hard-capped and exposed as a feature-flag/config lever
  (1→2 hops without recompile); an over-cap setting is clamped, not honoured.
- **Validation:** A 3-hop fixture proves depth=1 stops at the direct importer;
  a clamp test — `cargo test -p eddacraft-anvil-graph-cache -- impact_depth`
- **Files:** `crates/anvil-graph-cache/src/certify.rs`,
  `crates/anvil-intercept/src/save_time.rs`, `flags/manifest.json`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GV2-011

---

#### GV2-027: A→A′ backing swap behind `validate_paths`, with verdict parity

- **Status:** Draft
- **Intent:** Wire the resident GV2 hot-read index behind `validate_paths`,
  retiring the interim `KernelGraphCache` re-derive, and prove the swap is
  wire-invariant.
- **Expected Outcome:** `validate_paths`/`save_time` read the GV2 hot-read API;
  `backing_schema_version` bumps `interim-symbolgraph-v1` → `gv2-hotindex-v1`; a
  parity property test asserts verdict-identical `Certifiability` vs the interim
  backing over arbitrary delta sequences.
- **Validation:** `cargo test -p eddacraft-anvil-intercept -- backing_parity`;
  the existing diagnostic-parity gate stays green.
- **Files:** `crates/anvil-intercept/src/kernel_cache.rs`,
  `crates/anvil-intercept/src/validate_paths.rs`,
  `crates/anvil-intercept/src/save_time.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-022, GV2-024, GV2-028, GV2-029

---

#### GV2-028: Production parser feed for certified verdicts

- **Status:** Merged 2026-06-08 via PR #2438 — the kernel-side parser feed
  itself shipped under DSV-005 (PR #2282, ADR-067): `KernelSymbolParser`
  (`crates/anvil-cli/src/intercept_symbol_parser.rs`) is injected into the daemon
  via `ForegroundOpts::with_symbol_parser` on the `intercept start` path
  (`crates/anvil-cli/src/commands/intercept.rs`), so `validate_paths` certifies
  real TS/JS edits in production. This PR ratifies that wire-up and adds the
  user-facing watch-path proof
  (`watch_client_certifies_through_real_daemon_parser`): `anvil watch` → real
  daemon → real parser → `Certified` end to end, closing the "uncalled library"
  risk at the production entry point. The original `Files:` anticipated the wire
  in `watch.rs`; it actually landed in `intercept.rs` (daemon start), with
  `watch.rs`/`watch_save_time.rs` as the client that consumes the verdict.
- **Intent:** Wire the ADR-067 kernel-side parse feed so `fed_symbols` yields
  `FileSymbols` for TS/JS; until this lands every `ContentModify` returns
  `partial` regardless of graph quality (`validate_paths.rs`).
- **Expected Outcome:** A body-only edit on a parsed file returns `certified`,
  not `partial`, end-to-end through the daemon — proving the backing is live in
  production, not an uncalled library.
- **Validation:** `cargo test -p eddacraft-anvil-intercept -- parser_feed`
  (`validate_certifies_when_parser_feeds_matching_surface`, daemon-side, fed
  surface) plus the real-parser proofs in `eddacraft-anvil`:
  `real_parser_certifies_repeat_save_through_daemon` (direct `SaveTimeConn`) and
  `watch_client_certifies_through_real_daemon_parser` (through `anvil watch`).
- **Files:** `crates/anvil-intercept/src/validate_paths.rs`,
  `crates/anvil-cli/src/intercept_symbol_parser.rs`,
  `crates/anvil-cli/src/commands/intercept.rs`,
  `crates/anvil-cli/src/commands/watch_save_time.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-010

---

#### GV2-029: Wire privilege containment on the daemon certify path

- **Status:** Draft
- **Intent:** Per the owner decision to **claim privilege containment**, call
  `annotate_trust` on the daemon apply path (today it is never called, so
  `trust_level` is always `Unknown` and `previously_privileged` always empty —
  an inert dimension and a live false-certify). Extend the filter to treat
  `Boundary` as elevated alongside the `previously_public` diff.
- **Expected Outcome:** A change that newly imports `node:fs`/`child_process` and
  exposes a privileged surface does **not** certify clean. **Blocks the GV2-027
  swap until green.**
- **Validation:** `cargo test -p eddacraft-anvil-intercept -- privilege_certify`
  — a `node:fs`-importing privilege-expanding change is not `certified`.
- **Files:** `crates/anvil-intercept/src/kernel_cache.rs`,
  `crates/anvil-graph-cache/src/trust.rs`,
  `crates/anvil-graph-cache/src/certify.rs`,
  `crates/anvil-graph-cache/src/incremental.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GV2-012

---

#### GV2-030: Sealed-DTO snapshot serialisation + structural no-leak guard

- **Status:** Draft
- **Intent:** Enforce the ADR-069 privacy line in code (today it is a convention
  — no `SnapshotPayload` DTO, codec, or no-leak test exists). Sub-phase B
  persistence prerequisite.
- **Expected Outcome:** A sealed allowlist-only `SnapshotPayload` DTO + `postcard`
  codec; a test failing CI if any transitive field outside the allowlist
  (`Vec<u8>`, `serde(flatten)`, any source-text `String`) can reach the payload,
  and asserting every persisted path is workspace-root-relative; gates the
  `ANVIL_PERSIST_GRAPH` default.
- **Validation:** `cargo test -p eddacraft-anvil-graph-cache -- snapshot_no_leak`
  plus a golden round-trip + version-mismatch cold-rebuild test.
- **Files:** `crates/anvil-graph-cache/src/snapshot.rs`,
  `crates/anvil-graph-cache/src/lib.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GV2-002, GV2-003
- **Note:** Implement under conditions PV-6..PV-12 of the 2026-06-08
  [privacy verdict](../reviews/2026-06-08-gv2-privacy-review-verdict.md):
  (PV-6) the v1 DTO covers semantic + dependency only — zero
  session/attribution/provenance fields; GV2-013/GV2-014 need their own privacy
  ADRs before persisting (spec condition C-6); (PV-7) the no-leak test asserts
  no `PathBuf`-typed field exists in the payload, path `String`s are
  workspace-root-relative, `GraphDelta` (incl. `errors` and the `previously_*`
  baseline sets) is entirely absent, and identity strings are the only
  permitted `String` fields; (PV-8) the `WorktreeKey`→filename derivation is a
  named stable one-way hash, and the test extends to filenames under
  `graph-cache/`; (PV-9) acceptance criteria pin "identity keys are
  machine-local; any export/sync/transmit surface requires a new privacy
  review"; (PV-10) snapshot telemetry counters bind to a machine-local ADR-035
  pipe with outcome-enum labels only; (PV-11) `ANVIL_PERSIST_GRAPH` enters
  `flags/manifest.json` before graduation; (PV-12) extend the ADR-069 §8
  residual-risk note (backup/dotfile-sync/CI-mount pickup, hash correlation)
  and clarify the §10 GC startup predicate.

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Graph v2 becomes a generic graph database project | Medium | High | Keep scope tied to enforcement, provenance, hot reads, and deterministic safety decisions |
| MCP/context needs over-shape the foundation | Medium | High | GCTX depends on GV2 and consumes projections; schema authority stays with GV2 |
| Hot path accidentally includes expensive traversal | Medium | High | GV2-024 type-split makes denylist ops uncallable; GV2-025 ADR-031 CI gate + debug assertion enforce it |
| The grown wave (012/013/014/020/023) stalls and re-stales like the original module | Medium | Medium | A′ ships on the settled semantic+dependency subset (GV2-027 critical path); the grown rows trail as independently-flaggable work, not swap blockers |
| Backing swap silently changes verdicts | Medium | High | GV2-027 parity property test asserts verdict-identical Certifiability vs the interim backing |
| Privilege containment claimed but inert in prod | Medium | High | GV2-029 wires `annotate_trust` + a `node:fs` test, and blocks the GV2-027 swap until green |
| Stable identity wrong for rename-heavy work | Medium | Medium | GV2-002 includes rename/delete/recreate + overload validation before implementation |
| Persisted session/provenance data captures too much private context | Medium | High | GV2-030 sealed-DTO + no-leak test; [privacy verdict](../reviews/2026-06-08-gv2-privacy-review-verdict.md) pins v1 payload to semantic+dependency only (PV-6) with conditions PV-1..PV-12 folded into GV2-002/030 |
| Slice ships as an uncalled library (zero-callers) | Medium | High | GV2-020 e2e-through-registry test + GV2-028 parser feed prove the backing is live in `validate_paths` |

## Decisions

1. **Multiple joined graphs, not a mega-graph** — semantic code, dependency,
   trust/policy, control/session, and plan/provenance state have different
   lifecycles, privacy concerns, and latency needs.
2. **Anvil-first** — Graph v2 is justified by prevention, enforcement,
   provenance, and trust. Assistant context is a projection and accelerant, not
   the foundation's reason to exist.
3. **Hot indexes over hot traversal** — daemon/driver enforcement may read warm
   indexes; full recompute, transitive analysis, explanation, and context
   slicing stay outside the hot path.
4. **Derivable by default** — persisted graph snapshots are cache state unless a
   future ADR explicitly makes a field authoritative.
5. **Planless-first preserved** — plan/provenance joins enrich Anvil when APS is
   present, but Graph v2 must still work from source/config alone.
6. **Grow the wave (2026-06-05 verdict)** — 012/013/014 are in-wave so GV2-020 is
   the full multi-graph registry; A′ still ships on the semantic+dependency
   subset without waiting for them.
7. **Graduate GV2-002** — build real stable identity + export-diff now so precise
   edits stay `certified`, rather than shipping A′ on the conservative default.
8. **Claim privilege containment** — save-time certify attests privilege
   containment, so GV2-029 wires the trust pass and blocks the swap until proven.

## Stats

| Phase | Items | Completion | Status |
| ----- | ----- | ---------- | ------ |
| 0 — Architecture and Contracts | 3 | 3/3 done | Complete |
| 1 — Graph Schemas | 5 | 2/5 done | In Progress |
| 2 — Runtime Substrate | 4 | 1/4 done | Draft |
| 3 — Enforcement, Wiring, and the A′ Swap | 7 | 1/7 done | In Progress |
| **Total** | **19** | **7/19 done** | **In Progress** |
