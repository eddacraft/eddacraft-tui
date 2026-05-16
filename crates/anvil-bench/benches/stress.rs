use std::time::Duration;

use anvil_bench::scenarios::{
    cold_start_scaling, graph_memory, incremental_throughput, policy_scaling, watcher_saturation,
};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_graph_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_memory");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("small_graph", |b| {
        let config = graph_memory::GraphMemoryConfig {
            steps: vec![100, 500, 1_000],
            edges_per_node: 3,
        };
        b.iter(|| graph_memory::run(&config));
    });

    group.finish();
}

fn bench_watcher_saturation(c: &mut Criterion) {
    let mut group = c.benchmark_group("watcher_saturation");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("small_burst", |b| {
        let config = watcher_saturation::WatcherSaturationConfig {
            write_count: 50,
            settle_time: Duration::from_millis(150),
            repo_spec: anvil_bench::fixture::RepoSpec {
                file_count: 20,
                max_depth: 2,
                lines_per_file: 10,
                ..anvil_bench::fixture::RepoSpec::default()
            },
            ..watcher_saturation::WatcherSaturationConfig::default()
        };
        b.iter(|| watcher_saturation::run(&config));
    });

    group.finish();
}

fn bench_incremental_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_throughput");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("sustained_edits", |b| {
        let config = incremental_throughput::IncrementalThroughputConfig {
            initial_nodes: 500,
            edges_per_node: 2,
            sustain_duration: Duration::from_millis(100),
            batch_fraction: 0.1,
        };
        b.iter(|| incremental_throughput::run(&config));
    });

    group.finish();
}

fn bench_policy_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("policy_scaling");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("rule_scaling", |b| {
        let config = policy_scaling::PolicyScalingConfig {
            symbol_count: 200,
            rule_steps: vec![10, 50, 100],
        };
        b.iter(|| policy_scaling::run(&config));
    });

    group.finish();
}

fn bench_cold_start(c: &mut Criterion) {
    use anvil_bench::fixture::RepoSpec;

    let mut group = c.benchmark_group("cold_start");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("scaling", |b| {
        let config = cold_start_scaling::ColdStartConfig {
            steps: vec![
                RepoSpec {
                    file_count: 20,
                    max_depth: 2,
                    lines_per_file: 10,
                    ..RepoSpec::default()
                },
                RepoSpec {
                    file_count: 100,
                    max_depth: 3,
                    lines_per_file: 20,
                    ..RepoSpec::default()
                },
            ],
        };
        b.iter(|| cold_start_scaling::run(&config));
    });

    group.finish();
}

criterion_group!(
    stress,
    bench_watcher_saturation,
    bench_graph_memory,
    bench_incremental_throughput,
    bench_policy_scaling,
    bench_cold_start
);
criterion_main!(stress);
