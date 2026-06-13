use std::collections::{HashMap, HashSet};

/// Module-level dependency graph derived from import edges.
///
/// Nodes are file paths, edges are import relationships.
/// Built from the symbol-level import data extracted by the parser.
///
/// The graph keeps two invariants that let it be **maintained incrementally**
/// (GV2-011) and compared structurally against a cold rebuild:
/// - the forward (`edges`) and reverse (`reverse`) indexes are always mutual —
///   `a → b` is in `edges[a]` iff `b → a` is in `reverse[b]`;
/// - no empty edge set is ever retained, so a file with no remaining edges
///   leaves no key behind. A graph built incrementally via
///   [`set_dependencies`](Self::set_dependencies)/[`remove_file`](Self::remove_file)
///   therefore equals (`==`) one built from scratch via
///   [`add_dependency`](Self::add_dependency) for the same final edge set.
#[derive(Debug, PartialEq, Eq)]
pub struct DependencyGraph {
    /// Map from file to set of files it imports
    edges: HashMap<String, HashSet<String>>,
    /// Map from file to set of files that import it
    reverse: HashMap<String, HashSet<String>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Add a dependency: `from` imports `to`.
    pub fn add_dependency(&mut self, from: String, to: String) {
        self.edges
            .entry(from.clone())
            .or_default()
            .insert(to.clone());
        self.reverse.entry(to).or_default().insert(from);
    }

    /// Get all files that `file` directly imports.
    pub fn dependencies_of(&self, file: &str) -> Vec<&str> {
        self.edges
            .get(file)
            .map(|deps| deps.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Whether the directed dependency edge `from → to` exists (i.e. `from`
    /// imports `to`). Answered in O(1)/O(degree) from the resident forward
    /// index without allocating — the hot-path "known-edge existence" read
    /// (ADR-063 allowlist #2).
    pub fn has_edge(&self, from: &str, to: &str) -> bool {
        self.edges.get(from).is_some_and(|deps| deps.contains(to))
    }

    /// Get all files that directly import `file`.
    pub fn dependents_of(&self, file: &str) -> Vec<&str> {
        self.reverse
            .get(file)
            .map(|deps| deps.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Replace `file`'s outgoing dependencies with exactly `targets`, keeping the
    /// reverse index in step. Incoming edges (other files that import `file`) are
    /// left untouched — "who depends on me" is owned by those other files' edge
    /// sets, not by this call (the same scoping the `GraphDelta` `removed_edges`
    /// channel uses).
    ///
    /// This is the incremental-maintenance primitive that lets the save-time
    /// daemon refresh one file's dependency edges in O(file's edges) instead of
    /// the O(all edges) whole-graph re-derive (GV2-011). A self-dependency
    /// (`target == file`) is skipped, matching the cross-file-only rule of the
    /// cold rebuild, and a file left with no outgoing edges retains no empty set.
    pub fn set_dependencies<I, S>(&mut self, file: &str, targets: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        // Drop the file's current forward edges and their reverse back-pointers.
        if let Some(old) = self.edges.remove(file) {
            for dep in old {
                if let Some(rev) = self.reverse.get_mut(&dep) {
                    rev.remove(file);
                    if rev.is_empty() {
                        self.reverse.remove(&dep);
                    }
                }
            }
        }
        // De-duplicate (the caller may repeat a target) and drop self-edges in
        // one pass, so the reverse index is touched once per unique target.
        let new_edges: HashSet<String> = targets
            .into_iter()
            .map(Into::into)
            .filter(|t| t != file)
            .collect();
        if new_edges.is_empty() {
            // No outgoing edges → leave no empty forward set behind.
            return;
        }
        for target in &new_edges {
            self.reverse
                .entry(target.clone())
                .or_default()
                .insert(file.to_string());
        }
        self.edges.insert(file.to_string(), new_edges);
    }

    /// Remove all edges originating from AND pointing to `file`.
    pub fn remove_file(&mut self, file: &str) {
        // Remove outgoing edges (file → deps)
        if let Some(deps) = self.edges.remove(file) {
            for dep in deps {
                if let Some(rev) = self.reverse.get_mut(&dep) {
                    rev.remove(file);
                    if rev.is_empty() {
                        self.reverse.remove(&dep);
                    }
                }
            }
        }
        // Remove incoming edges (importers → file)
        if let Some(importers) = self.reverse.remove(file) {
            for importer in importers {
                if let Some(fwd) = self.edges.get_mut(&importer) {
                    fwd.remove(file);
                    if fwd.is_empty() {
                        self.edges.remove(&importer);
                    }
                }
            }
        }
    }

    /// Iterate the forward dependency edges as `(source, &targets)` pairs, in
    /// arbitrary order. The sole reader is GV2-030 snapshot serialisation (in this
    /// crate), which sorts deterministically before encoding; the reverse index is
    /// omitted by design (it is rebuilt from `edges` on load — ADR-069 §1).
    /// `pub(crate)`: no external consumer, so the API surface stays minimal.
    pub(crate) fn forward_edges(&self) -> impl Iterator<Item = (&str, &HashSet<String>)> {
        self.edges.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Total number of unique files in the graph.
    pub fn file_count(&self) -> usize {
        let mut files: HashSet<&str> = HashSet::new();
        for (k, v) in &self.edges {
            files.insert(k);
            for dep in v {
                files.insert(dep);
            }
        }
        files.len()
    }

    /// Total number of dependency edges.
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(HashSet::len).sum()
    }

    /// Detect circular dependencies. Returns the first cycle found, if any.
    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        let mut path = Vec::new();

        for file in self.edges.keys() {
            if !visited.contains(file.as_str())
                && let Some(cycle) = self.dfs_cycle(file, &mut visited, &mut stack, &mut path)
            {
                return Some(cycle);
            }
        }
        None
    }

    fn dfs_cycle<'a>(
        &'a self,
        node: &'a str,
        visited: &mut HashSet<&'a str>,
        stack: &mut HashSet<&'a str>,
        path: &mut Vec<&'a str>,
    ) -> Option<Vec<String>> {
        visited.insert(node);
        stack.insert(node);
        path.push(node);

        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                if !visited.contains(dep.as_str()) {
                    if let Some(cycle) = self.dfs_cycle(dep, visited, stack, path) {
                        return Some(cycle);
                    }
                } else if stack.contains(dep.as_str()) {
                    let start = path.iter().position(|&p| p == dep.as_str()).unwrap();
                    let mut cycle: Vec<String> =
                        path[start..].iter().copied().map(String::from).collect();
                    cycle.push(dep.clone());
                    return Some(cycle);
                }
            }
        }

        path.pop();
        stack.remove(node);
        None
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_query_dependencies() {
        let mut g = DependencyGraph::new();
        g.add_dependency("a.ts".to_string(), "b.ts".to_string());
        g.add_dependency("a.ts".to_string(), "c.ts".to_string());

        let deps = g.dependencies_of("a.ts");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"b.ts"));
        assert!(deps.contains(&"c.ts"));
    }

    #[test]
    fn reverse_lookup() {
        let mut g = DependencyGraph::new();
        g.add_dependency("a.ts".to_string(), "shared.ts".to_string());
        g.add_dependency("b.ts".to_string(), "shared.ts".to_string());

        let dependents = g.dependents_of("shared.ts");
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&"a.ts"));
        assert!(dependents.contains(&"b.ts"));
    }

    #[test]
    fn remove_file_clears_forward_edges() {
        let mut g = DependencyGraph::new();
        g.add_dependency("a.ts".to_string(), "b.ts".to_string());
        g.add_dependency("a.ts".to_string(), "c.ts".to_string());

        g.remove_file("a.ts");
        assert!(g.dependencies_of("a.ts").is_empty());
        assert!(g.dependents_of("b.ts").is_empty());
    }

    #[test]
    fn remove_target_clears_incoming_edges() {
        let mut g = DependencyGraph::new();
        g.add_dependency("a.ts".to_string(), "shared.ts".to_string());
        g.add_dependency("b.ts".to_string(), "shared.ts".to_string());

        // Remove the target file
        g.remove_file("shared.ts");

        // No dangling references
        assert!(g.dependents_of("shared.ts").is_empty());
        assert!(g.dependencies_of("a.ts").is_empty());
        assert!(g.dependencies_of("b.ts").is_empty());
    }

    #[test]
    fn detect_cycle() {
        let mut g = DependencyGraph::new();
        g.add_dependency("a.ts".to_string(), "b.ts".to_string());
        g.add_dependency("b.ts".to_string(), "c.ts".to_string());
        g.add_dependency("c.ts".to_string(), "a.ts".to_string());

        let cycle = g.find_cycle();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.len() >= 3);
    }

    #[test]
    fn no_cycle_in_dag() {
        let mut g = DependencyGraph::new();
        g.add_dependency("a.ts".to_string(), "b.ts".to_string());
        g.add_dependency("b.ts".to_string(), "c.ts".to_string());

        assert!(g.find_cycle().is_none());
    }

    #[test]
    fn counts() {
        let mut g = DependencyGraph::new();
        g.add_dependency("a.ts".to_string(), "b.ts".to_string());
        g.add_dependency("a.ts".to_string(), "c.ts".to_string());

        assert_eq!(g.edge_count(), 2);
        assert_eq!(g.file_count(), 3);
    }

    #[test]
    fn set_dependencies_replaces_outgoing_only() {
        let mut g = DependencyGraph::new();
        g.set_dependencies("a.ts", ["b.ts".to_string(), "c.ts".to_string()]);
        // d.ts importing a.ts is an *incoming* edge — set_dependencies on a.ts
        // must not disturb it.
        g.add_dependency("d.ts".to_string(), "a.ts".to_string());

        // Replace a.ts's outgoing set: drop c.ts, keep b.ts, add e.ts.
        g.set_dependencies("a.ts", ["b.ts".to_string(), "e.ts".to_string()]);

        let mut deps = g.dependencies_of("a.ts");
        deps.sort_unstable();
        assert_eq!(deps, vec!["b.ts", "e.ts"]);
        // Incoming edge survived; dropped target lost its back-pointer.
        assert_eq!(g.dependents_of("a.ts"), vec!["d.ts"]);
        assert!(g.dependents_of("c.ts").is_empty());
    }

    #[test]
    fn set_dependencies_to_empty_leaves_no_residue() {
        let mut g = DependencyGraph::new();
        g.set_dependencies("a.ts", ["b.ts".to_string()]);
        g.set_dependencies("a.ts", std::iter::empty::<String>());

        // Structurally identical to a never-touched graph (no empty sets left).
        assert_eq!(g, DependencyGraph::new());
    }

    #[test]
    fn set_dependencies_skips_self_edge() {
        let mut g = DependencyGraph::new();
        g.set_dependencies("a.ts", ["a.ts".to_string(), "b.ts".to_string()]);
        assert_eq!(g.dependencies_of("a.ts"), vec!["b.ts"]);
        assert!(g.dependents_of("a.ts").is_empty());
    }

    /// Deterministic linear-congruential generator — keeps the property test
    /// reproducible (Anvil determinism principle) without a `rand`/`proptest`
    /// dependency.
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

    /// Build a `DependencyGraph` from scratch (the "cold rebuild") for a model
    /// mapping each file to its outgoing-target set, using only the low-level
    /// `add_dependency` primitive.
    fn cold_rebuild(
        model: &HashMap<String, std::collections::BTreeSet<String>>,
    ) -> DependencyGraph {
        let mut g = DependencyGraph::new();
        for (from, targets) in model {
            for to in targets {
                if to != from {
                    g.add_dependency(from.clone(), to.clone());
                }
            }
        }
        g
    }

    /// GV2-011 cold-rebuild equivalence: an incrementally-maintained graph must
    /// equal a from-scratch rebuild after an *arbitrary* delta sequence. Drives
    /// `set_dependencies` (refresh a file's outgoing set) and `remove_file`
    /// (drop a file in both directions) against a ground-truth model and asserts
    /// structural equality (forward AND reverse indexes) after every step.
    #[test]
    fn index_consistency_under_arbitrary_delta_sequence() {
        use std::collections::BTreeSet;

        let files = ["a.ts", "b.ts", "c.ts", "d.ts", "e.ts"];
        let mut model: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut g = DependencyGraph::new();
        let mut rng = Lcg::new(0x00C0_FFEE_D00D);

        for step in 0..4000 {
            let file = files[rng.below(files.len())].to_string();
            if rng.below(6) == 0 {
                // Delete: the file vanishes as both a source and a target.
                model.remove(&file);
                for targets in model.values_mut() {
                    targets.remove(&file);
                }
                g.remove_file(&file);
            } else {
                // Refresh the file's outgoing set with a random cross-file set.
                let mut targets = BTreeSet::new();
                let count = rng.below(files.len());
                for _ in 0..count {
                    let t = files[rng.below(files.len())].to_string();
                    if t != file {
                        targets.insert(t);
                    }
                }
                model.insert(file.clone(), targets.clone());
                g.set_dependencies(&file, targets.iter().cloned());
            }

            assert_eq!(
                g,
                cold_rebuild(&model),
                "incremental dep graph diverged from cold rebuild at step {step}"
            );
        }
    }
}
