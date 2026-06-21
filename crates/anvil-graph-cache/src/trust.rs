use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use anvil_kernel_types::{
    EdgeType, EvidenceKind, ImportEdge, OverrideSource, PolicyEvidence, PolicyProfile,
    SideEffectSurface, SymbolIdentity, SymbolKind, TrustLevel, Visibility,
};

use super::incremental::{GraphDelta, NodeChange};
use super::symbol_graph::SymbolGraph;

/// Sensitive module names that indicate privileged access.
/// Matched by exact module token (or `node:` prefix), not substring, to avoid
/// false positives on packages like `fsevents` or `http-errors`.
///
/// CIB-093a: the spawn/exec (`worker_threads`) and sandbox-escape (`vm`, `v8`)
/// built-ins, plus the lower-level network built-ins (`dns`, `tls`, `dgram`),
/// are included alongside the classic capability surfaces — they grant
/// equivalent privilege and previously certified CLEAN. This list is the single
/// source of truth for what the GV2-027 certify path treats as newly privileged
/// (and what GV2-031 follows re-export chains to), so every entry here must also
/// map to exactly one [`SideEffectSurface`] in [`import_surface`] (enforced by
/// `trust_graph_import_surface_covers_all_privileged_modules`).
const PRIVILEGED_MODULES: &[&str] = &[
    "fs",
    "child_process",
    "net",
    "http",
    "https",
    "crypto",
    "worker_threads",
    "vm",
    "v8",
    "dns",
    "tls",
    "dgram",
];

/// External module patterns (not relative imports).
fn is_external_import(source: &str) -> bool {
    !source.starts_with('.') && !source.starts_with('/')
}

/// Reduce an import specifier to the lowercase top-level module token the
/// privileged-module heuristics match against (CIB-093 N3).
///
/// Node's built-in module resolution is case-sensitive on the *resolver*, but a
/// case-insensitive filesystem (macOS/Windows default) still loads the real
/// module for a mis-cased specifier: `import 'FS'` / `'Fs'` resolves `fs`, and
/// `'NODE:fs'` / `'Node:fs'` reaches the `node:`-prefixed built-in. A byte-exact
/// check missed all of these and false-certified CLEAN. Strip a case-insensitive
/// `node:` prefix, take the segment before the first `/` subpath, then lowercase
/// — so the lookup against the (lowercase) `PRIVILEGED_MODULES` table is
/// case-insensitive while a legitimate lowercase specifier is unchanged.
fn privileged_module_token(source: &str) -> String {
    let module = source
        .strip_prefix("node:")
        .or_else(|| {
            // Case-insensitive `node:` prefix strip without allocating on the
            // common lowercase path.
            source
                .get(..5)
                .filter(|p| p.eq_ignore_ascii_case("node:"))
                .map(|_| &source[5..])
        })
        .unwrap_or(source);
    // Only match the top-level module name (before any `/` subpath).
    let token = module.split('/').next().unwrap_or(module);
    token.to_ascii_lowercase()
}

/// Check whether an import source refers to a privileged Node.js module.
/// Matches the bare name (`"fs"`) or the `node:` prefixed form (`"node:fs"`)
/// as an exact token so that unrelated packages (e.g. `fsevents`, `http-errors`)
/// are not misclassified.
///
/// `pub(crate)` so `certify::export_surface_diff` can diff the file's privileged
/// *module* imports directly (GV2-029) — the side-effect-surface dimension that
/// is orthogonal to the symbol-identity trust diff.
pub(crate) fn is_privileged_import(source: &str) -> bool {
    // CIB-093 N3: case-fold the specifier (and the `node:` prefix) before the
    // lookup so a mis-cased import on a case-insensitive filesystem does not
    // false-certify CLEAN. `privileged_module_token` mirrors `import_surface`.
    PRIVILEGED_MODULES.contains(&privileged_module_token(source).as_str())
}

/// Collect the privileged external module specifiers a file reaches through
/// re-export edges, transitively (GV2-031).
///
/// Walks `EdgeType::Reexports` edges starting from `file`'s symbols. A target
/// that is a synthetic external privileged module node (`node:fs`, …, the kind
/// [`resolve_import`](crate::incremental) stamps for a bare specifier) is
/// collected; a target in another *project* file is followed, so a chain
/// `a → b → node:fs` (`b` re-exports `node:fs`, `a` re-exports `b`) attributes
/// the privileged surface to `a` as well as `b`. A wildcard re-export
/// (`export * from …`) carries through unchanged — the carrier is a file→module
/// edge regardless of `exported_name`. The result is a [`BTreeSet`] for
/// deterministic ordering, and a visited-file set bounds re-export cycles.
///
/// Both `annotate_trust` (to stamp the re-exporting file `Privileged`) and
/// `certify::export_surface_diff` (to add the module to the privileged-surface
/// diff) use this, so the trust pass and the certify diff stay consistent — the
/// same shared blind spot the GV2-029 review found in the import path.
pub(crate) fn reexported_privileged_modules(graph: &SymbolGraph, file: &str) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut queue: Vec<String> = vec![file.to_string()];

    while let Some(current) = queue.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        for symbol in graph.symbols_in_file(&current) {
            for edge in graph.outgoing_edges(symbol.id) {
                if edge.edge_type != EdgeType::Reexports {
                    continue;
                }
                let Some(target) = graph.get_symbol(edge.to) else {
                    continue;
                };
                if target.kind == SymbolKind::Module
                    && target.trust_level == TrustLevel::External
                    && is_privileged_import(&target.file)
                {
                    found.insert(target.file.clone());
                } else if !visited.contains(&target.file) {
                    queue.push(target.file.clone());
                }
            }
        }
    }
    found
}

/// The file-level re-export graph used by [`reexport_privileged_files`]: the
/// per-file `reached` seeds (files that re-export a privileged module directly),
/// the reverse `re_exported_by` edges, and the initial propagation worklist.
///
/// `re_exported_by[t]` maps each target file `t` to the set of files that
/// re-export it. It is a [`BTreeSet`] (not a `Vec`) so a **multi-symbol
/// re-exporter** — one file whose several symbols all re-export into the same
/// target file — is recorded exactly ONCE (CIB-093 regression). A `Vec` here
/// accumulated one duplicate entry per re-exporting symbol, which made the
/// backward-propagation step re-process the same source redundantly; the set both
/// deduplicates and keeps a deterministic iteration order.
type ReexportEdges = (
    HashMap<String, BTreeSet<String>>,
    HashMap<String, BTreeSet<String>>,
    Vec<String>,
);

fn build_reexport_edges(graph: &SymbolGraph) -> ReexportEdges {
    // `reached[f]` = privileged specifiers file `f` reaches by re-export.
    let mut reached: HashMap<String, BTreeSet<String>> = HashMap::new();
    // `re_exported_by[t]` = files that re-export file `t` (reverse edges), deduped.
    let mut re_exported_by: HashMap<String, BTreeSet<String>> = HashMap::new();
    // Files whose `reached` set grew and must propagate to their re-exporters.
    let mut worklist: Vec<String> = Vec::new();

    for node in graph.inner().node_weights() {
        for edge in graph.outgoing_edges(node.id) {
            if edge.edge_type != EdgeType::Reexports {
                continue;
            }
            let Some(target) = graph.get_symbol(edge.to) else {
                continue;
            };
            if target.file == node.file {
                continue;
            }
            if target.kind == SymbolKind::Module
                && target.trust_level == TrustLevel::External
                && is_privileged_import(&target.file)
            {
                if reached
                    .entry(node.file.clone())
                    .or_default()
                    .insert(target.file.clone())
                {
                    worklist.push(node.file.clone());
                }
            } else {
                re_exported_by
                    .entry(target.file.clone())
                    .or_default()
                    .insert(node.file.clone());
            }
        }
    }
    (reached, re_exported_by, worklist)
}

/// Per-file privileged module specifiers reached through re-export edges,
/// transitively (GV2-031) — computed in a single graph pass.
///
/// This is the whole-graph companion to [`reexported_privileged_modules`] (which
/// answers the same question for *one* file). `annotate_trust` runs on every
/// save over the whole warm graph, so calling the per-file walk once per file
/// would be O(files²) on a re-export-heavy graph (barrel files). Instead this
/// scans every `Reexports` edge once to build the file-level re-export graph,
/// seeds each file with the privileged specifiers it re-exports directly, then
/// propagates those specifier sets backwards along re-export edges to a fixpoint
/// — O(nodes + re-export edges). A `from_file == to_file` self-edge is ignored
/// (it cannot widen reach).
///
/// The result maps each file to the **set of privileged module specifiers** it
/// reaches (the values [`reexported_privileged_modules`] would return for that
/// file); a file with no privileged re-export reach is absent. CIB-093b memoises
/// this onto the [`SymbolGraph`] so `certify::export_surface_diff` reads the
/// per-file set instead of re-walking the BFS on every `ContentModify` verdict.
fn reexport_privileged_files(graph: &SymbolGraph) -> HashMap<String, BTreeSet<String>> {
    // Build the file-level re-export graph: the direct privileged-specifier seeds
    // and the reverse (`re_exported_by`) edges, deduped per file.
    let (mut reached, re_exported_by, mut worklist) = build_reexport_edges(graph);

    // Backward propagation: a file that re-exports file `f` reaches everything
    // `f` reaches. Propagate specifier sets along reverse edges until no set
    // grows. A file is re-queued only when it gained a new specifier, so the
    // fixpoint terminates (the universe of specifiers is finite) even on a
    // re-export cycle.
    while let Some(file) = worklist.pop() {
        let Some(specs) = reached.get(&file).cloned() else {
            continue;
        };
        if let Some(sources) = re_exported_by.get(&file) {
            // Iterate by reference — only `reached`/`worklist` are mutated in this
            // loop, never `re_exported_by`, so cloning the whole set is avoidable;
            // each `source` is cloned only when an owned key/worklist entry is
            // actually needed.
            for source in sources {
                let entry = reached.entry(source.clone()).or_default();
                let mut grew = false;
                for spec in &specs {
                    if entry.insert(spec.clone()) {
                        grew = true;
                    }
                }
                if grew {
                    worklist.push(source.clone());
                }
            }
        }
    }
    reached
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

    // GV2-031: a file that re-exports a privileged module (directly or through a
    // re-export chain) is privileged too, even though no `ImportEdge` names it.
    // Read off the lifted `Reexports` edges, which `update_file`/
    // `re_resolve_reexports` have already placed in the graph by the time the
    // certify path calls `annotate_trust`. One whole-graph pass, not a per-file
    // walk (see `reexport_privileged_files`).
    let reexport_privileged_map = reexport_privileged_files(graph);
    // The set of files that reach *some* privileged module by re-export — the
    // keys of the per-file map (a file is only inserted when it reaches one).
    let reexport_privileged: HashSet<&str> =
        reexport_privileged_map.keys().map(String::as_str).collect();

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

        let trust = if privileged_files.contains(file.as_str())
            || reexport_privileged.contains(file.as_str())
        {
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

    // CIB-093b: install the per-file re-export-privilege memo computed above so
    // the certify hot path reads it instead of re-walking the per-file BFS. The
    // `get_symbol_mut` loop just cleared any previous memo, so this is the fresh,
    // authoritative set for the post-update graph. `reexport_privileged` (a
    // borrow of the map) is no longer used past the loop, so the map moves in.
    graph.set_reexport_privileged(reexport_privileged_map);
}

/// Map a privileged import source to the side-effect surface it reaches.
///
/// Token extraction mirrors [`is_privileged_import`] exactly (via
/// [`privileged_module_token`]: case-insensitive `node:` strip, segment before
/// the first `/`, lowercased), so the two stay consistent: any source that
/// `is_privileged_import` accepts maps to exactly one surface here.
fn import_surface(source: &str) -> Option<SideEffectSurface> {
    // CIB-093 N3: same case-folding as `is_privileged_import` so the two
    // heuristics cannot drift (the covers-all check pins this).
    match privileged_module_token(source).as_str() {
        "fs" => Some(SideEffectSurface::Filesystem),
        // CIB-093a: worker_threads spawns/execs worker isolates; vm/v8 grant raw
        // code execution / VM-internal access — all process-control capabilities.
        "child_process" | "worker_threads" | "vm" | "v8" => Some(SideEffectSurface::Process),
        // CIB-093a: dns/tls/dgram are lower-level network built-ins alongside the
        // classic net/http/https surface.
        "net" | "http" | "https" | "dns" | "tls" | "dgram" => Some(SideEffectSurface::Network),
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
    fn case_insensitive_specifier_classifies_privileged() {
        // N3 (CIB-093): on a case-insensitive filesystem `import FS from 'FS'`
        // loads the real `fs`, but a byte-exact lookup misses it and certifies
        // CLEAN. The specifier must be lowercased (and a `node:` prefix stripped
        // case-insensitively) before the PRIVILEGED_MODULES lookup.
        for spec in [
            "FS",
            "Fs",
            "NODE:child_process",
            "Node:worker_threads",
            "Node:FS",
            "node:DGRAM",
        ] {
            let mut g = SymbolGraph::new();
            g.add_symbol(make_symbol(1, "f", "a.ts", Visibility::Internal))
                .unwrap();
            let imports = vec![ImportEdge {
                from_file: "a.ts".to_string(),
                to_source: spec.to_string(),
                line: 0,
            }];
            annotate_trust(&mut g, &imports);
            assert_eq!(
                g.get_symbol(1).unwrap().trust_level,
                TrustLevel::Privileged,
                "{spec} must classify Privileged regardless of case"
            );
        }
    }

    #[test]
    fn case_insensitive_specifier_maps_to_surface() {
        // The side-effect-surface map must follow the same case-folding so the
        // two heuristics stay consistent (the import_surface covers-all check).
        assert_eq!(
            import_surface("FS"),
            Some(SideEffectSurface::Filesystem),
            "uppercase fs maps to the filesystem surface"
        );
        assert_eq!(
            import_surface("NODE:child_process"),
            Some(SideEffectSurface::Process),
            "uppercase node:-prefixed child_process maps to the process surface"
        );
    }

    #[test]
    fn lowercase_specifier_still_matches() {
        // Regression guard: case-folding must not break the legitimate lowercase
        // path the import-surface/matching already supports.
        assert!(is_privileged_import("fs"));
        assert!(is_privileged_import("node:fs"));
        assert!(is_privileged_import("node:fs/promises"));
        assert!(!is_privileged_import("fsevents"));
        assert!(!is_privileged_import("express"));
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
    fn spawn_and_sandbox_escape_builtins_are_privileged() {
        // CIB-093a: worker_threads (spawn+exec) and vm/v8 (sandbox escape), plus
        // dns/tls/dgram (network), are spawn/sandbox-escape Node built-ins that
        // previously certified CLEAN. Both bare and `node:`-prefixed forms must
        // classify Privileged.
        for spec in [
            "worker_threads",
            "node:worker_threads",
            "vm",
            "node:vm",
            "v8",
            "node:v8",
            "dns",
            "node:dns",
            "tls",
            "node:tls",
            "dgram",
            "node:dgram",
        ] {
            let mut g = SymbolGraph::new();
            g.add_symbol(make_symbol(1, "f", "a.ts", Visibility::Internal))
                .unwrap();
            let imports = vec![ImportEdge {
                from_file: "a.ts".to_string(),
                to_source: spec.to_string(),
                line: 0,
            }];
            annotate_trust(&mut g, &imports);
            assert_eq!(
                g.get_symbol(1).unwrap().trust_level,
                TrustLevel::Privileged,
                "{spec} should be classified Privileged"
            );
        }
    }

    #[test]
    fn worker_threads_certifies_as_newly_privileged() {
        // CIB-093a: a symbol that imports node:worker_threads must surface as a
        // newly-privileged escalation through the certify side-effect dimension,
        // not certify clean.
        use crate::certify::export_surface_diff;

        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "run", "a.ts", Visibility::Internal))
            .unwrap();
        // The synthetic external module node update_file would stamp for the
        // resolved import target, plus the Imports edge to it.
        g.add_symbol(external_module(2, "node:worker_threads"))
            .unwrap();
        g.add_edge(anvil_kernel_types::SymbolEdge {
            from: 1,
            to: 2,
            edge_type: EdgeType::Imports,
        })
        .unwrap();
        annotate_trust(
            &mut g,
            &[ImportEdge {
                from_file: "a.ts".to_string(),
                to_source: "node:worker_threads".to_string(),
                line: 0,
            }],
        );

        // A fresh file with no prior privileged surface: the new import is newly
        // privileged.
        let delta = GraphDelta {
            file: "a.ts".to_string(),
            ..GraphDelta::default()
        };
        let diff = export_surface_diff(&g, &delta);
        assert!(
            diff.newly_privileged_imports
                .contains(&"node:worker_threads".to_string()),
            "node:worker_threads must be a newly-privileged surface, got {:?}",
            diff.newly_privileged_imports
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

    // --- GV2-031: re-export privilege ---

    fn external_module(id: u64, spec: &str) -> SymbolNode {
        SymbolNode {
            id,
            kind: SymbolKind::Module,
            name: spec.to_string(),
            visibility: Visibility::Public,
            file: spec.to_string(),
            trust_level: TrustLevel::External,
        }
    }

    fn add_reexport(g: &mut SymbolGraph, from: u64, to: u64) {
        g.add_edge(anvil_kernel_types::SymbolEdge {
            from,
            to,
            edge_type: EdgeType::Reexports,
        })
        .unwrap();
    }

    #[test]
    fn direct_reexport_of_privileged_module_marks_file_privileged() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "api", "barrel.ts", Visibility::Public))
            .unwrap();
        g.add_symbol(external_module(2, "node:fs")).unwrap();
        add_reexport(&mut g, 1, 2);

        // No ImportEdge names node:fs — the privilege is reachable only via the
        // re-export edge.
        annotate_trust(&mut g, &[]);

        assert_eq!(
            g.get_symbol(1).unwrap().trust_level,
            TrustLevel::Privileged,
            "a file re-exporting node:fs must be classified Privileged"
        );
    }

    #[test]
    fn transitive_reexport_chain_marks_file_privileged() {
        let mut g = SymbolGraph::new();
        // barrel.ts re-exports mid.ts; mid.ts re-exports node:fs.
        g.add_symbol(make_symbol(1, "api", "barrel.ts", Visibility::Public))
            .unwrap();
        g.add_symbol(make_symbol(2, "mid", "mid.ts", Visibility::Public))
            .unwrap();
        g.add_symbol(external_module(3, "node:fs")).unwrap();
        add_reexport(&mut g, 1, 2);
        add_reexport(&mut g, 2, 3);

        annotate_trust(&mut g, &[]);

        assert_eq!(
            g.get_symbol(1).unwrap().trust_level,
            TrustLevel::Privileged,
            "barrel re-exporting an intermediary that re-exports node:fs is Privileged"
        );
        assert_eq!(
            g.get_symbol(2).unwrap().trust_level,
            TrustLevel::Privileged,
            "the intermediary itself is Privileged"
        );
    }

    #[test]
    fn benign_local_reexport_does_not_mark_file_privileged() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "api", "barrel.ts", Visibility::Public))
            .unwrap();
        g.add_symbol(make_symbol(2, "helper", "local.ts", Visibility::Public))
            .unwrap();
        add_reexport(&mut g, 1, 2);

        annotate_trust(&mut g, &[]);

        // A public symbol with no privileged reach is Boundary, never Privileged.
        assert_eq!(
            g.get_symbol(1).unwrap().trust_level,
            TrustLevel::Boundary,
            "a benign local re-export must not escalate to Privileged"
        );
    }

    #[test]
    fn multi_symbol_reexporter_appears_once_in_propagation_set() {
        // CIB-093 regression: a barrel file whose *two* symbols both re-export the
        // SAME intermediary file must be recorded ONCE in that intermediary's
        // reverse (`re_exported_by`) set — a `Vec` previously accumulated one
        // duplicate per re-exporting symbol, doing redundant propagation work.
        let mut g = SymbolGraph::new();
        // barrel.ts has two public symbols, both re-exporting mid.ts.
        g.add_symbol(make_symbol(1, "a", "barrel.ts", Visibility::Public))
            .unwrap();
        g.add_symbol(make_symbol(2, "b", "barrel.ts", Visibility::Public))
            .unwrap();
        // mid.ts re-exports node:fs (so barrel.ts reaches it transitively).
        g.add_symbol(make_symbol(3, "mid", "mid.ts", Visibility::Public))
            .unwrap();
        g.add_symbol(external_module(4, "node:fs")).unwrap();
        add_reexport(&mut g, 1, 3); // barrel symbol a → mid
        add_reexport(&mut g, 2, 3); // barrel symbol b → mid (same target file)
        add_reexport(&mut g, 3, 4); // mid → node:fs

        let (_reached, re_exported_by, _worklist) = build_reexport_edges(&g);
        let into_mid = re_exported_by
            .get("mid.ts")
            .expect("mid.ts has re-exporters");
        assert_eq!(
            into_mid
                .iter()
                .filter(|f| f.as_str() == "barrel.ts")
                .count(),
            1,
            "a multi-symbol re-exporter must appear once in the propagation set, got {into_mid:?}",
        );

        // The dedup is a redundancy fix, not a semantic change: barrel.ts still
        // resolves to Privileged via the transitive re-export.
        annotate_trust(&mut g, &[]);
        assert_eq!(
            g.get_symbol(1).unwrap().trust_level,
            TrustLevel::Privileged,
            "barrel re-exporting an intermediary that reaches node:fs stays Privileged",
        );
    }

    #[test]
    fn reexport_cycle_terminates() {
        // a.ts ⇄ b.ts re-export each other (no privileged target): the walk must
        // terminate via the visited-file guard rather than loop forever.
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "a", "a.ts", Visibility::Public))
            .unwrap();
        g.add_symbol(make_symbol(2, "b", "b.ts", Visibility::Public))
            .unwrap();
        add_reexport(&mut g, 1, 2);
        add_reexport(&mut g, 2, 1);

        assert!(reexported_privileged_modules(&g, "a.ts").is_empty());
    }
}
