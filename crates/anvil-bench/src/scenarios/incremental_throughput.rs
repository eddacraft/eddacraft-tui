//! BENCH-015: Incremental throughput under sustained load.
//!
//! Simulates a sustained edit stream against a symbol graph and measures
//! how many incremental updates per second the graph can process.

use std::time::{Duration, Instant};

use anvil_kernel_types::{EdgeType, SymbolEdge, SymbolKind, SymbolNode, TrustLevel, Visibility};
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

use crate::measure::MemoryGuard;
use crate::report::ScenarioResult;

/// Configuration for the incremental throughput scenario.
#[derive(Debug, Clone)]
pub struct IncrementalThroughputConfig {
    /// Initial graph size (number of nodes).
    pub initial_nodes: usize,
    /// Edges per node in the initial graph.
    pub edges_per_node: usize,
    /// How long to sustain the edit stream.
    pub sustain_duration: Duration,
    /// Fraction of nodes to update per edit batch (0.0-1.0).
    pub batch_fraction: f64,
}

impl Default for IncrementalThroughputConfig {
    fn default() -> Self {
        Self {
            initial_nodes: 5_000,
            edges_per_node: 3,
            sustain_duration: Duration::from_secs(5),
            batch_fraction: 0.05,
        }
    }
}

/// Simulated incremental update: remove a node's edges and re-add them,
/// mimicking what happens when a file is re-parsed.
fn apply_incremental_update(
    graph: &mut Graph<SymbolNode, SymbolEdge>,
    target: NodeIndex,
    node_count: usize,
    edges_per_node: usize,
) {
    // `Graph::remove_edge` can swap internal edge slots, invalidating a
    // pre-collected `EdgeIndex` list. Re-query each next outgoing edge instead.
    while let Some(edge_id) = graph.edges(target).next().map(|e| e.id()) {
        graph.remove_edge(edge_id);
    }

    // Re-add edges with slightly different targets (simulating edit)
    let target_idx = target.index();
    let edge_types = [
        EdgeType::Contains,
        EdgeType::References,
        EdgeType::Calls,
        EdgeType::Imports,
    ];

    for e in 0..edges_per_node {
        let dest = (target_idx + e + 2) % node_count;
        graph.add_edge(
            target,
            NodeIndex::new(dest),
            SymbolEdge {
                from: target_idx as u64,
                to: dest as u64,
                edge_type: edge_types[e % edge_types.len()],
            },
        );
    }
}

/// Build the initial graph for the throughput test.
fn build_initial_graph(
    node_count: usize,
    edges_per_node: usize,
) -> (Graph<SymbolNode, SymbolEdge>, Vec<NodeIndex>) {
    let mut graph = Graph::new();
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
                visibility: Visibility::Internal,
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
                    edge_type: EdgeType::References,
                },
            );
        }
    }

    (graph, indices)
}

/// Run the incremental throughput scenario.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn run(config: &IncrementalThroughputConfig) -> ScenarioResult {
    if config.initial_nodes == 0 {
        let mut result = ScenarioResult::new("incremental_throughput");
        result.set_duration(Duration::ZERO);
        result.add_metric("initial_nodes", 0.0, "count");
        result.add_metric("total_updates", 0.0, "count");
        result.add_metric("updates_per_sec", 0.0, "ops/s");
        return result;
    }

    let mem = MemoryGuard::start();
    let (mut graph, indices) = build_initial_graph(config.initial_nodes, config.edges_per_node);

    let batch_size = ((config.initial_nodes as f64 * config.batch_fraction) as usize).max(1);

    let start = Instant::now();
    let mut total_updates = 0u64;
    let mut batch_count = 0u64;

    while start.elapsed() < config.sustain_duration {
        // Each batch updates a sliding window of nodes
        let offset = (batch_count as usize * batch_size) % config.initial_nodes;
        for i in 0..batch_size {
            let node_idx = (offset + i) % indices.len();
            apply_incremental_update(
                &mut graph,
                indices[node_idx],
                config.initial_nodes,
                config.edges_per_node,
            );
            total_updates += 1;
        }
        batch_count += 1;
    }

    let elapsed = start.elapsed();
    let mem_delta = mem.finish();

    let updates_per_sec = if elapsed.as_secs_f64() > 0.0 {
        total_updates as f64 / elapsed.as_secs_f64()
    } else {
        0.0
    };

    let mut result = ScenarioResult::new("incremental_throughput");
    result.set_duration(elapsed);
    result.add_metric("initial_nodes", config.initial_nodes as f64, "count");
    result.add_metric("batch_size", batch_size as f64, "count");
    result.add_metric("total_updates", total_updates as f64, "count");
    result.add_metric("batch_count", batch_count as f64, "count");
    result.add_metric("updates_per_sec", updates_per_sec, "ops/s");
    result.add_metric("final_node_count", graph.node_count() as f64, "count");
    result.add_metric("final_edge_count", graph.edge_count() as f64, "count");
    result.add_memory("incremental", &mem_delta);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremental_update_preserves_node_count() {
        let (mut graph, indices) = build_initial_graph(50, 2);
        let initial_nodes = graph.node_count();
        apply_incremental_update(&mut graph, indices[0], 50, 2);
        assert_eq!(graph.node_count(), initial_nodes);
    }

    #[test]
    fn scenario_produces_throughput_metrics() {
        let config = IncrementalThroughputConfig {
            initial_nodes: 100,
            edges_per_node: 2,
            sustain_duration: Duration::from_millis(200),
            batch_fraction: 0.1,
        };

        let result = run(&config);
        assert_eq!(result.scenario, "incremental_throughput");

        let ups = result
            .metrics
            .iter()
            .find(|m| m.name == "updates_per_sec")
            .expect("should have updates_per_sec metric");
        assert!(ups.value > 0.0, "should process some updates");
    }
}
