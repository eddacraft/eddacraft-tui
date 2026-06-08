use std::collections::HashMap;

use anvil_kernel_types::{SymbolIdentity, Visibility};

use crate::graph::{GraphDelta, SymbolGraph};
use crate::policy::config::ArchitectureConfig;
use crate::policy::engine::{Invariant, Severity, Violation};

/// Detects when a `GraphDelta` adds a new Public symbol. New exports expand
/// the API surface and warrant review.
pub struct PublicApiExpansion;

impl Invariant for PublicApiExpansion {
    fn id(&self) -> &'static str {
        "public-api-expansion"
    }

    fn evaluate(
        &self,
        delta: &GraphDelta,
        graph: &SymbolGraph,
        _config: &ArchitectureConfig,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Resolve stable identities once per file (GV2-002): ordinals keep
        // same-(kind, name) overloads distinct, so a new public overload of
        // an existing export is itself an API expansion. Computed lazily per
        // file — not per symbol — to keep the loop O(file symbols), not
        // O(added × file symbols).
        let mut identity_by_id: HashMap<&str, HashMap<u64, SymbolIdentity>> = HashMap::new();

        for &sym_id in &delta.added_symbols {
            let Some(sym) = graph.get_symbol(sym_id) else {
                continue;
            };
            if sym.visibility != Visibility::Public {
                continue;
            }
            let file_identities = identity_by_id.entry(sym.file.as_str()).or_insert_with(|| {
                let file_symbols = graph.symbols_in_file(&sym.file);
                let identities = SymbolIdentity::for_file_symbols(&file_symbols);
                file_symbols.iter().map(|s| s.id).zip(identities).collect()
            });
            let Some(identity) = file_identities.get(&sym_id) else {
                // Every added id must be reachable from its file's symbol
                // list; a miss means graph state diverged mid-evaluation.
                // Loud in debug builds, conservative skip in release.
                debug_assert!(
                    false,
                    "added symbol {sym_id} missing from its file's symbol list"
                );
                continue;
            };
            if !delta.previously_public.contains(identity) {
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
    fn does_not_fire_on_previously_public_symbol() {
        let mut graph = SymbolGraph::new();
        let existing = make_sym(1, "greet", "src/api.ts", Visibility::Public);

        let mut previously_public = std::collections::HashSet::new();
        previously_public.insert(SymbolIdentity {
            file: existing.file.clone(),
            kind: existing.kind,
            name: existing.name.clone(),
            ordinal: 0,
        });

        graph.add_symbol(existing).unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/api.ts".to_string(),
            previously_public,
            ..Default::default()
        };

        let inv = PublicApiExpansion;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert!(
            violations.is_empty(),
            "should not flag a symbol that was already public"
        );
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
