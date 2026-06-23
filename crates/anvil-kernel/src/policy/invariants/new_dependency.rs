use anvil_kernel_types::TrustLevel;

use crate::graph::SymbolGraph;
use crate::graph::incremental::GraphDelta;
use crate::policy::config::ArchitectureConfig;
use crate::policy::engine::{Invariant, Severity, Violation};

/// Detects when a `GraphDelta` adds a symbol in a file that imports an external
/// module not previously seen. Flags new external dependencies for review.
pub struct NewDependencyIntroduction;

impl Invariant for NewDependencyIntroduction {
    fn id(&self) -> &'static str {
        "new-dependency-introduction"
    }

    fn evaluate(
        &self,
        delta: &GraphDelta,
        graph: &SymbolGraph,
        _config: &ArchitectureConfig,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        if delta.added_symbols.is_empty() {
            return violations;
        }

        // Check added edges for new external imports that were NOT present
        // before this update (avoids false positives on re-added imports).
        for &(from_id, to_id, ref edge_type) in &delta.added_edges {
            if *edge_type != anvil_kernel_types::EdgeType::Imports {
                continue;
            }
            let Some(from_sym) = graph.get_symbol(from_id) else {
                continue;
            };
            let Some(target_sym) = graph.get_symbol(to_id) else {
                continue;
            };
            // Use trust level to identify external targets — synthetic external
            // nodes created by resolve_import have TrustLevel::External.
            // This correctly handles npm subpath imports like lodash/fp.js.
            if target_sym.trust_level == TrustLevel::External
                && !delta.previously_imported.contains(&target_sym.file)
            {
                violations.push(Violation {
                    policy_id: self.id().to_string(),
                    file: from_sym.file.clone(),
                    symbol: from_sym.name.clone(),
                    message: format!("new external dependency introduced: {}", target_sym.file),
                    severity: Severity::Medium,
                });
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{EdgeType, SymbolKind, SymbolNode, TrustLevel, Visibility};

    fn make_sym(id: u64, name: &str, file: &str) -> SymbolNode {
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

    fn make_external_sym(id: u64, name: &str, file: &str) -> SymbolNode {
        SymbolNode {
            id,
            kind: SymbolKind::Module,
            name: name.to_string(),
            visibility: Visibility::Public,
            file: file.to_string(),
            trust_level: TrustLevel::External,
            span: None,
        }
    }

    fn empty_config() -> ArchitectureConfig {
        ArchitectureConfig { layers: Vec::new() }
    }

    #[test]
    fn fires_on_new_external_dependency() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "handler", "src/api.ts"))
            .unwrap();
        graph
            .add_symbol(make_external_sym(2, "axios", "axios"))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            added_edges: vec![(1, 2, EdgeType::Imports)],
            file: "src/api.ts".to_string(),
            ..Default::default()
        };

        let inv = NewDependencyIntroduction;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy_id, "new-dependency-introduction");
        assert_eq!(violations[0].file, "src/api.ts");
        assert_eq!(violations[0].symbol, "handler");
    }

    #[test]
    fn does_not_fire_on_relative_import() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "handler", "src/api.ts"))
            .unwrap();
        graph.add_symbol(make_sym(2, "helper", "./utils")).unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            added_edges: vec![(1, 2, EdgeType::Imports)],
            file: "src/api.ts".to_string(),
            ..Default::default()
        };

        let inv = NewDependencyIntroduction;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_fire_on_empty_delta() {
        let graph = SymbolGraph::new();
        let delta = GraphDelta::default();

        let inv = NewDependencyIntroduction;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_fire_on_resolved_internal_path() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "handler", "src/api.ts"))
            .unwrap();
        graph
            .add_symbol(make_sym(2, "helper", "src/utils.ts"))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            added_edges: vec![(1, 2, EdgeType::Imports)],
            file: "src/api.ts".to_string(),
            ..Default::default()
        };

        let inv = NewDependencyIntroduction;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn fires_on_scoped_external_package() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "handler", "src/api.ts"))
            .unwrap();
        graph
            .add_symbol(make_external_sym(2, "zod", "@scope/pkg"))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            added_edges: vec![(1, 2, EdgeType::Imports)],
            file: "src/api.ts".to_string(),
            ..Default::default()
        };

        let inv = NewDependencyIntroduction;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn fires_on_npm_subpath_with_extension() {
        // lodash/fp.js is a valid npm subpath import — should be flagged
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "handler", "src/api.ts"))
            .unwrap();
        graph
            .add_symbol(make_external_sym(2, "lodash", "lodash/fp.js"))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            added_edges: vec![(1, 2, EdgeType::Imports)],
            file: "src/api.ts".to_string(),
            ..Default::default()
        };

        let inv = NewDependencyIntroduction;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert_eq!(violations.len(), 1);
    }
}
