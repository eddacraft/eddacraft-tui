use std::{
    io::{BufRead, BufReader},
    path::Path,
};

use anvil_witness::{WitnessLine, witness_paths};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use super::{format_utc, truncate_to_seconds};

pub const INSIGHTS_SCHEMA_VERSION: &str = "anvil.insights.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WeeklyInsights {
    pub schema_version: &'static str,
    pub window_start: String,
    pub window_end: String,
    pub witness_events_observed: u64,
    pub total_saves_observed: u64,
    pub findings_raised: u64,
    pub suppressions_applied: u64,
    pub suppressions_resolved: u64,
    pub baseline_edges_added: u64,
    /// Daemon uptime over the window, as a percentage.
    ///
    /// **Schema-locked placeholder:** this is always `0` until the
    /// lifecycle telemetry that backs it is instrumented. The JSON
    /// contract (`schemas/anvil-insights.v1.json`) pins it as a required
    /// integer, so the wire value must stay `0` — do not change the type.
    /// The human surface (`commands::insights::print_plain`) renders the
    /// `0` placeholder as "not yet measured" rather than a misleading
    /// `0%` that reads as "the daemon was down all week" (ACTMO-011; beta
    /// smoke feedback). When real instrumentation lands, update both the
    /// aggregation here and the human-render special-case together.
    pub daemon_uptime_percentage: u8,
}

pub fn weekly_summary(repo_root: &Path, now: DateTime<Utc>) -> anyhow::Result<WeeklyInsights> {
    let now = truncate_to_seconds(now);
    let window_start = now - Duration::days(7);
    let mut witness_events_observed = 0_u64;

    for path in witness_paths(repo_root) {
        let file = std::fs::File::open(&path)?;
        for raw in BufReader::new(file).lines() {
            let raw = raw?;
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(witness) = WitnessLine::from_ndjson_line(line.as_bytes()) else {
                continue;
            };
            let Ok(ts) = DateTime::parse_from_rfc3339(&witness.ts) else {
                continue;
            };
            let ts = ts.with_timezone(&Utc);
            if ts >= window_start && ts <= now {
                witness_events_observed += 1;
            }
        }
    }

    Ok(WeeklyInsights {
        schema_version: INSIGHTS_SCHEMA_VERSION,
        window_start: format_utc(window_start),
        window_end: format_utc(now),
        witness_events_observed,
        total_saves_observed: 0,
        findings_raised: 0,
        suppressions_applied: 0,
        suppressions_resolved: 0,
        baseline_edges_added: 0,
        // Schema-locked placeholder — see field docs. Stays `0` on the
        // wire; the human surface renders it as "not yet measured".
        daemon_uptime_percentage: 0,
    })
}
