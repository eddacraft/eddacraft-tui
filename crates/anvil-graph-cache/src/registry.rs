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

/// A ref-only provenance anchor (GV2-014): an opaque reference joining a symbol
/// to a plan/provenance authority. Bodies stay in the authoritative store
/// (TS-side Edda); only the anchor crosses to Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceAnchor {
    /// An APS work-item id (e.g. `GV2-020`).
    ApsItem(String),
    /// A commit SHA.
    Commit(String),
    /// An opaque Edda memory ref.
    Memory(String),
}

/// Resolves provenance anchors for a symbol (the GV2-014 join). Implemented by a
/// consumer that holds the live plan/provenance authority; the registry stays
/// free of that dependency (ADR-064). Use [`NoProvenance`] when none is wired.
pub trait ProvenanceJoin {
    /// The provenance anchors for `symbol`, if any.
    fn anchors_for(&self, symbol: &SymbolIdentity) -> Vec<ProvenanceAnchor>;
}

/// Resolves the controlling session/attribution for a workspace-relative file
/// (the GV2-013 join). Implemented by the control authority (the intercept
/// daemon's session registry); the registry stays free of the daemon proto
/// types — the ref is opaque. Use [`NoControl`] when none is wired.
pub trait ControlJoin {
    /// An opaque session/attribution ref controlling `file`, if any.
    fn attribution_for(&self, file: &str) -> Option<String>;
}

/// The default no-op provenance resolver — no plan/provenance authority wired.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoProvenance;

impl ProvenanceJoin for NoProvenance {
    fn anchors_for(&self, _symbol: &SymbolIdentity) -> Vec<ProvenanceAnchor> {
        Vec::new()
    }
}

/// The default no-op control resolver — no control authority wired.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoControl;

impl ControlJoin for NoControl {
    fn attribution_for(&self, _file: &str) -> Option<String> {
        None
    }
}

/// A joined view of one symbol across the code (semantic), trust, and
/// plan/provenance graphs — the worked "code/trust/provenance" join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolJoin {
    /// The symbol's identity.
    pub identity: SymbolIdentity,
    /// Whether the symbol is resident in the semantic graph.
    pub defined: bool,
    /// Its trust posture, if the trust graph carries a profile for it.
    pub trust: Option<TrustLevel>,
    /// Ref-only provenance anchors, resolved via the injected provenance join.
    pub provenance: Vec<ProvenanceAnchor>,
}

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
    /// denylist ops stay unreachable from here.
    ///
    /// A denylist op (the unbounded impact closure) does **not** compile from the
    /// hot surface returned by the registry:
    ///
    /// ```compile_fail
    /// use anvil_graph_cache::{DependencyGraph, GraphRegistry, SymbolGraph, TrustGraph};
    /// let (s, d, t) = (SymbolGraph::new(), DependencyGraph::new(), TrustGraph::new());
    /// let registry = GraphRegistry::new(&s, &d, &t);
    /// // `impact_closure_unbounded` lives on `BackgroundReadApi`, not the hot surface:
    /// let _ = registry.hot_read().impact_closure_unbounded("a.ts", 100);
    /// ```
    ///
    /// It compiles from the background surface, which is where denylist ops live:
    ///
    /// ```
    /// use anvil_graph_cache::{DependencyGraph, GraphRegistry, SymbolGraph, TrustGraph};
    /// let (s, d, t) = (SymbolGraph::new(), DependencyGraph::new(), TrustGraph::new());
    /// let registry = GraphRegistry::new(&s, &d, &t);
    /// let _ = registry.background_read().impact_closure_unbounded("a.ts", 100);
    /// ```
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

    /// Join one symbol across code (semantic residency), trust (posture), and
    /// plan/provenance (ref-only anchors). The provenance leg is resolved through
    /// the injected `provenance` join; pass [`NoProvenance`] when no
    /// plan/provenance authority is wired.
    ///
    /// The residency check walks the file's symbol identities once per call, so
    /// this is not hot-path-admissible when looped over every symbol in a large
    /// file — a future batch variant should compute the file's identities once.
    pub fn symbol_join<P: ProvenanceJoin>(
        &self,
        provenance: &P,
        symbol: &SymbolIdentity,
    ) -> SymbolJoin {
        let defined =
            SymbolIdentity::for_file_symbols(&self.semantic.symbols_in_file(&symbol.file))
                .iter()
                .any(|resident| resident == symbol);
        SymbolJoin {
            identity: symbol.clone(),
            defined,
            trust: self.trust.profile(symbol).map(|profile| profile.trust),
            provenance: provenance.anchors_for(symbol),
        }
    }

    /// Control/session join: the opaque attribution ref controlling a
    /// workspace-relative `file`, via the injected `control` join. Pass
    /// [`NoControl`] when no control authority is wired. (Callers relativise an
    /// absolute path to the `file` key with [`anvil_kernel_types::WorkspaceRoot`].)
    pub fn attribution_for<C: ControlJoin>(&self, control: &C, file: &str) -> Option<String> {
        control.attribution_for(file)
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
        // `dependents_of` returns from a `HashSet` (no duplicates); sort only —
        // for deterministic ordering, since set iteration order is unspecified.
        out.sort();
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

    use super::{
        ControlJoin, GraphRegistry, NoControl, NoProvenance, ProvenanceAnchor, ProvenanceJoin,
    };
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
            span: None,
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

    /// Step 3 — one join query spanning code (semantic residency), trust
    /// (posture), and plan/provenance (ref-only anchors via an injected join).
    #[test]
    fn registry_join_query_spans_code_trust_provenance() {
        struct StubProvenance;
        impl ProvenanceJoin for StubProvenance {
            fn anchors_for(&self, symbol: &SymbolIdentity) -> Vec<ProvenanceAnchor> {
                if symbol.name == "foo" {
                    vec![ProvenanceAnchor::ApsItem("GV2-020".to_string())]
                } else {
                    Vec::new()
                }
            }
        }

        let mut semantic = SymbolGraph::new();
        semantic
            .add_symbol(node("a.ts", 1, "foo", TrustLevel::Boundary))
            .unwrap();
        let dependency = DependencyGraph::new();
        let mut trust = TrustGraph::new();
        trust.upsert(ident("a.ts", "foo"), profile(TrustLevel::Boundary));

        let registry = GraphRegistry::new(&semantic, &dependency, &trust);

        let join = registry.symbol_join(&StubProvenance, &ident("a.ts", "foo"));
        assert!(join.defined); // code: resident in the semantic graph
        assert_eq!(join.trust, Some(TrustLevel::Boundary)); // trust: posture
        assert_eq!(
            join.provenance,
            vec![ProvenanceAnchor::ApsItem("GV2-020".to_string())]
        ); // provenance: ref-only anchor

        // The default no-op resolver yields no provenance; an unknown symbol is
        // not defined and has no posture.
        let empty = registry.symbol_join(&NoProvenance, &ident("a.ts", "missing"));
        assert!(!empty.defined);
        assert!(empty.trust.is_none());
        assert!(empty.provenance.is_empty());
    }

    /// Step 4 — the control/session join is a trait stub: no authority wired
    /// yields no attribution; an injected authority resolves it. (The contract-
    /// only control graph has no in-crate backing yet — GV2-013.)
    #[test]
    fn registry_control_join_is_a_resolver_seam() {
        struct StubControl;
        impl ControlJoin for StubControl {
            fn attribution_for(&self, file: &str) -> Option<String> {
                (file == "src/a.ts").then(|| "session:abc".to_string())
            }
        }

        let semantic = SymbolGraph::new();
        let dependency = DependencyGraph::new();
        let trust = TrustGraph::new();
        let registry = GraphRegistry::new(&semantic, &dependency, &trust);

        assert!(registry.attribution_for(&NoControl, "src/a.ts").is_none());
        assert_eq!(
            registry.attribution_for(&StubControl, "src/a.ts"),
            Some("session:abc".to_string())
        );
        assert!(registry.attribution_for(&StubControl, "src/b.ts").is_none());
    }

    /// `symbol_join` resolves overloaded symbols by ordinal — two same-(file,
    /// kind, name) symbols are distinct identities; the residency check must
    /// honour the ordinal, not just the name.
    #[test]
    fn registry_symbol_join_disambiguates_overloads_by_ordinal() {
        let mut semantic = SymbolGraph::new();
        // Two overloads of `foo` in one file → ordinals 0 and 1.
        semantic
            .add_symbol(node("a.ts", 1, "foo", TrustLevel::Unknown))
            .unwrap();
        semantic
            .add_symbol(node("a.ts", 2, "foo", TrustLevel::Unknown))
            .unwrap();
        let dependency = DependencyGraph::new();
        let trust = TrustGraph::new();
        let registry = GraphRegistry::new(&semantic, &dependency, &trust);

        let ord = |n: u32| SymbolIdentity {
            file: "a.ts".to_string(),
            kind: SymbolKind::Function,
            name: "foo".to_string(),
            ordinal: n,
        };
        assert!(registry.symbol_join(&NoProvenance, &ord(0)).defined);
        assert!(registry.symbol_join(&NoProvenance, &ord(1)).defined);
        // No third overload — ordinal 2 is not resident.
        assert!(!registry.symbol_join(&NoProvenance, &ord(2)).defined);
    }

    /// Pins the denylist method the seal doctest relies on: `impact_closure_unbounded`
    /// exists on `BackgroundReadApi` with the expected signature. If it were renamed
    /// or removed, the `compile_fail` seal doctest could pass for the wrong reason —
    /// this test would fail to compile, flagging it.
    #[test]
    fn background_read_exposes_unbounded_closure() {
        let semantic = SymbolGraph::new();
        let dependency = DependencyGraph::new();
        let trust = TrustGraph::new();
        let registry = GraphRegistry::new(&semantic, &dependency, &trust);
        let result: Option<std::collections::HashSet<String>> = registry
            .background_read()
            .impact_closure_unbounded("a.ts", 100);
        // Pins the method name + signature; an unknown file yields an empty (or
        // absent) closure — the value is incidental, the point is it compiles.
        assert!(result.is_none_or(|closure| closure.is_empty()));
    }
}
