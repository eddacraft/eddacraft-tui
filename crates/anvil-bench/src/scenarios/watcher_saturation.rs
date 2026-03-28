//! BENCH-013: Watcher saturation scenario.
//!
//! Floods a directory with rapid file writes and measures write throughput.
//! Note: watcher_integration=false — the event counter is not wired to a
//! real watcher here; integration tests in anvil-kernel should wire it.

use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::fixture::{RepoSpec, generate_repo};
use crate::measure::{MemoryGuard, time_iterations};
use crate::report::ScenarioResult;

/// Configuration for the watcher saturation scenario.
#[derive(Debug, Clone)]
pub struct WatcherSaturationConfig {
    /// Number of rapid file writes to perform.
    pub write_count: usize,
    /// Delay between writes (zero for maximum pressure).
    pub write_interval: Duration,
    /// How long to wait for events to settle after writes finish.
    pub settle_time: Duration,
    /// Repo spec for the base directory.
    pub repo_spec: RepoSpec,
}

impl Default for WatcherSaturationConfig {
    fn default() -> Self {
        Self {
            write_count: 500,
            write_interval: Duration::ZERO,
            settle_time: Duration::from_secs(2),
            repo_spec: RepoSpec::small(),
        }
    }
}

/// Results from the watcher saturation run.
#[derive(Debug, Clone)]
pub struct SaturationMetrics {
    pub writes_performed: u64,
    pub total_write_duration: Duration,
    pub avg_write_latency: Duration,
}

/// Perform rapid file mutations and measure write throughput.
///
/// The `event_counter` is exposed so that integration tests in anvil-kernel
/// can wire it to a real `notify` watcher. This scenario does not wire it
/// itself (watcher_integration=false).
pub fn run_write_flood(
    config: &WatcherSaturationConfig,
    dir: &Path,
    _event_counter: &Arc<AtomicU64>,
) -> SaturationMetrics {
    let target_file = dir.join("__bench_target.ts");
    fs::write(&target_file, "// initial\n").expect("failed to write seed file");

    let start = Instant::now();
    for i in 0..config.write_count {
        let content = format!("const x = {i}; // mutation {i}\n");
        fs::write(&target_file, content).expect("write failed during flood");

        if !config.write_interval.is_zero() {
            std::thread::sleep(config.write_interval);
        }
    }
    let write_duration = start.elapsed();

    // Allow watcher to settle
    std::thread::sleep(config.settle_time);

    let writes = config.write_count as u64;

    let avg_write_latency = if writes == 0 {
        Duration::ZERO
    } else {
        write_duration / u32::try_from(writes).unwrap_or(u32::MAX)
    };

    SaturationMetrics {
        writes_performed: writes,
        total_write_duration: write_duration,
        avg_write_latency,
    }
}

/// Run the full saturation scenario and produce a report result.
pub fn run(config: &WatcherSaturationConfig) -> ScenarioResult {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let repo = generate_repo(&config.repo_spec, dir.path()).expect("failed to generate repo");
    let root = repo.root().to_path_buf();

    let mem = MemoryGuard::start();

    // Measure raw write throughput — both paths target the same repo tree
    let write_timing = time_iterations("file_writes", config.write_count as u64, || {
        let path = root.join("__bench_churn.ts");
        fs::write(&path, "const churn = true;\n").expect("failed to write churn file");
    });

    // Run the flood with a simulated counter (not wired to a real watcher)
    let counter = Arc::new(AtomicU64::new(0));
    let metrics = run_write_flood(config, &root, &counter);

    let mem_delta = mem.finish();

    let mut result = ScenarioResult::new("watcher_saturation");
    result.set_duration(metrics.total_write_duration + config.settle_time);
    result.add_metric("writes_performed", metrics.writes_performed as f64, "count");
    result.add_metric(
        "avg_write_latency_us",
        metrics.avg_write_latency.as_secs_f64() * 1_000_000.0,
        "us",
    );
    result.add_timing(&write_timing);
    result.add_memory("saturation", &mem_delta);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_flood_completes_all_writes() {
        let dir = tempfile::tempdir().unwrap();
        let counter = Arc::new(AtomicU64::new(0));
        let config = WatcherSaturationConfig {
            write_count: 50,
            settle_time: Duration::from_millis(100),
            ..WatcherSaturationConfig::default()
        };

        let metrics = run_write_flood(&config, dir.path(), &counter);
        assert_eq!(metrics.writes_performed, 50);
    }

    #[test]
    fn scenario_produces_report() {
        let config = WatcherSaturationConfig {
            write_count: 10,
            settle_time: Duration::from_millis(50),
            repo_spec: RepoSpec {
                file_count: 5,
                max_depth: 1,
                lines_per_file: 5,
                ..RepoSpec::default()
            },
            ..WatcherSaturationConfig::default()
        };

        let result = run(&config);
        assert_eq!(result.scenario, "watcher_saturation");
        assert!(!result.metrics.is_empty());
    }
}
