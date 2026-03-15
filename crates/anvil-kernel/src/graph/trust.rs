use anvil_kernel_types::{TrustLevel, Visibility};

use super::symbol_graph::SymbolGraph;
use crate::parser::extract::ImportEdge;

/// Sensitive API patterns that indicate privileged access.
const PRIVILEGED_PATTERNS: &[&str] = &[
    "node:fs",
    "node:child_process",
    "node:net",
    "node:http",
    "node:https",
    "node:crypto",
    "fs",
    "child_process",
    "net",
    "http",
    "https",
    "exec",
    "spawn",
    "process.env",
];

/// External module patterns (not relative imports).
fn is_external_import(source: &str) -> bool {
    !source.starts_with('.') && !source.starts_with('/')
}

/// Annotate trust levels on all symbols in the graph based on heuristics.
///
/// This is a best-effort pass -- heuristics can be overridden by configuration.
pub fn annotate_trust(graph: &mut SymbolGraph, imports: &[ImportEdge]) {
    let mut external_files: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut privileged_files: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for import in imports {
        if is_external_import(&import.to_source) {
            external_files.insert(&import.from_file);
        }
        if PRIVILEGED_PATTERNS
            .iter()
            .any(|p| import.to_source.contains(p))
        {
            privileged_files.insert(&import.from_file);
        }
    }

    let symbol_info: Vec<(u64, String, Visibility)> = {
        let inner = graph.inner();
        inner
            .node_weights()
            .map(|n| (n.id, n.file.clone(), n.visibility))
            .collect()
    };

    for (id, file, visibility) in symbol_info {
        let trust = if privileged_files.contains(file.as_str()) {
            TrustLevel::Privileged
        } else if visibility == Visibility::Public {
            TrustLevel::Boundary
        } else if external_files.contains(file.as_str()) {
            TrustLevel::External
        } else {
            TrustLevel::Internal
        };

        if let Some(node) = graph.get_symbol_mut(id) {
            node.trust_level = trust;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, SymbolNode};

    fn make_symbol(id: u64, name: &str, file: &str, vis: Visibility) -> SymbolNode {
        SymbolNode {
            id,
            kind: SymbolKind::Function,
            name: name.to_string(),
            visibility: vis,
            file: file.to_string(),
            trust_level: TrustLevel::Unknown,
        }
    }

    #[test]
    fn public_symbols_get_boundary_trust() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "greet", "a.ts", Visibility::Public))
            .unwrap();

        annotate_trust(&mut g, &[]);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Boundary);
    }

    #[test]
    fn internal_symbols_stay_internal() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "helper", "a.ts", Visibility::Internal))
            .unwrap();

        annotate_trust(&mut g, &[]);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Internal);
    }

    #[test]
    fn external_import_marks_file_as_external() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "handler", "a.ts", Visibility::Internal))
            .unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "express".to_string(),
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::External);
    }

    #[test]
    fn privileged_import_overrides_other_trust() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "readFile", "a.ts", Visibility::Public))
            .unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "node:fs".to_string(),
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Privileged);
    }

    #[test]
    fn relative_imports_not_external() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "util", "a.ts", Visibility::Internal))
            .unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "./utils".to_string(),
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Internal);
    }
}
