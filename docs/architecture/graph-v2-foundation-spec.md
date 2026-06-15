# Graph v2 Foundation — Architecture Spec

| Type | Authority | Owner                                                                                              | Status | Freshness                                                                                                                                                                                                                                                                            |
| ---- | --------- | -------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Spec | Derived   | GV2 ([`plans/modules/graph-v2-foundation.aps.md`](../../plans/modules/graph-v2-foundation.aps.md)) | Live   | Taxonomy **ratified 2026-06-08** by council `plan-ec495f8b` (RATIFY-WITH-FIXES; conditions C-1..C-6 folded). Control/session + plan/provenance graph contracts added 2026-06-13 (GV2-013/GV2-014; G-05/G-02 contract-defined). Synthesis of ADR-061/063/064/067/069 + the GV2 module |

| Upstream                                                                                                                                                                                                                                                                                                                         | Downstream                                                                                                                                              |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADR-061, ADR-063, ADR-064, ADR-067, ADR-069, ADR-031, `crates/anvil-kernel-types`, `crates/anvil-graph-cache`, [`intercept-as-built.md`](./intercept-as-built.md), [`edda-stack.md`](./edda-stack.md), [`anvil-driver-framework-design-spec.md`](../../plans/specs/anvil-driver-framework/anvil-driver-framework-design-spec.md) | `graph-context-delivery` (GCTX), `surface-drivers` (DRVR), `multilayer-protection-v2` (INTD), `weave` (WEAVE), the daemon save-time validation contract |

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

| Layer                                   | Where it lives today                                                                                                                                 | State                                                                           |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Semantic code graph                     | `crates/anvil-graph-cache/src/symbol_graph.rs`, `crates/anvil-kernel-types/src/graph.rs`                                                             | Shipped (Sub-phase A); schema additions pending (GV2-010)                       |
| Dependency/impact graph + reverse index | `crates/anvil-graph-cache/src/dependency.rs`                                                                                                         | Shipped; incremental maintenance + hot-read API pending (GV2-011/022)           |
| Trust/policy graph                      | `PolicyProfile`/`TrustGraph` keyed on `SymbolIdentity` (`crates/anvil-kernel-types/src/trust.rs`, `crates/anvil-graph-cache/src/trust.rs`)           | Contract defined (GV2-012); daemon wiring pending (GV2-029)                     |
| Control/session graph                   | shipped **in INTD** as `SessionRecord`/`Attribution` ([`intercept-as-built.md`](./intercept-as-built.md) §10); join contract defined below (GV2-013) | Contract defined (GV2-013); bridge type + registry wiring pending (GV2-020)     |
| Plan/provenance graph                   | provenance shipped **in TS** ([`edda-stack.md`](./edda-stack.md)); join contract defined below (GV2-014)                                             | Contract defined (GV2-014); Rust read surface (`eddacraft-kindling`) proposed   |
| Registry + query traits                 | none yet — consumers call `certify()`/`with_graphs()` directly                                                                                       | Consumer contract defined (GV2-023, see below); registry impl pending (GV2-020) |

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

| Identity                                                               | Authoritative graph   | Stable across restart today?                                                                                                        | Crosses into                  |
| ---------------------------------------------------------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| File identity (path + content hash)                                    | Semantic              | path yes / content-hash yes                                                                                                         | dependency, trust, provenance |
| Symbol identity (stable, position-independent, overload-disambiguated) | Semantic (GV2-002)    | yes — `SymbolIdentity` `(file, kind, name, ordinal)` (GV2-002)                                                                      | trust, provenance             |
| Edge identity (typed `from → to` over symbol/file identity)            | Semantic / dependency | derived                                                                                                                             | dependency, provenance        |
| Session / worktree identity (`SessionId`, `WorktreeKey`)               | Control/session       | yes — `WorktreeKey` stays internal to `anvil-intercept`; the cross-graph join uses the shared `WorkspaceRoot` relativiser (GV2-013) | provenance, attribution       |
| Plan / commit / memory anchors (APS id, commit SHA, Edda ref)          | Plan/provenance       | external                                                                                                                            | provenance joins              |

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
| control/session ↔ semantic | worktree → file via `WorkspaceRoot::relativise` (**bridge defined — GV2-013**)      | "which session authored the change to F?" (`Attribution::Owned`) |
| plan/provenance ↔ all      | symbol/file/commit/session anchors (**ref-only — GV2-014**)                         | "why was this structural change allowed or challenged?"          |

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
 control/session join ──▶ Attribution::Owned { session: SessionRecord{id} }
   │                  (who saved it — worktree→file bridge via `WorkspaceRoot`, GV2-013)
 plan/provenance join ──▶ APS item PAY-007 · commit <sha> · policy: allowed · Edda memory <ref>
                      (ref-only anchors, APS optional — GV2-014; see "The plan/provenance graph contract")
```

If any single join in that chain cannot be followed by a defined identity key,
the trace breaks — which is why GV2-002 gates GV2-014, and why the
control/session join needs the worktree→file bridge now defined by GV2-013 (the
`WorkspaceRoot` relativiser, see "The control/session graph contract" below).

## The trust/policy graph contract (GV2-012)

The trust/policy graph is a **separate graph keyed on `SymbolIdentity`**, not a
bag of fields on the semantic node. It joins to the semantic graph via symbol
identity (the semantic ↔ trust join above) and never embeds a semantic node, so
a trust verdict can change without rewriting the code graph and a code edit can
re-key without losing its policy meaning. The contract types live in
`crates/anvil-kernel-types/src/trust.rs`; the store (`TrustGraph`) and its delta
logic live in `crates/anvil-graph-cache/src/trust.rs`.

### What a symbol's `PolicyProfile` carries

- **Trust level** — `TrustLevel`
  (`Unknown`/`Internal`/`Boundary`/`External`/`Privileged`), the single trust
  axis (shipped Sub-phase A).
- **Side-effect surfaces** — a `BTreeSet<SideEffectSurface>`: `Network`,
  `Filesystem`, `Process`, `Crypto`, `Environment`. The v0.8 import heuristic
  populates the four module-derived surfaces; `Environment` is part of the
  vocabulary for the config-access producer.
- **Data classifications** — a `BTreeSet<DataClassification>`
  (`Unknown`→`Public`→`Internal`→`Confidential`→`Secret`), ordered least-to-most
  sensitive.
- **Invariant guards** — a `BTreeSet<InvariantGuard>` naming the save-time
  guards that watch the symbol: `NewDependencyIntroduction`,
  `PrivilegeExpansion`, `ApiSurfaceExpansion`.
- **Policy evidence** — a `Vec<PolicyEvidence>`; each record anchors to a
  `SymbolIdentity` plus an optional no-text `ByteRange` span and **resolves back
  to source** via `PolicyEvidence::resolve()` (file + span). It carries no
  source text (privacy verdict PV-7(e)).
- **Override source** — `OverrideSource`
  (`Heuristic`/`Configuration`/`Baseline`/`Annotation`), so every verdict is
  auditable.

### Scope guard

These are **declarative classifications joined to a symbol**, derived from
bounded, local evidence (e.g. a file's import set) — **not** full
interprocedural data-flow analysis, which GV2-012 explicitly excludes. Richer
producers may refine the same fields later without changing the contract.

### Trust posture changes are emitted as deltas

`TrustGraph::posture_delta(before, after)` diffs two trust graphs into a
deterministic, identity-ordered `Vec<TrustPostureChange>`
(`Classified`/`Reclassified`/`Declassified`); `posture_changes_for_delta` scopes
that diff to the symbols a semantic `GraphDelta` actually touched, so a
save-time update emits exactly the trust posture changes for the symbols that
moved. `TrustPostureChange::is_privilege_escalation()` flags a move onto
`Privileged` — the signal the daemon trust-annotation wiring (GV2-029) acts on.

## The control/session graph contract (GV2-013)

The control/session graph is a **separate graph keyed on session and worktree
identity**, not a bag of fields on the semantic node. It owns execution hosts,
drivers, sessions, leases, fences, worktrees, and the attribution of _changes_
to sessions; it **must not** model code structure (the taxonomy anti-scope
above). It joins to the semantic graph in one direction only — worktree-root →
file — to answer "which session authored the change to F?", and never embeds a
semantic node, so a session can end without rewriting the code graph and a code
edit re-keys without losing its attribution. The contract **cites the shipped
INTD session model rather than inventing one**: `SessionRecord`, `SessionId`,
`SessionStatus`, and the `Attribution` enum (`Owned { session: SessionRecord }`
/ `Unknown`) are the authoritative types
([`intercept-as-built.md`](./intercept-as-built.md) §10;
`crates/anvil-intercept-proto/src/lib.rs`,
`crates/anvil-intercept/src/registry.rs`), with the session-key vocabulary
(`AgentTag`, `LineageAnchor`, `OperatorContext`) in
`crates/anvil-intercept-proto/src/session.rs`. The control plane that mints and
governs these is specified by the driver framework
([`anvil-driver-framework-design-spec.md`](../../plans/specs/anvil-driver-framework/anvil-driver-framework-design-spec.md));
this graph is the queryable projection of that runtime state, **not** a second
control authority.

### What the control/session graph carries

- **Execution hosts** — where a session runs (local, remote/SSH, hosted/web),
  from the driver framework's execution-host registry. The host bounds what
  enforcement the session's driver can offer.
- **Drivers** — the `DriverType` that launched a session (shell-local,
  shell-remote, process, tmux, editor, web-session, mcp, other) and its reported
  `EnforcementCapabilities`. The driver, not the graph, decides what control is
  possible; the graph records which driver owns a session so attribution
  confidence is legible.
- **Sessions** — `SessionRecord` keyed by `SessionId`, with `SessionStatus`
  (`Active`/`Ended`) and the `agent_tag` / `daemon_issued_tag` attribution
  identity.
- **Leases and fences** — the lease a session holds over a worktree/repo and the
  fence state that gates enforcement; control-plane state the graph references
  by session/worktree identity, not state it authors.
- **Worktrees** — the worktree root a session is bound to. The cross-graph join
  key is derived from this (the bridge below); the daemon's `WorktreeKey` (fence
  keying, internal to `anvil-intercept`) is **not** exposed here.
- **Attribution edges** — `Attribution::Owned { session }` links a changed _file
  identity_ to the session that authored it; `Attribution::Unknown` is the
  explicit no-owner case.

### Field classification: hot-path, telemetry, provenance

The join's hot-path obligation (ADR-063) is narrow: answering "which active
session owns this path?" must be `O(1)` from warm control-plane state with no
parse and no I/O. Each field has one primary role (a secondary role is noted in
the reason):

| Field                                                         | Role                         | Why                                                                                                  |
| ------------------------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------- |
| `SessionId`, `SessionStatus`                                  | **hot-path**                 | attribution lookup + active/ended gate                                                               |
| worktree-root → file key (relativised; see bridge)            | **hot-path**                 | the join key for `Attribution::Owned`                                                                |
| `pid` / `pgid`                                                | **hot-path**                 | process-group fencing/control (also provenance for audit)                                            |
| `AgentTag.pid_starttime`, `LineageAnchor`                     | **hot-path**                 | PID-reuse defence on the lineage cross-check                                                         |
| `started_at_unix`                                             | **hot-path**                 | deterministic tiebreaker in `attribute_path` (registry.rs); also registration-time PID-reuse defence |
| `last_heartbeat_unix`                                         | **telemetry-only**           | liveness/TTL refresh; observability, not a verdict input                                             |
| `OperatorContext` (`uid`/`pid`/`hostname`)                    | **persisted-for-provenance** | audit record on a cascade clear (server-populated, never client-trusted)                             |
| `agent_tag.driver_id`/`claimed_agent_id`, `daemon_issued_tag` | **persisted-for-provenance** | the _who / which-driver_ the plan/provenance join (GV2-014) reads                                    |
| `SessionRecord.worktree` (absolute `PathBuf`)                 | **never persisted**          | absolute path is home-dir/PII; only its relativised key crosses the join (C-6)                       |

Telemetry-only and provenance fields are **off the hot path**: they feed the
diagnostic/projection API tier, not the enforcement read. A warm miss on the
attribution lookup degrades to `Attribution::Unknown` — never a parse, rebuild,
or I/O.

### The worktree-root → file join bridge (closes G-05 / ratification C-1)

C-1's undesigned seam: `SessionRecord.worktree` is an absolute, canonicalised
`PathBuf`, the relativisation logic lives in `anvil-intercept`'s `WorktreeKey`
(`rule_cache.rs`, kept there for fence keying and not consumed cross-crate), and
the semantic/dependency graphs key files by relative `String` — so the
attribution hop had no shared key. The fix is not to expose `WorktreeKey`: the
graph layer (`anvil-graph-cache`) must not depend on the control layer
(`anvil-intercept`) at all, which is exactly the ADR-064 inversion. The contract
instead defines a shared relativiser in **`anvil-kernel-types`** — the crate
both `anvil-intercept` (Cargo.toml) and `anvil-graph-cache` (Cargo.toml) already
depend on, so neither takes a dependency on the other:

- **`WorkspaceRoot`** — a newtype over a canonical, absolute worktree root
  (constructed by the control layer from `SessionRecord.worktree`). It exposes
  one join operation: `relativise(&self, abs: &Path) -> Option<RelPath>`.
- **`RelPath`** — the workspace-root-relative, **forward-slash-normalised** key
  (the same `String` the semantic graph already uses for `SymbolNode.file` /
  `SourceLocation`, optionally newtyped for type-safety), so the relativised
  result looks up directly with no second keyspace.

Contract invariants (named, not frozen — the type lands in code with the
registry build, GV2-020):

1. **Deterministic and platform-stable** — canonicalisation happens once at
   construction; output is forward-slash-normalised so the key is identical on
   Unix and Windows (graph keys are portable `String`s).
2. **Total, never panicking** — a path outside the root returns `None`, which
   the attribution join maps to `Attribution::Unknown`; it never errors on the
   hot path.
3. **No PII retained** — the relative key strips the absolute root, so no
   home-dir prefix or username survives into the join or any persisted
   projection (C-6).
4. **Boundary-preserving** — it consolidates _cross-graph_ relativisation only;
   `WorktreeKey` stays internal to `anvil-intercept` for fence-cache keying. The
   two are distinct concerns, and the graph layer reaches neither — it depends
   only on `anvil-kernel-types`.

This makes the worked-trace `control/session join → Attribution::Owned` hop
followable by a defined key.

### Driver coverage: shell, editor, MCP

Attribution confidence is a function of the driver's `EnforcementCapabilities`,
and the graph records the `DriverType` so a consumer can read how strong an
`Attribution::Owned` edge is:

- **Shell** (shell-local/-remote, tmux, process) — strongest. The launcher wraps
  the process, so the session carries `pid`/`pgid` and a `LineageAnchor`;
  attribution is by process-group lineage _and_ worktree containment, and
  fencing is enforceable on the hot path.
- **Editor** — medium. An editor save may not run under a wrapped process group,
  so attribution is by **worktree containment** (the saved path relativises
  under a registered session's `WorkspaceRoot`) rather than pgid lineage. The
  edge is still `Attribution::Owned`, but the graph marks the driver so
  consumers know lineage is path-derived.
- **MCP** — weakest / best-effort (driver framework §6.2 `mcp-driver`: soft
  control, non-primary fallback; hosted/web sessions are §14). An MCP session
  may have no local pgid and only an _advisory_ `AgentTag` (`claimed_agent_id`),
  not a `daemon_issued_tag`. Attribution falls back to worktree containment
  where a root is known, and to `Attribution::Unknown` otherwise — the graph
  never fabricates a stronger edge than the driver can support. The
  `web-session` driver shares this tier (best-effort, frequently no local pgid).

In all three cases the graph models the **same** session/attribution types; only
the confidence and the available identity fields differ. The control plane stays
in the driver framework; the graph is its read projection.

### Privacy and persistence scope (ratification C-6)

The control/session graph does **not** inherit ADR-069's same-uid persistence
acceptance (proven only for the daemon semantic/dependency snapshot).
`SessionRecord.worktree` is absolute (home-dir/PII), so the relative-path /
no-home-prefix / no-PII rules bind this graph _before_ any persistence, and it
needs its **own** privacy ADR before it becomes persistable. Until then the
control/session graph is **resident control-plane state projected on demand**,
not a persisted snapshot.

## The plan/provenance graph contract (GV2-014)

The plan/provenance graph is the **join layer that explains a structural
change**: it ties a code change to the APS work item, commit, graph delta,
policy decision, trust-posture change, and Edda memory that surround it, so
Anvil can answer "why was this allowed or challenged?". It is **keyed entirely
on external anchors** — APS id, commit SHA, `SymbolIdentity` / file identity
(GV2-002), and Edda memory ref — and **references** every foreign record rather
than embedding it (the identity rule above). Two principles bind it harder than
any other graph:

- **Planless-first (principle 5).** APS anchors are _optional enrichment_. The
  join works from source + git + Edda alone; a present APS item enriches the
  trace, but the contract must **never make APS a runtime prerequisite** (the
  taxonomy anti-scope).
- **One provenance contract, shared with EDDA-SEAL — not a second.** Provenance
  is authoritatively modelled in TypeScript by the edda-stack contracts
  (`packages/edda-stack/src/contracts/provenance.ts`: `ProvenanceChain`,
  `KindlingRef`, `EmberRef`, `Attribution`; memories in `edda-memory.ts`), and
  the **proposed** EDDA-SEAL module
  ([roadmap](../../plans/brainstorms/git-native-governance/roadmap.md),
  EDDA-SEAL-002/003) would seal that same chain into a git-committed
  `anvil.edda-provenance.v1` bundle at promotion time. GV2-014 **reuses that
  contract by reference**; it does not define a graph-layer provenance type.

### What the plan/provenance graph carries (all by reference)

| Anchor               | Owned by                | Reference key                                                  |
| -------------------- | ----------------------- | -------------------------------------------------------------- |
| APS work item        | Anvil plan (optional)   | APS id (e.g. `PAY-007`) → `ProvenanceChain.related_plans`      |
| Commit               | git                     | commit SHA                                                     |
| Change / graph-state | semantic (GV2-003)      | `GraphDelta` ref (`schema_version` + touched `SymbolIdentity`) |
| Policy decision      | trust / certify         | certify verdict (`allow` / `warn` / `block`) + evidence ref    |
| Trust-posture change | trust/policy (GV2-012)  | `TrustPostureChange` (identity-anchored)                       |
| Memory / provenance  | Edda (TS-authoritative) | Edda memory ref (`MemoryId`) → `ProvenanceChain`               |

None of these is stored as a copied body in the graph layer; each row is an
identity lookup into the owning authority.

### Resolving the Rust↔TS provenance boundary (closes G-02)

G-02 was real: provenance is authoritative in TS and the Rust counterpart
(`eddacraft-kindling`) is still proposed, so a Rust graph join "to a surface
that does not exist in Rust" looked undesigned. The contract resolves it by
making the join **ref-only across the language boundary**:

1. **Anchors cross the boundary; bodies do not.** The Rust graph layer holds
   only the anchor keys above (APS id, commit SHA, `SymbolIdentity`,
   `MemoryId`). The provenance body — the `ProvenanceChain` and its memory —
   stays in the TS-authoritative Edda store (and, once the proposed EDDA-SEAL
   lands, durably in its git-committed `anvil.edda-provenance.v1` bundle).
2. **Resolution is a projection-tier read, off the hot path.** Following a
   `MemoryId` to the full chain is a diagnostic/projection query (ADR-063
   non-hot), so the join never blocks enforcement and never requires the Rust
   provenance surface to be resident.
3. **The contract does not wait on `eddacraft-kindling`.** Because the anchors
   are sufficient to _express_ the join, the contract is followable today
   against the **shipped** TS contracts (the `ProvenanceChain` the fixture
   validates); the git-committed sealed bundle is the target durable form once
   EDDA-SEAL lands, and the proposed Rust `eddacraft-kindling` read surface is
   an _implementation_ of step 2 — neither is a prerequisite of the contract.

This is the same move as the control/session bridge: identify the shared key,
keep each body with its owning authority, and forbid the cross-layer dependency.

### Privacy scope (ratification C-6)

The Edda store is **git-committed and shareable — outside** the same-uid
boundary the daemon snapshot relies on, so this graph does **not** inherit
ADR-069's persistence acceptance and needs its **own** privacy ADR before any
graph-side persistence. The join is **ref-only by contract**: memory / commit /
session refs plus structural anchors only — **never** inline memory bodies,
rationale prose, or secret-shaped literals cross into the graph layer or any
projection.

### Worked trace and fixture

The spine's worked join trace ("Worked join trace" above) is GV2-014's
validation scenario. It is realised as an executable fixture in
`packages/edda-stack/src/contracts/provenance-join.test.ts`, which links one
code change (`src/pay.ts` → `chargeCard`) to its APS item (`PAY-007`,
_optional_), commit, `GraphDelta`, trust-posture change, policy verdict, and a
`ProvenanceChain`-validated Edda record — plus a **planless variant** with no
APS anchor that still resolves, proving APS is not a prerequisite.

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
- **Background query API** (GV2-020 registry → GV2-023 consumer contract) — the
  diagnostic, provenance, and context-projection classes (see "The consumer
  query contract (GV2-023)" below); join queries, provenance reads, and context
  projections, all off the hot path. GCTX/MCP queries are explicitly projections
  over this trusted substrate, never a second schema.

## The consumer query contract (GV2-023)

The two API tiers above answer _where_ a read runs (hot vs. background). This
contract answers _what kind of read each consumer is allowed to ask for_, so
INTD, DRVR, GCTX, and WEAVE depend on one shared boundary and do not grow
incompatible graph adapters. It is the consumer-facing face of the GV2-020
registry; GV2-020 implements the registry and hot-read traits, this contract
fixes how downstream subsystems consume them.

The contract is **read-shape, not storage**: every consumer depends on traits
over joined state (the GV2-020 registry handle and the GV2-022 hot-read API),
never on `petgraph` internals, snapshot layout, or a consumer-local schema. On
any contract-vs-implementation conflict GV2 wins (see the GCTX seam below).

### Four read classes

A consumer query falls into exactly one of four classes, ordered by how far it
sits from the enforcement hot path:

| Read class             | Tier / surface                      | Admissibility | `warm`/`stale` marker | Returns                                                                                                            |
| ---------------------- | ----------------------------------- | ------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Enforcement**        | hot-read API (GV2-022)              | hot           | yes                   | allowlist ops only — per-file lookup, edge existence, bounded reverse impact, architectural-index check            |
| **Diagnostic**         | registry join queries (GV2-020)     | non-hot       | n/a (background)      | structural join reads across semantic/dependency/trust/control state                                               |
| **Provenance**         | registry → ref-only Edda resolution | non-hot       | n/a (background)      | anchors (APS id, commit SHA, `SymbolIdentity`, `MemoryId`) resolved into the TS-authoritative Edda store (GV2-014) |
| **Context projection** | `GctxProjector` over the substrate  | non-hot       | n/a (background)      | identity-only projections by default; source-text egress only behind `gctx.egress` (PV-9 CE-1)                     |

The classes share one substrate. A context projection is a _projection over_
diagnostic and provenance reads, never a parallel schema — the same trusted
graph state, re-shaped and redacted for an untrusted reader. Enforcement reads
keep the explicit `warm`/`stale` marker and the degrade-never-escalate rule from
the hot-read split; the three non-hot classes run on the background pool and may
perform joins, but never re-enter the hot path.

### One mapped scenario per consumer

The contract is satisfied when each downstream owner has at least one query
scenario mapped onto a class above:

| Consumer            | Class              | Mapped query scenario                                                                                                                                                                                                                                                                          |
| ------------------- | ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **INTD** (daemon)   | Enforcement        | save-time `certify(delta)` over resident warm indexes returns a `Certifiability` verdict on the hot path (GV2-022, ADR-061 wire)                                                                                                                                                               |
| **DRVR** (drivers)  | Enforcement        | mid-edit "is this new edge admissible?" — known-edge existence + bounded reverse impact through the _same_ allowlist (ADR-063 binds DRVR to one admission rule; no second policy). _Forward-looking: the driver call site lands with the GV2-020 registry; not wired today._                   |
| **GCTX** (MCP)      | Context projection | an assistant asks for a file's impact set; served from the **background** registry (not the hot-path bounded-reverse-impact op, which language it shares) as an identity-only projection through the single `GctxProjector` choke point (PV-9 CE-5), snippets only if `gctx.egress` is enabled |
| **WEAVE** (harness) | Diagnostic         | the agent harness's `GraphQueryTool` asks a structural question — "what imports this symbol", "what are the transitive callers", "what layer is this" — as a registry join read. _Also in-class for WEAVE: Provenance (APS id → ref-only GV2-014 anchors) for task grounding._                 |

These four scenarios discharge the GV2-023 validation (one mapped scenario per
GCTX, DRVR, INTD, and WEAVE owner). The consumers sit at different maturity
levels, and the contract is deliberately mapped **ahead of** the consumers that
do not exist yet — that is the point of a consumer contract (the Intent: stop
each consumer growing an incompatible graph adapter):

- **INTD — live.** The enforcement read path ships today: `validate_paths`
  certify reads the resident warm index on the save-time hot path.
- **DRVR — wired by design, not yet built.** ADR-063 binds driver mid-edit reads
  to the _same_ hot-read allowlist (no second admission policy), and the DRVR
  module is Complete, but its GV2 graph-read call site was explicitly deferred —
  it lands with the GV2-020 registry, not today. The "live" claim here is the
  admission _rule_, not a shipped driver→GV2-022 call.
- **GCTX — in flight.** The `graph-context-delivery` module is Ready (0/13);
  GCTX-001 builds the projection rules onto this contract.
- **WEAVE — planned.** `weave`/`anvil-weave` is a Draft module (0/21, no crate
  yet) — a greenfield import of the standalone `eddacraft/weave-rs` runtime.
  Mapping its `GraphQueryTool` onto the diagnostic (and provenance) tier now is
  exactly what keeps it from being built as a bespoke `petgraph` adapter later.

INTD and DRVR share the enforcement class by design — ADR-063's single admission
rule is the point. GCTX (context projection) and WEAVE (diagnostic/provenance)
both read the non-hot background tier; the privacy boundary that governs GCTX's
egress is specified in the
[context-egress privacy review (PV-9)](../../plans/reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
and absorbed by GCTX-001 (conditions CE-1..CE-12), not re-specified here.

> Scope: this contract fixes the consumer read boundary only. The registry and
> trait _implementation_ is GV2-020; the assistant projection _rules_
> (redaction, pagination, warming) are GCTX-001, now specified in
> [`graph-context-delivery-spec.md`](./graph-context-delivery-spec.md) (the
> identity-only default, sealed egress DTO + single `GctxProjector` choke point,
> egress allowlist/residual table, and CE-1..CE-12 fold). `GctxProjector` is
> named here as the GCTX-001 implementation target required by PV-9 CE-5, not a
> type this section freezes. This section deliberately does not freeze either —
> see "What this spec deliberately does not freeze".

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
- **graph ↔ control/session** — **types shipped; join contract defined
  (GV2-013).** The session model is shipped in INTD (`SessionRecord` /
  `SessionId` / `Attribution::Owned { session }`,
  [`intercept-as-built.md`](./intercept-as-built.md) §10), and GV2-013 cites
  those types rather than reinventing them. The **worktree→file join key
  bridge** that C-5/G-05 flagged as undesigned is now specified as a shared
  `WorkspaceRoot` relativiser in `anvil-kernel-types` (see "The control/session
  graph contract" above), so neither the graph layer nor the control layer
  depends on the other (ADR-064 preserved) and `WorktreeKey` stays internal to
  `anvil-intercept` for fence keying. The remaining work is landing that type
  with the registry build (GV2-020), not the contract.
- **graph ↔ MCP/context (GCTX)** — **pinned.** GCTX consumes projections, must
  not define GV2 schemas, and GV2 wins on conflict; mutually acknowledged in
  both module specs. GCTX-002 (which server hosts it) is a sequencing decision,
  not a seam gap.
- **graph ↔ trust/policy** — **pinned.** Trust levels on nodes are shipped
  (`TrustLevel`: `Unknown`/`Internal`/`Boundary`/`External`/`Privileged`); the
  richer trust/policy _graph_ contract (side-effect surfaces, data classes,
  invariant guards, policy evidence, override sources) is now defined as a
  separate `SymbolIdentity`-keyed graph (GV2-012, see "The trust/policy graph
  contract" above). The remaining work is wiring `annotate_trust` onto the
  daemon certify path (GV2-029), not the contract.
- **graph ↔ provenance/edda-stack** — **join contract defined (GV2-014); Rust
  read surface still proposed.** Provenance is authoritatively specified in
  **TypeScript** (Kindling → Ember → Edda, [`edda-stack.md`](./edda-stack.md));
  the Rust-side counterpart (`eddacraft-kindling`) is still proposed. GV2-014
  resolves the language boundary by making the join **ref-only** — anchors (APS
  id, commit SHA, `SymbolIdentity`, `MemoryId`) cross it, provenance bodies stay
  in the TS-authoritative Edda store (and, once the proposed EDDA-SEAL lands, in
  its git-committed `anvil.edda-provenance.v1` bundle), and resolution is a
  projection-tier read (see "The plan/provenance graph contract" above). The
  durable Edda store is git-committed and shareable — **outside** the same-uid
  boundary the daemon snapshot relies on (see the per-graph privacy scope
  below). What remains open is the proposed Rust read surface
  (`eddacraft-kindling`) — an _implementation_ of the resolution step, not a
  contract gap.

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

### G-02: Plan/provenance join contract — contract defined

**Contract defined by GV2-014.** The join no longer depends on a Rust provenance
surface existing: GV2-014 makes the plan/provenance join **ref-only** (anchors
cross the language boundary, provenance bodies stay in the TS-authoritative Edda
store — and, once the proposed EDDA-SEAL lands, its git-committed
`anvil.edda-provenance.v1` bundle — and resolution is a projection-tier read),
reusing the shipped `ProvenanceChain` contract rather than designing a second.
The contract is followable today against the **shipped** TS contracts
(executable fixture:
`packages/edda-stack/src/contracts/provenance-join.test.ts`). **Residual:** the
Rust read surface (`eddacraft-kindling`) and the EDDA-SEAL sealed bundle are
both still proposed — _implementations_ of the resolution step, not contract
gaps. **Risk:** Low (contract risk retired; the Rust read surface is a later
build).

### G-03: Trust/policy graph contract is documented — partially closed

**Closed for the contract by GV2-012.** The trust/policy graph is now an
authoritative, `SymbolIdentity`-keyed contract documented above ("The
trust/policy graph contract") and implemented in
`crates/anvil-kernel-types/src/trust.rs` +
`crates/anvil-graph-cache/src/trust.rs` (`PolicyProfile`, `TrustGraph`,
`posture_delta`). **Residual:** `annotate_trust` is still not wired on the
daemon certify path (privilege-containment risk). **Risk:** Low (contract risk
retired; wiring is mechanical). **Fix:** the daemon trust-wiring item (GV2-029).

### G-04: Top-level architecture docs predate Graph v2

`rust-architecture-endstate.md` still shows the daemon as deferred and never
mentions Graph v2; `anvil-full-architecture.md` has no joined-graph section; and
`intercept-as-built.md` (reviewed 2026-05-07) predates ADR-067. A reader of
`docs/architecture/` alone would not learn this model exists or that the INTD
symbol-feed seam is pinned. **Risk:** Low. **Fix:** a one-paragraph pointer in
`anvil-full-architecture.md`, a freshness pass on
`rust-architecture-endstate.md`, and an `intercept-as-built.md` refresh covering
ADR-067.

### G-05: The control/session → file join bridge — contract defined

**Contract defined by GV2-013 (named, not frozen).** The worktree→file bridge is
now a defined join key: the shared `WorkspaceRoot` relativiser in
`anvil-kernel-types` turns `SessionRecord.worktree` (absolute, canonicalised
`PathBuf`) plus a changed file's absolute path into the same
workspace-root-relative key the semantic/dependency graphs use, returning `None`
(→ `Attribution::Unknown`) rather than panicking on a path outside the root.
`WorktreeKey` stays internal to `anvil-intercept` (`rule_cache.rs`) for fence
keying; the graph layer reaches neither it nor `anvil-intercept`, so the ADR-064
boundary is not inverted (both layers depend only on `anvil-kernel-types`). See
"The control/session graph contract" above. **Residual:** the type is
named-not-frozen here; it lands in code with the registry build (GV2-020).
**Risk:** Low (design risk retired; the landing is mechanical).

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
  [`plans/modules/graph-context-delivery.aps.md`](../../plans/modules/graph-context-delivery.aps.md),
  [`graph-context-delivery-spec.md`](./graph-context-delivery-spec.md) (GCTX-001
  projection contract — the assistant-facing egress rules over this substrate)
