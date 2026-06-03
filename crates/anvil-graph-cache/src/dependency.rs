use std::collections::{HashMap, HashSet};

/// Module-level dependency graph derived from import edges.
///
/// Nodes are file paths, edges are import relationships.
/// Built from the symbol-level import data extracted by the parser.
#[derive(Debug)]
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

    /// Get all files that directly import `file`.
    pub fn dependents_of(&self, file: &str) -> Vec<&str> {
        self.reverse
            .get(file)
            .map(|deps| deps.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Remove all edges originating from AND pointing to `file`.
    pub fn remove_file(&mut self, file: &str) {
        // Remove outgoing edges (file → deps)
        if let Some(deps) = self.edges.remove(file) {
            for dep in deps {
                if let Some(rev) = self.reverse.get_mut(&dep) {
                    rev.remove(file);
                }
            }
        }
        // Remove incoming edges (importers → file)
        if let Some(importers) = self.reverse.remove(file) {
            for importer in importers {
                if let Some(fwd) = self.edges.get_mut(&importer) {
                    fwd.remove(file);
                }
            }
        }
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
}
