use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use anvil_kernel_types::{
    CallSite, CalleeRef, EdgeType, FileSymbols, ImportEdge, ReexportEdge, SymbolIdentity,
    SymbolKind, SymbolNode, TrustLevel, Visibility,
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
    /// Privileged external module specifiers (`node:fs`, …) this file already
    /// reached through re-export edges before the update — transitively, via
    /// [`crate::trust::reexported_privileged_modules`] (GV2-031). The certify
    /// privileged-surface diff subtracts this baseline so a pre-existing
    /// privileged re-export does not re-trip on an unrelated edit, keeping the
    /// `newly_privileged_imports` channel monotone for re-exports exactly as it
    /// already is for direct imports (`previously_imported`).
    pub previously_reexported_privileged: HashSet<String>,
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
    /// `true` when this file contains a dynamic import whose target could not be
    /// statically resolved to a string-literal specifier — a computed
    /// `require(someVar)` or `import(`./${x}`)` (CIB-093 N1). Such a call can
    /// reach a privileged built-in (`require(pickModule())`) with **no** static
    /// import edge, so the trust pass never marks the file `Privileged` and the
    /// export-surface diff stays empty. The certify path treats this as a
    /// fail-closed gate: a `ContentModify` carrying it can never certify clean,
    /// degrading to `Partial(ExportSurfaceChange)` instead of silently CLEAN. A
    /// *literal* dynamic import (`require('fs')`, `import('fs')`) does **not**
    /// set this — it produces a real `ImportEdge` and flows through
    /// `is_privileged_import` like a static import. Defaults `false` so older
    /// serialized deltas stay honest.
    pub has_unresolved_dynamic_import: bool,
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
            previously_reexported_privileged: HashSet::new(),
            previously_public: HashSet::new(),
            previously_privileged: HashSet::new(),
            previously_boundary: HashSet::new(),
            has_unresolved_dynamic_import: false,
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
    /// Privileged external modules this file re-exported before the update
    /// (GV2-031 baseline; see [`GraphDelta::previously_reexported_privileged`]).
    previously_reexported_privileged: HashSet<String>,
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

        // GV2-031: the privileged modules this file already reached via
        // re-export edges (transitively), captured from the pre-update graph so
        // the certify diff can stay monotone for re-exports.
        let previously_reexported_privileged =
            crate::trust::reexported_privileged_modules(graph, file)
                .into_iter()
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
            previously_reexported_privileged,
            old_identity_set,
            removed_edges,
        }
    }
}

/// Lift a file's import and re-export edges from `from` into the graph,
/// returning the `(from, to, EdgeType)` tuples actually added.
///
/// Imports become `Imports` edges; re-exports become `Reexports` edges
/// (GV2-031). Both resolve their specifier the same way (`resolve_import`): a
/// bare specifier (`node:fs`) becomes a synthetic external module node, a
/// relative one resolves to a known file. The `Reexports` tag is what lets
/// `annotate_trust` and `certify::export_surface_diff` follow a re-export to a
/// privileged module — a capability reached via `export * from 'node:fs'` is no
/// longer invisible. Forward references (target file not yet parsed) are retried
/// by [`re_resolve_imports`]/[`re_resolve_reexports`].
fn lift_file_edges(
    graph: &mut SymbolGraph,
    from: u64,
    file: &str,
    imports: Vec<ImportEdge>,
    reexports: Vec<ReexportEdge>,
    known_files: &[String],
) -> Vec<(u64, u64, EdgeType)> {
    let mut added = Vec::new();
    let mut lift = |graph: &mut SymbolGraph, to_source: &str, edge_type: EdgeType| {
        if let Some(to) = resolve_import(to_source, file, known_files, graph) {
            let edge = anvil_kernel_types::SymbolEdge {
                from,
                to,
                edge_type,
            };
            if graph.add_edge(edge).is_ok() {
                added.push((from, to, edge_type));
            }
        }
    };
    for import in imports {
        lift(graph, &import.to_source, EdgeType::Imports);
    }
    for reexport in reexports {
        lift(graph, &reexport.to_source, EdgeType::Reexports);
    }
    added
}

/// Apply an incremental update to the graph for a single file.
///
/// 1. Remove all symbols and edges for the file
/// 2. Add new symbols from the re-parsed file
/// 3. Return the delta for downstream consumers (policy engine)
// Linear remove-then-re-add orchestrator: the steps are sequential and share the
// `added_ids`/`errors`/baseline locals, so splitting it would only scatter that
// shared state across helpers. One field thread-through (CIB-093 N1) tips it one
// line over the lint's budget.
#[allow(clippy::too_many_lines)]
pub fn update_file(graph: &mut SymbolGraph, new_symbols: FileSymbols) -> GraphDelta {
    let file = new_symbols.file.clone();
    // CIB-093 N1: carry the parser's unresolved-dynamic-import signal onto the
    // delta (captured before `new_symbols` is consumed) so certify fails closed
    // on a computed `require(...)`/`import(...)` no static edge can represent.
    let has_unresolved_dynamic_import = new_symbols.has_unresolved_dynamic_import;
    // Snapshot everything about the file's prior state that `remove_file` is
    // about to destroy (baselines, prior identities, incident edges).
    let before = PriorState::capture(graph, &file);
    let PriorState {
        previously_public,
        previously_privileged,
        previously_boundary,
        previously_imported,
        previously_reexported_privileged,
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

    // GV2-032: stamp the file's content-freshness key (CE-7). `remove_file`
    // above cleared any prior key, so a re-extraction that supplies `None`
    // (tail languages, reconstructed feeds) correctly leaves the file unkeyed.
    graph.set_file_hash(file.clone(), new_symbols.content_hash);

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
    } else if !new_symbols.imports.is_empty() || !new_symbols.reexports.is_empty() {
        let synthetic_id = graph.next_id();
        let synthetic = SymbolNode {
            id: synthetic_id,
            kind: SymbolKind::Module,
            name: file.clone(),
            visibility: Visibility::Internal,
            file: file.clone(),
            trust_level: TrustLevel::Unknown,
            span: None,
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

    let mut added_edges = match from_id {
        Some(from) => lift_file_edges(
            graph,
            from,
            &file,
            new_symbols.imports,
            new_symbols.reexports,
            &known_files,
        ),
        None => Vec::new(),
    };

    // GCALL-003: a top-level (module-scope) call needs the file's synthetic
    // `Module` node as its caller anchor. Ensure it here — where `added_ids` is in
    // scope — so the delta's `added_symbols` stays complete (the on-demand node is
    // otherwise invisible to that channel). Reuses the one the side-effect-import
    // branch above may already have created.
    if new_symbols.calls.iter().any(|c| c.from.module_scope) {
        let module_id = ensure_module_node(graph, &file);
        if !added_ids.contains(&module_id) {
            added_ids.push(module_id);
        }
    }

    // GCALL-003: lift this file's call sites into resident `Calls` edges. Same-file
    // and already-resident callees resolve now; a callee not yet resident is left
    // for the daemon's `re_resolve_calls` over its accumulator (forward
    // references), mirroring how imports re-resolve.
    added_edges.extend(lift_calls_tracked(
        graph,
        &file,
        &new_symbols.calls,
        &known_files,
    ));

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
        previously_reexported_privileged,
        previously_public,
        previously_privileged,
        previously_boundary,
        has_unresolved_dynamic_import,
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
        // External/virtual module nodes are keyed by `file == specifier`, so the
        // file index answers this in O(1) — not an O(total-graph-nodes)
        // `node_weights().find` scan on every cross-file callee/import lift
        // (council ADV-4: the scan made call-lift cost grow with graph size).
        if let Some(existing) = graph.symbols_in_file(specifier).first() {
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
            span: None,
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

    // `resolved_str` is the fully-normalised workspace-root-relative target path,
    // and both `from_file` and `known_files` are workspace-root-relative, so an
    // EXACT match is the only sound resolution (CIB-093 / N7). A path-suffix
    // fallback (`f.ends_with("/<candidate>")`) would let `./utils` from
    // `src/main.ts` silently rebind to `packages/app/src/utils.ts` when the true
    // `src/utils.ts` target is deleted, corrupting the reverse-impact index with a
    // same-named lookalike in a different directory. Leaving a relative import
    // unresolved is correct; rebinding it to a lookalike is not.
    for candidate in &candidates {
        // Normalise path separators for comparison (Windows-authored paths).
        let normalised = candidate.replace('\\', "/");

        if let Some(file_path) = known_files
            .iter()
            .find(|f| f.replace('\\', "/") == normalised)
        {
            // O(1) file-index lookup, not an O(total-graph-nodes) scan (ADV-4).
            return graph.symbols_in_file(file_path).first().map(|s| s.id);
        }
    }

    None
}

/// Re-resolve imports that could not be resolved during initial scan because
/// the target file had not been parsed yet.
pub fn re_resolve_imports(graph: &mut SymbolGraph, imports: &[ImportEdge]) {
    // No tracking closure → no per-call allocation for the hot-path callers
    // (`watch.rs`, embedded builds, benches) that don't need the added-edge list.
    re_resolve_imports_inner(graph, imports, |_, _, _| {});
}

/// Like [`re_resolve_imports`], but returns the symbol-graph `Imports` edges it
/// actually added (this generation's `(from, to, EdgeType)` ids).
///
/// GV2-011: re-resolution can re-bind a *surviving* import of a file other than
/// the one being updated — e.g. a specifier that could not resolve during the
/// initial scan (its target had not been parsed yet) binds once that exact target
/// appears (`resolve_import` matches the fully-normalised target path exactly;
/// CIB-093/N7). An incremental consumer that maintains derived state (the
/// dependency graph) cannot see those edge changes from the updated file's
/// `GraphDelta` alone, so it would silently diverge from a cold rebuild. Returning
/// the added edges lets the consumer refresh exactly the affected source files
/// instead of re-deriving the whole graph.
///
/// Only *additions* are reported: this function never removes edges (an edge
/// that ceased to resolve is dropped by `remove_file`/`update_file` removing the
/// incident symbols, not here).
pub fn re_resolve_imports_tracked(
    graph: &mut SymbolGraph,
    imports: &[ImportEdge],
) -> Vec<(u64, u64, EdgeType)> {
    let mut added = Vec::new();
    re_resolve_imports_inner(graph, imports, |from, to, ty| added.push((from, to, ty)));
    added
}

/// Shared re-resolution body. Invokes `on_add(from, to, edge_type)` for each
/// `Imports` edge it inserts, so the non-tracking entry point pays no allocation
/// and the tracking one collects into a `Vec`.
fn re_resolve_imports_inner(
    graph: &mut SymbolGraph,
    imports: &[ImportEdge],
    mut on_add: impl FnMut(u64, u64, EdgeType),
) {
    // CIB-093c: enumerate the distinct resident files via the O(files) per-file
    // index (`file_names`), not an O(symbols) `node_weights()` scan — this runs
    // under the cache Mutex on every re-resolution. Sibling
    // `re_resolve_calls_tracked` already uses `file_names()`; both yield the same
    // distinct-file set.
    let known_files: Vec<String> = graph
        .file_names()
        .map(ToString::to_string)
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
        if graph.add_edge(edge).is_ok() {
            on_add(from, to, EdgeType::Imports);
        }
    }
}

/// Re-resolve re-export edges whose target file had not been parsed when the
/// re-exporting file was first processed (GV2-031).
///
/// Mirrors [`re_resolve_imports`] for `EdgeType::Reexports`: a relative
/// re-export (`export * from './intermediary'`) to a forward-referenced file
/// resolves once that file lands, so a privileged capability reached through a
/// re-export chain (`a → b → node:fs`) becomes visible regardless of the order
/// the files were saved. Bare re-exports (`export * from 'node:fs'`) already
/// resolve eagerly in [`update_file`] via a synthetic external node, so this is
/// load-bearing only for the relative-intermediary chain. Like
/// [`re_resolve_imports`], it only ever *adds* edges (a re-export that ceased to
/// resolve is dropped by `remove_file`/`update_file` removing the incident
/// symbols, not here) and never feeds the `Imports`-only dependency graph.
///
/// There is deliberately no `re_resolve_reexports_tracked`: the tracked import
/// variant exists to refresh the dependency graph on re-bound edges, and
/// re-exports never feed that graph. A future re-export-driven feature that
/// needs the added-edge list should add the tracked variant then.
pub fn re_resolve_reexports(graph: &mut SymbolGraph, reexports: &[ReexportEdge]) {
    // CIB-093c: O(files) `file_names()` index, not an O(symbols) `node_weights()`
    // scan (runs under the cache Mutex). Same distinct-file set, lower constant.
    let known_files: Vec<String> = graph
        .file_names()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    for reexport in reexports {
        let from_id = graph
            .symbols_in_file(&reexport.from_file)
            .first()
            .map(|s| s.id);
        let Some(from) = from_id else { continue };

        let to_id = resolve_import(
            &reexport.to_source,
            &reexport.from_file,
            &known_files,
            graph,
        );
        let Some(to) = to_id else { continue };

        let already_exists = graph
            .outgoing_edges(from)
            .iter()
            .any(|e| e.to == to && e.edge_type == EdgeType::Reexports);
        if already_exists {
            continue;
        }

        let edge = anvil_kernel_types::SymbolEdge {
            from,
            to,
            edge_type: EdgeType::Reexports,
        };
        let _ = graph.add_edge(edge);
    }
}

// ============================================================================
// GCALL-003 — resident symbol-level call edges (ADR-086)
// ============================================================================

/// Cap on the number of same-`(file, name)` overload candidates a single
/// ambiguous callee fans out to (ADR-086 §1). A name resolving to more than this
/// is treated as unresolved — a pathological name cannot multiply edge count
/// without bound.
pub const MAX_OVERLOAD_FANOUT: usize = 8;

/// Symbol kinds a call site can target — the callable definitions. Excludes
/// surface markers (`Module`, `Export`) and type-only declarations
/// (`Interface`, `TypeAlias`, `Enum`) so a call never binds to a non-callable.
fn is_callable_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Function | SymbolKind::Class | SymbolKind::Method
    )
}

/// Outcome of lifting a batch of [`CallSite`]s: the `Calls` edges added, plus
/// the intended callee `(target_file, export_name)` of every call site that
/// produced **no** edge.
///
/// The `unresolved` channel is the CALL-1 partial signal (ADR-086 §1): an
/// unresolved call (a missing caller, a default-export callee, an over-cap
/// overload, an import that does not resolve to a resident file) leaves no edge,
/// so it is invisible to the [`crate::callers_of`] walk. Surfacing the intended
/// callee here lets the daemon set the `find_callers` egress `partial` marker for
/// exactly the targets whose caller set may be incomplete, instead of silently
/// under-reporting (the council's load-bearing finding).
#[derive(Debug, Default)]
pub struct CallLift {
    /// The `(from, to, Calls)` edges added this lift.
    pub added: Vec<(u64, u64, EdgeType)>,
    /// `(intended_target_file, callee_export_name)` of each call site that
    /// resolved to no edge. `intended_target_file` is `None` when even the callee's
    /// file is unknown (a callee imported from a not-yet-resident relative file, or
    /// a default-export binding) — a `None` flags by name alone, the conservative
    /// over-approximation a partial marker should make.
    pub unresolved: Vec<(Option<String>, String)>,
}

/// Lift a file's [`CallSite`]s into resident `EdgeType::Calls` edges
/// (caller symbol → callee symbol). Idempotent: a `(from, to, Calls)` edge
/// already present is not re-added, so re-resolution over the daemon accumulator
/// never duplicates — and an already-present edge is still counted **resolved**
/// (not pushed to `unresolved`).
///
/// `from_file` is the file the calls were extracted from (the caller's file).
/// Caller and callee are resolved at **symbol** granularity — not the file-level
/// `first()`-symbol shortcut the import resolver uses (ADR-086 §2). A caller or
/// callee that does not resolve to a resident symbol yields no edge (the call is
/// left for a later re-resolution when its target lands, or stays unresolved) and
/// is recorded in [`CallLift::unresolved`].
///
/// The file's named-caller index is built **once** here (not per call site), so
/// the lift is O(file-symbols + call-sites), not O(call-sites × file-symbols)
/// (council OPS-3).
fn lift_calls(
    graph: &mut SymbolGraph,
    from_file: &str,
    calls: &[CallSite],
    known_files: &[String],
) -> CallLift {
    let mut out = CallLift::default();
    // Hoist: resolve every named caller through one (kind, name, ordinal) → id
    // index built once for `from_file`. Module-scope callers bind to the file's
    // synthetic node, materialised lazily and reused across call sites.
    let caller_index = build_caller_index(graph, from_file);
    let mut module_id: Option<u64> = None;
    let (mut caller_miss, mut callee_unresolved, mut overcap_drops) = (0usize, 0usize, 0usize);

    for call in calls {
        let from = if call.from.module_scope {
            Some(*module_id.get_or_insert_with(|| ensure_module_node(graph, from_file)))
        } else {
            caller_index
                .get(&(call.from.kind, call.from.name.clone(), call.from.ordinal))
                .copied()
        };
        let resolution = resolve_callee_targets(graph, from_file, &call.callee, known_files);

        // Resolved iff the caller is resident AND at least one callee target
        // resolved — independent of whether a *new* edge was added (an already
        // resident edge is still resolved, the idempotent re-resolution case).
        if let Some(from) = from.filter(|_| !resolution.targets.is_empty()) {
            // Snapshot the caller's existing `Calls` targets once per call site
            // (not once per candidate) so fan-out dedup is O(out-degree + fan-out).
            let mut existing: std::collections::HashSet<u64> = graph
                .outgoing_edges(from)
                .iter()
                .filter(|e| e.edge_type == EdgeType::Calls)
                .map(|e| e.to)
                .collect();
            for to in resolution.targets {
                if !existing.insert(to) {
                    continue;
                }
                let edge = anvil_kernel_types::SymbolEdge {
                    from,
                    to,
                    edge_type: EdgeType::Calls,
                };
                if graph.add_edge(edge).is_ok() {
                    out.added.push((from, to, EdgeType::Calls));
                }
            }
        } else {
            // No edge: record the intended callee so the egress `partial` marker
            // can fire for this target. `intended_file` is the callee's resolved
            // module file when known (a same-file callee carries the caller's own
            // file), or `None` when the file is unknown (forward-referenced or
            // default-export callee) — then the egress matches by name alone.
            out.unresolved
                .push((resolution.intended_file, call.callee.name.clone()));
            if from.is_none() {
                caller_miss += 1;
            } else if resolution.overcap {
                overcap_drops += 1;
            } else {
                callee_unresolved += 1;
            }
        }
    }

    if caller_miss + callee_unresolved + overcap_drops > 0 {
        // Counts only (no identities/paths) — PV-10 telemetry posture. Lets an
        // operator see what fraction of call sites are dropping without a profiler.
        tracing::debug!(
            target: "anvil_graph_cache::call_graph",
            added = out.added.len(),
            caller_miss,
            callee_unresolved,
            overcap = overcap_drops,
            "call lift: unresolved call sites dropped"
        );
    }
    out
}

/// Build `from_file`'s named-caller `(kind, name, ordinal) → node id` index using
/// the [`SymbolIdentity::for_file_symbols`] ordinal scheme the extractor used.
fn build_caller_index(
    graph: &SymbolGraph,
    file: &str,
) -> std::collections::HashMap<(SymbolKind, String, u32), u64> {
    let symbols = graph.symbols_in_file(file);
    let identities = SymbolIdentity::for_file_symbols(&symbols);
    identities
        .iter()
        .zip(symbols.iter())
        .map(|(id, node)| ((id.kind, id.name.clone(), id.ordinal), node.id))
        .collect()
}

/// Thin wrapper returning only the edges added — the `update_file` initial-lift
/// path, which does not maintain the daemon's unresolved-callee set (that is
/// rebuilt by [`re_resolve_calls_tracked`] over the affected neighbourhood).
fn lift_calls_tracked(
    graph: &mut SymbolGraph,
    from_file: &str,
    calls: &[CallSite],
    known_files: &[String],
) -> Vec<(u64, u64, EdgeType)> {
    lift_calls(graph, from_file, calls, known_files).added
}

/// Re-resolve call edges over a daemon accumulator of `(caller_file, CallSite)`
/// pairs (ADR-086 §2 forward-reference + callee-resave handling), returning the
/// edges added **and** the unresolved intended-callee keys (CALL-1 partial
/// signal). Idempotent via the dedup in [`lift_calls`].
///
/// Calls are grouped by caller file so each file's caller index is built once
/// (council OPS-3), and the output is deterministic (`BTreeMap` file order).
pub fn re_resolve_calls_tracked(
    graph: &mut SymbolGraph,
    calls: &[(String, CallSite)],
) -> CallResolution {
    let known_files: Vec<String> = graph
        .file_names()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let mut by_file: std::collections::BTreeMap<&str, Vec<CallSite>> =
        std::collections::BTreeMap::new();
    for (from_file, call) in calls {
        by_file
            .entry(from_file.as_str())
            .or_default()
            .push(call.clone());
    }

    let mut out = CallResolution::default();
    for (from_file, file_calls) in by_file {
        let lift = lift_calls(graph, from_file, &file_calls, &known_files);
        out.added.extend(lift.added);
        for (target_file, name) in lift.unresolved {
            out.unresolved
                .push((from_file.to_string(), target_file, name));
        }
    }
    out
}

/// The result of [`re_resolve_calls_tracked`].
#[derive(Debug, Default)]
pub struct CallResolution {
    /// `(from, to, Calls)` edges added this generation.
    pub added: Vec<(u64, u64, EdgeType)>,
    /// `(caller_file, intended_target_file, callee_export_name)` of each call
    /// site that produced no edge — the daemon's CALL-1 partial signal, keyed by
    /// caller file so it can be maintained incrementally (retain-by-`from_file`).
    /// `intended_target_file` is `None` when the callee's file is unknown.
    pub unresolved: Vec<(String, Option<String>, String)>,
}

/// Re-resolve call edges over a daemon accumulator (ADR-086 §2). Returns the
/// edges added this generation; callers needing the unresolved signal use
/// [`re_resolve_calls_tracked`].
pub fn re_resolve_calls(
    graph: &mut SymbolGraph,
    calls: &[(String, CallSite)],
) -> Vec<(u64, u64, EdgeType)> {
    re_resolve_calls_tracked(graph, calls).added
}

/// Find the file's synthetic `Module` node (the module-scope caller anchor), or
/// create one. Reuses the node `update_file` materialises for side-effect-only
/// imports so a file never carries two.
fn ensure_module_node(graph: &mut SymbolGraph, file: &str) -> u64 {
    if let Some(id) = graph
        .symbols_in_file(file)
        .iter()
        .find(|s| s.kind == SymbolKind::Module && s.name == file)
        .map(|s| s.id)
    {
        return id;
    }
    let id = graph.next_id();
    let node = SymbolNode {
        id,
        kind: SymbolKind::Module,
        name: file.to_string(),
        visibility: Visibility::Internal,
        file: file.to_string(),
        trust_level: TrustLevel::Unknown,
        span: None,
    };
    let _ = graph.add_symbol(node);
    id
}

/// A callee resolution: the resident target node id(s), the **intended** callee
/// file (for the unresolved-callee partial signal), and whether the callee was
/// dropped by the overload-fan-out cap.
struct CalleeResolution {
    /// Resolved resident callee nodes (empty ⇒ unresolved).
    targets: Vec<u64>,
    /// The file the callee was expected in, when determinable — the resolved
    /// module file for an import, or the caller's own file for a same-file
    /// callee. `None` when even the intended file is unknown (a default-export
    /// callee, or a specifier that resolves to no resident file).
    intended_file: Option<String>,
    /// True when the callee name resolved to more than [`MAX_OVERLOAD_FANOUT`]
    /// overloads and was dropped (distinguished from "no such symbol" for
    /// telemetry only).
    overcap: bool,
}

/// Resolve a [`CalleeRef`] to the resident callee node id(s) — best-effort and
/// static (ADR-086 §1). Same-file callees resolve in `from_file`; imported
/// callees resolve the module specifier (reusing [`resolve_import`]) then the
/// export name in the target file. Overloads fan out, capped at
/// [`MAX_OVERLOAD_FANOUT`]. A default import (`name == "default"`), a callee that
/// resolves to no resident symbol, or an over-cap fan-out yields no edge (left
/// `Unresolved` — barrel/re-export follow is a deferred refinement) but still
/// reports its `intended_file` where known so the egress `partial` marker fires.
fn resolve_callee_targets(
    graph: &mut SymbolGraph,
    from_file: &str,
    callee: &CalleeRef,
    known_files: &[String],
) -> CalleeResolution {
    match callee.via_import.as_deref() {
        None => {
            let (targets, overcap) = symbols_named(graph, from_file, &callee.name);
            CalleeResolution {
                targets,
                intended_file: Some(from_file.to_string()),
                overcap,
            }
        }
        // A default import is left Unresolved in v1 (ADR-086) — no edge. Resolve
        // the specifier to the target *file* (not the symbol) so the unresolved
        // record is precise: it matches only a `default`-named target in that
        // file, never every `default` across the workspace by name alone. The
        // file stays `None` only when the specifier resolves to no resident file.
        Some(specifier) if callee.name == "default" => {
            let intended_file = resolve_import(specifier, from_file, known_files, graph)
                .and_then(|node| graph.get_symbol(node).map(|s| s.file.clone()));
            CalleeResolution {
                targets: Vec::new(),
                intended_file,
                overcap: false,
            }
        }
        Some(specifier) => {
            let Some(target_node) = resolve_import(specifier, from_file, known_files, graph) else {
                return CalleeResolution {
                    targets: Vec::new(),
                    intended_file: None,
                    overcap: false,
                };
            };
            let Some(target_file) = graph.get_symbol(target_node).map(|s| s.file.clone()) else {
                return CalleeResolution {
                    targets: Vec::new(),
                    intended_file: None,
                    overcap: false,
                };
            };
            let (targets, overcap) = symbols_named(graph, &target_file, &callee.name);
            CalleeResolution {
                targets,
                intended_file: Some(target_file),
                overcap,
            }
        }
    }
}

/// The resident callable symbols in `file` named `name`, fan-out capped and
/// deterministically ordered by node id. Returns `(ids, overcap)`: over the
/// [`MAX_OVERLOAD_FANOUT`] cap the ids are empty and `overcap` is true.
fn symbols_named(graph: &SymbolGraph, file: &str, name: &str) -> (Vec<u64>, bool) {
    let mut ids: Vec<u64> = graph
        .symbols_in_file(file)
        .iter()
        .filter(|s| s.name == name && is_callable_kind(s.kind))
        .map(|s| s.id)
        .collect();
    ids.sort_unstable();
    if ids.len() > MAX_OVERLOAD_FANOUT {
        return (Vec::new(), true);
    }
    (ids, false)
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
    use anvil_kernel_types::{
        ImportEdge, LocalSymbolRef, ReexportEdge, SymbolKind, SymbolNode, TrustLevel, Visibility,
    };

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
                    span: None,
                })
                .collect(),
            imports: Vec::new(),
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
        }
    }

    fn reexport(from_file: &str, exported_name: &str, to_source: &str) -> ReexportEdge {
        ReexportEdge {
            from_file: from_file.to_string(),
            exported_name: exported_name.to_string(),
            to_source: to_source.to_string(),
            line: 0,
        }
    }

    /// GV2-031: `update_file` lifts `FileSymbols.reexports` into `Reexports`
    /// CIB-093 N1: `update_file` carries the parser's unresolved-dynamic-import
    /// signal through onto the delta, where `certify` reads it to fail closed.
    #[test]
    fn update_file_propagates_unresolved_dynamic_import_signal() {
        let mut g = SymbolGraph::new();
        let mut syms = make_file_symbols("a.ts", vec![(1, "f", SymbolKind::Function)]);
        syms.has_unresolved_dynamic_import = true;
        let delta = update_file(&mut g, syms);
        assert!(
            delta.has_unresolved_dynamic_import,
            "the delta must carry the parser's unresolved-dynamic-import signal"
        );

        // And the converse: a file with no dynamic import does not set it.
        let clean = make_file_symbols("b.ts", vec![(2, "g", SymbolKind::Function)]);
        let clean_delta = update_file(&mut g, clean);
        assert!(!clean_delta.has_unresolved_dynamic_import);
    }

    /// GV2-032: `apply_delta` stamps symbol spans and per-file content hashes on
    /// the resident graph for TS, Rust, and Python producer fixtures.
    #[test]
    fn span_population() {
        use anvil_kernel_types::ByteRange;

        let cases: [(&str, u64, &str, SymbolKind, ByteRange, u64); 3] = [
            (
                "src/a.ts",
                1,
                "alpha",
                SymbolKind::Function,
                ByteRange { start: 10, end: 40 },
                0xA1,
            ),
            (
                "src/lib.rs",
                2,
                "beta",
                SymbolKind::Function,
                ByteRange { start: 5, end: 55 },
                0xB2,
            ),
            (
                "pkg/mod.py",
                3,
                "gamma",
                SymbolKind::Function,
                ByteRange { start: 0, end: 20 },
                0xC3,
            ),
        ];

        let mut g = SymbolGraph::new();
        for (file, id, name, kind, span, hash) in cases {
            let syms = FileSymbols {
                file: file.to_string(),
                symbols: vec![SymbolNode {
                    id,
                    kind,
                    name: name.to_string(),
                    visibility: Visibility::Internal,
                    file: file.to_string(),
                    trust_level: TrustLevel::Unknown,
                    span: Some(span),
                }],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: Some(hash),
            };
            update_file(&mut g, syms);
            let node = g.get_symbol(id).expect("symbol resident");
            assert_eq!(node.span, Some(span), "{file}: span must land on the node");
            assert_eq!(g.file_hash(file), Some(hash), "{file}: content hash stamped");
        }
    }

    #[test]
    fn update_file_records_and_clears_content_hash_gv2_032() {
        let mut g = SymbolGraph::new();

        // update_file stamps the file's content-freshness key (CE-7).
        let mut syms = make_file_symbols("a.ts", vec![(1, "f", SymbolKind::Function)]);
        syms.content_hash = Some(0xABCD);
        update_file(&mut g, syms);
        assert_eq!(g.file_hash("a.ts"), Some(0xABCD));

        // A re-extraction that supplies no hash clears the stale key — the graph
        // never reports a freshness key it cannot stand behind.
        let cleared = make_file_symbols("a.ts", vec![(1, "f", SymbolKind::Function)]);
        update_file(&mut g, cleared);
        assert_eq!(g.file_hash("a.ts"), None);

        // remove_file clears it too.
        let mut other = make_file_symbols("b.ts", vec![(2, "g", SymbolKind::Function)]);
        other.content_hash = Some(0x1234);
        update_file(&mut g, other);
        assert_eq!(g.file_hash("b.ts"), Some(0x1234));
        g.remove_file("b.ts");
        assert_eq!(g.file_hash("b.ts"), None);
    }

    /// edges. A bare specifier (`node:fs`) resolves to a synthetic external
    /// module node, mirroring an import.
    #[test]
    fn update_file_lifts_reexports_into_reexport_edges() {
        let mut g = SymbolGraph::new();
        let mut syms = make_file_symbols("barrel.ts", vec![(1, "api", SymbolKind::Function)]);
        syms.reexports = vec![reexport("barrel.ts", "*", "node:fs")];

        let delta = update_file(&mut g, syms);

        let reexport_edges: Vec<_> = delta
            .added_edges
            .iter()
            .filter(|(_, _, ty)| *ty == EdgeType::Reexports)
            .collect();
        assert_eq!(
            reexport_edges.len(),
            1,
            "a re-export must lift to exactly one Reexports edge, got {:?}",
            delta.added_edges
        );
        let (_, to, _) = reexport_edges[0];
        let target = g.get_symbol(*to).expect("re-export target node exists");
        assert_eq!(target.file, "node:fs");
        assert_eq!(target.kind, SymbolKind::Module);
        assert_eq!(target.trust_level, TrustLevel::External);
    }

    /// GV2-031: a re-export-only module (no symbols, no imports) still gets a
    /// synthetic source node so the `Reexports` edge is recorded.
    #[test]
    fn update_file_lifts_reexports_for_symbolless_module() {
        let mut g = SymbolGraph::new();
        let mut syms = make_file_symbols("barrel.ts", vec![]);
        syms.reexports = vec![reexport("barrel.ts", "*", "node:fs")];

        let delta = update_file(&mut g, syms);

        assert!(
            delta
                .added_edges
                .iter()
                .any(|(_, _, ty)| *ty == EdgeType::Reexports),
            "a symbol-less re-export module must still record a Reexports edge"
        );
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
            span: None,
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
                span: None,
            }],
            imports: vec![ImportEdge {
                from_file: "src/api.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
        };

        let delta = update_file(&mut g, syms);

        assert_eq!(delta.added_edges.len(), 1);
        assert_eq!(delta.added_edges[0].0, 1);
        assert_eq!(delta.added_edges[0].1, 50);
        assert_eq!(delta.added_edges[0].2, EdgeType::Imports);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn reresolve_imports_file_enumeration_unchanged_by_index_swap() {
        // CIB-093c: `re_resolve_imports` enumerates known files via `file_names()`
        // (O(files)) rather than a `node_weights()` scan. The resolved set must be
        // identical: a forward-referenced relative import still binds once its
        // target file lands, and the known-file set equals the distinct files of
        // the resident symbols.
        let mut g = SymbolGraph::new();
        // The importer lands first; its './util' target does not exist yet.
        update_file(
            &mut g,
            FileSymbols {
                file: "src/api.ts".to_string(),
                symbols: vec![SymbolNode {
                    id: 1,
                    kind: SymbolKind::Function,
                    name: "handler".to_string(),
                    visibility: Visibility::Internal,
                    file: "src/api.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                    span: None,
                }],
                imports: vec![ImportEdge {
                    from_file: "src/api.ts".to_string(),
                    to_source: "./util".to_string(),
                    line: 0,
                }],
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );
        // No edge yet — the target file was not resident at parse time.
        assert_eq!(g.edge_count(), 0);

        // The target lands. The known-file set the swapped enumeration builds must
        // equal the distinct resident files, so the forward reference resolves.
        update_file(
            &mut g,
            make_file_symbols("src/util.ts", vec![(2, "u", SymbolKind::Function)]),
        );
        re_resolve_imports(
            &mut g,
            &[ImportEdge {
                from_file: "src/api.ts".to_string(),
                to_source: "./util".to_string(),
                line: 0,
            }],
        );

        // The enumeration via `file_names()` agrees with the `node_weights()` set.
        let via_index: BTreeSet<String> = g.file_names().map(ToString::to_string).collect();
        let via_scan: BTreeSet<String> = g.inner().node_weights().map(|s| s.file.clone()).collect();
        assert_eq!(
            via_index, via_scan,
            "file_names() must equal the node_weights() file set"
        );

        // And the forward reference is now resolved (the optimisation is behaviour-
        // preserving).
        assert_eq!(
            g.edge_count(),
            1,
            "forward-referenced import resolves after target lands"
        );
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
            span: None,
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
                span: None,
            }],
            imports: vec![ImportEdge {
                from_file: "src/main.ts".to_string(),
                to_source: "./utils".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
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
            span: None,
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
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
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
                span: None,
            }],
            imports: vec![ImportEdge {
                from_file: "src/api.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
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
                span: None,
            }],
            imports: vec![ImportEdge {
                from_file: "src/api.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
        };
        let delta2 = update_file(&mut g, syms2);

        assert!(
            delta2.previously_imported.contains("axios"),
            "re-added import should appear in previously_imported"
        );
    }

    #[test]
    fn relative_import_resolves_to_the_exact_target_not_a_lookalike() {
        let mut g = SymbolGraph::new();

        // The exact target of `./utils` from `src/main.ts` is `src/utils.ts`, plus
        // a same-named lookalike in another package directory.
        g.add_symbol(SymbolNode {
            id: 50,
            kind: SymbolKind::Function,
            name: "short_helper".to_string(),
            visibility: Visibility::Internal,
            file: "src/utils.ts".to_string(),
            trust_level: TrustLevel::Unknown,
            span: None,
        })
        .unwrap();
        g.add_symbol(SymbolNode {
            id: 51,
            kind: SymbolKind::Function,
            name: "long_helper".to_string(),
            visibility: Visibility::Internal,
            file: "packages/app/src/utils.ts".to_string(),
            trust_level: TrustLevel::Unknown,
            span: None,
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
                span: None,
            }],
            imports: vec![ImportEdge {
                from_file: "src/main.ts".to_string(),
                to_source: "./utils".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
        };

        let delta = update_file(&mut g, syms);

        assert_eq!(delta.added_edges.len(), 1, "should resolve the import");
        assert_eq!(
            delta.added_edges[0].1, 50,
            "should resolve to the exact target src/utils.ts (id=50), never the \
             cross-directory lookalike"
        );
    }

    #[test]
    fn relative_import_does_not_rebind_to_a_lookalike_when_exact_target_absent() {
        // CIB-093 / N7: `./utils` from `src/main.ts` resolves ONLY to `src/utils.ts`.
        // When that exact target is absent (e.g. deleted), the import must be left
        // UNRESOLVED — it must not silently rebind to a same-named file in another
        // directory (`packages/app/src/utils.ts`), which would corrupt the
        // reverse-impact index with a wrong dependency edge.
        let mut g = SymbolGraph::new();
        g.add_symbol(SymbolNode {
            id: 51,
            kind: SymbolKind::Function,
            name: "long_helper".to_string(),
            visibility: Visibility::Internal,
            file: "packages/app/src/utils.ts".to_string(),
            trust_level: TrustLevel::Unknown,
            span: None,
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
                span: None,
            }],
            imports: vec![ImportEdge {
                from_file: "src/main.ts".to_string(),
                to_source: "./utils".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
        };

        let delta = update_file(&mut g, syms);

        assert!(
            delta.added_edges.is_empty(),
            "an absent exact target must leave the import unresolved, not rebind \
             to the cross-directory lookalike packages/app/src/utils.ts"
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
                span: None,
            }],
            imports: vec![ImportEdge {
                from_file: "src/main.ts".to_string(),
                to_source: "./utils".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
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
                span: None,
            }],
            imports: vec![],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
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
                    span: None,
                },
                SymbolNode {
                    id: 2,
                    kind: SymbolKind::Function,
                    name: "f1".to_string(),
                    visibility: Visibility::Internal,
                    file: "a.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                    span: None,
                },
                SymbolNode {
                    id: 3,
                    kind: SymbolKind::Function,
                    name: "f2".to_string(),
                    visibility: Visibility::Internal,
                    file: "a.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                    span: None,
                },
            ],
            imports: vec![ImportEdge {
                from_file: "a.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
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
                    span: None,
                },
                SymbolNode {
                    id: base_id + 1,
                    kind: SymbolKind::Function,
                    name: "g1".to_string(),
                    visibility: Visibility::Internal,
                    file: "b.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                    span: None,
                },
            ],
            imports: vec![],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
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
            span: None,
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
                span: None,
            }],
            imports: vec![ImportEdge {
                from_file: "src/main.ts".to_string(),
                to_source: "axios".to_string(),
                line: 0,
            }],
            reexports: Vec::new(),
            calls: Vec::new(),
            calls_partial: false,
            has_unresolved_dynamic_import: false,
            content_hash: None,
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

    // --- GCALL-003 resident call-edge lift ---

    fn fn_node(id: u64, name: &str, file: &str) -> SymbolNode {
        SymbolNode {
            id,
            kind: SymbolKind::Function,
            name: name.to_string(),
            visibility: Visibility::Internal,
            file: file.to_string(),
            trust_level: TrustLevel::Unknown,
            span: None,
        }
    }

    fn from_fn(name: &str, ordinal: u32) -> LocalSymbolRef {
        LocalSymbolRef {
            kind: SymbolKind::Function,
            name: name.to_string(),
            ordinal,
            module_scope: false,
        }
    }

    fn calls_edges(graph: &SymbolGraph) -> Vec<(u64, u64)> {
        let mut out = Vec::new();
        for node in graph.inner().node_weights() {
            for e in graph.outgoing_edges(node.id) {
                if e.edge_type == EdgeType::Calls {
                    out.push((e.from, e.to));
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    #[test]
    fn lifts_same_file_call_edge() {
        let mut g = SymbolGraph::new();
        let mut fs = make_file_symbols("a.ts", vec![]);
        fs.symbols = vec![fn_node(0, "a", "a.ts"), fn_node(1, "b", "a.ts")];
        fs.calls = vec![CallSite {
            from: from_fn("b", 0),
            callee: CalleeRef {
                name: "a".into(),
                via_import: None,
            },
            line: 1,
        }];
        let delta = update_file(&mut g, fs);
        // b (id 1) → a (id 0)
        assert!(calls_edges(&g).contains(&(1, 0)));
        assert!(
            delta
                .added_edges
                .iter()
                .any(|(f, t, ty)| *f == 1 && *t == 0 && *ty == EdgeType::Calls)
        );
    }

    #[test]
    fn lifts_imported_call_edge_across_files() {
        let mut g = SymbolGraph::new();
        // Target file first so the callee is resident when the caller lands.
        let mut target = make_file_symbols("util.ts", vec![]);
        target.symbols = vec![fn_node(10, "helper", "util.ts")];
        update_file(&mut g, target);

        let mut caller = make_file_symbols("main.ts", vec![]);
        caller.symbols = vec![fn_node(20, "run", "main.ts")];
        caller.imports = vec![ImportEdge {
            from_file: "main.ts".into(),
            to_source: "./util".into(),
            line: 1,
        }];
        caller.calls = vec![CallSite {
            from: from_fn("run", 0),
            callee: CalleeRef {
                name: "helper".into(),
                via_import: Some("./util".into()),
            },
            line: 2,
        }];
        update_file(&mut g, caller);
        // run (20) → helper (10)
        assert!(calls_edges(&g).contains(&(20, 10)));
    }

    #[test]
    fn forward_reference_call_resolves_via_re_resolve() {
        let mut g = SymbolGraph::new();
        // Caller lands BEFORE the callee file → the import/call cannot resolve yet.
        let mut caller = make_file_symbols("main.ts", vec![]);
        caller.symbols = vec![fn_node(20, "run", "main.ts")];
        caller.imports = vec![ImportEdge {
            from_file: "main.ts".into(),
            to_source: "./util".into(),
            line: 1,
        }];
        let call = CallSite {
            from: from_fn("run", 0),
            callee: CalleeRef {
                name: "helper".into(),
                via_import: Some("./util".into()),
            },
            line: 2,
        };
        caller.calls = vec![call.clone()];
        update_file(&mut g, caller);
        assert!(calls_edges(&g).is_empty(), "callee not resident yet");

        // Callee lands; re_resolve over the accumulator wires the forward call.
        let mut target = make_file_symbols("util.ts", vec![]);
        target.symbols = vec![fn_node(10, "helper", "util.ts")];
        update_file(&mut g, target);
        re_resolve_calls(&mut g, &[("main.ts".to_string(), call)]);
        assert!(calls_edges(&g).contains(&(20, 10)));
    }

    #[test]
    fn re_resolve_is_idempotent() {
        let mut g = SymbolGraph::new();
        let mut fs = make_file_symbols("a.ts", vec![]);
        fs.symbols = vec![fn_node(0, "a", "a.ts"), fn_node(1, "b", "a.ts")];
        let call = CallSite {
            from: from_fn("b", 0),
            callee: CalleeRef {
                name: "a".into(),
                via_import: None,
            },
            line: 1,
        };
        fs.calls = vec![call.clone()];
        update_file(&mut g, fs);
        let before = calls_edges(&g);
        // Re-running resolution must not duplicate the edge.
        let added = re_resolve_calls(&mut g, &[("a.ts".to_string(), call)]);
        assert!(added.is_empty(), "no new edge on a second resolution");
        assert_eq!(calls_edges(&g), before);
    }

    #[test]
    fn overload_fan_out_targets_all_then_caps() {
        let mut g = SymbolGraph::new();
        let mut fs = make_file_symbols("a.ts", vec![]);
        // Two overloads of `t` + one caller.
        fs.symbols = vec![
            fn_node(0, "t", "a.ts"),
            fn_node(1, "t", "a.ts"),
            fn_node(2, "caller", "a.ts"),
        ];
        fs.calls = vec![CallSite {
            from: from_fn("caller", 0),
            callee: CalleeRef {
                name: "t".into(),
                via_import: None,
            },
            line: 1,
        }];
        update_file(&mut g, fs);
        // caller (2) fans out to both overloads (0 and 1).
        let edges = calls_edges(&g);
        assert!(edges.contains(&(2, 0)) && edges.contains(&(2, 1)));
    }

    #[test]
    fn over_cap_overload_fan_out_yields_no_edge() {
        let mut g = SymbolGraph::new();
        let mut fs = make_file_symbols("a.ts", vec![]);
        // MAX_OVERLOAD_FANOUT + 1 overloads of `t` + one caller — over the cap, so
        // the callee is treated as Unresolved (no edge), never a partial fan-out.
        let mut symbols: Vec<SymbolNode> = (0..=MAX_OVERLOAD_FANOUT as u64)
            .map(|i| fn_node(i, "t", "a.ts"))
            .collect();
        let caller_id = MAX_OVERLOAD_FANOUT as u64 + 1;
        symbols.push(fn_node(caller_id, "caller", "a.ts"));
        fs.symbols = symbols;
        fs.calls = vec![CallSite {
            from: from_fn("caller", 0),
            callee: CalleeRef {
                name: "t".into(),
                via_import: None,
            },
            line: 1,
        }];
        update_file(&mut g, fs);
        assert!(
            calls_edges(&g).is_empty(),
            "an over-cap fan-out produces no Calls edge"
        );
    }

    #[test]
    fn module_scope_caller_anchors_to_synthetic_module_node() {
        let mut g = SymbolGraph::new();
        let mut fs = make_file_symbols("a.ts", vec![]);
        fs.symbols = vec![fn_node(0, "a", "a.ts")];
        // A top-level (module-scope) call to `a`.
        fs.calls = vec![CallSite {
            from: LocalSymbolRef {
                kind: SymbolKind::Module,
                name: String::new(),
                ordinal: 0,
                module_scope: true,
            },
            callee: CalleeRef {
                name: "a".into(),
                via_import: None,
            },
            line: 1,
        }];
        update_file(&mut g, fs);
        // A synthetic Module node anchors the edge → a (id 0).
        let module_id = g
            .symbols_in_file("a.ts")
            .iter()
            .find(|s| s.kind == SymbolKind::Module && s.name == "a.ts")
            .map(|s| s.id)
            .expect("synthetic module node created");
        assert!(calls_edges(&g).contains(&(module_id, 0)));
    }

    #[test]
    fn default_import_callee_is_unresolved() {
        let mut g = SymbolGraph::new();
        let mut target = make_file_symbols("util.ts", vec![]);
        target.symbols = vec![fn_node(10, "thing", "util.ts")];
        update_file(&mut g, target);

        let mut caller = make_file_symbols("main.ts", vec![]);
        caller.symbols = vec![fn_node(20, "run", "main.ts")];
        caller.imports = vec![ImportEdge {
            from_file: "main.ts".into(),
            to_source: "./util".into(),
            line: 1,
        }];
        caller.calls = vec![CallSite {
            from: from_fn("run", 0),
            callee: CalleeRef {
                name: "default".into(),
                via_import: Some("./util".into()),
            },
            line: 2,
        }];
        update_file(&mut g, caller);
        // A default-import callee produces no edge in v1 (Unresolved).
        assert!(calls_edges(&g).is_empty());
    }

    /// CALL-1: `re_resolve_calls_tracked` reports the intended callee of a call
    /// site that resolves to no edge — here a same-file call to a name with no
    /// resident definition — keyed `(caller_file, intended_file, name)`. A
    /// resolved call in the same batch is **not** reported.
    #[test]
    fn tracked_reports_unresolved_callee_only() {
        let mut g = SymbolGraph::new();
        update_file(
            &mut g,
            FileSymbols {
                file: "a.ts".into(),
                symbols: vec![fn_node(1, "caller", "a.ts"), fn_node(2, "real", "a.ts")],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: Vec::new(),
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );
        let batch = vec![
            (
                "a.ts".to_string(),
                CallSite {
                    from: from_fn("caller", 0),
                    callee: CalleeRef {
                        name: "real".into(),
                        via_import: None,
                    },
                    line: 1,
                },
            ),
            (
                "a.ts".to_string(),
                CallSite {
                    from: from_fn("caller", 0),
                    callee: CalleeRef {
                        name: "missing".into(),
                        via_import: None,
                    },
                    line: 2,
                },
            ),
        ];
        let res = re_resolve_calls_tracked(&mut g, &batch);
        // The resolved `real` call produced an edge; the `missing` call did not.
        assert_eq!(res.added.len(), 1, "exactly the resolved call adds an edge");
        assert!(
            res.unresolved
                .iter()
                .any(|(ff, tf, n)| ff == "a.ts" && tf.as_deref() == Some("a.ts") && n == "missing"),
            "the unresolved callee `missing` is reported: {:?}",
            res.unresolved
        );
        assert!(
            !res.unresolved.iter().any(|(_, _, n)| n == "real"),
            "a resolved callee is not reported as unresolved"
        );
    }

    /// An already-resident `Calls` edge is **resolved**, not unresolved — the
    /// idempotent re-resolution case must not falsely flag `partial`.
    #[test]
    fn tracked_does_not_flag_already_resident_edge() {
        let mut g = SymbolGraph::new();
        update_file(
            &mut g,
            FileSymbols {
                file: "a.ts".into(),
                symbols: vec![fn_node(1, "caller", "a.ts"), fn_node(2, "real", "a.ts")],
                imports: Vec::new(),
                reexports: Vec::new(),
                calls: vec![CallSite {
                    from: from_fn("caller", 0),
                    callee: CalleeRef {
                        name: "real".into(),
                        via_import: None,
                    },
                    line: 1,
                }],
                calls_partial: false,
                has_unresolved_dynamic_import: false,
                content_hash: None,
            },
        );
        // The edge is already resident from update_file; re-resolving the same
        // call adds nothing AND reports nothing unresolved.
        let batch = vec![(
            "a.ts".to_string(),
            CallSite {
                from: from_fn("caller", 0),
                callee: CalleeRef {
                    name: "real".into(),
                    via_import: None,
                },
                line: 1,
            },
        )];
        let res = re_resolve_calls_tracked(&mut g, &batch);
        assert!(
            res.added.is_empty(),
            "already-resident edge is not re-added"
        );
        assert!(
            res.unresolved.is_empty(),
            "a resolved-but-present edge is not unresolved: {:?}",
            res.unresolved
        );
    }
}
