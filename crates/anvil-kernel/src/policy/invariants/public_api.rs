use anvil_kernel_types::Visibility;

use crate::graph::SymbolGraph;
use crate::graph::incremental::GraphDelta;
use crate::policy::config::ArchitectureConfig;
use crate::policy::engine::{Invariant, Severity, Violation};

/// Detects when a GraphDelta adds a new Public symbol. New exports expand
/// the API surface and warrant review.
pub struct PublicApiExpansion;

impl Invariant for PublicApiExpansion {
    fn id(&self) -> &str {
        "public-api-expansion"
    }

    fn evaluate(
        &self,
        delta: &GraphDelta,
        graph: &SymbolGraph,
        _config: &ArchitectureConfig,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        for &sym_id in &delta.added_symbols {
            let sym = match graph.get_symbol(sym_id) {
                Some(s) => s,
                None => continue,
            };
            if sym.visibility == Visibility::Public {
                violations.push(Violation {
                    policy_id: self.id().to_string(),
                    file: sym.file.clone(),
                    symbol: sym.name.clone(),
                    message: format!("new public symbol '{}' expands API surface", sym.name),
                    severity: Severity::Low,
                });
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel};

    fn make_sym(id: u64, name: &str, file: &str, vis: Visibility) -> SymbolNode {
        SymbolNode {
            id,
            kind: SymbolKind::Function,
            name: name.to_string(),
            visibility: vis,
            file: file.to_string(),
            trust_level: TrustLevel::Unknown,
        }
    }

    fn empty_config() -> ArchitectureConfig {
        ArchitectureConfig { layers: Vec::new() }
    }

    #[test]
    fn fires_on_new_public_symbol() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "greet", "src/api.ts", Visibility::Public))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/api.ts".to_string(),
            ..Default::default()
        };

        let inv = PublicApiExpansion;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy_id, "public-api-expansion");
        assert_eq!(violations[0].file, "src/api.ts");
        assert_eq!(violations[0].symbol, "greet");
    }

    #[test]
    fn does_not_fire_on_internal_symbol() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "helper", "src/util.ts", Visibility::Internal))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/util.ts".to_string(),
            ..Default::default()
        };

        let inv = PublicApiExpansion;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn correct_fields_in_violation() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(42, "MyClass", "lib/core.ts", Visibility::Public))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![42],
            file: "lib/core.ts".to_string(),
            ..Default::default()
        };

        let inv = PublicApiExpansion;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert_eq!(violations[0].policy_id, "public-api-expansion");
        assert_eq!(violations[0].file, "lib/core.ts");
        assert_eq!(violations[0].symbol, "MyClass");
        assert_eq!(violations[0].severity, Severity::Low);
    }
}
