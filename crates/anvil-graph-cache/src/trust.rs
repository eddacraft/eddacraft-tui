use anvil_kernel_types::{ImportEdge, SymbolKind, TrustLevel, Visibility};

use super::symbol_graph::SymbolGraph;

/// Sensitive module names that indicate privileged access.
/// Matched by exact module token (or `node:` prefix), not substring, to avoid
/// false positives on packages like `fsevents` or `http-errors`.
const PRIVILEGED_MODULES: &[&str] = &["fs", "child_process", "net", "http", "https", "crypto"];

/// External module patterns (not relative imports).
fn is_external_import(source: &str) -> bool {
    !source.starts_with('.') && !source.starts_with('/')
}

/// Check whether an import source refers to a privileged Node.js module.
/// Matches the bare name (`"fs"`) or the `node:` prefixed form (`"node:fs"`)
/// as an exact token so that unrelated packages (e.g. `fsevents`, `http-errors`)
/// are not misclassified.
fn is_privileged_import(source: &str) -> bool {
    let module = source.strip_prefix("node:").unwrap_or(source);
    // Only match the top-level module name (before any `/` subpath).
    let token = module.split('/').next().unwrap_or(module);
    PRIVILEGED_MODULES.contains(&token)
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
        if is_privileged_import(&import.to_source) {
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
        // Preserve TrustLevel::External on synthetic external module nodes created
        // by resolve_import — re-classifying them as Boundary would disable the
        // NewDependencyIntroduction invariant that checks for External targets.
        // Only protect Module-kind nodes (package placeholders like "react",
        // "node:fs"), not regular project symbols that may have External trust
        // from a previous pass.
        if let Some(node) = graph.get_symbol(id)
            && node.trust_level == TrustLevel::External
            && node.kind == SymbolKind::Module
        {
            continue;
        }

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
            line: 0,
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
            line: 0,
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Privileged);
    }

    #[test]
    fn substring_match_does_not_false_positive() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "watcher", "a.ts", Visibility::Internal))
            .unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "fsevents".to_string(),
            line: 0,
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(
            g.get_symbol(1).unwrap().trust_level,
            TrustLevel::External,
            "fsevents should not be classified as Privileged"
        );
    }

    #[test]
    fn http_errors_not_privileged() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "handler", "a.ts", Visibility::Internal))
            .unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "http-errors".to_string(),
            line: 0,
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(
            g.get_symbol(1).unwrap().trust_level,
            TrustLevel::External,
            "http-errors should not be classified as Privileged"
        );
    }

    #[test]
    fn node_prefixed_subpath_is_privileged() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "reader", "a.ts", Visibility::Internal))
            .unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "node:fs/promises".to_string(),
            line: 0,
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(
            g.get_symbol(1).unwrap().trust_level,
            TrustLevel::Privileged,
            "node:fs/promises should be classified as Privileged"
        );
    }

    #[test]
    fn external_trust_preserved_for_synthetic_module_nodes() {
        let mut g = SymbolGraph::new();
        // Synthetic external module node (as created by resolve_import)
        let mut syn = make_symbol(1, "axios", "axios", Visibility::Public);
        syn.trust_level = TrustLevel::External;
        syn.kind = SymbolKind::Module;
        g.add_symbol(syn).unwrap();

        // Regular project symbol that happens to have External trust
        let mut proj = make_symbol(2, "handler", "src/api.ts", Visibility::Public);
        proj.trust_level = TrustLevel::External;
        g.add_symbol(proj).unwrap();

        annotate_trust(&mut g, &[]);

        assert_eq!(
            g.get_symbol(1).unwrap().trust_level,
            TrustLevel::External,
            "synthetic Module node should preserve External trust"
        );
        assert_eq!(
            g.get_symbol(2).unwrap().trust_level,
            TrustLevel::Boundary,
            "regular project symbol should be reclassified based on visibility"
        );
    }

    #[test]
    fn relative_imports_not_external() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "util", "a.ts", Visibility::Internal))
            .unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "./utils".to_string(),
            line: 0,
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Internal);
    }
}
