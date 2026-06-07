# Graph v2 Foundation — Architecture Spec

| Type | Authority | Owner                                                                                              | Status | Freshness                                                                                                             |
| ---- | --------- | -------------------------------------------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------- |
| Spec | Derived   | GV2 ([`plans/modules/graph-v2-foundation.aps.md`](../../plans/modules/graph-v2-foundation.aps.md)) | Draft  | Drafted 2026-06-05 as a synthesis of ADR-061/063/064/067/069 + the GV2 module; **not yet council-ratified** (GV2-001) |

| Upstream                                                                                                                                                                                              | Downstream                                                                                                                                              |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADR-061, ADR-063, ADR-064, ADR-067, ADR-069, ADR-031; `crates/anvil-kernel-types`, `crates/anvil-graph-cache`; [`intercept-as-built.md`](./intercept-as-built.md), [`edda-stack.md`](./edda-stack.md) | `graph-context-delivery` (GCTX), `surface-drivers` (DRVR), `multilayer-protection-v2` (INTD), `weave` (WEAVE); the daemon save-time validation contract |

## Purpose and scope

This is the **spine** of Graph v2: the one place that states the joined-graph
model, the cross-graph identity contract, the join model, the query/registry API
shape, and the seams to other subsystems. It exists because that model is
**already decided** — ratified across ADR-061/063/064/067/069 and structured in
the GV2 module — but was never written down in one artefact that the work items
and ADRs all cite.

This document **synthesises and reconciles**; it does not re-decide. Where a
decision is frozen by an ADR, this spec points at the ADR and does not restate
its reasoning.

**In scope:** the five-graph taxonomy and what each graph owns; the cross-graph
identity model; the join model and a worked join trace; the query/registry API
_shape_ (interface, not implementation); the hot/non-hot read boundary; the
seams to INTD, DRVR, GCTX, trust/policy, and provenance;
persistence/derivability invariants.

**Out of scope (owned elsewhere, by design):**

- Per-graph field schemas — owned by the per-graph work items
  (GV2-010/012/013/014) and the `anvil-kernel-types` code, not duplicated here.
- The hot-/non-hot-path admission rule itself — frozen in
  [ADR-063](../../plans/decisions/063-gv2-hot-path-boundary.md); summarised, not
  re-derived.
- The crate boundary — frozen in
  [ADR-064](../../plans/decisions/064-intercept-graph-cache-crate-boundary.md).
- Persistence/snapshot/privacy mechanics — frozen in
  [ADR-069](../../plans/decisions/069-graph-v2-persistence.md).
- The save-time wire — frozen in
  [ADR-061](../../plans/decisions/061-save-time-daemon-delta-validation.md) and
  the
  [save-time validation contract](../../plans/specs/2026-06-01-daemon-save-time-validation-contract.md).

## Substrate status (orientation)

Graph v2 is **not** green-field. The semantic + dependency layer shipped as the
Sub-phase A backing and lives in `crates/anvil-graph-cache/` (ADR-064). This
spec describes the target end-state; the table below marks how far each piece is
from it so a reader knows what is design vs reality.

| Layer                                   | Where it lives today                                                                                                                   | State                                                                 |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Semantic code graph                     | `crates/anvil-graph-cache/src/symbol_graph.rs`, `crates/anvil-kernel-types/src/graph.rs`                                               | Shipped (Sub-phase A); schema additions pending (GV2-010)             |
| Dependency/impact graph + reverse index | `crates/anvil-graph-cache/src/dependency.rs`                                                                                           | Shipped; incremental maintenance + hot-read API pending (GV2-011/022) |
| Trust/policy graph                      | trust metadata on nodes (`crates/anvil-kernel-types/src/trust.rs`); richer graph contract Draft                                        | Partial (GV2-012)                                                     |
| Control/session graph                   | shipped **in INTD** as `SessionRecord`/`Attribution` ([`intercept-as-built.md`](./intercept-as-built.md) §10); graph-shaped join Draft | Partial (GV2-013)                                                     |
| Plan/provenance graph                   | provenance shipped **in TS** ([`edda-stack.md`](./edda-stack.md)); Rust join Draft                                                     | Open seam (GV2-014)                                                   |
| Registry + query traits                 | none yet — consumers call `certify()`/`with_graphs()` directly                                                                         | Draft (GV2-020/023)                                                   |

## Principles

These are inherited from the GV2 module's Decisions and are binding on every
graph and join below:

1. **Multiple joined graphs, not a mega-graph.** Semantic, dependency,
   trust/policy, control/session, and plan/provenance state have different
   lifecycles, privacy concerns, and latency needs. They join through shared
   identity, not through one structure.
2. **Anvil-first.** Graph v2 is an enforcement/provenance/trust primitive.
   Assistant context (GCTX) is a _projection_ over it, never a driver of its
   schema.
3. **Hot indexes over hot traversal.** Enforcement may read warm resident
   indexes; recompute, transitive analysis, explanation, and context slicing
   stay off the hot path
   ([ADR-063](../../plans/decisions/063-gv2-hot-path-boundary.md)).
4. **Derivable by default.** Persisted graph state is cache state, rebuildable
   from source, unless a future ADR explicitly makes a field authoritative
   ([ADR-069](../../plans/decisions/069-graph-v2-persistence.md)).
5. **Planless-first preserved.** Plan/provenance joins enrich Anvil when APS is
   present, but the substrate must still work from source/config alone.

## The five-graph taxonomy

Each graph owns a distinct slice of structural truth. The "must not own" column
is the anti-scope that keeps the layers from bleeding into one another.

| Graph                   | Owns                                                                                                                                  | Must not own                                                                                | Owning item |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ----------- |
| **Semantic code**       | files, modules, symbols, imports, exports, calls, references, source spans, language metadata, visibility                             | trust verdicts, session identity, plan/provenance links                                     | GV2-010     |
| **Dependency / impact** | file/module dependency edges, the reverse-impact index, boundary membership, symbol ownership, precomputed architectural-index checks | raw symbol bodies, transitive closure as stored state (it is _derived_, bounded, on demand) | GV2-011     |
| **Trust / policy**      | trust level, side-effect surfaces, data classifications, invariant guards, policy evidence, override sources                          | the raw semantic graph (it _joins_ to it via symbol identity)                               | GV2-012     |
| **Control / session**   | execution hosts, drivers, sessions, leases, fences, worktrees, attribution                                                            | code structure; it attributes _changes_ to sessions, it does not model code                 | GV2-013     |
| **Plan / provenance**   | APS work items, commits, change events, memories, policy decisions, graph-state changes, trust-posture changes                        | runtime control; it must never become an APS-required prerequisite                          | GV2-014     |

## Cross-graph identity (the load-bearing seam)

Identity is the contract that lets five graphs join without merging. **Getting
identity wrong forces a refactor that ripples across every join and every
consumer**, so it is pinned here and owned by GV2-002.

### Current reality vs. required contract

Today, in-graph symbol identity is the `SymbolNode.id: u64`
(`crates/anvil-kernel-types/src/graph.rs:37`) — a **session-local monotonic
counter**, not stable across restart, edit, or rename. The set-key used by the
save-time certify path is `file::kind::name`
(`crates/anvil-graph-cache/src/incremental.rs`), which **conflates identity with
position** and collapses same-`(kind, name)` overloads. This is sound-but-coarse
for Sub-phase A and is the named gap GV2-002 must close before warm-start
snapshots (which need cross-restart comparability) or precise export diffing can
be trusted.

### The identity keys that cross graph boundaries

| Identity                                                               | Authoritative graph   | Stable across restart today?                | Crosses into                  |
| ---------------------------------------------------------------------- | --------------------- | ------------------------------------------- | ----------------------------- |
| File identity (path + content hash)                                    | Semantic              | path yes / content-hash yes                 | dependency, trust, provenance |
| Symbol identity (stable, position-independent, overload-disambiguated) | Semantic (GV2-002)    | **no — `u64` counter; GV2-002 closes this** | trust, provenance             |
| Edge identity (typed `from → to` over symbol/file identity)            | Semantic / dependency | derived                                     | dependency, provenance        |
| Session / worktree identity (`SessionId`, `WorktreeKey`)               | Control/session       | yes (`SessionRecord`, INTD)                 | provenance, attribution       |
| Plan / commit / memory anchors (APS id, commit SHA, Edda ref)          | Plan/provenance       | external                                    | provenance joins              |

**Rule:** every graph references foreign nodes **only** through the identity
owned by the foreign graph — never by storing a copy of the foreign node. A join
is an identity lookup, not a structural embedding.

## The join model

A join is a typed query that follows one graph's identity into another. The
spine fixes _which_ joins exist and _what key_ each bridges; the per-graph items
own the field detail.

| Join                       | Key bridged                        | Consuming query (example)                                        |
| -------------------------- | ---------------------------------- | ---------------------------------------------------------------- |
| semantic ↔ dependency      | file + symbol identity             | "what imports symbol S / file F?" (`dependents_of`)              |
| semantic ↔ trust           | symbol identity                    | "what is the trust level / side-effect surface of S?"            |
| dependency ↔ trust         | file/edge identity                 | "does this new edge cross a trust boundary?"                     |
| control/session ↔ semantic | worktree identity → file identity  | "which session authored the change to F?" (`Attribution::Owned`) |
| plan/provenance ↔ all      | symbol/file/commit/session anchors | "why was this structural change allowed or challenged?"          |

### Worked join trace (the spec's proof)

This is GV2-014's own validation scenario, expressed as a path through the
joins. One code change must resolve to one trace:

```text
edit src/pay.ts ──semantic──▶ symbol `chargeCard` (Boundary trust)
   │                              │
   │                          trust join ──▶ side-effect surface: network
   │
 dependency join ──▶ dependents_of(`chargeCard`) = [checkout.ts]
   │
 control/session join ──▶ Attribution::Owned(SessionRecord{id})  (who saved it)
   │
 plan/provenance join ──▶ APS item PAY-007 · commit <sha> · policy: allowed · Edda memory <ref>
```

If any single join in that chain cannot be followed by stable identity, the
trace breaks — which is why GV2-002 gates GV2-014.

## The query / registry API shape

Consumers (INTD, DRVR, GCTX, WEAVE) must depend on **traits over joined state**,
not on `petgraph` internals or storage. The shape — not the implementation — is
fixed here; GV2-020 implements it and GV2-023 is the consumer-facing contract.

### Hot vs. non-hot split (summary of ADR-063)

The single admission rule
([ADR-063](../../plans/decisions/063-gv2-hot-path-boundary.md), binding on INTD,
DRVR, and GV2 alike): a read is hot-path-admissible **iff** it is answerable
from resident warm indexes in `O(1)` or `O(bounded fan-out)` with no parse, no
cross-file resolution, no transitive traversal beyond the configured (default
1-hop, hard-capped) reverse-impact depth, and no blocking I/O.

| Read class                                                                | Admissibility | Surface                |
| ------------------------------------------------------------------------- | ------------- | ---------------------- |
| resident per-file symbol/extract lookup                                   | hot           | hot-read API (GV2-022) |
| known-edge existence (`A → B`?)                                           | hot           | hot-read API           |
| bounded reverse impact (depth ≤ hard cap)                                 | hot           | hot-read API           |
| precomputed architectural-index check                                     | hot           | hot-read API           |
| parse / re-extract / cross-file resolution                                | **non-hot**   | background pool only   |
| transitive impact beyond cap, full scans, index rebuild, persistence load | **non-hot**   | background pool only   |

Every hot read returns an explicit `warm` / `stale` marker; a warm miss maps to
a typed `StaleReason` and **degrades** — it never escalates to a parse, rebuild,
or I/O on the hot path. The split is enforced by a type boundary (non-admissible
ops are not callable from the hot-read API) plus an ADR-031 Criterion benchmark
that fails CI on budget regression.

### The two API tiers

- **Enforcement / hot-read API** (GV2-022) — the allowlist above; consumed by
  the daemon `validate_paths` path and by driver mid-edit reads. One allowlist,
  one admission rule, no surface-local "cheap" reads.
- **Diagnostic / projection API** (GV2-020 registry → GV2-023 consumer contract)
  — join queries, provenance reads, and context projections; runs off the hot
  path. GCTX/MCP queries are explicitly projections over this trusted substrate,
  never a second schema.

## Seams to other subsystems

The crux of "is the system designed": for each seam, where is it pinned?

- **graph ↔ intercept daemon (INTD)** — **pinned.** Hot-read admission
  ([ADR-063](../../plans/decisions/063-gv2-hot-path-boundary.md)); crate
  boundary
  ([ADR-064](../../plans/decisions/064-intercept-graph-cache-crate-boundary.md));
  the parse feed that supplies `FileSymbols` to the parser-free daemon
  ([ADR-067](../../plans/decisions/067-daemon-symbol-feed-parse-hook.md)); the
  frozen save-time wire
  ([ADR-061](../../plans/decisions/061-save-time-daemon-delta-validation.md)).
  The backing can swap (interim cache → GV2 warm index → persistence) with zero
  wire change.
- **graph ↔ drivers (DRVR)** — **pinned by reference.** ADR-063 binds DRVR
  mid-edit reads to the _same_ hot-read allowlist; a second admission policy is
  explicitly rejected.
- **graph ↔ control/session** — **defined, cite don't redesign.** The session
  model is shipped in INTD: `SessionRecord` / `SessionId` and
  `Attribution::Owned(SessionRecord)`
  ([`intercept-as-built.md`](./intercept-as-built.md) §10). GV2-013 joins to
  these types; it does not invent a session model.
- **graph ↔ MCP/context (GCTX)** — **pinned.** GCTX consumes projections, must
  not define GV2 schemas, and GV2 wins on conflict; mutually acknowledged in
  both module specs. GCTX-002 (which server hosts it) is a sequencing decision,
  not a seam gap.
- **graph ↔ trust/policy** — **partial.** Trust levels on nodes are shipped
  (`TrustLevel`: `Unknown`/`Internal`/`Boundary`/`External`/`Privileged`,
  `crates/anvil-kernel-types/src/trust.rs`); the richer trust/policy _graph_
  contract (side-effect surfaces, data classes, policy evidence) is Draft inside
  GV2-012. No separate authoritative trust-graph doc exists yet.
- **graph ↔ provenance/edda-stack** — **the open seam.** Provenance is
  authoritatively specified in **TypeScript** (Kindling → Ember → Edda,
  [`edda-stack.md`](./edda-stack.md)); the Rust-side counterpart
  (`eddacraft-kindling`) is still proposed. GV2-014's join therefore bridges a
  **language boundary to a surface that does not yet exist in Rust**. This is
  the one seam where "not designed yet" is genuinely true, and it is a
  provenance/kindling question, not a graph-layer one.

## Persistence and derivability invariants

Binding on every graph (full mechanics in
[ADR-069](../../plans/decisions/069-graph-v2-persistence.md)):

- Persisted graph state is **derivable cache**, never authoritative, unless an
  ADR says otherwise.
- Snapshots version their schema (`format_version` + `backing_schema_version`)
  and **cold-rebuild** on mismatch/corruption — never panic, never refuse to
  start. A backing swap bumps the version (one cold rebuild).
- **Warm-start restores indexes, never the verdict.** A restored workspace comes
  up `stale`/`pending` and re-certifies.
- **Privacy line:** persist structural identity (symbol names, import/path
  identity, edges, content hashes) needed for boundary checks; **never** persist
  raw source bodies, snippets, comment text, or secret-shaped literals. Source
  spans are no-text byte ranges. The privacy gate is a sealed-allowlist DTO with
  a structural no-leak test, not a review convention.
- Persistence is **default-off** until that guard is green.

## What this spec deliberately does not freeze

- Per-graph field schemas (GV2-010/012/013/014 own them, each with its own
  design stage before implementation).
- The hot-read admission rule, crate boundary, persistence mechanics, and
  save-time wire — owned by ADR-063/064/069/061 respectively; this spec only
  summarises them.
- Implementation of the registry, the hot-read type split, the benchmark gate,
  and the backing swap — those are the GV2 wave's build items.

## Known gaps

### G-01: Stable identity is unpinned and load-bearing

`SymbolNode.id` is a session-local `u64`; the certify set-key conflates identity
with position. Blocks precise export diffing, warm-start snapshot comparability,
and the provenance/trust joins. **Risk:** High. **Fix:** GV2-002.

### G-02: Rust-side provenance surface does not exist

GV2-014 joins to `edda-stack`, which is authoritative in TS; the Rust
counterpart is proposed only. **Risk:** Medium (only the provenance join, not
the A′ swap). **Fix:** a provenance/kindling design stage; tracked as the open
seam above.

### G-03: Trust/policy graph contract is Draft and undocumented outside GV2

No authoritative trust-graph doc; the contract lives only inside GV2-012's
assumptions, and `annotate_trust` is not yet wired on the daemon certify path
(privilege-containment risk). **Risk:** Medium. **Fix:** GV2-012 + the daemon
trust-wiring item.

### G-04: Top-level architecture docs predate Graph v2

`rust-architecture-endstate.md` still shows the daemon as deferred and never
mentions Graph v2; `anvil-full-architecture.md` has no joined-graph section. A
reader of `docs/architecture/` alone would not learn this model exists.
**Risk:** Low. **Fix:** a one-paragraph pointer in `anvil-full-architecture.md`
and a freshness pass on `rust-architecture-endstate.md`.

## Related docs

- Module plan:
  [`plans/modules/graph-v2-foundation.aps.md`](../../plans/modules/graph-v2-foundation.aps.md)
- Wave verdict:
  [`plans/reviews/2026-06-05-gv2-wave-planning-council-verdict.md`](../../plans/reviews/2026-06-05-gv2-wave-planning-council-verdict.md)
- ADRs: [061](../../plans/decisions/061-save-time-daemon-delta-validation.md),
  [063](../../plans/decisions/063-gv2-hot-path-boundary.md),
  [064](../../plans/decisions/064-intercept-graph-cache-crate-boundary.md),
  [067](../../plans/decisions/067-daemon-symbol-feed-parse-hook.md),
  [069](../../plans/decisions/069-graph-v2-persistence.md)
- Save-time contract:
  [`plans/specs/2026-06-01-daemon-save-time-validation-contract.md`](../../plans/specs/2026-06-01-daemon-save-time-validation-contract.md)
- Seam sources: [`intercept-as-built.md`](./intercept-as-built.md) §10,
  [`edda-stack.md`](./edda-stack.md)
- Consumers:
  [`plans/modules/graph-context-delivery.aps.md`](../../plans/modules/graph-context-delivery.aps.md)
