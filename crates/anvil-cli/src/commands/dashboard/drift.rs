//! `anvil dashboard drift` — native drift-snapshots dashboard (TDASH-003).
//!
//! Reads the snapshot history under `.anvil/snapshots/` via the `drift`
//! command's existing readers and the architecture baseline
//! (`.anvil/architecture.json`, the same baseline `anvil drift snapshot` scores
//! against). `--json` emits a structured snapshot, non-TTY prints a plain
//! summary, TTY runs the Ratatui surface. No snapshots is a legitimate empty
//! state, not an error.
//!
//! Note: the TDASH-003 item names `.anvil/baseline.json`; in the live tree the
//! drift baseline that snapshots are scored against is the architecture
//! baseline at `.anvil/architecture.json` (the per-finding fingerprint store at
//! `.anvil/baseline.json` is a separate MLP-007 concern). New-vs-baseline here
//! compares the latest snapshot's boundary violations against that architecture
//! baseline, matching how the snapshot's violations were captured.

use std::collections::BTreeSet;
use std::io::IsTerminal;

use anvil_architecture::load_baseline;
use anvil_architecture::types::ArchitectureBaseline;
use anvil_tui::surfaces::dashboard::drift::{
    DriftDashboardState, DriftDelta, DriftSnapshotRow, DriftView,
};
use serde::Serialize;

use crate::commands::drift::{DriftSnapshot, list_snapshot_files, load_snapshot_file};
use crate::{GlobalArgs, tui, util};

/// Serializable per-snapshot record for `--json`.
#[derive(Debug, Serialize)]
struct SnapshotRecord {
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    boundary_violations: usize,
    antipattern_count: usize,
    suppression_count: usize,
    files_analysed: usize,
}

/// Serializable delta between the two most recent snapshots for `--json`.
#[derive(Debug, Serialize)]
struct DeltaRecord {
    before_created_at: String,
    after_created_at: String,
    violations_added: usize,
    violations_removed: usize,
    antipatterns_added: usize,
    antipatterns_removed: usize,
    net_violations: i64,
    net_antipatterns: i64,
    trend: &'static str,
}

/// Stable `--json` envelope. `snapshots_present` lets consumers distinguish
/// "no snapshots" from a present-but-empty history without the top-level value
/// flipping shape. `baseline_present` separates "no baseline" from "zero new".
#[derive(Debug, Serialize)]
struct DriftJson {
    snapshots_present: bool,
    snapshot_count: usize,
    baseline_present: bool,
    new_vs_baseline: Option<usize>,
    latest_delta: Option<DeltaRecord>,
    snapshots: Vec<SnapshotRecord>,
}

/// Run the drift dashboard. Returns how the surface exited so the picker can
/// return to itself on [`SurfaceExit::Back`]. Non-interactive branches
/// (`--json`, no-TTY) print and report `Quit`.
pub fn run(global: &GlobalArgs) -> anyhow::Result<tui::SurfaceExit> {
    let root = util::workspace_root()?;
    let snapshots = load_all_snapshots(&root)?;
    // Propagate load errors (corrupt/unreadable baseline); `Ok(None)` is the
    // legitimate no-baseline state, which renders new-vs-baseline as unknown.
    let baseline = load_baseline(&root)?;

    if global.json {
        let payload = build_json(&snapshots, baseline.as_ref());
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(tui::SurfaceExit::Quit);
    }

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        print_summary(&snapshots, baseline.as_ref());
        return Ok(tui::SurfaceExit::Quit);
    }

    let view = build_view(&snapshots, baseline.as_ref());
    let (_, exit) = tui::run_surface_with_exit(DriftDashboardState::new(view))?;
    Ok(exit)
}

/// Load every snapshot newest-first, skipping (with a warning) any that fail to
/// parse — mirrors the `drift list` corrupt-file behaviour.
fn load_all_snapshots(root: &std::path::Path) -> anyhow::Result<Vec<DriftSnapshot>> {
    let mut snapshots = Vec::new();
    for path in list_snapshot_files(root)? {
        match load_snapshot_file(&path) {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(e) => eprintln!("warning: skipping corrupt snapshot {}: {e}", path.display()),
        }
    }
    Ok(snapshots)
}

/// Count boundary violations in the latest snapshot whose fingerprint is not in
/// the architecture baseline. `None` when there is no baseline to compare to.
fn new_vs_baseline(
    latest: &DriftSnapshot,
    baseline: Option<&ArchitectureBaseline>,
) -> Option<usize> {
    let baseline = baseline?;
    let baseline_ids: BTreeSet<&str> = baseline
        .baseline_snapshot
        .violations
        .iter()
        .map(|v| v.id.as_str())
        .collect();
    let count = latest
        .violations
        .iter()
        .filter(|v| !baseline_ids.contains(v.id.as_str()))
        .count();
    Some(count)
}

/// Compute the delta between two snapshots (added/removed by ID-set diff, net by
/// metric difference). Antipattern identity mirrors `drift compare`'s
/// `file:line:id` key.
fn compute_delta(before: &DriftSnapshot, after: &DriftSnapshot) -> DriftDelta {
    let before_viol: BTreeSet<&str> = before.violations.iter().map(|v| v.id.as_str()).collect();
    let after_viol: BTreeSet<&str> = after.violations.iter().map(|v| v.id.as_str()).collect();

    let before_ap = ap_keys(before);
    let after_ap = ap_keys(after);

    DriftDelta {
        before_created_at: before.created_at.clone(),
        after_created_at: after.created_at.clone(),
        violations_added: after_viol.difference(&before_viol).count(),
        violations_removed: before_viol.difference(&after_viol).count(),
        antipatterns_added: after_ap.difference(&before_ap).count(),
        antipatterns_removed: before_ap.difference(&after_ap).count(),
        net_violations: net(
            after.metrics.boundary_violations,
            before.metrics.boundary_violations,
        ),
        net_antipatterns: net(
            after.metrics.antipattern_count,
            before.metrics.antipattern_count,
        ),
    }
}

/// Borrowed `file:line:id` antipattern identity — same key as `drift compare`'s
/// string form, without allocating a String per antipattern. A free fn (not a
/// closure) so lifetime elision ties the borrowed keys to the snapshot.
fn ap_keys(snapshot: &DriftSnapshot) -> BTreeSet<(&str, usize, &str)> {
    snapshot
        .antipatterns
        .iter()
        .map(|a| (a.file.as_str(), a.line, a.id.as_str()))
        .collect()
}

/// Signed `after - before` count delta. Computed in `i128` (every `usize` is
/// exactly representable) and clamped to `i64`, so two distinct large counts
/// can't both saturate and net to a misleading zero.
fn net(after: usize, before: usize) -> i64 {
    let delta = after as i128 - before as i128;
    // The clamp guarantees the value is in i64 range, so try_from is infallible
    // here; expect makes a future change to the clamp bounds fail loudly.
    i64::try_from(delta.clamp(i128::from(i64::MIN), i128::from(i64::MAX)))
        .expect("value clamped to i64 range always fits i64")
}

fn row_from(snapshot: &DriftSnapshot) -> DriftSnapshotRow {
    DriftSnapshotRow {
        created_at: snapshot.created_at.clone(),
        name: snapshot.name.clone(),
        boundary_violations: snapshot.metrics.boundary_violations,
        antipattern_count: snapshot.metrics.antipattern_count,
        suppression_count: snapshot.metrics.suppression_count,
        files_analysed: snapshot.metrics.files_analysed,
    }
}

/// Build the render-only view. `None` when there are no snapshots (empty state).
/// `snapshots` are newest-first, so the delta runs from `[1]` (before) to `[0]`
/// (after).
fn build_view(
    snapshots: &[DriftSnapshot],
    baseline: Option<&ArchitectureBaseline>,
) -> Option<DriftView> {
    let latest = snapshots.first()?;
    let latest_delta = snapshots.get(1).map(|before| compute_delta(before, latest));
    Some(DriftView {
        snapshots: snapshots.iter().map(row_from).collect(),
        new_vs_baseline: new_vs_baseline(latest, baseline),
        latest_delta,
    })
}

fn build_json(snapshots: &[DriftSnapshot], baseline: Option<&ArchitectureBaseline>) -> DriftJson {
    let latest_delta = match (snapshots.first(), snapshots.get(1)) {
        (Some(after), Some(before)) => {
            let delta = compute_delta(before, after);
            Some(DeltaRecord {
                trend: delta.trend(),
                before_created_at: delta.before_created_at,
                after_created_at: delta.after_created_at,
                violations_added: delta.violations_added,
                violations_removed: delta.violations_removed,
                antipatterns_added: delta.antipatterns_added,
                antipatterns_removed: delta.antipatterns_removed,
                net_violations: delta.net_violations,
                net_antipatterns: delta.net_antipatterns,
            })
        }
        _ => None,
    };
    DriftJson {
        snapshots_present: !snapshots.is_empty(),
        snapshot_count: snapshots.len(),
        baseline_present: baseline.is_some(),
        new_vs_baseline: snapshots
            .first()
            .and_then(|latest| new_vs_baseline(latest, baseline)),
        latest_delta,
        snapshots: snapshots
            .iter()
            .map(|snapshot| SnapshotRecord {
                created_at: snapshot.created_at.clone(),
                name: snapshot.name.clone(),
                boundary_violations: snapshot.metrics.boundary_violations,
                antipattern_count: snapshot.metrics.antipattern_count,
                suppression_count: snapshot.metrics.suppression_count,
                files_analysed: snapshot.metrics.files_analysed,
            })
            .collect(),
    }
}

fn print_summary(snapshots: &[DriftSnapshot], baseline: Option<&ArchitectureBaseline>) {
    if snapshots.is_empty() {
        println!("No drift snapshots found under .anvil/snapshots/.");
        println!("Run `anvil drift snapshot` to capture one.");
        return;
    }
    println!("Drift Snapshots");
    println!("  Snapshots:        {}", snapshots.len());
    match snapshots
        .first()
        .and_then(|latest| new_vs_baseline(latest, baseline))
    {
        Some(count) => println!("  New vs baseline:  {count}"),
        None => println!("  New vs baseline:  (no architecture baseline)"),
    }
    if let (Some(after), Some(before)) = (snapshots.first(), snapshots.get(1)) {
        let delta = compute_delta(before, after);
        println!(
            "  Latest delta:     {} (violations +{}/-{}, antipatterns +{}/-{})",
            delta.trend(),
            delta.violations_added,
            delta.violations_removed,
            delta.antipatterns_added,
            delta.antipatterns_removed
        );
    }
    for snapshot in snapshots {
        let name = snapshot.name.as_deref().unwrap_or("—");
        println!(
            "  {} {name}  violations={} antipatterns={} files={}",
            snapshot.created_at,
            snapshot.metrics.boundary_violations,
            snapshot.metrics.antipattern_count,
            snapshot.metrics.files_analysed
        );
    }
}

#[cfg(test)]
mod tests {
    use anvil_architecture::baseline::{CreateBaselineOptions, create_baseline};
    use anvil_architecture::types::BaselineViolation;

    use crate::commands::drift::{SnapshotMetrics, SnapshotViolation};

    use super::*;

    fn metrics(violations: usize, antipatterns: usize) -> SnapshotMetrics {
        SnapshotMetrics {
            boundary_violations: violations,
            antipattern_count: antipatterns,
            suppression_count: 0,
            expired_suppressions: 0,
            files_analysed: 100,
        }
    }

    fn violation(id: &str) -> SnapshotViolation {
        SnapshotViolation {
            id: id.to_string(),
            violation_type: "boundary".to_string(),
            from_file: "a.ts".to_string(),
            to_file: "b.ts".to_string(),
            from_layer: Some("ui".to_string()),
            to_layer: Some("db".to_string()),
            line: 1,
        }
    }

    fn snapshot(created_at: &str, violations: Vec<SnapshotViolation>) -> DriftSnapshot {
        DriftSnapshot {
            schema_version: "1.0.0".to_string(),
            created_at: created_at.to_string(),
            name: None,
            metrics: metrics(violations.len(), 0),
            antipattern_breakdown: None,
            violations,
            antipatterns: vec![],
            suppressions: vec![],
            git_ref: None,
        }
    }

    fn baseline_with(ids: &[&str]) -> ArchitectureBaseline {
        create_baseline(CreateBaselineOptions {
            entry_points: vec![],
            layers: None,
            boundaries: None,
            violations: ids
                .iter()
                .map(|id| BaselineViolation {
                    id: (*id).to_string(),
                    from_layer: "ui".to_string(),
                    to_layer: "db".to_string(),
                    from_file: "a.ts".to_string(),
                    to_file: "b.ts".to_string(),
                    import_line: 1,
                    rule: None,
                })
                .collect(),
            module_count: 1,
        })
    }

    #[test]
    fn new_vs_baseline_counts_violations_absent_from_baseline() {
        let latest = snapshot("2026-05-20T10:00:00Z", vec![violation("a"), violation("b")]);
        let baseline = baseline_with(&["a"]);
        assert_eq!(new_vs_baseline(&latest, Some(&baseline)), Some(1));
    }

    #[test]
    fn new_vs_baseline_is_none_without_baseline() {
        let latest = snapshot("2026-05-20T10:00:00Z", vec![violation("a")]);
        assert_eq!(new_vs_baseline(&latest, None), None);
    }

    #[test]
    fn compute_delta_diffs_violation_ids_and_net() {
        let before = snapshot("2026-05-13T09:00:00Z", vec![violation("a"), violation("b")]);
        let after = snapshot("2026-05-20T10:00:00Z", vec![violation("b"), violation("c")]);
        let delta = compute_delta(&before, &after);
        assert_eq!(delta.violations_added, 1); // c
        assert_eq!(delta.violations_removed, 1); // a
        assert_eq!(delta.net_violations, 0); // 2 → 2
        assert_eq!(delta.trend(), "stable");
    }

    #[test]
    fn build_view_is_none_without_snapshots() {
        assert!(build_view(&[], None).is_none());
    }

    #[test]
    fn build_view_has_delta_with_two_snapshots() {
        let snapshots = vec![
            snapshot("2026-05-20T10:00:00Z", vec![violation("a")]),
            snapshot("2026-05-13T09:00:00Z", vec![]),
        ];
        let view = build_view(&snapshots, None).unwrap();
        assert_eq!(view.snapshots.len(), 2);
        assert!(view.latest_delta.is_some());
        assert_eq!(view.new_vs_baseline, None);
    }

    #[test]
    fn build_view_single_snapshot_has_no_delta() {
        let snapshots = vec![snapshot("2026-05-20T10:00:00Z", vec![])];
        let view = build_view(&snapshots, None).unwrap();
        assert!(view.latest_delta.is_none());
    }

    #[test]
    fn json_envelope_reports_counts_and_presence() {
        let snapshots = vec![
            snapshot("2026-05-20T10:00:00Z", vec![violation("a"), violation("b")]),
            snapshot("2026-05-13T09:00:00Z", vec![violation("a")]),
        ];
        let baseline = baseline_with(&["a"]);
        let json = build_json(&snapshots, Some(&baseline));
        assert!(json.snapshots_present);
        assert_eq!(json.snapshot_count, 2);
        assert!(json.baseline_present);
        assert_eq!(json.new_vs_baseline, Some(1)); // b is new
        assert!(json.latest_delta.is_some());
    }
}
