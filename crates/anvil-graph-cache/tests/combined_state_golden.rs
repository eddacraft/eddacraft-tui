//! GBASE-007: combined-state golden parity (base + overlay vs cold scan).
//!
//! Hermetic pin of composed graph shape, including surviving-base → re-added
//! overlay `Imports` / `Reexports` reconstruction.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anvil_graph_cache::overlay::{ChangedSet, OverlayCoverage};
use anvil_graph_cache::{
    DependencyGraph, OverlayFragment, SnapshotPayload, SymbolGraph, compose, re_resolve_imports,
    re_resolve_reexports, update_file,
};
use anvil_kernel_types::{
    EdgeType, FileSymbols, ImportEdge, ReexportEdge, SymbolIdentity, SymbolKind, SymbolNode,
    TrustLevel, Visibility,
};

// ---------------------------------------------------------------------------
// Fixture builders — hand-scripted base + overlay (id-collisions deliberate).
// ---------------------------------------------------------------------------

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

fn reexport(from_file: &str, exported_name: &str, to_source: &str) -> ReexportEdge {
    ReexportEdge {
        from_file: from_file.to_owned(),
        exported_name: exported_name.to_owned(),
        to_source: to_source.to_owned(),
        line: 0,
    }
}

/// A `FileSymbols` with raw parser ids, import/reexport specifiers, and no
/// content hash (`None` — the hashless default; `.go` files model the
/// tail-language extractor, `.ts` files are kept hashless here too since layer
/// (i) hand-builds the fragment and hashing is a layer-(ii) concern).
fn file_symbols(
    file: &str,
    symbols: &[(u64, &str)],
    imports: &[ImportEdge],
    reexports: &[ReexportEdge],
) -> FileSymbols {
    FileSymbols {
        file: file.to_owned(),
        symbols: symbols
            .iter()
            .map(|(id, name)| sym_node(*id, name, file))
            .collect(),
        imports: imports.to_vec(),
        reexports: reexports.to_vec(),
        calls: Vec::new(),
        calls_partial: false,
        has_unresolved_dynamic_import: false,
        content_hash: None,
    }
}

/// The shared base `base(X)`, round-tripped through the persisted `ANVILGB1`
/// artefact so the returned payload is recovered exactly as GBASE-006 loads it
/// (the watermark survives serialisation). Files (all workspace-root-relative,
/// flat so `./x` specifiers resolve unambiguously):
///
/// - `core.ts` (`core`) — a leaf base symbol imported across the boundary;
/// - `widget.ts` (`widget`) — **modified** by the overlay;
/// - `gone.ts` (`gone`) — **deleted** by the overlay;
/// - `helper.go` (`Help`) — hashless survivor (unchanged);
/// - `svc.go` (`Svc`) — hashless, **modified** by the overlay;
/// - `consumer.ts` (`consumer`) — imports `./widget` (surviving base importer of
///   a modified file → `BaseReresolve`);
/// - `reexporter.ts` (`reexporter`) — re-exports `* from './widget'` (surviving
///   base re-exporter of a modified file → `BaseReresolve` for `Reexports`);
/// - `needsgone.ts` (`needs`) — imports `./gone` (a surviving importer of a
///   deleted file).
fn base_payload() -> SnapshotPayload {
    let mut sym = SymbolGraph::new();
    // Targets first so importers resolve eagerly; re_resolve below is belt-and-braces.
    update_file(&mut sym, file_symbols("core.ts", &[(1, "core")], &[], &[]));
    update_file(
        &mut sym,
        file_symbols("widget.ts", &[(2, "widget")], &[], &[]),
    );
    update_file(&mut sym, file_symbols("gone.ts", &[(3, "gone")], &[], &[]));
    update_file(
        &mut sym,
        file_symbols("helper.go", &[(4, "Help")], &[], &[]),
    );
    update_file(&mut sym, file_symbols("svc.go", &[(5, "Svc")], &[], &[]));
    update_file(
        &mut sym,
        file_symbols(
            "consumer.ts",
            &[(6, "consumer")],
            &[import("consumer.ts", "./widget")],
            &[],
        ),
    );
    update_file(
        &mut sym,
        file_symbols(
            "reexporter.ts",
            &[(7, "reexporter")],
            &[],
            &[reexport("reexporter.ts", "*", "./widget")],
        ),
    );
    update_file(
        &mut sym,
        file_symbols(
            "needsgone.ts",
            &[(8, "needs")],
            &[import("needsgone.ts", "./gone")],
            &[],
        ),
    );

    // Belt-and-braces forward-reference resolution (order-independent).
    re_resolve_imports(
        &mut sym,
        &[
            import("consumer.ts", "./widget"),
            import("needsgone.ts", "./gone"),
        ],
    );
    re_resolve_reexports(&mut sym, &[reexport("reexporter.ts", "*", "./widget")]);

    let dep = derive_dep(&sym);
    let payload = SnapshotPayload::from_graphs(&sym, &dep).expect("base payload builds");
    // Round-trip through the base artefact bytes — recovered exactly as loaded.
    SnapshotPayload::from_base_bytes(&payload.to_base_bytes()).expect("base decodes")
}

/// The overlay fragment: modify `widget.ts` + `svc.go` (tombstone base shadow +
/// re-add), add `feature.ts` + `mid.ts`, delete `gone.ts`. Raw ids collide with
/// the base range to prove rebasing moves them.
fn fragment() -> OverlayFragment {
    OverlayFragment {
        upserts: vec![
            // widget.ts modified — new symbol `widget2` (raw id collides with base).
            file_symbols("widget.ts", &[(2, "widget2")], &[], &[]),
            // svc.go modified (hashless) — same symbol name, re-parsed (raw id collides).
            file_symbols("svc.go", &[(5, "Svc")], &[], &[]),
            // feature.ts added — overlay→base (`./core`), overlay→modified (`./widget`),
            // and the multi-hop hop (`./mid`).
            file_symbols(
                "feature.ts",
                &[(20, "feature")],
                &[
                    import("feature.ts", "./core"),
                    import("feature.ts", "./widget"),
                    import("feature.ts", "./mid"),
                ],
                &[],
            ),
            // mid.ts added — overlay→base (`./core`); completes feature→mid→core.
            file_symbols("mid.ts", &[(21, "mid")], &[import("mid.ts", "./core")], &[]),
        ],
        // Deletions ∪ modifications (base shadow removed before compose).
        tombstones: vec![
            "gone.ts".to_owned(),
            "svc.go".to_owned(),
            "widget.ts".to_owned(),
        ],
        changed: ChangedSet {
            added: vec!["feature.ts".to_owned(), "mid.ts".to_owned()],
            modified: vec!["svc.go".to_owned(), "widget.ts".to_owned()],
            deleted: vec!["gone.ts".to_owned()],
        },
        coverage: OverlayCoverage {
            // Combined on-disk state: core, widget, consumer, reexporter, needsgone,
            // helper.go, svc.go, feature, mid = 9 files (gone.ts deleted).
            walked_files: 9,
            total_files: 9,
            skipped_unreadable: 0,
        },
    }
}

/// A **cold scan** of the combined on-disk state — the parity ground truth, built
/// from scratch over every surviving/added file with fresh ids (`gone.ts` absent).
fn cold_scan_combined() -> SymbolGraph {
    let mut sym = SymbolGraph::new();
    // Fresh ids (100+) to prove the parity relation is id-independent.
    update_file(
        &mut sym,
        file_symbols("core.ts", &[(100, "core")], &[], &[]),
    );
    update_file(
        &mut sym,
        file_symbols("widget.ts", &[(101, "widget2")], &[], &[]),
    );
    update_file(
        &mut sym,
        file_symbols("helper.go", &[(102, "Help")], &[], &[]),
    );
    update_file(&mut sym, file_symbols("svc.go", &[(103, "Svc")], &[], &[]));
    update_file(
        &mut sym,
        file_symbols(
            "consumer.ts",
            &[(104, "consumer")],
            &[import("consumer.ts", "./widget")],
            &[],
        ),
    );
    update_file(
        &mut sym,
        file_symbols(
            "reexporter.ts",
            &[(105, "reexporter")],
            &[],
            &[reexport("reexporter.ts", "*", "./widget")],
        ),
    );
    update_file(
        &mut sym,
        file_symbols(
            "needsgone.ts",
            &[(106, "needs")],
            &[import("needsgone.ts", "./gone")],
            &[],
        ),
    );
    update_file(
        &mut sym,
        file_symbols(
            "feature.ts",
            &[(107, "feature")],
            &[
                import("feature.ts", "./core"),
                import("feature.ts", "./widget"),
                import("feature.ts", "./mid"),
            ],
            &[],
        ),
    );
    update_file(
        &mut sym,
        file_symbols(
            "mid.ts",
            &[(108, "mid")],
            &[import("mid.ts", "./core")],
            &[],
        ),
    );

    // Catch every forward reference regardless of insertion order.
    re_resolve_imports(
        &mut sym,
        &[
            import("consumer.ts", "./widget"),
            import("needsgone.ts", "./gone"),
            import("feature.ts", "./core"),
            import("feature.ts", "./widget"),
            import("feature.ts", "./mid"),
            import("mid.ts", "./core"),
        ],
    );
    re_resolve_reexports(&mut sym, &[reexport("reexporter.ts", "*", "./widget")]);
    sym
}

// ---------------------------------------------------------------------------
// Id-independent parity relations.
// ---------------------------------------------------------------------------

/// The Imports-only cross-file dependency graph derived from a symbol graph —
/// the cold oracle's rule (`derive_dependency_graph`), so a composed dep graph
/// compares against a cold-scan dep graph.
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

/// The full id-independent edge set keyed by stable `SymbolIdentity`, so two
/// graphs with different raw ids compare structurally.
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

/// The `Imports`-only slice of [`edge_identities`] — the ADR-105 §3
/// **reconstructable** cross-edge contract the parity assertion pins byte-equal.
fn import_identities(sym: &SymbolGraph) -> BTreeSet<(SymbolIdentity, SymbolIdentity, EdgeType)> {
    edge_identities(sym)
        .into_iter()
        .filter(|(_, _, ty)| *ty == EdgeType::Imports)
        .collect()
}

/// An id-free view of a dependency graph: file → sorted dependency targets.
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

const COMBINED_FILES: &[&str] = &[
    "core.ts",
    "widget.ts",
    "consumer.ts",
    "reexporter.ts",
    "needsgone.ts",
    "helper.go",
    "svc.go",
    "feature.ts",
    "mid.ts",
];

/// Compose the fixture through the **production** `compose` function.
fn composed() -> (SymbolGraph, DependencyGraph) {
    compose(base_payload(), &fragment()).expect("production compose")
}

// ---------------------------------------------------------------------------
// (1) Parity: composed import set + dependency graph == cold scan.
// ---------------------------------------------------------------------------

#[test]
fn composed_import_edges_and_dependency_graph_equal_the_cold_scan() {
    let (sym, dep) = composed();
    let cold = cold_scan_combined();

    // Import edges (the reconstructable ADR-105 §3 contract) are byte-equal under
    // the id-independent identity relation.
    let composed_imports = import_identities(&sym);
    let cold_imports = import_identities(&cold);
    assert_eq!(
        composed_imports, cold_imports,
        "GBASE-007 parity: composed IMPORT edges must equal a cold scan of the combined state\n\
         composed = {composed_imports:#?}\ncold = {cold_imports:#?}"
    );
    // Guard against a vacuous pass: the fixture must actually exercise the cross
    // edges (consumer→widget, feature→core, feature→widget, feature→mid, mid→core).
    assert_eq!(
        composed_imports.len(),
        5,
        "fixture must exercise all five reconstructable import cross-edges"
    );

    // Dependency graph (Imports-only) equals a fresh derive of the cold scan.
    assert_eq!(
        dep_view(&dep, COMBINED_FILES),
        dep_view(&derive_dep(&cold), COMBINED_FILES),
        "composed dependency graph must equal the cold scan's"
    );
}

// ---------------------------------------------------------------------------
// (2) Surviving-base → re-added-overlay Reexports parity (formerly the
//     imports-only exclusion; closed by reconstructing non-import directives).
// ---------------------------------------------------------------------------

#[test]
fn reexport_into_modified_overlay_matches_cold_scan() {
    let (sym, _) = composed();
    let cold = cold_scan_combined();

    let composed_all = edge_identities(&sym);
    let cold_all = edge_identities(&cold);

    // A surviving base file (`reexporter.ts`) re-exports `* from './widget'`, and
    // `widget.ts` was modified. Composition must re-establish the Reexports edge
    // onto the new overlay symbol so the composed graph matches a cold scan.
    let reexporter = SymbolIdentity {
        file: "reexporter.ts".to_owned(),
        kind: SymbolKind::Function,
        name: "reexporter".to_owned(),
        ordinal: 0,
    };
    let widget = SymbolIdentity {
        file: "widget.ts".to_owned(),
        kind: SymbolKind::Function,
        name: "widget2".to_owned(),
        ordinal: 0,
    };
    let reexport_edge = (reexporter, widget, EdgeType::Reexports);

    assert!(
        composed_all.contains(&reexport_edge),
        "composed graph must restore surviving-base → re-added-overlay Reexports; missing {reexport_edge:#?}"
    );
    assert!(
        cold_all.contains(&reexport_edge),
        "cold scan must include the reexport (fixture sanity); missing {reexport_edge:#?}"
    );

    let only_in_cold: BTreeSet<_> = cold_all.difference(&composed_all).cloned().collect();
    let only_in_composed: BTreeSet<_> = composed_all.difference(&cold_all).cloned().collect();
    assert!(
        only_in_cold.is_empty(),
        "composed must not lag the cold scan; only_in_cold = {only_in_cold:#?}"
    );
    assert!(
        only_in_composed.is_empty(),
        "composition must never invent an edge the cold scan lacks; found {only_in_composed:#?}"
    );
}

// ---------------------------------------------------------------------------
// (3) The committed golden: the composed shape pinned to a tracked fixture file.
// ---------------------------------------------------------------------------

/// Path to the committed golden fixture (tracked in git; **not** generated during
/// a normal test run — see the regeneration note below).
fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("gbase007_combined_state.snap")
}

/// The exact composed shape, pinned byte-for-byte against a **committed**
/// serialised artefact (the sealed `SnapshotPayload` wire bytes of the composed
/// `(SymbolGraph, DependencyGraph)`). Any future composition divergence fails
/// against this committed pin, not a recomputed expectation.
///
/// # Regeneration discipline (snapshot golden pattern)
///
/// The composed wire form is deterministic (ids are path-stable, every collection
/// is sorted), so these bytes only move on a **deliberate** change to the compose
/// output or the snapshot wire format. To regenerate after such a change, run:
///
/// ```text
/// ANVIL_BLESS_GBASE007=1 cargo test -p anvil-graph-cache --test combined_state_golden
/// ```
///
/// then commit the updated `.snap`. On a wire-format change also bump
/// `SNAPSHOT_BACKING_SCHEMA_VERSION` deliberately (see `snapshot.rs`). Do NOT
/// bless to "fix" a red test without understanding why the composed shape moved —
/// an unexpected move is exactly the regression this golden exists to catch.
#[test]
fn composed_shape_matches_committed_golden() {
    let (sym, dep) = composed();
    let got = SnapshotPayload::from_graphs(&sym, &dep)
        .expect("composed payload builds")
        .to_bytes();

    let path = golden_path();
    if std::env::var_os("ANVIL_BLESS_GBASE007").is_some() {
        std::fs::write(&path, &got).expect("write golden fixture");
        eprintln!(
            "blessed GBASE-007 golden: {} ({} bytes)",
            path.display(),
            got.len()
        );
        return;
    }

    let expected = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "committed golden fixture missing at {} ({e}); regenerate with \
             ANVIL_BLESS_GBASE007=1 cargo test -p anvil-graph-cache --test combined_state_golden",
            path.display()
        )
    });
    assert_eq!(
        got, expected,
        "composed shape drifted from the committed GBASE-007 golden — investigate the \
         composition change; only bless (ANVIL_BLESS_GBASE007=1) once the drift is understood \
         and intended"
    );

    // Sanity: the committed golden must itself still decode + round-trip, so a
    // blessed fixture can never pin a structurally-broken artefact.
    let decoded = SnapshotPayload::from_bytes(&expected).expect("golden bytes decode");
    assert_eq!(
        decoded.to_bytes(),
        expected,
        "committed golden bytes must round-trip through from_bytes"
    );
}

// ---------------------------------------------------------------------------
// (4) Determinism: two composes agree, structurally and against the pin.
// ---------------------------------------------------------------------------

#[test]
fn compose_is_deterministic_and_matches_the_pin() {
    let (sym_a, dep_a) = composed();
    let (sym_b, dep_b) = composed();

    // Structural (id-independent) equality across two runs.
    assert_eq!(
        edge_identities(&sym_a),
        edge_identities(&sym_b),
        "the composed edge set is deterministic"
    );
    assert_eq!(
        dep_view(&dep_a, COMBINED_FILES),
        dep_view(&dep_b, COMBINED_FILES),
        "the composed dependency graph is deterministic"
    );

    // Raw ids are path-stable, so the sealed wire bytes are byte-identical across
    // runs — and equal to the committed pin (unless blessing).
    let bytes_a = SnapshotPayload::from_graphs(&sym_a, &dep_a)
        .unwrap()
        .to_bytes();
    let bytes_b = SnapshotPayload::from_graphs(&sym_b, &dep_b)
        .unwrap()
        .to_bytes();
    assert_eq!(
        bytes_a, bytes_b,
        "composed wire bytes are deterministic across runs"
    );

    if std::env::var_os("ANVIL_BLESS_GBASE007").is_none() {
        let expected = std::fs::read(golden_path()).expect("committed golden present");
        assert_eq!(
            bytes_a, expected,
            "both runs equal the committed golden pin"
        );
    }
}
