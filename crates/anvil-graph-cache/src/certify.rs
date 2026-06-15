//! Bounded reverse-impact certifiability for the save-time daemon
//! (ADR-061 §6, ADR-064 §5).
//!
//! [`certify`] decides whether a single changed file's verdict can be trusted
//! from the warm `(SymbolGraph, DependencyGraph)` cache alone, or whether the
//! change reaches across file boundaries so the affected set must be
//! revalidated (a `Partial` verdict the daemon reports as stale).
//!
//! # Sub-phase A contract (council verdict 2026-06-01, B4)
//!
//! Ship the **conservative export-surface default**: any change that touches a
//! file's public or privileged *symbol surface* is `Partial`; only a body-only
//! edit that leaves the public/privileged surface identical certifies
//! self-only. The export-surface decision is the `GraphDelta.previously_public`
//! / `previously_privileged` set-diff against the post-update graph — no
//! dedicated `export_surface_changed()` primitive is *mandated*, but one is
//! provided here for clarity and direct fixture coverage. This is conservatively
//! safe: `update_file` removes-all-then-re-adds, so a rename reads as
//! delete+add = surface change, and an internal→public promotion adds a new
//! public key — both fall to `Partial`.
//!
//! # Reverse-impact discovery (council verdict B1)
//!
//! Importer discovery reads [`DependencyGraph::dependents_of`] **exclusively**,
//! never `GraphDelta::removed_edges`: a removed *symbol* edge does not imply a
//! removed *file* dependency (another symbol in the same file may carry the
//! same import), so the resident reverse index is the only sound importer
//! source. GV2-003 populates `removed_edges`, but this path still must not
//! branch on it. The daemon caches the `(SymbolGraph, DependencyGraph)` pair
//! (DSV-004 Task 7) precisely so this reverse index is reachable on the hot
//! path.
//!
//! # Crate boundary (ADR-064 §2)
//!
//! The `Partial` reason carried here is **graph-cache-local**
//! ([`CertifyStale`]); the daemon orchestration (DSV-005) maps it to the wire
//! `StaleReason`. Likewise the change descriptor is the local [`ChangeKind`],
//! not `anvil-intercept`'s `CanonicalChange` (which would invert the dependency
//! graph into a cycle). Both keep this crate's frozen ADR-064 §2 dependency set
//! (`anvil-kernel-types` + `petgraph` + `serde` + `thiserror`) intact — no
//! `anvil-intercept-proto` / `anvil-intercept` dependency.

use std::collections::HashSet;
use std::path::PathBuf;

use anvil_kernel_types::{EdgeType, SymbolIdentity, SymbolKind, TrustLevel, Visibility};

use crate::dependency::DependencyGraph;
use crate::hot_index::MAX_REVERSE_IMPACT_DEPTH;
use crate::incremental::GraphDelta;
use crate::symbol_graph::SymbolGraph;
use crate::trust::is_privileged_import;

/// The shape of a single changed path, reduced to what certifiability needs.
///
/// Maps from `anvil-intercept`'s `CanonicalChange` at the daemon boundary; kept
/// local so this crate does not depend on `anvil-intercept` (ADR-064 §2). The
/// `from` path of a rename is irrelevant to certify and is dropped at the
/// mapping site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// The file's content changed in place (covers atomic-save inode flips).
    ContentModify,
    /// The path is newly present — its cross-file imports are unresolved.
    Create,
    /// The path is gone.
    Delete,
    /// The path is the destination of a rename.
    Rename,
}

/// Why a change could not be certified self-only.
///
/// Graph-cache-local: the daemon maps these to the wire `StaleReason` at the
/// boundary (DSV-005). The mapping is *not* fully 1:1 — `ImpactSetOverflow`,
/// `Deleted`, `Renamed`, and `CrossFileResolutionNeeded` match wire variant
/// names directly, but `ExportSurfaceChange` and `UnreliableGraph` have no
/// dedicated wire variant today and DSV-005 must choose their `StaleReason`
/// (e.g. an export/public-API reason and a generic stale reason respectively).
/// Variant names match the wire vocabulary where a counterpart exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertifyStale {
    /// A public/privileged symbol surface changed; importers may be affected
    /// and cannot be certified clean from the warm cache alone.
    ExportSurfaceChange,
    /// The reverse-impact closure exceeded `budget` distinct files.
    ImpactSetOverflow,
    /// The file was deleted; its importers are invalidated.
    Deleted,
    /// The file is a rename destination.
    Renamed,
    /// A newly created file whose cross-file imports are not yet resolved.
    CrossFileResolutionNeeded,
    /// The file's `update_file` reported errors, so the post-update graph is
    /// unreliable and its surface cannot be trusted. A distinct cause from a
    /// surface change — kept separate for honest telemetry/debugging.
    UnreliableGraph,
}

/// The certifiability verdict for one changed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Certifiability {
    /// The verdict is trustworthy for exactly `paths` (self-only in Sub-phase
    /// A) from the warm cache.
    Certified {
        /// Files the verdict certifies.
        paths: Vec<PathBuf>,
    },
    /// The change reaches beyond what the warm cache can certify; the daemon
    /// must report stale with the mapped reason.
    Partial {
        /// Why certification was withheld.
        reason: CertifyStale,
    },
}

/// The export-diff primitive (GV2-002): the precise change to a file's
/// public/privileged symbol surface across one update.
///
/// Replaces the boolean-only surface check with a real
/// added/removed/renamed-public-symbol diff over stable [`SymbolIdentity`]
/// keys, graduating the Sub-phase A "any touched public symbol → `partial`"
/// default towards reason-precise verdicts. Identities are
/// overload-disambiguated, so adding a same-`(kind, name)` overload to an
/// already-public symbol reads as `added_public` instead of collapsing into
/// the baseline.
///
/// Rename classification is an in-memory pairing over this single diff —
/// nothing is retained across updates and no pre-rename name is persisted
/// (privacy verdict PV-4). The pairing is deliberately conservative: a
/// removed name and an added name of the same kind are paired as a rename
/// only when each is the *sole* wholly-removed / wholly-added name of that
/// kind and their overload counts match. Anything ambiguous (two renames of
/// the same kind in one save, count mismatches) stays classified as
/// adds + removes — still a surface change, never a missed one.
///
/// # Asymmetry and residual limitation
///
/// The privileged surface (`added_privileged` / `removed_privileged`) is
/// **not** rename-classified: a privileged rename appears as one entry in
/// each list. Callers must not read `added_privileged.is_empty()` as "no
/// privileged rename happened".
///
/// One overload shape still escapes detection: removing one overload and
/// adding a different one **in the same save with the total count
/// preserved** (e.g. drop `foo` #0, append a new `foo`) re-assigns the same
/// `{foo:0, foo:1}` identity set, so the surface reads unchanged and the
/// save certifies. This was equally invisible to the old string-key scheme;
/// closing it needs per-symbol signature content, which privacy verdict
/// PV-1 deliberately excludes from identity. The common overload cases —
/// adding one (count grows) or removing one (count shrinks) — are detected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportSurfaceDiff {
    /// Public identities present after the update but not before.
    pub added_public: Vec<SymbolIdentity>,
    /// Public identities present before the update but not after.
    pub removed_public: Vec<SymbolIdentity>,
    /// Unambiguous public renames, as `(old, new)` identity pairs.
    pub renamed_public: Vec<(SymbolIdentity, SymbolIdentity)>,
    /// Privileged identities present after the update but not before.
    pub added_privileged: Vec<SymbolIdentity>,
    /// Privileged identities present before the update but not after.
    pub removed_privileged: Vec<SymbolIdentity>,
    /// Privileged *module* specifiers (e.g. `node:fs`, `child_process`) this
    /// file imports after the update but did not before — the side-effect
    /// **surface** dimension, orthogonal to the symbol-identity sets above
    /// (GV2-029).
    ///
    /// Necessary because `annotate_trust` is *file-granular*: one privileged
    /// import marks **every** symbol in the file `Privileged`. So a Boundary →
    /// Privileged escalation on an all-public file nets zero `added_privileged`
    /// against the `Privileged ∪ Boundary` baseline, and a *second* capability
    /// added to an already-privileged file (e.g. `fs` → `fs + child_process`)
    /// changes no symbol identity at all. Diffing the privileged module imports
    /// directly is a separate monotone check — fail-closed on growth, and
    /// rename-robust (a rename inside a privileged file leaves the import set
    /// untouched). De-escalation (a privileged import removed) is intentionally
    /// *not* flagged here: dropping a capability is the safe direction.
    pub newly_privileged_imports: Vec<String>,
}

impl ExportSurfaceDiff {
    /// True when the public, privileged, and privileged-import surfaces are all
    /// unchanged.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added_public.is_empty()
            && self.removed_public.is_empty()
            && self.renamed_public.is_empty()
            && self.added_privileged.is_empty()
            && self.removed_privileged.is_empty()
            && self.newly_privileged_imports.is_empty()
    }
}

/// Compute the [`ExportSurfaceDiff`] for one file update.
///
/// Compares the pre-update baselines carried on the [`GraphDelta`] against
/// the post-update graph state for the same file, over stable identities.
/// Body-only edits produce an empty diff.
///
/// Two orthogonal trust dimensions are diffed independently (a product
/// lattice: a join on one axis must never absorb a rise on the other):
///
/// 1. **Symbol-identity elevation** (`added_privileged` / `removed_privileged`):
///    the elevated-surface comparison is `Privileged ∪ Boundary` on both sides
///    (`previously_privileged ∪ previously_boundary` vs the
///    [`is_elevated_trust`](crate::incremental::is_elevated_trust) filter),
///    closing spec gap G-06. Note `annotate_trust` assigns `Boundary` to every
///    public symbol outside privileged files, so where it has run, a new public
///    symbol appears in **both** `added_public` and `added_privileged` — the
///    overlap is by construction, not double-counting.
/// 2. **Privileged-module surface** (`newly_privileged_imports`): a monotone
///    add-only diff of the file's privileged module imports against the
///    privileged subset of `previously_imported` (GV2-029). The union in (1)
///    masks a `Boundary → Privileged` escalation on an all-public file, and a
///    second capability added to an already-privileged file moves no symbol
///    identity, so this axis is what makes the daemon certify path actually
///    withhold-clean on privilege expansion (the kernel `PrivilegeExpansion`
///    invariant that would otherwise catch it is deliberately off the save-time
///    hot path — ADR-061 §6).
///
/// Reads only the `previously_*` baselines and the post-update graph; never
/// touches `delta.removed_edges` (the certify verdict reads importers from the
/// `DependencyGraph` reverse index, not the per-update edge churn — GV2-003
/// populates `removed_edges`, but this path still does not branch on it).
#[must_use]
pub fn export_surface_diff(sym: &SymbolGraph, delta: &GraphDelta) -> ExportSurfaceDiff {
    let current = sym.symbols_in_file(&delta.file);
    let identities = SymbolIdentity::for_file_symbols(&current);

    let current_public: HashSet<SymbolIdentity> = current
        .iter()
        .zip(&identities)
        .filter(|(s, _)| s.visibility == Visibility::Public)
        .map(|(_, identity)| identity.clone())
        .collect();
    let current_elevated: HashSet<SymbolIdentity> = current
        .iter()
        .zip(&identities)
        .filter(|(s, _)| crate::incremental::is_elevated_trust(s.trust_level))
        .map(|(_, identity)| identity.clone())
        .collect();
    let baseline_elevated: HashSet<SymbolIdentity> = delta
        .previously_privileged
        .union(&delta.previously_boundary)
        .cloned()
        .collect();

    let mut added_public: Vec<SymbolIdentity> = current_public
        .difference(&delta.previously_public)
        .cloned()
        .collect();
    let mut removed_public: Vec<SymbolIdentity> = delta
        .previously_public
        .difference(&current_public)
        .cloned()
        .collect();
    added_public.sort();
    removed_public.sort();

    let renamed_public = pair_renames(
        &mut added_public,
        &mut removed_public,
        &delta.previously_public,
        &current_public,
    );

    let mut added_privileged: Vec<SymbolIdentity> = current_elevated
        .difference(&baseline_elevated)
        .cloned()
        .collect();
    let mut removed_privileged: Vec<SymbolIdentity> = baseline_elevated
        .difference(&current_elevated)
        .cloned()
        .collect();
    added_privileged.sort();
    removed_privileged.sort();

    // GV2-029: the side-effect-surface dimension. The file's privileged module
    // imports now (read off the post-update graph's resolved `Imports` targets —
    // a `node:fs` import resolves to a synthetic external `Module` node whose
    // `file` is the specifier `"node:fs"`) vs the privileged subset of the
    // pre-update `previously_imported` baseline. A monotone add-only diff: a
    // privileged module the file did not import before is an escalation,
    // independent of the symbol-identity sets above (which `annotate_trust`'s
    // file-granular `Privileged` stamping makes blind to it).
    //
    // Only synthetic *external* module placeholders count (`resolve_import`
    // stamps bare specifiers `kind = Module, trust = External`). Without that
    // guard a benign relative import to a project file named exactly `net`/`fs`/
    // … (resolved `file` == the bare token) would be misclassified privileged by
    // `is_privileged_import` and falsely withhold. Resolved relative imports
    // never produce an `External` `Module` node, so the guard excludes them.
    //
    // GV2-031: re-exports are now lifted into `EdgeType::Reexports` edges, so a
    // privileged capability reached via `export * from 'node:fs'` (directly or
    // through a re-export chain) is no longer invisible. `current_privileged`
    // is the union of two surfaces: privileged modules the file *imports*
    // (direct `Imports` edges) and privileged modules it *re-exports*
    // (`reexported_privileged_modules` follows `Reexports` edges transitively).
    let mut current_privileged: HashSet<String> = current
        .iter()
        .flat_map(|s| sym.outgoing_edges(s.id))
        .filter(|e| e.edge_type == EdgeType::Imports)
        .filter_map(|e| sym.get_symbol(e.to))
        .filter(|t| {
            t.kind == SymbolKind::Module
                && t.trust_level == TrustLevel::External
                && is_privileged_import(&t.file)
        })
        .map(|t| t.file.clone())
        .collect();
    current_privileged.extend(crate::trust::reexported_privileged_modules(
        sym,
        &delta.file,
    ));

    // Monotone add-only diff: a privileged module the file did not reach before
    // — by import (`previously_imported`) or by re-export
    // (`previously_reexported_privileged`) — is the escalation. Subtracting both
    // baselines keeps a pre-existing privileged re-export from re-tripping on an
    // unrelated edit, matching the import dimension's behaviour.
    let mut newly_privileged_imports: Vec<String> = current_privileged
        .iter()
        .filter(|src| {
            !delta.previously_imported.contains(*src)
                && !delta.previously_reexported_privileged.contains(*src)
        })
        .cloned()
        .collect();
    newly_privileged_imports.sort();

    // Counts only — never identity values — so the trace stays inside the
    // privacy verdict's PV-10 label rules even if it is ever piped further.
    tracing::debug!(
        target: "anvil_graph_cache::certify",
        added_public = added_public.len(),
        removed_public = removed_public.len(),
        renamed_public = renamed_public.len(),
        added_privileged = added_privileged.len(),
        removed_privileged = removed_privileged.len(),
        newly_privileged_imports = newly_privileged_imports.len(),
        "export surface diff computed"
    );

    ExportSurfaceDiff {
        added_public,
        removed_public,
        renamed_public,
        added_privileged,
        removed_privileged,
        newly_privileged_imports,
    }
}

/// Pair unambiguous renames out of the public add/remove lists.
///
/// A name is *wholly removed* when no identity with its `(kind, name)`
/// remains in the current set, and *wholly added* when none existed in the
/// baseline. For each kind with exactly one wholly-removed and exactly one
/// wholly-added name carrying the same overload count, the pair is
/// classified as a rename (ordinal-by-ordinal) and its entries are drained
/// from `added` / `removed`. Everything else is left as adds + removes.
fn pair_renames(
    added: &mut Vec<SymbolIdentity>,
    removed: &mut Vec<SymbolIdentity>,
    baseline: &HashSet<SymbolIdentity>,
    current: &HashSet<SymbolIdentity>,
) -> Vec<(SymbolIdentity, SymbolIdentity)> {
    // BTreeMap for deterministic iteration order — pair selection must be
    // stable across runs.
    use std::collections::BTreeMap;

    let group = |items: &[SymbolIdentity],
                 still_present: &HashSet<SymbolIdentity>|
     -> BTreeMap<SymbolKind, BTreeMap<String, u32>> {
        // Precompute the present (kind, name) pairs once so the membership
        // probe below is O(1) instead of rescanning the whole set per item.
        let present_pairs: HashSet<(SymbolKind, &str)> = still_present
            .iter()
            .map(|c| (c.kind, c.name.as_str()))
            .collect();
        let mut by_kind: BTreeMap<SymbolKind, BTreeMap<String, u32>> = BTreeMap::new();
        for identity in items {
            // "Wholly" gone/new: no ordinal of this (kind, name) survives in
            // (resp. pre-existed in) the other set.
            if !present_pairs.contains(&(identity.kind, identity.name.as_str())) {
                *by_kind
                    .entry(identity.kind)
                    .or_default()
                    .entry(identity.name.clone())
                    .or_insert(0) += 1;
            }
        }
        by_kind
    };

    let wholly_removed = group(removed, current);
    let wholly_added = group(added, baseline);

    let mut renames = Vec::new();
    for (kind, removed_names) in &wholly_removed {
        let Some(added_names) = wholly_added.get(kind) else {
            continue;
        };
        // Unambiguous only: one candidate on each side, equal overload counts.
        if removed_names.len() != 1 || added_names.len() != 1 {
            tracing::debug!(
                target: "anvil_graph_cache::certify",
                removed_candidates = removed_names.len(),
                added_candidates = added_names.len(),
                "ambiguous rename shape — conservatively kept as adds + removes"
            );
            continue;
        }
        let (old_name, old_count) = removed_names.iter().next().expect("len checked");
        let (new_name, new_count) = added_names.iter().next().expect("len checked");
        if old_count != new_count {
            continue;
        }
        let mut old_ids: Vec<SymbolIdentity> = removed
            .iter()
            .filter(|i| i.kind == *kind && i.name == *old_name)
            .cloned()
            .collect();
        let mut new_ids: Vec<SymbolIdentity> = added
            .iter()
            .filter(|i| i.kind == *kind && i.name == *new_name)
            .cloned()
            .collect();
        // Pair ordinal-by-ordinal, explicitly — independent of the field
        // order behind SymbolIdentity's derived Ord.
        old_ids.sort_by_key(|i| i.ordinal);
        new_ids.sort_by_key(|i| i.ordinal);
        removed.retain(|i| !(i.kind == *kind && i.name == *old_name));
        added.retain(|i| !(i.kind == *kind && i.name == *new_name));
        renames.extend(old_ids.into_iter().zip(new_ids));
    }
    renames
}

/// Does this update change the file's public or privileged symbol surface?
///
/// Boolean convenience over [`export_surface_diff`]; see it for the precise
/// added/removed/renamed breakdown.
#[must_use]
pub fn export_surface_changed(sym: &SymbolGraph, delta: &GraphDelta) -> bool {
    !export_surface_diff(sym, delta).is_empty()
}

/// Clamp a requested reverse-impact hop depth into the ADR-063 §3 envelope: the
/// 1-hop default floor and the [`MAX_REVERSE_IMPACT_DEPTH`] hard-cap ceiling.
///
/// This is the **runtime lever** GV2-026 exposes (ADR-063 §3): the
/// latency↔coverage trade can move 1 → 2 hops "without re-coding", but an
/// over-cap setting is **clamped, not honoured** — `clamp(1, 2)` folds 0/unset
/// up to the 1-hop default and any over-cap request down to the hard cap. Pure
/// and deterministic, so the config layer can resolve the lever once and the hot
/// path stays free of policy.
#[must_use]
pub fn clamp_reverse_impact_depth(requested: u32) -> u32 {
    requested.clamp(1, MAX_REVERSE_IMPACT_DEPTH)
}

/// The save-time (hot-path) certifiability closure: a breadth-first reverse-impact
/// walk of `file` hard-capped at the ADR-063 ceiling [`MAX_REVERSE_IMPACT_DEPTH`]
/// (ADR-077, path A). `certify` calls this directly.
///
/// Walks `dependents_of` outward (direct importers, then their importers — the
/// re-export chain), deduplicating. Returns `None` the moment the distinct
/// impacted set would exceed `budget` (the [`CertifyStale::ImpactSetOverflow`]
/// signal). An over-cap chain is **truncated**, never walked unbounded, so no
/// transitive traversal beyond the ceiling is reachable on the hot path. Capping
/// is monotone — it can only *shrink* the closure — so it only ever flips an
/// over-cap-chain verdict from `ImpactSetOverflow` to `ExportSurfaceChange` (both
/// `Partial`); coverage and soundness are unchanged (the closure sizes the stale
/// reason but is never inline-validated).
///
/// **Origin on a cycle:** `file` is not seeded into the impacted set and, on an
/// acyclic graph, never enters it. On a cyclic import graph it *can* re-enter as
/// a dependent of one of its own importers and then count against `budget` — this
/// is the pre-ADR-077 walk's behavior, preserved deliberately so the depth cap is
/// the *only* change here (it differs from [`crate::hot_index::HotReadApi::reverse_impact`],
/// which excludes `file` explicitly; both verdicts stay `Partial` regardless).
///
/// `depth` MUST be within the ADR-063 hard cap on any save-time call. A
/// `debug_assert` enforces it, so a future caller that lifts the cap on the hot
/// path trips under test rather than silently reintroducing an unbounded walk
/// (the runtime 1→2-hop lever stays GV2-026's scope, still under the cap).
pub(crate) fn bounded_impact_closure(
    dep: &DependencyGraph,
    file: &str,
    depth: u32,
    budget: usize,
) -> Option<HashSet<String>> {
    debug_assert!(
        depth <= MAX_REVERSE_IMPACT_DEPTH,
        "hot-path reverse-impact depth {depth} exceeds the ADR-063 cap {MAX_REVERSE_IMPACT_DEPTH}"
    );
    let mut impacted: HashSet<String> = HashSet::new();
    let mut frontier: Vec<String> = vec![file.to_string()];

    for _ in 0..depth {
        let mut next: Vec<String> = Vec::new();
        for current in &frontier {
            for importer in dep.dependents_of(current) {
                if impacted.insert(importer.to_string()) {
                    if impacted.len() > budget {
                        return None;
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

    Some(impacted)
}

/// Unbounded transitive reverse-impact closure — the **ADR-063 denylist** walk,
/// reachable only via [`crate::hot_index::BackgroundReadApi`], never the hot
/// path. Walks the full re-export chain to any depth, bounded only by `budget`
/// (`None` on overflow). This is the pre-ADR-077 closure, retired from save-time
/// `certify`; it stays valid for the background pool, where "slower but
/// complete" is the goal (ADR-063 miss/stale policy).
pub(crate) fn impact_closure_unbounded(
    dep: &DependencyGraph,
    file: &str,
    budget: usize,
) -> Option<HashSet<String>> {
    let mut impacted: HashSet<String> = HashSet::new();
    let mut frontier: Vec<String> = vec![file.to_string()];

    while let Some(current) = frontier.pop() {
        for importer in dep.dependents_of(&current) {
            if impacted.insert(importer.to_string()) {
                if impacted.len() > budget {
                    return None;
                }
                frontier.push(importer.to_string());
            }
        }
    }

    Some(impacted)
}

/// Decide whether `change` to `delta.file` can be certified self-only.
///
/// See the module docs for the Sub-phase A contract. In short:
/// - `Delete` / `Rename` / `Create` are never certified — they map to their
///   dedicated stale reasons.
/// - A `ContentModify` whose `update_file` reported errors is never certified —
///   the post-update graph state for the file is unreliable.
/// - `ContentModify` with no public/privileged surface change certifies
///   self-only (`Certified{[file]}`).
/// - `ContentModify` with a surface change is `Partial`: `ImpactSetOverflow`
///   if the importer closure exceeds `budget`, otherwise `ExportSurfaceChange`.
///
/// # Precondition
///
/// `sym` must reflect the post-update state of `delta.file` (i.e. `update_file`
/// for this change has already been applied to `sym`). `export_surface_changed`
/// reads the file's *current* symbols from `sym`; if `sym` were stale, an
/// absent file would read as "no public surface" and a body-only change would
/// false-certify. The daemon's `apply_delta` (DSV-004 Task 7) guarantees this
/// ordering; the `delta.errors` guard below is the defence-in-depth backstop
/// for the one in-band failure mode (`update_file` partially failing) that can
/// leave `sym` inconsistent while still returning a delta.
///
/// # Surface decision (GV2-002)
///
/// The surface decision is a set-diff of the `previously_*` baselines over
/// stable [`SymbolIdentity`] keys (`incremental.rs` builds them). Identities
/// are overload-disambiguated by ordinal, so the former Sub-phase A
/// limitation — a second public symbol with the same `kind` and `name`
/// collapsing into one key and reading as no surface change — is closed for
/// count-changing overload edits; see [`ExportSurfaceDiff`] for the one
/// count-preserving shape that remains undetectable by design.
///
/// The elevated surface unions `TrustLevel::Privileged` and
/// `TrustLevel::Boundary` (`incremental::is_elevated_trust`), closing spec
/// gap G-06 on the export-diff path. The `PrivilegeExpansion` policy
/// invariant deliberately stays `Privileged`-only against the
/// `Privileged`-only `previously_privileged` baseline — see
/// `incremental::GraphDelta::previously_boundary` for the split's rationale.
///
/// # Reverse-impact depth (GV2-026)
///
/// `max_depth` is the runtime reverse-impact hop lever (ADR-063 §3). It is
/// clamped via [`clamp_reverse_impact_depth`] into `1..=MAX_REVERSE_IMPACT_DEPTH`
/// before the closure walk — the 1-hop default holds when unset/0, and an
/// over-cap request is clamped, not honoured. Because the impacted set is only
/// used to distinguish overflow from a bounded impact set (the verdict is
/// `Partial` either way once the surface changed), the lever only moves overflow
/// sensitivity; it never flips a clean (`Certified`) verdict to `Partial` or
/// vice versa.
#[must_use]
pub fn certify(
    sym: &SymbolGraph,
    dep: &DependencyGraph,
    change: &ChangeKind,
    delta: &GraphDelta,
    budget: usize,
    max_depth: u32,
) -> Certifiability {
    match change {
        ChangeKind::Delete => Certifiability::Partial {
            reason: CertifyStale::Deleted,
        },
        ChangeKind::Rename => Certifiability::Partial {
            reason: CertifyStale::Renamed,
        },
        ChangeKind::Create => Certifiability::Partial {
            reason: CertifyStale::CrossFileResolutionNeeded,
        },
        ChangeKind::ContentModify => {
            // Defence in depth: a partial `update_file` failure leaves `sym`
            // inconsistent for this file, so its surface cannot be trusted —
            // never certify clean off an unreliable graph.
            if !delta.errors.is_empty() {
                return Certifiability::Partial {
                    reason: CertifyStale::UnreliableGraph,
                };
            }
            let surface = export_surface_diff(sym, delta);
            if surface.is_empty() {
                return Certifiability::Certified {
                    paths: vec![PathBuf::from(&delta.file)],
                };
            }
            // GV2-029: surface a privilege escalation above `debug` so an
            // operator can see (and rate-track) the daemon withholding clean on
            // a newly-privileged surface — a bad shared-module deploy that now
            // imports `node:fs` should not be invisible. Gated on
            // `newly_privileged_imports` only: that is the unambiguous
            // new-capability signal, whereas `added_privileged` is the
            // `Privileged ∪ Boundary` diff and so also fires on ordinary public
            // API additions (a new public symbol is `Boundary`). Counts only,
            // never identity/path values, to stay inside the PV-10 label rules.
            if !surface.newly_privileged_imports.is_empty() {
                tracing::warn!(
                    target: "anvil_graph_cache::certify",
                    newly_privileged_imports = surface.newly_privileged_imports.len(),
                    "certify withholding clean: privileged module surface expanded"
                );
            }
            // The closure is computed only to distinguish overflow from a
            // bounded impact set; the impacted paths themselves are unused in
            // Sub-phase A (every surface change is `Partial` regardless). The
            // hop depth is the GV2-026 runtime lever, clamped into the ADR-063
            // §3 envelope (1-hop default, hard-capped at MAX_REVERSE_IMPACT_DEPTH
            // per ADR-077 path A) — an over-cap request is clamped, not honoured.
            let depth = clamp_reverse_impact_depth(max_depth);
            match bounded_impact_closure(dep, &delta.file, depth, budget) {
                None => Certifiability::Partial {
                    reason: CertifyStale::ImpactSetOverflow,
                },
                Some(_) => Certifiability::Partial {
                    reason: CertifyStale::ExportSurfaceChange,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel};

    /// Build a graph holding one file with the given `(name, visibility,
    /// trust)` symbols, and a `GraphDelta` whose `previously_*` baselines are
    /// seeded from `baseline` (the pre-update public/privileged surface).
    fn sym_with(file: &str, symbols: &[(&str, Visibility, TrustLevel)]) -> SymbolGraph {
        let mut g = SymbolGraph::new();
        for (i, (name, vis, trust)) in symbols.iter().enumerate() {
            g.add_symbol(SymbolNode {
                id: i as u64,
                kind: SymbolKind::Function,
                name: (*name).to_string(),
                visibility: *vis,
                file: file.to_string(),
                trust_level: *trust,
            })
            .unwrap();
        }
        g
    }

    fn key(file: &str, name: &str) -> SymbolIdentity {
        SymbolIdentity {
            file: file.to_string(),
            kind: SymbolKind::Function,
            name: name.to_string(),
            ordinal: 0,
        }
    }

    fn delta_for(file: &str, prev_public: &[&str], prev_privileged: &[&str]) -> GraphDelta {
        GraphDelta {
            file: file.to_string(),
            previously_public: prev_public.iter().map(|n| key(file, n)).collect(),
            previously_privileged: prev_privileged.iter().map(|n| key(file, n)).collect(),
            ..Default::default()
        }
    }

    // ---- export_surface_changed: B4-required fixtures ----

    #[test]
    fn identity_overload_added_to_already_public_symbol_is_surface_change() {
        // GV2-002 red test: a.ts had one public `foo`; an overload (second
        // public symbol with the same kind+name) is added. The string-keyed
        // set collapses both into one key, so the surface reads unchanged
        // and the change falsely certifies clean.
        let sym = sym_with(
            "a.ts",
            &[
                ("foo", Visibility::Public, TrustLevel::Unknown),
                ("foo", Visibility::Public, TrustLevel::Unknown),
            ],
        );
        let delta = delta_for("a.ts", &["foo"], &[]);
        assert!(
            export_surface_changed(&sym, &delta),
            "adding a public overload must read as a surface change"
        );
    }

    #[test]
    fn body_only_change_certifies_self_only() {
        // Public surface identical before and after → body-only.
        let sym = sym_with("a.ts", &[("foo", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        assert!(!export_surface_changed(&sym, &delta));

        let v = certify(
            &sym,
            &DependencyGraph::new(),
            &ChangeKind::ContentModify,
            &delta,
            64,
            1,
        );
        assert_eq!(
            v,
            Certifiability::Certified {
                paths: vec![PathBuf::from("a.ts")]
            }
        );
    }

    #[test]
    fn touched_public_symbol_defaults_to_partial() {
        // Was public `foo`; now public `bar` (the public key set differs).
        let sym = sym_with("a.ts", &[("bar", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        assert!(export_surface_changed(&sym, &delta));
    }

    #[test]
    fn rename_is_export_surface_change() {
        // A public symbol renamed within the file: delete `foo` + add `baz`.
        let sym = sym_with("a.ts", &[("baz", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        assert!(export_surface_changed(&sym, &delta));
    }

    #[test]
    fn delete_is_export_surface_change() {
        // A public symbol removed: post-update file has no public surface.
        let sym = sym_with(
            "a.ts",
            &[("priv_only", Visibility::Internal, TrustLevel::Unknown)],
        );
        let delta = delta_for("a.ts", &["foo"], &[]);
        assert!(export_surface_changed(&sym, &delta));
    }

    #[test]
    fn internal_to_public_defaults_to_partial() {
        // A symbol promoted from internal to public adds a new public key.
        let sym = sym_with("a.ts", &[("foo", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &[], &[]);
        assert!(export_surface_changed(&sym, &delta));
    }

    #[test]
    fn reexport_add_remove_is_surface_change() {
        // Re-export add: a new public symbol appears alongside the existing one.
        let sym = sym_with(
            "a.ts",
            &[
                ("foo", Visibility::Public, TrustLevel::Unknown),
                ("reexported", Visibility::Public, TrustLevel::Unknown),
            ],
        );
        let delta = delta_for("a.ts", &["foo"], &[]);
        assert!(export_surface_changed(&sym, &delta));
    }

    #[test]
    fn privilege_expansion_is_surface_change() {
        // No public change, but a symbol became Privileged.
        let sym = sym_with(
            "a.ts",
            &[("op", Visibility::Internal, TrustLevel::Privileged)],
        );
        let delta = delta_for("a.ts", &[], &[]);
        assert!(export_surface_changed(&sym, &delta));
    }

    #[test]
    fn identity_boundary_trust_is_elevated_surface() {
        // G-06 regression: a symbol crossing onto TrustLevel::Boundary is a
        // privilege-surface change, exactly like Privileged.
        let sym = sym_with(
            "a.ts",
            &[("op", Visibility::Internal, TrustLevel::Boundary)],
        );
        let delta = delta_for("a.ts", &[], &[]);
        let diff = export_surface_diff(&sym, &delta);
        assert_eq!(diff.added_privileged.len(), 1);
        assert!(export_surface_changed(&sym, &delta));
    }

    /// Attach an `Imports` edge from `a.ts`'s symbol id 0 to a target node, so
    /// the module-surface diff (GV2-029) has an edge to read.
    fn graph_importing(target: SymbolNode) -> SymbolGraph {
        use anvil_kernel_types::{EdgeType, SymbolEdge};
        let mut g = sym_with(
            "a.ts",
            &[("entry", Visibility::Public, TrustLevel::Unknown)],
        );
        let target_id = target.id;
        g.add_symbol(target).unwrap();
        g.add_edge(SymbolEdge {
            from: 0,
            to: target_id,
            edge_type: EdgeType::Imports,
        })
        .unwrap();
        g
    }

    #[test]
    fn newly_privileged_module_import_is_surface_change() {
        // GV2-029: a.ts imports an external `node:fs` module node not present in
        // previously_imported → the module-surface dimension fires.
        let sym = graph_importing(SymbolNode {
            id: 100,
            kind: SymbolKind::Module,
            name: "node:fs".to_string(),
            visibility: Visibility::Public,
            file: "node:fs".to_string(),
            trust_level: TrustLevel::External,
        });
        let delta = delta_for("a.ts", &["entry"], &[]); // previously_imported empty
        let diff = export_surface_diff(&sym, &delta);
        assert_eq!(diff.newly_privileged_imports, vec!["node:fs".to_string()]);
        assert!(export_surface_changed(&sym, &delta));
    }

    #[test]
    fn relative_import_to_file_named_like_a_module_does_not_fire() {
        // Attack-3 guard: a benign relative import resolving to a project file
        // literally named "net" (a real Function symbol, Internal trust — NOT an
        // External Module placeholder) must NOT be misread as a privileged
        // module import, even though is_privileged_import("net") is true.
        let sym = graph_importing(SymbolNode {
            id: 100,
            kind: SymbolKind::Function,
            name: "handler".to_string(),
            visibility: Visibility::Public,
            file: "net".to_string(),
            trust_level: TrustLevel::Internal,
        });
        let delta = delta_for("a.ts", &["entry"], &[]);
        let diff = export_surface_diff(&sym, &delta);
        assert!(
            diff.newly_privileged_imports.is_empty(),
            "a relative import to a project file named 'net' must not read as a privileged module"
        );
    }

    /// GV2-031: a re-export of a privileged module surfaces in the
    /// privileged-module diff just like a direct import. `a.ts` re-exports
    /// `node:fs` via a `Reexports` edge with `node:fs` absent from
    /// `previously_imported`/`previously_reexported_privileged`.
    #[test]
    fn newly_privileged_module_reexport_is_surface_change() {
        use anvil_kernel_types::{EdgeType, SymbolEdge};
        let mut sym = sym_with(
            "a.ts",
            &[("entry", Visibility::Public, TrustLevel::Unknown)],
        );
        sym.add_symbol(SymbolNode {
            id: 100,
            kind: SymbolKind::Module,
            name: "node:fs".to_string(),
            visibility: Visibility::Public,
            file: "node:fs".to_string(),
            trust_level: TrustLevel::External,
        })
        .unwrap();
        sym.add_edge(SymbolEdge {
            from: 0,
            to: 100,
            edge_type: EdgeType::Reexports,
        })
        .unwrap();

        let diff = export_surface_diff(&sym, &delta_for("a.ts", &["entry"], &[]));
        assert_eq!(diff.newly_privileged_imports, vec!["node:fs".to_string()]);
    }

    /// GV2-031 monotonicity: a privileged module the file *already* re-exported
    /// (recorded in `previously_reexported_privileged`) does not re-fire on an
    /// unrelated edit, matching the direct-import baseline behaviour.
    #[test]
    fn preexisting_privileged_reexport_does_not_refire() {
        use anvil_kernel_types::{EdgeType, SymbolEdge};
        let mut sym = sym_with(
            "a.ts",
            &[("entry", Visibility::Public, TrustLevel::Unknown)],
        );
        sym.add_symbol(SymbolNode {
            id: 100,
            kind: SymbolKind::Module,
            name: "node:fs".to_string(),
            visibility: Visibility::Public,
            file: "node:fs".to_string(),
            trust_level: TrustLevel::External,
        })
        .unwrap();
        sym.add_edge(SymbolEdge {
            from: 0,
            to: 100,
            edge_type: EdgeType::Reexports,
        })
        .unwrap();

        let mut delta = delta_for("a.ts", &["entry"], &[]);
        delta
            .previously_reexported_privileged
            .insert("node:fs".to_string());

        let diff = export_surface_diff(&sym, &delta);
        assert!(
            diff.newly_privileged_imports.is_empty(),
            "a pre-existing privileged re-export must not re-fire, got {:?}",
            diff.newly_privileged_imports
        );
    }

    #[test]
    fn certify_never_reads_removed_edges() {
        // removed_edges is non-empty here, but certify must ignore it: a
        // body-only edit (identical public surface) still certifies self-only.
        let sym = sym_with("a.ts", &[("foo", Visibility::Public, TrustLevel::Unknown)]);
        let mut delta = delta_for("a.ts", &["foo"], &[]);
        delta
            .removed_edges
            .push((1, 2, anvil_kernel_types::EdgeType::Imports));
        let v = certify(
            &sym,
            &DependencyGraph::new(),
            &ChangeKind::ContentModify,
            &delta,
            64,
            1,
        );
        assert!(matches!(v, Certifiability::Certified { .. }));
    }

    // ---- certify: change-kind handling ----

    #[test]
    fn content_modify_no_export_change_certifies_self_only() {
        let sym = sym_with("a.ts", &[("foo", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let v = certify(
            &sym,
            &DependencyGraph::new(),
            &ChangeKind::ContentModify,
            &delta,
            64,
            1,
        );
        assert_eq!(
            v,
            Certifiability::Certified {
                paths: vec![PathBuf::from("a.ts")]
            }
        );
    }

    #[test]
    fn delete_invalidates_importers() {
        let sym = sym_with("a.ts", &[("foo", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let v = certify(
            &sym,
            &DependencyGraph::new(),
            &ChangeKind::Delete,
            &delta,
            64,
            1,
        );
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::Deleted
            }
        );
    }

    #[test]
    fn rename_change_kind_is_partial_renamed() {
        let sym = sym_with("a.ts", &[("foo", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let v = certify(
            &sym,
            &DependencyGraph::new(),
            &ChangeKind::Rename,
            &delta,
            64,
            1,
        );
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::Renamed
            }
        );
    }

    #[test]
    fn create_is_partial_cross_file_resolution() {
        let sym = sym_with("a.ts", &[("foo", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let v = certify(
            &sym,
            &DependencyGraph::new(),
            &ChangeKind::Create,
            &delta,
            64,
            1,
        );
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::CrossFileResolutionNeeded
            }
        );
    }

    // ---- certify: reverse-impact closure (B1) ----

    #[test]
    fn export_surface_change_pulls_in_direct_importers() {
        // foo renamed (surface change); b.ts imports a.ts. Within budget, the
        // verdict is Partial (the importer is pulled into the impact set and
        // cannot be certified clean), not Certified.
        let sym = sym_with("a.ts", &[("baz", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string());

        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 64, 1);
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::ExportSurfaceChange
            }
        );
    }

    #[test]
    fn new_export_making_unchanged_importer_illegal_is_not_certified_clean() {
        // The soundness headline: adding a public export (surface change) must
        // never certify clean, even though the importer file itself is
        // unchanged.
        let sym = sym_with(
            "a.ts",
            &[
                ("foo", Visibility::Public, TrustLevel::Unknown),
                ("new_export", Visibility::Public, TrustLevel::Unknown),
            ],
        );
        let delta = delta_for("a.ts", &["foo"], &[]);
        let mut dep = DependencyGraph::new();
        dep.add_dependency("importer.ts".to_string(), "a.ts".to_string());

        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 64, 1);
        assert!(
            !matches!(v, Certifiability::Certified { .. }),
            "a surface-expanding change must not be certified clean"
        );
    }

    #[test]
    fn reexport_chain_recurses_within_budget() {
        // c.ts -> b.ts -> a.ts; a surface change to a.ts recurses through the
        // re-export chain and stays within budget → ExportSurfaceChange, not
        // overflow.
        let sym = sym_with("a.ts", &[("baz", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string());
        dep.add_dependency("c.ts".to_string(), "b.ts".to_string());

        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 64, 1);
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::ExportSurfaceChange
            }
        );
    }

    #[test]
    fn overflow_returns_partial() {
        // budget = 1, two distinct importers → the closure overflows.
        let sym = sym_with("a.ts", &[("baz", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string());
        dep.add_dependency("c.ts".to_string(), "a.ts".to_string());

        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 1, 1);
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::ImpactSetOverflow
            }
        );
    }

    /// a.ts ← b.ts ← c.ts ← {d,e,f}: the unbounded closure is `{b,c,d,e,f}` (5),
    /// but ADR-077 hard-depth-caps the certifiability closure at
    /// `MAX_REVERSE_IMPACT_DEPTH` (2) so it truncates to `{b,c}` — no unbounded
    /// transitive walk is reachable on the hot path.
    #[test]
    fn impact_closure_is_hard_depth_capped_adr077() {
        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string()); // depth 1
        dep.add_dependency("c.ts".to_string(), "b.ts".to_string()); // depth 2
        for importer in ["d.ts", "e.ts", "f.ts"] {
            dep.add_dependency(importer.to_string(), "c.ts".to_string()); // depth 3 (> cap)
        }
        let closure = bounded_impact_closure(&dep, "a.ts", MAX_REVERSE_IMPACT_DEPTH, 64)
            .expect("within budget");
        let mut got: Vec<&str> = closure.iter().map(String::as_str).collect();
        got.sort_unstable();
        assert_eq!(
            got,
            vec!["b.ts", "c.ts"],
            "closure truncates at MAX_REVERSE_IMPACT_DEPTH={MAX_REVERSE_IMPACT_DEPTH} hops"
        );
    }

    /// Same 3-hop chain with `budget = 3`: the unbounded closure (5) would
    /// overflow → `ImpactSetOverflow`, but the depth-capped closure (`{b,c}` = 2)
    /// fits → `ExportSurfaceChange`. ADR-077: capping is monotone (it can only
    /// turn overflow into surface-change, never the reverse) and verdict-neutral
    /// (both are `Partial`).
    #[test]
    fn over_cap_chain_within_budget_is_surface_change_not_overflow_adr077() {
        let sym = sym_with("a.ts", &[("baz", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string());
        dep.add_dependency("c.ts".to_string(), "b.ts".to_string());
        for importer in ["d.ts", "e.ts", "f.ts"] {
            dep.add_dependency(importer.to_string(), "c.ts".to_string());
        }
        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 3, 1);
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::ExportSurfaceChange
            }
        );
    }

    /// ADR-063/077 admission guard: the shared bounded walk debug-asserts its
    /// depth stays within the hard cap, so a hot-path caller that requested an
    /// over-cap walk trips under test rather than silently traversing unbounded.
    /// Debug-only (the `debug_assert` is compiled out in release).
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "exceeds the ADR-063 cap")]
    fn admission_bounded_closure_trips_beyond_cap() {
        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string());
        let _ = bounded_impact_closure(&dep, "a.ts", MAX_REVERSE_IMPACT_DEPTH + 1, 64);
    }

    #[test]
    fn certify_uses_dependency_graph_reverse_not_symbol_graph_scan() {
        // The impact closure must read dep.dependents_of, not scan the
        // SymbolGraph. Proof: the SymbolGraph holds no import edges at all, yet
        // a surface change with importers recorded only in the DependencyGraph
        // overflows a zero budget. A SymbolGraph scan would see no importers
        // and could not overflow.
        let sym = sym_with("a.ts", &[("baz", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);

        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string());
        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 0, 1);
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::ImpactSetOverflow
            },
            "importers live only in the DependencyGraph; reading it must overflow budget 0"
        );

        // Same change, empty DependencyGraph → no importers → within budget.
        let v_empty = certify(
            &sym,
            &DependencyGraph::new(),
            &ChangeKind::ContentModify,
            &delta,
            0,
            1,
        );
        assert_eq!(
            v_empty,
            Certifiability::Partial {
                reason: CertifyStale::ExportSurfaceChange
            },
            "with no reverse edges the closure is empty and does not overflow"
        );
    }

    #[test]
    fn update_errors_force_partial() {
        // A body-only-looking delta (identical public surface) must NOT certify
        // when update_file reported an error — the graph is unreliable for the
        // file. Defence in depth against a partial-update stale-graph race.
        let sym = sym_with("a.ts", &[("foo", Visibility::Public, TrustLevel::Unknown)]);
        let mut delta = delta_for("a.ts", &["foo"], &[]);
        delta.errors.push("symbol 7: duplicate".to_string());
        let v = certify(
            &sym,
            &DependencyGraph::new(),
            &ChangeKind::ContentModify,
            &delta,
            64,
            1,
        );
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::UnreliableGraph
            },
            "an update with errors must report UnreliableGraph, not a surface change"
        );
    }

    #[test]
    fn non_function_kind_surface_change_is_partial() {
        // Surface detection must work for any SymbolKind, not just Function:
        // a public Class added where none existed is a surface change.
        let mut g = SymbolGraph::new();
        g.add_symbol(SymbolNode {
            id: 0,
            kind: SymbolKind::Class,
            name: "Widget".to_string(),
            visibility: Visibility::Public,
            file: "a.ts".to_string(),
            trust_level: TrustLevel::Unknown,
        })
        .unwrap();
        // previously_public empty → the public Class is new → surface change.
        let delta = delta_for("a.ts", &[], &[]);
        assert!(export_surface_changed(&g, &delta));
        let v = certify(
            &g,
            &DependencyGraph::new(),
            &ChangeKind::ContentModify,
            &delta,
            64,
            1,
        );
        assert!(matches!(v, Certifiability::Partial { .. }));
    }

    // ---- GV2-026: reverse-impact hop-depth lever (ADR-063 §3) ----

    /// `clamp_reverse_impact_depth` honours the ADR-063 envelope: 0/unset folds
    /// up to the 1-hop default, an in-range request passes through, and an
    /// over-cap request is **clamped, not honoured** to `MAX_REVERSE_IMPACT_DEPTH`.
    #[test]
    fn clamp_reverse_impact_depth_respects_adr063_envelope() {
        assert_eq!(clamp_reverse_impact_depth(0), 1, "0/unset → 1-hop default");
        assert_eq!(
            clamp_reverse_impact_depth(1),
            1,
            "in-range 1 passes through"
        );
        assert_eq!(
            clamp_reverse_impact_depth(2),
            2,
            "in-range 2 passes through"
        );
        assert_eq!(
            clamp_reverse_impact_depth(2),
            MAX_REVERSE_IMPACT_DEPTH,
            "2 == the hard cap"
        );
        assert_eq!(
            clamp_reverse_impact_depth(5),
            MAX_REVERSE_IMPACT_DEPTH,
            "over-cap 5 clamps to the hard cap"
        );
        assert_eq!(
            clamp_reverse_impact_depth(u32::MAX),
            MAX_REVERSE_IMPACT_DEPTH,
            "an extreme over-cap request is clamped, never honoured"
        );
    }

    /// The resolved default lever is 1 hop (ADR-063: "Default 1 hop").
    #[test]
    fn default_reverse_impact_depth_is_one_hop() {
        assert_eq!(clamp_reverse_impact_depth(0), 1);
    }

    /// a.ts ← b.ts ← c.ts ← d.ts. At depth 1 the bounded closure reaches only
    /// the direct importer of the edited file; at depth 2 it reaches the second
    /// hop too — the runtime lever moves coverage 1 → 2 hops without re-coding.
    #[test]
    fn bounded_closure_reverse_impact_depth_lever_widens_one_to_two_hops() {
        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string()); // hop 1
        dep.add_dependency("c.ts".to_string(), "b.ts".to_string()); // hop 2
        dep.add_dependency("d.ts".to_string(), "c.ts".to_string()); // hop 3 (> cap)

        let depth1 = bounded_impact_closure(&dep, "a.ts", 1, 64).expect("within budget");
        let mut got1: Vec<&str> = depth1.iter().map(String::as_str).collect();
        got1.sort_unstable();
        assert_eq!(got1, vec!["b.ts"], "depth 1 stops at the direct importer");

        let depth2 = bounded_impact_closure(&dep, "a.ts", 2, 64).expect("within budget");
        let mut got2: Vec<&str> = depth2.iter().map(String::as_str).collect();
        got2.sort_unstable();
        assert_eq!(got2, vec!["b.ts", "c.ts"], "depth 2 reaches two hops");
    }

    /// An over-cap `max_depth` through the public `certify` entry behaves
    /// identically to `max_depth = MAX_REVERSE_IMPACT_DEPTH` — clamped, never a
    /// panic. budget = 1 with the 3-hop chain over-caps to 2 (closure `{b,c}`,
    /// size 2) → `ImpactSetOverflow`, the same as requesting exactly the cap.
    #[test]
    fn certify_over_cap_reverse_impact_depth_is_clamped_not_honoured() {
        let sym = sym_with("a.ts", &[("baz", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let mut dep = DependencyGraph::new();
        dep.add_dependency("b.ts".to_string(), "a.ts".to_string());
        dep.add_dependency("c.ts".to_string(), "b.ts".to_string());
        dep.add_dependency("d.ts".to_string(), "c.ts".to_string());

        let over_cap = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 1, 99);
        let at_cap = certify(
            &sym,
            &dep,
            &ChangeKind::ContentModify,
            &delta,
            1,
            MAX_REVERSE_IMPACT_DEPTH,
        );
        assert_eq!(over_cap, at_cap, "over-cap depth clamps to the hard cap");
        assert_eq!(
            over_cap,
            Certifiability::Partial {
                reason: CertifyStale::ImpactSetOverflow
            }
        );
    }

    /// The depth lever is verdict-neutral on the certified/partial axis: depth 1
    /// vs depth 2 only changes overflow sensitivity, never a clean verdict. A
    /// body-only edit certifies clean at either depth.
    #[test]
    fn certify_reverse_impact_depth_does_not_change_clean_verdict() {
        let sym = sym_with("a.ts", &[("foo", Visibility::Public, TrustLevel::Unknown)]);
        let delta = delta_for("a.ts", &["foo"], &[]);
        let dep = DependencyGraph::new();
        for depth in [1, 2] {
            let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 64, depth);
            assert_eq!(
                v,
                Certifiability::Certified {
                    paths: vec![PathBuf::from("a.ts")]
                },
                "body-only edit certifies clean at depth {depth}"
            );
        }
    }

    #[test]
    fn internal_only_file_with_no_public_surface_certifies() {
        // Lock the legitimate empty/internal-only case: a file whose public
        // surface is empty before and after a body-only edit certifies
        // self-only (the errors guard must not over-reject this).
        let sym = sym_with(
            "a.ts",
            &[("helper", Visibility::Internal, TrustLevel::Unknown)],
        );
        let delta = delta_for("a.ts", &[], &[]);
        assert!(!export_surface_changed(&sym, &delta));
        let v = certify(
            &sym,
            &DependencyGraph::new(),
            &ChangeKind::ContentModify,
            &delta,
            64,
            1,
        );
        assert_eq!(
            v,
            Certifiability::Certified {
                paths: vec![PathBuf::from("a.ts")]
            }
        );
    }
}
