//! GBASE-006 (ADR-105 §1/§3): **compose** a shared base graph with a worktree
//! overlay into ONE materialised resident graph pair per worktree.
//!
//! GBASE-004 produced the [`OverlayFragment`] (the changed-file diff) and
//! GBASE-005 produced the [`ComposePlan`](crate::rebase::ComposePlan) (the
//! disjoint-id/parity plan). This module performs the plan against a freshly
//! **replayed** base, mutating one materialised `petgraph` per worktree so the
//! result is *identical to a cold scan of the combined on-disk state* — the
//! GBASE-007 correctness anchor. The base is shared **on disk only** (ADR-105 §1):
//! every worktree calls [`compose`] with the same loaded [`SnapshotPayload`] and
//! gets its **own** owned `(SymbolGraph, DependencyGraph)` — nothing is aliased.
//!
//! # The composition sequence (ADR-105 §3)
//!
//! 1. **Replay the base** ([`SnapshotPayload::into_graphs`]) — `petgraph`
//!    re-derives its own `NodeIndex`es and the persisted `next_id` high-water mark
//!    is restored as the overlay **watermark**. No on-disk `NodeIndex` is trusted.
//! 2. **Plan** ([`plan_compose`]) — rebase the overlay's ids into `[watermark, ..)`
//!    (disjoint from the base's `[0, watermark)`) and derive the cross-boundary
//!    edge work.
//! 3. **Apply tombstones** — remove the base shadow of every deleted file **and**
//!    every modified file. Removing a node drops its incident edges, so the base's
//!    now-stale edges into those files vanish here — exactly the
//!    [`invalidated_base_edges`](crate::rebase::ComposePlan::invalidated_base_edges)
//!    set retired. Their removal is *asserted* in debug builds (a `debug_assert`
//!    inside `compose` checks no tombstoned target survives), so the plan field is
//!    read on the production path, not merely computed. This is the file-removal
//!    primitive the composition relies on: [`SymbolGraph::remove_file`] on the
//!    symbol side, [`DependencyGraph::remove_file`] on the dependency side.
//! 4. **Apply upserts** ([`update_file`]) — lift each re-added/added file through
//!    the same path the base producer and daemon use. Ids are already disjoint;
//!    each file's imports resolve against the **composed** file set (base ∪
//!    overlay), so an overlay→base import binds to the base symbol with no plan
//!    entry (it falls out of the upsert).
//! 5. **Re-resolve forward references** — retry the overlay's own
//!    imports/re-exports/calls (a file processed before its target). The base is
//!    already fully resolved, so only the overlay's edges need retrying; this
//!    mirrors the cold path's `re_resolve_*` passes, scoped to the overlay.
//! 6. **Re-bind surviving-base → re-added-overlay imports**
//!    ([`base_reresolve`](crate::rebase::ComposePlan::base_reresolve)) — a
//!    surviving base file imported a file the overlay tombstoned-and-re-added; the
//!    base's persisted edge pointed at the file's **old** id, which the tombstone
//!    removed. Re-bind BOTH endpoints against the **live** composed graph — the
//!    surviving file's anchor and the re-added file's **new** overlay symbol —
//!    never a persisted (stale) id.
//! 7. **Compose the dependency graph** — base dep edges **minus** the tombstoned
//!    files, **plus** the recomputed forward edges of every file the overlay
//!    touched, read from the composed symbol graph (Imports-only, in lockstep with
//!    the cold oracle).
//!
//! # Determinism
//!
//! Every step is deterministic: the payload is sorted, [`plan_compose`] sorts all
//! collections, upserts are applied in sorted-file order, and the re-resolve and
//! re-bind passes iterate sorted inputs. The same base and fragment always compose
//! to the same graph pair.
//!
//! # Trust line (ADR-105 §4, inherited from ADR-069 verbatim)
//!
//! Composing changes only *where the restored indexes come from on disk*, never
//! what a restored index is worth. A composed graph comes up **stale** exactly as
//! a per-worktree snapshot did: the daemon-side installer marks the entry a
//! restored stand-in, and the content-hash reconcile re-establishes `clean` before
//! any `Certified` verdict. This module produces the indexes; it never asserts a
//! verdict.
//!
//! # Scope of the import contract (a documented persisted-format boundary)
//!
//! Composition re-resolves **imports** — exactly what ADR-105 §3 binds and what the
//! persisted Imports-only dependency forward map supports (see the GBASE-005 note
//! in [`crate::rebase`]). Two edge classes are **not** reconstructable from the
//! persisted format and are a known follow-up (schema-additive, deferred):
//! - **base→overlay re-exports and calls** — the base stores *resolved* edges, not
//!   the surviving file's raw `ReexportEdge`/`CallSite`, so a surviving base file
//!   that re-exports from, or calls into, a modified overlay file loses that edge.
//! - **base→added forward references** — a committed base file whose (dangling at
//!   commit time) import is first satisfied by a worktree-**added** file. The base
//!   never held that edge and does not persist the surviving file's raw specifier,
//!   so composition cannot re-establish it. A cold scan would.
//!
//! Both are the same limitation class the GBASE-005 note names; the GBASE-007
//! parity fixture must scope its cross-edge assertions accordingly.

use std::collections::BTreeSet;

use anvil_kernel_types::{CallSite, EdgeType, ImportEdge, ReexportEdge, SymbolEdge};

use crate::dependency::DependencyGraph;
use crate::incremental::{re_resolve_calls, re_resolve_imports, re_resolve_reexports, update_file};
use crate::overlay::OverlayFragment;
use crate::rebase::plan_compose;
use crate::snapshot::{SnapshotLoadError, SnapshotPayload};
use crate::symbol_graph::SymbolGraph;

/// Compose a loaded shared base with a worktree overlay into one materialised
/// resident graph pair (ADR-105 §1/§3 — GBASE-006).
///
/// `base` is a base payload **as loaded** (via `base_store::load_base`); it is
/// replayed here (the caller does not pre-replay). `fragment` is the GBASE-004
/// [`OverlayFragment`] for the worktree. The returned `(SymbolGraph,
/// DependencyGraph)` is **owned** — a sibling worktree composing from the same
/// base gets an independent pair.
///
/// The result is deterministic and, for the ADR-105 §3 import contract, identical
/// to a cold scan of the combined on-disk state (the GBASE-007 anchor). A clean
/// worktree (empty fragment) composes to the base unchanged.
///
/// # Errors
/// [`SnapshotLoadError::Corrupt`] if:
/// - the base payload replays inconsistently (a duplicate id or a dangling edge
///   endpoint — from [`SnapshotPayload::into_graphs`]); or
/// - the fragment breaks the **modified-implies-tombstoned** invariant (a
///   base-preexisting file upserted without a matching tombstone — a malformed
///   overlay that would silently drop surviving importers' edges).
///
/// The daemon-side caller treats either as "serve cold", never a panic. The
/// remaining steps operate on in-memory graphs and do not themselves return
/// errors; the disjoint-id watermark makes an `update_file` id collision (which
/// would drop-and-log a symbol) impossible by construction, so it is *not* a
/// silent failure mode here.
pub fn compose(
    base: SnapshotPayload,
    fragment: &OverlayFragment,
) -> Result<(SymbolGraph, DependencyGraph), SnapshotLoadError> {
    // 1. Replay the base by insert (ADR-069 §1) — petgraph re-derives its own
    //    NodeIndexes; the persisted next_id high-water mark is restored as the
    //    overlay watermark (recovered inside `plan_compose` via `next_id()`).
    let (mut sym, mut dep) = base.into_graphs()?;

    // 2. Derive the disjoint-id compose plan (GBASE-005). The plan supersedes the
    //    reference `apply_plan` helper that lived in the `rebase` test module.
    let plan = plan_compose(&sym, &dep, fragment);

    // Fast path: a clean worktree identical to its base composes to the base.
    if plan.is_empty() {
        return Ok((sym, dep));
    }

    // GBASE-006 council MAJOR 2: enforce the **modified-implies-tombstoned**
    // invariant before mutating. A well-formed GBASE-004 overlay tombstones the
    // base shadow of every base-preexisting file it re-parses. If a base file were
    // upserted WITHOUT a tombstone, `update_file`'s internal `remove_file` would
    // strip the surviving importers' `importer → file` edges while `plan_compose`
    // — keying re-resolution on `tombstoned ∩ upsert` — would emit no
    // `base_reresolve` directive to restore them, silently dropping those edges.
    // Refuse such a fragment as corrupt rather than miscompose. (`sym` is still the
    // pristine replayed base here, so its `file_names` is the base file set.)
    //
    // NOTE: `file_names()` yields the underlying `HashMap`'s order, which is *not*
    // deterministic — it is consumed here only for **set membership** (collected
    // into a `BTreeSet`), never for ordered iteration, so compose stays
    // deterministic regardless of that order.
    let base_files: BTreeSet<&str> = sym.file_names().collect();
    let tomb_set: BTreeSet<&str> = plan.tombstones.iter().map(String::as_str).collect();
    let untombstoned_base_upserts = plan
        .rebased_upserts
        .iter()
        .filter(|fs| base_files.contains(fs.file.as_str()) && !tomb_set.contains(fs.file.as_str()))
        .count();
    if untombstoned_base_upserts > 0 {
        // Return the error path (never a silent miscomposition), and make it
        // observable — count only, no paths (PV-10 telemetry posture). Deliberately
        // NOT a `debug_assert!`: this is a *rejection contract* the daemon-side
        // caller serves cold on and a test exercises, not an unreachable
        // invariant, so it must return rather than panic in debug builds.
        tracing::error!(
            target: "anvil_graph_cache::compose",
            violations = untombstoned_base_upserts,
            "compose refused: base-preexisting upsert(s) not tombstoned \
             (modified-implies-tombstoned invariant broken)",
        );
        return Err(SnapshotLoadError::Corrupt);
    }

    // 3. Symbol graph: remove the base shadow of every tombstoned file (deletions
    //    + the base shadow of every modified file). Removing a symbol drops its
    //    incident edges, so the base's now-stale edges into these files vanish
    //    with them — the `plan.invalidated_base_edges` set retired.
    for file in &plan.tombstones {
        sym.remove_file(file);
    }

    // GBASE-006 council MINOR f: the tombstones above must have removed every node
    // the plan flagged as an invalidated base edge's (tombstoned) target, so no
    // surviving edge can still point at a tombstoned base id. Assert it in debug
    // builds — this is the **explicit** validation of `plan.invalidated_base_edges`
    // (retired implicitly by the removal above, verified here), so the field is
    // read on the production path, not merely computed.
    #[cfg(debug_assertions)]
    for edge in &plan.invalidated_base_edges {
        debug_assert!(
            sym.get_symbol(edge.to_id).is_none(),
            "invalidated base edge target {} survived the tombstone",
            edge.to_id,
        );
    }

    // 4. Apply the rebased upserts through the same `update_file` lift the base
    //    producer and daemon use. Each file's imports/re-exports/calls resolve
    //    against the COMPOSED file set (base ∪ overlay), so an overlay→base import
    //    binds to the base symbol's id directly (no plan entry needed).
    for upsert in &plan.rebased_upserts {
        update_file(&mut sym, upsert.clone());
    }

    // 5. Retry forward references among the OVERLAY's own edges (a file applied
    //    before its target). The base is already fully resolved, so only the
    //    overlay's edges need re-resolution — the cold path runs the same passes
    //    over the whole feed.
    let overlay_imports: Vec<ImportEdge> = plan
        .rebased_upserts
        .iter()
        .flat_map(|fs| fs.imports.iter().cloned())
        .collect();
    let overlay_reexports: Vec<ReexportEdge> = plan
        .rebased_upserts
        .iter()
        .flat_map(|fs| fs.reexports.iter().cloned())
        .collect();
    let overlay_calls: Vec<(String, CallSite)> = plan
        .rebased_upserts
        .iter()
        .flat_map(|fs| fs.calls.iter().map(|c| (fs.file.clone(), c.clone())))
        .collect();
    re_resolve_imports(&mut sym, &overlay_imports);
    re_resolve_reexports(&mut sym, &overlay_reexports);
    re_resolve_calls(&mut sym, &overlay_calls);

    // 6. Re-establish each surviving-base → re-added-overlay import edge from the
    //    plan (ADR-105 §3, "never trusted by a stale id"): resolve BOTH endpoints
    //    against the LIVE composed graph — the surviving base file's anchor and
    //    the re-added file's NEW overlay symbol — never a persisted id. Imports
    //    only (the reexport/call boundary is documented above).
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
            // Both endpoints resolve from live symbols, so `add_edge` (which only
            // errors on a missing endpoint) cannot fail here. GBASE-006 council
            // MINOR c: make an invariant break observable rather than silent — a
            // stray failure is non-fatal (the edge is simply absent, as an
            // unresolved cold import would be), but it should never happen.
            if let Err(err) = sym.add_edge(SymbolEdge {
                from,
                to,
                edge_type: directive.edge_type,
            }) {
                debug_assert!(
                    false,
                    "base-reresolve add_edge failed with resolved endpoints: {err}",
                );
                tracing::warn!(
                    target: "anvil_graph_cache::compose",
                    error = %err,
                    "base-reresolve edge omitted: add_edge failed despite resolved endpoints",
                );
            }
        }
    }

    // 7. Dependency graph (Imports-only, in lockstep with the cold oracle): base
    //    dep edges MINUS the tombstoned files, PLUS the recomputed forward edges of
    //    every file the overlay touched, read from the COMPOSED symbol graph.
    //
    //    `remove_file` clears both directions, so a deleted file vanishes as a
    //    source AND a target (a surviving importer of a deleted file correctly
    //    loses that edge). For a modified file it also clears the surviving
    //    importers' `importer → file` edges; those are restored below by refreshing
    //    each `base_reresolve` source from the composed symbol graph.
    for file in &plan.tombstones {
        dep.remove_file(file);
    }
    for upsert in &plan.rebased_upserts {
        refresh_file_dependencies(&mut dep, &sym, &upsert.file);
    }
    for directive in &plan.base_reresolve {
        refresh_file_dependencies(&mut dep, &sym, &directive.from_file);
    }

    Ok((sym, dep))
}

/// Refresh `file`'s outgoing dependency edges in `dep` from the composed symbol
/// graph, replacing them with its resolved cross-file **import** targets.
///
/// The graph-cache-local twin of the daemon's `kernel_cache::refresh_file_dependencies`
/// (GV2-011) and the cold `derive_dependency_graph` oracle — kept **Imports-only**
/// in lockstep with both (GV2-031): a `Reexports` edge widens the privilege
/// surface the certify diff reads off the symbol graph, not the module-dependency
/// graph, and a self-edge is skipped (matching `set_dependencies`). Composing the
/// dependency graph by refreshing exactly the touched neighbourhood (never a whole
/// re-derive) keeps warm-start off the O(all-edges) path (the ADR-105 §11 latency
/// budget).
fn refresh_file_dependencies(dep: &mut DependencyGraph, sym: &SymbolGraph, file: &str) {
    let mut targets: Vec<String> = Vec::new();
    for symbol in sym.symbols_in_file(file) {
        for edge in sym.outgoing_edges(symbol.id) {
            if edge.edge_type != EdgeType::Imports {
                continue;
            }
            if let Some(to) = sym.get_symbol(edge.to)
                && to.file != file
            {
                targets.push(to.file.clone());
            }
        }
    }
    dep.set_dependencies(file, targets);
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use anvil_kernel_types::{
        FileSymbols, ImportEdge, SymbolIdentity, SymbolKind, SymbolNode, TrustLevel, Visibility,
    };

    use crate::incremental::update_file;
    use crate::overlay::{ChangedSet, OverlayCoverage, OverlayFragment};

    // ---- fixture builders (independent of rebase.rs) ----------------------

    fn sym_node(id: u64, name: &str, file: &str) -> SymbolNode {
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

    fn file_symbols(file: &str, symbols: &[(u64, &str)], imports: &[ImportEdge]) -> FileSymbols {
        FileSymbols {
            file: file.to_owned(),
            symbols: symbols
                .iter()
                .map(|(id, name)| sym_node(*id, name, file))
                .collect(),
            imports: imports.to_vec(),
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
        }
    }

    /// Derive the Imports-only cross-file dependency graph from a symbol graph —
    /// the same rule the cold oracle uses, so a composed dep graph can be compared
    /// against a cold-scan dep graph.
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

    /// Id-independent edge set keyed by stable identity, so two graphs with
    /// different raw ids compare structurally (the parity relation).
    fn edge_identities(sym: &SymbolGraph) -> BTreeSet<(SymbolIdentity, SymbolIdentity, EdgeType)> {
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

    /// The base file set (workspace-relative) → sorted dependency targets, an
    /// id-free view of a dependency graph for structural comparison.
    fn dep_view(dep: &DependencyGraph, files: &[&str]) -> BTreeMap<String, Vec<String>> {
        let mut out = BTreeMap::new();
        for f in files {
            let mut targets: Vec<String> = dep
                .dependencies_of(f)
                .iter()
                .map(|s| (*s).to_owned())
                .collect();
            targets.sort();
            if !targets.is_empty() {
                out.insert((*f).to_owned(), targets);
            }
        }
        out
    }

    /// The shared base: `f1.ts` imports `f2.ts`, round-tripped through the
    /// persisted `ANVILGB1` artefact so the returned payload is recovered exactly
    /// as GBASE-006 loads it (proving the watermark survives serialisation).
    fn base_payload() -> SnapshotPayload {
        let mut sym = SymbolGraph::new();
        update_file(&mut sym, file_symbols("f2.ts", &[(1, "bFn")], &[]));
        update_file(
            &mut sym,
            file_symbols("f1.ts", &[(2, "aFn")], &[import("f1.ts", "./f2")]),
        );
        let dep = derive_dep(&sym);
        let payload = SnapshotPayload::from_graphs(&sym, &dep).expect("base payload builds");
        // Round-trip through the base artefact bytes.
        SnapshotPayload::from_base_bytes(&payload.to_base_bytes()).expect("base decodes")
    }

    /// Modify `f2.ts` (tombstone base shadow + re-add) and add `f3.ts` importing
    /// **both** `f1.ts` (base-only) and `f2.ts` (modified). Raw ids collide with
    /// the base range to prove rebasing moves them.
    fn fragment() -> OverlayFragment {
        OverlayFragment {
            upserts: vec![
                file_symbols("f2.ts", &[(1, "bFn2")], &[]),
                file_symbols(
                    "f3.ts",
                    &[(2, "cFn")],
                    &[import("f3.ts", "./f1"), import("f3.ts", "./f2")],
                ),
            ],
            tombstones: vec!["f2.ts".to_owned()],
            changed: ChangedSet {
                added: vec!["f3.ts".to_owned()],
                modified: vec!["f2.ts".to_owned()],
                deleted: Vec::new(),
            },
            coverage: OverlayCoverage {
                walked_files: 3,
                total_files: 3,
                skipped_unreadable: 0,
            },
        }
    }

    /// A cold scan of the combined on-disk state — the parity ground truth, built
    /// from scratch over every file with fresh ids.
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
        let imports = [
            import("f1.ts", "./f2"),
            import("f3.ts", "./f1"),
            import("f3.ts", "./f2"),
        ];
        crate::incremental::re_resolve_imports(&mut sym, &imports);
        sym
    }

    // ---- (a) parity: composed == cold scan of the combined state ----------

    #[test]
    fn composed_graph_matches_cold_scan_of_combined_state() {
        let (sym, dep) = compose(base_payload(), &fragment()).expect("compose");
        let cold = cold_scan_combined();

        // Symbol-graph edge set (id-independent) equals the cold scan.
        let composed_edges = edge_identities(&sym);
        let cold_edges = edge_identities(&cold);
        assert_eq!(
            composed_edges, cold_edges,
            "GBASE-006 parity: base+overlay must equal a cold scan of the combined state\n\
             composed = {composed_edges:#?}\ncold = {cold_edges:#?}"
        );
        assert_eq!(
            cold_edges.len(),
            3,
            "fixture must exercise f1→f2, f3→f1, f3→f2"
        );

        // Dependency graph equals a fresh derive of the cold scan.
        let files = ["f1.ts", "f2.ts", "f3.ts"];
        assert_eq!(
            dep_view(&dep, &files),
            dep_view(&derive_dep(&cold), &files),
            "composed dependency graph must equal the cold scan's"
        );
    }

    // ---- (b) tombstoned base file's symbols/edges absent post-compose -----

    #[test]
    fn tombstoned_base_symbols_and_stale_edges_are_absent() {
        // The base's original f2 symbol (bFn) is the STALE id the tombstone removes.
        let base = base_payload();
        let (base_sym, _) = base.clone().into_graphs().expect("replay for probe");
        let stale_f2_id = base_sym.symbols_in_file("f2.ts")[0].id;
        let stale_f2_name = base_sym.symbols_in_file("f2.ts")[0].name.clone();
        assert_eq!(stale_f2_name, "bFn");

        let (sym, _) = compose(base, &fragment()).expect("compose");

        // The stale base symbol id is gone.
        assert!(
            sym.get_symbol(stale_f2_id).is_none(),
            "the tombstoned base f2 symbol must be absent post-compose"
        );
        // No surviving symbol still carries the old name in f2 — only the re-added
        // overlay symbol (bFn2) remains.
        let f2_names: Vec<&str> = sym
            .symbols_in_file("f2.ts")
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(f2_names, vec!["bFn2"], "f2 holds only the re-added symbol");
        // No edge anywhere points at the stale id.
        let points_at_stale = sym
            .inner()
            .edge_weights()
            .any(|e| e.from == stale_f2_id || e.to == stale_f2_id);
        assert!(!points_at_stale, "no edge may reference the tombstoned id");
    }

    // ---- deleted file: symbols + incident edges gone ----------------------

    #[test]
    fn deleted_base_file_and_its_incident_edges_are_gone() {
        // Base: f1 imports f2. Delete f2 (tombstone only, no re-add).
        let base = base_payload();
        let deletion = OverlayFragment {
            upserts: Vec::new(),
            tombstones: vec!["f2.ts".to_owned()],
            changed: ChangedSet {
                added: Vec::new(),
                modified: Vec::new(),
                deleted: vec!["f2.ts".to_owned()],
            },
            coverage: OverlayCoverage {
                walked_files: 1,
                total_files: 1,
                skipped_unreadable: 0,
            },
        };
        let (sym, dep) = compose(base, &deletion).expect("compose");

        assert!(
            sym.symbols_in_file("f2.ts").is_empty(),
            "the deleted file's symbols are gone"
        );
        // f1 survives but its import of the now-deleted f2 resolves to nothing.
        assert!(!sym.symbols_in_file("f1.ts").is_empty(), "f1 survives");
        assert!(
            dep.dependencies_of("f1.ts").is_empty(),
            "f1's dependency on the deleted f2 is dropped (both directions)"
        );
        assert!(dep.dependents_of("f2.ts").is_empty());
    }

    // ---- MAJOR 2: modified-implies-tombstoned invariant enforced ----------

    #[test]
    fn base_preexisting_upsert_without_tombstone_is_refused() {
        // A malformed fragment: it re-parses (upserts) f1.ts — a base-preexisting
        // file — but does NOT tombstone it. A well-formed GBASE-004 overlay always
        // tombstones the base shadow of a modified file; without it, `update_file`
        // would strip f1's surviving importers' edges while `plan_compose` emits no
        // `base_reresolve` directive to restore them (a silent miscomposition).
        // compose() must refuse it via the error path, not miscompose.
        let violating = OverlayFragment {
            // f1.ts is in the base; upserted here with NO matching tombstone.
            upserts: vec![file_symbols("f1.ts", &[(1, "aFn2")], &[])],
            tombstones: Vec::new(),
            changed: ChangedSet {
                added: Vec::new(),
                modified: vec!["f1.ts".to_owned()],
                deleted: Vec::new(),
            },
            coverage: OverlayCoverage {
                walked_files: 2,
                total_files: 2,
                skipped_unreadable: 0,
            },
        };
        let result = compose(base_payload(), &violating);
        assert!(
            matches!(result, Err(SnapshotLoadError::Corrupt)),
            "a base-preexisting upsert without a tombstone must be refused as corrupt, got {result:?}"
        );

        // Sanity: the SAME file upserted WITH its tombstone composes fine (this is
        // the well-formed modified-file shape), so the guard is specific to the
        // missing-tombstone violation, not to touching a base file at all.
        let well_formed = OverlayFragment {
            upserts: vec![file_symbols("f1.ts", &[(1, "aFn2")], &[])],
            tombstones: vec!["f1.ts".to_owned()],
            changed: ChangedSet {
                added: Vec::new(),
                modified: vec!["f1.ts".to_owned()],
                deleted: Vec::new(),
            },
            coverage: OverlayCoverage {
                walked_files: 2,
                total_files: 2,
                skipped_unreadable: 0,
            },
        };
        assert!(
            compose(base_payload(), &well_formed).is_ok(),
            "the same file upserted WITH its tombstone composes cleanly"
        );
    }

    // ---- (g) determinism: two runs structurally identical -----------------

    #[test]
    fn compose_is_deterministic_over_two_runs() {
        let (sym_a, dep_a) = compose(base_payload(), &fragment()).expect("compose a");
        let (sym_b, dep_b) = compose(base_payload(), &fragment()).expect("compose b");

        assert_eq!(
            edge_identities(&sym_a),
            edge_identities(&sym_b),
            "symbol-graph edge set is deterministic"
        );
        // Raw ids are path-stable across runs (base replay + watermark allocation
        // are both deterministic), so the id-keyed node sets match too.
        let ids = |s: &SymbolGraph| -> Vec<(u64, String, String)> {
            let mut v: Vec<(u64, String, String)> = s
                .inner()
                .node_weights()
                .map(|n| (n.id, n.name.clone(), n.file.clone()))
                .collect();
            v.sort();
            v
        };
        assert_eq!(ids(&sym_a), ids(&sym_b), "node ids are deterministic");
        // DependencyGraph derives structural (set-based) equality.
        assert_eq!(dep_a, dep_b, "dependency graph is deterministic");
    }

    // ---- empty fragment ⇒ base unchanged ----------------------------------

    #[test]
    fn empty_fragment_composes_to_the_base_unchanged() {
        let base = base_payload();
        let (base_sym, base_dep) = base.clone().into_graphs().expect("replay");
        let empty = OverlayFragment {
            upserts: Vec::new(),
            tombstones: Vec::new(),
            changed: ChangedSet::default(),
            coverage: OverlayCoverage {
                walked_files: 2,
                total_files: 2,
                skipped_unreadable: 0,
            },
        };

        let (sym, dep) = compose(base, &empty).expect("compose");
        assert_eq!(
            edge_identities(&sym),
            edge_identities(&base_sym),
            "a clean worktree composes to the base symbol graph unchanged"
        );
        let files = ["f1.ts", "f2.ts"];
        assert_eq!(
            dep_view(&dep, &files),
            dep_view(&base_dep, &files),
            "a clean worktree composes to the base dependency graph unchanged"
        );
    }

    // ---- sibling independence at the graph level --------------------------

    #[test]
    fn two_composes_from_one_base_are_independent_instances() {
        // Both compose from a base built the same way (the on-disk artefact is
        // shared); the returned pairs are owned, so mutating one cannot touch the
        // other. Full daemon-cache sibling independence is proven intercept-side;
        // this pins the graph-cache invariant that `compose` returns owned graphs.
        let (mut sym_a, _) = compose(base_payload(), &fragment()).expect("compose a");
        let (sym_b, _) = compose(base_payload(), &fragment()).expect("compose b");

        let before_b = edge_identities(&sym_b);
        // Mutate A: drop a file.
        sym_a.remove_file("f3.ts");
        assert!(sym_a.symbols_in_file("f3.ts").is_empty());
        // B is untouched.
        assert_eq!(
            edge_identities(&sym_b),
            before_b,
            "mutating one composed graph must not affect its sibling"
        );
        assert!(
            !sym_b.symbols_in_file("f3.ts").is_empty(),
            "sibling still holds f3"
        );
    }
}
