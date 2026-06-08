use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use anvil_kernel_types::{
    EvidenceKind, ImportEdge, OverrideSource, PolicyEvidence, PolicyProfile, SideEffectSurface,
    SymbolIdentity, SymbolKind, TrustLevel, Visibility,
};

use super::incremental::{GraphDelta, NodeChange};
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

/// Map a privileged import source to the side-effect surface it reaches.
///
/// Token extraction mirrors [`is_privileged_import`] exactly (strip `node:`,
/// take the segment before the first `/`), so the two stay consistent: any
/// source that `is_privileged_import` accepts maps to exactly one surface here.
fn import_surface(source: &str) -> Option<SideEffectSurface> {
    let module = source.strip_prefix("node:").unwrap_or(source);
    let token = module.split('/').next().unwrap_or(module);
    match token {
        "fs" => Some(SideEffectSurface::Filesystem),
        "child_process" => Some(SideEffectSurface::Process),
        "net" | "http" | "https" => Some(SideEffectSurface::Network),
        "crypto" => Some(SideEffectSurface::Crypto),
        _ => None,
    }
}

/// The trust/policy graph: a join-by-identity store of [`PolicyProfile`] keyed
/// on [`SymbolIdentity`] (GV2-012).
///
/// Owns trust/policy verdicts only. It **joins** to the semantic graph by
/// symbol identity and never embeds a semantic node — the contract boundary the
/// spine spec pins ("the raw semantic graph ... it _joins_ to it via symbol
/// identity"). Backed by a [`BTreeMap`] so iteration and posture diffs are
/// deterministic (Anvil determinism invariant).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrustGraph {
    profiles: BTreeMap<SymbolIdentity, PolicyProfile>,
}

impl TrustGraph {
    /// An empty trust graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace the profile for `symbol`.
    pub fn upsert(&mut self, symbol: SymbolIdentity, profile: PolicyProfile) {
        self.profiles.insert(symbol, profile);
    }

    /// The profile joined to `symbol`, if any. This is the semantic↔trust join:
    /// follow a semantic symbol's identity into its policy verdict.
    #[must_use]
    pub fn profile(&self, symbol: &SymbolIdentity) -> Option<&PolicyProfile> {
        self.profiles.get(symbol)
    }

    /// Number of classified symbols.
    #[must_use]
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// Whether any symbol is classified.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Profiles in deterministic identity order.
    pub fn iter(&self) -> impl Iterator<Item = (&SymbolIdentity, &PolicyProfile)> {
        self.profiles.iter()
    }

    /// Diff two trust graphs into the set of trust-posture changes between them.
    ///
    /// The result is the trust graph's own delta: classification gained, profile
    /// changed, or classification removed — the "trust posture changes are
    /// emitted as graph deltas" contract. Changes are returned in deterministic
    /// identity order; identical profiles emit nothing.
    #[must_use]
    pub fn posture_delta(before: &TrustGraph, after: &TrustGraph) -> Vec<TrustPostureChange> {
        let mut keys: BTreeSet<&SymbolIdentity> = before.profiles.keys().collect();
        keys.extend(after.profiles.keys());

        let mut changes = Vec::new();
        for key in keys {
            match (before.profiles.get(key), after.profiles.get(key)) {
                (None, Some(after_profile)) => changes.push(TrustPostureChange::Classified {
                    symbol: key.clone(),
                    profile: after_profile.clone(),
                }),
                (Some(before_profile), None) => changes.push(TrustPostureChange::Declassified {
                    symbol: key.clone(),
                    profile: before_profile.clone(),
                }),
                (Some(before_profile), Some(after_profile)) if before_profile != after_profile => {
                    changes.push(TrustPostureChange::Reclassified {
                        symbol: key.clone(),
                        before: before_profile.clone(),
                        after: after_profile.clone(),
                    });
                }
                _ => {}
            }
        }
        changes
    }

    /// The trust-posture changes implied by a semantic graph delta.
    ///
    /// Walks the symbols the semantic update touched (`delta.node_changes`) and
    /// reports how each one's posture moved between `before` and `after`, so a
    /// save-time graph update emits exactly the trust posture changes for the
    /// symbols that moved. Iterates the delta's identities directly (not the
    /// full graph diff), so cost scales with the delta, not the graph size.
    ///
    /// This is the *semantic-update-driven* path. A trust change with no
    /// corresponding semantic change — a configuration or baseline override that
    /// reclassifies a symbol the parser did not touch — will not appear in
    /// `node_changes`; emit those via [`posture_delta`](Self::posture_delta) over
    /// the whole trust graph instead.
    #[must_use]
    pub fn posture_changes_for_delta(
        delta: &GraphDelta,
        before: &TrustGraph,
        after: &TrustGraph,
    ) -> Vec<TrustPostureChange> {
        // Dedup + order the touched identities so the output is deterministic
        // even if `node_changes` repeats or reorders an identity.
        let affected: BTreeSet<&SymbolIdentity> = delta
            .node_changes
            .iter()
            .map(NodeChange::identity)
            .collect();

        let mut changes = Vec::new();
        for symbol in affected {
            match (before.profiles.get(symbol), after.profiles.get(symbol)) {
                (None, Some(after_profile)) => changes.push(TrustPostureChange::Classified {
                    symbol: symbol.clone(),
                    profile: after_profile.clone(),
                }),
                (Some(before_profile), None) => changes.push(TrustPostureChange::Declassified {
                    symbol: symbol.clone(),
                    profile: before_profile.clone(),
                }),
                (Some(before_profile), Some(after_profile)) if before_profile != after_profile => {
                    changes.push(TrustPostureChange::Reclassified {
                        symbol: symbol.clone(),
                        before: before_profile.clone(),
                        after: after_profile.clone(),
                    });
                }
                _ => {}
            }
        }
        changes
    }
}

/// One trust-posture change emitted by a trust-graph diff.
///
/// Serialisable so the daemon trust-annotation wiring (GV2-029) can emit posture
/// changes on its notification path without a later breaking addition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustPostureChange {
    /// A symbol gained a policy profile (none existed before).
    Classified {
        symbol: SymbolIdentity,
        profile: PolicyProfile,
    },
    /// An existing symbol's policy profile changed.
    Reclassified {
        symbol: SymbolIdentity,
        before: PolicyProfile,
        after: PolicyProfile,
    },
    /// A symbol's policy profile was removed.
    Declassified {
        symbol: SymbolIdentity,
        profile: PolicyProfile,
    },
}

impl TrustPostureChange {
    /// The symbol this change anchors to.
    #[must_use]
    pub fn symbol(&self) -> &SymbolIdentity {
        match self {
            TrustPostureChange::Classified { symbol, .. }
            | TrustPostureChange::Reclassified { symbol, .. }
            | TrustPostureChange::Declassified { symbol, .. } => symbol,
        }
    }

    /// Whether this change moves the symbol onto `TrustLevel::Privileged` from a
    /// non-privileged posture — the escalation the `PrivilegeExpansion`
    /// invariant cares about.
    #[must_use]
    pub fn is_privilege_escalation(&self) -> bool {
        match self {
            TrustPostureChange::Classified { profile, .. } => {
                profile.trust == TrustLevel::Privileged
            }
            TrustPostureChange::Reclassified { before, after, .. } => {
                after.trust == TrustLevel::Privileged && before.trust != TrustLevel::Privileged
            }
            TrustPostureChange::Declassified { .. } => false,
        }
    }
}

/// Build a trust/policy graph for `graph` from the same bounded import
/// heuristic [`annotate_trust`] uses (GV2-012 producer).
///
/// Determines, per file, the side-effect surfaces reachable through its
/// privileged imports, then attaches a [`PolicyProfile`] to every symbol of
/// that file: the symbol's current `trust_level` plus those surfaces, with one
/// [`PolicyEvidence`] record per asserted fact so each verdict resolves back to
/// the symbol's file. Bounded and local — no interprocedural data-flow
/// (GV2-012 scope guard). Spans stay `None` until a span-producing pass exists
/// (ADR-075 A′ slice). Run [`annotate_trust`] first if you want heuristic trust
/// levels reflected.
#[must_use]
pub fn policy_profiles(graph: &SymbolGraph, imports: &[ImportEdge]) -> TrustGraph {
    // file -> the surfaces its privileged imports reach.
    let mut file_surfaces: BTreeMap<&str, BTreeSet<SideEffectSurface>> = BTreeMap::new();
    for import in imports {
        if let Some(surface) = import_surface(&import.to_source) {
            file_surfaces
                .entry(&import.from_file)
                .or_default()
                .insert(surface);
        }
    }

    // Distinct files in deterministic order.
    let files: BTreeSet<String> = graph
        .inner()
        .node_weights()
        .map(|n| n.file.clone())
        .collect();

    let mut trust_graph = TrustGraph::new();
    for file in &files {
        let symbols = graph.symbols_in_file(file);
        let identities = SymbolIdentity::for_file_symbols(&symbols);
        let surfaces = file_surfaces
            .get(file.as_str())
            .cloned()
            .unwrap_or_default();

        for (node, identity) in symbols.iter().zip(identities) {
            // Skip *synthetic external* module placeholders only — the on-demand
            // nodes `resolve_import` creates for non-relative imports, where
            // `name == file == specifier` (e.g. `"node:fs"`, `"axios"`;
            // `incremental::resolve_import`). They are dependency-graph
            // infrastructure, not source symbols, so they carry no trust verdict.
            // A real source module (Rust `mod foo`, a TS namespace) has
            // `name != file` and is classified normally.
            if node.kind == SymbolKind::Module && node.name == node.file {
                continue;
            }

            // Evidence is built in canonical order — the `Trust` record first,
            // then one `SideEffect` per surface in `BTreeSet` order — so a
            // profile produced from the same inputs is byte-identical across
            // runs (Anvil determinism invariant).
            let mut evidence = vec![PolicyEvidence {
                symbol: identity.clone(),
                kind: EvidenceKind::Trust(node.trust_level),
                source: OverrideSource::Heuristic,
                span: None,
            }];
            for surface in &surfaces {
                evidence.push(PolicyEvidence {
                    symbol: identity.clone(),
                    kind: EvidenceKind::SideEffect(*surface),
                    source: OverrideSource::Heuristic,
                    span: None,
                });
            }

            let profile = PolicyProfile {
                trust: node.trust_level,
                side_effects: surfaces.clone(),
                data_classes: BTreeSet::new(),
                guards: BTreeSet::new(),
                evidence,
                override_source: OverrideSource::Heuristic,
            };
            trust_graph.upsert(identity, profile);
        }
    }
    trust_graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{ByteRange, OverrideSource, SymbolKind, SymbolNode};

    fn ident(file: &str, name: &str) -> SymbolIdentity {
        SymbolIdentity {
            file: file.to_string(),
            kind: SymbolKind::Function,
            name: name.to_string(),
            ordinal: 0,
        }
    }

    fn profile_with_trust(trust: TrustLevel) -> PolicyProfile {
        PolicyProfile {
            trust,
            ..PolicyProfile::default()
        }
    }

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

    // --- Trust/policy graph contract (GV2-012) ---

    #[test]
    fn trust_graph_evidence_resolves_to_source_span() {
        let id = ident("src/pay.ts", "chargeCard");
        let span = ByteRange { start: 5, end: 20 };
        let profile = PolicyProfile {
            trust: TrustLevel::Boundary,
            evidence: vec![PolicyEvidence {
                symbol: id.clone(),
                kind: EvidenceKind::SideEffect(SideEffectSurface::Network),
                source: OverrideSource::Heuristic,
                span: Some(span),
            }],
            ..PolicyProfile::default()
        };

        let mut tg = TrustGraph::new();
        tg.upsert(id.clone(), profile);

        let resolved = tg.profile(&id).expect("profile present");
        let loc = resolved.evidence[0].resolve();
        assert_eq!(
            loc.file, "src/pay.ts",
            "evidence resolves to the symbol's file"
        );
        assert_eq!(loc.span, Some(span), "and to the recorded byte span");
    }

    #[test]
    fn trust_graph_join_is_by_identity_not_node() {
        // The store keys on SymbolIdentity; a profile is found by the same
        // identity a semantic symbol would produce — the semantic↔trust join.
        let id = ident("a.ts", "f");
        let mut tg = TrustGraph::new();
        tg.upsert(id.clone(), profile_with_trust(TrustLevel::Internal));

        assert!(tg.profile(&id).is_some());
        assert!(
            tg.profile(&ident("a.ts", "other")).is_none(),
            "a different identity does not collide"
        );
    }

    #[test]
    fn trust_graph_posture_delta_emits_privilege_escalation() {
        let id = ident("a.ts", "readFile");
        let mut before = TrustGraph::new();
        before.upsert(id.clone(), profile_with_trust(TrustLevel::Boundary));
        let mut after = TrustGraph::new();
        after.upsert(id.clone(), profile_with_trust(TrustLevel::Privileged));

        let changes = TrustGraph::posture_delta(&before, &after);
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            TrustPostureChange::Reclassified {
                symbol,
                before,
                after,
            } => {
                assert_eq!(symbol, &id);
                assert_eq!(before.trust, TrustLevel::Boundary);
                assert_eq!(after.trust, TrustLevel::Privileged);
            }
            other => panic!("expected Reclassified, got {other:?}"),
        }
        assert!(
            changes[0].is_privilege_escalation(),
            "Boundary → Privileged is a privilege escalation"
        );
    }

    #[test]
    fn trust_graph_posture_delta_classifies_and_declassifies() {
        let id = ident("a.ts", "f");
        let mut populated = TrustGraph::new();
        populated.upsert(id.clone(), profile_with_trust(TrustLevel::Internal));
        let empty = TrustGraph::new();

        let classified = TrustGraph::posture_delta(&empty, &populated);
        assert!(matches!(
            classified.as_slice(),
            [TrustPostureChange::Classified { symbol, .. }] if symbol == &id
        ));

        let declassified = TrustGraph::posture_delta(&populated, &empty);
        assert!(matches!(
            declassified.as_slice(),
            [TrustPostureChange::Declassified { symbol, .. }] if symbol == &id
        ));
    }

    #[test]
    fn trust_graph_posture_delta_skips_unchanged_and_is_ordered() {
        let mut before = TrustGraph::new();
        before.upsert(ident("a.ts", "a"), profile_with_trust(TrustLevel::Internal));
        before.upsert(ident("a.ts", "b"), profile_with_trust(TrustLevel::Internal));

        let mut after = TrustGraph::new();
        // `a` unchanged, `b` escalates.
        after.upsert(ident("a.ts", "a"), profile_with_trust(TrustLevel::Internal));
        after.upsert(
            ident("a.ts", "b"),
            profile_with_trust(TrustLevel::Privileged),
        );

        let changes = TrustGraph::posture_delta(&before, &after);
        assert_eq!(changes.len(), 1, "unchanged symbols emit nothing");
        assert_eq!(changes[0].symbol(), &ident("a.ts", "b"));

        // Deterministic: a second run yields the identical sequence.
        assert_eq!(changes, TrustGraph::posture_delta(&before, &after));
    }

    #[test]
    fn trust_graph_posture_changes_scoped_to_graph_delta() {
        let touched = ident("a.ts", "touched");
        let untouched = ident("a.ts", "untouched");

        let before = TrustGraph::new();
        let mut after = TrustGraph::new();
        after.upsert(touched.clone(), profile_with_trust(TrustLevel::Privileged));
        after.upsert(
            untouched.clone(),
            profile_with_trust(TrustLevel::Privileged),
        );

        // The semantic delta only reports `touched` as changed.
        let delta = GraphDelta {
            node_changes: vec![NodeChange::Changed(touched.clone())],
            ..GraphDelta::default()
        };

        let scoped = TrustGraph::posture_changes_for_delta(&delta, &before, &after);
        assert_eq!(scoped.len(), 1, "only the delta's symbols surface");
        assert_eq!(scoped[0].symbol(), &touched);
    }

    #[test]
    fn trust_graph_producer_attaches_privileged_surface() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "readFile", "a.ts", Visibility::Public))
            .unwrap();
        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "node:fs".to_string(),
            line: 0,
        }];

        annotate_trust(&mut g, &imports);
        let tg = policy_profiles(&g, &imports);

        let id = ident("a.ts", "readFile");
        let profile = tg.profile(&id).expect("profile produced for the symbol");
        assert_eq!(profile.trust, TrustLevel::Privileged);
        assert!(
            profile
                .side_effects
                .contains(&SideEffectSurface::Filesystem),
            "fs import yields the Filesystem surface"
        );
        assert!(
            profile.evidence.iter().all(|e| e.resolve().file == "a.ts"),
            "every evidence record resolves back to the symbol's file"
        );
    }

    #[test]
    fn trust_graph_producer_change_emits_posture_delta() {
        // A file with no privileged import, then the same file after a `node:fs`
        // import is added: the producer's two trust graphs diff to an escalation.
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "doWork", "a.ts", Visibility::Internal))
            .unwrap();

        annotate_trust(&mut g, &[]);
        let before = policy_profiles(&g, &[]);

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "node:fs".to_string(),
            line: 0,
        }];
        annotate_trust(&mut g, &imports);
        let after = policy_profiles(&g, &imports);

        let changes = TrustGraph::posture_delta(&before, &after);
        assert_eq!(changes.len(), 1);
        assert!(
            changes[0].is_privilege_escalation(),
            "adding a privileged import escalates the symbol's posture"
        );
    }

    #[test]
    fn trust_graph_producer_skips_only_synthetic_module_nodes() {
        let mut g = SymbolGraph::new();
        // A real source symbol.
        g.add_symbol(make_symbol(1, "handler", "a.ts", Visibility::Public))
            .unwrap();
        // A synthetic external module placeholder (name == file == specifier),
        // as `resolve_import` creates and `annotate_trust` preserves.
        let mut synthetic = make_symbol(2, "axios", "axios", Visibility::Public);
        synthetic.kind = SymbolKind::Module;
        g.add_symbol(synthetic).unwrap();
        // A REAL source module symbol (Rust `mod foo`): kind Module, but
        // name != file. Must NOT be skipped (Copilot review feedback).
        let mut real_module = make_symbol(3, "foo", "src/lib.rs", Visibility::Public);
        real_module.kind = SymbolKind::Module;
        g.add_symbol(real_module).unwrap();

        let tg = policy_profiles(&g, &[]);

        assert!(tg.profile(&ident("a.ts", "handler")).is_some());
        // Real source module is classified.
        let real_id = SymbolIdentity {
            file: "src/lib.rs".to_string(),
            kind: SymbolKind::Module,
            name: "foo".to_string(),
            ordinal: 0,
        };
        assert!(
            tg.profile(&real_id).is_some(),
            "a real source module (name != file) must be classified"
        );
        // Synthetic external placeholder is skipped.
        let synthetic_id = SymbolIdentity {
            file: "axios".to_string(),
            kind: SymbolKind::Module,
            name: "axios".to_string(),
            ordinal: 0,
        };
        assert!(
            tg.profile(&synthetic_id).is_none(),
            "synthetic external module placeholders carry no policy verdict"
        );
        assert_eq!(
            tg.len(),
            2,
            "the real symbol and the real module, not the placeholder"
        );
    }

    #[test]
    fn trust_graph_import_surface_covers_all_privileged_modules() {
        // Every privileged module maps to exactly one surface, so the two
        // heuristics (`is_privileged_import` / `import_surface`) cannot drift.
        for module in PRIVILEGED_MODULES {
            assert!(
                import_surface(module).is_some(),
                "privileged module {module:?} has no side-effect surface"
            );
            let prefixed = format!("node:{module}");
            assert_eq!(
                import_surface(&prefixed),
                import_surface(module),
                "node: prefix must resolve to the same surface"
            );
        }
    }

    #[test]
    fn trust_graph_posture_changes_for_delta_scopes_to_semantic_changes() {
        let touched = ident("a.ts", "touched");
        let untouched = ident("a.ts", "untouched");

        let mut before = TrustGraph::new();
        before.upsert(untouched.clone(), profile_with_trust(TrustLevel::Boundary));
        let mut after = TrustGraph::new();
        after.upsert(touched.clone(), profile_with_trust(TrustLevel::Privileged));
        // `untouched` also moved (a config/baseline-style change), but the
        // semantic delta does not name it.
        after.upsert(
            untouched.clone(),
            profile_with_trust(TrustLevel::Privileged),
        );

        let delta = GraphDelta {
            node_changes: vec![NodeChange::Changed(touched.clone())],
            ..GraphDelta::default()
        };

        let scoped = TrustGraph::posture_changes_for_delta(&delta, &before, &after);
        assert_eq!(scoped.len(), 1, "only the semantic delta's symbols surface");
        assert_eq!(scoped[0].symbol(), &touched);

        // The whole-graph diff DOES surface the change the semantic delta missed.
        assert_eq!(
            TrustGraph::posture_delta(&before, &after).len(),
            2,
            "posture_delta sees the config-driven change too"
        );
    }
}
