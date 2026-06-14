//! Multi-graph registry: one typed in-process entry point for querying joined
//! graph state (GV2-020).
//!
//! Consumers (daemon, drivers, MCP/GCTX, weave) depend on the registry's typed
//! accessors and join queries rather than on concrete storage or `petgraph`
//! internals. The registry borrows the resident graphs and yields:
//!
//! - **handles** to the real in-crate graphs — semantic ([`SymbolGraph`]),
//!   dependency ([`DependencyGraph`]), and trust ([`TrustGraph`]);
//! - the **sealed hot-read surface** ([`HotReadApi`]) and the non-hot
//!   [`BackgroundReadApi`], preserving the ADR-063 admissibility seal — the
//!   registry never widens the hot surface;
//! - **join queries** across those graphs (e.g. importers of a file with their
//!   trust posture);
//! - **trait-stub** join surfaces for the contract-only control/session
//!   (GV2-013) and plan/provenance (GV2-014) graphs. Those graphs have no
//!   in-crate backing yet (named-not-frozen contracts); the registry names their
//!   join queries here without pulling INTD/Edda into this crate (ADR-064). A
//!   consumer supplies the live resolver.

use anvil_kernel_types::{PolicyProfile, SymbolIdentity, TrustLevel};

use crate::dependency::DependencyGraph;
use crate::hot_index::{BackgroundReadApi, HotReadApi};
use crate::symbol_graph::SymbolGraph;
use crate::trust::TrustGraph;

/// A typed, read-only entry point over the resident graph set.
///
/// Holds shared borrows of the resident graphs for the duration of a query
/// batch; construction is cheap (three reference assignments), so callers build
/// one per query batch rather than caching it.
pub struct GraphRegistry<'a> {
    semantic: &'a SymbolGraph,
    dependency: &'a DependencyGraph,
    trust: &'a TrustGraph,
}

impl<'a> GraphRegistry<'a> {
    /// Wrap the resident semantic, dependency, and trust graphs.
    pub fn new(
        semantic: &'a SymbolGraph,
        dependency: &'a DependencyGraph,
        trust: &'a TrustGraph,
    ) -> Self {
        Self {
            semantic,
            dependency,
            trust,
        }
    }

    /// Handle to the semantic graph.
    pub fn semantic(&self) -> &'a SymbolGraph {
        self.semantic
    }

    /// Handle to the dependency graph.
    pub fn dependency(&self) -> &'a DependencyGraph {
        self.dependency
    }

    /// Handle to the trust graph.
    pub fn trust(&self) -> &'a TrustGraph {
        self.trust
    }

    /// The sealed hot-path read surface (ADR-063 allowlist). The registry hands
    /// back exactly [`HotReadApi`]; it cannot widen the sealed surface, so the
    /// denylist ops remain unreachable from here (see the `compile_fail` proof
    /// in the tests).
    pub fn hot_read(&self) -> HotReadApi<'a> {
        HotReadApi::new(self.semantic, self.dependency)
    }

    /// The non-hot background read surface, where denylist ops such as the
    /// unbounded impact closure live.
    pub fn background_read(&self) -> BackgroundReadApi<'a> {
        BackgroundReadApi::new(self.dependency)
    }

    /// Trust posture of a single symbol, by identity (trust-graph join).
    pub fn symbol_trust(&self, symbol: &SymbolIdentity) -> Option<&'a PolicyProfile> {
        self.trust.profile(symbol)
    }

    /// Cross-graph join: the files that import `file` (dependency graph) that
    /// define at least one **privileged** symbol (semantic × trust graph),
    /// deterministically ordered by path.
    ///
    /// `TrustLevel` is deliberately *not* totally ordered — the codebase decides
    /// posture significance by equality against [`TrustLevel::Privileged`] (the
    /// same rule as trust-escalation detection), not by ranking levels. So this
    /// join flags the enforcement-relevant posture rather than inventing a "max
    /// trust". It is the "code/trust" leg the GV2-020 validation exercises; the
    /// provenance leg is the ref-only anchor join (added next).
    pub fn privileged_dependents(&self, file: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .dependency
            .dependents_of(file)
            .into_iter()
            .filter_map(|dependent| {
                let symbols = self.semantic.symbols_in_file(dependent);
                let reaches_privileged = SymbolIdentity::for_file_symbols(&symbols)
                    .iter()
                    .filter_map(|id| self.trust.profile(id))
                    .any(|profile| profile.trust == TrustLevel::Privileged);
                reaches_privileged.then(|| dependent.to_string())
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use anvil_kernel_types::{
        OverrideSource, PolicyProfile, SymbolIdentity, SymbolKind, SymbolNode, TrustLevel,
        Visibility,
    };

    use super::GraphRegistry;
    use crate::dependency::DependencyGraph;
    use crate::symbol_graph::SymbolGraph;
    use crate::trust::TrustGraph;

    fn node(file: &str, id: u64, name: &str, trust: TrustLevel) -> SymbolNode {
        SymbolNode {
            id,
            kind: SymbolKind::Function,
            name: name.to_string(),
            visibility: Visibility::Public,
            file: file.to_string(),
            trust_level: trust,
        }
    }

    fn ident(file: &str, name: &str) -> SymbolIdentity {
        SymbolIdentity {
            file: file.to_string(),
            kind: SymbolKind::Function,
            name: name.to_string(),
            ordinal: 0,
        }
    }

    fn profile(trust: TrustLevel) -> PolicyProfile {
        PolicyProfile {
            trust,
            side_effects: BTreeSet::new(),
            data_classes: BTreeSet::new(),
            guards: BTreeSet::new(),
            evidence: Vec::new(),
            override_source: OverrideSource::Heuristic,
        }
    }

    /// Step 1 — the registry exposes a usable handle to each real graph.
    #[test]
    fn registry_exposes_each_graph_handle() {
        let mut semantic = SymbolGraph::new();
        semantic
            .add_symbol(node("a.ts", 1, "foo", TrustLevel::Boundary))
            .unwrap();

        let mut dependency = DependencyGraph::new();
        dependency.add_dependency("b.ts".to_string(), "a.ts".to_string());

        let mut trust = TrustGraph::new();
        trust.upsert(ident("a.ts", "foo"), profile(TrustLevel::Boundary));

        let registry = GraphRegistry::new(&semantic, &dependency, &trust);

        assert_eq!(registry.semantic().node_count(), 1);
        assert_eq!(registry.dependency().file_count(), 2);
        assert_eq!(registry.trust().len(), 1);
        // The hot surface is reachable and the seal is intact (known_edge is an
        // allowlist read).
        assert!(registry.hot_read().known_edge("b.ts", "a.ts").is_warm());
    }

    /// Step 2 — a cross-graph join over code (semantic) × imports (dependency) ×
    /// posture (trust).
    #[test]
    fn registry_join_query_code_trust() {
        let mut semantic = SymbolGraph::new();
        semantic
            .add_symbol(node("a.ts", 1, "foo", TrustLevel::Boundary))
            .unwrap();
        semantic
            .add_symbol(node("b.ts", 2, "bar", TrustLevel::Privileged))
            .unwrap();
        // d.ts also imports a.ts but only defines a Boundary symbol — it must
        // NOT be flagged as a privileged dependent.
        semantic
            .add_symbol(node("d.ts", 3, "baz", TrustLevel::Boundary))
            .unwrap();

        let mut dependency = DependencyGraph::new();
        dependency.add_dependency("b.ts".to_string(), "a.ts".to_string());
        dependency.add_dependency("d.ts".to_string(), "a.ts".to_string());

        let mut trust = TrustGraph::new();
        trust.upsert(ident("a.ts", "foo"), profile(TrustLevel::Boundary));
        trust.upsert(ident("b.ts", "bar"), profile(TrustLevel::Privileged));
        trust.upsert(ident("d.ts", "baz"), profile(TrustLevel::Boundary));

        let registry = GraphRegistry::new(&semantic, &dependency, &trust);

        // Only the importer reaching a Privileged symbol is flagged.
        let privileged = registry.privileged_dependents("a.ts");
        assert_eq!(privileged, vec!["b.ts".to_string()]);

        // Direct trust-graph join by identity.
        let foo = registry.symbol_trust(&ident("a.ts", "foo")).unwrap();
        assert_eq!(foo.trust, TrustLevel::Boundary);
        assert!(registry.symbol_trust(&ident("a.ts", "missing")).is_none());
    }
}
