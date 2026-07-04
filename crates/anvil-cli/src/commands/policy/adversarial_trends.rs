//! ATC-004 — `anvil policy probe-trends`: adversarial trend reporting.
//!
//! Surfaces adversarial probe pass/fail trends **by category over time**, read
//! from the same eval-regression history the policy suites persist to. Because
//! probe runs are folded into the eval store as `probe:<category>` suites
//! (ATC-003) using the unchanged eval record schema, this command needs no
//! bespoke storage: it filters the store history to probe suites, groups by
//! category, and reports each category's run history so teams can spot recurring
//! weak points and regressions.
//!
//! The trend computation is a pure library function ([`category_trends`]) over
//! already-loaded records, so the reporting semantics are directly testable
//! without touching the filesystem; the command is a thin store-read wrapper.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use anvil_policy::adversarial::category_from_suite;
use anvil_policy::eval::{EvalRecord, EvalResultStore};

use crate::GlobalArgs;
use crate::install_root;
use crate::output;

#[derive(Debug, Args)]
pub struct ProbeTrendsArgs {
    /// Directory holding the eval history. Defaults to `<ANVIL_HOME>/eval`,
    /// falling back to `.anvil/eval` when no Anvil home is resolvable.
    #[arg(long, value_name = "DIR")]
    store: Option<PathBuf>,
}

/// One recorded probe-category run in the trend series.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TrendPoint {
    /// The run id the record was stored under.
    run_id: String,
    /// RFC 3339 timestamp the run was recorded at.
    recorded_at: String,
    /// Whether the category's run passed (gate exit 0).
    passed: bool,
    /// Count of blocking probe failures in that run.
    error_count: usize,
}

/// The pass/fail history for one probe category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct CategoryTrend {
    /// The probe category (kebab-case wire label).
    category: String,
    /// Total recorded runs for this category.
    total_runs: usize,
    /// How many of those runs failed the gate.
    failed_runs: usize,
    /// Whether the most recent run passed — the category's current health.
    currently_passing: bool,
    /// Every run for this category, in chronological order.
    points: Vec<TrendPoint>,
}

/// Compute per-category adversarial trends from eval history records.
///
/// Only probe suites (`probe:<category>`) are considered; policy suites are
/// ignored. Records are assumed to be in chronological (store append) order,
/// which is preserved within each category series. Categories are returned
/// sorted by label for deterministic output. Pure — no IO — so the reporting
/// semantics are directly testable.
fn category_trends(records: &[EvalRecord]) -> Vec<CategoryTrend> {
    let mut by_category: BTreeMap<String, Vec<TrendPoint>> = BTreeMap::new();
    for record in records {
        let Some(category) = category_from_suite(&record.suite) else {
            continue;
        };
        by_category
            .entry(category.to_string())
            .or_default()
            .push(TrendPoint {
                run_id: record.run_id.clone(),
                recorded_at: record.recorded_at.clone(),
                passed: record.passed,
                error_count: record.error_count,
            });
    }

    by_category
        .into_iter()
        .map(|(category, points)| {
            let failed_runs = points.iter().filter(|p| !p.passed).count();
            let currently_passing = points.last().is_some_and(|p| p.passed);
            CategoryTrend {
                category,
                total_runs: points.len(),
                failed_runs,
                currently_passing,
                points,
            }
        })
        .collect()
}

/// Resolve the eval history directory: explicit `--store`, else
/// `<ANVIL_HOME>/eval`, else `.anvil/eval`.
fn resolve_store_dir(explicit: Option<&PathBuf>) -> PathBuf {
    if let Some(dir) = explicit {
        return dir.clone();
    }
    install_root::install_root()
        .user_dir()
        .map_or_else(|| PathBuf::from(".anvil").join("eval"), |d| d.join("eval"))
}

pub fn run(args: &ProbeTrendsArgs, global: &GlobalArgs) -> Result<()> {
    let store = EvalResultStore::new(resolve_store_dir(args.store.as_ref()));
    let records = store.all().context("reading eval history")?;
    let trends = category_trends(&records);

    if global.json {
        output::json::print(&trends)?;
    } else {
        render_plain(&trends);
    }
    Ok(())
}

fn render_plain(trends: &[CategoryTrend]) {
    use crate::output::plain;

    plain::blank();
    plain::section("Adversarial probe trends");
    if trends.is_empty() {
        plain::info("  no probe runs recorded yet");
        plain::blank();
        return;
    }
    for trend in trends {
        // ✓ currently passing, ✗ currently failing.
        let icon = if trend.currently_passing {
            "\u{2713}"
        } else {
            "\u{2717}"
        };
        println!(
            "  {icon} {category:<24} runs:{total} failed:{failed}",
            category = trend.category,
            total = trend.total_runs,
            failed = trend.failed_runs,
        );
    }
    plain::blank();
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_policy::eval::{EvalFinding, EvalRunSummary, EvalSeverity};

    /// Build a probe-category record for `category` at `recorded_at`, failing
    /// when `errors > 0`.
    fn probe_record(category: &str, run_id: &str, recorded_at: &str, errors: usize) -> EvalRecord {
        let findings: Vec<EvalFinding> = (0..errors)
            .map(|i| EvalFinding {
                severity: EvalSeverity::Error,
                message: format!("probe-{i}"),
                from: None,
                to: None,
                fingerprint: Some(format!("probe-{i}")),
            })
            .collect();
        let summary = EvalRunSummary {
            suite: format!("probe:{category}"),
            schema_version: "1.0.0".into(),
            policy: "baseline".into(),
            query: "adversarial.probes".into(),
            findings,
            exit_code: i32::from(errors > 0),
        };
        EvalRecord::from_summary(&summary, run_id, recorded_at)
    }

    fn policy_record(suite: &str) -> EvalRecord {
        let summary = EvalRunSummary {
            suite: suite.into(),
            schema_version: "1.0.0".into(),
            policy: "p".into(),
            query: "q".into(),
            findings: vec![],
            exit_code: 0,
        };
        EvalRecord::from_summary(&summary, "r", "2026-07-04T00:00:00Z")
    }

    #[test]
    fn adversarial_trends_group_by_category_over_time() {
        let records = [
            probe_record("prompt-injection", "r1", "2026-07-01T00:00:00Z", 0),
            probe_record("prompt-injection", "r2", "2026-07-02T00:00:00Z", 2),
            probe_record("data-exfiltration", "r3", "2026-07-02T00:00:00Z", 0),
        ];
        let trends = category_trends(&records);
        assert_eq!(trends.len(), 2);
        // Deterministic, sorted by category label.
        assert_eq!(trends[0].category, "data-exfiltration");
        assert_eq!(trends[1].category, "prompt-injection");

        let injection = &trends[1];
        assert_eq!(injection.total_runs, 2);
        assert_eq!(injection.failed_runs, 1);
        // The latest run failed, so the category is currently failing.
        assert!(!injection.currently_passing);
        // Chronological order preserved.
        assert_eq!(injection.points[0].run_id, "r1");
        assert_eq!(injection.points[1].run_id, "r2");
        assert_eq!(injection.points[1].error_count, 2);
    }

    #[test]
    fn adversarial_trends_ignore_policy_suites() {
        // Only probe suites contribute; ordinary policy suites are excluded so
        // the trend view is adversarial-only.
        let records = [
            policy_record("arch_boundary"),
            probe_record("boundary-evasion", "r1", "2026-07-01T00:00:00Z", 0),
        ];
        let trends = category_trends(&records);
        assert_eq!(trends.len(), 1);
        assert_eq!(trends[0].category, "boundary-evasion");
        assert!(trends[0].currently_passing);
    }

    #[test]
    fn adversarial_trends_empty_history_is_empty() {
        assert!(category_trends(&[]).is_empty());
    }

    #[test]
    fn adversarial_trends_recovery_flips_current_health() {
        // A category that failed then passed reports currently passing but keeps
        // the failure in its history, so a recurring weak point stays visible.
        let records = [
            probe_record("unsafe-tool-invocation", "r1", "2026-07-01T00:00:00Z", 1),
            probe_record("unsafe-tool-invocation", "r2", "2026-07-02T00:00:00Z", 0),
        ];
        let trends = category_trends(&records);
        assert_eq!(trends.len(), 1);
        let trend = &trends[0];
        assert!(trend.currently_passing);
        assert_eq!(trend.failed_runs, 1);
        assert_eq!(trend.total_runs, 2);
    }

    #[test]
    fn adversarial_trends_serialises_for_json_output() {
        let records = [probe_record(
            "prompt-injection",
            "r1",
            "2026-07-01T00:00:00Z",
            0,
        )];
        let trends = category_trends(&records);
        let json: serde_json::Value = serde_json::to_value(&trends).expect("serialise");
        assert_eq!(json[0]["category"], "prompt-injection");
        assert_eq!(json[0]["total_runs"], 1);
        assert_eq!(json[0]["currently_passing"], true);
        assert_eq!(json[0]["points"][0]["run_id"], "r1");
    }
}
