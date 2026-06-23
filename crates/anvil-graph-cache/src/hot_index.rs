//! Hot-path read API for save-time and mid-edit enforcement (GV2-022, ADR-063).
//!
//! [ADR-063](../../../plans/decisions/063-gv2-hot-path-boundary.md) freezes the
//! exact set of graph reads the intercept daemon (INTD) and surface drivers
//! (DRVR) may execute inside a `validate_paths` / mid-edit call against the
//! ADR-031 latency budget. A read is **hot-path-admissible** only if it is
//! answerable from **resident warm indexes** in O(1) or O(bounded fan-out) with
//! no parse, no cross-file resolution, no transitive traversal beyond the
//! configured reverse-impact depth, and no blocking I/O.
//!
//! This module is that surface. It holds only shared references to the warm
//! `(SymbolGraph, DependencyGraph)` pair, so every read is a pure in-memory
//! lookup — structurally incapable of parsing, rebuilding, or touching the
//! filesystem. The four admissible reads are:
//!
//! 1. **Resident per-file symbol lookup** ([`HotReadApi::resident_symbols`],
//!    [`HotReadApi::symbol_owner`]) — the warm record for an already-extracted
//!    file; an absent/evicted file is a typed warm-miss.
//! 2. **Known-edge existence** ([`HotReadApi::known_edge`]) — "does edge
//!    `A → B` exist?", O(1)/O(degree) from the resident dependency index.
//! 3. **Bounded reverse impact** ([`HotReadApi::reverse_impact`]) — importer
//!    closure to a hard-capped depth; overflow past the budget is a typed miss,
//!    never an unbounded walk.
//! 4. **Precomputed boundary/trust membership**
//!    ([`HotReadApi::boundary_membership`]) — the resident `TrustLevel` flag
//!    `annotate_trust` precomputed onto the symbol, read O(1), never recomputed.
//!
//! # Miss-degrades-to-fallback (the load-bearing rule)
//!
//! A read that cannot be served from warm, resident state returns
//! [`HotRead::Stale`] with a typed [`HotReadMiss`]. The caller **must** degrade
//! to its existing fallback (the daemon-absent full validation path); it
//! **must not** escalate to an on-hot-path parse, resolve, traversal, rebuild,
//! or I/O. "Slower but complete" is never a hot-path option — completeness is
//! the background pool's job (ADR-063 miss/stale policy).
//!
//! # Crate boundary (ADR-064 §2)
//!
//! [`HotReadMiss`] is **graph-cache-local**, mirroring [`crate::certify::CertifyStale`].
//! The daemon (DSV-005, `anvil-intercept`) maps it to the wire `StaleReason` at
//! the boundary, so this crate keeps its frozen dependency set
//! (`anvil-kernel-types` + `petgraph` + `serde` + `thiserror`) — no
//! `anvil-intercept` dependency. The intended mapping is:
//! `WarmStateEvicted → StaleReason::WarmStateEvicted`,
//! `CrossFileResolutionNeeded → StaleReason::CrossFileResolutionNeeded`,
//! `ImpactSetOverflow → StaleReason::ImpactSetOverflow`.
//!
//! # Hot-path seal (GV2-024)
//!
//! The admissibility above is **compile-enforced**, not just documented:
//! non-admissible (denylist) ops live on [`BackgroundReadApi`] and are
//! unreachable from this hot surface, [`HotPathSurface`] is a sealed marker only
//! [`HotReadApi`] implements, and the bounded walks carry a `debug_assert` depth
//! guard (ADR-063 §3 / ADR-077). Two `compile_fail` doctests on
//! [`HotPathSurface`] prove the seal holds.
//!
//! # Out of scope
//!
//! The runtime-configurable depth lever wired through `flags/manifest.json` is
//! GV2-026; the Criterion CI latency gate is GV2-025 (landed); the A→A′ backing
//! swap behind `validate_paths` is GV2-027 (landed).

use std::collections::HashSet;

use anvil_kernel_types::{SymbolNode, TrustLevel};

use crate::certify::{Certifiability, ChangeKind, certify};
use crate::dependency::DependencyGraph;
use crate::incremental::GraphDelta;
use crate::symbol_graph::SymbolGraph;

/// Hard ceiling on the reverse-impact hop depth.
///
/// The runtime-configurable lever (GV2-026) sets the *effective* depth at or
/// below this cap; a request above the cap is **clamped, not honoured**
/// (ADR-063: "an over-cap setting is clamped, not honoured"), so no
/// configuration can reintroduce the unbounded-traversal the boundary exists to
/// prevent. The default-1-hop policy (the ADR-061 §6 certifiability closure)
/// lives at the caller; this constant only guarantees the ceiling.
pub const MAX_REVERSE_IMPACT_DEPTH: u32 = 2;

/// Why a hot read could not be served from warm, resident state.
///
/// Graph-cache-local (ADR-064 §2); the daemon maps these to the wire
/// `StaleReason` at the boundary. Variant names match the wire vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotReadMiss {
    /// The file or symbol is not resident — never extracted, or evicted from
    /// the warm cache.
    WarmStateEvicted,
    /// Answering the read would require cross-file symbol resolution that is
    /// not present in the warm index (a denylist operation on the hot path).
    ///
    /// Part of the ADR-063 miss taxonomy and reserved for resolution-shaped
    /// reads: no read on this API emits it today (the four allowlist reads are
    /// all answerable from resident state or via the bounded closure), but it
    /// is the declared graph-cache-local counterpart the daemon maps from
    /// `CertifyStale::CrossFileResolutionNeeded` / the wire `StaleReason`, kept
    /// here so the miss vocabulary matches the frozen boundary rather than
    /// being invented at the call site.
    CrossFileResolutionNeeded,
    /// The bounded reverse-impact closure exceeded its budget at the configured
    /// (hard-capped) depth.
    ImpactSetOverflow,
}

/// The result of a hot-path read: a warm answer, or a typed miss the caller
/// **must** degrade to fallback on (never escalate to parse/rebuild/I/O).
///
/// `#[must_use]` because silently dropping a hot read discards the warm/stale
/// distinction the whole contract turns on.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum HotRead<T> {
    /// Served from resident warm state; `T` is the answer.
    Warm(T),
    /// Could not be served warm; the caller must degrade to fallback.
    Stale(HotReadMiss),
}

impl<T> HotRead<T> {
    /// Whether the read was served from warm state.
    pub fn is_warm(&self) -> bool {
        matches!(self, HotRead::Warm(_))
    }

    /// Whether the read missed and must degrade to fallback.
    pub fn is_stale(&self) -> bool {
        matches!(self, HotRead::Stale(_))
    }

    /// The warm answer, if any.
    pub fn warm(self) -> Option<T> {
        match self {
            HotRead::Warm(value) => Some(value),
            HotRead::Stale(_) => None,
        }
    }

    /// The typed miss reason, if this was a miss.
    pub fn miss(&self) -> Option<HotReadMiss> {
        match self {
            HotRead::Stale(reason) => Some(*reason),
            HotRead::Warm(_) => None,
        }
    }
}

/// A read-only view over the resident warm graph pair, exposing only the
/// ADR-063 hot-path allowlist. Constructing it performs no work and borrows the
/// graphs immutably, so it can be held briefly inside a `validate_paths` call
/// without blocking writers beyond the borrow.
pub struct HotReadApi<'a> {
    sym: &'a SymbolGraph,
    dep: &'a DependencyGraph,
}

impl<'a> HotReadApi<'a> {
    /// Wrap the warm `(SymbolGraph, DependencyGraph)` pair the daemon holds per
    /// worktree.
    pub fn new(sym: &'a SymbolGraph, dep: &'a DependencyGraph) -> Self {
        Self { sym, dep }
    }

    /// Allowlist #1 — the resident symbols of an already-extracted `file`.
    ///
    /// A file with no recorded symbols — never extracted, evicted, or extracted
    /// but symbol-empty (all leave no `files` entry; see
    /// [`SymbolGraph::contains_file`]) — is a typed warm-miss, so the daemon
    /// degrades to fallback rather than trusting a false-empty surface.
    pub fn resident_symbols(&self, file: &str) -> HotRead<Vec<&'a SymbolNode>> {
        if self.sym.contains_file(file) {
            HotRead::Warm(self.sym.symbols_in_file(file))
        } else {
            HotRead::Stale(HotReadMiss::WarmStateEvicted)
        }
    }

    /// Allowlist #1 — which file owns symbol `id` (symbol ownership), O(1).
    pub fn symbol_owner(&self, id: u64) -> HotRead<&'a str> {
        match self.sym.get_symbol(id) {
            Some(node) => HotRead::Warm(node.file.as_str()),
            None => HotRead::Stale(HotReadMiss::WarmStateEvicted),
        }
    }

    /// Allowlist #2 — known-edge existence: does `from` import `to`?
    ///
    /// Always warm: the resident dependency index is authoritative for the
    /// edges it holds, so a missing edge is a definitive `false`, not a miss.
    pub fn known_edge(&self, from: &str, to: &str) -> HotRead<bool> {
        HotRead::Warm(self.dep.has_edge(from, to))
    }

    /// Allowlist #3 — the bounded reverse-impact closure of `file`.
    ///
    /// Breadth-first over [`DependencyGraph::dependents_of`] outward from
    /// `file` to at most `min(max_depth, MAX_REVERSE_IMPACT_DEPTH)` hops. The
    /// result excludes `file` itself, and `seen` deduplication plus the depth
    /// cap terminate any cycle without an unbounded walk.
    ///
    /// `budget` is the maximum number of distinct files the warm closure may
    /// hold: a warm result contains at most `budget` files, and the read
    /// returns [`HotReadMiss::ImpactSetOverflow`] the moment the closure *would*
    /// exceed it — the same `len() > budget` semantics as
    /// [`mod@crate::certify`]'s `bounded_impact_closure` (the certify-side capped
    /// walk), so the worst case is bounded by construction (cap × budget). A file
    /// with no importers warms to an empty
    /// set; `max_depth == 0` (a degenerate input below the ADR-061 §6 default of
    /// 1 hop) skips the walk and warms to an empty set.
    pub fn reverse_impact(
        &self,
        file: &str,
        max_depth: u32,
        budget: usize,
    ) -> HotRead<HashSet<String>> {
        // This method runs its own walk (below). An over-cap request is clamped,
        // not honoured (ADR-063 §3), so the cap is enforced here by construction
        // and no assertion is needed. (The separate certify-side walk,
        // `bounded_impact_closure`, takes `depth` as a direct parameter and so
        // carries the `debug_assert` that trips on an over-cap depth.)
        let depth = max_depth.min(MAX_REVERSE_IMPACT_DEPTH);
        let mut seen: HashSet<String> = HashSet::new();
        let mut frontier: Vec<String> = vec![file.to_string()];

        for _ in 0..depth {
            let mut next: Vec<String> = Vec::new();
            for current in &frontier {
                for importer in self.dep.dependents_of(current) {
                    // Exclude the origin and dedupe; `seen` also terminates
                    // cycles (a → b → a) without an unbounded walk.
                    if importer != file && seen.insert(importer.to_string()) {
                        if seen.len() > budget {
                            return HotRead::Stale(HotReadMiss::ImpactSetOverflow);
                        }
                        next.push(importer.to_string());
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }

        HotRead::Warm(seen)
    }

    /// Allowlist #4 — the precomputed boundary/trust membership flag for symbol
    /// `id`, read O(1) from the resident node (`annotate_trust` populates it;
    /// it is never recomputed on read). An absent symbol is a warm-miss.
    pub fn boundary_membership(&self, id: u64) -> HotRead<TrustLevel> {
        match self.sym.get_symbol(id) {
            Some(node) => HotRead::Warm(node.trust_level),
            None => HotRead::Stale(HotReadMiss::WarmStateEvicted),
        }
    }

    /// Certify one changed file's verdict over this resident warm backing — the
    /// save-time daemon's A′ entry point (GV2-027).
    ///
    /// This makes the hot-read API the **live backing** the daemon reads behind
    /// the frozen `validate_paths` wire (ADR-061): the A→A′ swap routes
    /// certification through `HotReadApi` rather than threading the raw
    /// `(SymbolGraph, DependencyGraph)` pair, so the resident GV2 hot-index is
    /// the access path and GV2-024 has a single surface to seal.
    ///
    /// It is **verdict-identical** to [`crate::certify::certify`] by
    /// construction — it delegates to the same verdict authority over the same
    /// resident graphs, so the backing swap is wire-invariant (proven at the
    /// daemon layer by the `backing_parity` property test).
    ///
    /// **GV2-024 / ADR-077 (resolved):** the GV2-027 fork — adopt a depth-capped
    /// closure here, or exclude `certify` from the seal — was decided **path A**
    /// by [ADR-077](../../../plans/decisions/077-cert-closure-depth-cap.md): the
    /// shared [`mod@crate::certify`] closure is now hard-depth-capped at
    /// [`MAX_REVERSE_IMPACT_DEPTH`], so this entry carries **no** unbounded
    /// traversal and the GV2-024 seal covers the whole `HotReadApi` uniformly —
    /// no carve-out. The cap can only flip an over-cap-chain stale reason from
    /// `ImpactSetOverflow` to `ExportSurfaceChange` (both `Partial`); the
    /// `certified | partial` verdict is unchanged. The cap's verdict-neutrality
    /// (the over-cap-chain flip) is covered by the unit tests in
    /// [`mod@crate::certify`]; `backing_parity` independently proves warm == cold
    /// for the corpus it generates (both backings share this capped closure).
    #[must_use]
    pub fn certify(
        &self,
        change: &ChangeKind,
        delta: &GraphDelta,
        budget: usize,
        max_depth: u32,
    ) -> Certifiability {
        certify(self.sym, self.dep, change, delta, budget, max_depth)
    }
}

mod sealed {
    /// Private supertrait that makes [`super::HotPathSurface`] **sealed**: it can
    /// only be named (and therefore implemented) inside this crate.
    pub trait Sealed {}
    impl Sealed for super::HotReadApi<'_> {}
}

/// Marker for the ADR-063 hot-path read surface — the type a `validate_paths` /
/// mid-edit caller holds while it issues save-time reads.
///
/// **Sealed**: only [`HotReadApi`] implements it (the supertrait is private), so
/// no external type can present itself as hot-path-admissible, and hot-path
/// generic code can bound on `T: HotPathSurface` to accept *only* the allowlist
/// surface. The ADR-063 denylist — unbounded transitive traversal, cross-file
/// resolution, full-graph scans — lives on [`BackgroundReadApi`] and is
/// unreachable through this marker.
///
/// The seal is enforced at compile time. A denylist op is uncallable from the
/// hot surface:
///
/// ```compile_fail
/// use anvil_graph_cache::{DependencyGraph, HotReadApi, SymbolGraph};
/// let (sym, dep) = (SymbolGraph::new(), DependencyGraph::new());
/// let hot = HotReadApi::new(&sym, &dep);
/// // `impact_closure_unbounded` is a BackgroundReadApi (denylist) op — it does
/// // not exist on the hot surface, so this does not compile.
/// let _ = hot.impact_closure_unbounded("a.ts", 64);
/// ```
///
/// …and the marker itself cannot be implemented outside this crate:
///
/// ```compile_fail
/// struct Rogue;
/// impl anvil_graph_cache::HotPathSurface for Rogue {} // sealed supertrait → error
/// ```
pub trait HotPathSurface: sealed::Sealed {}
impl HotPathSurface for HotReadApi<'_> {}

/// The background-pool read surface (ADR-063 **denylist**): reads that are *not*
/// hot-path-admissible. Deliberately a distinct type from [`HotReadApi`] so
/// these ops are structurally **unreachable** from a save-time / mid-edit call —
/// completeness is the background pool's job, never the hot path (ADR-063
/// miss/stale policy). It borrows the same resident graphs immutably.
///
/// Today it hosts the unbounded reverse-impact closure (retired from the hot
/// `certify` path by ADR-077); cross-file resolution and full-graph scans are
/// the other ADR-063 denylist reads that will land here as the background pool
/// grows. The constructor takes only what the current denylist read needs
/// (`dep`); it will take `sym` when a symbol-graph denylist read lands.
///
/// **Public despite having no in-crate caller yet:** the background pool that
/// consumes it lives in a *sibling* crate (`anvil-intercept`), so the surface
/// must be `pub` to be reachable there — `pub(crate)` would not do. Standing the
/// boundary up now (ADR-064's "fix the boundary once" posture) is the deliberate
/// GV2-024 decision: it gives the seal a concrete denylist home and the
/// `compile_fail` proofs something real to point at, rather than waiting for the
/// first consumer to retro-fit the split.
pub struct BackgroundReadApi<'a> {
    dep: &'a DependencyGraph,
}

impl<'a> BackgroundReadApi<'a> {
    /// Wrap the resident dependency graph for background (non-hot-path) reads.
    pub fn new(dep: &'a DependencyGraph) -> Self {
        Self { dep }
    }

    /// Unbounded transitive reverse-impact closure of `file` — the full
    /// re-export chain to any depth, bounded only by `budget` (`None` on
    /// overflow). This is the **denylist** counterpart to
    /// [`HotReadApi::reverse_impact`]'s hard-capped walk: admissible only off the
    /// hot path, where "slower but complete" is the goal.
    pub fn impact_closure_unbounded(&self, file: &str, budget: usize) -> Option<HashSet<String>> {
        crate::certify::impact_closure_unbounded(self.dep, file, budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, Visibility};

    fn symbol(id: u64, name: &str, file: &str, trust: TrustLevel) -> SymbolNode {
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

    /// One symbol per file, so `symbols_in_file`/ownership reads are simple.
    fn sym_graph() -> SymbolGraph {
        let mut g = SymbolGraph::new();
        g.add_symbol(symbol(1, "a", "a.ts", TrustLevel::Boundary))
            .unwrap();
        g.add_symbol(symbol(2, "b", "b.ts", TrustLevel::Internal))
            .unwrap();
        g
    }

    /// a.ts ← b.ts ← c.ts ; d.ts ← e.ts (two disjoint import chains).
    fn dep_graph() -> DependencyGraph {
        let mut g = DependencyGraph::new();
        g.add_dependency("b.ts".to_string(), "a.ts".to_string());
        g.add_dependency("c.ts".to_string(), "b.ts".to_string());
        g.add_dependency("e.ts".to_string(), "d.ts".to_string());
        g
    }

    #[test]
    fn hot_read_resident_symbols_warm_for_resident_file() {
        let sym = sym_graph();
        let dep = DependencyGraph::new();
        let api = HotReadApi::new(&sym, &dep);

        let read = api.resident_symbols("a.ts");
        assert!(read.is_warm());
        let symbols = read.warm().expect("warm");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "a");
    }

    #[test]
    fn hot_read_resident_symbols_miss_for_unknown_file() {
        let sym = sym_graph();
        let dep = DependencyGraph::new();
        let api = HotReadApi::new(&sym, &dep);

        // A never-extracted file degrades to a typed miss — it does NOT escalate
        // to a parse or return a false-empty surface.
        let read = api.resident_symbols("never-seen.ts");
        assert!(read.is_stale());
        assert_eq!(read.miss(), Some(HotReadMiss::WarmStateEvicted));
    }

    #[test]
    fn hot_read_symbol_owner_warm_and_miss() {
        let sym = sym_graph();
        let dep = DependencyGraph::new();
        let api = HotReadApi::new(&sym, &dep);

        assert_eq!(api.symbol_owner(1).warm(), Some("a.ts"));
        assert_eq!(
            api.symbol_owner(999).miss(),
            Some(HotReadMiss::WarmStateEvicted)
        );
    }

    #[test]
    fn hot_read_known_edge_is_always_warm() {
        let sym = SymbolGraph::new();
        let dep = dep_graph();
        let api = HotReadApi::new(&sym, &dep);

        // Present edge → Warm(true); absent edge → Warm(false), never a miss.
        assert_eq!(api.known_edge("b.ts", "a.ts"), HotRead::Warm(true));
        assert_eq!(api.known_edge("a.ts", "b.ts"), HotRead::Warm(false));
        assert_eq!(api.known_edge("nope.ts", "a.ts"), HotRead::Warm(false));
    }

    #[test]
    fn hot_read_boundary_membership_reads_resident_trust_flag() {
        let sym = sym_graph();
        let dep = DependencyGraph::new();
        let api = HotReadApi::new(&sym, &dep);

        assert_eq!(
            api.boundary_membership(1).warm(),
            Some(TrustLevel::Boundary)
        );
        assert_eq!(
            api.boundary_membership(2).warm(),
            Some(TrustLevel::Internal)
        );
        assert_eq!(
            api.boundary_membership(999).miss(),
            Some(HotReadMiss::WarmStateEvicted)
        );
    }

    #[test]
    fn hot_read_reverse_impact_depth_one_stops_at_direct_importers() {
        let sym = SymbolGraph::new();
        let dep = dep_graph();
        let api = HotReadApi::new(&sym, &dep);

        // a.ts is imported directly by b.ts; c.ts is two hops away.
        let read = api.reverse_impact("a.ts", 1, 100);
        let closure = read.warm().expect("warm");
        assert_eq!(closure, HashSet::from(["b.ts".to_string()]));
    }

    #[test]
    fn hot_read_reverse_impact_depth_two_reaches_transitive_importers() {
        let sym = SymbolGraph::new();
        let dep = dep_graph();
        let api = HotReadApi::new(&sym, &dep);

        let read = api.reverse_impact("a.ts", 2, 100);
        let closure = read.warm().expect("warm");
        assert_eq!(
            closure,
            HashSet::from(["b.ts".to_string(), "c.ts".to_string()])
        );
    }

    #[test]
    fn hot_read_reverse_impact_clamps_depth_to_hard_cap() {
        let sym = SymbolGraph::new();
        let dep = dep_graph();
        let api = HotReadApi::new(&sym, &dep);

        // A request well above the cap is clamped, not honoured — it yields the
        // same closure as the cap, never an unbounded walk.
        let capped = api.reverse_impact("a.ts", u32::MAX, 100).warm().unwrap();
        let at_cap = api
            .reverse_impact("a.ts", MAX_REVERSE_IMPACT_DEPTH, 100)
            .warm()
            .unwrap();
        assert_eq!(capped, at_cap);
    }

    #[test]
    fn hot_read_reverse_impact_overflow_is_typed_miss() {
        let sym = SymbolGraph::new();
        let dep = dep_graph();
        let api = HotReadApi::new(&sym, &dep);

        // a.ts has one direct importer (b.ts); a budget of 0 cannot hold it, so
        // the read degrades to a typed overflow miss rather than walking on.
        let read = api.reverse_impact("a.ts", 1, 0);
        assert!(read.is_stale());
        assert_eq!(read.miss(), Some(HotReadMiss::ImpactSetOverflow));
    }

    #[test]
    fn hot_read_reverse_impact_no_importers_warms_empty() {
        let sym = SymbolGraph::new();
        let dep = dep_graph();
        let api = HotReadApi::new(&sym, &dep);

        // d.ts is a leaf importer — nobody imports e.ts's importer chain head.
        let read = api.reverse_impact("d.ts", 2, 100);
        assert_eq!(read.warm(), Some(HashSet::from(["e.ts".to_string()])));

        // A file no one imports warms to an empty closure (not a miss).
        let read = api.reverse_impact("c.ts", 2, 100);
        assert_eq!(read.warm(), Some(HashSet::new()));
    }

    #[test]
    fn hot_read_reverse_impact_terminates_on_cycle() {
        // a → b → a import cycle must not loop; `seen` + origin-exclusion bound it.
        let sym = SymbolGraph::new();
        let mut dep = DependencyGraph::new();
        dep.add_dependency("a.ts".to_string(), "b.ts".to_string());
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string());
        let api = HotReadApi::new(&sym, &dep);

        let read = api.reverse_impact("a.ts", u32::MAX, 100);
        // Importers of a.ts = {b.ts}; importers of b.ts = {a.ts} (excluded).
        assert_eq!(read.warm(), Some(HashSet::from(["b.ts".to_string()])));
    }

    #[test]
    fn hot_read_reverse_impact_terminates_on_three_node_cycle() {
        // a → b → c → a: the cycle rejoins the origin two hops out, so the
        // origin-exclusion guard must fire at the rejoining node, not before.
        let sym = SymbolGraph::new();
        let mut dep = DependencyGraph::new();
        dep.add_dependency("a.ts".to_string(), "b.ts".to_string());
        dep.add_dependency("b.ts".to_string(), "c.ts".to_string());
        dep.add_dependency("c.ts".to_string(), "a.ts".to_string());
        let api = HotReadApi::new(&sym, &dep);

        // reverse_impact("a") walks importers: hop1 {c imports a? no} ...
        // edges are from→to (importer→target): a imports b, b imports c,
        // c imports a. So importers-of-a = {c}; importers-of-c = {b};
        // importers-of-b = {a} (excluded). Closure = {c, b}.
        let read = api.reverse_impact("a.ts", u32::MAX, 100);
        assert_eq!(
            read.warm(),
            Some(HashSet::from(["b.ts".to_string(), "c.ts".to_string()]))
        );
    }

    #[test]
    fn hot_read_reverse_impact_depth_zero_warms_empty() {
        // max_depth == 0 is a degenerate input below the ADR-061 §6 default of
        // 1 hop: the walk is skipped and the closure is empty (not a miss).
        let sym = SymbolGraph::new();
        let dep = dep_graph();
        let api = HotReadApi::new(&sym, &dep);

        let read = api.reverse_impact("a.ts", 0, 100);
        assert_eq!(read.warm(), Some(HashSet::new()));
    }

    #[test]
    fn hot_read_resident_symbols_miss_for_extracted_but_symbol_empty_file() {
        // A file that was extracted but yielded zero symbols leaves no `files`
        // entry, so it reads as a typed warm-miss — the documented conservative
        // contract: the daemon degrades to fallback rather than trusting a
        // false-empty surface. (Same code path as a never-seen file, asserted
        // separately so the contract is pinned.)
        let mut sym = SymbolGraph::new();
        sym.add_symbol(symbol(1, "only", "other.ts", TrustLevel::Internal))
            .unwrap();
        let dep = DependencyGraph::new();
        let api = HotReadApi::new(&sym, &dep);

        // "empty.ts" was never recorded with a symbol → non-resident → miss.
        let read = api.resident_symbols("empty.ts");
        assert_eq!(read.miss(), Some(HotReadMiss::WarmStateEvicted));
    }

    /// The denylist unbounded closure lives on [`BackgroundReadApi`] and walks
    /// past the hard cap that bounds the hot [`HotReadApi::reverse_impact`] — the
    /// type split in action: the background surface sees the full transitive
    /// chain, the hot surface truncates at the cap.
    #[test]
    fn background_impact_closure_unbounded_walks_past_the_hot_cap() {
        // a.ts ← b.ts ← c.ts ← d.ts: a 3-hop chain, one hop past the cap (2).
        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string());
        dep.add_dependency("c.ts".to_string(), "b.ts".to_string());
        dep.add_dependency("d.ts".to_string(), "c.ts".to_string());

        // Hot surface: hard-capped at MAX_REVERSE_IMPACT_DEPTH hops → {b, c}.
        let sym = SymbolGraph::new();
        let hot = HotReadApi::new(&sym, &dep);
        let capped = hot
            .reverse_impact("a.ts", MAX_REVERSE_IMPACT_DEPTH, 64)
            .warm()
            .expect("warm");
        assert_eq!(capped.len(), 2, "hot reverse_impact truncates at the cap");
        assert!(
            !capped.contains("d.ts"),
            "the 3rd hop is beyond the hot cap"
        );

        // Background surface: unbounded → reaches the full chain {b, c, d}.
        let bg = BackgroundReadApi::new(&dep);
        let full = bg
            .impact_closure_unbounded("a.ts", 64)
            .expect("within budget");
        let mut got: Vec<&str> = full.iter().map(String::as_str).collect();
        got.sort_unstable();
        assert_eq!(got, vec!["b.ts", "c.ts", "d.ts"]);
    }
}
