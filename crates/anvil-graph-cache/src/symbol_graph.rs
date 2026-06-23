use std::collections::{BTreeSet, HashMap};

use anvil_kernel_types::{SymbolEdge, SymbolNode};
use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("symbol with id {0} not found")]
    SymbolNotFound(u64),
    #[error("duplicate symbol id {0}")]
    DuplicateSymbol(u64),
}

#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub files: usize,
}

#[derive(Debug)]
pub struct SymbolGraph {
    graph: DiGraph<SymbolNode, SymbolEdge>,
    index: HashMap<u64, NodeIndex>,
    files: HashMap<String, Vec<u64>>,
    /// Monotonic high-water mark: one past the largest id ever inserted.
    /// Synthetic-node creation in `incremental::{update_file, resolve_import}`
    /// reads this to pick a fresh id in O(1), and `watch.rs` syncs its own
    /// per-file allocator against it after every `update_file` so the two
    /// allocators can never collide. Never decremented on `remove_file` —
    /// ids must stay unique across the lifetime of the graph.
    next_id: u64,
    /// CIB-093b: memoised per-file re-export-reached privileged module
    /// specifiers, stamped by `trust::annotate_trust` in a single whole-graph
    /// pass. The certify hot path (`certify::export_surface_diff`) reads this
    /// instead of re-walking a per-file `Reexports` BFS on every `ContentModify`
    /// verdict under the cache Mutex.
    ///
    /// `None` = not computed (or invalidated by a mutation) ⇒ the reader falls
    /// back to the on-demand BFS, so the optimisation can never serve a stale or
    /// missing verdict. `Some(map)` = authoritative; a file absent from the map
    /// reaches no privileged module by re-export. Any graph mutation clears it.
    reexport_privileged: Option<HashMap<String, BTreeSet<String>>>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index: HashMap::new(),
            files: HashMap::new(),
            next_id: 0,
            reexport_privileged: None,
        }
    }

    pub fn add_symbol(&mut self, node: SymbolNode) -> Result<NodeIndex, GraphError> {
        self.reexport_privileged = None;
        if self.index.contains_key(&node.id) {
            return Err(GraphError::DuplicateSymbol(node.id));
        }
        let id = node.id;
        let file = node.file.clone();
        let idx = self.graph.add_node(node);
        self.index.insert(id, idx);
        self.files.entry(file).or_default().push(id);
        if id >= self.next_id {
            self.next_id = id + 1;
        }
        Ok(idx)
    }

    /// One past the largest id ever inserted. Use this to pick a fresh id
    /// without iterating `node_weights()`.
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Raise the `next_id` high-water mark to at least `floor` (never lowers it).
    ///
    /// Used by snapshot restore (GV2-030): a session that inserted then removed
    /// high ids has a `next_id` above any surviving node, so replaying only the
    /// survivors would under-restore the mark and re-issue removed ids. The
    /// persisted `next_id` is replayed through this floor so post-load synthetic
    /// allocation cannot collide with an id the original session already spent.
    pub fn set_next_id_floor(&mut self, floor: u64) {
        self.next_id = self.next_id.max(floor);
    }

    pub fn add_edge(&mut self, edge: SymbolEdge) -> Result<(), GraphError> {
        let from_idx = self
            .index
            .get(&edge.from)
            .copied()
            .ok_or(GraphError::SymbolNotFound(edge.from))?;
        let to_idx = self
            .index
            .get(&edge.to)
            .copied()
            .ok_or(GraphError::SymbolNotFound(edge.to))?;
        self.graph.add_edge(from_idx, to_idx, edge);
        self.reexport_privileged = None;
        Ok(())
    }

    pub fn get_symbol(&self, id: u64) -> Option<&SymbolNode> {
        self.index.get(&id).map(|idx| &self.graph[*idx])
    }

    /// Whether `file` has at least one recorded symbol — O(1), no scan.
    ///
    /// A file leaves no `files` entry unless a symbol was added for it, so this
    /// returns `false` for a never-seen file, an evicted file, AND a file that
    /// was extracted but yielded zero symbols — those cases are not
    /// distinguished here. The hot-read API treats every such non-resident file
    /// as a warm-miss rather than trusting a false-empty surface.
    pub fn contains_file(&self, file: &str) -> bool {
        self.files.contains_key(file)
    }

    pub fn get_symbol_mut(&mut self, id: u64) -> Option<&mut SymbolNode> {
        // A caller may mutate `trust_level`/`file`, which the re-export-privilege
        // memo (CIB-093b) derives from. Invalidate conservatively; `annotate_trust`
        // re-stamps the memo via `set_reexport_privileged` after its own
        // `get_symbol_mut` loop completes.
        self.reexport_privileged = None;
        self.index.get(&id).map(|idx| &mut self.graph[*idx])
    }

    /// CIB-093b: install the memoised per-file re-export-reached privileged module
    /// specifiers. Called by [`crate::trust::annotate_trust`] after its whole-graph
    /// trust pass; the certify hot path reads it via
    /// [`Self::reexport_privileged_for`] instead of re-walking the per-file BFS.
    pub(crate) fn set_reexport_privileged(&mut self, map: HashMap<String, BTreeSet<String>>) {
        self.reexport_privileged = Some(map);
    }

    /// CIB-093b: the memoised re-export-reached privileged module specifiers for
    /// `file`, or `None` when the memo is absent/invalidated (the caller must then
    /// fall back to the on-demand BFS). A `Some(empty)` means the memo is present
    /// and `file` reaches no privileged module by re-export.
    #[must_use]
    pub(crate) fn reexport_privileged_for(&self, file: &str) -> Option<BTreeSet<String>> {
        self.reexport_privileged
            .as_ref()
            .map(|map| map.get(file).cloned().unwrap_or_default())
    }

    /// The distinct files with at least one resident symbol, in arbitrary order.
    ///
    /// O(files), not O(symbols): iterates the per-file index keys rather than
    /// every node. A GCTX-style enumeration that needs per-file parse order
    /// (for `SymbolIdentity::for_file_symbols` ordinals) iterates this and calls
    /// [`Self::symbols_in_file`] per file, avoiding a full `node_weights()` scan
    /// to rediscover the file set.
    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// Symbols of `file` in insertion order — which equals parse (source)
    /// order because `update_file` inserts `FileSymbols.symbols` in parser
    /// emission order. `SymbolIdentity::for_file_symbols` ordinals depend on
    /// this; do not reorder the per-file id list.
    pub fn symbols_in_file(&self, file: &str) -> Vec<&SymbolNode> {
        self.files
            .get(file)
            .map(|ids| ids.iter().filter_map(|id| self.get_symbol(*id)).collect())
            .unwrap_or_default()
    }

    pub fn remove_file(&mut self, file: &str) -> Vec<u64> {
        self.reexport_privileged = None;
        let ids = self.files.remove(file).unwrap_or_default();

        // Collect all NodeIndex values up-front before any removal.
        let mut indices: Vec<(u64, NodeIndex)> = ids
            .iter()
            .filter_map(|&id| self.index.remove(&id).map(|idx| (id, idx)))
            .collect();

        // Sort by descending raw index so we always remove higher indices first.
        // petgraph::Graph::remove_node swap-moves the last node into the removed
        // slot. By processing nodes from highest index to lowest, we ensure that
        // any node that gets swap-moved will not later be removed again by its
        // original index, because that original index has already been handled.
        indices.sort_by_key(|(_, idx)| std::cmp::Reverse(idx.index()));

        for (_, idx) in &indices {
            self.graph.remove_node(*idx);
            // petgraph swaps the last node into the removed slot,
            // so we need to update the index for the swapped node
            if let Some(swapped) = self.graph.node_weight(*idx) {
                self.index.insert(swapped.id, *idx);
            }
        }

        ids
    }

    pub fn stats(&self) -> GraphStats {
        GraphStats {
            node_count: self.graph.node_count(),
            edge_count: self.graph.edge_count(),
            files: self.files.len(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get outgoing edges from a symbol (what does this symbol depend on?).
    pub fn outgoing_edges(&self, id: u64) -> Vec<&SymbolEdge> {
        self.index
            .get(&id)
            .map(|idx| self.graph.edges(*idx).map(|e| e.weight()).collect())
            .unwrap_or_default()
    }

    /// Get incoming edges to a symbol (what depends on this symbol?).
    pub fn incoming_edges(&self, id: u64) -> Vec<&SymbolEdge> {
        self.index
            .get(&id)
            .map(|idx| {
                self.graph
                    .edges_directed(*idx, petgraph::Direction::Incoming)
                    .map(|e| e.weight())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the underlying petgraph for advanced queries.
    pub fn inner(&self) -> &DiGraph<SymbolNode, SymbolEdge> {
        &self.graph
    }
}

impl Default for SymbolGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{EdgeType, SymbolKind, TrustLevel, Visibility};

    fn make_symbol(id: u64, name: &str, file: &str, kind: SymbolKind) -> SymbolNode {
        SymbolNode {
            id,
            kind,
            name: name.to_string(),
            visibility: Visibility::Internal,
            file: file.to_string(),
            trust_level: TrustLevel::Unknown,
            span: None,
        }
    }

    #[test]
    fn add_and_retrieve_symbols() {
        let mut g = SymbolGraph::new();
        let s = make_symbol(1, "foo", "src/a.ts", SymbolKind::Function);
        g.add_symbol(s).unwrap();

        let retrieved = g.get_symbol(1).unwrap();
        assert_eq!(retrieved.name, "foo");
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn duplicate_symbol_rejected() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function))
            .unwrap();
        let result = g.add_symbol(make_symbol(1, "bar", "b.ts", SymbolKind::Function));
        assert!(matches!(result, Err(GraphError::DuplicateSymbol(1))));
    }

    #[test]
    fn add_and_query_edges() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(2, "bar", "b.ts", SymbolKind::Function))
            .unwrap();

        g.add_edge(SymbolEdge {
            from: 1,
            to: 2,
            edge_type: EdgeType::Calls,
        })
        .unwrap();

        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.outgoing_edges(1).len(), 1);
        assert_eq!(g.outgoing_edges(1)[0].edge_type, EdgeType::Calls);
        assert_eq!(g.incoming_edges(2).len(), 1);
    }

    #[test]
    fn edge_with_missing_node_rejected() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function))
            .unwrap();

        let result = g.add_edge(SymbolEdge {
            from: 1,
            to: 99,
            edge_type: EdgeType::Calls,
        });
        assert!(matches!(result, Err(GraphError::SymbolNotFound(99))));
    }

    #[test]
    fn symbols_in_file() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(2, "bar", "a.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(3, "baz", "b.ts", SymbolKind::Function))
            .unwrap();

        let syms = g.symbols_in_file("a.ts");
        assert_eq!(syms.len(), 2);
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"bar"));
    }

    #[test]
    fn file_names_lists_distinct_resident_files() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(2, "bar", "a.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(3, "baz", "b.ts", SymbolKind::Function))
            .unwrap();

        let mut files: Vec<&str> = g.file_names().collect();
        files.sort_unstable();
        assert_eq!(files, ["a.ts", "b.ts"]);
    }

    #[test]
    fn remove_file_clears_nodes() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(2, "bar", "a.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(3, "baz", "b.ts", SymbolKind::Function))
            .unwrap();

        let removed = g.remove_file("a.ts");
        assert_eq!(removed.len(), 2);
        assert_eq!(g.node_count(), 1);
        assert!(g.get_symbol(3).is_some());
    }

    #[test]
    fn remove_file_with_interleaved_indices_preserves_other_files() {
        let mut g = SymbolGraph::new();

        // Interleave symbols from two files so their NodeIndex values alternate:
        // a.ts gets indices 0, 2 and b.ts gets indices 1, 3
        g.add_symbol(make_symbol(1, "a_first", "a.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(2, "b_first", "b.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(3, "a_second", "a.ts", SymbolKind::Class))
            .unwrap();
        g.add_symbol(make_symbol(4, "b_second", "b.ts", SymbolKind::Class))
            .unwrap();

        assert_eq!(g.node_count(), 4);

        // Remove the file whose symbols are interleaved with b.ts.
        // Without descending-order removal, petgraph's swap-remove would
        // corrupt the index mapping for b.ts symbols.
        let removed = g.remove_file("a.ts");
        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&1));
        assert!(removed.contains(&3));

        // a.ts symbols must be gone
        assert!(g.get_symbol(1).is_none());
        assert!(g.get_symbol(3).is_none());
        assert!(g.symbols_in_file("a.ts").is_empty());

        // b.ts symbols must survive with correct data
        let b1 = g.get_symbol(2).expect("b_first should still exist");
        assert_eq!(b1.name, "b_first");
        assert_eq!(b1.file, "b.ts");

        let b2 = g.get_symbol(4).expect("b_second should still exist");
        assert_eq!(b2.name, "b_second");
        assert_eq!(b2.file, "b.ts");

        assert_eq!(g.node_count(), 2);
        assert_eq!(g.symbols_in_file("b.ts").len(), 2);
    }

    #[test]
    fn stats() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function))
            .unwrap();
        g.add_symbol(make_symbol(2, "bar", "b.ts", SymbolKind::Function))
            .unwrap();
        g.add_edge(SymbolEdge {
            from: 1,
            to: 2,
            edge_type: EdgeType::Imports,
        })
        .unwrap();

        let stats = g.stats();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
        assert_eq!(stats.files, 2);
    }
}
