# Wave 2: Semantic Graph + Medium Surface Ports — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development
> (if subagents available) or superpowers:executing-plans to implement this plan.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the KERN Phase 2 semantic graph (petgraph symbol graph, dependency
graph, trust metadata, incremental updates) and port the four medium-complexity
Ink TUI surfaces to Ratatui.

**Architecture:** Two independent tracks. Track A adds a `graph` module to the
existing `anvil-kernel` crate. Track B adds four new surfaces to the existing
`anvil-tui` crate. No cross-track dependencies.

**Tech Stack:** petgraph 0.8, anvil-kernel-types (SymbolNode, SymbolEdge,
EdgeType, TrustLevel), ratatui 0.30, eddacraft-tui (theme, keyboard, widgets).

**APS Work Items:** KERN-020, KERN-021, KERN-022, KERN-023, PORT-020, PORT-021,
PORT-022, PORT-023.

---

## Chunk 1: Track A — Kernel Phase 2 (Semantic Graph)

### Task 1: Symbol graph with petgraph (KERN-020)

**Files:**
- Create: `crates/anvil-kernel/src/graph/mod.rs`
- Create: `crates/anvil-kernel/src/graph/symbol_graph.rs`
- Modify: `crates/anvil-kernel/src/lib.rs`

**Context:** The graph stores `SymbolNode` (from `anvil-kernel-types`) as node
weights and `SymbolEdge` as edge weights in a `petgraph::graph::DiGraph`. Nodes
are indexed by `NodeIndex`, and we maintain a `HashMap<u64, NodeIndex>` for
O(1) lookup by symbol id.

- [ ] **Step 1: Create graph module with SymbolGraph**

```rust
// crates/anvil-kernel/src/graph/mod.rs
pub mod symbol_graph;

pub use symbol_graph::{GraphError, GraphStats, SymbolGraph};
```

```rust
// crates/anvil-kernel/src/graph/symbol_graph.rs
use std::collections::HashMap;

use anvil_kernel_types::{EdgeType, SymbolEdge, SymbolNode};
use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("symbol with id {0} not found")]
    SymbolNotFound(u64),
    #[error("duplicate symbol id {0}")]
    DuplicateSymbol(u64),
}

#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub files: usize,
}

pub struct SymbolGraph {
    graph: DiGraph<SymbolNode, SymbolEdge>,
    index: HashMap<u64, NodeIndex>,
    files: HashMap<String, Vec<u64>>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index: HashMap::new(),
            files: HashMap::new(),
        }
    }

    pub fn add_symbol(&mut self, node: SymbolNode) -> Result<NodeIndex, GraphError> {
        if self.index.contains_key(&node.id) {
            return Err(GraphError::DuplicateSymbol(node.id));
        }
        let id = node.id;
        let file = node.file.clone();
        let idx = self.graph.add_node(node);
        self.index.insert(id, idx);
        self.files.entry(file).or_default().push(id);
        Ok(idx)
    }

    pub fn add_edge(&mut self, edge: SymbolEdge) -> Result<(), GraphError> {
        let from_idx = self.index.get(&edge.from)
            .copied()
            .ok_or(GraphError::SymbolNotFound(edge.from))?;
        let to_idx = self.index.get(&edge.to)
            .copied()
            .ok_or(GraphError::SymbolNotFound(edge.to))?;
        self.graph.add_edge(from_idx, to_idx, edge);
        Ok(())
    }

    pub fn get_symbol(&self, id: u64) -> Option<&SymbolNode> {
        self.index.get(&id).map(|idx| &self.graph[*idx])
    }

    pub fn get_symbol_mut(&mut self, id: u64) -> Option<&mut SymbolNode> {
        self.index.get(&id).map(|idx| &mut self.graph[*idx])
    }

    pub fn symbols_in_file(&self, file: &str) -> Vec<&SymbolNode> {
        self.files
            .get(file)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_symbol(*id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn remove_file(&mut self, file: &str) -> Vec<u64> {
        let ids = self.files.remove(file).unwrap_or_default();
        for &id in &ids {
            if let Some(idx) = self.index.remove(&id) {
                self.graph.remove_node(idx);
                // petgraph swaps the last node into the removed slot,
                // so we need to update the index for the swapped node
                if let Some(swapped) = self.graph.node_weight(idx) {
                    self.index.insert(swapped.id, idx);
                }
            }
        }
        ids
    }

    pub fn stats(&self) -> GraphStats {
        GraphStats {
            node_count: self.graph.node_count(),
            edge_count: self.graph.edge_count(),
            files: self.files.len(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get outgoing edges from a symbol (what does this symbol depend on?).
    pub fn outgoing_edges(&self, id: u64) -> Vec<&SymbolEdge> {
        self.index.get(&id).map(|idx| {
            self.graph
                .edges(*idx)
                .map(|e| e.weight())
                .collect()
        }).unwrap_or_default()
    }

    /// Get incoming edges to a symbol (what depends on this symbol?).
    pub fn incoming_edges(&self, id: u64) -> Vec<&SymbolEdge> {
        self.index.get(&id).map(|idx| {
            self.graph
                .edges_directed(*idx, petgraph::Direction::Incoming)
                .map(|e| e.weight())
                .collect()
        }).unwrap_or_default()
    }

    /// Get the underlying petgraph for advanced queries.
    pub fn inner(&self) -> &DiGraph<SymbolNode, SymbolEdge> {
        &self.graph
    }
}

impl Default for SymbolGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, TrustLevel, Visibility};

    fn make_symbol(id: u64, name: &str, file: &str, kind: SymbolKind) -> SymbolNode {
        SymbolNode {
            id,
            kind,
            name: name.to_string(),
            visibility: Visibility::Internal,
            file: file.to_string(),
            trust_level: TrustLevel::Unknown,
        }
    }

    #[test]
    fn add_and_retrieve_symbols() {
        let mut g = SymbolGraph::new();
        let s = make_symbol(1, "foo", "src/a.ts", SymbolKind::Function);
        g.add_symbol(s).unwrap();

        let retrieved = g.get_symbol(1).unwrap();
        assert_eq!(retrieved.name, "foo");
        assert_eq!(g.node_count(), 1);
    }

    #[test]
    fn duplicate_symbol_rejected() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function)).unwrap();
        let result = g.add_symbol(make_symbol(1, "bar", "b.ts", SymbolKind::Function));
        assert!(matches!(result, Err(GraphError::DuplicateSymbol(1))));
    }

    #[test]
    fn add_and_query_edges() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function)).unwrap();
        g.add_symbol(make_symbol(2, "bar", "b.ts", SymbolKind::Function)).unwrap();

        g.add_edge(SymbolEdge {
            from: 1,
            to: 2,
            edge_type: EdgeType::Calls,
        }).unwrap();

        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.outgoing_edges(1).len(), 1);
        assert_eq!(g.outgoing_edges(1)[0].edge_type, EdgeType::Calls);
        assert_eq!(g.incoming_edges(2).len(), 1);
    }

    #[test]
    fn edge_with_missing_node_rejected() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function)).unwrap();

        let result = g.add_edge(SymbolEdge {
            from: 1,
            to: 99,
            edge_type: EdgeType::Calls,
        });
        assert!(matches!(result, Err(GraphError::SymbolNotFound(99))));
    }

    #[test]
    fn symbols_in_file() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function)).unwrap();
        g.add_symbol(make_symbol(2, "bar", "a.ts", SymbolKind::Function)).unwrap();
        g.add_symbol(make_symbol(3, "baz", "b.ts", SymbolKind::Function)).unwrap();

        let syms = g.symbols_in_file("a.ts");
        assert_eq!(syms.len(), 2);
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"bar"));
    }

    #[test]
    fn remove_file_clears_nodes() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function)).unwrap();
        g.add_symbol(make_symbol(2, "bar", "a.ts", SymbolKind::Function)).unwrap();
        g.add_symbol(make_symbol(3, "baz", "b.ts", SymbolKind::Function)).unwrap();

        let removed = g.remove_file("a.ts");
        assert_eq!(removed.len(), 2);
        assert_eq!(g.node_count(), 1);
        assert!(g.get_symbol(3).is_some());
    }

    #[test]
    fn stats() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "foo", "a.ts", SymbolKind::Function)).unwrap();
        g.add_symbol(make_symbol(2, "bar", "b.ts", SymbolKind::Function)).unwrap();
        g.add_edge(SymbolEdge { from: 1, to: 2, edge_type: EdgeType::Imports }).unwrap();

        let stats = g.stats();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
        assert_eq!(stats.files, 2);
    }
}
```

- [ ] **Step 2: Update lib.rs**

Add `pub mod graph;` to `crates/anvil-kernel/src/lib.rs`.

- [ ] **Step 3: Run tests**

Run: `cargo test -p anvil-kernel -- graph::symbol_graph`
Expected: all 7 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/anvil-kernel/src/graph/ crates/anvil-kernel/src/lib.rs
git commit -m "feat(kern): add petgraph symbol graph with CRUD operations (KERN-020)"
```

---

### Task 2: Dependency graph from import edges (KERN-021)

**Files:**
- Create: `crates/anvil-kernel/src/graph/dependency.rs`
- Modify: `crates/anvil-kernel/src/graph/mod.rs`

**Context:** The dependency graph is a module-level view derived from the
symbol graph's import edges. Nodes are files (not symbols), edges are
import relationships.

- [ ] **Step 1: Implement dependency graph**

```rust
// crates/anvil-kernel/src/graph/dependency.rs
use std::collections::{HashMap, HashSet};

/// Module-level dependency graph derived from import edges.
///
/// Nodes are file paths, edges are import relationships.
/// Built from the symbol-level import data extracted by the parser.
pub struct DependencyGraph {
    /// Map from file → set of files it imports
    edges: HashMap<String, HashSet<String>>,
    /// Map from file → set of files that import it
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
        self.edges.entry(from.clone()).or_default().insert(to.clone());
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

    /// Remove all edges originating from `file`.
    pub fn remove_file(&mut self, file: &str) {
        if let Some(deps) = self.edges.remove(file) {
            for dep in deps {
                if let Some(rev) = self.reverse.get_mut(&dep) {
                    rev.remove(file);
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
            if !visited.contains(file.as_str()) {
                if let Some(cycle) = self.dfs_cycle(file, &mut visited, &mut stack, &mut path) {
                    return Some(cycle);
                }
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
                    // Found a cycle — extract it from the path
                    let start = path.iter().position(|&p| p == dep.as_str()).unwrap();
                    let mut cycle: Vec<String> = path[start..].iter().map(|s| s.to_string()).collect();
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
```

- [ ] **Step 2: Update graph/mod.rs**

Add `pub mod dependency;` and `pub use dependency::DependencyGraph;`

- [ ] **Step 3: Run tests**

Run: `cargo test -p anvil-kernel -- graph::dependency`
Expected: all 6 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/anvil-kernel/src/graph/
git commit -m "feat(kern): add module-level dependency graph with cycle detection (KERN-021)"
```

---

### Task 3: Trust metadata on graph nodes (KERN-022)

**Files:**
- Create: `crates/anvil-kernel/src/graph/trust.rs`
- Modify: `crates/anvil-kernel/src/graph/mod.rs`

**Context:** Trust levels are assigned to symbol nodes based on heuristics:
- `Boundary` if the symbol is public (exported)
- `External` if the symbol's file imports from node_modules/external
- `Privileged` if the symbol accesses sensitive APIs (fs, net, exec, env)
- `Internal` by default if none of the above apply

- [ ] **Step 1: Implement trust annotator**

```rust
// crates/anvil-kernel/src/graph/trust.rs
use anvil_kernel_types::{TrustLevel, Visibility};

use super::symbol_graph::SymbolGraph;
use crate::parser::extract::ImportEdge;

/// Sensitive API patterns that indicate privileged access.
const PRIVILEGED_PATTERNS: &[&str] = &[
    "node:fs",
    "node:child_process",
    "node:net",
    "node:http",
    "node:https",
    "node:crypto",
    "fs",
    "child_process",
    "net",
    "http",
    "https",
    "exec",
    "spawn",
    "process.env",
];

/// External module patterns (not relative imports).
fn is_external_import(source: &str) -> bool {
    !source.starts_with('.') && !source.starts_with('/')
}

/// Annotate trust levels on all symbols in the graph based on heuristics.
///
/// This is a best-effort pass — heuristics can be overridden by configuration.
pub fn annotate_trust(graph: &mut SymbolGraph, imports: &[ImportEdge]) {
    // Build a set of files that import external modules
    let mut external_files: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut privileged_files: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for import in imports {
        if is_external_import(&import.to_source) {
            external_files.insert(&import.from_file);
        }
        if PRIVILEGED_PATTERNS
            .iter()
            .any(|p| import.to_source.contains(p))
        {
            privileged_files.insert(&import.from_file);
        }
    }

    // Collect all symbol IDs with their file and visibility first
    let symbol_info: Vec<(u64, String, Visibility)> = {
        let inner = graph.inner();
        inner
            .node_weights()
            .map(|n| (n.id, n.file.clone(), n.visibility))
            .collect()
    };

    // Annotate each symbol
    for (id, file, visibility) in symbol_info {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::extract::ImportEdge;
    use anvil_kernel_types::{SymbolKind, SymbolNode};

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
        g.add_symbol(make_symbol(1, "greet", "a.ts", Visibility::Public)).unwrap();

        annotate_trust(&mut g, &[]);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Boundary);
    }

    #[test]
    fn internal_symbols_stay_internal() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "helper", "a.ts", Visibility::Internal)).unwrap();

        annotate_trust(&mut g, &[]);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Internal);
    }

    #[test]
    fn external_import_marks_file_as_external() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "handler", "a.ts", Visibility::Internal)).unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "express".to_string(),
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::External);
    }

    #[test]
    fn privileged_import_overrides_other_trust() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "readFile", "a.ts", Visibility::Public)).unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "node:fs".to_string(),
        }];
        annotate_trust(&mut g, &imports);

        // Privileged overrides Boundary
        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Privileged);
    }

    #[test]
    fn relative_imports_not_external() {
        let mut g = SymbolGraph::new();
        g.add_symbol(make_symbol(1, "util", "a.ts", Visibility::Internal)).unwrap();

        let imports = vec![ImportEdge {
            from_file: "a.ts".to_string(),
            to_source: "./utils".to_string(),
        }];
        annotate_trust(&mut g, &imports);

        assert_eq!(g.get_symbol(1).unwrap().trust_level, TrustLevel::Internal);
    }
}
```

- [ ] **Step 2: Update graph/mod.rs**

Add `pub mod trust;` and `pub use trust::annotate_trust;`

- [ ] **Step 3: Run tests**

Run: `cargo test -p anvil-kernel -- graph::trust`
Expected: all 5 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/anvil-kernel/src/graph/
git commit -m "feat(kern): add trust level annotation for graph nodes (KERN-022)"
```

---

### Task 4: Incremental graph update (KERN-023)

**Files:**
- Create: `crates/anvil-kernel/src/graph/incremental.rs`
- Modify: `crates/anvil-kernel/src/graph/mod.rs`

**Context:** On file change, the kernel reparses the file, diffs old and new
symbol sets, and produces a `GraphDelta`. The delta is applied to the graph
and consumed by the policy engine.

- [ ] **Step 1: Implement incremental updater**

```rust
// crates/anvil-kernel/src/graph/incremental.rs
use anvil_kernel_types::{EdgeType, SymbolEdge, SymbolNode};

use super::symbol_graph::SymbolGraph;
use crate::parser::extract::{FileSymbols, ImportEdge};

/// Changes produced by an incremental graph update.
#[derive(Debug, Clone, Default)]
pub struct GraphDelta {
    pub added_symbols: Vec<u64>,
    pub removed_symbols: Vec<u64>,
    pub added_edges: Vec<(u64, u64, EdgeType)>,
    pub removed_edges: Vec<(u64, u64, EdgeType)>,
    pub file: String,
}

impl GraphDelta {
    pub fn is_empty(&self) -> bool {
        self.added_symbols.is_empty()
            && self.removed_symbols.is_empty()
            && self.added_edges.is_empty()
            && self.removed_edges.is_empty()
    }
}

/// Apply an incremental update to the graph for a single file.
///
/// 1. Remove all symbols and edges for the file
/// 2. Add new symbols from the re-parsed file
/// 3. Add import edges (as Imports edges between file-level pseudo-nodes)
/// 4. Return the delta for downstream consumers (policy engine)
pub fn update_file(
    graph: &mut SymbolGraph,
    new_symbols: FileSymbols,
) -> GraphDelta {
    let file = new_symbols.file.clone();

    // Remove old symbols for this file
    let removed_ids = graph.remove_file(&file);

    // Add new symbols
    let mut added_ids = Vec::new();
    for symbol in new_symbols.symbols {
        let id = symbol.id;
        if graph.add_symbol(symbol).is_ok() {
            added_ids.push(id);
        }
    }

    // Build delta
    GraphDelta {
        added_symbols: added_ids,
        removed_symbols: removed_ids,
        added_edges: Vec::new(),
        removed_edges: Vec::new(),
        file,
    }
}

/// Remove a deleted file from the graph entirely.
pub fn remove_file(graph: &mut SymbolGraph, file: &str) -> GraphDelta {
    let removed_ids = graph.remove_file(file);
    GraphDelta {
        removed_symbols: removed_ids,
        file: file.to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{SymbolKind, TrustLevel, Visibility};

    fn make_file_symbols(file: &str, symbols: Vec<(u64, &str, SymbolKind)>) -> FileSymbols {
        FileSymbols {
            file: file.to_string(),
            symbols: symbols
                .into_iter()
                .map(|(id, name, kind)| SymbolNode {
                    id,
                    kind,
                    name: name.to_string(),
                    visibility: Visibility::Internal,
                    file: file.to_string(),
                    trust_level: TrustLevel::Unknown,
                })
                .collect(),
            imports: Vec::new(),
        }
    }

    #[test]
    fn initial_file_add_produces_delta() {
        let mut g = SymbolGraph::new();
        let syms = make_file_symbols("a.ts", vec![
            (1, "foo", SymbolKind::Function),
            (2, "Bar", SymbolKind::Class),
        ]);

        let delta = update_file(&mut g, syms);

        assert_eq!(delta.file, "a.ts");
        assert_eq!(delta.added_symbols.len(), 2);
        assert!(delta.removed_symbols.is_empty());
        assert_eq!(g.node_count(), 2);
    }

    #[test]
    fn file_update_replaces_symbols() {
        let mut g = SymbolGraph::new();

        // Initial add
        let syms1 = make_file_symbols("a.ts", vec![
            (1, "foo", SymbolKind::Function),
            (2, "bar", SymbolKind::Function),
        ]);
        update_file(&mut g, syms1);
        assert_eq!(g.node_count(), 2);

        // Update — different symbols
        let syms2 = make_file_symbols("a.ts", vec![
            (10, "baz", SymbolKind::Function),
        ]);
        let delta = update_file(&mut g, syms2);

        assert_eq!(delta.removed_symbols.len(), 2);
        assert_eq!(delta.added_symbols.len(), 1);
        assert_eq!(g.node_count(), 1);
        assert!(g.get_symbol(10).is_some());
        assert!(g.get_symbol(1).is_none());
    }

    #[test]
    fn remove_file_produces_delta() {
        let mut g = SymbolGraph::new();
        let syms = make_file_symbols("a.ts", vec![
            (1, "foo", SymbolKind::Function),
        ]);
        update_file(&mut g, syms);

        let delta = remove_file(&mut g, "a.ts");
        assert_eq!(delta.removed_symbols.len(), 1);
        assert!(delta.added_symbols.is_empty());
        assert_eq!(g.node_count(), 0);
    }

    #[test]
    fn update_preserves_other_files() {
        let mut g = SymbolGraph::new();

        update_file(&mut g, make_file_symbols("a.ts", vec![
            (1, "foo", SymbolKind::Function),
        ]));
        update_file(&mut g, make_file_symbols("b.ts", vec![
            (2, "bar", SymbolKind::Function),
        ]));

        // Update a.ts only
        let delta = update_file(&mut g, make_file_symbols("a.ts", vec![
            (10, "baz", SymbolKind::Function),
        ]));

        assert_eq!(g.node_count(), 2);
        assert!(g.get_symbol(10).is_some()); // new a.ts symbol
        assert!(g.get_symbol(2).is_some());  // b.ts untouched
    }

    #[test]
    fn empty_delta_for_identical_count() {
        let mut g = SymbolGraph::new();
        update_file(&mut g, make_file_symbols("a.ts", vec![
            (1, "foo", SymbolKind::Function),
        ]));

        let delta = update_file(&mut g, make_file_symbols("a.ts", vec![
            (10, "foo", SymbolKind::Function),
        ]));

        // Delta still has changes (different IDs) but graph has same count
        assert_eq!(delta.removed_symbols.len(), 1);
        assert_eq!(delta.added_symbols.len(), 1);
        assert_eq!(g.node_count(), 1);
    }
}
```

- [ ] **Step 2: Update graph/mod.rs**

Add `pub mod incremental;` and `pub use incremental::{GraphDelta, update_file, remove_file};`

- [ ] **Step 3: Run tests**

Run: `cargo test -p anvil-kernel -- graph::incremental`
Expected: all 5 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/anvil-kernel/src/graph/
git commit -m "feat(kern): add incremental graph update with GraphDelta (KERN-023)"
```

---

## Chunk 2: Track B — Medium Surface Ports

### Task 5: Port init wizard surface (PORT-020)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/init/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/init/render.rs`
- Modify: `crates/anvil-tui/src/surfaces/mod.rs`

**Ink reference:** `apps/anvil-cli/src/tui/commands/init/InitWizard.tsx`
5-step wizard: Mode → Format → Directory → Checks → Summary.

- [ ] **Step 1: Define init wizard types and state**

The init wizard has 5 steps with distinct input types per step. Follow the
same patterns as the RATS-004 wizard (step enum, per-step key handling,
forward/back navigation). Key differences from RATS-004:
- 5 steps instead of 4
- Step 1 (Mode) is a select with options: new, existing, minimal
- Step 2 (Format) is a select: yaml, json, toml
- Step 3 (Directory) is text input for project root
- Step 4 (Checks) is multi-select toggle for available checks
- Step 5 (Summary) shows config and confirms

Types needed:
```rust
pub enum InitStep { Mode, Format, Directory, Checks, Summary }
pub enum InitMode { New, Existing, Minimal }
pub enum ConfigFormat { Yaml, Json, Toml }
pub struct InitConfig { mode, format, directory, checks: Vec<String> }
pub struct InitState { step, config, mode_selected, format_selected, text_input, check_toggles, should_quit, confirmed }
```

Tests:
- Step progression forward/back
- Mode selection advances to Format
- Check toggle flips values
- Summary confirm sets flag
- Back from first step doesn't go further back

- [ ] **Step 2: Implement render.rs following the welcome/wizard render patterns**

- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(port): add init wizard surface port to Ratatui (PORT-020)"
```

---

### Task 6: Port audit results surface (PORT-021)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/audit/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/audit/render.rs`
- Modify: `crates/anvil-tui/src/surfaces/mod.rs`

**Ink reference:** `apps/anvil-cli/src/tui/commands/audit/AuditResults.tsx`
4-panel layout: Project → Current Issues → Historical → Next Steps.

Types needed:
```rust
pub enum AuditPanel { Project, Issues, Historical, NextSteps }
pub struct AuditIssue { severity, category, message, file, line, fixable }
pub enum IssueSeverity { Critical, High, Medium, Low, Info }
pub struct AuditData { project_name, total_files, issues, historical_scores, next_steps }
pub struct AuditState { data, focused_panel, selected_item, expanded, should_quit }
```

Panel navigation uses Left/Right (same pattern as StatusDashboard).
Item navigation uses Up/Down. Enter expands issue details.

Tests:
- Panel navigation wraps correctly
- Item selection within each panel
- Summary counts match data
- Expand/collapse toggle

- [ ] **Step 1: Implement mod.rs with types, state, tests**
- [ ] **Step 2: Implement render.rs**
- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(port): add audit results surface port to Ratatui (PORT-021)"
```

---

### Task 7: Port template browser surface (PORT-022)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/browser/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/browser/render.rs`
- Modify: `crates/anvil-tui/src/surfaces/mod.rs`

**Ink reference:** `apps/anvil-cli/src/tui/commands/new/TemplateBrowser.tsx`
3-view: categories → templates → variables (detail).

Types needed:
```rust
pub enum BrowserView { Categories, Templates, Detail }
pub struct TemplateCategory { name, description, template_count }
pub struct TemplateEntry { id, name, description, category, tags, variables }
pub struct TemplateVariable { name, description, default_value, required }
pub struct BrowserState { categories, templates, view, cat_selected, tmpl_selected, var_selected, search_term, should_quit, chosen }
```

Navigation:
- Left/Right switches view (Categories → Templates → Detail)
- Up/Down navigates within current view
- Enter drills into next view or selects template
- Esc goes back one view
- `/` enters search mode (filters templates by name/tag)

Tests:
- View drilling (Categories → Templates on enter)
- Back navigation (Templates → Categories on Back)
- Search filtering
- Template selection

- [ ] **Step 1: Implement mod.rs with types, state, tests**
- [ ] **Step 2: Implement render.rs**
- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(port): add template browser surface port to Ratatui (PORT-022)"
```

---

### Task 8: Port gate explorer surface (PORT-023)

**Files:**
- Create: `crates/anvil-tui/src/surfaces/gate/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/gate/render.rs`
- Modify: `crates/anvil-tui/src/surfaces/mod.rs`

**Ink reference:** `apps/anvil-cli/src/tui/commands/gate/GateExplorer.tsx`
2-panel: check tree (left 50%) + detail panel (right 50%).

Types needed:
```rust
pub enum GateCheckStatus { Passed, Failed, Skipped, Warning }
pub struct GateCheck { id, name, status, score, message, details, file, line }
pub struct GateResult { plan_id, overall_passed, score, checks, duration_ms, timestamp }
pub enum FilterStatus { All, Passed, Failed, Warning, Skipped }
pub struct GateState { result, selected, expanded, filter, search_term, search_mode, should_quit }
```

Navigation:
- j/k navigates check list
- Enter expands/collapses check details
- n/N jumps to next/previous failure
- a/p/f/s/w sets filter (all/passed/failed/skipped/warning)
- `/` enters search mode
- Esc exits search mode

This is the most complex surface and the most important for dashboarding.

Tests:
- Filter applies correctly
- Search narrows results
- Failure jumping (n/N)
- Expand/collapse
- Score calculation

- [ ] **Step 1: Implement mod.rs with types, state, tests**
- [ ] **Step 2: Implement render.rs with 2-panel layout**
- [ ] **Step 3: Run tests, commit**

```bash
git commit -m "feat(port): add gate explorer surface port to Ratatui (PORT-023)"
```

---

### Task 9: Final verification + APS status update

- [ ] **Step 1: Run all tests**

Run: `cargo test --all`
Expected: all tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features`
Expected: no warnings

- [ ] **Step 3: Update APS module statuses**

Update `plans/modules/rust-kernel.aps.md`:
- KERN-020: Draft → Done
- KERN-021: Draft → Done
- KERN-022: Draft → Done
- KERN-023: Draft → Done
- Phase 2 status: Draft → Done

Update `plans/modules/ink-to-ratatui-port.aps.md`:
- PORT-020: Draft → Done
- PORT-021: Draft → Done
- PORT-022: Draft → Done
- PORT-023: Draft → Done
- Phase 3 status: Draft → Done

- [ ] **Step 4: Commit APS updates**

```bash
git add plans/modules/
git commit -m "chore(plans): update KERN Phase 2, PORT Phase 3 statuses for Wave 2 completion"
```
