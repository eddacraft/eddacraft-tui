//! GV2-003 delta + event-contract replay-equivalence tests.
//!
//! The contract's honesty test: applying a sequence of file operations
//! incrementally (each producing a [`GraphDelta`]) must converge to the same
//! observable `(SymbolGraph, DependencyGraph)` state as a full rebuild from
//! the final file set — including the awkward sequences the wave verdict
//! named: atomic-save inode flip (same path, new content), rename
//! (delete + create), and delete/recreate. Plus per-field honesty checks that
//! `removed_edges`, `node_changes`, and `schema_version` report truthfully.

use std::collections::{BTreeMap, BTreeSet};

use anvil_graph_cache::incremental::{
    GRAPH_DELTA_SCHEMA_VERSION, GraphDelta, NodeChange, re_resolve_imports, remove_file,
    update_file,
};
use anvil_graph_cache::symbol_graph::SymbolGraph;
use anvil_kernel_types::{
    EdgeType, FileSymbols, ImportEdge, SymbolKind, SymbolNode, TrustLevel, Visibility,
};

// ---------- fixtures ----------

/// A file's content as a fixture: public function names + import specifiers.
#[derive(Clone)]
struct FileSpec {
    names: Vec<&'static str>,
    imports: Vec<&'static str>,
}

fn spec(names: &[&'static str], imports: &[&'static str]) -> FileSpec {
    FileSpec {
        names: names.to_vec(),
        imports: imports.to_vec(),
    }
}

/// Build `FileSymbols` for `file` from a spec, allocating ids from `base`.
fn file_symbols(file: &str, s: &FileSpec, base: u64) -> FileSymbols {
    FileSymbols {
        file: file.to_string(),
        symbols: s
            .names
            .iter()
            .enumerate()
            .map(|(i, n)| SymbolNode {
                id: base + i as u64,
                kind: SymbolKind::Function,
                name: (*n).to_string(),
                visibility: Visibility::Public,
                file: file.to_string(),
                trust_level: TrustLevel::Unknown,
                span: None,
            })
            .collect(),
        imports: s
            .imports
            .iter()
            .map(|spec| ImportEdge {
                from_file: file.to_string(),
                to_source: (*spec).to_string(),
                line: 0,
            })
            .collect(),
        reexports: Vec::new(),
        calls: Vec::new(),
        calls_partial: false,
        has_unresolved_dynamic_import: false,
    }
}

/// A structural fingerprint of a `SymbolGraph` that ignores session-local ids
/// and node ordering: a **multiset** (count per `(file, kind, name)`) of
/// symbols plus the set of import edges expressed as `from_file -> to_file`.
///
/// The symbol channel is a count map, not a set, on purpose: overloads share
/// `(file, kind, name)` (they differ only by ordinal — GV2-002), so a set
/// would collapse them and hide an overload-count divergence between the
/// incremental and rebuilt graphs. Two graphs with this fingerprint are
/// observably equivalent for the contract (same symbol multiset, same import
/// topology), regardless of how ids were allocated.
#[derive(Debug, PartialEq, Eq)]
struct GraphFingerprint {
    symbols: BTreeMap<(String, String, String), usize>,
    import_edges: BTreeSet<(String, String)>,
}

fn fingerprint(g: &SymbolGraph) -> GraphFingerprint {
    let mut symbols: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for n in g.inner().node_weights() {
        *symbols
            .entry((n.file.clone(), format!("{:?}", n.kind), n.name.clone()))
            .or_insert(0) += 1;
    }
    let mut import_edges = BTreeSet::new();
    for n in g.inner().node_weights() {
        for e in g.outgoing_edges(n.id) {
            if e.edge_type != EdgeType::Imports {
                continue;
            }
            let (Some(from), Some(to)) = (g.get_symbol(e.from), g.get_symbol(e.to)) else {
                continue;
            };
            import_edges.insert((from.file.clone(), to.file.clone()));
        }
    }
    GraphFingerprint {
        symbols,
        import_edges,
    }
}

/// File-level dependency edges derived from a `SymbolGraph` — the same
/// derivation the daemon's `derive_dependency_graph` performs (cross-file
/// import edges only). Used to assert `DependencyGraph` equivalence without
/// depending on `anvil-intercept`.
fn dep_edges(g: &SymbolGraph) -> BTreeSet<(String, String)> {
    let mut deps = BTreeSet::new();
    for n in g.inner().node_weights() {
        for e in g.outgoing_edges(n.id) {
            if e.edge_type != EdgeType::Imports {
                continue;
            }
            let (Some(from), Some(to)) = (g.get_symbol(e.from), g.get_symbol(e.to)) else {
                continue;
            };
            if from.file != to.file {
                deps.insert((from.file.clone(), to.file.clone()));
            }
        }
    }
    deps
}

/// Every import edge across a live file set.
fn imports_of(files: &BTreeMap<String, FileSpec>) -> Vec<ImportEdge> {
    files
        .iter()
        .flat_map(|(path, s)| {
            s.imports.iter().map(move |spec| ImportEdge {
                from_file: path.clone(),
                to_source: (*spec).to_string(),
                line: 0,
            })
        })
        .collect()
}

/// Rebuild a graph from scratch from a file set, mirroring how a cold start
/// would: add every file, then run a re-resolve pass so forward references
/// (importer parsed before importee) resolve — exactly what the incremental
/// path also needs.
fn rebuild(files: &BTreeMap<String, FileSpec>) -> SymbolGraph {
    let mut g = SymbolGraph::new();
    for (path, s) in files {
        let base = g.next_id();
        update_file(&mut g, file_symbols(path, s, base));
    }
    re_resolve_imports(&mut g, &imports_of(files));
    g
}

// ---------- the headline property ----------

/// Drive a sequence of operations incrementally and assert the result is
/// observably equivalent to a full rebuild from the final file set. `ops` are
/// `(path, Some(spec))` for create/modify or `(path, None)` for delete.
fn assert_replay_equivalent(ops: &[(&str, Option<FileSpec>)]) {
    let mut g = SymbolGraph::new();
    let mut live: BTreeMap<String, FileSpec> = BTreeMap::new();

    for (path, maybe_spec) in ops {
        if let Some(s) = maybe_spec {
            let base = g.next_id();
            update_file(&mut g, file_symbols(path, s, base));
            live.insert((*path).to_string(), s.clone());
        } else {
            remove_file(&mut g, path);
            live.remove(*path);
        }
    }
    // A re-resolve pass mirrors the rebuild's: cross-file edges that could not
    // resolve when the importer was applied before the importee get fixed up.
    // It must use the *currently live* import set, not historical imports — a
    // dropped import must stay dropped (the daemon re-resolves against the
    // current scan, never an accumulated log).
    let live_imports = imports_of(&live);
    re_resolve_imports(&mut g, &live_imports);

    let rebuilt = rebuild(&live);

    assert_eq!(
        fingerprint(&g),
        fingerprint(&rebuilt),
        "incremental SymbolGraph diverged from rebuild"
    );
    assert_eq!(
        dep_edges(&g),
        dep_edges(&rebuilt),
        "incremental DependencyGraph diverged from rebuild"
    );
}

#[test]
fn delta_replay_equivalence_atomic_save_inode_flip() {
    // Same path re-saved with fresh ids (atomic save = write temp + rename
    // over): the second update must converge to a one-file rebuild.
    assert_replay_equivalent(&[
        ("src/a.ts", Some(spec(&["foo", "bar"], &[]))),
        ("src/a.ts", Some(spec(&["foo", "baz"], &[]))),
    ]);
}

#[test]
fn delta_replay_equivalence_rename_is_delete_plus_create() {
    assert_replay_equivalent(&[
        ("src/old.ts", Some(spec(&["foo"], &["./util"]))),
        ("src/util.ts", Some(spec(&["helper"], &[]))),
        ("src/old.ts", None),
        ("src/new.ts", Some(spec(&["foo"], &["./util"]))),
    ]);
}

#[test]
fn delta_replay_equivalence_delete_recreate() {
    assert_replay_equivalent(&[
        ("src/a.ts", Some(spec(&["foo"], &[]))),
        ("src/a.ts", None),
        ("src/a.ts", Some(spec(&["foo"], &[]))),
    ]);
}

#[test]
fn delta_replay_equivalence_cross_file_imports_in_any_order() {
    // Importer applied before importee, plus an external bare import.
    assert_replay_equivalent(&[
        ("src/main.ts", Some(spec(&["app"], &["./svc", "axios"]))),
        ("src/svc.ts", Some(spec(&["serve"], &["axios"]))),
    ]);
}

#[test]
fn delta_replay_equivalence_modify_drops_an_import() {
    assert_replay_equivalent(&[
        ("src/svc.ts", Some(spec(&["serve"], &[]))),
        ("src/main.ts", Some(spec(&["app"], &["./svc"]))),
        // main re-saved without the ./svc import.
        ("src/main.ts", Some(spec(&["app"], &[]))),
    ]);
}

#[test]
fn delta_replay_equivalence_interleaved_sequence() {
    assert_replay_equivalent(&[
        ("a.ts", Some(spec(&["a1", "a2"], &["./b"]))),
        ("b.ts", Some(spec(&["b1"], &[]))),
        ("c.ts", Some(spec(&["c1"], &["./a", "./b"]))),
        ("a.ts", Some(spec(&["a1"], &["./b"]))),  // drop a2
        ("b.ts", None),                           // delete b
        ("b.ts", Some(spec(&["b1", "b2"], &[]))), // recreate b larger
        ("d.ts", Some(spec(&["d1"], &["axios"]))),
    ]);
}

// ---------- per-field honesty ----------

#[test]
fn schema_version_is_set_not_zero() {
    let mut g = SymbolGraph::new();
    let d = update_file(&mut g, file_symbols("a.ts", &spec(&["foo"], &[]), 0));
    assert_eq!(d.schema_version, GRAPH_DELTA_SCHEMA_VERSION);
    assert_ne!(d.schema_version, 0, "schema_version must never be unset");

    let del = remove_file(&mut g, "a.ts");
    assert_eq!(del.schema_version, GRAPH_DELTA_SCHEMA_VERSION);
}

#[test]
fn removed_edges_populated_when_import_dropped() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols("src/svc.ts", &spec(&["serve"], &[]), 0),
    );
    update_file(
        &mut g,
        file_symbols("src/main.ts", &spec(&["app"], &["./svc"]), 100),
    );

    // Re-save main without the import: the old import edge must be reported.
    let d = update_file(
        &mut g,
        file_symbols("src/main.ts", &spec(&["app"], &[]), 200),
    );
    assert!(
        d.removed_edges
            .iter()
            .any(|(_, _, t)| *t == EdgeType::Imports),
        "dropping an import must populate removed_edges, got {:?}",
        d.removed_edges
    );
}

#[test]
fn removed_edges_populated_on_delete() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols("src/svc.ts", &spec(&["serve"], &[]), 0),
    );
    update_file(
        &mut g,
        file_symbols("src/main.ts", &spec(&["app"], &["./svc"]), 100),
    );
    let d = remove_file(&mut g, "src/main.ts");
    assert!(
        !d.removed_edges.is_empty(),
        "deleting a file with an import must report the removed edge"
    );
}

#[test]
fn node_changes_classify_added_changed_removed() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols("a.ts", &spec(&["keep", "drop"], &[]), 0),
    );

    // Re-save: keep `keep` (Changed — re-inserted same identity), remove
    // `drop` (Removed), add `fresh` (Added).
    let d = update_file(
        &mut g,
        file_symbols("a.ts", &spec(&["keep", "fresh"], &[]), 100),
    );

    let named = |pred: &dyn Fn(&NodeChange) -> bool| {
        d.node_changes
            .iter()
            .filter(|c| pred(c))
            .map(|c| c.identity().name.clone())
            .collect::<BTreeSet<_>>()
    };
    let added = named(&|c| matches!(c, NodeChange::Added(_)));
    let changed = named(&|c| matches!(c, NodeChange::Changed(_)));
    let removed = named(&|c| matches!(c, NodeChange::Removed(_)));

    assert!(
        changed.contains("keep"),
        "keep should be Changed: {changed:?}"
    );
    assert!(added.contains("fresh"), "fresh should be Added: {added:?}");
    assert!(
        removed.contains("drop"),
        "drop should be Removed: {removed:?}"
    );
}

#[test]
fn node_changes_empty_meaning_for_body_only_edit() {
    // Re-saving identical content: every identity persists, so all changes are
    // Changed (re-inserted) — none Added, none Removed.
    let mut g = SymbolGraph::new();
    update_file(&mut g, file_symbols("a.ts", &spec(&["foo", "bar"], &[]), 0));
    let d = update_file(
        &mut g,
        file_symbols("a.ts", &spec(&["foo", "bar"], &[]), 100),
    );

    assert!(
        d.node_changes
            .iter()
            .all(|c| matches!(c, NodeChange::Changed(_))),
        "a body-only re-save yields only Changed nodes, got {:?}",
        d.node_changes
    );
    assert_eq!(d.node_changes.len(), 2);
}

#[test]
fn delete_reports_all_nodes_removed() {
    let mut g = SymbolGraph::new();
    update_file(&mut g, file_symbols("a.ts", &spec(&["foo", "bar"], &[]), 0));
    let d = remove_file(&mut g, "a.ts");
    assert_eq!(d.node_changes.len(), 2);
    assert!(
        d.node_changes
            .iter()
            .all(|c| matches!(c, NodeChange::Removed(_))),
        "delete must classify every node as Removed"
    );
}

// ---------- overload-sensitivity + edge-scope (council-driven) ----------

/// Build `FileSymbols` allowing repeated names (overloads) — the multiset
/// fingerprint must distinguish overload counts.
fn file_symbols_overloads(file: &str, names: &[&str], base: u64) -> FileSymbols {
    FileSymbols {
        file: file.to_string(),
        symbols: names
            .iter()
            .enumerate()
            .map(|(i, n)| SymbolNode {
                id: base + i as u64,
                kind: SymbolKind::Function,
                name: (*n).to_string(),
                visibility: Visibility::Public,
                file: file.to_string(),
                trust_level: TrustLevel::Unknown,
                span: None,
            })
            .collect(),
        imports: Vec::new(),
        reexports: Vec::new(),
        calls: Vec::new(),
        calls_partial: false,
        has_unresolved_dynamic_import: false,
    }
}

#[test]
fn delta_replay_equivalence_is_overload_count_sensitive() {
    // Incrementally end at two `foo` overloads; rebuild also has two. The
    // multiset fingerprint must see count 2, not collapse to 1 — and the
    // incremental path (add 2, drop to 1, add back to 2) must converge.
    let mut g = SymbolGraph::new();
    update_file(&mut g, file_symbols_overloads("a.ts", &["foo", "foo"], 0));
    update_file(&mut g, file_symbols_overloads("a.ts", &["foo"], 100)); // drop one
    update_file(&mut g, file_symbols_overloads("a.ts", &["foo", "foo"], 200)); // back to two

    let mut rebuilt = SymbolGraph::new();
    update_file(
        &mut rebuilt,
        file_symbols_overloads("a.ts", &["foo", "foo"], 0),
    );

    let fp = fingerprint(&g);
    assert_eq!(fp, fingerprint(&rebuilt));
    assert_eq!(
        fp.symbols
            .get(&("a.ts".into(), "Function".into(), "foo".into())),
        Some(&2),
        "fingerprint must record both overloads, not collapse them"
    );
}

#[test]
fn removed_edges_excludes_incoming_cross_file_edges() {
    // b.ts imports a.ts. Re-saving a.ts (body-only) must NOT report the
    // b->a edge in removed_edges — that incoming edge belongs to b's import
    // decision, which did not change. (Council finding C-001.)
    let mut g = SymbolGraph::new();
    update_file(&mut g, file_symbols("src/a.ts", &spec(&["target"], &[]), 0));
    update_file(
        &mut g,
        file_symbols("src/b.ts", &spec(&["caller"], &["./a"]), 100),
    );
    // Sanity: the b->a dependency exists.
    assert!(
        dep_edges(&g).contains(&("src/b.ts".to_string(), "src/a.ts".to_string())),
        "precondition: b imports a"
    );

    // Re-save a.ts with the same surface.
    let d = update_file(
        &mut g,
        file_symbols("src/a.ts", &spec(&["target"], &[]), 200),
    );
    assert!(
        d.removed_edges.is_empty(),
        "a body-only re-save of an imported file must not report incoming \
         edges as removed, got {:?}",
        d.removed_edges
    );
}

#[test]
fn per_delta_added_edges_account_for_resolved_imports() {
    // Importee-before-importer order: update_file alone (no re_resolve) must
    // report the cross-file edge in its own added_edges — not rely on a later
    // re-resolve pass to paper it over. (Council finding C-002.)
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols("src/util.ts", &spec(&["helper"], &[]), 0),
    );
    let d: GraphDelta = update_file(
        &mut g,
        file_symbols("src/main.ts", &spec(&["app"], &["./util"]), 100),
    );
    assert!(
        d.added_edges
            .iter()
            .any(|(_, _, t)| *t == EdgeType::Imports),
        "update_file must itself report the resolvable import edge, got {:?}",
        d.added_edges
    );
}

#[test]
fn node_changes_derive_total_order_is_stable() {
    // The derived NodeChange Ord must be a stable total order: classifying the
    // same change twice yields identical, sorted vectors.
    let mut g1 = SymbolGraph::new();
    update_file(
        &mut g1,
        file_symbols("a.ts", &spec(&["keep", "drop"], &[]), 0),
    );
    let d1 = update_file(
        &mut g1,
        file_symbols("a.ts", &spec(&["keep", "new"], &[]), 100),
    );

    let mut g2 = SymbolGraph::new();
    update_file(
        &mut g2,
        file_symbols("a.ts", &spec(&["keep", "drop"], &[]), 0),
    );
    let d2 = update_file(
        &mut g2,
        file_symbols("a.ts", &spec(&["keep", "new"], &[]), 100),
    );

    assert_eq!(
        d1.node_changes, d2.node_changes,
        "classification not stable"
    );
    let mut sorted = d1.node_changes.clone();
    sorted.sort();
    assert_eq!(
        d1.node_changes, sorted,
        "node_changes must be emitted sorted"
    );
}
