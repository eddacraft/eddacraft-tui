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
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anvil_bench::report::{BenchReport, ScenarioResult};
use anvil_bench::scenarios::{
    cold_start_scaling, graph_memory, incremental_throughput, policy_scaling, token_reduction,
    watcher_saturation,
};

type Scenario = (&'static str, Box<dyn Fn() -> ScenarioResult>);

fn main() {
    let filter = env::args().nth(1);
    let mut report = BenchReport::new("anvil-stress");
    let start = Instant::now();

    let scenarios: Vec<Scenario> = vec![
        (
            "watcher_saturation",
            Box::new(|| {
                watcher_saturation::run(&watcher_saturation::WatcherSaturationConfig::default())
            }),
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
        (
            "token_reduction",
            Box::new(|| token_reduction::run(&token_reduction::TokenReductionConfig::default())),
        ),
    ];

    for (name, run_fn) in &scenarios {
        if let Some(ref f) = filter
            && name != f
        {
            continue;
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
        eprintln!("No scenarios matched filter {filter:?}");
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

    let path = report_output_path(&out_dir, report.generated_at_epoch);
    write_report_exclusive(&report, &path).expect("failed to write report");
    eprintln!("Report written to {}", path.display());

    // Also print summary to stdout
    print_summary(&report);
}

fn report_output_path(out_dir: &std::path::Path, generated_at_epoch: u64) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    out_dir.join(format!(
        "stress-{generated_at_epoch}-{}-{nanos}.json",
        process::id()
    ))
}

fn write_report_exclusive(report: &BenchReport, path: &std::path::Path) -> std::io::Result<()> {
    let json = report
        .to_json()
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(json.as_bytes())
}

fn print_summary(report: &BenchReport) {
    println!("\n{:=<60}", "");
    println!(" anvil Stress Test Results");
    println!("{:=<60}\n", "");

    for result in &report.results {
        println!("── {} ({:.2}s)", result.scenario, result.duration_secs);
        for metric in &result.metrics {
            println!(
                "   {:<40} {:>10.2} {}",
                metric.name, metric.value, metric.unit
            );
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_report_exclusive_refuses_to_overwrite_existing_report() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stress-fixed.json");
        let report = BenchReport::new("anvil-stress");

        write_report_exclusive(&report, &path).expect("write initial report");
        let first = fs::read_to_string(&path).expect("read initial report");

        let err = write_report_exclusive(&report, &path)
            .expect_err("second write to same report path should fail");

        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(
            fs::read_to_string(&path).expect("report should remain unchanged"),
            first
        );
    }
}
