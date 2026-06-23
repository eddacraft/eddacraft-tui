//! BENCH-014: Graph memory ceiling scenario.
//!
//! Builds progressively larger symbol graphs and measures RSS at each step
//! to identify memory scaling characteristics and ceilings.

use std::time::Instant;

use anvil_kernel_types::{EdgeType, SymbolEdge, SymbolKind, SymbolNode, TrustLevel, Visibility};
use petgraph::Graph;

use crate::measure::{MemoryGuard, MemorySnapshot};
use crate::report::ScenarioResult;

/// Configuration for the graph memory scenario.
#[derive(Debug, Clone)]
pub struct GraphMemoryConfig {
    /// Node counts to test at each step (ascending).
    pub steps: Vec<usize>,
    /// Average edges per node (fanout).
    pub edges_per_node: usize,
}

impl Default for GraphMemoryConfig {
    fn default() -> Self {
        Self {
            steps: vec![100, 500, 1_000, 5_000, 10_000, 50_000],
            edges_per_node: 3,
        }
    }
}

/// A single measurement point for the graph memory curve.
#[derive(Debug, Clone)]
pub struct GraphStepMeasurement {
    pub node_count: usize,
    pub edge_count: usize,
    pub rss_kib: u64,
    pub rss_delta_kib: u64,
    pub build_duration_ms: f64,
}

/// Build a petgraph graph with the given number of nodes and edges per node.
fn build_graph(node_count: usize, edges_per_node: usize) -> Graph<SymbolNode, SymbolEdge> {
    let mut graph = Graph::new();

    if node_count == 0 {
        return graph;
    }

    let edge_types = [
        EdgeType::Contains,
        EdgeType::References,
        EdgeType::Calls,
        EdgeType::Imports,
    ];
    let kinds = [
        SymbolKind::Function,
        SymbolKind::Class,
        SymbolKind::Module,
        SymbolKind::Export,
    ];

    let indices: Vec<_> = (0..node_count)
        .map(|i| {
            graph.add_node(SymbolNode {
                id: i as u64,
                kind: kinds[i % kinds.len()],
                name: format!("sym_{i}"),
                visibility: if i % 3 == 0 {
                    Visibility::Public
                } else {
                    Visibility::Internal
                },
                file: format!("src/mod_{}.ts", i / 10),
                trust_level: TrustLevel::Internal,
                span: None,
            })
        })
        .collect();

    for (i, &from_idx) in indices.iter().enumerate() {
        for e in 0..edges_per_node {
            let target = (i + e + 1) % node_count;
            graph.add_edge(
                from_idx,
                indices[target],
                SymbolEdge {
                    from: i as u64,
                    to: target as u64,
                    edge_type: edge_types[(i + e) % edge_types.len()],
                },
            );
        }
    }

    graph
}

/// Run the graph memory ceiling scenario.
pub fn run(config: &GraphMemoryConfig) -> ScenarioResult {
    let mem = MemoryGuard::start();
    let mut measurements = Vec::with_capacity(config.steps.len());

    for &node_count in &config.steps {
        let before = MemorySnapshot::now();
        let start = Instant::now();

        let graph = build_graph(node_count, config.edges_per_node);

        let build_ms = start.elapsed().as_secs_f64() * 1000.0;
        let after = MemorySnapshot::now();

        let delta_kib = after.rss_kib.saturating_sub(before.rss_kib);

        measurements.push(GraphStepMeasurement {
            node_count,
            edge_count: graph.edge_count(),
            rss_kib: after.rss_kib,
            rss_delta_kib: delta_kib,
            build_duration_ms: build_ms,
        });

        // Keep graph alive through measurement, then drop
        drop(graph);
    }

    let mem_delta = mem.finish();

    let mut result = ScenarioResult::new("graph_memory");
    result.set_duration(std::time::Duration::from_secs_f64(
        measurements
            .iter()
            .map(|m| m.build_duration_ms)
            .sum::<f64>()
            / 1000.0,
    ));

    for m in &measurements {
        let prefix = format!("step_{}", m.node_count);
        result.add_metric(&format!("{prefix}_nodes"), m.node_count as f64, "count");
        result.add_metric(&format!("{prefix}_edges"), m.edge_count as f64, "count");
        result.add_metric(&format!("{prefix}_rss_kib"), m.rss_kib as f64, "KiB");
        result.add_metric(
            &format!("{prefix}_rss_delta_kib"),
            m.rss_delta_kib as f64,
            "KiB",
        );
        result.add_metric(&format!("{prefix}_build_ms"), m.build_duration_ms, "ms");
    }

    if let (Some(first), Some(last)) = (measurements.first(), measurements.last())
        && first.node_count > 0
        && last.node_count > first.node_count
    {
        let rss_per_node = (last.rss_kib as f64 - first.rss_kib as f64)
            / (last.node_count as f64 - first.node_count as f64);
        result.add_metric("rss_per_node_bytes", rss_per_node * 1024.0, "bytes");
    }

    result.add_memory("graph_total", &mem_delta);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_graph_creates_correct_counts() {
        let graph = build_graph(100, 3);
        assert_eq!(graph.node_count(), 100);
        assert_eq!(graph.edge_count(), 300);
    }

    #[test]
    fn scenario_produces_report() {
        let config = GraphMemoryConfig {
            steps: vec![10, 50, 100],
            edges_per_node: 2,
        };

        let result = run(&config);
        assert_eq!(result.scenario, "graph_memory");
        assert!(result.metrics.len() >= 15); // 5 metrics per step
    }

    #[test]
    fn graph_scales_linearly_in_edges() {
        let g1 = build_graph(100, 2);
        let g2 = build_graph(100, 4);
        assert_eq!(g1.edge_count(), 200);
        assert_eq!(g2.edge_count(), 400);
    }
}
