//! anvil-bench: stress test runner for anvil-kernel.
//!
//! Runs all scenarios at production scale and writes JSON reports to
//! `bench-results/`. Intended for nightly runs on dedicated hardware
//! where results are comparable across runs.
//!
//! Usage:
//!   cargo run -p anvil-bench --release           # run all scenarios
//!   cargo run -p anvil-bench --release -- <name>  # run one scenario

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use anvil_bench::report::BenchReport;
use anvil_bench::scenarios::{
    cold_start_scaling, graph_memory, incremental_throughput, policy_scaling, watcher_saturation,
};

fn main() {
    let filter = env::args().nth(1);
    let mut report = BenchReport::new("anvil-stress");
    let start = Instant::now();

    let scenarios: Vec<(&str, Box<dyn Fn() -> anvil_bench::report::ScenarioResult>)> = vec![
        (
            "watcher_saturation",
            Box::new(|| watcher_saturation::run(&watcher_saturation::WatcherSaturationConfig::default())),
        ),
        (
            "graph_memory",
            Box::new(|| graph_memory::run(&graph_memory::GraphMemoryConfig::default())),
        ),
        (
            "incremental_throughput",
            Box::new(|| {
                incremental_throughput::run(
                    &incremental_throughput::IncrementalThroughputConfig::default(),
                )
            }),
        ),
        (
            "policy_scaling",
            Box::new(|| policy_scaling::run(&policy_scaling::PolicyScalingConfig::default())),
        ),
        (
            "cold_start_scaling",
            Box::new(|| cold_start_scaling::run(&cold_start_scaling::ColdStartConfig::default())),
        ),
    ];

    for (name, run_fn) in &scenarios {
        if let Some(ref f) = filter {
            if name != f {
                continue;
            }
        }

        eprintln!("▸ Running {name}...");
        let scenario_start = Instant::now();
        let result = run_fn();
        eprintln!(
            "  done in {:.2}s ({} metrics)",
            scenario_start.elapsed().as_secs_f64(),
            result.metrics.len()
        );
        report.add_result(result);
    }

    if report.results.is_empty() {
        eprintln!("No scenarios matched filter {:?}", filter);
        std::process::exit(1);
    }

    eprintln!(
        "\n✓ {} scenarios in {:.2}s",
        report.results.len(),
        start.elapsed().as_secs_f64()
    );

    // Write JSON report
    let out_dir = PathBuf::from("bench-results");
    fs::create_dir_all(&out_dir).expect("failed to create bench-results/");

    let timestamp = report.generated_at_epoch;
    let path = out_dir.join(format!("stress-{timestamp}.json"));
    report.write_to(&path).expect("failed to write report");
    eprintln!("Report written to {}", path.display());

    // Also print summary to stdout
    print_summary(&report);
}

fn print_summary(report: &BenchReport) {
    println!("\n{:=<60}", "");
    println!(" Anvil Stress Test Results");
    println!("{:=<60}\n", "");

    for result in &report.results {
        println!("── {} ({:.2}s)", result.scenario, result.duration_secs);
        for metric in &result.metrics {
            println!("   {:<40} {:>10.2} {}", metric.name, metric.value, metric.unit);
        }
        println!();
    }
}
