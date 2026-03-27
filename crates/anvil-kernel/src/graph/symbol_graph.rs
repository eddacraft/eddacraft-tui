use std::collections::HashMap;

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

pub struct SymbolGraph {
    graph: DiGraph<SymbolNode, SymbolEdge>,
    index: HashMap<u64, NodeIndex>,
    files: HashMap<String, Vec<u64>>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index: HashMap::new(),
            files: HashMap::new(),
        }
    }

    pub fn add_symbol(&mut self, node: SymbolNode) -> Result<NodeIndex, GraphError> {
        if self.index.contains_key(&node.id) {
            return Err(GraphError::DuplicateSymbol(node.id));
        }
        let id = node.id;
        let file = node.file.clone();
        let idx = self.graph.add_node(node);
        self.index.insert(id, idx);
        self.files.entry(file).or_default().push(id);
        Ok(idx)
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
        Ok(())
    }

    pub fn get_symbol(&self, id: u64) -> Option<&SymbolNode> {
        self.index.get(&id).map(|idx| &self.graph[*idx])
    }

    pub fn get_symbol_mut(&mut self, id: u64) -> Option<&mut SymbolNode> {
        self.index.get(&id).map(|idx| &mut self.graph[*idx])
    }

    pub fn symbols_in_file(&self, file: &str) -> Vec<&SymbolNode> {
        self.files
            .get(file)
            .map(|ids| ids.iter().filter_map(|id| self.get_symbol(*id)).collect())
            .unwrap_or_default()
    }

    pub fn remove_file(&mut self, file: &str) -> Vec<u64> {
        let ids = self.files.remove(file).unwrap_or_default();

        // Collect all NodeIndex values up-front before any removal.
        let mut indices: Vec<(u64, NodeIndex)> = ids
            .iter()
            .filter_map(|&id| self.index.remove(&id).map(|idx| (id, idx)))
            .collect();

        // Sort by descending raw index so we always remove from the end first.
        // This guarantees swap-remove never displaces a node we still need to
        // process, because the swapped-in node always has a lower index than
        // the one being removed.
        indices.sort_by(|a, b| b.1.index().cmp(&a.1.index()));

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
