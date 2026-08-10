//! `devacc-bench-1` run report schema.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "devacc-bench-1";

/// Single scenario × arm run record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DevaccReport {
    pub schema_version: String,
    pub anvil_sha: String,
    pub model: Option<String>,
    pub host_class: String,
    pub scenario: String,
    pub arm: String,
    pub tier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub task_success: bool,
    pub rubric_score: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub tokens_total: u64,
    pub tokens_tool_results: u64,
    pub tokens_file_reads: u64,
    pub tokens_gctx: u64,
    pub tool_calls: u64,
    pub file_reads: u64,
    pub gctx_calls: u64,
    pub validate_calls: u64,
    pub blocked_writes: u64,
    pub rework_cycles: u64,
    pub wall_ms: u64,
    pub turns: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub false_block_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimator: Option<String>,
}

impl DevaccReport {
    pub fn new_base(
        scenario: impl Into<String>,
        arm: impl Into<String>,
        tier: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            anvil_sha: resolve_anvil_sha(),
            model: None,
            host_class: default_host_class(),
            scenario: scenario.into(),
            arm: arm.into(),
            tier: tier.into(),
            label: None,
            task_success: false,
            rubric_score: 0.0,
            tokens_in: 0,
            tokens_out: 0,
            tokens_total: 0,
            tokens_tool_results: 0,
            tokens_file_reads: 0,
            tokens_gctx: 0,
            tool_calls: 0,
            file_reads: 0,
            gctx_calls: 0,
            validate_calls: 0,
            blocked_writes: 0,
            rework_cycles: 0,
            wall_ms: 0,
            turns: 0,
            false_block_rate: None,
            notes: None,
            estimator: Some(anvil_graph_cache::GCTX_TOKEN_ESTIMATOR_VERSION.to_string()),
        }
    }

    pub fn validate_shape(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "schema_version {} != {SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if !self.scenario.starts_with("DEVACC-SCN-") {
            return Err(format!("bad scenario id {}", self.scenario));
        }
        // When provider splits are present, they must sum to tokens_total.
        // Tier A often sets only tokens_total (tokens_in == tokens_total, tokens_out == 0)
        // or fills tokens_total without independent in/out — allow those.
        if self.tokens_in > 0 || self.tokens_out > 0 {
            let sum = self.tokens_in.saturating_add(self.tokens_out);
            if sum != self.tokens_total {
                return Err(format!(
                    "tokens_in ({}) + tokens_out ({}) != tokens_total ({})",
                    self.tokens_in, self.tokens_out, self.tokens_total
                ));
            }
        }
        Ok(())
    }
}

fn resolve_anvil_sha() -> String {
    std::env::var("ANVIL_DEVACC_SHA")
        .or_else(|_| std::env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "unknown".into())
}

fn default_host_class() -> String {
    std::env::var("ANVIL_DEVACC_HOST_CLASS").unwrap_or_else(|_| "local-dev".into())
}

/// Quality veto: do not include failed runs in token-win means.
pub fn token_reduction_vs_control(control: &DevaccReport, treatment: &DevaccReport) -> Option<f64> {
    if !control.task_success || !treatment.task_success {
        return None;
    }
    if control.tokens_total == 0 {
        return None;
    }
    Some(1.0 - (treatment.tokens_total as f64 / control.tokens_total as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devacc_report_schema_roundtrip() {
        let mut r = DevaccReport::new_base("DEVACC-SCN-01", "control", "A");
        r.task_success = true;
        r.tokens_total = 100;
        r.tokens_tool_results = 80;
        r.validate_shape().unwrap();
        let json = serde_json::to_string_pretty(&r).unwrap();
        let back: DevaccReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_version, SCHEMA_VERSION);
        assert_eq!(back.scenario, "DEVACC-SCN-01");
    }

    #[test]
    fn devacc_quality_veto_skips_failed() {
        let mut c = DevaccReport::new_base("DEVACC-SCN-01", "control", "A");
        let mut t = DevaccReport::new_base("DEVACC-SCN-01", "full-accel", "A");
        c.task_success = true;
        c.tokens_total = 200;
        t.task_success = false;
        t.tokens_total = 50;
        assert!(token_reduction_vs_control(&c, &t).is_none());
        t.task_success = true;
        let red = token_reduction_vs_control(&c, &t).unwrap();
        assert!((red - 0.75).abs() < 1e-9);
    }
}
