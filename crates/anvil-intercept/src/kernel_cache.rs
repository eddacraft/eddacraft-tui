//! DSV-004 Task 7 (ADR-061 §4, ADR-064 §5): the interim per-`WorktreeKey`
//! `(SymbolGraph, DependencyGraph)` cache the save-time daemon holds and mutates.
//!
//! The daemon keeps one warm graph pair per worktree so `certify`
//! ([`anvil_graph_cache::certify`]) can reach `DependencyGraph::dependents_of`
//! on the hot path (council verdict B1). [`KernelGraphCache::apply_delta`]
//! mutates the `SymbolGraph` in place from **already-parsed** [`FileSymbols`]
//! fed over the kernel→daemon channel — the daemon never links or invokes a
//! parser (ADR-064; guarded by `tests/daemon_dep_boundary.rs`).
//!
//! # Dependency graph maintenance
//!
//! `apply_delta` mirrors `anvil-kernel`'s `watch.rs` for the `SymbolGraph`: it
//! updates it in place, maintains a per-key `all_imports` accumulator, re-runs
//! [`re_resolve_imports_tracked`] so a forward reference (a file processed before
//! its import target) resolves once the target lands, and re-runs
//! [`annotate_trust`](anvil_graph_cache::annotate_trust) so the warm graph's
//! `trust_level` stays live for the `certify` privilege dimension (GV2-029).
//!
//! The `DependencyGraph` is **maintained incrementally** (GV2-011): the changed
//! file's outgoing edges are refreshed via [`refresh_file_dependencies`]
//! (`DependencyGraph::set_dependencies`), and when that file just became
//! resident — the only case in which another file's forward reference can newly
//! resolve against it — its dependents are refreshed too; a delete drops the
//! file in both directions. This retires the O(all-edges)
//! `derive_dependency_graph` full re-derive that the interim Sub-phase A backing
//! ran per save (ADR-063 / GV2 sub-phase A′), under the same frozen wire. The
//! incrementally-maintained index stays structurally equal to a cold rebuild
//! across arbitrary delta sequences — proven by the dep-graph property test
//! (`index_consistency_under_arbitrary_delta_sequence`,
//! `eddacraft-anvil-graph-cache`) and the end-to-end equivalence test here
//! (`reverse_index_consistent_after_delta`). The `SymbolGraph` itself is mutated
//! in place (never rebuilt).
//!
//! # Bounded capacity, generation guard, unregister hook
//!
//! Mirrors the [`crate::rule_cache::RuleSetCache`] shape (minus its
//! config-invalidation rate-limiter, which is rule-set specific): a bounded LRU
//! evicts the least-recently-used worktree on insert at capacity, and a
//! per-key **generation** — bumped on eviction or [`KernelGraphCache::invalidate`]
//! and surviving entry removal — lets a caller that snapshotted a generation
//! detect that its warm read was invalidated underneath it
//! ([`KernelGraphCache::is_generation_current`]). The daemon wires
//! [`KernelGraphCache::invalidate`] into the registry's
//! [`crate::registry::WorktreeUnregisterHook`] so a register/unregister cycle
//! leaves no cache residue.
//!
//! A cold key (first contact) yields [`StaleReason::CrossFileResolutionNeeded`]
//! and an eviction yields [`StaleReason::WarmStateEvicted`] (B6); the workspace
//! assurance *state machine* that consumes these lands with the `validate_paths`
//! orchestration (DSV-005).

use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;

use anvil_graph_cache::certify::ChangeKind;
use anvil_graph_cache::{
    DependencyGraph, GraphDelta, SymbolGraph, annotate_trust, re_resolve_imports_tracked,
    remove_file, update_file,
};
use anvil_intercept_proto::protocol::StaleReason;
use anvil_kernel_types::{EdgeType, FileSymbols, ImportEdge};

use crate::rule_cache::WorktreeKey;

/// Default capacity, matched to the rule-set cache
/// ([`crate::rule_cache::DEFAULT_RULE_SET_CACHE_CAPACITY`]) — both are bounded
/// by the same per-daemon concurrent-worktree cap.
pub const DEFAULT_KERNEL_CACHE_CAPACITY: usize = 1024;

/// Identifies the resident backing the save-time daemon certifies against,
/// behind the frozen `validate_paths` wire (ADR-061).
///
/// Sub-phase A shipped over `interim-symbolgraph-v1` — a `SymbolGraph` cache
/// rebuilt on restart. GV2-027 completes the A→A′ swap: the daemon now certifies
/// through the resident GV2 hot-read index ([`anvil_graph_cache::HotReadApi`]),
/// so the backing is `gv2-hotindex-v1`. The wire is unchanged — this marker is
/// the internal record of which backing answered, for diagnostics and the
/// `backing_parity` proof, never a wire field.
pub const BACKING_SCHEMA_VERSION: &str = "gv2-hotindex-v1";

/// A warm graph pair plus its LRU recency stamp.
#[derive(Debug)]
struct Entry {
    sym: SymbolGraph,
    dep: DependencyGraph,
    /// Every live import edge in the worktree, keyed by `from_file`. Mirrors
    /// `anvil-kernel`'s `watch.rs` `all_imports`: `update_file` can only resolve
    /// an import whose target file is already in the graph, so a file processed
    /// before its import targets leaves the edge unresolved. Re-running
    /// `re_resolve_imports` against this accumulator after each delta retries
    /// those forward references — without it the warm graph would permanently
    /// miss cross-file edges the cold rebuild resolves, and `dependents_of`
    /// would under-report importers (a false-clean certify hazard).
    all_imports: Vec<ImportEdge>,
    /// Generation at this entry's most recent access; the lowest in the map is
    /// the LRU victim.
    last_used: u64,
}

#[derive(Debug, Default)]
struct Inner {
    map: HashMap<WorktreeKey, Entry>,
    /// Monotonic recency counter, bumped on every access (apply/read hit).
    next_recency: u64,
    /// Per-key warmth generation, bumped on eviction or invalidate. Survives
    /// entry removal so a caller holding an old generation observes the bump.
    generations: HashMap<WorktreeKey, u64>,
    /// Cumulative LRU evictions since construction.
    evictions: u64,
}

/// The outcome of applying one parse-free delta.
#[derive(Debug)]
pub struct ApplyOutcome {
    /// The graph delta produced by the in-place `SymbolGraph` update.
    pub delta: GraphDelta,
    /// The key's warmth generation after this apply.
    pub generation: u64,
    /// `Some` when this was a cold build (the key had no warm pair): the daemon
    /// reports [`StaleReason::CrossFileResolutionNeeded`] because cross-file
    /// imports are not yet resolved on first contact (B6). `None` for a warm
    /// in-place update.
    pub cold_reason: Option<StaleReason>,
}

/// Thread-safe per-`WorktreeKey` graph cache. See the module docs.
#[derive(Debug)]
pub struct KernelGraphCache {
    inner: Mutex<Inner>,
    capacity: usize,
}

impl Default for KernelGraphCache {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelGraphCache {
    /// Empty cache with the default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_KERNEL_CACHE_CAPACITY)
    }

    /// Empty cache with a custom capacity, clamped to a minimum of 1 (a
    /// zero-capacity cache would evict every entry it just built). Eviction
    /// tests drive small capacities (1, 2).
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            capacity: capacity.max(1),
        }
    }

    /// Maximum warm worktrees before LRU eviction. Pinned at construction.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The resident backing this cache certifies against — `gv2-hotindex-v1`
    /// after the GV2-027 A→A′ swap (see [`BACKING_SCHEMA_VERSION`]).
    #[must_use]
    pub fn backing_schema_version(&self) -> &'static str {
        BACKING_SCHEMA_VERSION
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Number of warm worktrees currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lock().map.len()
    }

    /// `true` when no worktree is warm.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lock().map.is_empty()
    }

    /// Cumulative LRU evictions since construction.
    #[must_use]
    pub fn evictions(&self) -> u64 {
        self.lock().evictions
    }

    /// Whether a warm pair is currently held for `key`.
    #[must_use]
    pub fn contains(&self, key: &WorktreeKey) -> bool {
        self.lock().map.contains_key(key)
    }

    /// The current warmth generation for `key` (0 if never seen).
    #[must_use]
    pub fn generation(&self, key: &WorktreeKey) -> u64 {
        self.lock().generations.get(key).copied().unwrap_or(0)
    }

    /// Whether `generation` is still the current warmth generation for `key`.
    ///
    /// A caller snapshots [`ApplyOutcome::generation`] (or [`Self::generation`])
    /// alongside a warm read, then re-checks here before trusting that read: an
    /// eviction or invalidate between snapshot and use bumps the generation, so
    /// this returns `false` and the caller must treat its read as stale.
    ///
    /// # Scope of the guard
    ///
    /// This guards against **eviction and invalidate** only — `apply_delta`
    /// mutates a warm pair in place and deliberately does **not** bump the
    /// generation (a save is not an invalidation of the cache slot). It is
    /// therefore *not* a defence against a concurrent `apply_delta` racing a
    /// reader on the same key; that is prevented one layer up by the DSV-006
    /// per-`WorktreeKey` in-flight admission token, which serialises
    /// apply/validate for a key. The guard's job is the cross-event case: a key
    /// evicted (capacity pressure from other worktrees) or unregistered between
    /// a reader's snapshot and its use.
    #[must_use]
    pub fn is_generation_current(&self, key: &WorktreeKey, generation: u64) -> bool {
        self.generation(key) == generation
    }

    /// Read the warm graph pair for `key` under the lock, if present. Bumps the
    /// entry's recency so a subsequent eviction treats it as most-recently-used.
    pub fn with_graphs<R>(
        &self,
        key: &WorktreeKey,
        f: impl FnOnce(&SymbolGraph, &DependencyGraph) -> R,
    ) -> Option<R> {
        let mut guard = self.lock();
        let recency = guard.next_recency;
        guard.next_recency = recency.wrapping_add(1);
        let entry = guard.map.get_mut(key)?;
        entry.last_used = recency;
        Some(f(&entry.sym, &entry.dep))
    }

    /// Apply one **already-parsed** delta to `key`'s warm graph pair, building
    /// the pair cold on first contact.
    ///
    /// `symbols` is consumed as-is — the daemon never parses. `ContentModify`,
    /// `Create`, and `Rename` (destination) re-extract the file's symbols via
    /// [`anvil_graph_cache::update_file`] then retry forward-reference imports
    /// via [`anvil_graph_cache::re_resolve_imports`]; `Delete` drops the file
    /// via [`anvil_graph_cache::remove_file`] (its `symbols.file` names the
    /// path). The `DependencyGraph` is then maintained **incrementally** from
    /// the same edge changes (GV2-011) — no whole-graph re-derive. A `Delete`
    /// against a key with no warm pair is a no-op: it
    /// builds no phantom entry and returns `cold_reason: None` (there is nothing
    /// to invalidate).
    pub fn apply_delta(
        &self,
        key: &WorktreeKey,
        change: ChangeKind,
        symbols: FileSymbols,
    ) -> ApplyOutcome {
        let mut guard = self.lock();

        let cold = !guard.map.contains_key(key);

        // A Delete on a cold key has nothing to remove — never materialise an
        // empty phantom entry (it would occupy a capacity slot and read back as
        // "no symbols / no importers" rather than "not warm").
        if cold && change == ChangeKind::Delete {
            return ApplyOutcome {
                delta: remove_file(&mut SymbolGraph::new(), &symbols.file),
                generation: guard.generations.get(key).copied().unwrap_or(0),
                cold_reason: None,
            };
        }

        if cold && guard.map.len() >= self.capacity {
            evict_lru(&mut guard);
        }

        let recency = guard.next_recency;
        guard.next_recency = recency.wrapping_add(1);

        let entry = guard.map.entry(key.clone()).or_insert_with(|| Entry {
            sym: SymbolGraph::new(),
            dep: DependencyGraph::new(),
            all_imports: Vec::new(),
            last_used: recency,
        });
        entry.last_used = recency;

        let file = symbols.file.clone();
        let delta = if change == ChangeKind::Delete {
            let delta = remove_file(&mut entry.sym, &file);
            entry.all_imports.retain(|i| i.from_file != file);
            // GV2-011: a delete drops the file in both directions; no re-derive.
            // `remove_file` clears the reverse index, so dependents that imported
            // it correctly lose the edge.
            entry.dep.remove_file(&file);
            delta
        } else {
            // ContentModify / Create / Rename(destination) re-extract the file's
            // symbols, then retry every accumulated import so a forward
            // reference (a file processed before its target) resolves once the
            // target lands — matching the cold-rebuild path (watch.rs).
            //
            // GV2-011: maintain the dependency graph incrementally rather than
            // re-deriving the whole graph (the retired O(all-edges) cost). The set
            // of files whose symbol-graph import edges can change in one update is
            // the local neighbourhood, refreshed individually:
            //  - `file` itself (its own imports were re-extracted);
            //  - `file`'s prior dependents — `update_file` drops every edge
            //    incident to `file`'s old symbols, including `importer → file`
            //    edges, so each prior dependent must be reconciled (the edge is
            //    re-added if still resolved, dropped if it was stale or no longer
            //    resolves); captured before the symbol graph is mutated;
            //  - the sources of edges re-resolution adds — forward references that
            //    resolved now `file` exists, and surviving imports of *other*
            //    files that re-bind to a new target (e.g. the file a specifier
            //    resolved to was deleted, so a different candidate now wins).
            let new_imports = symbols.imports.clone();
            let mut affected: BTreeSet<String> = entry
                .dep
                .dependents_of(&file)
                .into_iter()
                .map(str::to_string)
                .collect();
            affected.insert(file.clone());
            let delta = update_file(&mut entry.sym, symbols);
            entry.all_imports.retain(|i| i.from_file != file);
            entry.all_imports.extend(new_imports);
            let readded = re_resolve_imports_tracked(&mut entry.sym, &entry.all_imports);
            for (from, _to, _ty) in &readded {
                if let Some(symbol) = entry.sym.get_symbol(*from) {
                    affected.insert(symbol.file.clone());
                }
            }
            for other in &affected {
                refresh_file_dependencies(&mut entry.dep, &entry.sym, other);
            }
            delta
        };

        // GV2-029: re-annotate trust on the warm graph after every mutation,
        // completing the `watch.rs` mirror. Without this `trust_level` stayed
        // `Unknown` on every daemon-resident symbol, so the `certify` privilege
        // dimension was inert and a change that newly imported `node:fs`/
        // `child_process` (escalating a symbol to `Privileged`) falsely
        // certified clean. Annotating here — after `re_resolve_imports_tracked`,
        // over the current `all_imports` — makes both the post-update graph
        // `certify` reads and the baseline the next `update_file` captures carry
        // live trust. Disjoint field borrows (`sym` mut, `all_imports` shared)
        // mirror `annotate_trust(&mut state.graph, &state.all_imports)`.
        annotate_trust(&mut entry.sym, &entry.all_imports);

        let generation = guard.generations.get(key).copied().unwrap_or(0);
        ApplyOutcome {
            delta,
            generation,
            cold_reason: cold.then_some(StaleReason::CrossFileResolutionNeeded),
        }
    }

    /// Drop the warm pair for `key` and bump its generation. Returns `true`
    /// when an entry was present. Wired to the registry unregister hook so a
    /// register/unregister cycle leaves no residue.
    pub fn invalidate(&self, key: &WorktreeKey) -> bool {
        let mut guard = self.lock();
        let dropped = guard.map.remove(key).is_some();
        let token = guard.generations.entry(key.clone()).or_insert(0);
        *token = token.wrapping_add(1);
        dropped
    }
}

/// Evict the least-recently-used entry, bumping its generation and the eviction
/// counter. Caller holds the lock and has confirmed the map is at capacity.
fn evict_lru(inner: &mut Inner) {
    let victim = inner
        .map
        .iter()
        .min_by_key(|(_, e)| e.last_used)
        .map(|(k, _)| k.clone());
    if let Some(victim) = victim {
        inner.map.remove(&victim);
        let token = inner.generations.entry(victim).or_insert(0);
        *token = token.wrapping_add(1);
        inner.evictions = inner.evictions.saturating_add(1);
    }
}

/// Refresh `file`'s outgoing dependency edges in `dep` from the current symbol
/// graph, replacing them with the file's resolved cross-file import targets.
///
/// GV2-011: this is the bounded, incremental replacement for the whole-graph
/// `derive_dependency_graph` re-derive. Cost is O(file's symbols × their import
/// edges) — the local neighbourhood, never the whole graph. Intra-file edges
/// are skipped, matching the cold rebuild.
///
/// The dependency graph today is built from `EdgeType::Imports` edges only —
/// consistent with the cold oracle [`derive_dependency_graph`] and the symbol
/// graph, which carries only `Imports` edges (`FileSymbols.reexports` is not yet
/// lifted into the graph). If a future change lifts another dependency-bearing
/// edge kind (e.g. `EdgeType::Reexports`) into the symbol graph, this filter,
/// `derive_dependency_graph`, and `re_resolve_imports` must be updated in
/// lockstep or the incremental graph will diverge from the cold rebuild.
fn refresh_file_dependencies(dep: &mut DependencyGraph, sym: &SymbolGraph, file: &str) {
    let mut targets: Vec<String> = Vec::new();
    for symbol in sym.symbols_in_file(file) {
        for edge in sym.outgoing_edges(symbol.id) {
            if edge.edge_type != EdgeType::Imports {
                continue;
            }
            if let Some(to) = sym.get_symbol(edge.to)
                && to.file != file
            {
                targets.push(to.file.clone());
            }
        }
    }
    dep.set_dependencies(file, targets);
}

/// Re-derive the module dependency graph from a `SymbolGraph`'s resolved import
/// edges. Cross-file `Imports` edges become `from_file -> to_file` dependencies;
/// intra-file edges are skipped.
///
/// GV2-011: this whole-graph re-derive is **no longer on the save-time path** —
/// [`KernelGraphCache::apply_delta`] now maintains the dependency graph
/// incrementally via [`refresh_file_dependencies`]. It survives only as the
/// cold-rebuild oracle for the equivalence property test below.
#[cfg(test)]
fn derive_dependency_graph(sym: &SymbolGraph) -> DependencyGraph {
    let mut dep = DependencyGraph::new();
    for node in sym.inner().node_weights() {
        for edge in sym.outgoing_edges(node.id) {
            if edge.edge_type != EdgeType::Imports {
                continue;
            }
            let (Some(from), Some(to)) = (sym.get_symbol(edge.from), sym.get_symbol(edge.to))
            else {
                continue;
            };
            if from.file != to.file {
                dep.add_dependency(from.file.clone(), to.file.clone());
            }
        }
    }
    dep
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_graph_cache::re_resolve_imports;
    use anvil_kernel_types::{ImportEdge, SymbolKind, SymbolNode, TrustLevel, Visibility};
    use std::path::PathBuf;

    fn key(name: &str) -> WorktreeKey {
        WorktreeKey::from_canonical(PathBuf::from(format!("/wt/{name}")))
    }

    /// A `FileSymbols` for `file` with the given public functions and import
    /// specifiers, ids allocated from `base`.
    fn file_symbols(file: &str, names: &[&str], imports: &[&str], base: u64) -> FileSymbols {
        FileSymbols {
            file: file.to_string(),
            symbols: names
                .iter()
                .enumerate()
                .map(|(i, n)| SymbolNode {
                    id: base + i as u64,
                    kind: SymbolKind::Function,
                    name: (*n).to_string(),
                    visibility: Visibility::Public,
                    file: file.to_string(),
                    trust_level: TrustLevel::Unknown,
                })
                .collect(),
            imports: imports
                .iter()
                .map(|src| ImportEdge {
                    from_file: file.to_string(),
                    to_source: (*src).to_string(),
                    line: 0,
                })
                .collect(),
            reexports: Vec::new(),
        }
    }

    #[test]
    fn cold_build_then_warm_read() {
        let cache = KernelGraphCache::new();
        let k = key("a");
        assert!(!cache.contains(&k));

        let out = cache.apply_delta(
            &k,
            ChangeKind::Create,
            file_symbols("a.ts", &["foo"], &[], 0),
        );
        assert_eq!(
            out.cold_reason,
            Some(StaleReason::CrossFileResolutionNeeded),
            "first contact is a cold build"
        );
        assert!(cache.contains(&k));

        // Warm read sees the built graph; second apply is not cold.
        let node_count = cache.with_graphs(&k, |sym, _| sym.node_count());
        assert_eq!(node_count, Some(1));
        let out2 = cache.apply_delta(
            &k,
            ChangeKind::ContentModify,
            file_symbols("a.ts", &["bar"], &[], 10),
        );
        assert_eq!(
            out2.cold_reason, None,
            "second apply is a warm in-place update"
        );
    }

    #[test]
    fn delta_update_mutates_in_place_not_rebuild() {
        let cache = KernelGraphCache::new();
        let k = key("a");
        cache.apply_delta(
            &k,
            ChangeKind::Create,
            file_symbols("a.ts", &["a_fn"], &[], 0),
        );
        cache.apply_delta(
            &k,
            ChangeKind::Create,
            file_symbols("b.ts", &["b_fn"], &[], 10),
        );

        // Updating a.ts must leave b.ts's symbol untouched in place.
        cache.apply_delta(
            &k,
            ChangeKind::ContentModify,
            file_symbols("a.ts", &["a_fn2"], &[], 20),
        );
        let (count, b_present) = cache
            .with_graphs(&k, |sym, _| {
                (sym.node_count(), sym.get_symbol(10).is_some())
            })
            .unwrap();
        assert_eq!(count, 2, "a.ts updated in place, b.ts preserved");
        assert!(b_present, "b.ts symbol id 10 survives an a.ts update");
    }

    /// GV2-029: the daemon apply path must run `annotate_trust` so a change that
    /// newly imports a privileged module (`node:fs`) escalates its symbols'
    /// trust and the warm certify path withholds a clean verdict. Before the
    /// wiring `trust_level` stayed `Unknown` on every warm symbol — the
    /// privilege dimension was inert and this privilege-expanding edit falsely
    /// certified clean.
    #[test]
    fn privilege_certify_withholds_clean_on_new_node_fs_import() {
        use anvil_graph_cache::certify::{Certifiability, certify};

        // `handler.ts` with a public entry and an internal helper, plus the
        // given import specifiers. The internal helper is the witness: a public
        // symbol is already `Boundary` (elevated), so its `Boundary →
        // Privileged` shift is invisible by design, whereas the internal helper
        // moves `Internal → Privileged` and surfaces as an added-privileged
        // identity once the file imports `node:fs`.
        let symbols = |imports: &[&str], base: u64| FileSymbols {
            file: "handler.ts".to_string(),
            symbols: vec![
                SymbolNode {
                    id: base,
                    kind: SymbolKind::Function,
                    name: "handle".to_string(),
                    visibility: Visibility::Public,
                    file: "handler.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                },
                SymbolNode {
                    id: base + 1,
                    kind: SymbolKind::Function,
                    name: "do_io".to_string(),
                    visibility: Visibility::Internal,
                    file: "handler.ts".to_string(),
                    trust_level: TrustLevel::Unknown,
                },
            ],
            imports: imports
                .iter()
                .map(|src| ImportEdge {
                    from_file: "handler.ts".to_string(),
                    to_source: (*src).to_string(),
                    line: 0,
                })
                .collect(),
            reexports: Vec::new(),
        };

        let cache = KernelGraphCache::new();
        let k = key("svc");

        // Warm the cache with the pre-edit file: no privileged import.
        cache.apply_delta(&k, ChangeKind::Create, symbols(&[], 0));

        // The edit adds a `node:fs` import — a privilege-expanding change that
        // leaves the public symbol names untouched.
        let outcome = cache.apply_delta(&k, ChangeKind::ContentModify, symbols(&["node:fs"], 10));

        let verdict = cache
            .with_graphs(&k, |sym, dep| {
                certify(sym, dep, &ChangeKind::ContentModify, &outcome.delta, 64)
            })
            .expect("warm key present after apply");

        assert!(
            matches!(verdict, Certifiability::Partial { .. }),
            "a change newly importing node:fs must not certify clean, got {verdict:?}"
        );
    }

    /// GV2-029: the common case the symbol-identity diff alone misses. An
    /// **all-public** file's first `node:fs` import — every symbol is already
    /// `Boundary`, so its `Boundary → Privileged` shift nets zero
    /// `added_privileged` against the `Privileged ∪ Boundary` baseline. The
    /// `newly_privileged_imports` module-surface dimension catches it. Before
    /// that dimension, this edit false-certified clean even with the wiring.
    #[test]
    fn privilege_certify_withholds_clean_on_all_public_node_fs_import() {
        use anvil_graph_cache::certify::{Certifiability, CertifyStale, certify};

        let cache = KernelGraphCache::new();
        let k = key("api");

        cache.apply_delta(
            &k,
            ChangeKind::Create,
            file_symbols("api.ts", &["getData"], &[], 0),
        );
        let outcome = cache.apply_delta(
            &k,
            ChangeKind::ContentModify,
            file_symbols("api.ts", &["getData"], &["node:fs"], 10),
        );

        let verdict = cache
            .with_graphs(&k, |sym, dep| {
                certify(sym, dep, &ChangeKind::ContentModify, &outcome.delta, 64)
            })
            .expect("warm key present after apply");

        assert!(
            matches!(
                verdict,
                Certifiability::Partial {
                    reason: CertifyStale::ExportSurfaceChange
                }
            ),
            "an all-public file gaining node:fs must not certify clean, got {verdict:?}"
        );
    }

    /// GV2-029: a **second** capability added to an already-privileged file
    /// (`fs` → `fs + child_process`). `annotate_trust` is file-granular, so every
    /// symbol was already `Privileged` and the symbol-identity sets are empty —
    /// only the `newly_privileged_imports` module diff sees the new capability.
    #[test]
    fn privilege_certify_withholds_clean_on_second_capability_import() {
        use anvil_graph_cache::certify::{Certifiability, CertifyStale, certify};

        let cache = KernelGraphCache::new();
        let k = key("api2");

        cache.apply_delta(
            &k,
            ChangeKind::Create,
            file_symbols("api.ts", &["getData"], &[], 0),
        );
        // Already privileged via node:fs.
        cache.apply_delta(
            &k,
            ChangeKind::ContentModify,
            file_symbols("api.ts", &["getData"], &["node:fs"], 10),
        );
        // The edit under test: add child_process — a genuinely new capability
        // that changes no symbol identity (all symbols were already Privileged).
        let outcome = cache.apply_delta(
            &k,
            ChangeKind::ContentModify,
            file_symbols("api.ts", &["getData"], &["node:fs", "child_process"], 20),
        );

        let verdict = cache
            .with_graphs(&k, |sym, dep| {
                certify(sym, dep, &ChangeKind::ContentModify, &outcome.delta, 64)
            })
            .expect("warm key present after apply");

        assert!(
            matches!(
                verdict,
                Certifiability::Partial {
                    reason: CertifyStale::ExportSurfaceChange
                }
            ),
            "adding child_process to an already-privileged file must not certify clean, got {verdict:?}"
        );
    }

    #[test]
    fn apply_delta_consumes_fed_file_symbols_not_a_daemon_parse() {
        // The contract: apply_delta takes already-parsed FileSymbols and builds
        // the graph from them verbatim — the daemon never invokes a parser.
        // Proof: hand-built symbols with ids the daemon could not have parsed
        // appear in the graph exactly as fed.
        let cache = KernelGraphCache::new();
        let k = key("a");
        let fed = file_symbols("a.ts", &["fed_symbol"], &[], 42);
        cache.apply_delta(&k, ChangeKind::Create, fed);
        let name = cache
            .with_graphs(&k, |sym, _| sym.get_symbol(42).map(|s| s.name.clone()))
            .flatten();
        assert_eq!(name.as_deref(), Some("fed_symbol"));
    }

    #[test]
    fn eviction_bumps_generation() {
        let cache = KernelGraphCache::with_capacity(1);
        let k1 = key("a");
        let k2 = key("b");
        cache.apply_delta(
            &k1,
            ChangeKind::Create,
            file_symbols("a.ts", &["x"], &[], 0),
        );
        let gen_before = cache.generation(&k1);

        // Inserting k2 at capacity 1 evicts k1.
        cache.apply_delta(
            &k2,
            ChangeKind::Create,
            file_symbols("b.ts", &["y"], &[], 0),
        );
        assert!(!cache.contains(&k1), "k1 evicted at capacity 1");
        assert_eq!(cache.evictions(), 1);
        assert!(
            cache.generation(&k1) > gen_before,
            "eviction bumps the victim's generation"
        );
    }

    #[test]
    fn generation_guard_blocks_stale_resolve() {
        let cache = KernelGraphCache::with_capacity(1);
        let k1 = key("a");
        let k2 = key("b");
        let out = cache.apply_delta(
            &k1,
            ChangeKind::Create,
            file_symbols("a.ts", &["x"], &[], 0),
        );
        let snapshot = out.generation;
        assert!(cache.is_generation_current(&k1, snapshot));

        // Evict k1 by inserting k2; a held snapshot is now stale.
        cache.apply_delta(
            &k2,
            ChangeKind::Create,
            file_symbols("b.ts", &["y"], &[], 0),
        );
        assert!(
            !cache.is_generation_current(&k1, snapshot),
            "a resolve from before the eviction must be rejected"
        );
    }

    #[test]
    fn delete_on_cold_key_creates_no_phantom_entry() {
        // A Delete for a worktree the cache never warmed must not materialise an
        // empty entry (it would occupy a capacity slot and read back as
        // "no symbols / no importers" rather than "not warm").
        let cache = KernelGraphCache::new();
        let k = key("a");
        let out = cache.apply_delta(&k, ChangeKind::Delete, file_symbols("gone.ts", &[], &[], 0));
        assert!(!cache.contains(&k), "Delete on a cold key warms nothing");
        assert_eq!(cache.len(), 0);
        assert_eq!(
            out.cold_reason, None,
            "a Delete is not a cold cross-file build"
        );
    }

    #[test]
    fn invalidate_drops_entry_and_bumps_generation() {
        let cache = KernelGraphCache::new();
        let k = key("a");
        cache.apply_delta(&k, ChangeKind::Create, file_symbols("a.ts", &["x"], &[], 0));
        let gen_before = cache.generation(&k);
        assert!(cache.invalidate(&k));
        assert!(!cache.contains(&k));
        assert!(cache.generation(&k) > gen_before);
        assert!(!cache.invalidate(&k), "second invalidate is a no-op drop");
    }

    #[test]
    fn reverse_index_consistent_after_delta() {
        // B1: the apply_delta-maintained DependencyGraph reverse index must
        // match a cold rebuild after an arbitrary multi-step delta sequence.
        // Sequence: add b.ts (imports ./a) -> add a.ts -> update a.ts removing
        // its public symbol -> update b.ts dropping the import.
        let k = key("a");

        let steps = || {
            vec![
                (
                    ChangeKind::Create,
                    file_symbols("a/b.ts", &["b_fn"], &["./a"], 0),
                ),
                (
                    ChangeKind::Create,
                    file_symbols("a/a.ts", &["a_fn"], &[], 10),
                ),
                (
                    ChangeKind::ContentModify,
                    file_symbols("a/a.ts", &["a_fn2"], &[], 20),
                ),
                (
                    ChangeKind::ContentModify,
                    file_symbols("a/b.ts", &["b_fn"], &[], 30),
                ),
            ]
        };

        // Apply step by step through the cache, checking consistency after each.
        for n in 1..=steps().len() {
            let cache = KernelGraphCache::new();
            for (change, syms) in steps().into_iter().take(n) {
                cache.apply_delta(&k, change, syms);
            }
            // True cold rebuild: replay the same first n steps into a fresh
            // graph WITH the same all_imports accumulation + re_resolve_imports
            // the warm path uses — otherwise the cold side would never resolve
            // forward references and the comparison would be vacuous (both
            // empty). This mirrors anvil-kernel's watch.rs cold path.
            let mut cold = SymbolGraph::new();
            let mut cold_imports: Vec<ImportEdge> = Vec::new();
            for (change, syms) in steps().into_iter().take(n) {
                let file = syms.file.clone();
                if change == ChangeKind::Delete {
                    remove_file(&mut cold, &file);
                    cold_imports.retain(|i| i.from_file != file);
                } else {
                    let new_imports = syms.imports.clone();
                    update_file(&mut cold, syms);
                    cold_imports.retain(|i| i.from_file != file);
                    cold_imports.extend(new_imports);
                    re_resolve_imports(&mut cold, &cold_imports);
                }
            }
            let cold_dep = derive_dependency_graph(&cold);

            let consistent = cache
                .with_graphs(&k, |_, dep| {
                    for f in ["a/a.ts", "a/b.ts"] {
                        let mut warm = dep.dependents_of(f);
                        let mut cold = cold_dep.dependents_of(f);
                        warm.sort_unstable();
                        cold.sort_unstable();
                        if warm != cold {
                            return false;
                        }
                    }
                    true
                })
                .unwrap();
            assert!(
                consistent,
                "reverse index diverged from cold rebuild at step {n}"
            );
        }

        // Non-vacuous guard: after step 2 (b.ts imports ./a, then a.ts lands)
        // the forward reference MUST have resolved — b.ts is a dependent of
        // a.ts. This is exactly the edge the missing re_resolve_imports dropped.
        let cache = KernelGraphCache::new();
        for (change, syms) in steps().into_iter().take(2) {
            cache.apply_delta(&k, change, syms);
        }
        let dependents = cache
            .with_graphs(&k, |_, dep| {
                dep.dependents_of("a/a.ts")
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap();
        assert_eq!(
            dependents,
            vec!["a/b.ts".to_string()],
            "forward-reference import b.ts -> ./a must resolve once a.ts lands"
        );
    }

    /// Deterministic LCG — reproducible delta sequences without a `rand`/
    /// `proptest` dependency (Anvil determinism principle).
    struct Lcg(u64);
    impl Lcg {
        fn new(seed: u64) -> Self {
            Self(seed)
        }
        fn next_u64(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next_u64() >> 33) as usize % n
        }
    }

    /// GV2-011 end-to-end cold-rebuild equivalence: the incrementally-maintained
    /// warm `DependencyGraph` must equal a full cold rebuild (the retired
    /// `derive_dependency_graph` oracle) after an *arbitrary* delta sequence —
    /// covering create, body-only modify (import shape unchanged), import-set
    /// change, delete, and delete/recreate, including forward references that
    /// resolve only once their target lands. Compares the whole graph (forward
    /// AND reverse), not a hand-picked file pair.
    #[test]
    fn warm_dep_graph_matches_cold_rebuild_over_arbitrary_sequence() {
        // Modules at TWO depths sharing basenames, so a root file importing the
        // *other* basename ("./a"/"./b") is ambiguous: `resolve_import` matches by
        // path suffix and the shortest path wins, so deleting the root winner
        // re-binds the importer to the nested candidate. This exercises the
        // re-resolution re-binding path, not just same-directory resolution.
        let files: [(&str, &str); 4] = [
            ("a.ts", "a"),
            ("b.ts", "b"),
            ("nested/a.ts", "a"),
            ("nested/b.ts", "b"),
        ];

        // Several independent seeds, each a long-lived warm cache (incremental,
        // accumulating state) checked against a fresh cold rebuild after every
        // step.
        for seed in [0x5EED_1234_ABCD_u64, 0x0BAD_F00D, 0xDEAD_BEEF, 0x1357_2468] {
            let mut rng = Lcg::new(seed);
            let k = key("prop");
            let cache = KernelGraphCache::new();
            let mut steps: Vec<(ChangeKind, FileSymbols)> = Vec::new();
            let mut id_base: u64 = 0;

            for _ in 0..400 {
                let (file, base) = files[rng.below(files.len())];
                // 1-in-5 delete, else create/modify with a random import subset.
                let step = if rng.below(5) == 0 {
                    (ChangeKind::Delete, file_symbols(file, &[], &[], id_base))
                } else {
                    // Random imports of the other basename ("./a"/"./b"), excluding
                    // the file's own basename to avoid trivial self-resolution.
                    let imports: Vec<&str> = [("a", "./a"), ("b", "./b")]
                        .iter()
                        .filter(|(n, _)| *n != base && rng.below(2) == 0)
                        .map(|(_, spec)| *spec)
                        .collect();
                    // A symbol set that sometimes empties (drop the public
                    // symbol), making the file unresolvable as an import target.
                    let syms: &[&str] = if rng.below(4) == 0 { &[] } else { &["sym"] };
                    let kind = if rng.below(2) == 0 {
                        ChangeKind::Create
                    } else {
                        ChangeKind::ContentModify
                    };
                    (kind, file_symbols(file, syms, &imports, id_base))
                };
                id_base += 100;

                // Apply this one step to the long-lived warm cache (incremental).
                cache.apply_delta(&k, step.0, step.1.clone());
                steps.push(step);

                // Cold rebuild: replay the sequence into a fresh graph with the
                // all_imports + re_resolve the warm path uses, then derive.
                let mut cold = SymbolGraph::new();
                let mut cold_imports: Vec<ImportEdge> = Vec::new();
                for (change, syms) in &steps {
                    let f = syms.file.clone();
                    if *change == ChangeKind::Delete {
                        remove_file(&mut cold, &f);
                        cold_imports.retain(|i| i.from_file != f);
                    } else {
                        let new_imports = syms.imports.clone();
                        update_file(&mut cold, syms.clone());
                        cold_imports.retain(|i| i.from_file != f);
                        cold_imports.extend(new_imports);
                        re_resolve_imports(&mut cold, &cold_imports);
                    }
                }
                let cold_dep = derive_dependency_graph(&cold);

                let consistent = match cache.with_graphs(&k, |_, dep| *dep == cold_dep) {
                    Some(eq) => eq,
                    // No warm entry (e.g. only deletes on a cold key) ⇒ empty.
                    None => cold_dep == DependencyGraph::new(),
                };
                assert!(
                    consistent,
                    "seed {seed:#x}: warm dep graph diverged from cold rebuild \
                     after {} steps (last: {:?} {})",
                    steps.len(),
                    steps.last().map(|(c, _)| c),
                    steps.last().map_or("", |(_, s)| s.file.as_str()),
                );
            }
        }
    }

    /// GV2-027 backing parity: the A→A′ swap routes certification through the
    /// resident GV2 hot-read index ([`anvil_graph_cache::HotReadApi`]). This
    /// proves the swap is **wire-invariant** — over an arbitrary delta sequence,
    /// certifying a change against the warm, incrementally-maintained backing
    /// yields the **verdict-identical** [`Certifiability`] as certifying the same
    /// change (same delta) against a cold rebuild (the `interim-symbolgraph-v1`
    /// backing the swap retires). Extends the GV2-011 *structural* cold-rebuild
    /// equivalence to the certify verdict, covering the export-surface (`sym`)
    /// and reverse-impact (`dep`) reads plus the GV2-029 trust annotation
    /// (occasional `node:fs` imports escalate a symbol to `Privileged`).
    #[test]
    // One coherent property: budget → seed → step → (warm apply, cold rebuild,
    // backing-equality + verdict-parity assert) → per-budget distribution check.
    // Splitting the nested loops out would obscure the proof's flow.
    #[allow(clippy::too_many_lines)]
    fn backing_parity_warm_matches_cold_rebuild_over_arbitrary_sequence() {
        use anvil_graph_cache::HotReadApi;
        use anvil_graph_cache::certify::{Certifiability, CertifyStale, certify};

        let files: [(&str, &str); 4] = [
            ("a.ts", "a"),
            ("b.ts", "b"),
            ("nested/a.ts", "a"),
            ("nested/b.ts", "b"),
        ];

        // Two budgets: 64 is realistic (this 4-file corpus can never reach it, so
        // the closure is always bounded → ExportSurfaceChange/Certified only); 1
        // forces ImpactSetOverflow on any file with ≥2 importers, so the overflow
        // verdict is on the parity surface too. The per-budget distribution
        // assertions below pin that neither branch is silently unexercised.
        for budget in [64usize, 1usize] {
            let (mut certified, mut surface, mut overflow) = (0usize, 0usize, 0usize);

            for seed in [0x5EED_1234_ABCD_u64, 0x0BAD_F00D, 0xDEAD_BEEF, 0x1357_2468] {
                let mut rng = Lcg::new(seed);
                let k = key("parity");
                let cache = KernelGraphCache::new();
                // The cache certifies against the A′ resident hot-index backing.
                assert_eq!(cache.backing_schema_version(), BACKING_SCHEMA_VERSION);
                let mut steps: Vec<(ChangeKind, FileSymbols)> = Vec::new();
                let mut id_base: u64 = 0;

                for _ in 0..400 {
                    let (file, base) = files[rng.below(files.len())];
                    let step = if rng.below(5) == 0 {
                        (ChangeKind::Delete, file_symbols(file, &[], &[], id_base))
                    } else {
                        let mut imports: Vec<&str> = [("a", "./a"), ("b", "./b")]
                            .iter()
                            .filter(|(n, _)| *n != base && rng.below(2) == 0)
                            .map(|(_, spec)| *spec)
                            .collect();
                        // 1-in-4: newly import a privileged module so the GV2-029
                        // trust dimension (Internal → Privileged escalation) is
                        // part of the parity surface, not just structural edges.
                        if rng.below(4) == 0 {
                            imports.push("node:fs");
                        }
                        let syms: &[&str] = if rng.below(4) == 0 { &[] } else { &["sym"] };
                        let kind = if rng.below(2) == 0 {
                            ChangeKind::Create
                        } else {
                            ChangeKind::ContentModify
                        };
                        (kind, file_symbols(file, syms, &imports, id_base))
                    };
                    id_base += 100;

                    let outcome = cache.apply_delta(&k, step.0, step.1.clone());
                    steps.push(step.clone());

                    // Cold rebuild of (sym, dep) replaying the whole sequence,
                    // step-faithful to apply_delta: update/remove + re_resolve +
                    // annotate_trust *after every step* (so a cold-side delta, were
                    // it computed, would see the same `previously_*` baselines).
                    let mut cold = SymbolGraph::new();
                    let mut cold_imports: Vec<ImportEdge> = Vec::new();
                    for (change, syms) in &steps {
                        let f = syms.file.clone();
                        if *change == ChangeKind::Delete {
                            remove_file(&mut cold, &f);
                            cold_imports.retain(|i| i.from_file != f);
                        } else {
                            let new_imports = syms.imports.clone();
                            update_file(&mut cold, syms.clone());
                            cold_imports.retain(|i| i.from_file != f);
                            cold_imports.extend(new_imports);
                            re_resolve_imports(&mut cold, &cold_imports);
                        }
                        annotate_trust(&mut cold, &cold_imports);
                    }
                    let cold_dep = derive_dependency_graph(&cold);

                    // Certify the SAME change (ContentModify, as validate_paths
                    // does) with the warm-produced delta against both backings.
                    // Using the warm delta on both arms is sound *because* the
                    // backings are proven structurally identical each step (the
                    // `dep_eq` assertion below + identical deterministic
                    // update_file replay), so a cold-computed delta would be the
                    // same — the only variable under test is the backing's
                    // (sym, dep) reads. The warm arm goes through `HotReadApi` —
                    // the live A′ backing.
                    let dep_eq = cache.with_graphs(&k, |_, dep| *dep == cold_dep);
                    let v_warm = cache.with_graphs(&k, |sym, dep| {
                        HotReadApi::new(sym, dep).certify(
                            &ChangeKind::ContentModify,
                            &outcome.delta,
                            budget,
                        )
                    });
                    let v_cold = certify(
                        &cold,
                        &cold_dep,
                        &ChangeKind::ContentModify,
                        &outcome.delta,
                        budget,
                    );

                    match v_warm {
                        Some(v_warm) => {
                            assert_eq!(
                                dep_eq,
                                Some(true),
                                "seed {seed:#x} budget {budget}: warm dep graph \
                                 diverged from cold rebuild after {} steps",
                                steps.len(),
                            );
                            assert_eq!(
                                v_warm,
                                v_cold,
                                "seed {seed:#x} budget {budget}: warm hot-index \
                                 verdict diverged from cold rebuild after {} steps \
                                 (last: {:?} {})",
                                steps.len(),
                                steps.last().map(|(c, _)| c),
                                steps.last().map_or("", |(_, s)| s.file.as_str()),
                            );
                            match &v_warm {
                                Certifiability::Certified { .. } => certified += 1,
                                Certifiability::Partial {
                                    reason: CertifyStale::ExportSurfaceChange,
                                } => surface += 1,
                                Certifiability::Partial {
                                    reason: CertifyStale::ImpactSetOverflow,
                                } => overflow += 1,
                                Certifiability::Partial { .. } => {}
                            }
                        }
                        // `with_graphs` is None only for a key with no warm entry.
                        // The cold rebuild must then also be empty — assert it
                        // rather than skipping, so a warm-eviction-shaped
                        // divergence can't hide here.
                        None => assert_eq!(
                            cold_dep,
                            DependencyGraph::new(),
                            "seed {seed:#x} budget {budget}: no warm entry but cold \
                             rebuild is non-empty after {} steps",
                            steps.len(),
                        ),
                    }
                }
            }

            // Non-vacuousness: prove the parity surface actually exercised the
            // verdict branches it claims to cover, deterministically (the LCG is
            // seeded, so these counts are fixed).
            assert!(
                certified > 0,
                "budget {budget}: no Certified verdict across the run — \
                 parity test may be vacuous",
            );
            assert!(
                surface > 0,
                "budget {budget}: no ExportSurfaceChange verdict across the run",
            );
            if budget == 1 {
                assert!(
                    overflow > 0,
                    "budget 1: ImpactSetOverflow branch never exercised — \
                     overflow parity is unproven",
                );
            } else {
                assert_eq!(
                    overflow, 0,
                    "budget {budget}: closure cannot exceed budget with this \
                     4-file corpus, so overflow must not occur",
                );
            }
        }
    }

    /// GV2-011 regression (council CRITICAL): re-resolution can re-bind a
    /// *surviving* import of a file other than the one being saved. Here `main.ts`
    /// imports `./a`, which suffix-matches both `a.ts` (shortest — wins) and
    /// `nested/a.ts`. Deleting `a.ts` and then saving an unrelated file makes
    /// `re_resolve_imports` re-bind `main.ts` to `nested/a.ts` in the symbol
    /// graph; the incremental dependency graph must follow even though `main.ts`
    /// was neither the saved file nor a dependent of it. Before the
    /// `re_resolve_imports_tracked` fix the warm graph kept `main.ts` with no
    /// outgoing edge while the cold rebuild had `main.ts → nested/a.ts`.
    #[test]
    fn re_resolution_rebinds_surviving_import_after_target_delete() {
        let k = key("rebind");
        let cache = KernelGraphCache::new();

        cache.apply_delta(&k, ChangeKind::Create, file_symbols("a.ts", &["x"], &[], 0));
        cache.apply_delta(
            &k,
            ChangeKind::Create,
            file_symbols("nested/a.ts", &["y"], &[], 10),
        );
        cache.apply_delta(
            &k,
            ChangeKind::Create,
            file_symbols("main.ts", &["app"], &["./a"], 20),
        );
        // main.ts → a.ts (shortest-path winner).
        let before = cache
            .with_graphs(&k, |_, dep| {
                dep.dependencies_of("main.ts")
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap();
        assert_eq!(before, vec!["a.ts".to_string()], "setup: main imports a.ts");

        // Delete the winner, then save an unrelated file to trigger re-resolution.
        cache.apply_delta(&k, ChangeKind::Delete, file_symbols("a.ts", &[], &[], 30));
        cache.apply_delta(
            &k,
            ChangeKind::ContentModify,
            file_symbols("other.ts", &["z"], &[], 40),
        );

        let deps = cache
            .with_graphs(&k, |_, dep| {
                dep.dependencies_of("main.ts")
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap();
        assert_eq!(
            deps,
            vec!["nested/a.ts".to_string()],
            "main.ts must re-bind to nested/a.ts after a.ts is deleted"
        );
        let dependents = cache
            .with_graphs(&k, |_, dep| {
                dep.dependents_of("nested/a.ts")
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap();
        assert_eq!(
            dependents,
            vec!["main.ts".to_string()],
            "reverse index (the certify path's input) must show main.ts"
        );
    }
}
