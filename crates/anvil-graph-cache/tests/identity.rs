//! GV2-002 stable-identity + export-diff contract tests.
//!
//! Exercises the identity contract end-to-end through `update_file` /
//! `remove_file` and the `export_surface_diff` primitive, covering the
//! validation cases the work item names: file rename, symbol rename,
//! delete/recreate, a same-`(kind, name)` overload added to an
//! already-public symbol (red before GV2-002), and same-name symbols in
//! different scopes — plus cross-restart identity stability.

use anvil_graph_cache::certify::{
    Certifiability, CertifyStale, ChangeKind, certify, export_surface_changed, export_surface_diff,
};
use anvil_graph_cache::dependency::DependencyGraph;
use anvil_graph_cache::incremental::{remove_file, update_file};
use anvil_graph_cache::symbol_graph::SymbolGraph;
use anvil_kernel_types::{
    FileSymbols, SymbolIdentity, SymbolKind, SymbolNode, TrustLevel, Visibility,
};

fn public_fn(id: u64, name: &str, file: &str) -> SymbolNode {
    SymbolNode {
        id,
        kind: SymbolKind::Function,
        name: name.to_string(),
        visibility: Visibility::Public,
        file: file.to_string(),
        trust_level: TrustLevel::Unknown,
    }
}

fn file_symbols(file: &str, symbols: Vec<SymbolNode>) -> FileSymbols {
    FileSymbols {
        file: file.to_string(),
        symbols,
        imports: Vec::new(),
        reexports: Vec::new(),
    }
}

/// A symbol rename within a file is classified as `renamed_public`, not as
/// an unrelated add + remove — and is still a surface change.
#[test]
fn identity_symbol_rename_classified_as_rename() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols("a.ts", vec![public_fn(1, "fetchUser", "a.ts")]),
    );

    // Save: fetchUser renamed to loadUser.
    let delta = update_file(
        &mut g,
        file_symbols("a.ts", vec![public_fn(10, "loadUser", "a.ts")]),
    );

    let diff = export_surface_diff(&g, &delta);
    assert!(!diff.is_empty(), "a public rename is a surface change");
    assert_eq!(diff.renamed_public.len(), 1);
    let (old, new) = &diff.renamed_public[0];
    assert_eq!(old.name, "fetchUser");
    assert_eq!(new.name, "loadUser");
    assert!(
        diff.added_public.is_empty() && diff.removed_public.is_empty(),
        "the rename pair must be drained from add/remove"
    );
}

/// Deleting a file and recreating it with identical content yields identical
/// identities — the surface diff across delete/recreate is empty, even
/// though every session-local id differs.
#[test]
fn identity_delete_recreate_yields_identical_identities() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols(
            "a.ts",
            vec![public_fn(1, "foo", "a.ts"), public_fn(2, "foo", "a.ts")],
        ),
    );
    let before = SymbolIdentity::for_file_symbols(&g.symbols_in_file("a.ts"));

    remove_file(&mut g, "a.ts");
    // Recreate with fresh (different) session-local ids.
    let delta = update_file(
        &mut g,
        file_symbols(
            "a.ts",
            vec![public_fn(100, "foo", "a.ts"), public_fn(101, "foo", "a.ts")],
        ),
    );
    let after = SymbolIdentity::for_file_symbols(&g.symbols_in_file("a.ts"));

    assert_eq!(before, after, "identities are session-id independent");
    // The recreate's delta carries an empty baseline (the file was absent),
    // so its diff reports the whole surface as added — certify already maps
    // Create to CrossFileResolutionNeeded. What must hold is comparability:
    // the recreated surface equals the pre-delete surface.
    assert_eq!(delta.added_symbols.len(), 2);
}

/// A file rename is delete + create: identities change with the path, and
/// the rename `ChangeKind` is never certified (documented stance — no
/// rename history is tracked, per privacy verdict PV-4).
#[test]
fn identity_file_rename_changes_identity_and_stays_partial() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols("old.ts", vec![public_fn(1, "foo", "old.ts")]),
    );
    let old_ids = SymbolIdentity::for_file_symbols(&g.symbols_in_file("old.ts"));

    // Rename arrives as delete(old) + create(new).
    remove_file(&mut g, "old.ts");
    let delta = update_file(
        &mut g,
        file_symbols("new.ts", vec![public_fn(2, "foo", "new.ts")]),
    );
    let new_ids = SymbolIdentity::for_file_symbols(&g.symbols_in_file("new.ts"));

    assert_ne!(old_ids, new_ids, "path is part of identity");
    let v = certify(
        &g,
        &DependencyGraph::new(),
        &ChangeKind::Rename,
        &delta,
        64,
        1,
    );
    assert_eq!(
        v,
        Certifiability::Partial {
            reason: CertifyStale::Renamed
        }
    );
}

/// The headline GV2-002 fix (red before stable identity): adding a public
/// overload — a second symbol with the same `(kind, name)` — to an
/// already-public symbol is a surface change and must not certify clean.
#[test]
fn identity_overload_added_to_public_symbol_does_not_certify_clean() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols("a.ts", vec![public_fn(1, "foo", "a.ts")]),
    );

    // Save: an overload of foo is added.
    let delta = update_file(
        &mut g,
        file_symbols(
            "a.ts",
            vec![public_fn(10, "foo", "a.ts"), public_fn(11, "foo", "a.ts")],
        ),
    );

    let diff = export_surface_diff(&g, &delta);
    assert_eq!(
        diff.added_public.len(),
        1,
        "the new overload is an added public identity"
    );
    assert_eq!(diff.added_public[0].ordinal, 1);
    assert!(diff.renamed_public.is_empty());

    let v = certify(
        &g,
        &DependencyGraph::new(),
        &ChangeKind::ContentModify,
        &delta,
        64,
        1,
    );
    assert!(
        matches!(v, Certifiability::Partial { .. }),
        "an overload addition must not certify clean, got {v:?}"
    );
}

/// Same-name symbols in different scopes stay distinct: a free function and
/// a method whose name encodes its owner never collapse into one identity.
#[test]
fn identity_same_name_different_scope_stays_distinct() {
    let mut g = SymbolGraph::new();
    let render_fn = public_fn(1, "render", "a.ts");
    let render_method = SymbolNode {
        id: 2,
        kind: SymbolKind::Method,
        name: "Widget.render".to_string(),
        visibility: Visibility::Public,
        file: "a.ts".to_string(),
        trust_level: TrustLevel::Unknown,
    };
    update_file(
        &mut g,
        file_symbols("a.ts", vec![render_fn.clone(), render_method.clone()]),
    );

    // Body-only save: same surface → empty diff (the two `render`s did not
    // collide into a false ordinal bump).
    let delta = update_file(
        &mut g,
        file_symbols(
            "a.ts",
            vec![
                SymbolNode {
                    id: 10,
                    ..render_fn
                },
                SymbolNode {
                    id: 11,
                    ..render_method
                },
            ],
        ),
    );
    assert!(
        !export_surface_changed(&g, &delta),
        "distinct-scope same-name symbols must not read as a surface change"
    );
}

/// Cross-restart stability: two independent sessions parsing the same file
/// produce equal public baselines, regardless of id allocation order.
#[test]
fn identity_stable_across_sessions() {
    let build = |base_id: u64| {
        let mut g = SymbolGraph::new();
        update_file(
            &mut g,
            file_symbols(
                "src/api.ts",
                vec![
                    public_fn(base_id, "foo", "src/api.ts"),
                    public_fn(base_id + 1, "foo", "src/api.ts"),
                    public_fn(base_id + 2, "bar", "src/api.ts"),
                ],
            ),
        );
        // A body-only re-save exposes the baseline the next delta carries.
        update_file(
            &mut g,
            file_symbols(
                "src/api.ts",
                vec![
                    public_fn(base_id + 10, "foo", "src/api.ts"),
                    public_fn(base_id + 11, "foo", "src/api.ts"),
                    public_fn(base_id + 12, "bar", "src/api.ts"),
                ],
            ),
        )
        .previously_public
    };

    let session_a = build(0);
    let session_b = build(5000);
    assert_eq!(
        session_a, session_b,
        "baselines must be comparable across restarts"
    );
}

/// Ambiguous rename shapes stay conservative: two public renames of the same
/// kind in one save are reported as adds + removes, never mispaired.
#[test]
fn identity_ambiguous_renames_fall_back_to_add_remove() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols(
            "a.ts",
            vec![public_fn(1, "alpha", "a.ts"), public_fn(2, "beta", "a.ts")],
        ),
    );

    let delta = update_file(
        &mut g,
        file_symbols(
            "a.ts",
            vec![
                public_fn(10, "gamma", "a.ts"),
                public_fn(11, "delta", "a.ts"),
            ],
        ),
    );

    let diff = export_surface_diff(&g, &delta);
    assert!(diff.renamed_public.is_empty(), "ambiguous pairing refused");
    assert_eq!(diff.added_public.len(), 2);
    assert_eq!(diff.removed_public.len(), 2);
    assert!(!diff.is_empty());
}

/// A body-only edit still certifies self-only under identity baselines —
/// the conservative default did not get more conservative.
#[test]
fn identity_body_only_edit_still_certifies() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols("a.ts", vec![public_fn(1, "foo", "a.ts")]),
    );
    let delta = update_file(
        &mut g,
        file_symbols("a.ts", vec![public_fn(10, "foo", "a.ts")]),
    );
    let v = certify(
        &g,
        &DependencyGraph::new(),
        &ChangeKind::ContentModify,
        &delta,
        64,
        1,
    );
    assert!(matches!(v, Certifiability::Certified { .. }));
}

/// Documents the known residual (see `ExportSurfaceDiff` docs): removing one
/// overload and adding a different one in the same save with the total count
/// preserved re-assigns the same identity set, so the surface reads
/// unchanged and the save certifies. Shared with the old string-key scheme;
/// closing it needs per-symbol signature content, which privacy verdict PV-1
/// excludes from identity. If this test ever fails, the limitation has been
/// closed — update the docs that name it.
#[test]
fn identity_count_preserving_overload_churn_is_a_known_residual() {
    let mut g = SymbolGraph::new();
    update_file(
        &mut g,
        file_symbols(
            "a.ts",
            vec![public_fn(1, "foo", "a.ts"), public_fn(2, "foo", "a.ts")],
        ),
    );

    // One overload removed AND a different one appended: count preserved.
    let delta = update_file(
        &mut g,
        file_symbols(
            "a.ts",
            vec![public_fn(10, "foo", "a.ts"), public_fn(11, "foo", "a.ts")],
        ),
    );

    let diff = export_surface_diff(&g, &delta);
    assert!(
        diff.is_empty(),
        "count-preserving same-name churn is the documented undetectable shape"
    );
}
