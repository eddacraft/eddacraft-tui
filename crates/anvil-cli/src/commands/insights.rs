use chrono::Utc;
use clap::Args;

use crate::{
    GlobalArgs,
    insights::{aggregator, drift_trend, suppressions},
    util,
};

#[derive(Debug, Args)]
pub struct InsightsArgs {
    /// Show the suppression health view: every active `@anvil-ignore`
    /// suppression with stale ones (underlying violation gone) first.
    #[arg(long)]
    pub suppressions: bool,

    /// Show the drift trend: new cross-boundary edges per week over the
    /// last 8 weeks, as a sparkline derived from `anvil drift` snapshots.
    #[arg(long, conflicts_with = "suppressions")]
    pub drift: bool,
}

pub fn run(args: &InsightsArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let root = util::workspace_root()?;

    if args.suppressions {
        let health = suppressions::suppression_health(&root);
        if global.json {
            println!("{}", serde_json::to_string_pretty(&health)?);
        } else {
            print_suppressions(&health);
        }
        return Ok(());
    }

    if args.drift {
        let trend = drift_trend::drift_trend(&root, Utc::now())?;
        if global.json {
            println!("{}", serde_json::to_string_pretty(&trend)?);
        } else {
            print!("{}", drift_trend::render_drift_trend(&trend));
        }
        return Ok(());
    }

    let now = Utc::now();
    let summary = aggregator::weekly_summary(&root, now)?;
    // INSIGHTS-004: record the view so first-week nudges are suppressed
    // for the remainder of the week (even for --json consumers).
    crate::insights::first_week_hint::record_insights_viewed(&root, now);
    if global.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_plain(&summary);
    }
    Ok(())
}

fn print_suppressions(health: &suppressions::SuppressionHealth) {
    println!("anvil suppression health");
    println!(
        "{} @anvil-ignore directive(s): {} active, {} stale (underlying violation gone)",
        health.total, health.active, health.stale
    );
    if health.entries.is_empty() {
        println!("No active suppressions found.");
        return;
    }
    for entry in &health.entries {
        let marker = if entry.stale { "STALE" } else { " ok  " };
        let date = entry.date.as_deref().unwrap_or("—");
        println!(
            "[{}] {}:{}  {}  ({})  {}",
            marker, entry.file, entry.line, entry.rule, date, entry.reason
        );
    }
    if health.stale > 0 {
        println!("\nRemove stale suppressions — their underlying violation no longer fires.");
    }
}

fn print_plain(summary: &aggregator::WeeklyInsights) {
    println!("anvil insights (last 7 days)");
    println!("Window: {} to {}", summary.window_start, summary.window_end);
    println!(
        "Witness events observed: {}",
        summary.witness_events_observed
    );
    println!("Saves observed: {}", summary.total_saves_observed);
    println!("Findings raised: {}", summary.findings_raised);
    println!("Suppressions applied: {}", summary.suppressions_applied);
    println!("Suppressions resolved: {}", summary.suppressions_resolved);
    println!("Baseline edges added: {}", summary.baseline_edges_added);
    println!("{}", uptime_line(summary.daemon_uptime_percentage));
}

/// Human-facing daemon-uptime line.
///
/// ACTMO-011: `daemon_uptime_percentage` is a schema-locked placeholder
/// that is always `0` until the metric is instrumented (see
/// [`aggregator::WeeklyInsights`]). Rendering "0%" reads as "the daemon
/// was down all week" and contradicts a daemon that is plainly running,
/// so show the placeholder honestly. A genuine 0% over a 7-day window is
/// not reachable today; when real instrumentation lands, drop this
/// special-case alongside the aggregation change.
fn uptime_line(pct: u8) -> String {
    if pct == 0 {
        "Daemon uptime: not yet measured".to_string()
    } else {
        format!("Daemon uptime: {pct}%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_witness::{GenesisAnchor, WitnessLine};
    use chrono::{TimeZone, Utc};
    use std::fs;
    use tempfile::TempDir;

    fn witness_line(seq: u64, ts: &str) -> WitnessLine {
        WitnessLine {
            seq,
            scope: "active".to_string(),
            kind: "witness".to_string(),
            prev_line_hash: GenesisAnchor::Fresh.anchor_string().to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
            commit_sha: Some(format!("{seq:040x}")),
            parent_commits: Vec::new(),
            prev_line_hashes: Vec::new(),
            agent_tag: None,
            rules_sha: None,
            cutoff_commit: None,
            ts: ts.to_string(),
            validation_at: "pre-commit".to_string(),
        }
    }

    fn write_line(path: &std::path::Path, line: &WitnessLine) {
        fs::write(path, line.to_ndjson_line().unwrap()).unwrap();
    }

    #[test]
    fn weekly_summary_matches_fixture() {
        let tmp = TempDir::new().unwrap();
        let witness_dir = tmp.path().join("anvil/witness");
        fs::create_dir_all(&witness_dir).unwrap();
        let active = witness_dir.join("active.ndjson");
        let lines = [
            witness_line(1, "2026-05-08T00:00:00Z"),
            witness_line(2, "2026-05-12T00:00:00Z"),
            witness_line(3, "2026-05-16T00:00:00Z"),
        ];
        let payload = lines
            .iter()
            .map(|line| String::from_utf8(line.to_ndjson_line().unwrap()).unwrap())
            .collect::<String>();
        fs::write(active, payload).unwrap();

        let now = Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).unwrap();
        let summary = aggregator::weekly_summary(tmp.path(), now).unwrap();

        assert_eq!(summary.schema_version, "anvil.insights.v1");
        assert_eq!(summary.window_start, "2026-05-10T00:00:00Z");
        assert_eq!(summary.window_end, "2026-05-17T00:00:00Z");
        assert_eq!(summary.witness_events_observed, 2);
        assert_eq!(summary.total_saves_observed, 0);
        assert_eq!(summary.findings_raised, 0);
        assert_eq!(summary.suppressions_applied, 0);
        assert_eq!(summary.suppressions_resolved, 0);
        assert_eq!(summary.baseline_edges_added, 0);
        // Schema-locked placeholder stays `0` on the wire (ACTMO-011 only
        // changes how the human surface renders it).
        assert_eq!(summary.daemon_uptime_percentage, 0);
    }

    #[test]
    fn uptime_line_renders_placeholder_as_not_yet_measured() {
        // ACTMO-011: the schema-locked `0` placeholder must read as "not
        // yet measured", never a misleading "0%".
        assert_eq!(uptime_line(0), "Daemon uptime: not yet measured");
        // A real (future) measurement renders as a percentage.
        assert_eq!(uptime_line(97), "Daemon uptime: 97%");
    }

    #[test]
    fn derives_from_witness_chain() {
        let tmp = TempDir::new().unwrap();
        let archive_dir = tmp.path().join("anvil/witness/archive");
        fs::create_dir_all(&archive_dir).unwrap();
        write_line(
            &archive_dir.join("active-00000000000000000001-old.ndjson"),
            &witness_line(1, "2026-05-14T00:00:00Z"),
        );
        fs::create_dir_all(tmp.path().join("anvil/witness")).unwrap();
        write_line(
            &tmp.path().join("anvil/witness/active.ndjson"),
            &witness_line(2, "2026-05-16T00:00:00Z"),
        );

        let now = Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).unwrap();
        let summary = aggregator::weekly_summary(tmp.path(), now).unwrap();

        assert_eq!(summary.witness_events_observed, 2);
    }

    // ── INSIGHTS-003: drift trend ───────────────────────────────────

    fn drift_snapshot(
        created_at: &str,
        edge_ids: &[&str],
    ) -> crate::commands::drift::DriftSnapshot {
        use crate::commands::drift::{DriftSnapshot, SnapshotMetrics, SnapshotViolation};
        let violations = edge_ids
            .iter()
            .enumerate()
            .map(|(i, id)| SnapshotViolation {
                id: (*id).to_string(),
                violation_type: "boundary".to_string(),
                from_file: format!("src/from_{i}.rs"),
                to_file: format!("src/to_{i}.rs"),
                from_layer: Some("app".to_string()),
                to_layer: Some("infra".to_string()),
                line: 1,
            })
            .collect();
        DriftSnapshot {
            schema_version: "1.0.0".to_string(),
            created_at: created_at.to_string(),
            name: None,
            metrics: SnapshotMetrics {
                boundary_violations: edge_ids.len(),
                antipattern_count: 0,
                suppression_count: 0,
                expired_suppressions: 0,
                files_analysed: 0,
            },
            antipattern_breakdown: None,
            violations,
            antipatterns: Vec::new(),
            suppressions: Vec::new(),
            sql_findings: Vec::new(),
            git_ref: None,
        }
    }

    fn write_drift_snapshot(dir: &std::path::Path, stem: &str, created_at: &str, edges: &[&str]) {
        let snap = drift_snapshot(created_at, edges);
        let path = dir.join(format!("snapshot-{stem}.json"));
        fs::write(path, serde_json::to_string_pretty(&snap).unwrap()).unwrap();
    }

    #[test]
    fn drift_trend_matches_fixture() {
        let tmp = TempDir::new().unwrap();
        let snaps = tmp.path().join(".anvil/snapshots");
        fs::create_dir_all(&snaps).unwrap();

        // now = 2026-05-29 → window_start = 2026-04-03. Week buckets:
        //   wk0 04-03, wk1 04-10, wk2 04-17, wk3 04-24,
        //   wk4 05-01, wk5 05-08, wk6 05-15, wk7 05-22.
        // Baseline {e1,e2} in wk0; +e3 in wk2; +e4,e5 in wk5 (e2 removed,
        // not counted); +e6..e9 in wk7.
        write_drift_snapshot(&snaps, "a", "2026-04-05T00:00:00Z", &["e1", "e2"]);
        write_drift_snapshot(&snaps, "b", "2026-04-20T00:00:00Z", &["e1", "e2", "e3"]);
        write_drift_snapshot(
            &snaps,
            "c",
            "2026-05-10T00:00:00Z",
            &["e1", "e3", "e4", "e5"],
        );
        write_drift_snapshot(
            &snaps,
            "d",
            "2026-05-26T00:00:00Z",
            &["e1", "e3", "e4", "e5", "e6", "e7", "e8", "e9"],
        );

        let now = Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap();
        let trend = drift_trend::drift_trend(tmp.path(), now).unwrap();

        assert_eq!(trend.schema_version, "anvil.drift_trend.v1");
        assert_eq!(trend.window_start, "2026-04-03T00:00:00Z");
        assert_eq!(trend.window_end, "2026-05-29T00:00:00Z");
        assert_eq!(trend.weeks.len(), 8);
        assert!(trend.sufficient_data);
        assert_eq!(trend.weeks_with_data, 4);

        // New edges per week.
        assert_eq!(trend.weeks[0].new_edges, 0); // baseline — no prior pair
        assert!(trend.weeks[0].has_data);
        assert_eq!(trend.weeks[2].new_edges, 1); // +e3
        assert_eq!(trend.weeks[5].new_edges, 2); // +e4,e5
        assert_eq!(trend.weeks[7].new_edges, 4); // +e6..e9

        // Weeks with no snapshot are no-data, not a measured zero.
        assert!(!trend.weeks[1].has_data);
        assert!(!trend.weeks[3].has_data);
        assert!(!trend.weeks[4].has_data);
        assert!(!trend.weeks[6].has_data);

        // Sparkline: max measured = 4 → bar indices 0/1/3/7, gaps between.
        assert_eq!(trend.sparkline, "▁·▂··▄·█");

        let rendered = drift_trend::render_drift_trend(&trend);
        assert!(rendered.contains("▁·▂··▄·█"));
        assert!(rendered.contains("no snapshot"));
    }

    #[test]
    fn insufficient_data_reports_clearly() {
        let tmp = TempDir::new().unwrap();
        let snaps = tmp.path().join(".anvil/snapshots");
        fs::create_dir_all(&snaps).unwrap();
        // Only one of the trailing 8 weeks has a snapshot → below the
        // 2-week threshold for a meaningful trend.
        write_drift_snapshot(&snaps, "only", "2026-05-26T00:00:00Z", &["e1", "e2"]);

        let now = Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap();
        let trend = drift_trend::drift_trend(tmp.path(), now).unwrap();

        assert!(!trend.sufficient_data);
        assert_eq!(trend.weeks_with_data, 1);
        assert!(trend.sparkline.is_empty());

        let rendered = drift_trend::render_drift_trend(&trend);
        assert!(
            rendered.contains("Not enough snapshot history"),
            "insufficient-data render must explain itself: {rendered}"
        );
        assert!(!rendered.contains('█'));
    }

    #[test]
    fn drift_trend_sums_introductions_within_one_week() {
        // Two snapshots land in the same trailing week. The metric counts
        // edge *introductions* per week: e3 is introduced in the first
        // intra-week pair, e4 in the second, and e5 is introduced then
        // resolved within the week — it still counts once (it was new
        // drift that week). Expected week total: 3, not a net delta of 2.
        let tmp = TempDir::new().unwrap();
        let snaps = tmp.path().join(".anvil/snapshots");
        fs::create_dir_all(&snaps).unwrap();

        // Pre-window-ish baseline two weeks earlier so the in-week pairs
        // are what drives the count (not the first-snapshot baseline rule).
        write_drift_snapshot(&snaps, "base", "2026-05-12T00:00:00Z", &["e1", "e2"]);
        // Same week (wk7: 2026-05-22..2026-05-29):
        write_drift_snapshot(
            &snaps,
            "w7a",
            "2026-05-23T00:00:00Z",
            &["e1", "e2", "e3", "e5"],
        );
        write_drift_snapshot(
            &snaps,
            "w7b",
            "2026-05-26T00:00:00Z",
            &["e1", "e2", "e3", "e4"],
        );

        let now = Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap();
        let trend = drift_trend::drift_trend(tmp.path(), now).unwrap();

        // base→w7a introduces e3,e5 (2); w7a→w7b introduces e4 (1, e5
        // dropped). Week 7 = 3 introductions.
        assert_eq!(trend.weeks[7].new_edges, 3);
        assert!(trend.weeks[7].has_data);
    }

    #[test]
    fn drift_trend_ignores_duplicate_violation_ids() {
        // A snapshot that carries the same violation id twice must not
        // inflate the new-edge count (set-based diff on both sides).
        let tmp = TempDir::new().unwrap();
        let snaps = tmp.path().join(".anvil/snapshots");
        fs::create_dir_all(&snaps).unwrap();
        write_drift_snapshot(&snaps, "base", "2026-05-12T00:00:00Z", &["e1"]);
        // e2 appears twice in the same snapshot; it is one new edge.
        write_drift_snapshot(&snaps, "dup", "2026-05-26T00:00:00Z", &["e1", "e2", "e2"]);

        let now = Utc.with_ymd_and_hms(2026, 5, 29, 0, 0, 0).unwrap();
        let trend = drift_trend::drift_trend(tmp.path(), now).unwrap();

        assert_eq!(trend.weeks[7].new_edges, 1);
    }
}
