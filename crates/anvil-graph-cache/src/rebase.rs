//! GBASE-005 (ADR-105): disjoint id allocation and compose-time cross-boundary
//! re-resolution so a shared base and worktree overlay merge without id clash.
//!
//! Id spaces stay disjoint; cross edges re-resolve at compose. Known parity
//! limits vs cold scan live in the GBASE parity fixtures, not here.

use std::collections::BTreeSet;

use anvil_kernel_types::{EdgeType, FileSymbols};

use crate::dependency::DependencyGraph;
use crate::overlay::OverlayFragment;
use crate::symbol_graph::SymbolGraph;

/// Deterministically rebases an overlay fragment's raw parser ids into the
/// reserved range `[watermark, ..)`, keeping them disjoint from the base's
/// `[0, watermark)` (ADR-105 §3).
///
/// The watermark is the base's `base_next_id` — one past the largest id the base
/// ever used — so overlay ids continue exactly where the base left off and can
/// never collide with a base id (nor with each other). Determinism: files are
/// visited in **sorted path order** and, within a file, symbols in **parse
/// order** (the order [`FileSymbols::symbols`] preserves), so the same fragment
/// against the same watermark always yields the same ids.
#[derive(Debug, Clone)]
pub struct OverlayIdAllocator {
    watermark: u64,
    next: u64,
}

impl OverlayIdAllocator {
    /// A fresh allocator over `[watermark, ..)`.
    ///
    /// `watermark` MUST be the base's `base_next_id` — recover it from a
    /// freshly-loaded base with [`SymbolGraph::next_id`](crate::SymbolGraph::next_id)
    /// (the persisted `next_id` high-water mark, restored on load). Do not derive
    /// it from `max(base id) + 1`: after a base session inserted then removed high
    /// ids, the persisted mark sits *above* any surviving node, and using a
    /// re-derived max would re-issue an id the base already spent (the exact bug
    /// [`SymbolGraph::set_next_id_floor`](crate::SymbolGraph::set_next_id_floor)
    /// exists to prevent).
    #[must_use]
    pub fn new(watermark: u64) -> Self {
        Self {
            watermark,
            next: watermark,
        }
    }

    /// The reserved-range floor: overlay ids are all `>= watermark`, base ids all
    /// `< watermark`.
    #[must_use]
    pub fn watermark(&self) -> u64 {
        self.watermark
    }

    /// Rebase `upserts`: return a copy of each [`FileSymbols`] whose every
    /// `SymbolNode.id` is reassigned to a fresh id in `[watermark, ..)`, visiting
    /// files in sorted path order and symbols in parse order.
    ///
    /// Only `SymbolNode.id` is rewritten. `imports` / `reexports` / `calls` are
    /// carried through **unchanged** — they reference their targets by specifier /
    /// export-name, which composition re-resolves at lift time, so they hold no
    /// id to rebase. An empty input yields an empty output (identity).
    ///
    /// The allocator advances across the whole batch, so calling `rebase` once
    /// over all upserts assigns a single contiguous, collision-free id block.
    #[must_use]
    pub fn rebase(&mut self, upserts: &[FileSymbols]) -> Vec<FileSymbols> {
        // Sort by file for a deterministic, path-stable id assignment regardless
        // of the fragment's incoming order.
        let mut ordered: Vec<&FileSymbols> = upserts.iter().collect();
        ordered.sort_by(|a, b| a.file.cmp(&b.file));

        ordered
            .into_iter()
            .map(|fs| {
                let mut rebased = fs.clone();
                for symbol in &mut rebased.symbols {
                    symbol.id = self.next;
                    self.next += 1;
                }
                rebased
            })
            .collect()
    }
}

/// A base symbol edge invalidated by a tombstone (ADR-105 §3): a **surviving**
/// base file's edge whose target lives in a **tombstoned** file. The tombstone
/// removes the target node, so this edge — expressed in the base's now-stale ids
/// — must never be trusted; the import subset is re-established via
/// [`BaseReresolve`].
///
/// Composition drops these implicitly (removing the tombstoned nodes removes
/// their incident edges); the set is emitted so GBASE-006 can *validate* the
/// invalidation rather than infer it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InvalidatedEdge {
    /// The surviving base symbol the edge originates from (a base id `< watermark`).
    pub from_id: u64,
    /// The tombstoned base symbol the edge pointed at (a base id `< watermark`,
    /// removed by the tombstone).
    pub to_id: u64,
    /// The kind of edge invalidated.
    pub edge_type: EdgeType,
}

/// A cross-boundary import edge to **re-establish** after upserts (ADR-105 §3).
///
/// A surviving base file `from_file` imported `to_file`, which the overlay
/// tombstoned and re-added (a modified file). The edge is expressed at **file
/// granularity** — deliberately *not* as ids — so composition re-resolves it
/// against the live composed graph: `from_file`'s anchor to `to_file`'s **new**
/// overlay symbol. Resolving from files, not stored ids, is the "never trusted by
/// stale id" guarantee made concrete.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BaseReresolve {
    /// The surviving base file whose import must be re-bound (its anchor is the
    /// edge source).
    pub from_file: String,
    /// The tombstoned-and-re-added (modified) overlay file the import targets.
    pub to_file: String,
    /// The edge kind to re-establish (always [`EdgeType::Imports`] under the
    /// ADR-105 §3 import contract; typed so the reexport/call extension has a
    /// place to land).
    pub edge_type: EdgeType,
}

/// The typed output of GBASE-005 that GBASE-006 composes (ADR-105 §3).
///
/// It carries everything composition needs to build a graph identical to a cold
/// scan of the combined state, with base and overlay ids kept disjoint:
///
/// 1. [`rebased_upserts`](Self::rebased_upserts) — the overlay [`FileSymbols`]
///    with ids in `[watermark, ..)`, ready to apply through
///    [`update_file`](crate::update_file).
/// 2. [`tombstones`](Self::tombstones) — base files to remove first (deletions +
///    the base shadow of every modified file), passed through from the fragment.
/// 3. [`invalidated_base_edges`](Self::invalidated_base_edges) — the base edges
///    the tombstones invalidate (stale ids that must not be trusted).
/// 4. [`base_reresolve`](Self::base_reresolve) — the surviving-base → overlay
///    import edges to re-establish from the persisted forward map.
///
/// Composition itself (mutating a materialised graph) is **out of scope** — this
/// is the plan, not its application.
#[derive(Debug, Clone)]
pub struct ComposePlan {
    /// The reserved-range floor: base ids `< watermark`, overlay ids `>= watermark`.
    pub watermark: u64,
    /// Overlay upserts with `SymbolNode.id`s rebased into `[watermark, ..)`,
    /// sorted by file. Apply through [`update_file`](crate::update_file).
    pub rebased_upserts: Vec<FileSymbols>,
    /// Base files to remove before applying the upserts (deletions + the base
    /// shadow of every modified file). Sorted, from the fragment.
    pub tombstones: Vec<String>,
    /// Base edges invalidated by the tombstones (surviving-file → tombstoned-file),
    /// in the base's stale ids. Sorted; for GBASE-006 validation.
    pub invalidated_base_edges: Vec<InvalidatedEdge>,
    /// Surviving-base → re-added-overlay import edges to re-establish, resolved
    /// from the persisted file-dependency forward map. Sorted, de-duplicated.
    pub base_reresolve: Vec<BaseReresolve>,
}

impl ComposePlan {
    /// `true` when the fragment changed nothing — no upserts, tombstones, or
    /// cross-boundary work (a clean worktree identical to its base).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rebased_upserts.is_empty()
            && self.tombstones.is_empty()
            && self.invalidated_base_edges.is_empty()
            && self.base_reresolve.is_empty()
    }
}

/// Build the [`ComposePlan`] for applying `fragment` onto the loaded base
/// (`base_sym` / `base_dep`) — GBASE-005's core (ADR-105 §3).
///
/// `base_sym` / `base_dep` are the base **as freshly loaded** (via
/// [`SnapshotPayload::into_graphs`](crate::SnapshotPayload), before any
/// mutation); the watermark is read from `base_sym.next_id()`. `fragment` is the
/// GBASE-004 [`OverlayFragment`] (its upserts carry raw parser ids). The result
/// is fully **deterministic**: the same base and fragment always yield an
/// identical plan (all collections sorted).
#[must_use]
pub fn plan_compose(
    base_sym: &SymbolGraph,
    base_dep: &DependencyGraph,
    fragment: &OverlayFragment,
) -> ComposePlan {
    // The watermark IS the base's recovered next_id high-water mark (ADR-105 §3);
    // see `OverlayIdAllocator::new`.
    let watermark = base_sym.next_id();
    let mut allocator = OverlayIdAllocator::new(watermark);
    let rebased_upserts = allocator.rebase(&fragment.upserts);

    let tombstones = fragment.tombstones.clone();
    let tomb_set: BTreeSet<&str> = tombstones.iter().map(String::as_str).collect();

    // A modified file is BOTH tombstoned (its base shadow) AND re-added (an
    // upsert). A deleted file is tombstoned only. The base→overlay re-resolution
    // targets are exactly the re-added set (tombstoned ∩ upsert files); edges into
    // a deleted file correctly stay dropped (a cold scan of the combined state has
    // no such file, so no such edge).
    let readded: BTreeSet<&str> = fragment
        .upserts
        .iter()
        .map(|fs| fs.file.as_str())
        .filter(|f| tomb_set.contains(f))
        .collect();

    let invalidated_base_edges = invalidated_base_edges(base_sym, &tomb_set);
    let base_reresolve = base_reresolve(base_dep, &tomb_set, &readded);

    ComposePlan {
        watermark,
        rebased_upserts,
        tombstones,
        invalidated_base_edges,
        base_reresolve,
    }
}

/// The base edges the tombstones invalidate: a **surviving** base symbol's edge
/// into a **tombstoned** file (ADR-105 §3). Sorted, deterministic.
fn invalidated_base_edges(
    base_sym: &SymbolGraph,
    tomb_set: &BTreeSet<&str>,
) -> Vec<InvalidatedEdge> {
    // Work proportional to the TOMBSTONED subgraph, not the whole base: an
    // overlay is typically a handful of files against a large base, so a full
    // |V|+|E| walk per compose would dominate compose time (and compose sits on
    // the warm-start path, which carries the GBASE-001 latency budget). For
    // each tombstoned file, visit only its own symbols' incoming edges and
    // classify the source's file.
    let mut edges: Vec<InvalidatedEdge> = tomb_set
        .iter()
        .flat_map(|file| base_sym.symbols_in_file(file))
        .flat_map(|node| base_sym.incoming_edges(node.id))
        .filter_map(|e| {
            let from_file = base_sym.get_symbol(e.from)?.file.as_str();
            // A surviving source whose target is tombstoned — the edge that
            // vanishes and whose stored id becomes stale.
            (!tomb_set.contains(from_file)).then_some(InvalidatedEdge {
                from_id: e.from,
                to_id: e.to,
                edge_type: e.edge_type,
            })
        })
        .collect();
    edges.sort_unstable();
    edges.dedup();
    edges
}

/// The surviving-base → re-added-overlay import edges to re-establish, derived
/// from the persisted Imports-only file-dependency forward map (ADR-105 §3).
///
/// For each re-added (modified) file `M`, its base **dependents** (files that
/// imported it) that are **not themselves tombstoned** need their import of `M`
/// re-bound to `M`'s new overlay symbol. A dependent that *is* tombstoned is
/// itself re-parsed as an upsert, so `update_file` re-lifts its import of `M`
/// naturally — including it here would double the edge, so it is excluded.
fn base_reresolve(
    base_dep: &DependencyGraph,
    tomb_set: &BTreeSet<&str>,
    readded: &BTreeSet<&str>,
) -> Vec<BaseReresolve> {
    let mut out: Vec<BaseReresolve> = Vec::new();
    for &to_file in readded {
        for from_file in base_dep.dependents_of(to_file) {
            if tomb_set.contains(from_file) {
                // A modified/deleted importer re-lifts (or drops) its own edge.
                continue;
            }
            out.push(BaseReresolve {
                from_file: from_file.to_owned(),
                to_file: to_file.to_owned(),
                edge_type: EdgeType::Imports,
            });
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use anvil_kernel_types::{
        EdgeType, ImportEdge, SymbolIdentity, SymbolKind, SymbolNode, TrustLevel, Visibility,
    };

    use crate::incremental::{re_resolve_imports, update_file};
    use crate::snapshot::SnapshotPayload;

    // ---- fixture builders -------------------------------------------------

    fn sym(id: u64, name: &str, file: &str) -> SymbolNode {
        SymbolNode {
            id,
            kind: SymbolKind::Function,
            name: name.to_owned(),
            visibility: Visibility::Public,
            file: file.to_owned(),
            trust_level: TrustLevel::Internal,
            span: None,
        }
    }

    fn import(from_file: &str, to_source: &str) -> ImportEdge {
        ImportEdge {
            from_file: from_file.to_owned(),
            to_source: to_source.to_owned(),
            line: 0,
        }
    }

    /// A `FileSymbols` with the given raw parser ids and import specifiers.
    fn file_symbols(file: &str, symbols: &[(u64, &str)], imports: &[ImportEdge]) -> FileSymbols {
        FileSymbols {
            file: file.to_owned(),
            symbols: symbols
                .iter()
                .map(|(id, name)| sym(*id, name, file))
                .collect(),
            imports: imports.to_vec(),
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
        }
    }

    /// Derive the Imports-only, cross-file dependency graph from a symbol graph —
    /// the same rule `graph_base_producer::derive_dependency_graph` uses, so the
    /// base's persisted forward map matches production.
    fn derive_dep(sym: &SymbolGraph) -> DependencyGraph {
        let mut dep = DependencyGraph::new();
        for node in sym.inner().node_weights() {
            for edge in sym.outgoing_edges(node.id) {
                if edge.edge_type != EdgeType::Imports {
                    continue;
                }
                if let (Some(from), Some(to)) = (sym.get_symbol(edge.from), sym.get_symbol(edge.to))
                    && from.file != to.file
                {
                    dep.add_dependency(from.file.clone(), to.file.clone());
                }
            }
        }
        dep
    }

    /// Edge set expressed by **stable identity** (file, kind, name, ordinal) so
    /// two graphs with different raw ids compare structurally — the id-independent
    /// parity relation.
    fn edge_identities(sym: &SymbolGraph) -> BTreeSet<(SymbolIdentity, SymbolIdentity, EdgeType)> {
        // id → identity, built per file so ordinals match the lift-time scheme.
        let mut identity_of: BTreeMap<u64, SymbolIdentity> = BTreeMap::new();
        let files: BTreeSet<&str> = sym.file_names().collect();
        for file in files {
            let symbols = sym.symbols_in_file(file);
            let identities = SymbolIdentity::for_file_symbols(&symbols);
            for (node, identity) in symbols.iter().zip(identities) {
                identity_of.insert(node.id, identity);
            }
        }
        sym.inner()
            .edge_weights()
            .filter_map(|e| {
                Some((
                    identity_of.get(&e.from)?.clone(),
                    identity_of.get(&e.to)?.clone(),
                    e.edge_type,
                ))
            })
            .collect()
    }

    /// Build the shared base (two files with an import between them:
    /// `f1.ts` imports `f2.ts`), then round-trip it through the persisted
    /// payload — so the returned base is recovered exactly as GBASE-006 would load
    /// it, proving `base_next_id` survives serialisation.
    fn build_base() -> (SymbolGraph, DependencyGraph) {
        let mut sym = SymbolGraph::new();
        // f2 first so f1's import resolves eagerly.
        update_file(&mut sym, file_symbols("f2.ts", &[(1, "bFn")], &[]));
        update_file(
            &mut sym,
            file_symbols("f1.ts", &[(2, "aFn")], &[import("f1.ts", "./f2")]),
        );
        let dep = derive_dep(&sym);

        // Round-trip through the persisted base artefact (ANVILGB1) — the watermark
        // must come back intact.
        let payload = SnapshotPayload::from_graphs(&sym, &dep).expect("base payload builds");
        let bytes = payload.to_base_bytes();
        let decoded = SnapshotPayload::from_base_bytes(&bytes).expect("base decodes");
        decoded.into_graphs().expect("base replays")
    }

    /// The overlay fragment for the fixture: modify `f2.ts` (tombstone its base
    /// shadow + re-add new content) and add `f3.ts` importing **both** `f1.ts`
    /// (base-only) and `f2.ts` (modified). Raw ids deliberately collide with the
    /// base range to prove rebasing moves them.
    fn build_fragment() -> OverlayFragment {
        OverlayFragment {
            upserts: vec![
                // f2.ts modified — raw id 1 collides with the base's bFn id.
                file_symbols("f2.ts", &[(1, "bFn2")], &[]),
                // f3.ts added — raw id 2 collides with the base's aFn id.
                file_symbols(
                    "f3.ts",
                    &[(2, "cFn")],
                    &[import("f3.ts", "./f1"), import("f3.ts", "./f2")],
                ),
            ],
            tombstones: vec!["f2.ts".to_owned()],
            changed: crate::overlay::ChangedSet::default(),
            coverage: crate::overlay::OverlayCoverage {
                walked_files: 3,
                total_files: 3,
                skipped_unreadable: 0,
            },
        }
    }

    /// Apply a [`ComposePlan`] onto the loaded base and return the composed graph.
    ///
    /// This is a **reference application** of the plan using existing public graph
    /// machinery — a preview of GBASE-006, kept in the test harness so GBASE-005's
    /// shipped surface stays the plan, not the composition. It is the sequence
    /// GBASE-006 performs: remove tombstones, apply the rebased upserts, re-resolve
    /// forward references, then re-establish the base→overlay import edges by
    /// resolving each directive against the **live** composed graph (never a stale
    /// base id).
    fn apply_plan(base_sym: SymbolGraph, plan: &ComposePlan) -> SymbolGraph {
        let mut sym = base_sym;
        for file in &plan.tombstones {
            sym.remove_file(file);
        }
        for upsert in &plan.rebased_upserts {
            update_file(&mut sym, upsert.clone());
        }
        // Forward references across the applied upserts.
        let overlay_imports: Vec<ImportEdge> = plan
            .rebased_upserts
            .iter()
            .flat_map(|fs| fs.imports.iter().cloned())
            .collect();
        re_resolve_imports(&mut sym, &overlay_imports);

        // base → overlay: re-bind each surviving base file's import to the target
        // file's CURRENT (new overlay) first symbol — resolved live.
        for directive in &plan.base_reresolve {
            let from = sym
                .symbols_in_file(&directive.from_file)
                .first()
                .map(|s| s.id);
            let to = sym
                .symbols_in_file(&directive.to_file)
                .first()
                .map(|s| s.id);
            if let (Some(from), Some(to)) = (from, to) {
                sym.add_edge(anvil_kernel_types::SymbolEdge {
                    from,
                    to,
                    edge_type: directive.edge_type,
                })
                .expect("re-resolved endpoints exist by construction");
            }
        }
        sym
    }

    /// A cold scan of the combined on-disk state — the parity ground truth. Builds
    /// a graph from scratch over every file in the combined state, with fresh ids,
    /// using the same machinery.
    fn cold_scan_combined() -> SymbolGraph {
        let mut sym = SymbolGraph::new();
        update_file(&mut sym, file_symbols("f2.ts", &[(10, "bFn2")], &[]));
        update_file(
            &mut sym,
            file_symbols("f1.ts", &[(11, "aFn")], &[import("f1.ts", "./f2")]),
        );
        update_file(
            &mut sym,
            file_symbols(
                "f3.ts",
                &[(12, "cFn")],
                &[import("f3.ts", "./f1"), import("f3.ts", "./f2")],
            ),
        );
        // Catch any forward reference regardless of insertion order.
        let imports = [
            import("f1.ts", "./f2"),
            import("f3.ts", "./f1"),
            import("f3.ts", "./f2"),
        ];
        re_resolve_imports(&mut sym, &imports);
        sym
    }

    // ---- (a) watermark disjointness --------------------------------------

    #[test]
    fn watermark_is_recovered_base_next_id() {
        let (base_sym, _) = build_base();
        // Base used ids 1 and 2 ⇒ next_id (the watermark) is 3, intact through the
        // persisted round-trip.
        assert_eq!(
            base_sym.next_id(),
            3,
            "base_next_id must survive persistence"
        );
    }

    #[test]
    fn rebased_overlay_ids_are_disjoint_from_base() {
        let (base_sym, base_dep) = build_base();
        let watermark = base_sym.next_id();
        let plan = plan_compose(&base_sym, &base_dep, &build_fragment());

        assert_eq!(plan.watermark, watermark);
        // Every base id is below the watermark.
        for node in base_sym.inner().node_weights() {
            assert!(
                node.id < watermark,
                "base id {} must be < watermark",
                node.id
            );
        }
        // Every rebased overlay id is at or above the watermark, and no two collide.
        let mut seen = BTreeSet::new();
        for fs in &plan.rebased_upserts {
            for s in &fs.symbols {
                assert!(
                    s.id >= watermark,
                    "overlay id {} must be >= watermark",
                    s.id
                );
                assert!(seen.insert(s.id), "overlay id {} allocated twice", s.id);
            }
        }
    }

    /// Property-style: over many fragments whose raw ids deliberately collide with
    /// the base range, rebasing always lands ids `>= watermark` and disjoint.
    #[test]
    fn rebasing_is_disjoint_over_arbitrary_fragments() {
        // Deterministic LCG — reproducible, no `rand`/`proptest` dependency.
        struct Lcg(u64);
        impl Lcg {
            fn next(&mut self) -> u64 {
                self.0 = self
                    .0
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                self.0
            }
        }
        let mut rng = Lcg(0x0BAD_F00D_D15E_A5ED);

        for _ in 0..500 {
            let watermark = rng.next() % 64; // small so overlap with raw ids is common
            let file_count = (rng.next() % 5) as usize;
            let mut upserts = Vec::new();
            for f in 0..file_count {
                let sym_count = (rng.next() % 5) as usize;
                let symbols: Vec<(u64, &str)> = (0..sym_count)
                    .map(|_| (rng.next() % 128, "s")) // raw ids overlap [0, watermark)
                    .collect();
                upserts.push(file_symbols(&format!("f{f}.ts"), &symbols, &[]));
            }

            let rebased = OverlayIdAllocator::new(watermark).rebase(&upserts);
            let mut seen = BTreeSet::new();
            for fs in &rebased {
                for s in &fs.symbols {
                    assert!(s.id >= watermark, "id {} below watermark {watermark}", s.id);
                    assert!(seen.insert(s.id), "duplicate rebased id {}", s.id);
                }
            }
        }
    }

    // ---- (b) overlay → base ----------------------------------------------

    #[test]
    fn overlay_import_resolves_to_base_symbol() {
        let (base_sym, base_dep) = build_base();
        let watermark = base_sym.next_id();
        let plan = plan_compose(&base_sym, &base_dep, &build_fragment());
        let composed = apply_plan(base_sym, &plan);

        // f3 (overlay) imports f1 (base-only). The edge must land on f1's BASE id.
        let f3_anchor = composed.symbols_in_file("f3.ts")[0].id;
        let f1_anchor = composed.symbols_in_file("f1.ts")[0].id;
        assert!(f1_anchor < watermark, "f1 keeps its base id");
        assert!(f3_anchor >= watermark, "f3 is an overlay symbol");
        let has_edge = composed
            .outgoing_edges(f3_anchor)
            .iter()
            .any(|e| e.to == f1_anchor && e.edge_type == EdgeType::Imports);
        assert!(
            has_edge,
            "overlay f3 → base f1 import must resolve to the base id"
        );
    }

    // ---- (c) base → overlay (never a stale id) ---------------------------

    #[test]
    fn base_import_reresolves_to_new_overlay_id_never_stale() {
        let (base_sym, base_dep) = build_base();
        let watermark = base_sym.next_id();
        // The base's original f2 (bFn) id — the STALE id that must not be trusted.
        let stale_f2_id = base_sym.symbols_in_file("f2.ts")[0].id;
        assert!(stale_f2_id < watermark);

        let plan = plan_compose(&base_sym, &base_dep, &build_fragment());
        // The plan must carry exactly the base→overlay directive.
        assert_eq!(
            plan.base_reresolve,
            vec![BaseReresolve {
                from_file: "f1.ts".to_owned(),
                to_file: "f2.ts".to_owned(),
                edge_type: EdgeType::Imports,
            }],
        );

        let composed = apply_plan(base_sym, &plan);
        let f1_anchor = composed.symbols_in_file("f1.ts")[0].id;
        let new_f2_id = composed.symbols_in_file("f2.ts")[0].id;
        assert!(
            new_f2_id >= watermark,
            "the re-added f2 is an overlay symbol"
        );
        assert!(
            composed.get_symbol(stale_f2_id).is_none(),
            "the stale base f2 symbol must be gone after the tombstone"
        );

        // f1 (surviving base) → f2 must point at the NEW overlay id, never the stale one.
        let edge_to = composed
            .outgoing_edges(f1_anchor)
            .iter()
            .find(|e| e.edge_type == EdgeType::Imports)
            .map(|e| e.to);
        assert_eq!(
            edge_to,
            Some(new_f2_id),
            "must re-resolve to the new overlay id"
        );
        assert_ne!(
            edge_to,
            Some(stale_f2_id),
            "must never trust the stale base id"
        );
    }

    // ---- (d) mini-parity: composed == cold scan --------------------------

    #[test]
    fn composed_edge_set_matches_cold_scan_of_combined_state() {
        // GBASE-006: point the COMPOSED side at the production `compose` function
        // (not the local `apply_plan` reference), while keeping the independent
        // cold-scan builder as ground truth — so this parity anchor now pins the
        // shipped composition path against a from-scratch scan, not a test helper
        // against another test helper (strengthening non-circularity).
        let (base_sym, base_dep) = build_base();
        let payload = SnapshotPayload::from_graphs(&base_sym, &base_dep).expect("base payload");
        let (composed, _) =
            crate::compose::compose(payload, &build_fragment()).expect("production compose");
        let cold = cold_scan_combined();

        let composed_edges = edge_identities(&composed);
        let cold_edges = edge_identities(&cold);
        assert_eq!(
            composed_edges, cold_edges,
            "GBASE-005 mini-parity: base+overlay must equal a cold scan of the combined state\n\
             composed = {composed_edges:#?}\ncold = {cold_edges:#?}"
        );
        // Guard the fixture is actually exercising cross-edges (not a vacuous pass).
        assert_eq!(cold_edges.len(), 3, "fixture must have f1→f2, f3→f1, f3→f2");
    }

    // ---- (e) determinism --------------------------------------------------

    #[test]
    fn plan_is_deterministic() {
        let (base_sym, base_dep) = build_base();
        let a = plan_compose(&base_sym, &base_dep, &build_fragment());
        let b = plan_compose(&base_sym, &base_dep, &build_fragment());

        assert_eq!(a.watermark, b.watermark);
        assert_eq!(a.tombstones, b.tombstones);
        assert_eq!(a.base_reresolve, b.base_reresolve);
        assert_eq!(a.invalidated_base_edges, b.invalidated_base_edges);
        // Rebased ids identical file-by-file.
        let ids = |plan: &ComposePlan| -> Vec<(String, Vec<u64>)> {
            plan.rebased_upserts
                .iter()
                .map(|fs| (fs.file.clone(), fs.symbols.iter().map(|s| s.id).collect()))
                .collect()
        };
        assert_eq!(ids(&a), ids(&b));
    }

    // ---- (f) empty fragment ⇒ identity -----------------------------------

    #[test]
    fn empty_fragment_yields_empty_plan() {
        let (base_sym, base_dep) = build_base();
        let empty = OverlayFragment {
            upserts: Vec::new(),
            tombstones: Vec::new(),
            changed: crate::overlay::ChangedSet::default(),
            coverage: crate::overlay::OverlayCoverage {
                walked_files: 0,
                total_files: 0,
                skipped_unreadable: 0,
            },
        };
        let plan = plan_compose(&base_sym, &base_dep, &empty);
        assert!(
            plan.is_empty(),
            "an empty fragment must yield an empty plan"
        );
        assert_eq!(plan.watermark, base_sym.next_id());
        assert!(plan.rebased_upserts.is_empty());

        // Applying an empty plan leaves the base structurally unchanged.
        let before = edge_identities(&base_sym);
        let composed = apply_plan(build_base().0, &plan);
        assert_eq!(before, edge_identities(&composed));
    }

    // ---- invalidated-base-edge set ---------------------------------------

    #[test]
    fn invalidated_base_edges_names_the_stale_edge() {
        let (base_sym, base_dep) = build_base();
        let f1_anchor = base_sym.symbols_in_file("f1.ts")[0].id;
        let f2_anchor = base_sym.symbols_in_file("f2.ts")[0].id;
        let plan = plan_compose(&base_sym, &base_dep, &build_fragment());

        // f1 → f2 (Imports) is invalidated: f2.ts is tombstoned, f1.ts survives.
        assert_eq!(
            plan.invalidated_base_edges,
            vec![InvalidatedEdge {
                from_id: f1_anchor,
                to_id: f2_anchor,
                edge_type: EdgeType::Imports,
            }],
        );
    }
}
