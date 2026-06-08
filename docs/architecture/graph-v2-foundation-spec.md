# Graph v2 Foundation — Architecture Spec

| Type | Authority | Owner                                                                                              | Status | Freshness                                                                                                                                                          |
| ---- | --------- | -------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Spec | Derived   | GV2 ([`plans/modules/graph-v2-foundation.aps.md`](../../plans/modules/graph-v2-foundation.aps.md)) | Live   | Taxonomy **ratified 2026-06-08** by council `plan-ec495f8b` (RATIFY-WITH-FIXES; conditions C-1..C-6 folded). Synthesis of ADR-061/063/064/067/069 + the GV2 module |

| Upstream                                                                                                                                                                                              | Downstream                                                                                                                                              |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADR-061, ADR-063, ADR-064, ADR-067, ADR-069, ADR-031, `crates/anvil-kernel-types`, `crates/anvil-graph-cache`, [`intercept-as-built.md`](./intercept-as-built.md), [`edda-stack.md`](./edda-stack.md) | `graph-context-delivery` (GCTX), `surface-drivers` (DRVR), `multilayer-protection-v2` (INTD), `weave` (WEAVE), the daemon save-time validation contract |

## Purpose and scope

This is the **spine** of Graph v2: the one place that states the joined-graph
model, the cross-graph identity contract, the join model, the query/registry API
shape, and the seams to other subsystems. It exists because that model is
**already decided** — ratified across ADR-061/063/064/067/069 and structured in
the GV2 module — but was never written down in one artefact that the work items
and ADRs all cite.

This document **synthesises and reconciles**; it does not re-decide. Where a
decision is frozen by an ADR, this spec points at the ADR and does not restate
its reasoning. The taxonomy was ratified 2026-06-08 by council `plan-ec495f8b`
(RATIFY-WITH-FIXES); the corrections it required are folded in below and tracked
in
[the ratification verdict](../../plans/reviews/2026-06-08-gv2-taxonomy-ratification-verdict.md).

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

The `SymbolNode.id: u64` (`crates/anvil-kernel-types/src/graph.rs`) remains a
**session-local monotonic counter** used as the in-memory graph handle — it is
not, and need not be, stable. The _comparable_ identity is `SymbolIdentity`
(`anvil-kernel-types`, GV2-002): `(file, kind, name, ordinal)`, where `ordinal`
is the occurrence index among same-`(kind, name)` symbols in parse order — a
structural overload disambiguator derived from source ordering, never from
parameter source text (privacy verdict PV-1). The save-time certify baselines
(`previously_public` / `previously_privileged`) and the `export_surface_diff`
primitive are keyed on it, so same-`(kind, name)` overloads no longer collapse
and a rename is classified rather than read as unrelated churn. **Rename
stance:** rename = delete + create at both file and symbol level; rename
classification is a per-update, in-memory pairing and no pre-rename name is
retained or persisted (privacy verdict PV-4). Session/worktree identity and
APS/provenance references remain join-time-only contract rows — resolved from
their graph authorities, never persisted graph fields (privacy verdict PV-3).
Warm-start snapshot comparability never required stable identity — ADR-069
persists the `u64` ids in its sealed DTO and reconciles by content hash — so
GV2-002 gated GV2-014 and precise export-diffing, **not** Sub-phase B
persistence (ratification condition C-4).

### The identity keys that cross graph boundaries

| Identity                                                               | Authoritative graph   | Stable across restart today?                                         | Crosses into                  |
| ---------------------------------------------------------------------- | --------------------- | -------------------------------------------------------------------- | ----------------------------- |
| File identity (path + content hash)                                    | Semantic              | path yes / content-hash yes                                          | dependency, trust, provenance |
| Symbol identity (stable, position-independent, overload-disambiguated) | Semantic (GV2-002)    | yes — `SymbolIdentity` `(file, kind, name, ordinal)` (GV2-002)       | trust, provenance             |
| Edge identity (typed `from → to` over symbol/file identity)            | Semantic / dependency | derived                                                              | dependency, provenance        |
| Session / worktree identity (`SessionId`, `WorktreeKey`)               | Control/session       | yes — but `WorktreeKey` is crate-private to `anvil-intercept` (G-05) | provenance, attribution       |
| Plan / commit / memory anchors (APS id, commit SHA, Edda ref)          | Plan/provenance       | external                                                             | provenance joins              |

**Rule:** every graph references foreign nodes **only** through the identity
owned by the foreign graph — never by storing a copy of the foreign node. A join
is an identity lookup, not a structural embedding.

## The join model

A join is a typed query that follows one graph's identity into another. The
spine fixes _which_ joins exist and _what key_ each bridges; the per-graph items
own the field detail. The "key bridged" column states what the **shipped
substrate** can follow today — where a finer key is the freeze-target, it is
named explicitly.

| Join                       | Key bridged                                                                         | Consuming query (example)                                        |
| -------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| semantic ↔ dependency      | file identity (symbol-granular is a freeze-target — GV2-011 symbol edges + GV2-002) | "what imports file F?" (`dependents_of`, file-keyed today)       |
| semantic ↔ trust           | symbol identity                                                                     | "what is the trust level / side-effect surface of S?"            |
| dependency ↔ trust         | file/edge identity                                                                  | "does this new edge cross a trust boundary?"                     |
| control/session ↔ semantic | worktree → file (**bridge undesigned — G-05**)                                      | "which session authored the change to F?" (`Attribution::Owned`) |
| plan/provenance ↔ all      | symbol/file/commit/session anchors                                                  | "why was this structural change allowed or challenged?"          |

### Worked join trace (the spec's proof)

This is GV2-014's own validation scenario, expressed as a path through the
joins. One code change must resolve to one trace. Per ratification condition
C-2, each hop names the identity it actually bridges in the shipped substrate:

```text
edit src/pay.ts ──semantic──▶ symbol `chargeCard` (Boundary trust)
   │                              │
   │                          trust join ──▶ side-effect surface: network
   │
 dependency join ──▶ dependents_of(file of chargeCard = src/pay.ts) = [checkout.ts]
   │                  (file-keyed today; symbol-granular needs GV2-011 edges + GV2-002)
 control/session join ──▶ Attribution::Owned(SessionRecord{id})
   │                  (who saved it — worktree→file bridge undesigned, G-05)
 plan/provenance join ──▶ APS item PAY-007 · commit <sha> · policy: allowed · Edda memory <ref>
```

If any single join in that chain cannot be followed by a defined identity key,
the trace breaks — which is why GV2-002 gates GV2-014 and why G-05 (the
worktree→file bridge) gates the control/session join.

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

| Read class                                                                                                     | Admissibility | Surface                |
| -------------------------------------------------------------------------------------------------------------- | ------------- | ---------------------- |
| resident per-file symbol/extract lookup                                                                        | hot           | hot-read API (GV2-022) |
| known-edge existence (`A → B`?)                                                                                | hot           | hot-read API           |
| bounded reverse impact (depth ≤ hard cap — **freeze-target**; today file-count-budgeted, depth cap is GV2-026) | hot           | hot-read API           |
| precomputed architectural-index check                                                                          | hot           | hot-read API           |
| parse / re-extract / cross-file resolution                                                                     | **non-hot**   | background pool only   |
| transitive impact beyond cap, full scans, index rebuild, persistence load                                      | **non-hot**   | background pool only   |

Every hot read returns an explicit `warm` / `stale` marker; a warm miss maps to
a typed `StaleReason` and **degrades** — it never escalates to a parse, rebuild,
or I/O on the hot path. The split is enforced by a type boundary (non-admissible
ops are not callable from the hot-read API) plus an ADR-031 Criterion benchmark
that fails CI on budget regression. Note (C-3): the "depth ≤ hard cap" is the
admission contract to freeze, but the shipped `impact_closure` enforces a
**file-count budget, not a hop-depth cap** (`certify.rs:149-165`) — a
star-shaped graph can reach all importers in "one hop". The genuine depth cap is
GV2-026; the GV2-025 benchmark must gate against it, not against the current
budget.

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
  wire change. _(C-5: `intercept-as-built.md` was last reviewed 2026-05-07 and
  predates ADR-067 (2026-06-03), so the symbol-feed pin is verifiable from
  ADR-067 only until that as-built is refreshed — see G-04.)_
- **graph ↔ drivers (DRVR)** — **pinned by reference.** ADR-063 binds DRVR
  mid-edit reads to the _same_ hot-read allowlist; a second admission policy is
  explicitly rejected.
- **graph ↔ control/session** — **types shipped; join contract undesigned**
  (C-5). The session model is shipped in INTD (`SessionRecord` / `SessionId` /
  `Attribution::Owned(SessionRecord)`,
  [`intercept-as-built.md`](./intercept-as-built.md) §10), and GV2-013 cites
  those types rather than reinventing them — but the **worktree→file join key
  bridge is undesigned** (G-05): `SessionRecord.worktree` is an absolute
  `PathBuf` and `WorktreeKey` is crate-private to `anvil-intercept`, so
  relativising it to the graph's relative file keys needs a shared type GV2-013
  must define (not an `anvil-intercept` dependency, which would invert ADR-064).
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
  **language boundary to a surface that does not yet exist in Rust**, and the
  durable Edda store is git-committed and shareable — **outside** the same-uid
  boundary the daemon snapshot relies on (see the per-graph privacy scope
  below). This is the one seam where "not designed yet" is genuinely true, and
  it is a provenance/kindling question, not a graph-layer one.

## Persistence and derivability invariants

Binding on every persistable graph (full mechanics in
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
- **Per-graph scope (ratification condition C-6).** ADR-069's sealed-DTO +
  no-leak enforcement and its same-uid residual-risk acceptance are **proven
  only for the daemon semantic/dependency snapshot**. The control/session graph
  (`SessionRecord.worktree` is absolute → home-dir/PII) and the plan/provenance
  graph (Edda is git-committed and shareable — outside the same-uid boundary) do
  **not** inherit that acceptance; each needs its **own** privacy ADR before it
  becomes persistable. The relative-path / no-home-prefix / no-PII rules bind
  GV2-013 before persistence, and the GV2-014 Edda join must be **ref-only** —
  memory/commit/session refs + structural anchors, never inline memory bodies or
  secret-shaped literals.

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
with position. Blocks **precise export diffing** (overload collapse) and the
**provenance/trust symbol joins**. Does **not** block warm-start snapshot
comparability (ADR-069's DTO persists the ids and reconciles by content hash).
**Risk:** High. **Fix:** GV2-002 — gates GV2-014, not Sub-phase B persistence.

### G-02: Rust-side provenance surface does not exist

GV2-014 joins to `edda-stack`, which is authoritative in TS; the Rust
counterpart is proposed only. **Risk:** Medium (only the provenance join, not
the A′ swap). **Fix:** a provenance/kindling design stage; tracked as the open
seam above.

### G-03: Trust/policy graph contract is Draft and undocumented outside GV2

No authoritative trust-graph doc; the contract lives only inside GV2-012's
assumptions, and `annotate_trust` is not yet wired on the daemon certify path
(privilege-containment risk). **Risk:** Medium. **Fix:** GV2-012 + the daemon
trust-wiring item (GV2-029).

### G-04: Top-level architecture docs predate Graph v2

`rust-architecture-endstate.md` still shows the daemon as deferred and never
mentions Graph v2; `anvil-full-architecture.md` has no joined-graph section; and
`intercept-as-built.md` (reviewed 2026-05-07) predates ADR-067. A reader of
`docs/architecture/` alone would not learn this model exists or that the INTD
symbol-feed seam is pinned. **Risk:** Low. **Fix:** a one-paragraph pointer in
`anvil-full-architecture.md`, a freshness pass on
`rust-architecture-endstate.md`, and an `intercept-as-built.md` refresh covering
ADR-067.

### G-05: The control/session → file join bridge is undesigned

`SessionRecord.worktree` is an absolute, canonicalised `PathBuf`; `WorktreeKey`
is crate-private to `anvil-intercept` (`rule_cache.rs`); the semantic/dependency
graphs key files by relative `String`. Relativising worktree-root → file
identity is undesigned and untyped, and importing `WorktreeKey` into the graph
layer would invert the ADR-064 boundary. Gates the control/session join in the
worked trace. **Risk:** Medium. **Fix:** GV2-013 defines a shared
root-relativisation type in `anvil-kernel-types`.

### G-06: `TrustLevel::Boundary` is excluded from the privileged-surface baseline — closed

**Closed by GV2-002.** `incremental::is_elevated_trust` feeds both
`TrustLevel::Privileged` and `TrustLevel::Boundary` into
`previously_privileged`, and `certify::export_surface_diff` applies the same
predicate to the post-update surface, so a producer emitting `Boundary` can no
longer make the export-diff silently under-fire (regression-tested in
`certify`).

## Related docs

- Module plan:
  [`plans/modules/graph-v2-foundation.aps.md`](../../plans/modules/graph-v2-foundation.aps.md)
- Ratification verdict:
  [`plans/reviews/2026-06-08-gv2-taxonomy-ratification-verdict.md`](../../plans/reviews/2026-06-08-gv2-taxonomy-ratification-verdict.md)
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
