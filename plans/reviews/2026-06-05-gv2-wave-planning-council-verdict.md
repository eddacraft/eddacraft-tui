# Graph v2 Foundation — Wave Planning Council Verdict

**Date:** 2026-06-05
**Session:** `plan-dc4da9f2` (Planning Council, 6 personas)
**Personas:** architect, kernel-maintainer, pragmatic-lead, adversarial-reviewer,
security-analyst, operations-reviewer.
**Scope:** the proposed implementation wave **GV2-001 → 002 → 003 → 010 → 011 →
020 → 022** (the "hot-read slice" enabling daemon save-time validation sub-phase
A′). Mandate: _"ensure we have it right and haven't undercooked it; expand if
needed."_
**Inputs:** `plans/modules/graph-v2-foundation.aps.md`;
`plans/specs/2026-06-01-daemon-save-time-validation-contract.md`; ADR-061, -063,
-064, -069, -031; `plans/reviews/2026-06-01-daemon-graph-council-verdict.md`;
and current `crates/anvil-graph-cache/` + `crates/anvil-intercept/` code at
`main` (`93bd6534b`).

---

## 1. Verdict

**The wave as written is BOTH over-scoped in paperwork AND undercooked in
substance — and it is grounded against a tree that no longer exists.** All six
personas independently discovered that **ADR-064 (Accepted 2026-06-02) moved the
graph code into a new `crates/anvil-graph-cache/` crate**, and that the core of
Phase 0/1 (the resident reverse index, `certify()`, `export_surface_changed()`,
the `GraphDelta` type) **already shipped under Sub-phase A**. Every `Files:` path
in the module still points at `crates/anvil-kernel/src/graph/…`, which **does not
exist**.

So two things are simultaneously true:

- **Reframe, don't rebuild:** GV2-001/003/010 are largely "ratify what shipped +
  close the named residual gaps," not green-field design.
- **The parts that make A′ _real_ are missing entirely:** the ADR-063 hot-path
  enforcement (type split + Criterion gate + debug assertion), the A′ wire-in,
  the backing-swap parity proof, the production parser feed, and the privilege /
  privacy enforcement guards. None of these exist in code or as work items.

Net: **do not execute the wave as written.** Re-ground it, reframe the shipped
items, and add the enforcement + wiring items below. Per the owner's decisions
(§3) the wave is also being **grown** to the full multi-graph foundation.

---

## 2. The reframing finding (unanimous, CRITICAL)

| Claim | Evidence |
| --- | --- |
| Graph code lives in `crates/anvil-graph-cache/`, not `crates/anvil-kernel/src/graph/`. Every module `Files:` path is stale. | ADR-064 (`064-intercept-graph-cache-crate-boundary.md`); `ls crates/anvil-graph-cache/src/` → `certify.rs dependency.rs incremental.rs lib.rs symbol_graph.rs trust.rs`. GV2-011 cites `crates/anvil-kernel/src/graph/dependency.rs` (absent). |
| The resident reverse index is **built and maintained** — the prior council's B1 ("`reverse` is net-new / unpopulated / zero callers") is **now stale**. | `dependency.rs:8-29` (`reverse: HashMap`, `add_dependency` populates both directions); `kernel_cache.rs` derives + stores a `DependencyGraph` per `WorktreeKey` on every delta. |
| `export_surface_changed()` and all B4 fixtures (body-only, rename, delete, internal→public, re-export) **already exist**. | `certify.rs:122-140`, fixtures `certify.rs:299-379`. |
| `GraphDelta` is fully shipped. | `incremental.rs:11-25`. |

**Consequence:** a re-grounding pass (correct every `Files:` path; flip the
genuinely-shipped acceptance criteria to Done; record the residual gaps as
explicit scoped deferrals) is the **first** step of the wave and unblocks every
reviewer/implementer who would otherwise hunt for non-existent files.

---

## 3. Owner decisions (locked 2026-06-05)

1. **GV2-020 → GROW THE WAVE.** Pull GV2-012 (trust/policy), GV2-013
   (control/session), GV2-014 (plan/provenance) into the wave so GV2-020 is
   buildable as the **full** multi-graph registry, not a descoped skeleton.
2. **Export precision → GRADUATE NOW.** Build GV2-002 stable identity + a real
   export-diff primitive in this wave so precise body-only edits stay
   `certified` instead of conservatively defaulting to `partial`. GV2-002 is
   therefore **in-wave critical**, not a deferred successor.
3. **Privilege certify → CLAIM CONTAINMENT (wire it).** Save-time certify is
   intended to attest privilege containment, so the inert `annotate_trust` path
   (§4, SEC-03) is a **live false-certify** and must be fixed; the backing swap
   is blocked until a `node:fs`-importing privilege-expansion change provably
   does **not** certify clean.
4. **Process → REPORT FIRST.** This document. Module rewrite + index update
   follow on approval.

---

## 4. Prioritised findings (consolidated across personas)

### CRITICAL

- **C1 — Stale grounding / phantom file paths.** (ARCH-02, KERN-07, DEL-01,
  ADV-06, OPS-01) — see §2. _Fix:_ re-ground all paths to
  `crates/anvil-graph-cache/src/`; reframe shipped items.

- **C2 — ADR-063 hot-path enforcement is entirely unimplemented.** ADR-063 says
  admissibility is "enforced, not aspirational" via **(a)** a GV2-022 type split
  so non-admissible ops are "not even callable from the hot-read API" and **(b)**
  the ADR-031 Criterion benchmark that "fails CI on budget regression," plus a
  hot-path debug assertion. **None of the three exist.** `lib.rs:1-21` exposes
  the full graph surface with no hot/background type boundary; there is **no
  `benches/` dir** in `anvil-graph-cache`; `grep debug_assert` finds no
  admission guard. (KERN-03, ADV-01, OPS-01, OPS-02, ARCH-05) _Fix:_ new items
  GV2-024 (type split + assertion) and GV2-025 (bench + CI gate).

- **C3 — No A′ wire-in: the slice would ship as an uncalled library.** Nothing in
  the wave replaces `kernel_cache.rs`'s O(edges) `derive_dependency_graph()`
  re-derive (`kernel_cache.rs:282,323-340`) with the resident incremental index
  behind `validate_paths`, and `fed_symbols` currently yields `None` so **every
  `ContentModify` returns `Partial` regardless of graph quality**
  (`validate_paths.rs:484`). This is the `with_cross_check_context` zero-callers
  failure mode. (ARCH-07, DEL-03, DEL-10, KERN-05, OPS-06) _Fix:_ new items
  GV2-027 (backing swap-in + incremental maintenance + parity) and GV2-028
  (production parser feed).

- **C4 — Privilege-expansion certify is INERT in production (false-certify).**
  `annotate_trust` is **never called** on the daemon certify path
  (`kernel_cache.rs:264-281`; `grep annotate_trust crates/anvil-intercept` →
  none), so `trust_level` is always `Unknown`, `previously_privileged` is always
  empty, and a change that newly imports `node:fs`/`child_process` and exposes a
  privileged operation **certifies clean** if its public name set is unchanged.
  Per owner decision §3.3 this is a live bug. (SEC-03, KERN-06, ADV-04) _Fix:_
  new item GV2-029.

### MAJOR

- **M1 — GV2-002 stable identity (now in-wave critical).** Symbol identity is
  `file::kind::name` (`incremental.rs:28-30`) — position-conflated and
  session-local (`symbol_graph.rs` `next_id`). Two same-`(kind,name)` public
  overloads collapse → adding an overload reads as no surface change
  (`certify.rs:196-199`). Also blocks Sub-phase B snapshot comparability. _Fix
  (decision §3.2):_ expand GV2-002 to deliver (a) content/position-independent,
  cross-restart-stable identity with overload disambiguation; (b) a documented
  rename stance; (c) the export-diff primitive that graduates the conservative
  `partial` default. Add the overload fixture (red today). (ARCH-03, KERN-01,
  ADV-05)

- **M2 — GV2-003 "complete delta contract" is false as written.** `removed_edges`
  is hard-wired empty (`incremental.rs:151`, `:297`; `certify.rs:24` "never
  branch on it"); `changed`-node channel and `schema_version` are absent; a
  modify is modelled as full churn. The item's own replay-equivalence Validation
  is only partially covered (`reverse_index_consistent_after_delta` covers the
  dep graph only). _Fix:_ either populate `removed_edges` + add a changed-node
  channel + `schema_version`, **or** narrow the Expected Outcome and mark those
  channels out-of-scope-for-A′ with a forward note — **pick one and align the
  Validation.** Add a full `(SymbolGraph, DependencyGraph)` replay-equivalence
  property test over arbitrary delta sequences (incl. atomic-save inode flip,
  rename=delete+create, delete/recreate). (ARCH-04, KERN-02, ADV-06)

- **M3 — GV2-011 under-specifies the actual A′ win.** The deliverable is making
  the dep graph **maintained incrementally** in `apply_delta` (retiring
  `derive_dependency_graph`), not "defining" indexes that already exist. Add: a
  cold-rebuild-equivalence property test; an explicit list of which architectural
  checks are precomputed-resident vs background (ADR-063 denylist); bind the
  benchmark to the save-time budget **class** (ADR-031 interactive p95), not a
  generic component budget. (ARCH-05, KERN-05, DEL-03, OPS-06)

- **M4 — `impact_closure` has no hop-depth lever.** ADR-063 mandates a
  runtime-configurable, **hard-capped**, feature-flagged depth (default 1 hop).
  Code has only a file-count budget with **unbounded depth**
  (`certify.rs:149-165`). _Fix:_ new item GV2-026 (depth param + hard ceiling +
  feature-flag surface + 3-hop fixture proving depth=1 stops at the direct
  importer). (ARCH-06, ADV-03, ADV-09)

- **M5 — No backing-swap parity proof.** "Wire-invariant swap" is untested:
  nothing asserts the GV2 hot-index backing yields verdict-identical
  `Certifiability` to the interim cache for the same delta sequence — the swap
  can silently change verdicts. _Fix:_ fold a parity property test into GV2-027
  (the diagnostic-parity gate's equivalent for the backing). (ADV-02)

- **M6 — Privacy gate is unenforced.** ADR-069 §8 promises the privacy line is a
  "compile-time + unit-tested property," but there is **no** `SnapshotPayload`
  DTO, no `postcard` codec, no no-leak test, no `write_snapshot`/`load_snapshot`
  (`grep` → zero hits); the live graph types aren't even `serde`. The module
  attributes the gate to the merged **ADR** (GV2-021), i.e. to a doc, not a
  guard. The "Privacy review completed" checkbox is unchecked while GV2-002/010
  freeze the persistable field shape. _Fix:_ new item GV2-030 (sealed-DTO +
  structural no-leak + relative-path test; gates `ANVIL_PERSIST_GRAPH` default);
  make the privacy-review checkbox a hard blocker for GV2-002 and GV2-010
  specifically. (SEC-01, SEC-02, ADV-10, OPS-04)

- **M7 — GV2-020 validation is one line for a foundational shared trait.** Risk:
  green unit tests + APS-closed ≠ wired. _Fix (with the grown wave):_ add (a) a
  compile-time/sealed-trait test that the hot-read API cannot reach a denylist
  op, and (b) an end-to-end test that drives `validate_paths` **through** the
  registry path with a non-vacuous verdict. (ADV-07, OPS-01)

### MINOR

- **m1 — GV2-010 schema gaps:** no source spans (blocks GCTX-013), no `Reexport`
  edge type (re-export recursion currently rides file-level `dependents_of`),
  `Method` parent edges deferred. Split A′-critical subset (identity, visibility,
  import/dep edges, re-export edge) from the GCTX-projection subset (spans,
  calls, references, language metadata); spans must be a no-text `ByteRange`.
  (ARCH-08, DEL-05, SEC-06)
- **m2 — `CertifyStale::ExportSurfaceChange` collapses to `CrossFileResolutionNeeded`
  on the wire** (`validate_paths.rs:95`) — two distinct stale reasons
  indistinguishable to clients. Consider a dedicated `StaleReason` variant.
  (KERN — code_facts)
- **m3 — Observability:** the ADR-035 Notification envelope for assurance
  transitions is deferred ("Phase E"); `snapshot_load_result`/`write_result`
  counters (ADR-069 §10) are absent. A fleet operator can't distinguish a
  snapshot-load failure from a normal stale transition. (OPS-05)
- **m4 — At-rest threat-model line:** a warm-start snapshot is a durable
  structural map (names, path identity, edge topology, hashes) of a private
  codebase — a qualitatively new surface vs in-memory-only Sub-phase A. Add one
  reviewed threat-model line parallel to the spec §4 C3 live-read decision.
  (SEC-05)
- **m5 — `SAVE_TIME_CERTIFY_BUDGET` is a compile-time constant** (`save_time.rs:69`),
  not the operator-config lever ADR-063 implies; when the depth lever lands
  (M4), fold both into a `CertifyBudget { max_files, max_depth }` with clamped
  ceilings. (ADV-09)
- **m6 — Bench environment:** the hot-read benches must declare a quiet/CI box
  (this dev box is inotify-exhausted; the bench harness is flaky in backgrounded
  agent shells). (OPS-07)

---

## 5. The reshaped wave (grown + re-grounded + expanded)

Ordering reflects the real critical path. **Step 0 is a prerequisite for
everything.**

| # | Item | Disposition | One-line scope |
| --- | --- | --- | --- |
| 0 | **Re-grounding pass** | new (housekeeping) | Correct every `Files:` path to `crates/anvil-graph-cache/src/`; flip shipped criteria to Done; record residual gaps as scoped deferrals. |
| 1 | GV2-001 | reframe → mostly ratify | Taxonomy spec; ratify what ADR-063/064/069 froze; non-blocking for the swap but required for the grown registry (012/013/014). |
| 2 | **GV2-002** | **expand (in-wave critical)** | Stable cross-restart identity + overload disambiguation + export-diff primitive; graduates the `partial` default (decision §3.2). |
| 3 | GV2-003 | expand / decide | `removed_edges` + changed-node + `schema_version`, **or** narrow the claim; full pair replay-equivalence property test. |
| 4 | GV2-010 | reframe + split | A′-critical subset vs GCTX-projection subset; add `Reexport` edge; no-text spans. |
| 5 | GV2-011 | expand | Incremental dep-graph maintenance (retire `derive_dependency_graph`); cold-rebuild-equivalence test; precomputed-vs-background list. |
| 6 | **GV2-012** | **grown in** | Trust/policy graph contract — intersects GV2-029 (privilege wiring). |
| 7 | **GV2-013** | **grown in** | Control/session graph contract (INTD/DRVR alignment). |
| 8 | **GV2-014** | **grown in** | Plan/provenance graph contract. |
| 9 | GV2-020 | now buildable (full) | Multi-graph registry + typed query traits; + sealed-trait negative test + end-to-end-through-registry test (M7). |
| 10 | GV2-022 | expand | Typed hot-read API + warm/stale markers + depth lever wiring; depends on GV2-011 (drop the GV2-020 dep — sequencing fix). |
| 11 | GV2-023 | **recommended add** | Consumer query contract (GCTX/DRVR/INTD/WEAVE) — the capstone that stops a full registry shipping without a consumer boundary; depends on 020+022. |

### New work items (assign GV2-024 … GV2-030)

- **GV2-024 — Hot-read type split + hot-path debug assertion.** A sealed
  `HotReadApi` exposing only the four ADR-063 allowlist ops; denylist ops
  reachable only via a separate `BackgroundReadApi`. Debug assertion trips on any
  parse/resolve/traversal/I/O inside a hot call. _Validation:_ a denylist op does
  not compile when called from the hot type; the assertion fires in a unit test
  that fakes an I/O call. (C2)
- **GV2-025 — Criterion hot-read bench + ADR-031 CI gate.**
  `crates/anvil-graph-cache/benches/hot_read.rs` measuring per-file lookup,
  `dependents_of`, `impact_closure` at depth 1 and at the hard cap, on the
  latency corpus; **fails CI when p95 exceeds the ADR-031 interactive budget.**
  Declares its quiet/CI box requirement. (C2, M3, m6)
- **GV2-026 — `impact_closure` hop-depth lever.** `max_depth` distinct from the
  file-count budget; hard-capped; feature-flag/config surface (depth 1→2 without
  recompile); 3-hop fixture proves depth=1 stops at the direct importer; a
  config above the ceiling is clamped, not honoured. (M4, m5)
- **GV2-027 — A→A′ backing swap-in + parity.** Wire the resident incremental
  index behind `validate_paths`; retire `derive_dependency_graph`; bump
  `backing_schema_version` (`interim-symbolgraph-v1` → `gv2-hotindex-v1`); a
  parity property test asserts verdict-identical `Certifiability` vs the interim
  cache over arbitrary delta sequences. (C3, M5)
- **GV2-028 — Production parser feed.** Wire
  `ForegroundOpts::with_symbol_parser` so `fed_symbols` yields `FileSymbols` for
  TS/JS; until this lands every `ContentModify` is `Partial`. _Validation:_ a
  body-only edit on a parsed file returns `certified`, not `partial`, end-to-end.
  (C3 / DEL-10)
- **GV2-029 — Wire privilege containment (decision §3.3).** Call `annotate_trust`
  on the daemon apply path; extend `previously_privileged`/`current_privileged`
  to treat `Boundary` as elevated together with the `previously_public` diff.
  _Validation:_ a `node:fs`-importing change that adds a privileged surface does
  **not** certify clean. **Blocks the GV2-027 swap.** (C4, SEC-04)
- **GV2-030 — Sealed-DTO snapshot serialisation + structural no-leak guard.**
  `SnapshotPayload` allowlist DTO; `postcard` codec; a test that fails CI if any
  transitive field outside the allowlist (`Vec<u8>`, `serde(flatten)`, any
  source-text `String`) can reach the payload, and that every persisted path is
  workspace-root-relative; gates `ANVIL_PERSIST_GRAPH` default. Sub-phase B
  prerequisite. (M6)

> **Privacy-review gate:** the unchecked "Privacy review completed" Ready-checklist
> item becomes a **hard blocker for GV2-002 and GV2-010** (they freeze the
> persistable field shape), and its output is the input to the GV2-030 allowlist.

---

## 6. Critical path to a shippable, honest A′

```
GV2-0 (re-ground) ─► GV2-002 (stable id + export-diff) ─► GV2-003 (delta decided)
        │                                                        │
        └─► GV2-011 (incremental maintenance) ─► GV2-024 (type split + assert)
                                                      └─► GV2-025 (bench + CI gate)
                                                            └─► GV2-026 (depth lever)
GV2-028 (parser feed) ─┐
GV2-029 (privilege wiring, BLOCKS swap) ─┴─► GV2-027 (backing swap + parity) ─► A′ live
GV2-012/013/014 (grown) ─► GV2-020 (registry) ─► GV2-022 (hot-read API) ─► GV2-023 (consumer contract)
GV2-030 (privacy DTO) ─► gates Sub-phase B persistence default-on
```

The **minimum honest A′** is the top three rows: re-ground, the GV2-002/003/011
foundation with its enforcement (024–026), the production feed (028) and
privilege wiring (029), then the swap with its parity proof (027). The grown
registry rows (012/013/014 → 020 → 022 → 023) and persistence (030) extend the
foundation per the owner's "fully cook it" mandate but are not on the narrowest
swap path.

---

## 7. Open questions still outstanding

1. **GV2-003 direction:** fix the pipeline (`removed_edges` + changed-node +
   `schema_version`) or narrow the contract claim? Both are defensible; the
   provenance joins (GV2-014, now in-wave) lean toward fixing it.
2. **`ExportSurfaceChange` wire variant (m2):** add a dedicated `StaleReason`, or
   accept the collapse for Sub-phase A?
3. **At-rest residual (SEC, m4):** is the cleartext-names/specifiers residual
   accepted for `ANVIL_PERSIST_GRAPH` **default-on** graduation, or must a
   scrub/redaction pass land first?
4. **RLB-008 calibration:** the go/no-go numbers for the O(edges) re-derive on a
   representative corpus — confirm RLB-008 is the gating item before the swap
   ships at scale (OPS-06).

---

## 8. Appendix — per-persona verdicts

| Persona | Verdict | Headline |
| --- | --- | --- |
| architect | undercooked | Foundation largely shipped under Sub-phase A so the wave is stale-grounded/mis-scoped; the two genuine net-new items (011, 020) are under-specified and 020's deps are unsatisfiable as written. |
| kernel-maintainer | sound-with-fixes | Directionally sound but GV2-002 overload gap, no Criterion harness, and GV2-003's permanently-empty `removed_edges` are undercooked against the real baseline. |
| pragmatic-lead | over-scoped | A′ needs three real items; reframe the shipped ones, the registry isn't needed for the narrow swap — but the owner chose to grow it deliberately. |
| adversarial-reviewer | undercooked | No parity test, no bench harness, no depth lever, silent `Boundary` exclusion, and four items too thin to prevent zero-caller shipping. |
| security-analyst | undercooked | Privacy gate unenforced (no sealed DTO/no-leak test), privilege-expansion certify inert in the daemon, `Boundary` gap documented-but-unclosed. |
| operations-reviewer | sound-with-fixes | Structurally sound but ADR-063's "enforced not aspirational" admission rule has zero of its two enforcement mechanisms in code. |
