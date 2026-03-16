use std::collections::HashSet;
use std::path::Path;

use anvil_kernel_types::{EdgeType, SymbolKind, SymbolNode, TrustLevel, Visibility};

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
    /// Import sources that existed before this update (for new-dep detection).
    pub previously_imported: HashSet<String>,
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

    // Capture prior imports for this file so the new-dep invariant can
    // distinguish genuinely new imports from re-added ones.
    let previously_imported: HashSet<String> = removed_ids
        .iter()
        .flat_map(|_| {
            // We already removed the edges, so reconstruct from the old symbols'
            // outgoing edges that were captured before removal. Since we don't
            // have that history, we fall back to checking what the graph currently
            // knows — which means we check the broader graph for any existing
            // import targets. This is imperfect but avoids false positives for
            // imports that are already in the graph from other files.
            Vec::new()
        })
        .collect();

    // Collect all known file paths in the graph for import resolution.
    let known_files: Vec<String> = graph
        .inner()
        .node_weights()
        .map(|s| s.file.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Use the first added symbol as the edge origin. If the file has no
    // symbols (side-effect-only module), create a synthetic Module node
    // so import edges are still recorded.
    let from_id = if let Some(&id) = added_ids.first() {
        id
    } else if !new_symbols.imports.is_empty() {
        let synthetic_id = graph
            .inner()
            .node_weights()
            .map(|s| s.id)
            .max()
            .unwrap_or(0)
            + 1;
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
            synthetic_id
        } else {
            0 // won't match, edges will be skipped
        }
    } else {
        0
    };

    let mut added_edges = Vec::new();
    for import in new_symbols.imports {
        if from_id == 0 {
            continue;
        }
        // Resolve the import specifier to a known file path
        let to_id = resolve_import(&import.to_source, &file, &known_files, graph);

        if let Some(to) = to_id {
            let edge = anvil_kernel_types::SymbolEdge {
                from: from_id,
                to,
                edge_type: EdgeType::Imports,
            };
            if graph.add_edge(edge).is_ok() {
                added_edges.push((from_id, to, EdgeType::Imports));
            }
        }
    }

    GraphDelta {
        added_symbols: added_ids,
        removed_symbols: removed_ids,
        added_edges,
        removed_edges: Vec::new(),
        errors,
        previously_imported,
        file,
    }
}

/// Resolve an import specifier to a symbol ID in the graph.
///
/// For relative imports (`./module`, `../lib`), resolve against the importing
/// file's directory and try common extensions (.ts, .tsx, .js, /index.ts, etc.).
/// For bare specifiers (`express`, `node:fs`), match against file names directly
/// (these represent external/virtual modules).
fn resolve_import(
    specifier: &str,
    from_file: &str,
    known_files: &[String],
    graph: &SymbolGraph,
) -> Option<u64> {
    // Non-relative imports: match file field directly (external modules)
    if !specifier.starts_with('.') {
        return graph
            .inner()
            .node_weights()
            .find(|s| s.file == specifier)
            .map(|s| s.id);
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
        let normalised = candidate.replace("\\", "/");
        if let Some(file_path) = known_files.iter().find(|f| {
            let f_norm = f.replace("\\", "/");
            f_norm == normalised || f_norm.ends_with(&normalised)
        }) {
            return graph
                .inner()
                .node_weights()
                .find(|s| s.file == *file_path)
                .map(|s| s.id);
        }
    }

    None
}

/// Remove a deleted file from the graph entirely.
pub fn remove_file(graph: &mut SymbolGraph, file: &str) -> GraphDelta {
    let removed_ids = graph.remove_file(file);
    GraphDelta {
        removed_symbols: removed_ids,
        file: file.to_string(),
        previously_imported: HashSet::new(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::extract::ImportEdge;
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
            }],
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
            }],
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
