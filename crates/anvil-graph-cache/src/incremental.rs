use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use anvil_kernel_types::{
    EdgeType, FileSymbols, ImportEdge, SymbolIdentity, SymbolKind, SymbolNode, TrustLevel,
    Visibility,
};

use super::symbol_graph::SymbolGraph;

/// Schema version of the [`GraphDelta`] event contract (GV2-003).
///
/// Bumped whenever the meaning or completeness of a delta field changes, so a
/// consumer can refuse a delta it does not understand rather than silently
/// mis-apply it. Carried on every delta via the manual [`Default`] impl and
/// the constructing functions; never 0 (a 0 would mean "unset", which the
/// manual `Default` prevents).
pub const GRAPH_DELTA_SCHEMA_VERSION: u32 = 1;

// A delta's `schema_version` of 0 means "unset"; the contract requires a real
// version on every delta. Guard the invariant at compile time.
const _: () = assert!(GRAPH_DELTA_SCHEMA_VERSION != 0);

/// How a symbol's stable identity related to the file's prior state across one
/// update (GV2-003). Anchors each touched node to its [`SymbolIdentity`] so a
/// consumer can reason about the change without re-deriving identities or
/// reaching into the (already-mutated) graph for removed nodes.
///
/// The derived `Ord` is `Added < Changed < Removed`, then by identity — the
/// order [`classify_node_changes`] sorts into.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeChange {
    /// The identity is new this update — no symbol with it existed before.
    Added(SymbolIdentity),
    /// The identity existed before and still exists; the symbol was
    /// re-inserted (a body edit, or a visibility/trust change). "Changed"
    /// here is identity-preserving: same `(file, kind, name, ordinal)`,
    /// potentially different visibility/trust.
    ///
    /// This variant says *that* the node changed, not *what* changed — the
    /// node payload (visibility, trust) is not carried. To learn whether the
    /// change was security-relevant (an `Internal → Public` surface expansion
    /// or a trust escalation), join the identity against the delta's
    /// `previously_public` / `previously_privileged` / `previously_boundary`
    /// baselines, which is exactly what `certify::export_surface_diff` does.
    Changed(SymbolIdentity),
    /// The identity existed before and is gone after this update.
    Removed(SymbolIdentity),
}

impl NodeChange {
    /// The stable identity this change anchors to.
    #[must_use]
    pub fn identity(&self) -> &SymbolIdentity {
        match self {
            NodeChange::Added(id) | NodeChange::Changed(id) | NodeChange::Removed(id) => id,
        }
    }
}

/// Changes produced by an incremental graph update.
///
/// The id-keyed channels (`added_symbols`, `removed_symbols`, `added_edges`,
/// `removed_edges`) describe the change in terms of this graph generation's
/// session-local `u64` ids — the form the policy invariants and `watch.rs`
/// consume. The identity-anchored `node_changes` channel (GV2-003) describes
/// the *same* change in terms of stable [`SymbolIdentity`] so the distinction
/// between a genuinely new node, an identity-preserving change, and a removal
/// survives the remove-then-re-add that `update_file` performs.
///
/// Every field reports honestly: `removed_edges` is populated (GV2-003 — it
/// was permanently empty before), and no field is present that the pipeline
/// cannot fill. **Content hashes** ride the `FileSymbols` parser feed
/// (hashed at the `validate_paths` boundary, not recomputable here without
/// file bytes) and **provenance/session anchors** are join-time-only (privacy
/// verdict PV-3, 2026-06-08); neither is a `GraphDelta` field because this
/// layer cannot populate them truthfully.
#[derive(Debug, Clone)]
pub struct GraphDelta {
    /// Event-contract schema version. See [`GRAPH_DELTA_SCHEMA_VERSION`].
    pub schema_version: u32,
    pub added_symbols: Vec<u64>,
    pub removed_symbols: Vec<u64>,
    pub added_edges: Vec<(u64, u64, EdgeType)>,
    /// The file's own **outgoing** edges removed by this update, in this
    /// generation's ids — every `EdgeType`, not only `Imports` (today the graph
    /// only carries `Imports` edges, but this channel reports whatever outgoing
    /// edges the file's symbols held, so it stays complete as new edge kinds
    /// land). Captured from the file's symbols before `remove_file` drops them.
    /// Scoped to outgoing deliberately: an *incoming* edge (another file imports
    /// this one) belongs to that other file's decision and its dependency does
    /// not cease when this file is re-saved — "who depends on me" is answered by
    /// `DependencyGraph::dependents_of`, never by this channel (see `certify`
    /// rustdoc). On a body-only edit the old ids are genuinely gone, so this
    /// honestly reports the removed edges even when the same edge is re-added
    /// under fresh ids; the identity-level "did the import shape change" question
    /// is answered by `node_changes` + `previously_imported`, not by diffing raw
    /// ids.
    pub removed_edges: Vec<(u64, u64, EdgeType)>,
    /// Identity-anchored view of the node change set (GV2-003): added,
    /// identity-preserving-changed, and removed nodes for `file`. Lets a
    /// consumer replay or reconcile over stable identity rather than
    /// session-local ids.
    pub node_changes: Vec<NodeChange>,
    pub errors: Vec<String>,
    /// Import sources that existed before this update (for new-dep detection).
    pub previously_imported: HashSet<String>,
    /// Stable identities that were already public before this update (for
    /// API-expansion detection). Keyed by [`SymbolIdentity`] (GV2-002), so
    /// same-`(kind, name)` overloads stay distinct via their ordinal instead
    /// of collapsing into one string key.
    pub previously_public: HashSet<SymbolIdentity>,
    /// Stable identities that were already `TrustLevel::Privileged` before
    /// this update — and only `Privileged`. The `PrivilegeExpansion`
    /// invariant compares against this set exclusively, so a
    /// `Boundary → Privileged` escalation is *not* in the baseline and
    /// correctly fires.
    pub previously_privileged: HashSet<SymbolIdentity>,
    /// Stable identities that were already `TrustLevel::Boundary` before
    /// this update. Kept separate from `previously_privileged` because the
    /// two consumers need different semantics: the certify export-diff
    /// treats `Privileged ∪ Boundary` as the elevated surface (spec gap
    /// G-06), while the privilege-expansion invariant must see
    /// `Privileged`-only (`Boundary` marks a public API boundary, not
    /// privileged module access — `annotate_trust` assigns it to every
    /// public symbol outside privileged files).
    pub previously_boundary: HashSet<SymbolIdentity>,
    pub file: String,
}

impl Default for GraphDelta {
    /// An empty delta carrying the current schema version — never version 0.
    fn default() -> Self {
        Self {
            schema_version: GRAPH_DELTA_SCHEMA_VERSION,
            added_symbols: Vec::new(),
            removed_symbols: Vec::new(),
            added_edges: Vec::new(),
            removed_edges: Vec::new(),
            node_changes: Vec::new(),
            errors: Vec::new(),
            previously_imported: HashSet::new(),
            previously_public: HashSet::new(),
            previously_privileged: HashSet::new(),
            previously_boundary: HashSet::new(),
            file: String::new(),
        }
    }
}

/// Is this trust level part of the elevated surface the export-diff watches?
///
/// `Privileged` and `Boundary` both count: a symbol crossing onto either is
/// an elevated-surface change for `certify::export_surface_diff` (spec gap
/// G-06 — the old filter dropped `Boundary`, so a producer emitting it on a
/// non-public symbol would have made the export-diff silently under-fire).
/// This predicate is for the *diff* path only: the `PrivilegeExpansion`
/// invariant intentionally checks `Privileged` alone, against the
/// `Privileged`-only `previously_privileged` baseline, so `annotate_trust`'s
/// blanket `Boundary` on public symbols never spams privileged-access
/// violations and a `Boundary → Privileged` escalation still fires.
#[must_use]
pub fn is_elevated_trust(trust: TrustLevel) -> bool {
    matches!(trust, TrustLevel::Privileged | TrustLevel::Boundary)
}

impl GraphDelta {
    pub fn is_empty(&self) -> bool {
        self.added_symbols.is_empty()
            && self.removed_symbols.is_empty()
            && self.added_edges.is_empty()
            && self.removed_edges.is_empty()
            && self.node_changes.is_empty()
            && self.errors.is_empty()
    }
}

/// Everything about a file's prior graph state that `remove_file` destroys and
/// the resulting [`GraphDelta`] needs — captured in one pass before removal.
struct PriorState {
    previously_public: HashSet<SymbolIdentity>,
    previously_privileged: HashSet<SymbolIdentity>,
    previously_boundary: HashSet<SymbolIdentity>,
    previously_imported: HashSet<String>,
    /// The file's stable identities before the update — for the
    /// changed/removed node-change classification after re-add.
    old_identity_set: HashSet<SymbolIdentity>,
    /// The file's own outgoing edges (every `EdgeType`), de-duplicated and
    /// sorted — the edges this file's symbols are dropping (GV2-003). Incoming
    /// edges are deliberately excluded: they belong to other files' deltas, and
    /// capturing them would make this an O(graph) scan on the save-time path
    /// for a widely-imported file.
    removed_edges: Vec<(u64, u64, EdgeType)>,
}

impl PriorState {
    fn capture(graph: &SymbolGraph, file: &str) -> Self {
        let old_symbols = graph.symbols_in_file(file);
        let old_ids: Vec<u64> = old_symbols.iter().map(|s| s.id).collect();
        // Stable identities are assigned over the file's full parse-ordered
        // symbol list (GV2-002): ordinals disambiguate same-(kind, name)
        // overloads regardless of visibility, so the baselines stay distinct
        // per overload instead of collapsing into one key.
        // INVARIANT: symbols_in_file returns insertion order, which equals
        // parse order because update_file feeds FileSymbols.symbols in parser
        // emission order — the ordering contract for_file_symbols documents.
        let old_identities = SymbolIdentity::for_file_symbols(&old_symbols);
        let by_trust = |want: TrustLevel| -> HashSet<SymbolIdentity> {
            old_symbols
                .iter()
                .zip(&old_identities)
                .filter(|(s, _)| s.trust_level == want)
                .map(|(_, identity)| identity.clone())
                .collect()
        };
        let previously_public: HashSet<SymbolIdentity> = old_symbols
            .iter()
            .zip(&old_identities)
            .filter(|(s, _)| s.visibility == Visibility::Public)
            .map(|(_, identity)| identity.clone())
            .collect();
        let previously_privileged = by_trust(TrustLevel::Privileged);
        let previously_boundary = by_trust(TrustLevel::Boundary);
        let old_identity_set: HashSet<SymbolIdentity> = old_identities.into_iter().collect();

        let previously_imported: HashSet<String> = old_ids
            .iter()
            .flat_map(|&id| graph.outgoing_edges(id))
            .filter(|e| e.edge_type == EdgeType::Imports)
            .filter_map(|e| graph.get_symbol(e.to).map(|s| s.file.clone()))
            .collect();

        let mut edge_set: HashSet<(u64, u64, EdgeType)> = HashSet::new();
        for &id in &old_ids {
            for e in graph.outgoing_edges(id) {
                edge_set.insert((e.from, e.to, e.edge_type));
            }
        }
        let mut removed_edges: Vec<_> = edge_set.into_iter().collect();
        removed_edges.sort();

        Self {
            previously_public,
            previously_privileged,
            previously_boundary,
            previously_imported,
            old_identity_set,
            removed_edges,
        }
    }
}

/// Apply an incremental update to the graph for a single file.
///
/// 1. Remove all symbols and edges for the file
/// 2. Add new symbols from the re-parsed file
/// 3. Return the delta for downstream consumers (policy engine)
pub fn update_file(graph: &mut SymbolGraph, new_symbols: FileSymbols) -> GraphDelta {
    let file = new_symbols.file.clone();

    // Snapshot everything about the file's prior state that `remove_file` is
    // about to destroy (baselines, prior identities, incident edges).
    let before = PriorState::capture(graph, &file);
    let PriorState {
        previously_public,
        previously_privileged,
        previously_boundary,
        previously_imported,
        old_identity_set,
        removed_edges,
    } = before;

    let removed_ids = graph.remove_file(&file);

    let mut added_ids = Vec::new();
    let mut errors = Vec::new();
    for symbol in new_symbols.symbols {
        let id = symbol.id;
        match graph.add_symbol(symbol) {
            Ok(_) => added_ids.push(id),
            Err(e) => {
                tracing::warn!("graph: failed to insert symbol {id}: {e}");
                errors.push(format!("symbol {id}: {e}"));
            }
        }
    }

    // Collect all known file paths in the graph for import resolution.
    // Use BTreeSet for deterministic ordering so ambiguous matches are resolved
    // consistently regardless of HashMap iteration order.
    let known_files: Vec<String> = graph
        .inner()
        .node_weights()
        .map(|s| s.file.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // Use the first added symbol as the edge origin. If the file has no
    // symbols (side-effect-only module), create a synthetic Module node
    // so import edges are still recorded. None means we have no usable
    // source node — id 0 is a legitimate symbol id and must not be used
    // as a sentinel here.
    let from_id: Option<u64> = if let Some(&id) = added_ids.first() {
        Some(id)
    } else if !new_symbols.imports.is_empty() {
        let synthetic_id = graph.next_id();
        let synthetic = SymbolNode {
            id: synthetic_id,
            kind: SymbolKind::Module,
            name: file.clone(),
            visibility: Visibility::Internal,
            file: file.clone(),
            trust_level: TrustLevel::Unknown,
        };
        if graph.add_symbol(synthetic).is_ok() {
            added_ids.push(synthetic_id);
            Some(synthetic_id)
        } else {
            None
        }
    } else {
        None
    };

    let mut added_edges = Vec::new();
    if let Some(from) = from_id {
        for import in new_symbols.imports {
            // Resolve the import specifier to a known file path
            let to_id = resolve_import(&import.to_source, &file, &known_files, graph);

            if let Some(to) = to_id {
                let edge = anvil_kernel_types::SymbolEdge {
                    from,
                    to,
                    edge_type: EdgeType::Imports,
                };
                if graph.add_edge(edge).is_ok() {
                    added_edges.push((from, to, EdgeType::Imports));
                }
            }
        }
    }

    // GV2-003: classify the file's nodes against their prior identities.
    // `update_file` removes-then-re-adds, so a body edit re-inserts the same
    // identity under a fresh id — that is a `Changed`, not an `Added`. A new
    // identity is `Added`; a prior identity now absent is `Removed`.
    let node_changes = classify_node_changes(graph, &file, &old_identity_set);

    // Counts only — no identities, names, or paths beyond the target file the
    // caller already supplied — so this stays inside the privacy verdict's
    // PV-10 telemetry rules.
    tracing::debug!(
        target: "anvil_graph_cache::delta",
        added = added_ids.len(),
        removed = removed_ids.len(),
        added_edges = added_edges.len(),
        removed_edges = removed_edges.len(),
        node_changes = node_changes.len(),
        "update_file delta"
    );

    GraphDelta {
        schema_version: GRAPH_DELTA_SCHEMA_VERSION,
        added_symbols: added_ids,
        removed_symbols: removed_ids,
        added_edges,
        removed_edges,
        node_changes,
        errors,
        previously_imported,
        previously_public,
        previously_privileged,
        previously_boundary,
        file,
    }
}

/// Build the identity-anchored [`NodeChange`] list for `file` against the
/// `before` identity set (GV2-003).
///
/// Reads the post-update symbols of `file` from `graph`, assigns their stable
/// identities, and partitions: a post-update identity present in `before` is
/// `Changed` (re-inserted), absent from `before` is `Added`; a `before`
/// identity absent post-update is `Removed`. Output is sorted for a
/// reproducible delta.
fn classify_node_changes(
    graph: &SymbolGraph,
    file: &str,
    before: &HashSet<SymbolIdentity>,
) -> Vec<NodeChange> {
    let current = graph.symbols_in_file(file);
    let current_identities = SymbolIdentity::for_file_symbols(&current);
    let current_set: HashSet<SymbolIdentity> = current_identities.iter().cloned().collect();

    let mut changes: Vec<NodeChange> = Vec::new();
    for identity in &current_identities {
        if before.contains(identity) {
            changes.push(NodeChange::Changed(identity.clone()));
        } else {
            changes.push(NodeChange::Added(identity.clone()));
        }
    }
    for identity in before {
        if !current_set.contains(identity) {
            changes.push(NodeChange::Removed(identity.clone()));
        }
    }
    // Added/Changed/Removed are disjoint by construction (an identity is in
    // current-only, both, or before-only), so the derived `NodeChange` order
    // (variant, then identity) is already a total order with no ties to break.
    changes.sort();
    changes
}

/// Resolve an import specifier to a symbol ID in the graph.
///
/// For relative imports (`./module`, `../lib`), resolve against the importing
/// file's directory and try common extensions (.ts, .tsx, .js, /index.ts, etc.).
/// For bare specifiers (`express`, `node:fs`), match against file names directly
/// (these represent external/virtual modules).
pub(crate) fn resolve_import(
    specifier: &str,
    from_file: &str,
    known_files: &[String],
    graph: &mut SymbolGraph,
) -> Option<u64> {
    // Non-relative imports: find or create an external module node.
    // External packages (axios, node:fs, etc.) won't have pre-existing
    // graph nodes, so we create one on demand to enable edge tracking.
    if !specifier.starts_with('.') {
        if let Some(existing) = graph.inner().node_weights().find(|s| s.file == specifier) {
            return Some(existing.id);
        }
        // Create a synthetic external node
        let ext_id = graph.next_id();
        let ext_node = SymbolNode {
            id: ext_id,
            kind: SymbolKind::Module,
            name: specifier.to_string(),
            visibility: Visibility::Public,
            file: specifier.to_string(),
            trust_level: TrustLevel::External,
        };
        if graph.add_symbol(ext_node).is_ok() {
            return Some(ext_id);
        }
        return None;
    }

    // Relative imports: resolve against the importing file's directory
    let from_dir = Path::new(from_file).parent().unwrap_or(Path::new(""));
    let raw_joined = from_dir.join(specifier);
    // Normalise away . and .. components (no filesystem access needed)
    let mut components = Vec::new();
    for comp in raw_joined.components() {
        match comp {
            std::path::Component::CurDir => {} // skip "."
            std::path::Component::ParentDir => {
                components.pop();
            }
            other => components.push(other),
        }
    }
    let resolved: std::path::PathBuf = components.iter().collect();
    let resolved_str = resolved.to_string_lossy();

    // Try exact match, then common extensions
    let candidates = [
        resolved_str.to_string(),
        format!("{resolved_str}.ts"),
        format!("{resolved_str}.tsx"),
        format!("{resolved_str}.js"),
        format!("{resolved_str}.jsx"),
        format!("{resolved_str}/index.ts"),
        format!("{resolved_str}/index.js"),
    ];

    for candidate in &candidates {
        // Normalise path separators for comparison
        let normalised = candidate.replace('\\', "/");

        // Collect all matching files, preferring exact matches then shortest path
        // (most specific) to avoid nondeterministic resolution when multiple
        // files share a suffix (e.g. src/utils.ts vs packages/app/src/utils.ts).
        let mut matches: Vec<&String> = known_files
            .iter()
            .filter(|f| {
                let f_norm = f.replace('\\', "/");
                f_norm == normalised || f_norm.ends_with(&format!("/{normalised}"))
            })
            .collect();

        if !matches.is_empty() {
            // Prefer exact match, then shortest path (most specific)
            matches.sort_by_key(|f| f.len());
            let file_path = matches[0];
            return graph
                .inner()
                .node_weights()
                .find(|s| s.file == *file_path)
                .map(|s| s.id);
        }
    }

    None
}

/// Re-resolve imports that could not be resolved during initial scan because
/// the target file had not been parsed yet.
pub fn re_resolve_imports(graph: &mut SymbolGraph, imports: &[ImportEdge]) {
    let known_files: Vec<String> = graph
        .inner()
        .node_weights()
        .map(|s| s.file.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    for import in imports {
        let from_id = graph
            .symbols_in_file(&import.from_file)
            .first()
            .map(|s| s.id);
        let Some(from) = from_id else { continue };

        let to_id = resolve_import(&import.to_source, &import.from_file, &known_files, graph);
        let Some(to) = to_id else { continue };

        let already_exists = graph
            .outgoing_edges(from)
            .iter()
            .any(|e| e.to == to && e.edge_type == EdgeType::Imports);
        if already_exists {
            continue;
        }

        let edge = anvil_kernel_types::SymbolEdge {
            from,
            to,
            edge_type: EdgeType::Imports,
        };
        let _ = graph.add_edge(edge);
    }
}

/// Remove a deleted file from the graph entirely.
///
/// GV2-003: reports the removal honestly — `removed_edges` carries the file's
/// own outgoing import edges (scoped exactly as `update_file`'s; see the field
/// docs), and `node_changes` lists each removed identity as
/// [`NodeChange::Removed`], sorted to match `update_file`'s ordering — so a
/// delete is a complete, consistently-shaped event, not just a list of
/// vanished ids.
///
/// `previously_imported` is intentionally left empty: it exists for the
/// new-dependency invariant's re-add suppression, which is only meaningful on
/// an `update_file`, never on a delete (a deleted file imports nothing after).
pub fn remove_file(graph: &mut SymbolGraph, file: &str) -> GraphDelta {
    let old_symbols = graph.symbols_in_file(file);
    let old_ids: Vec<u64> = old_symbols.iter().map(|s| s.id).collect();
    let mut removed_node_changes: Vec<NodeChange> = SymbolIdentity::for_file_symbols(&old_symbols)
        .into_iter()
        .map(NodeChange::Removed)
        .collect();
    removed_node_changes.sort();
    drop(old_symbols);

    let mut removed_edge_set: HashSet<(u64, u64, EdgeType)> = HashSet::new();
    for &id in &old_ids {
        for e in graph.outgoing_edges(id) {
            removed_edge_set.insert((e.from, e.to, e.edge_type));
        }
    }
    let mut removed_edges: Vec<_> = removed_edge_set.into_iter().collect();
    removed_edges.sort();

    let removed_ids = graph.remove_file(file);
    GraphDelta {
        removed_symbols: removed_ids,
        removed_edges,
        node_changes: removed_node_changes,
        file: file.to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{ImportEdge, SymbolKind, SymbolNode, TrustLevel, Visibility};

    fn make_file_symbols(file: &str, symbols: Vec<(u64, &str, SymbolKind)>) -> FileSymbols {
        FileSymbols {
            file: file.to_string(),
            symbols: symbols
                .into_iter()
                .map(|(id, name, kind)| SymbolNode {
                    id,
                    kind,
                    name: name.to_string(),
                    visibility: Visibility::Internal,
                    file: file.to_string(),
                    trust_level: TrustLevel::Unknown,
                })
                .collect(),
            imports: Vec::new(),
            reexports: Vec::new(),
        }
    }

    #[test]
    fn initial_file_add_produces_delta() {
        let mut g = SymbolGraph::new();
        let syms = make_file_symbols(
            "a.ts",
            vec![
                (1, "foo", SymbolKind::Function),
                (2, "Bar", SymbolKind::Class),
            ],
        );

        let delta = update_file(&mut g, syms);

        assert_eq!(delta.file, "a.ts");
        assert_eq!(delta.added_symbols.len(), 2);
        assert!(delta.removed_symbols.is_empty());
        assert_eq!(g.node_count(), 2);
    }

    #[test]
    fn file_update_replaces_symbols() {
        let mut g = SymbolGraph::new();

        let syms1 = make_file_symbols(
            "a.ts",
            vec![
                (1, "foo", SymbolKind::Function),
                (2, "bar", SymbolKind::Function),
            ],
        );
        update_file(&mut g, syms1);
        assert_eq!(g.node_count(), 2);

        let syms2 = make_file_symbols("a.ts", vec![(10, "baz", SymbolKind::Function)]);
        let delta = update_file(&mut g, syms2);

        assert_eq!(delta.removed_symbols.len(), 2);
        assert_eq!(delta.added_symbols.len(), 1);
        assert_eq!(g.node_count(), 1);
        assert!(g.get_symbol(10).is_some());
        assert!(g.get_symbol(1).is_none());
    }

    #[test]
    fn remove_file_produces_delta() {
        let mut g = SymbolGraph::new();
        let syms = make_file_symbols("a.ts", vec![(1, "foo", SymbolKind::Function)]);
        update_file(&mut g, syms);

        let delta = remove_file(&mut g, "a.ts");
        assert_eq!(delta.removed_symbols.len(), 1);
        assert!(delta.added_symbols.is_empty());
        assert_eq!(g.node_count(), 0);
    }

    #[test]
    fn update_preserves_other_files() {
        let mut g = SymbolGraph::new();

        update_file(
            &mut g,
            make_file_symbols("a.ts", vec![(1, "foo", SymbolKind::Function)]),
        );
        update_file(
            &mut g,
            make_file_symbols("b.ts", vec![(2, "bar", SymbolKind::Function)]),
        );

        let _delta = update_file(
            &mut g,
            make_file_symbols("a.ts", vec![(10, "baz", SymbolKind::Function)]),
        );

        assert_eq!(g.node_count(), 2);
        assert!(g.get_symbol(10).is_some());
        assert!(g.get_symbol(2).is_some());
    }

    #[test]
    fn update_populates_added_edges_from_imports() {
        use anvil_kernel_types::ImportEdge;

        let mut g = SymbolGraph::new();
        // Pre-add the target symbol so the edge can resolve
        g.add_symbol(SymbolNode {
            id: 50,
            kind: SymbolKind::Function,
            name: "axios".to_string(),
            visibility: Visibility::Internal,
            file: "axios".to_string(),
            trust_level: TrustLevel::Unknown,
        })
        .unwrap();

        let syms = FileSymbols {
            file: "src/api.ts".to_string(),
            symbols: vec![SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "handler".to_string(),
                visibility: Visibility::Internal,
                file: "src/api.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            }],
            imports: vec![ImportEdge {
                from_file: "src/api.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
        };

        let delta = update_file(&mut g, syms);

        assert_eq!(delta.added_edges.len(), 1);
        assert_eq!(delta.added_edges[0].0, 1);
        assert_eq!(delta.added_edges[0].1, 50);
        assert_eq!(delta.added_edges[0].2, EdgeType::Imports);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn resolves_relative_import_to_file_path() {
        let mut g = SymbolGraph::new();
        // Target file at src/utils.ts
        g.add_symbol(SymbolNode {
            id: 50,
            kind: SymbolKind::Function,
            name: "helper".to_string(),
            visibility: Visibility::Internal,
            file: "src/utils.ts".to_string(),
            trust_level: TrustLevel::Unknown,
        })
        .unwrap();

        let syms = FileSymbols {
            file: "src/main.ts".to_string(),
            symbols: vec![SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "app".to_string(),
                visibility: Visibility::Internal,
                file: "src/main.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            }],
            imports: vec![ImportEdge {
                from_file: "src/main.ts".to_string(),
                to_source: "./utils".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
        };

        let delta = update_file(&mut g, syms);

        assert_eq!(delta.added_edges.len(), 1, "relative import should resolve");
        assert_eq!(delta.added_edges[0].1, 50);
    }

    #[test]
    fn side_effect_module_creates_synthetic_node_for_edges() {
        let mut g = SymbolGraph::new();
        g.add_symbol(SymbolNode {
            id: 50,
            kind: SymbolKind::Module,
            name: "polyfill".to_string(),
            visibility: Visibility::Internal,
            file: "polyfill".to_string(),
            trust_level: TrustLevel::Unknown,
        })
        .unwrap();

        // File with no symbols, only an import
        let syms = FileSymbols {
            file: "src/setup.ts".to_string(),
            symbols: vec![],
            imports: vec![ImportEdge {
                from_file: "src/setup.ts".to_string(),
                to_source: "polyfill".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
        };

        let delta = update_file(&mut g, syms);

        // Synthetic module node created + edge added
        assert!(
            !delta.added_symbols.is_empty(),
            "synthetic node should be added"
        );
        assert_eq!(delta.added_edges.len(), 1, "import edge should be created");
    }

    #[test]
    fn previously_imported_populated_for_existing_imports() {
        let mut g = SymbolGraph::new();

        // Initial parse: src/api.ts imports axios
        let syms = FileSymbols {
            file: "src/api.ts".to_string(),
            symbols: vec![SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "handler".to_string(),
                visibility: Visibility::Internal,
                file: "src/api.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            }],
            imports: vec![ImportEdge {
                from_file: "src/api.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
        };
        let delta1 = update_file(&mut g, syms);
        assert!(
            delta1.previously_imported.is_empty(),
            "first add has no prior imports"
        );

        // Re-parse: same file still imports axios
        let syms2 = FileSymbols {
            file: "src/api.ts".to_string(),
            symbols: vec![SymbolNode {
                id: 10,
                kind: SymbolKind::Function,
                name: "handler".to_string(),
                visibility: Visibility::Internal,
                file: "src/api.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            }],
            imports: vec![ImportEdge {
                from_file: "src/api.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
        };
        let delta2 = update_file(&mut g, syms2);

        assert!(
            delta2.previously_imported.contains("axios"),
            "re-added import should appear in previously_imported"
        );
    }

    #[test]
    fn ambiguous_relative_import_resolves_to_shortest_path() {
        let mut g = SymbolGraph::new();

        // Two files that both end with "src/utils.ts"
        g.add_symbol(SymbolNode {
            id: 50,
            kind: SymbolKind::Function,
            name: "short_helper".to_string(),
            visibility: Visibility::Internal,
            file: "src/utils.ts".to_string(),
            trust_level: TrustLevel::Unknown,
        })
        .unwrap();
        g.add_symbol(SymbolNode {
            id: 51,
            kind: SymbolKind::Function,
            name: "long_helper".to_string(),
            visibility: Visibility::Internal,
            file: "packages/app/src/utils.ts".to_string(),
            trust_level: TrustLevel::Unknown,
        })
        .unwrap();

        let syms = FileSymbols {
            file: "src/main.ts".to_string(),
            symbols: vec![SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "app".to_string(),
                visibility: Visibility::Internal,
                file: "src/main.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            }],
            imports: vec![ImportEdge {
                from_file: "src/main.ts".to_string(),
                to_source: "./utils".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
        };

        let delta = update_file(&mut g, syms);

        assert_eq!(delta.added_edges.len(), 1, "should resolve the import");
        assert_eq!(
            delta.added_edges[0].1, 50,
            "should resolve to shortest path (src/utils.ts, id=50)"
        );
    }

    #[test]
    fn re_resolve_imports_adds_missing_edges() {
        let mut g = SymbolGraph::new();

        // Simulate watch-mode initial scan: main.ts parsed before utils.ts.
        // main.ts imports ./utils but utils.ts isn't in the graph yet, so the
        // edge can't resolve during update_file.
        let main_syms = FileSymbols {
            file: "src/main.ts".to_string(),
            symbols: vec![SymbolNode {
                id: 1,
                kind: SymbolKind::Function,
                name: "app".to_string(),
                visibility: Visibility::Internal,
                file: "src/main.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            }],
            imports: vec![ImportEdge {
                from_file: "src/main.ts".to_string(),
                to_source: "./utils".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
        };
        let delta1 = update_file(&mut g, main_syms);
        assert!(
            delta1.added_edges.is_empty(),
            "edge should NOT resolve yet — utils.ts not in graph"
        );

        // Now parse utils.ts
        let util_syms = FileSymbols {
            file: "src/utils.ts".to_string(),
            symbols: vec![SymbolNode {
                id: 50,
                kind: SymbolKind::Function,
                name: "helper".to_string(),
                visibility: Visibility::Internal,
                file: "src/utils.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            }],
            imports: vec![],
            reexports: Vec::new(),
        };
        update_file(&mut g, util_syms);

        assert_eq!(g.edge_count(), 0, "no edges yet before re-resolve");

        // Re-resolve: should now create main->utils edge
        let all_imports = vec![ImportEdge {
            from_file: "src/main.ts".to_string(),
            to_source: "./utils".to_string(),
            line: 0,
        }];
        re_resolve_imports(&mut g, &all_imports);

        assert_eq!(
            g.edge_count(),
            1,
            "re_resolve_imports should add the missing edge"
        );
    }

    #[test]
    fn empty_delta_for_identical_count() {
        let mut g = SymbolGraph::new();
        update_file(
            &mut g,
            make_file_symbols("a.ts", vec![(1, "foo", SymbolKind::Function)]),
        );

        let delta = update_file(
            &mut g,
            make_file_symbols("a.ts", vec![(10, "foo", SymbolKind::Function)]),
        );

        assert_eq!(delta.removed_symbols.len(), 1);
        assert_eq!(delta.added_symbols.len(), 1);
        assert_eq!(g.node_count(), 1);
    }

    /// Regression test for the watch-mode "duplicate symbol id" cascade.
    ///
    /// Before the fix, `update_file` and `resolve_import` allocated
    /// synthetic external/module ids by calling
    /// `graph.node_weights().map(|s| s.id).max() + 1`, independent of the
    /// caller's per-file id allocator. When watch.rs picked the next file's
    /// `base_id` from its own `state.next_id` counter, that counter still
    /// pointed at the slot already taken by the synthetic external node,
    /// and every subsequent `add_symbol` returned `DuplicateSymbol`.
    ///
    /// After the fix, ids flow through `graph.next_id()`, so callers that
    /// take `(base + count).max(graph.next_id())` cannot land on a slot
    /// the graph already owns.
    #[test]
    fn external_synthetic_does_not_collide_with_next_files_base_id() {
        use anvil_kernel_types::ImportEdge;

        let mut g = SymbolGraph::new();

        // File a.ts: 3 symbols (ids 1,2,3) and a bare "axios" import that
        // creates a synthetic external node. Ids start at 1 to keep this
        // test focused on the duplicate-id cascade; id-0 sources are
        // covered separately by `id_zero_first_symbol_still_emits_import_edges`.
        let a = FileSymbols {
            file: "a.ts".to_string(),
            symbols: vec![
                SymbolNode {
                    id: 1,
                    kind: SymbolKind::Function,
                    name: "f0".to_string(),
                    visibility: Visibility::Internal,
                    file: "a.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                },
                SymbolNode {
                    id: 2,
                    kind: SymbolKind::Function,
                    name: "f1".to_string(),
                    visibility: Visibility::Internal,
                    file: "a.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                },
                SymbolNode {
                    id: 3,
                    kind: SymbolKind::Function,
                    name: "f2".to_string(),
                    visibility: Visibility::Internal,
                    file: "a.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                },
            ],
            imports: vec![ImportEdge {
                from_file: "a.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
        };
        let delta_a = update_file(&mut g, a);
        assert!(delta_a.errors.is_empty(), "first file inserts cleanly");

        // The synthetic axios node consumed id 4 (one past the last file
        // symbol id of 3), so the graph's allocator advances to 5.
        assert!(
            g.next_id() > 4,
            "graph.next_id() must reflect the synthetic external node, got {}",
            g.next_id()
        );

        // Mimic watch.rs's per-file allocator: take the high-water mark
        // from the graph rather than naively incrementing by symbol count.
        let base_id = g.next_id();
        let b = FileSymbols {
            file: "b.ts".to_string(),
            symbols: vec![
                SymbolNode {
                    id: base_id,
                    kind: SymbolKind::Function,
                    name: "g0".to_string(),
                    visibility: Visibility::Internal,
                    file: "b.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                },
                SymbolNode {
                    id: base_id + 1,
                    kind: SymbolKind::Function,
                    name: "g1".to_string(),
                    visibility: Visibility::Internal,
                    file: "b.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                },
            ],
            imports: vec![],
            reexports: Vec::new(),
        };
        let delta_b = update_file(&mut g, b);
        assert!(
            delta_b.errors.is_empty(),
            "second file must not collide with the synthetic external id, errors: {:?}",
            delta_b.errors
        );
        assert_eq!(delta_b.added_symbols.len(), 2);
    }

    /// Regression test for the symbol-id-zero sentinel.
    ///
    /// `update_file` previously used `from_id == 0` as a "no source node"
    /// marker, so the very first file in a fresh watch session — whose
    /// first symbol gets id 0 from the per-file allocator — silently
    /// dropped every import edge. Switching the marker to `Option<u64>`
    /// makes id 0 a valid source.
    #[test]
    fn id_zero_first_symbol_still_emits_import_edges() {
        use anvil_kernel_types::ImportEdge;

        let mut g = SymbolGraph::new();

        // Pre-add the import target so resolve_import returns Some.
        g.add_symbol(SymbolNode {
            id: 50,
            kind: SymbolKind::Module,
            name: "axios".to_string(),
            visibility: Visibility::Public,
            file: "axios".to_string(),
            trust_level: TrustLevel::External,
        })
        .unwrap();

        // First file in a fresh watch session: its first symbol takes
        // id 0 (state.next_id starts at 0). Before the fix this id-0
        // value was treated as "no source", so the axios import edge
        // was never added.
        let syms = FileSymbols {
            file: "src/main.ts".to_string(),
            symbols: vec![SymbolNode {
                id: 0,
                kind: SymbolKind::Function,
                name: "main".to_string(),
                visibility: Visibility::Internal,
                file: "src/main.ts".to_string(),
                trust_level: TrustLevel::Unknown,
            }],
            imports: vec![ImportEdge {
                from_file: "src/main.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
        };

        let delta = update_file(&mut g, syms);

        assert_eq!(
            delta.added_edges.len(),
            1,
            "id-0 source must still record import edges"
        );
        assert_eq!(delta.added_edges[0].0, 0);
        assert_eq!(delta.added_edges[0].1, 50);
    }
}
