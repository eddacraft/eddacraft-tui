use anvil_kernel_types::EdgeType;

use crate::graph::SymbolGraph;
use crate::graph::incremental::GraphDelta;
use crate::policy::config::ArchitectureConfig;
use crate::policy::engine::{Invariant, Severity, Violation};

/// Detects when a file in layer A imports from layer B where B is not
/// in A's `allowed_imports`.
pub struct CrossLayerViolation;

impl Invariant for CrossLayerViolation {
    fn id(&self) -> &'static str {
        "cross-layer-violation"
    }

    fn evaluate(
        &self,
        delta: &GraphDelta,
        graph: &SymbolGraph,
        config: &ArchitectureConfig,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        let from_layer = match config.layer_for_file(&delta.file) {
            Some(l) => l.name.clone(),
            None => return violations,
        };

        for &sym_id in &delta.added_symbols {
            let edges = graph.outgoing_edges(sym_id);
            for edge in edges {
                if edge.edge_type != EdgeType::Imports {
                    continue;
                }
                let Some(target) = graph.get_symbol(edge.to) else {
                    continue;
                };
                let to_layer = match config.layer_for_file(&target.file) {
                    Some(l) => &l.name,
                    None => continue,
                };
                if !config.is_import_allowed(&from_layer, to_layer) {
                    let from_sym = graph.get_symbol(sym_id);
                    let sym_name = from_sym.map_or("unknown", |s| &s.name);
                    violations.push(Violation {
                        policy_id: self.id().to_string(),
                        file: delta.file.clone(),
                        symbol: sym_name.to_string(),
                        message: format!(
                            "layer '{from_layer}' cannot import from layer '{to_layer}'"
                        ),
                        severity: Severity::High,
                    });
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};

    fn make_config() -> ArchitectureConfig {
        ArchitectureConfig::from_yaml(
            r#"
layers:
  - name: domain
    paths: ["src/domain/*"]
    allowed_imports: [domain]
  - name: infra
    paths: ["src/infra/*"]
    allowed_imports: [domain, infra]
"#,
        )
        .unwrap()
    }

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

    #[test]
    fn fires_on_cross_layer_import() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "domainFn", "src/domain/user.ts"))
            .unwrap();
        graph
            .add_symbol(make_sym(2, "infraFn", "src/infra/db.ts"))
            .unwrap();
        graph
            .add_edge(anvil_kernel_types::SymbolEdge {
                from: 1,
                to: 2,
                edge_type: EdgeType::Imports,
            })
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/domain/user.ts".to_string(),
            ..Default::default()
        };

        let inv = CrossLayerViolation;
        let violations = inv.evaluate(&delta, &graph, &make_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy_id, "cross-layer-violation");
        assert_eq!(violations[0].file, "src/domain/user.ts");
        assert_eq!(violations[0].symbol, "domainFn");
    }

    #[test]
    fn does_not_fire_on_allowed_import() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "infraFn", "src/infra/db.ts"))
            .unwrap();
        graph
            .add_symbol(make_sym(2, "domainFn", "src/domain/user.ts"))
            .unwrap();
        graph
            .add_edge(anvil_kernel_types::SymbolEdge {
                from: 1,
                to: 2,
                edge_type: EdgeType::Imports,
            })
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/infra/db.ts".to_string(),
            ..Default::default()
        };

        let inv = CrossLayerViolation;
        let violations = inv.evaluate(&delta, &graph, &make_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_fire_for_unmatched_file() {
        let graph = SymbolGraph::new();
        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "test/helpers.ts".to_string(),
            ..Default::default()
        };

        let inv = CrossLayerViolation;
        let violations = inv.evaluate(&delta, &graph, &make_config());

        assert!(violations.is_empty());
    }
}
