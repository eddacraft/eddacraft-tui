use std::collections::HashMap;

use anvil_kernel_types::{SymbolIdentity, TrustLevel};

use crate::graph::{GraphDelta, SymbolGraph};
use crate::policy::config::ArchitectureConfig;
use crate::policy::engine::{Invariant, Severity, Violation};

/// Detects when a `GraphDelta` adds a symbol with `TrustLevel::Privileged`
/// (imports `node:fs`, `child_process`, etc.). New privileged access warrants review.
pub struct PrivilegeExpansion;

impl Invariant for PrivilegeExpansion {
    fn id(&self) -> &'static str {
        "privilege-expansion"
    }

    fn evaluate(
        &self,
        delta: &GraphDelta,
        graph: &SymbolGraph,
        _config: &ArchitectureConfig,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Resolve stable identities once per file (GV2-002): ordinals keep
        // same-(kind, name) overloads distinct, so a newly privileged
        // overload cannot hide behind an already-privileged baseline entry.
        // This check is deliberately Privileged-only — `Boundary` marks a
        // public API boundary, not privileged module access, and lives in
        // `GraphDelta::previously_boundary` for the certify export-diff. The
        // Privileged-only baseline also means a Boundary → Privileged
        // escalation is absent from it and correctly fires here.
        let mut identity_by_id: HashMap<&str, HashMap<u64, SymbolIdentity>> = HashMap::new();

        for &sym_id in &delta.added_symbols {
            let Some(sym) = graph.get_symbol(sym_id) else {
                continue;
            };
            if sym.trust_level != TrustLevel::Privileged {
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
            if !delta.previously_privileged.contains(identity) {
                violations.push(Violation {
                    policy_id: self.id().to_string(),
                    file: sym.file.clone(),
                    symbol: sym.name.clone(),
                    message: format!(
                        "symbol '{}' has privileged access — review required",
                        sym.name
                    ),
                    severity: Severity::Critical,
                });
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, SymbolNode, Visibility};

    fn make_sym(id: u64, name: &str, file: &str, trust: TrustLevel) -> SymbolNode {
        SymbolNode {
            id,
            kind: SymbolKind::Function,
            name: name.to_string(),
            visibility: Visibility::Internal,
            file: file.to_string(),
            trust_level: trust,
            span: None,
        }
    }

    fn empty_config() -> ArchitectureConfig {
        ArchitectureConfig { layers: Vec::new() }
    }

    #[test]
    fn fires_on_privileged_symbol() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(
                1,
                "deleteFiles",
                "src/cleanup.ts",
                TrustLevel::Privileged,
            ))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/cleanup.ts".to_string(),
            ..Default::default()
        };

        let inv = PrivilegeExpansion;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy_id, "privilege-expansion");
        assert_eq!(violations[0].file, "src/cleanup.ts");
        assert_eq!(violations[0].symbol, "deleteFiles");
        assert_eq!(violations[0].severity, Severity::Critical);
    }

    #[test]
    fn does_not_fire_on_non_privileged_symbol() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "helper", "src/util.ts", TrustLevel::Internal))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/util.ts".to_string(),
            ..Default::default()
        };

        let inv = PrivilegeExpansion;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_fire_on_previously_privileged_symbol() {
        let mut graph = SymbolGraph::new();
        let existing = make_sym(1, "deleteFiles", "src/cleanup.ts", TrustLevel::Privileged);

        let mut previously_privileged = std::collections::HashSet::new();
        previously_privileged.insert(SymbolIdentity {
            file: existing.file.clone(),
            kind: existing.kind,
            name: existing.name.clone(),
            ordinal: 0,
        });

        graph.add_symbol(existing).unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/cleanup.ts".to_string(),
            previously_privileged,
            ..Default::default()
        };

        let inv = PrivilegeExpansion;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert!(
            violations.is_empty(),
            "should not flag a symbol that was already privileged"
        );
    }

    #[test]
    fn does_not_fire_on_boundary_symbol() {
        // Boundary marks a public API boundary, not privileged module
        // access — annotate_trust assigns it to every public symbol outside
        // privileged files, so firing here would spam Critical violations.
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(1, "publicApi", "src/api.ts", TrustLevel::Boundary))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/api.ts".to_string(),
            ..Default::default()
        };

        let violations = PrivilegeExpansion.evaluate(&delta, &graph, &empty_config());
        assert!(
            violations.is_empty(),
            "Boundary trust must not fire privilege-expansion"
        );
    }

    #[test]
    fn fires_on_boundary_to_privileged_escalation() {
        // The baseline is Privileged-only, so a symbol that was Boundary
        // yesterday and Privileged today is NOT in the baseline and must
        // fire — the Boundary identities live in previously_boundary, not
        // previously_privileged.
        let mut graph = SymbolGraph::new();
        let escalated = make_sym(1, "nowPrivileged", "src/api.ts", TrustLevel::Privileged);

        let mut previously_boundary = std::collections::HashSet::new();
        previously_boundary.insert(SymbolIdentity {
            file: escalated.file.clone(),
            kind: escalated.kind,
            name: escalated.name.clone(),
            ordinal: 0,
        });

        graph.add_symbol(escalated).unwrap();

        let delta = GraphDelta {
            added_symbols: vec![1],
            file: "src/api.ts".to_string(),
            previously_boundary,
            ..Default::default()
        };

        let violations = PrivilegeExpansion.evaluate(&delta, &graph, &empty_config());
        assert_eq!(
            violations.len(),
            1,
            "a Boundary -> Privileged escalation must fire the invariant"
        );
    }

    #[test]
    fn correct_fields_in_violation() {
        let mut graph = SymbolGraph::new();
        graph
            .add_symbol(make_sym(
                99,
                "execCmd",
                "lib/shell.ts",
                TrustLevel::Privileged,
            ))
            .unwrap();

        let delta = GraphDelta {
            added_symbols: vec![99],
            file: "lib/shell.ts".to_string(),
            ..Default::default()
        };

        let inv = PrivilegeExpansion;
        let violations = inv.evaluate(&delta, &graph, &empty_config());

        assert_eq!(violations[0].policy_id, "privilege-expansion");
        assert_eq!(violations[0].file, "lib/shell.ts");
        assert_eq!(violations[0].symbol, "execCmd");
    }
}
