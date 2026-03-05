use std::time::Instant;

use petgraph::prelude::*;

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct SymbolNode {
    id: u64,
    kind: SymbolKind,
    name: String,
    file: String,
}

#[derive(Clone, Copy, Debug)]
enum SymbolKind {
    Function,
    Class,
    Module,
    Export,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct SymbolEdge {
    edge_type: EdgeType,
}

#[derive(Clone, Copy, Debug)]
enum EdgeType {
    Contains,
    References,
    Calls,
    Imports,
}

fn main() {
    println!("=== KERN-003: petgraph memory and performance spike ===\n");

    let scenarios: &[(usize, usize, &str)] = &[
        (100, 300, "tiny (100 nodes, 300 edges)"),
        (500, 2000, "small (500 nodes, 2000 edges)"),
        (
            2_000,
            15_000,
            "medium (2000 nodes, 15k edges) — target repo",
        ),
        (10_000, 50_000, "large (10k nodes, 50k edges)"),
        (50_000, 200_000, "xlarge (50k nodes, 200k edges)"),
    ];

    for &(node_count, edge_count, label) in scenarios {
        benchmark_graph(label, node_count, edge_count);
    }

    println!("\n=== Spike complete ===");
}

#[allow(clippy::cast_precision_loss)]
fn benchmark_graph(label: &str, node_count: usize, edge_count: usize) {
    println!("--- {label} ---\n");

    let rss_before = get_rss_bytes();
    let build_start = Instant::now();

    let mut graph: DiGraph<SymbolNode, SymbolEdge> = DiGraph::with_capacity(node_count, edge_count);
    let kinds = [
        SymbolKind::Function,
        SymbolKind::Class,
        SymbolKind::Module,
        SymbolKind::Export,
    ];
    let edge_types = [
        EdgeType::Contains,
        EdgeType::References,
        EdgeType::Calls,
        EdgeType::Imports,
    ];

    let mut node_indices = Vec::with_capacity(node_count);
    for i in 0..node_count {
        let idx = graph.add_node(SymbolNode {
            id: i as u64,
            kind: kinds[i % kinds.len()],
            name: format!("symbol_{i}"),
            file: format!("src/file_{}.ts", i / 10),
        });
        node_indices.push(idx);
    }

    for i in 0..edge_count {
        let from = node_indices[i % node_count];
        let to = node_indices[(i * 7 + 13) % node_count];
        graph.add_edge(
            from,
            to,
            SymbolEdge {
                edge_type: edge_types[i % edge_types.len()],
            },
        );
    }

    let build_elapsed = build_start.elapsed();
    let rss_after = get_rss_bytes();

    // NOTE: RSS delta is approximate — allocator reuse across scenarios in a single process
    // means later benchmarks may show artificially low deltas. For production validation,
    // run each scenario in a separate process or use absolute RSS instead.
    match (rss_before, rss_after) {
        (Some(before), Some(after)) => {
            let rss_delta = after.saturating_sub(before);
            println!("  Build time: {build_elapsed:.1?}");
            println!(
                "  RSS delta: {:.2} MB",
                rss_delta as f64 / (1024.0 * 1024.0)
            );
            println!(
                "  Nodes: {}, Edges: {}",
                graph.node_count(),
                graph.edge_count()
            );

            let query_start = Instant::now();
            let mut traversed = 0_u64;
            for idx in graph.node_indices() {
                for _neighbour in graph.neighbors(idx) {
                    traversed += 1;
                }
            }
            let query_elapsed = query_start.elapsed();
            println!("  Full traversal ({traversed} edges visited): {query_elapsed:.1?}");

            if node_count == 2000 {
                let budget_mb = 500.0;
                let rss_mb = rss_delta as f64 / (1024.0 * 1024.0);
                if rss_mb < budget_mb {
                    println!("  ✓ PASS — {rss_mb:.2} MB < {budget_mb} MB budget");
                } else {
                    println!("  ✗ FAIL — {rss_mb:.2} MB exceeds {budget_mb} MB budget");
                }
            }
        }
        _ => {
            println!("  Build time: {build_elapsed:.1?}");
            println!("  RSS: unavailable (/proc/self/statm not found — non-Linux platform?)");
            println!(
                "  Nodes: {}, Edges: {}",
                graph.node_count(),
                graph.edge_count()
            );

            let query_start = Instant::now();
            let mut traversed = 0_u64;
            for idx in graph.node_indices() {
                for _neighbour in graph.neighbors(idx) {
                    traversed += 1;
                }
            }
            let query_elapsed = query_start.elapsed();
            println!("  Full traversal ({traversed} edges visited): {query_elapsed:.1?}");

            if node_count == 2000 {
                println!("  ⚠ SKIP — memory budget check requires /proc (Linux only)");
            }
        }
    }
    println!();
}

fn get_rss_bytes() -> Option<usize> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages = statm.split_whitespace().nth(1)?.parse::<usize>().ok()?;
    Some(pages * page_size())
}

fn page_size() -> usize {
    rustix::param::page_size()
}
