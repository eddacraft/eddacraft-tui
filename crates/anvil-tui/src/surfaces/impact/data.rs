//! Impact-view data layer: derive crate-level and per-crate module graphs
//! from the daemon's warm graph-cache snapshot (`ANVILGC1`, ADR-069).
//!
//! Ported from the `spike-flow` validation spike (PRs #4074/#4081). The
//! snapshot's dependency edges are `(file path, import specifier)` pairs;
//! the crate-level graph keeps only specifiers whose first segment matches a
//! workspace lib name — the graph of **used** imports, not declared Cargo
//! dependencies. `crate::a::b` specifiers resolve to files by path-prefix
//! for the per-crate internals view.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Above this many crate-level nodes the Sugiyama layout stops being usable
/// in a terminal; the surface degrades honestly instead of rendering soup.
pub const MAX_RENDERABLE_NODES: usize = 250;

/// Raw file-level import data decoded from the snapshot.
pub struct RawGraph {
    /// `(source file, import specifier)` pairs.
    pub edges: Vec<(String, String)>,
    /// Tracked workspace-relative file paths.
    pub files: BTreeSet<String>,
}

/// The impact view's model: crate-level edges plus the raw data internals
/// views are derived from on demand.
pub struct ImpactGraph {
    /// Workspace root the snapshot describes.
    pub root: PathBuf,
    /// Crate-level used-import edges, sorted and deduplicated.
    pub crate_edges: Vec<(String, String)>,
    raw: RawGraph,
}

/// Why the impact view cannot render a graph. Every variant is named on
/// screen — an empty canvas is never the answer.
#[derive(Debug, thiserror::Error)]
pub enum ImpactDataError {
    #[error("no warm graph snapshot found for {root}")]
    NoSnapshot { root: PathBuf },
    #[error("graph snapshot could not be decoded: {detail}")]
    SnapshotRejected { detail: String },
    #[error("no crate-level import edges in the snapshot (nothing under crates/?)")]
    EmptyGraph,
    #[error("{nodes} crates exceed the {MAX_RENDERABLE_NODES}-node render budget")]
    TooLarge { nodes: usize },
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl ImpactGraph {
    /// Load the warm snapshot for `root` and derive the crate-level graph.
    ///
    /// # Errors
    ///
    /// Every failure mode maps to a named [`ImpactDataError`] the surface
    /// renders as a degraded state.
    /// The node-count render budget is *not* applied here — a graph too
    /// large for the TUI layout is still perfectly good data for `--json`
    /// and `--no-tui`; the surface layer degrades on [`MAX_RENDERABLE_NODES`].
    pub fn load(root: &Path) -> Result<Self, ImpactDataError> {
        let root = dunce::canonicalize(root)?;
        let raw = load_snapshot(&root)?;
        let crate_edges = crate_level(&raw);
        if crate_edges.is_empty() {
            return Err(ImpactDataError::EmptyGraph);
        }
        Ok(Self {
            root,
            crate_edges,
            raw,
        })
    }

    /// Distinct crate count in the crate-level graph.
    #[must_use]
    pub fn crate_count(&self) -> usize {
        node_count(&self.crate_edges)
    }

    /// Fixture constructor for tests and the `--json` path: no snapshot IO.
    #[must_use]
    pub fn from_parts(root: PathBuf, crate_edges: Vec<(String, String)>, raw: RawGraph) -> Self {
        Self {
            root,
            crate_edges,
            raw,
        }
    }

    /// Edges touching `focus` (its direct neighbourhood).
    #[must_use]
    pub fn neighbourhood(&self, focus: &str) -> Vec<(String, String)> {
        self.crate_edges
            .iter()
            .filter(|(a, b)| a == focus || b == focus)
            .cloned()
            .collect()
    }

    /// File-level internal module graph of one crate.
    #[must_use]
    pub fn internals(&self, krate: &str) -> Vec<(String, String)> {
        internals_of(&self.raw, krate)
    }
}

fn node_count(edges: &[(String, String)]) -> usize {
    let mut set = BTreeSet::new();
    for (a, b) in edges {
        set.insert(a);
        set.insert(b);
    }
    set.len()
}

/// A `.root` sidecar holds one workspace path; anything bigger is not ours.
const MAX_ROOT_SIDECAR_BYTES: u64 = 4096;

/// The persistent graph-cache directory, mirroring the daemon's resolver
/// (`anvil-intercept/src/snapshot_io.rs`, ADR-060/ADR-069): `ANVIL_HOME` →
/// `<prefix>/graph-cache`; else `$XDG_STATE_HOME/anvil/graph-cache`; else
/// `$HOME`/`%USERPROFILE%` `.local/state/anvil/graph-cache`.
fn graph_cache_dir() -> Option<PathBuf> {
    let non_empty = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    if let Some(prefix) = non_empty("ANVIL_HOME") {
        return Some(PathBuf::from(prefix).join("graph-cache"));
    }
    if let Some(state) = non_empty("XDG_STATE_HOME") {
        return Some(PathBuf::from(state).join("anvil").join("graph-cache"));
    }
    non_empty("HOME")
        .or_else(|| non_empty("USERPROFILE"))
        .map(|h| {
            PathBuf::from(h)
                .join(".local")
                .join("state")
                .join("anvil")
                .join("graph-cache")
        })
}

/// Read a `.root` sidecar, refusing anything over the size bound.
fn read_root_sidecar(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_ROOT_SIDECAR_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// Locate and decode the warm graph-cache snapshot for `root`.
fn load_snapshot(root: &Path) -> Result<RawGraph, ImpactDataError> {
    use anvil_graph_cache::snapshot::SnapshotPayload;

    let Some(state) = graph_cache_dir() else {
        return Err(ImpactDataError::NoSnapshot {
            root: root.to_path_buf(),
        });
    };

    let mut snap_path = None;
    if let Ok(entries) = std::fs::read_dir(&state) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "root")
                && read_root_sidecar(&p).is_some_and(|c| Path::new(c.trim()) == root)
            {
                snap_path = Some(p.with_extension("snap"));
                break;
            }
        }
    }
    let Some(snap_path) = snap_path else {
        return Err(ImpactDataError::NoSnapshot {
            root: root.to_path_buf(),
        });
    };

    let bytes = std::fs::read(&snap_path)?;
    let payload =
        SnapshotPayload::from_bytes(&bytes).map_err(|e| ImpactDataError::SnapshotRejected {
            detail: format!("{e:?}"),
        })?;
    let files: BTreeSet<String> = payload
        .tracked_files()
        .iter()
        .map(ToString::to_string)
        .collect();
    let (_symbols, deps) =
        payload
            .into_graphs()
            .map_err(|e| ImpactDataError::SnapshotRejected {
                detail: format!("{e:?}"),
            })?;

    let mut edges = Vec::new();
    for file in &files {
        for target in deps.dependencies_of(file) {
            edges.push((file.clone(), target.to_string()));
        }
    }
    Ok(RawGraph { edges, files })
}

/// The crate a workspace-relative file belongs to, if any.
fn crate_of(path: &str) -> Option<&str> {
    path.strip_prefix("crates/")?.split('/').next()
}

/// Crate-level graph from raw import edges: the first segment of each `use`
/// path is matched against workspace lib names (`anvil_checks` →
/// `anvil-checks`); `std`/`crate`/`super`/external crates drop out.
fn crate_level(raw: &RawGraph) -> Vec<(String, String)> {
    let lib_to_crate: BTreeMap<String, String> = raw
        .files
        .iter()
        .filter_map(|f| crate_of(f))
        .map(|c| (c.replace('-', "_"), c.to_string()))
        // eddacraft-anvil-* packages export anvil_* lib names
        .flat_map(|(lib, c)| {
            let stripped = lib.trim_start_matches("eddacraft_").to_string();
            [(lib, c.clone()), (stripped, c)]
        })
        .collect();

    let mut edges = BTreeSet::new();
    for (file, spec) in &raw.edges {
        let Some(from) = crate_of(file) else { continue };
        let Some(seg) = spec.split("::").next() else {
            continue;
        };
        if let Some(to) = lib_to_crate.get(seg)
            && to != from
        {
            edges.insert((from.to_string(), to.clone()));
        }
    }
    edges.into_iter().collect()
}

/// File-level internal graph of one crate: `crate::a::b` specifiers resolve
/// to files by deepest existing path-prefix (`src/a/b.rs`, `src/a/b/mod.rs`).
fn internals_of(raw: &RawGraph, krate: &str) -> Vec<(String, String)> {
    let prefix = format!("crates/{krate}/");
    let label = |f: &str| f.strip_prefix(&prefix).unwrap_or(f).to_string();

    let resolve = |module_path: &[&str]| -> Option<String> {
        for depth in (1..=module_path.len()).rev() {
            let joined = module_path[..depth].join("/");
            for cand in [
                format!("{prefix}src/{joined}.rs"),
                format!("{prefix}src/{joined}/mod.rs"),
            ] {
                if raw.files.contains(&cand) {
                    return Some(label(&cand));
                }
            }
        }
        None
    };

    let mut edges = BTreeSet::new();
    for (file, spec) in &raw.edges {
        if !file.starts_with(&prefix) {
            continue;
        }
        let segs: Vec<&str> = spec.split("::").collect();
        if segs.first() != Some(&"crate") || segs.len() < 2 {
            continue;
        }
        if let Some(to) = resolve(&segs[1..]) {
            let from = label(file);
            if from != to {
                edges.insert((from, to));
            }
        }
    }
    edges.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_fixture() -> RawGraph {
        let files: BTreeSet<String> = [
            "crates/alpha/src/lib.rs",
            "crates/alpha/src/inner.rs",
            "crates/beta/src/lib.rs",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect();
        let edges = vec![
            (
                "crates/alpha/src/lib.rs".to_string(),
                "beta::Thing".to_string(),
            ),
            (
                "crates/alpha/src/lib.rs".to_string(),
                "crate::inner::helper".to_string(),
            ),
            ("crates/alpha/src/lib.rs".to_string(), "std::io".to_string()),
        ];
        RawGraph { edges, files }
    }

    #[test]
    fn crate_level_keeps_workspace_imports_only() {
        let edges = crate_level(&raw_fixture());
        assert_eq!(edges, vec![("alpha".to_string(), "beta".to_string())]);
    }

    #[test]
    fn internals_resolve_crate_specifiers_to_files() {
        let edges = internals_of(&raw_fixture(), "alpha");
        assert_eq!(
            edges,
            vec![("src/lib.rs".to_string(), "src/inner.rs".to_string())]
        );
    }
}
