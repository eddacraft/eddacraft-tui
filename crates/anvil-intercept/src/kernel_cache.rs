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
//! `apply_delta` mirrors `anvil-kernel`'s `watch.rs` exactly: it updates the
//! `SymbolGraph` in place, maintains a per-key `all_imports` accumulator, and
//! re-runs [`re_resolve_imports`] so a forward reference (a file processed
//! before its import target) resolves once the target lands. The
//! `DependencyGraph` is then re-derived from the resolved import edges. This is
//! the **interim** Sub-phase A backing: re-deriving is O(edges) per save, which
//! the GV2 sub-phase A′ hot-read swap (ADR-063) later replaces with a resident
//! incremental reverse index under the same frozen wire. Because the warm path
//! runs the same update + re-resolve the cold rebuild does, the reverse index
//! stays consistent with a cold rebuild across arbitrary delta sequences
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

use std::collections::HashMap;
use std::sync::Mutex;

use anvil_graph_cache::certify::ChangeKind;
use anvil_graph_cache::{
    DependencyGraph, GraphDelta, SymbolGraph, re_resolve_imports, remove_file, update_file,
};
use anvil_intercept_proto::protocol::StaleReason;
use anvil_kernel_types::{EdgeType, FileSymbols, ImportEdge};

use crate::rule_cache::WorktreeKey;

/// Default capacity, matched to the rule-set cache
/// ([`crate::rule_cache::DEFAULT_RULE_SET_CACHE_CAPACITY`]) — both are bounded
/// by the same per-daemon concurrent-worktree cap.
pub const DEFAULT_KERNEL_CACHE_CAPACITY: usize = 1024;

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
    /// path). The `DependencyGraph` is re-derived from the updated
    /// `SymbolGraph`. A `Delete` against a key with no warm pair is a no-op: it
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
            delta
        } else {
            // ContentModify / Create / Rename(destination) re-extract the file's
            // symbols, then retry every accumulated import so a forward
            // reference (a file processed before its target) resolves once the
            // target lands — matching the cold-rebuild path (watch.rs).
            let new_imports = symbols.imports.clone();
            let delta = update_file(&mut entry.sym, symbols);
            entry.all_imports.retain(|i| i.from_file != file);
            entry.all_imports.extend(new_imports);
            re_resolve_imports(&mut entry.sym, &entry.all_imports);
            delta
        };
        entry.dep = derive_dependency_graph(&entry.sym);

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

/// Re-derive the module dependency graph from a `SymbolGraph`'s resolved import
/// edges. Cross-file `Imports` edges become `from_file -> to_file` dependencies;
/// intra-file edges are skipped. Mirrors what a cold rebuild produces.
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
}
