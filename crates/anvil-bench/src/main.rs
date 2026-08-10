//! anvil-bench: stress test runner and DEVACC task-level suite.
//!
//! Usage:
//!   cargo run -p anvil-bench --release                  # all stress scenarios
//!   cargo run -p anvil-bench --release -- <name>        # one stress scenario
//!   cargo run -p anvil-bench --release -- devacc --tier A
//!   cargo run -p anvil-bench --release -- devacc --tier B --dry-run

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anvil_bench::devacc::{RunTierAOptions, RunTierBOptions, run_tier_a, run_tier_b};
use anvil_bench::report::{BenchReport, ScenarioResult};
use anvil_bench::scenarios::{
    cold_start_scaling, graph_memory, incremental_throughput, policy_scaling, token_reduction,
    watcher_saturation,
};

type Scenario = (&'static str, Box<dyn Fn() -> ScenarioResult>);

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("devacc") {
        args.remove(0);
        if let Err(e) = run_devacc(&args) {
            eprintln!("devacc error: {e}");
            process::exit(1);
        }
        return;
    }

    let filter = args.first().cloned();
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
        process::exit(1);
    }

    eprintln!(
        "\n✓ {} scenarios in {:.2}s",
        report.results.len(),
        start.elapsed().as_secs_f64()
    );

    let out_dir = PathBuf::from("bench-results");
    fs::create_dir_all(&out_dir).expect("failed to create bench-results/");

    let path = report_output_path(&out_dir, report.generated_at_epoch);
    write_report_exclusive(&report, &path).expect("failed to write report");
    eprintln!("Report written to {}", path.display());
}

fn run_devacc(args: &[String]) -> Result<(), String> {
    let mut tier = "A".to_string();
    let mut scenario: Option<String> = None;
    let mut arm: Option<String> = None;
    let mut dry_run = true;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--tier" => {
                i += 1;
                tier = args
                    .get(i)
                    .ok_or("--tier requires A|B")?
                    .to_uppercase();
            }
            "--scenario" => {
                i += 1;
                scenario = Some(args.get(i).ok_or("--scenario requires id")?.clone());
            }
            "--arm" => {
                i += 1;
                arm = Some(args.get(i).ok_or("--arm requires id")?.clone());
            }
            "--dry-run" => dry_run = true,
            "--live" => dry_run = false,
            "--out" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("--out requires path")?));
            }
            "-h" | "--help" => {
                print_devacc_help();
                return Ok(());
            }
            other => return Err(format!("unknown devacc arg: {other}")),
        }
        i += 1;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let default_out = PathBuf::from(format!("benchmark-results/devacc-{stamp}"));
    let out_dir = out.unwrap_or(default_out);

    match tier.as_str() {
        "A" => {
            eprintln!("▸ DEVACC Tier A (deterministic, on-demand)...");
            let reports = run_tier_a(&RunTierAOptions {
                repo_root: None,
                scenario_filter: scenario,
                arm_filter: arm,
                out_dir: Some(out_dir.clone()),
            })?;
            eprintln!(
                "✓ {} records → {}",
                reports.len(),
                out_dir.display()
            );
            for r in &reports {
                eprintln!(
                    "  {} arm={} success={} tokens_total={}",
                    r.scenario, r.arm, r.task_success, r.tokens_total
                );
            }
        }
        "B" => {
            eprintln!("▸ DEVACC Tier B (driver={}, dry_run={dry_run})...", 
                std::env::var("ANVIL_DEVACC_DRIVER").unwrap_or_else(|_| "dry-run".into()));
            let reports = run_tier_b(&RunTierBOptions {
                repo_root: None,
                scenario_filter: scenario,
                arm_filter: arm,
                out_dir: Some(out_dir.clone()),
                dry_run,
            })?;
            eprintln!(
                "✓ {} records → {}",
                reports.len(),
                out_dir.display()
            );
            for r in &reports {
                eprintln!(
                    "  {} arm={} success={} tokens_total={} model={:?}",
                    r.scenario, r.arm, r.task_success, r.tokens_total, r.model
                );
            }
        }
        other => return Err(format!("unknown tier {other}; use A or B")),
    }
    Ok(())
}

fn print_devacc_help() {
    eprintln!(
        "\
DEVACC — Developer Acceleration benchmarks (on-demand)

  cargo run -p anvil-bench -- devacc --tier A
  cargo run -p anvil-bench -- devacc --tier A --scenario DEVACC-SCN-01
  cargo run -p anvil-bench -- devacc --tier B --dry-run
  ANVIL_DEVACC_DRIVER=external ANVIL_DEVACC_MODEL=… ANVIL_DEVACC_EXTERNAL_CMD=… \\
    cargo run -p anvil-bench -- devacc --tier B --live

Options:
  --tier A|B          Suite tier (default A)
  --scenario ID       Filter scenario
  --arm NAME          Filter arm
  --out DIR           Output directory (default benchmark-results/devacc-<ts>/)
  --dry-run           Tier B without live model (default)
  --live              Tier B live/external driver

Env:
  ANVIL_DEVACC_DRIVER=dry-run|external
  ANVIL_DEVACC_MODEL=…
  ANVIL_DEVACC_EXTERNAL_CMD=…
  ANVIL_DEVACC_SHA=…  ANVIL_DEVACC_HOST_CLASS=…
"
    );
}

fn report_output_path(out_dir: &std::path::Path, epoch: u64) -> PathBuf {
    out_dir.join(format!("report-{epoch}.json"))
}

fn write_report_exclusive(report: &BenchReport, path: &PathBuf) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let json = serde_json::to_string_pretty(report).expect("serialize report");
    file.write_all(json.as_bytes())?;
    Ok(())
}
