use anvil_kernel_types::EdgeType;

use super::symbol_graph::SymbolGraph;
use crate::parser::extract::FileSymbols;

/// Changes produced by an incremental graph update.
#[derive(Debug, Clone, Default)]
pub struct GraphDelta {
    pub added_symbols: Vec<u64>,
    pub removed_symbols: Vec<u64>,
    pub added_edges: Vec<(u64, u64, EdgeType)>,
    pub removed_edges: Vec<(u64, u64, EdgeType)>,
    pub errors: Vec<String>,
    pub file: String,
}

impl GraphDelta {
    pub fn is_empty(&self) -> bool {
        self.added_symbols.is_empty()
            && self.removed_symbols.is_empty()
            && self.added_edges.is_empty()
            && self.removed_edges.is_empty()
            && self.errors.is_empty()
    }
}

/// Apply an incremental update to the graph for a single file.
///
/// 1. Remove all symbols and edges for the file
/// 2. Add new symbols from the re-parsed file
/// 3. Return the delta for downstream consumers (policy engine)
pub fn update_file(graph: &mut SymbolGraph, new_symbols: FileSymbols) -> GraphDelta {
    let file = new_symbols.file.clone();

    let removed_ids = graph.remove_file(&file);

    let mut added_ids = Vec::new();
    let mut errors = Vec::new();
    for symbol in new_symbols.symbols {
        let id = symbol.id;
        match graph.add_symbol(symbol) {
            Ok(_) => added_ids.push(id),
            Err(e) => {
                eprintln!("graph: failed to insert symbol {id}: {e}");
                errors.push(format!("symbol {id}: {e}"));
            }
        }
    }

    // Build edges from imports so downstream invariants (e.g. new-dependency)
    // can inspect added_edges in the delta.
    let mut added_edges = Vec::new();
    for import in new_symbols.imports {
        // Find a symbol in the source file to use as the edge origin
        let from_id = added_ids.first().copied();
        // Find the target symbol by matching the import source against file names
        let to_id = graph
            .inner()
            .node_weights()
            .find(|s| s.file == import.to_source)
            .map(|s| s.id);

        if let (Some(from), Some(to)) = (from_id, to_id) {
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

    GraphDelta {
        added_symbols: added_ids,
        removed_symbols: removed_ids,
        added_edges,
        removed_edges: Vec::new(),
        errors,
        file,
    }
}

/// Remove a deleted file from the graph entirely.
pub fn remove_file(graph: &mut SymbolGraph, file: &str) -> GraphDelta {
    let removed_ids = graph.remove_file(file);
    GraphDelta {
        removed_symbols: removed_ids,
        file: file.to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};

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
        use crate::parser::extract::ImportEdge;

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
            }],
        };

        let delta = update_file(&mut g, syms);

        assert_eq!(delta.added_edges.len(), 1);
        assert_eq!(delta.added_edges[0].0, 1);
        assert_eq!(delta.added_edges[0].1, 50);
        assert_eq!(delta.added_edges[0].2, EdgeType::Imports);
        assert_eq!(g.edge_count(), 1);
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
}
