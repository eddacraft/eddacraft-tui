use std::path::{Path, PathBuf};

use chrono::Utc;
use clap::Args;
use serde::Serialize;

use crate::{
    GlobalArgs,
    insights::{
        aggregator,
        cumulative::{self, CumulativeValue},
        drift_trend, scorecard, suppressions,
    },
    util,
};

/// Schema version of the extended (weekly + cumulative) JSON document
/// emitted by `--cumulative --json`. The default `--json` output stays
/// on `anvil.insights.v1` so existing consumers are unaffected.
pub const INSIGHTS_SCHEMA_VERSION_V2: &str = "anvil.insights.v2";

/// Default filename the shareable scorecard is written to when
/// `--output` is not given.
const DEFAULT_SCORECARD_FILENAME: &str = "anvil-scorecard.html";

#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)] // CLI flags, intentional shape.
pub struct InsightsArgs {
    /// Show the suppression health view: every active `@anvil-ignore`
    /// suppression with stale ones (underlying violation gone) first.
    #[arg(long)]
    pub suppressions: bool,

    /// Show the drift trend: new cross-boundary edges per week over the
    /// last 8 weeks, as a sparkline derived from `anvil drift` snapshots.
    #[arg(long, conflicts_with = "suppressions")]
    pub drift: bool,

    /// Show only the cumulative value scoreboard: witness events since
    /// first run plus save-time protection counts over the retained
    /// evidence window. With `--json`, emits the extended v2 document.
    #[arg(long, conflicts_with_all = ["suppressions", "drift"])]
    pub cumulative: bool,

    /// Write a shareable, self-contained HTML scorecard of the headline
    /// numbers (counts and evidence window only — no repository details)
    /// and print the plain-text summary.
    #[arg(long, conflicts_with_all = ["suppressions", "drift", "cumulative"])]
    pub share: bool,

    /// Destination path for the shared scorecard (defaults to
    /// anvil-scorecard.html in the current directory).
    #[arg(long, requires = "share", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

/// The `anvil.insights.v2` wire document: every v1 rolling-window field
/// plus the cumulative aggregate. Kept as an explicit field list (not a
/// serde flatten of the v1 struct) so the v2 contract is visible in one
/// place and cannot drift silently when v1 changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InsightsV2 {
    pub schema_version: &'static str,
    pub window_start: String,
    pub window_end: String,
    pub witness_events_observed: u64,
    pub total_saves_observed: u64,
    pub findings_raised: u64,
    pub suppressions_applied: u64,
    pub suppressions_resolved: u64,
    pub baseline_edges_added: u64,
    pub daemon_uptime_percentage: u8,
    pub cumulative: CumulativeValue,
}

/// Compose the v2 document from the v1 weekly summary plus the
/// cumulative aggregate.
#[must_use]
pub fn insights_v2(weekly: aggregator::WeeklyInsights, cumulative: CumulativeValue) -> InsightsV2 {
    InsightsV2 {
        schema_version: INSIGHTS_SCHEMA_VERSION_V2,
        window_start: weekly.window_start,
        window_end: weekly.window_end,
        witness_events_observed: weekly.witness_events_observed,
        total_saves_observed: weekly.total_saves_observed,
        findings_raised: weekly.findings_raised,
        suppressions_applied: weekly.suppressions_applied,
        suppressions_resolved: weekly.suppressions_resolved,
        baseline_edges_added: weekly.baseline_edges_added,
        daemon_uptime_percentage: weekly.daemon_uptime_percentage,
        cumulative,
    }
}

/// Resolve the cumulative aggregate for `root` against the user-scoped
/// usage sidecar (the same path the DPO producers write).
fn cumulative_for_root(root: &Path) -> anyhow::Result<CumulativeValue> {
    let sidecar = crate::usage::default_usage_log_path()?;
    cumulative::cumulative_value(root, &sidecar)
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

    if args.share {
        anyhow::ensure!(
            !global.json,
            "--share writes an HTML scorecard and prints a plain summary; \
             it does not support --json (use --cumulative --json for the v2 document)"
        );
        let value = cumulative_for_root(&root)?;
        let Some(card) = scorecard::render_html_card(&value) else {
            // No evidence: say so honestly and write nothing — an
            // all-zero card would read as a measured claim.
            println!("{}", scorecard::NO_EVENTS_LINE);
            return Ok(());
        };
        let path = args
            .output
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SCORECARD_FILENAME));
        std::fs::write(&path, card)
            .map_err(|err| anyhow::anyhow!("write scorecard {}: {err}", path.display()))?;
        print!("{}", scorecard::render_plain(&value));
        println!("Scorecard written to {}", path.display());
        return Ok(());
    }

    if args.cumulative {
        let value = cumulative_for_root(&root)?;
        if global.json {
            let now = Utc::now();
            let weekly = aggregator::weekly_summary(&root, now)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&insights_v2(weekly, value))?
            );
        } else {
            print!("{}", scorecard::render_plain(&value));
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
        // Cumulative scoreboard: surfaced by default on the human view,
        // degrading gracefully (stderr note) if the user-scoped sidecar
        // cannot be resolved — the weekly summary must keep working.
        match cumulative_for_root(&root) {
            Ok(value) => {
                println!();
                print!("{}", scorecard::render_plain(&value));
            }
            Err(err) => eprintln!("Note: cumulative scoreboard unavailable ({err})"),
        }
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

    // ── CIB-073: cumulative value scoreboard + shareable scorecard ──

    use crate::insights::{cumulative, scorecard};
    use anvil_intercept::kindling_observation::{
        ConstraintAppliedObservation, Enforcement, KIND_CONSTRAINT_APPLIED, KIND_GATE_EVALUATED,
        ObservationInputs, Outcome,
    };

    fn save_time_row(
        ts: &str,
        outcome: Outcome,
        enforcement: Enforcement,
        rules_violated: &[&str],
        changed_files: &[&str],
    ) -> String {
        let obs = anvil_intercept::kindling_observation::GateEvaluatedObservation {
            kind: KIND_GATE_EVALUATED.to_string(),
            session_id: "0d6f3a52-8f0e-4a3f-9f3e-marker-sess".to_string(),
            timestamp: ts.to_string(),
            gate_eval_id: "marker-eval-id".to_string(),
            gate_id: "save-time".to_string(),
            inputs: ObservationInputs {
                file_count: u32::try_from(changed_files.len()).unwrap(),
                changed_files: changed_files.iter().map(|s| (*s).to_string()).collect(),
                baseline_hash: Some("marker-baseline-hash".to_string()),
            },
            outcome,
            rules_evaluated: rules_violated.iter().map(|s| (*s).to_string()).collect(),
            rules_violated: if rules_violated.is_empty() {
                None
            } else {
                Some(rules_violated.iter().map(|s| (*s).to_string()).collect())
            },
            enforcement,
            duration_ms: 12,
            violation_count: None,
            warning_count: None,
            partial: false,
        };
        serde_json::to_string(&obs).unwrap()
    }

    fn fence_row(ts: &str, worktree: &str) -> String {
        let obs = ConstraintAppliedObservation {
            kind: KIND_CONSTRAINT_APPLIED.to_string(),
            session_id: "0d6f3a52-8f0e-4a3f-9f3e-marker-sess".to_string(),
            timestamp: ts.to_string(),
            constraint_id: "daemon.fence".to_string(),
            gate_id: "daemon.fence".to_string(),
            worktree: worktree.to_string(),
            reason: "operator".to_string(),
            cascade: false,
        };
        serde_json::to_string(&obs).unwrap()
    }

    /// A repo + sidecar fixture whose every free-text source field is
    /// seeded with the `marker` string, so a single substring assertion
    /// proves the redaction contract on every rendered output.
    fn marker_fixture() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        let witness_dir = tmp.path().join("anvil/witness");
        fs::create_dir_all(&witness_dir).unwrap();
        let mut w1 = witness_line(1, "2026-01-05T08:00:00Z");
        w1.agent_tag = Some("marker-agent".to_string());
        let mut w2 = witness_line(2, "2026-06-20T09:30:00Z");
        w2.agent_tag = Some("marker-agent".to_string());
        let w3 = witness_line(3, "2026-07-01T10:00:00Z");
        let payload: String = [&w1, &w2, &w3]
            .iter()
            .map(|line| String::from_utf8(line.to_ndjson_line().unwrap()).unwrap())
            .collect();
        fs::write(witness_dir.join("active.ndjson"), payload).unwrap();

        let sidecar = tmp.path().join("kindling/usage.ndjson");
        fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        let lines = [
            // A clean save (pass) — counted as an evaluation only.
            save_time_row(
                "2026-07-05T10:00:00Z",
                Outcome::Pass,
                Enforcement::Informational,
                &[],
                &[],
            ),
            // A warning-level fail — flagged, not blocked.
            save_time_row(
                "2026-07-06T11:00:00Z",
                Outcome::Fail,
                Enforcement::Warning,
                &["path-deny"],
                &["/home/markeruser/marker-repo/src/marker_file.rs"],
            ),
            // A blocking fail with two secret findings.
            save_time_row(
                "2026-07-08T12:00:00Z",
                Outcome::Fail,
                Enforcement::Blocking,
                &["secret-detection", "path-deny", "secret-detection"],
                &["/home/markeruser/marker-repo/.env.marker-branch"],
            ),
            // A fence engagement.
            fence_row("2026-07-07T09:00:00Z", "/home/markeruser/marker-repo"),
            // A command.invoked row — ignored by the aggregate (and its
            // principal must never leak even if a raw email was recorded).
            r#"{"kind":"command.invoked","command":"marker-command","principal":"marker@example.com","timestamp":"2026-07-05T10:30:00Z","args":[],"flag_set":[]}"#.to_string(),
            // A mid-edit gate row that must NOT be scooped up.
            save_time_row(
                "2026-07-05T10:45:00Z",
                Outcome::Fail,
                Enforcement::Blocking,
                &["secret-detection"],
                &[],
            ).replace("\"save-time\"", "\"midEdit\""),
            // A row with no parseable timestamp — cannot back a claim.
            save_time_row(
                "not-a-timestamp",
                Outcome::Fail,
                Enforcement::Blocking,
                &["secret-detection"],
                &[],
            ),
            // Torn / malformed line.
            "{ this is not json".to_string(),
        ];
        fs::write(&sidecar, format!("{}\n", lines.join("\n"))).unwrap();
        (tmp, sidecar)
    }

    #[test]
    fn cumulative_value_aggregates_witness_and_sidecar() {
        let (tmp, sidecar) = marker_fixture();
        let value = cumulative::cumulative_value(tmp.path(), &sidecar).unwrap();

        // Evidence window bounds come from the data, never a wall clock.
        assert_eq!(value.since.as_deref(), Some("2026-01-05T08:00:00Z"));
        assert_eq!(value.as_of.as_deref(), Some("2026-07-08T12:00:00Z"));

        // Witness chain is genuinely cumulative; rolling windows anchor
        // at `as_of` (2026-07-08): 30d ⊇ {06-20? no, 07-01}, 90d ⊇
        // {06-20, 07-01}.
        assert_eq!(value.witness_events_total, 3);
        assert_eq!(value.witness_events_last_30_days, 2);
        assert_eq!(value.witness_events_last_90_days, 2);

        let save = &value.save_time;
        assert_eq!(save.window_start.as_deref(), Some("2026-07-05T10:00:00Z"));
        assert_eq!(save.window_end.as_deref(), Some("2026-07-08T12:00:00Z"));
        assert_eq!(save.evaluations_observed, 3, "pass + two fails");
        assert_eq!(save.risky_writes_flagged, 2);
        assert_eq!(save.writes_blocked, 1, "only the blocking fail");
        assert_eq!(save.secret_findings_caught, 2, "two secret-detection hits");
        assert_eq!(save.fences_engaged, 1);
    }

    #[test]
    fn cumulative_empty_sources_report_no_evidence() {
        let tmp = TempDir::new().unwrap();
        let sidecar = tmp.path().join("kindling/usage.ndjson"); // absent
        let value = cumulative::cumulative_value(tmp.path(), &sidecar).unwrap();

        assert_eq!(value.since, None);
        assert_eq!(value.as_of, None);
        assert_eq!(value.witness_events_total, 0);
        assert!(!value.save_time.has_evidence());

        // Honest empty state: stated, never zero-filled as measured.
        let plain = scorecard::render_plain(&value);
        assert!(plain.contains(scorecard::NO_EVENTS_LINE), "{plain}");
        assert!(!plain.contains(" 0 "), "no zero-filled claims: {plain}");

        // And nothing shareable: an all-zero card would read as a claim.
        assert!(scorecard::render_html_card(&value).is_none());
    }

    #[test]
    fn cumulative_save_time_only_evidence_still_reports() {
        // Sidecar rows but an empty witness chain (e.g. save-time-only
        // adoption): the scoreboard must render the save-time window
        // and an honest zero-witness line, not fail or fabricate.
        let tmp = TempDir::new().unwrap();
        let sidecar = tmp.path().join("kindling/usage.ndjson");
        fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        fs::write(
            &sidecar,
            format!(
                "{}\n",
                save_time_row(
                    "2026-07-05T10:00:00Z",
                    Outcome::Fail,
                    Enforcement::Blocking,
                    &["secret-detection"],
                    &[],
                )
            ),
        )
        .unwrap();

        let value = cumulative::cumulative_value(tmp.path(), &sidecar).unwrap();
        assert_eq!(value.since.as_deref(), Some("2026-07-05T10:00:00Z"));
        assert_eq!(value.witness_events_total, 0);
        assert_eq!(value.save_time.writes_blocked, 1);
        assert!(scorecard::render_html_card(&value).is_some());
    }

    #[test]
    fn insights_v2_extends_v1_with_cumulative_fields() {
        let (tmp, sidecar) = marker_fixture();
        let now = Utc.with_ymd_and_hms(2026, 7, 8, 12, 0, 0).unwrap();
        let weekly = aggregator::weekly_summary(tmp.path(), now).unwrap();
        // The v1 constant is untouched by the v2 introduction.
        assert_eq!(weekly.schema_version, "anvil.insights.v1");

        let value = cumulative::cumulative_value(tmp.path(), &sidecar).unwrap();
        let v2 = insights_v2(weekly.clone(), value);
        assert_eq!(v2.schema_version, "anvil.insights.v2");
        assert_eq!(v2.witness_events_observed, weekly.witness_events_observed);

        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&v2).unwrap()).unwrap();
        assert_eq!(json["schema_version"], "anvil.insights.v2");
        // Every v1 field survives at the top level.
        for key in [
            "window_start",
            "window_end",
            "witness_events_observed",
            "total_saves_observed",
            "findings_raised",
            "suppressions_applied",
            "suppressions_resolved",
            "baseline_edges_added",
            "daemon_uptime_percentage",
        ] {
            assert!(json.get(key).is_some(), "v1 field {key} missing from v2");
        }
        let cumulative = &json["cumulative"];
        assert_eq!(cumulative["witness_events_total"], 3);
        assert_eq!(cumulative["save_time"]["writes_blocked"], 1);
        assert_eq!(cumulative["since"], "2026-01-05T08:00:00Z");
    }

    /// The redaction gate for CIB-073: every free-text field in every
    /// source row is seeded with `marker` (paths, repo/file/branch
    /// names, session ids, principals/emails, agent tags, commands),
    /// and no rendered surface — plain scoreboard, HTML card, v2 JSON —
    /// may contain it.
    #[test]
    fn redaction_no_marker_survives_into_any_rendered_output() {
        let (tmp, sidecar) = marker_fixture();
        let value = cumulative::cumulative_value(tmp.path(), &sidecar).unwrap();

        let plain = scorecard::render_plain(&value);
        let card = scorecard::render_html_card(&value).expect("evidence present");
        let now = Utc.with_ymd_and_hms(2026, 7, 8, 12, 0, 0).unwrap();
        let weekly = aggregator::weekly_summary(tmp.path(), now).unwrap();
        let v2_json = serde_json::to_string_pretty(&insights_v2(weekly, value)).unwrap();

        for (name, output) in [("plain", &plain), ("card", &card), ("v2 json", &v2_json)] {
            let lower = output.to_lowercase();
            assert!(
                !lower.contains("marker"),
                "{name} output leaked a seeded source string:\n{output}"
            );
            // Belt-and-braces on the concrete leak classes.
            for leak in ["/home/", ".rs", ".env", "@example.com", "secret-detection"] {
                assert!(!lower.contains(leak), "{name} output leaked {leak:?}");
            }
        }
    }

    #[test]
    fn scorecard_html_is_self_contained_and_deterministic() {
        let (tmp, sidecar) = marker_fixture();
        let value = cumulative::cumulative_value(tmp.path(), &sidecar).unwrap();
        let card = scorecard::render_html_card(&value).expect("evidence present");

        // Deterministic: a second render from the same aggregate is
        // byte-identical (no generation timestamps anywhere).
        let again = scorecard::render_html_card(
            &cumulative::cumulative_value(tmp.path(), &sidecar).unwrap(),
        )
        .unwrap();
        assert_eq!(card, again);

        // Self-contained: embedded styling only, no scripts, no network
        // references of any kind.
        assert!(card.contains("<style>"));
        for forbidden in ["<script", "http://", "https://", "src=", "@import", "url("] {
            assert!(
                !card.contains(forbidden),
                "card must not contain {forbidden:?}"
            );
        }

        // The evidence window's own bounds name the window; the headline
        // counts are present.
        assert!(card.contains("2026-01-05 to 2026-07-08"));
        assert!(card.contains("retained window 2026-07-05 to 2026-07-08"));
        assert!(card.contains("witness events since first run"));
        assert!(card.contains("writes blocked"));
    }

    #[test]
    fn plain_scoreboard_names_its_evidence_windows() {
        let (tmp, sidecar) = marker_fixture();
        let value = cumulative::cumulative_value(tmp.path(), &sidecar).unwrap();
        let plain = scorecard::render_plain(&value);
        assert!(
            plain.contains("Evidence window: 2026-01-05 to 2026-07-08"),
            "{plain}"
        );
        assert!(
            plain.contains("retained window 2026-07-05 to 2026-07-08"),
            "the save-time counts must name their bounded window: {plain}"
        );
        assert!(plain.contains("since first run"));
        assert!(plain.contains("Writes blocked: 1"), "{plain}");
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
