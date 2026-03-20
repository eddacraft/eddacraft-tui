use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anvil_kernel::embedded::{EmbeddedConfig, run_embedded};
use anvil_kernel::graph::{GraphDelta, SymbolGraph, update_file};
use anvil_kernel::parser::Parser;
use anvil_kernel::parser::extract::extract_symbols;
use anvil_kernel::policy::config::ArchitectureConfig;
use anvil_kernel::policy::engine::PolicyEngine;
use anvil_kernel::policy::invariants::cross_layer::CrossLayerViolation;
use anvil_kernel::policy::invariants::new_dependency::NewDependencyIntroduction;
use anvil_kernel::policy::invariants::privilege_expansion::PrivilegeExpansion;
use anvil_kernel::policy::invariants::public_api::PublicApiExpansion;
use anvil_kernel::protocol::emitter::EventEmitter;
use anvil_kernel::watcher::debounce::Debouncer;
use anvil_kernel::watcher::events::{ChangeKind, FileChange};

use anvil_kernel_types::{EdgeType, EngineId, SymbolKind, SymbolNode, TrustLevel, Visibility};
use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use tempfile::TempDir;

const SAMPLE_TS: &str = r"
import { something } from './utils';
import * as fs from 'node:fs';

export function processFile(path: string): string {
    const content = fs.readFileSync(path, 'utf-8');
    return something(content);
}

export class FileProcessor {
    private cache: Map<string, string> = new Map();

    process(path: string): string {
        if (this.cache.has(path)) {
            return this.cache.get(path)!;
        }
        const result = processFile(path);
        this.cache.set(path, result);
        return result;
    }
}

function internalHelper(data: string): number {
    return data.length;
}

const transform = (input: string) => input.toUpperCase();
";

fn generate_fixture(file_count: usize) -> TempDir {
    let tmp = TempDir::new().unwrap();

    for i in 0..file_count {
        let dir = tmp.path().join(format!("src/module_{}", i / 10));
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join(format!("file_{i}.ts"));
        // Vary content slightly to avoid cache hits
        let content = format!("// file {i}\n{SAMPLE_TS}\nexport const FILE_ID_{i} = {i};\n");
        fs::write(file_path, content).unwrap();
    }

    tmp
}

/// Generate a TypeScript source string of approximately `target_loc` lines.
fn generate_ts_content(target_loc: usize, file_index: usize) -> String {
    let mut lines = Vec::with_capacity(target_loc + 10);
    lines.push(format!(
        "// Generated file #{file_index} with ~{target_loc} LOC"
    ));
    lines.push("import { something } from './utils';".to_string());
    lines.push(String::new());

    let mut loc = 3;
    let mut fn_idx = 0;

    while loc < target_loc {
        // Each function block adds ~8 lines
        lines.push(format!(
            "export function handler_{file_index}_{fn_idx}(input: string): string {{"
        ));
        lines.push("    const step1 = input.trim();".to_string());
        lines.push("    const step2 = step1.toUpperCase();".to_string());
        lines.push("    const step3 = step2.replace(/[^A-Z]/g, '');".to_string());
        lines.push("    if (step3.length === 0) {".to_string());
        lines.push("        return 'empty';".to_string());
        lines.push("    }".to_string());
        lines.push("    return step3;".to_string());
        lines.push("}".to_string());
        lines.push(String::new());

        fn_idx += 1;
        loc += 10;
    }

    lines.push(format!(
        "export const FILE_MARKER_{file_index} = {file_index};"
    ));
    lines.join("\n")
}

/// Build a [`SymbolGraph`] with the given number of nodes. Groups of 10
/// symbols share the same file so `symbols_in_file` returns ~10 results.
/// Import edges fan out from every 3rd node to the next 3 nodes so
/// `outgoing_edges` returns a variable (1-3) number of edges that scales
/// with graph density, not a constant 1.
fn build_graph_fixture(node_count: usize) -> SymbolGraph {
    let mut graph = SymbolGraph::new();

    for i in 0..node_count {
        let node = SymbolNode {
            id: i as u64,
            kind: SymbolKind::Function,
            name: format!("symbol_{i}"),
            visibility: if i % 3 == 0 {
                Visibility::Public
            } else {
                Visibility::Internal
            },
            // Group 10 symbols per file so symbols_in_file returns ~10
            file: format!("src/module_{}/file_{}.ts", i / 100, i / 10),
            trust_level: TrustLevel::Unknown,
        };
        graph.add_symbol(node).unwrap();
    }

    // Add import edges: every 3rd node fans out to the next 1-3 nodes
    for i in 0..node_count.saturating_sub(1) {
        if i % 3 == 0 {
            for offset in 1..=3.min(node_count - 1 - i) {
                let edge = anvil_kernel_types::SymbolEdge {
                    from: i as u64,
                    to: (i + offset) as u64,
                    edge_type: EdgeType::Imports,
                };
                let _ = graph.add_edge(edge);
            }
        }
    }

    graph
}

// ---------------------------------------------------------------------------
// BENCH-001: Cold graph build — extended to 500, 1k, 5k (10k omitted as it
// makes CI too slow; use the stress harness for 10k+)
// ---------------------------------------------------------------------------

fn bench_cold_graph_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_graph_build");

    for &file_count in &[10, 50, 100, 500, 1000, 5000] {
        let fixture = generate_fixture(file_count);

        group.bench_with_input(
            BenchmarkId::new("files", file_count),
            &file_count,
            |b, _| {
                b.iter(|| {
                    let config = EmbeddedConfig {
                        root: fixture.path().to_path_buf(),
                        architecture_config: None,
                        filter: None,
                    };
                    let result = run_embedded(black_box(&config)).unwrap();
                    black_box(&result.stats);
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Original: Incremental update (single file)
// ---------------------------------------------------------------------------

fn bench_incremental_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_update");

    let content = SAMPLE_TS.as_bytes();
    let path = Path::new("src/module/target.ts");

    group.bench_function("single_file_reparse_and_update", |b| {
        let mut parser = Parser::new();
        let mut graph = SymbolGraph::new();

        // Pre-populate
        let result = parser.parse_bytes(path, content).unwrap();
        let symbols = extract_symbols(&result.tree, content, path, 0);
        update_file(&mut graph, symbols);

        let updated_content = format!("{SAMPLE_TS}\nexport const UPDATED = true;\n");
        let updated_bytes = updated_content.as_bytes();

        b.iter(|| {
            let result = parser.parse_bytes(path, black_box(updated_bytes)).unwrap();
            let symbols = extract_symbols(&result.tree, updated_bytes, path, 1000);
            let delta = update_file(&mut graph, symbols);
            black_box(&delta);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// BENCH-004: Incremental update with varied file complexity
// ---------------------------------------------------------------------------

fn bench_incremental_update_varied(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_update_varied");

    for &loc in &[10, 100, 500, 1000] {
        let content = generate_ts_content(loc, 0);
        let content_bytes = content.as_bytes();
        let path = Path::new("src/module/target.ts");

        group.bench_with_input(BenchmarkId::new("loc", loc), &loc, |b, _| {
            let updated = format!("{content}\nexport const UPDATED = true;\n");
            let updated_bytes = updated.as_bytes();

            b.iter_batched(
                || {
                    // Setup: fresh parser + prepopulated graph (untimed)
                    let mut parser = Parser::new();
                    let mut graph = SymbolGraph::new();
                    let init = parser.parse_bytes(path, content_bytes).unwrap();
                    let symbols = extract_symbols(&init.tree, content_bytes, path, 0);
                    update_file(&mut graph, symbols);
                    (parser, graph)
                },
                |(mut parser, mut graph)| {
                    // Timed: only the incremental update path
                    let result = parser.parse_bytes(path, black_box(updated_bytes)).unwrap();
                    let symbols = extract_symbols(&result.tree, updated_bytes, path, 10_000);
                    let delta = update_file(&mut graph, symbols);
                    black_box(&delta);
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Original: Policy evaluation (4 H1 invariants)
// ---------------------------------------------------------------------------

fn bench_policy_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("policy_evaluation");

    let content = SAMPLE_TS.as_bytes();
    let path = Path::new("src/domain/service.ts");

    let config_yaml = r#"
layers:
  - name: domain
    paths: ["src/domain/*"]
    allowed_imports: [domain]
  - name: infra
    paths: ["src/infra/*"]
    allowed_imports: [domain, infra]
"#;

    group.bench_function("evaluate_all_invariants", |b| {
        let arch_config = ArchitectureConfig::from_yaml(config_yaml).unwrap();

        let mut parser = Parser::new();
        let result = parser.parse_bytes(path, content).unwrap();
        let symbols = extract_symbols(&result.tree, content, path, 0);

        let mut graph = SymbolGraph::new();
        let delta = update_file(&mut graph, symbols);

        b.iter(|| {
            let mut engine = PolicyEngine::new();
            engine.register(Box::new(CrossLayerViolation));
            engine.register(Box::new(NewDependencyIntroduction));
            engine.register(Box::new(PublicApiExpansion));
            engine.register(Box::new(PrivilegeExpansion));

            let violations = engine.evaluate(black_box(&delta), &graph, &arch_config);
            black_box(&violations);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// BENCH-005: Policy evaluation scaling — vary invariant count and delta size
// ---------------------------------------------------------------------------

fn bench_policy_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("policy_scaling");

    let config_yaml = r#"
layers:
  - name: domain
    paths: ["src/domain/*"]
    allowed_imports: [domain]
  - name: infra
    paths: ["src/infra/*"]
    allowed_imports: [domain, infra]
  - name: api
    paths: ["src/api/*"]
    allowed_imports: [domain, api]
"#;

    let arch_config = ArchitectureConfig::from_yaml(config_yaml).unwrap();

    // Vary invariant count: 4 (H1 real) then pad with duplicated invariants
    // to simulate higher invariant counts. We use the real invariants
    // repeatedly to keep evaluation cost realistic.
    for &invariant_count in &[4, 10, 25, 50] {
        // Vary delta size
        for &delta_symbols in &[1u64, 10, 50] {
            // Rebuild graph per input combination so edges don't accumulate
            // across iterations, which would make later benchmarks measure
            // a progressively denser graph than the label suggests.
            let mut graph = SymbolGraph::new();
            for i in 0..100u64 {
                // Mix trust levels so PrivilegeExpansion fires on ~25% of
                // added symbols (those with TrustLevel::Privileged).
                let trust = if i % 4 == 0 {
                    TrustLevel::Privileged
                } else {
                    TrustLevel::Unknown
                };
                let node = SymbolNode {
                    id: i,
                    kind: SymbolKind::Function,
                    name: format!("fn_{i}"),
                    visibility: Visibility::Public,
                    file: format!("src/domain/mod_{}.ts", i / 10),
                    trust_level: trust,
                };
                graph.add_symbol(node).unwrap();
            }
            for i in 100..200u64 {
                // Mark ~50% of infra symbols as External so
                // NewDependencyIntroduction fires on cross-layer edges
                // targeting external modules.
                let trust = if i % 2 == 0 {
                    TrustLevel::External
                } else {
                    TrustLevel::Unknown
                };
                let node = SymbolNode {
                    id: i,
                    kind: SymbolKind::Function,
                    name: format!("fn_{i}"),
                    visibility: Visibility::Public,
                    file: format!("src/infra/mod_{}.ts", i / 10),
                    trust_level: trust,
                };
                graph.add_symbol(node).unwrap();
            }

            // Insert cross-layer edges into the graph so
            // CrossLayerViolation actually traverses real imports
            let edge_count = delta_symbols.min(50);
            for i in 0..edge_count {
                let _ = graph.add_edge(anvil_kernel_types::SymbolEdge {
                    from: i,
                    to: 100 + i,
                    edge_type: EdgeType::Imports,
                });
            }

            let delta = GraphDelta {
                added_symbols: (0..delta_symbols).collect(),
                removed_symbols: Vec::new(),
                added_edges: (0..edge_count)
                    .map(|i| (i, 100 + i, EdgeType::Imports))
                    .collect(),
                removed_edges: Vec::new(),
                errors: Vec::new(),
                previously_imported: HashSet::new(),
                previously_public: HashSet::new(),
                previously_privileged: HashSet::new(),
                file: "src/domain/mod_0.ts".to_string(),
            };

            group.bench_with_input(
                BenchmarkId::new(
                    format!("{invariant_count}_invariants"),
                    format!("{delta_symbols}_symbols"),
                ),
                &(invariant_count, delta_symbols),
                |b, _| {
                    b.iter(|| {
                        // NOTE: PolicyEngine::new() + register() are kept inside
                        // b.iter because evaluate() is &mut self (it mutates
                        // `seen`), so we need a fresh engine each sample to avoid
                        // the dedup set masking evaluation cost on repeat runs.
                        // The allocation cost of new() + register() is negligible
                        // (Vec::push) compared to evaluate().
                        let mut engine = PolicyEngine::new();
                        for j in 0..invariant_count {
                            match j % 4 {
                                0 => engine.register(Box::new(CrossLayerViolation)),
                                1 => engine.register(Box::new(NewDependencyIntroduction)),
                                2 => engine.register(Box::new(PublicApiExpansion)),
                                _ => engine.register(Box::new(PrivilegeExpansion)),
                            }
                        }
                        let violations = engine.evaluate(black_box(&delta), &graph, &arch_config);
                        black_box(&violations);
                    });
                },
            );
        }
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Original: Event emission
// ---------------------------------------------------------------------------

fn bench_event_emission(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_emission");

    group.bench_function("emit_1000_progress_events", |b| {
        b.iter(|| {
            let (tx, rx) = std::sync::mpsc::channel();
            let emitter = EventEmitter::new(tx, EngineId::Rust);

            for i in 0..1000 {
                emitter.progress("bench", i, 1000);
            }

            drop(emitter);
            let count = rx.try_iter().count();
            black_box(count);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// BENCH-002: Graph query benchmarks
// ---------------------------------------------------------------------------

fn bench_graph_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_query");

    for &node_count in &[1_000, 5_000, 10_000] {
        let graph = build_graph_fixture(node_count);

        // Benchmark symbols_in_file for a file in the middle of the graph.
        // Must match build_graph_fixture's file format: module_{i/100}/file_{i/10}.
        let mid = node_count / 2;
        let target_file = format!("src/module_{}/file_{}.ts", mid / 100, mid / 10);

        group.bench_with_input(
            BenchmarkId::new("symbols_in_file", node_count),
            &node_count,
            |b, _| {
                b.iter(|| {
                    let symbols = graph.symbols_in_file(black_box(&target_file));
                    black_box(&symbols);
                });
            },
        );

        // Benchmark outgoing_edges for a node with edges
        let target_id = ((node_count / 2) / 3 * 3) as u64; // ensure it's a node with edges
        group.bench_with_input(
            BenchmarkId::new("outgoing_edges", node_count),
            &node_count,
            |b, _| {
                b.iter(|| {
                    let edges = graph.outgoing_edges(black_box(target_id));
                    black_box(&edges);
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// BENCH-003: Debouncer throughput benchmarks
// ---------------------------------------------------------------------------

fn bench_debouncer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("debouncer_throughput");

    for &pending_count in &[100, 500, 1000] {
        // Benchmark: record N changes then tick to flush
        group.bench_with_input(
            BenchmarkId::new("record_and_tick", pending_count),
            &pending_count,
            |b, &n| {
                b.iter(|| {
                    // Use a zero window so tick() flushes all pending changes
                    let mut debouncer = Debouncer::new(Duration::from_millis(0), n + 1);
                    let mut flush_count = 0u32;

                    for i in 0..n {
                        let change = FileChange {
                            path: PathBuf::from(format!("src/file_{i}.ts")),
                            kind: ChangeKind::Modified,
                        };
                        if debouncer.record(change).is_some() {
                            flush_count += 1;
                        }
                    }

                    // Flush via tick to measure the full record+tick cycle
                    if let Some(batch) = debouncer.tick() {
                        flush_count += 1;
                        black_box(&batch);
                    }
                    black_box(flush_count);
                });
            },
        );

        // Benchmark: record N changes with backpressure (max_pending = N/2)
        group.bench_with_input(
            BenchmarkId::new("backpressure_flush", pending_count),
            &pending_count,
            |b, &n| {
                b.iter(|| {
                    let mut debouncer = Debouncer::new(Duration::from_secs(60), n / 2);
                    let mut flush_count = 0u32;

                    for i in 0..n {
                        let change = FileChange {
                            path: PathBuf::from(format!("src/file_{i}.ts")),
                            kind: ChangeKind::Modified,
                        };
                        if debouncer.record(change).is_some() {
                            flush_count += 1;
                        }
                    }

                    black_box(flush_count);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cold_graph_build,
    bench_incremental_update,
    bench_incremental_update_varied,
    bench_policy_evaluation,
    bench_policy_scaling,
    bench_event_emission,
    bench_graph_query,
    bench_debouncer_throughput,
);
criterion_main!(benches);
