//! BENCH-013: Watcher saturation scenario.
//!
//! Floods a watched directory with rapid file writes and measures event
//! delivery, drop rate, and settle latency through the real anvil-kernel watcher.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use anvil_kernel::watcher::filter::FileFilter;
use anvil_kernel::watcher::{WatcherConfig, start_watcher};

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
    pub unique_events_received: u64,
    pub event_changes_received: u64,
    pub drop_rate: f64,
    pub total_write_duration: Duration,
    pub settle_latency: Duration,
    pub avg_write_latency: Duration,
    pub watched_directories: u64,
    pub watch_registration_failures: u64,
}

/// Perform rapid file mutations under the real watcher and measure event
/// delivery. One file is written per event so debouncing can coalesce timing but
/// should not collapse the expected path cardinality.
pub fn run_write_flood(config: &WatcherSaturationConfig, dir: &Path) -> SaturationMetrics {
    let watcher_config = WatcherConfig {
        root: dir.to_path_buf(),
        debounce_window: Duration::from_millis(10),
        max_pending: config.write_count.saturating_add(1).max(1),
        tick_interval: Duration::from_millis(5),
        filter: Some(FileFilter::default()),
        extra_watch_dirs: Vec::new(),
    };
    let (_handle, batch_rx, diagnostics) =
        start_watcher(&watcher_config, None).expect("failed to start watcher");

    let mut expected = HashSet::with_capacity(config.write_count);
    for i in 0..config.write_count {
        let path = dir.join(format!("__bench_target_{i:04}.ts"));
        fs::write(&path, "// initial\n").expect("failed to write seed file");
        expected.insert(path);
    }

    // Give notify a short moment to finish registration and consume seed writes.
    std::thread::sleep(Duration::from_millis(25));
    while batch_rx.try_recv().is_ok() {}

    let start = Instant::now();
    for i in 0..config.write_count {
        let target_file = dir.join(format!("__bench_target_{i:04}.ts"));
        let content = format!("const x = {i}; // mutation {i}\n");
        fs::write(&target_file, content).expect("write failed during flood");

        if !config.write_interval.is_zero() {
            std::thread::sleep(config.write_interval);
        }
    }
    let write_duration = start.elapsed();

    let deadline = Instant::now() + config.settle_time;
    let mut received = HashSet::with_capacity(config.write_count);
    let mut event_changes = 0u64;
    let mut last_event_at = None;

    while received.len() < expected.len() {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let timeout = deadline
            .saturating_duration_since(now)
            .min(Duration::from_millis(25));
        match batch_rx.recv_timeout(timeout) {
            Ok(batch) => {
                last_event_at = Some(Instant::now());
                for change in batch.changes {
                    event_changes += 1;
                    if expected.contains(&change.path) {
                        received.insert(change.path);
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let writes = config.write_count as u64;
    let unique_events = received.len() as u64;
    let drop_rate = if writes == 0 {
        0.0
    } else {
        1.0 - (unique_events as f64 / writes as f64)
    };

    let avg_write_latency = if writes == 0 {
        Duration::ZERO
    } else {
        write_duration / u32::try_from(writes).unwrap_or(u32::MAX)
    };

    SaturationMetrics {
        writes_performed: writes,
        unique_events_received: unique_events,
        event_changes_received: event_changes,
        drop_rate,
        total_write_duration: write_duration,
        settle_latency: last_event_at
            .map_or(Duration::ZERO, |last| last.saturating_duration_since(start)),
        avg_write_latency,
        watched_directories: diagnostics.registered,
        watch_registration_failures: diagnostics.failed,
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

    let metrics = run_write_flood(config, &root);

    let mem_delta = mem.finish();

    let mut result = ScenarioResult::new("watcher_saturation");
    result.set_duration(metrics.total_write_duration + config.settle_time);
    result.add_metric("writes_performed", metrics.writes_performed as f64, "count");
    result.add_metric(
        "unique_events_received",
        metrics.unique_events_received as f64,
        "count",
    );
    result.add_metric(
        "event_changes_received",
        metrics.event_changes_received as f64,
        "count",
    );
    result.add_metric("drop_rate", metrics.drop_rate, "ratio");
    result.add_metric(
        "settle_latency_ms",
        metrics.settle_latency.as_secs_f64() * 1000.0,
        "ms",
    );
    result.add_metric(
        "avg_write_latency_us",
        metrics.avg_write_latency.as_secs_f64() * 1_000_000.0,
        "us",
    );
    result.add_metric(
        "watched_directories",
        metrics.watched_directories as f64,
        "count",
    );
    result.add_metric(
        "watch_registration_failures",
        metrics.watch_registration_failures as f64,
        "count",
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
        let config = WatcherSaturationConfig {
            write_count: 50,
            settle_time: Duration::from_secs(2),
            ..WatcherSaturationConfig::default()
        };

        let metrics = run_write_flood(&config, dir.path());
        assert_eq!(metrics.writes_performed, 50);
        assert_eq!(metrics.watch_registration_failures, 0);
        assert!(metrics.watched_directories > 0);
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
