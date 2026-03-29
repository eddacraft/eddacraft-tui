//! BENCH-017: Cold start scaling scenario.
//!
//! Measures startup time (file discovery + initial graph construction)
//! against progressively larger synthetic repositories.

use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::fixture::{RepoSpec, count_files, generate_repo};
use crate::measure::MemoryGuard;
use crate::report::ScenarioResult;

/// Configuration for the cold start scenario.
#[derive(Debug, Clone)]
pub struct ColdStartConfig {
    /// Repo specs to test at each step (ascending by file count).
    pub steps: Vec<RepoSpec>,
}

impl Default for ColdStartConfig {
    fn default() -> Self {
        Self {
            steps: vec![
                RepoSpec {
                    file_count: 50,
                    max_depth: 2,
                    lines_per_file: 30,
                    ..RepoSpec::default()
                },
                RepoSpec {
                    file_count: 200,
                    max_depth: 4,
                    lines_per_file: 50,
                    ..RepoSpec::default()
                },
                RepoSpec {
                    file_count: 1_000,
                    max_depth: 6,
                    lines_per_file: 80,
                    ..RepoSpec::default()
                },
                RepoSpec {
                    file_count: 5_000,
                    max_depth: 8,
                    lines_per_file: 100,
                    ..RepoSpec::default()
                },
            ],
        }
    }
}

/// Simulate a cold start: walk the directory tree, read every file, and
/// compute basic statistics (mimicking initial parse + graph build).
fn simulate_cold_start(root: &Path) -> ColdStartMetrics {
    let start = Instant::now();

    let mut files_found = 0u64;
    let mut total_bytes = 0u64;
    let mut total_lines = 0u64;
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                stack.push(entry.path());
            } else if ft.is_file() {
                files_found += 1;
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    total_bytes += content.len() as u64;
                    total_lines += content.lines().count() as u64;
                }
            }
        }
    }

    let discovery_duration = start.elapsed();

    ColdStartMetrics {
        files_found,
        total_bytes,
        total_lines,
        discovery_ms: discovery_duration.as_secs_f64() * 1000.0,
    }
}

#[derive(Debug, Clone)]
struct ColdStartMetrics {
    #[cfg_attr(not(test), allow(dead_code))]
    files_found: u64,
    total_bytes: u64,
    total_lines: u64,
    discovery_ms: f64,
}

/// Run the cold start scaling scenario.
pub fn run(config: &ColdStartConfig) -> ScenarioResult {
    let mem = MemoryGuard::start();
    let scenario_start = Instant::now();

    let mut result = ScenarioResult::new("cold_start_scaling");

    for spec in &config.steps {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let repo = generate_repo(spec, dir.path()).expect("failed to generate repo");

        let verified_count = count_files(repo.root());
        let metrics = simulate_cold_start(repo.root());

        let prefix = format!("files_{}", spec.file_count);
        result.add_metric(
            &format!("{prefix}_actual_count"),
            verified_count as f64,
            "count",
        );
        result.add_metric(
            &format!("{prefix}_discovery_ms"),
            metrics.discovery_ms,
            "ms",
        );
        result.add_metric(
            &format!("{prefix}_total_bytes"),
            metrics.total_bytes as f64,
            "bytes",
        );
        result.add_metric(
            &format!("{prefix}_total_lines"),
            metrics.total_lines as f64,
            "count",
        );

        if verified_count > 0 {
            result.add_metric(
                &format!("{prefix}_ms_per_file"),
                metrics.discovery_ms / verified_count as f64,
                "ms",
            );
        }
    }

    let mem_delta = mem.finish();
    result.set_duration(scenario_start.elapsed());
    result.add_memory("cold_start", &mem_delta);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_start_reads_all_files() {
        let dir = tempfile::tempdir().unwrap();
        let spec = RepoSpec {
            file_count: 15,
            max_depth: 2,
            lines_per_file: 10,
            ..RepoSpec::default()
        };
        let repo = generate_repo(&spec, dir.path()).unwrap();
        let metrics = simulate_cold_start(repo.root());
        assert_eq!(metrics.files_found, 15);
        assert!(metrics.total_bytes > 0);
        assert!(metrics.total_lines > 0);
    }

    #[test]
    fn scenario_produces_scaling_report() {
        let config = ColdStartConfig {
            steps: vec![
                RepoSpec {
                    file_count: 10,
                    max_depth: 1,
                    lines_per_file: 5,
                    ..RepoSpec::default()
                },
                RepoSpec {
                    file_count: 30,
                    max_depth: 2,
                    lines_per_file: 5,
                    ..RepoSpec::default()
                },
            ],
        };

        let result = run(&config);
        assert_eq!(result.scenario, "cold_start_scaling");
        assert!(
            result
                .metrics
                .iter()
                .any(|m| m.name == "files_10_discovery_ms")
        );
        assert!(
            result
                .metrics
                .iter()
                .any(|m| m.name == "files_30_discovery_ms")
        );
    }
}
