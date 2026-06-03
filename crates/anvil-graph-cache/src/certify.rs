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
//! Importer discovery reads [`DependencyGraph::dependents_of`] **exclusively**.
//! The `GraphDelta::removed_edges` channel is always empty (`incremental.rs`
//! never populates it), so certify must never branch on it. The daemon caches
//! the `(SymbolGraph, DependencyGraph)` pair (DSV-004 Task 7) precisely so this
//! reverse index is reachable on the hot path.
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

use anvil_kernel_types::{TrustLevel, Visibility};

use crate::dependency::DependencyGraph;
use crate::incremental::GraphDelta;
use crate::symbol_graph::SymbolGraph;

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
/// Graph-cache-local: the daemon maps these to the wire `StaleReason`
/// (DSV-005). Names mirror the wire variants they map to so the mapping is
/// mechanical.
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
    CrossFileResolution,
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

/// Does this update change the file's public or privileged symbol surface?
///
/// Compares the pre-update public/privileged baselines carried on the
/// [`GraphDelta`] against the post-update graph state for the same file. Any
/// asymmetry — added, removed, or renamed public/privileged symbol — is a
/// surface change. Body-only edits leave both sets identical.
///
/// Reads only `delta.previously_public` / `delta.previously_privileged` and the
/// post-update graph; never touches `delta.removed_edges` (always empty).
#[must_use]
pub fn export_surface_changed(sym: &SymbolGraph, delta: &GraphDelta) -> bool {
    let current = sym.symbols_in_file(&delta.file);

    let current_public: HashSet<String> = current
        .iter()
        .filter(|s| s.visibility == Visibility::Public)
        .map(|s| GraphDelta::symbol_baseline_key(s))
        .collect();
    if current_public != delta.previously_public {
        return true;
    }

    let current_privileged: HashSet<String> = current
        .iter()
        .filter(|s| s.trust_level == TrustLevel::Privileged)
        .map(|s| GraphDelta::symbol_baseline_key(s))
        .collect();
    current_privileged != delta.previously_privileged
}

/// Collect every file transitively impacted by a surface change to `file`,
/// reading reverse edges from the [`DependencyGraph`].
///
/// Walks `dependents_of` outward (direct importers, then their importers — the
/// re-export chain), deduplicating. Returns `None` the moment the distinct
/// impacted set would exceed `budget`, the overflow signal that maps to
/// [`CertifyStale::ImpactSetOverflow`]. `file` itself is not counted.
fn impact_closure(dep: &DependencyGraph, file: &str, budget: usize) -> Option<HashSet<String>> {
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
/// - `ContentModify` with no public/privileged surface change certifies
///   self-only (`Certified{[file]}`).
/// - `ContentModify` with a surface change is `Partial`: `ImpactSetOverflow`
///   if the importer closure exceeds `budget`, otherwise `ExportSurfaceChange`.
#[must_use]
pub fn certify(
    sym: &SymbolGraph,
    dep: &DependencyGraph,
    change: &ChangeKind,
    delta: &GraphDelta,
    budget: usize,
) -> Certifiability {
    match change {
        ChangeKind::Delete => Certifiability::Partial {
            reason: CertifyStale::Deleted,
        },
        ChangeKind::Rename => Certifiability::Partial {
            reason: CertifyStale::Renamed,
        },
        ChangeKind::Create => Certifiability::Partial {
            reason: CertifyStale::CrossFileResolution,
        },
        ChangeKind::ContentModify => {
            if !export_surface_changed(sym, delta) {
                return Certifiability::Certified {
                    paths: vec![PathBuf::from(&delta.file)],
                };
            }
            match impact_closure(dep, &delta.file, budget) {
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
    use anvil_kernel_types::{SymbolKind, SymbolNode};

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

    fn key(file: &str, name: &str) -> String {
        // Mirrors GraphDelta::symbol_baseline_key for SymbolKind::Function.
        format!("{file}::{:?}::{name}", SymbolKind::Function)
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
        );
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::CrossFileResolution
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

        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 64);
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

        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 64);
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

        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 64);
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

        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 1);
        assert_eq!(
            v,
            Certifiability::Partial {
                reason: CertifyStale::ImpactSetOverflow
            }
        );
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
        let v = certify(&sym, &dep, &ChangeKind::ContentModify, &delta, 0);
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
        );
        assert_eq!(
            v_empty,
            Certifiability::Partial {
                reason: CertifyStale::ExportSurfaceChange
            },
            "with no reverse edges the closure is empty and does not overflow"
        );
    }
}
