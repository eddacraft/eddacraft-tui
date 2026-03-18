use std::fs;
use std::path::Path;

use anvil_kernel::embedded::{EmbeddedConfig, run_embedded};
use anvil_kernel::graph::{SymbolGraph, update_file};
use anvil_kernel::parser::Parser;
use anvil_kernel::parser::extract::extract_symbols;
use anvil_kernel::policy::config::ArchitectureConfig;
use anvil_kernel::policy::engine::PolicyEngine;
use anvil_kernel::policy::invariants::cross_layer::CrossLayerViolation;
use anvil_kernel::policy::invariants::new_dependency::NewDependencyIntroduction;
use anvil_kernel::policy::invariants::privilege_expansion::PrivilegeExpansion;
use anvil_kernel::policy::invariants::public_api::PublicApiExpansion;
use anvil_kernel::protocol::emitter::EventEmitter;

use anvil_kernel_types::EngineId;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
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

fn bench_cold_graph_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_graph_build");

    for &file_count in &[10, 50, 100] {
        let fixture = generate_fixture(file_count);

        group.bench_function(format!("{file_count}_files"), |b| {
            b.iter(|| {
                let config = EmbeddedConfig {
                    root: fixture.path().to_path_buf(),
                    architecture_config: None,
                    filter: None,
                };
                let result = run_embedded(black_box(&config)).unwrap();
                black_box(&result.stats);
            });
        });
    }

    group.finish();
}

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

criterion_group!(
    benches,
    bench_cold_graph_build,
    bench_incremental_update,
    bench_policy_evaluation,
    bench_event_emission,
);
criterion_main!(benches);
