use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::measure::{MemoryDelta, TimingResult};

/// A single metric data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

/// Result of running a single benchmark scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario: String,
    pub generated_at_epoch: u64,
    pub duration_secs: f64,
    pub metrics: Vec<Metric>,
}

impl ScenarioResult {
    #[must_use]
    pub fn new(scenario: &str) -> Self {
        Self {
            scenario: scenario.to_string(),
            generated_at_epoch: epoch_secs(),
            duration_secs: 0.0,
            metrics: Vec::new(),
        }
    }

    pub fn set_duration(&mut self, d: Duration) {
        self.duration_secs = d.as_secs_f64();
    }

    pub fn add_metric(&mut self, name: &str, value: f64, unit: &str) {
        self.metrics.push(Metric {
            name: name.to_string(),
            value,
            unit: unit.to_string(),
        });
    }

    pub fn add_timing(&mut self, timing: &TimingResult) {
        self.add_metric(
            &format!("{}_total_ms", timing.label),
            timing.duration.as_secs_f64() * 1000.0,
            "ms",
        );
        self.add_metric(
            &format!("{}_iterations", timing.label),
            timing.iterations as f64,
            "count",
        );
        if timing.iterations > 0 {
            self.add_metric(
                &format!("{}_per_iter_us", timing.label),
                timing.per_iteration().as_secs_f64() * 1_000_000.0,
                "us",
            );
        }
    }

    pub fn add_memory(&mut self, label: &str, delta: &MemoryDelta) {
        self.add_metric(
            &format!("{label}_rss_before_mib"),
            delta.before.rss_mib(),
            "MiB",
        );
        self.add_metric(
            &format!("{label}_rss_after_mib"),
            delta.after.rss_mib(),
            "MiB",
        );
        self.add_metric(
            &format!("{label}_rss_delta_mib"),
            delta.delta_rss_mib(),
            "MiB",
        );
    }
}

/// Full benchmark report containing multiple scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchReport {
    pub suite: String,
    pub generated_at_epoch: u64,
    pub results: Vec<ScenarioResult>,
}

impl BenchReport {
    #[must_use]
    pub fn new(suite: &str) -> Self {
        Self {
            suite: suite.to_string(),
            generated_at_epoch: epoch_secs(),
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: ScenarioResult) {
        self.results.push(result);
    }

    /// Serialise to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Write the report to a file.
    pub fn write_to(&self, path: &Path) -> std::io::Result<()> {
        let json = self
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_result_serialises_to_json() {
        let mut result = ScenarioResult::new("test_scenario");
        result.set_duration(Duration::from_millis(1234));
        result.add_metric("files_processed", 100.0, "count");

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test_scenario"));
        assert!(json.contains("files_processed"));
    }

    #[test]
    fn report_roundtrips_through_json() {
        let mut report = BenchReport::new("stress-tests");
        let mut result = ScenarioResult::new("graph_memory");
        result.add_metric("peak_rss_mib", 256.5, "MiB");
        report.add_result(result);

        let json = report.to_json().unwrap();
        let parsed: BenchReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.suite, "stress-tests");
        assert_eq!(parsed.results.len(), 1);
        assert_eq!(parsed.results[0].metrics[0].name, "peak_rss_mib");
    }

    #[test]
    fn write_to_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.json");

        let report = BenchReport::new("test");
        report.write_to(&path).unwrap();

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"suite\": \"test\""));
    }

    #[test]
    fn add_timing_populates_metrics() {
        let timing = TimingResult {
            label: "parse".to_string(),
            duration: Duration::from_millis(500),
            iterations: 100,
        };

        let mut result = ScenarioResult::new("test");
        result.add_timing(&timing);

        assert_eq!(result.metrics.len(), 3);
        assert!(result.metrics.iter().any(|m| m.name == "parse_total_ms"));
        assert!(result.metrics.iter().any(|m| m.name == "parse_iterations"));
        assert!(result
            .metrics
            .iter()
            .any(|m| m.name == "parse_per_iter_us"));
    }
}
