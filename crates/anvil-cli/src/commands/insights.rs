use chrono::Utc;
use clap::Args;

use crate::{
    GlobalArgs,
    insights::{aggregator, suppressions},
    util,
};

#[derive(Debug, Args)]
pub struct InsightsArgs {
    /// Show the suppression health view: every active `@anvil-ignore`
    /// suppression with stale ones (underlying violation gone) first.
    /// [INSIGHTS-002]
    #[arg(long)]
    pub suppressions: bool,
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

    let summary = aggregator::weekly_summary(&root, Utc::now())?;
    if global.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_plain(&summary);
    }
    Ok(())
}

fn print_suppressions(health: &suppressions::SuppressionHealth) {
    println!("Anvil suppression health");
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
    println!("Anvil insights (last 7 days)");
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
    println!("Daemon uptime: {}%", summary.daemon_uptime_percentage);
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
        assert_eq!(summary.daemon_uptime_percentage, 0);
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
}
