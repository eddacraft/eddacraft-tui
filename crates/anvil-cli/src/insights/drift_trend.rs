//! INSIGHTS-003: Drift Trend Sparkline.
//!
//! Renders new cross-boundary edges per week over the last 8 weeks as a
//! terminal sparkline. The data source is the existing `anvil drift`
//! snapshot store (`.anvil/snapshots/snapshot-*.json`): each
//! [`DriftSnapshot`] records the boundary-violation edge set at a point in
//! time, and consecutive snapshots are diffed by stable violation `id` —
//! the same identity [`crate::commands::drift`]'s `compare_snapshots` uses
//! — to count edges that are *new* relative to the previous snapshot.
//!
//! See the INSIGHTS-003 spec reconciliation (2026-05-29) in
//! `plans/modules/usage-insights.aps.md` for why the originally specified
//! "baseline diff entries" source could not back an 8-week trend (the
//! baseline is a single overwritten snapshot with no per-finding history,
//! and the witness chain carries no edge payload).

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::commands::drift::{DriftSnapshot, list_snapshot_files, load_snapshot_file};

/// Schema-version tag emitted by `anvil insights --drift --json`.
pub const DRIFT_TREND_SCHEMA_VERSION: &str = "anvil.drift_trend.v1";

/// Number of trailing weekly buckets in the trend.
pub const TREND_WEEKS: usize = 8;

/// Calendar span of the trend window, in days (`7 × TREND_WEEKS`).
const WINDOW_DAYS: i64 = 56;

/// Minimum number of weeks (within the window) that must contain a
/// snapshot before a trend line is meaningful. Below this the command
/// reports insufficient data instead of drawing a misleading line.
pub const MIN_WEEKS_WITH_DATA: usize = 2;

/// Sparkline bar glyphs, lowest → highest. Index 0 (`▁`) is a *measured*
/// zero; the separate gap glyph marks a week with no snapshot at all.
const SPARK_BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Rendered for a week that has no snapshot — distinct from a measured
/// zero so the reader does not mistake "we didn't look" for "no drift".
const SPARK_GAP: char = '·';

/// One trailing week of the trend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WeekBucket {
    /// Inclusive UTC start of the week (RFC3339, second precision).
    pub week_start: String,
    /// UTC end of the week (RFC3339, second precision). Exclusive for
    /// every bucket except the final one, whose end equals the window
    /// end (`now`) and is inclusive — a snapshot taken exactly at `now`
    /// is counted in the last week.
    pub week_end: String,
    /// True when at least one snapshot's `created_at` falls in this week.
    /// Distinguishes a measured zero from "no snapshot this week".
    pub has_data: bool,
    /// New cross-boundary edges first observed in this week — the sum of
    /// added-edge counts over snapshot pairs whose later snapshot lands in
    /// this week. Always 0 when `has_data` is false.
    pub new_edges: u64,
}

/// The full drift trend for the window ending at `now`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriftTrend {
    pub schema_version: &'static str,
    pub window_start: String,
    pub window_end: String,
    /// False when fewer than [`MIN_WEEKS_WITH_DATA`] of the trailing weeks
    /// contain a snapshot — callers render the insufficient-data message
    /// instead of the sparkline.
    pub sufficient_data: bool,
    /// Count of trailing weeks that contain at least one snapshot.
    pub weeks_with_data: usize,
    /// Oldest → newest, always [`TREND_WEEKS`] entries.
    pub weeks: Vec<WeekBucket>,
    /// Sparkline string, one glyph per week (oldest → newest). Empty when
    /// `sufficient_data` is false.
    pub sparkline: String,
}

/// Build the drift trend for the [`TREND_WEEKS`] weeks ending at `now`,
/// reading the snapshot store under `repo_root`.
pub fn drift_trend(repo_root: &Path, now: DateTime<Utc>) -> anyhow::Result<DriftTrend> {
    let now = super::truncate_to_seconds(now);
    let window_start = now - Duration::days(WINDOW_DAYS);

    // Load every snapshot, skipping corrupt ones (consistent with
    // `anvil drift list`/`report`). Snapshots whose timestamp won't parse
    // are dropped: they cannot be placed in a bucket. `list_snapshot_files`
    // returns newest-first; we re-sort oldest → newest because the
    // new-edge attribution below walks adjacent pairs front-to-back.
    let mut snapshots: Vec<(DateTime<Utc>, DriftSnapshot)> = Vec::new();
    for path in list_snapshot_files(repo_root)? {
        let Ok(snap) = load_snapshot_file(&path) else {
            continue;
        };
        let Ok(ts) = DateTime::parse_from_rfc3339(&snap.created_at) else {
            continue;
        };
        snapshots.push((ts.with_timezone(&Utc), snap));
    }
    snapshots.sort_by_key(|(ts, _)| *ts);

    let mut weeks: Vec<WeekBucket> = (0..TREND_WEEKS)
        .map(|i| {
            // `i` is bounded by TREND_WEEKS (8); the try_from cannot fail.
            let start = window_start + Duration::days(7 * i64::try_from(i).unwrap_or(0));
            WeekBucket {
                week_start: super::format_utc(start),
                week_end: super::format_utc(start + Duration::days(7)),
                has_data: false,
                new_edges: 0,
            }
        })
        .collect();

    // Pass 1 — presence. Mark every trailing week that contains a snapshot.
    // This is the sole authority for `has_data`: it covers solitary
    // snapshots that have no adjacent pair (e.g. the first in-window
    // snapshot when there is no pre-window seed), which pass 2 would miss.
    for (ts, _) in &snapshots {
        if let Some(i) = bucket_index(*ts, window_start, now) {
            weeks[i].has_data = true;
        }
    }

    // Pass 2 — new-edge counts. Attribute each adjacent pair's newly
    // introduced edges to the *later* snapshot's week. Walking the full
    // chronological list (not just the in-window slice) means the snapshot
    // immediately before the window seeds the first in-window week's
    // baseline. Multiple snapshots in one week sum their per-pair new
    // edges, so a week's count is the number of edge *introductions* that
    // week — an edge added and resolved within the same week still counts
    // once (it was new drift), and an edge that reappears after being
    // removed in an earlier week counts again. This is the intended
    // "introductions per week" signal, not a net end-of-week delta.
    for pair in snapshots.windows(2) {
        let (_, prev) = &pair[0];
        let (curr_ts, curr) = &pair[1];
        if let Some(i) = bucket_index(*curr_ts, window_start, now) {
            weeks[i].new_edges += count_new_edges(prev, curr);
        }
    }

    let weeks_with_data = weeks.iter().filter(|w| w.has_data).count();
    let sufficient_data = weeks_with_data >= MIN_WEEKS_WITH_DATA;
    let sparkline = if sufficient_data {
        render_sparkline(&weeks)
    } else {
        String::new()
    };

    Ok(DriftTrend {
        schema_version: DRIFT_TREND_SCHEMA_VERSION,
        window_start: super::format_utc(window_start),
        window_end: super::format_utc(now),
        sufficient_data,
        weeks_with_data,
        weeks,
        sparkline,
    })
}

/// Plain-text render of a [`DriftTrend`] for the default (non-JSON)
/// output. Pure (returns the string) so the render is unit-testable.
pub fn render_drift_trend(trend: &DriftTrend) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Anvil drift trend — new cross-boundary edges per week (last {TREND_WEEKS} weeks)"
    );
    let _ = writeln!(
        out,
        "Window: {} to {}",
        trend.window_start, trend.window_end
    );
    let _ = writeln!(out);

    if !trend.sufficient_data {
        let _ = writeln!(
            out,
            "Not enough snapshot history to show a trend: found snapshots in {} of \
             the last {TREND_WEEKS} weeks (need at least {MIN_WEEKS_WITH_DATA}).",
            trend.weeks_with_data
        );
        let _ = writeln!(
            out,
            "Run `anvil drift snapshot` regularly to build the trend."
        );
        return out;
    }

    let _ = writeln!(out, "  {}", trend.sparkline);
    let _ = writeln!(out);
    for w in &trend.weeks {
        let date = w.week_start.split('T').next().unwrap_or(&w.week_start);
        if w.has_data {
            let _ = writeln!(out, "  {date}  {} new", w.new_edges);
        } else {
            let _ = writeln!(out, "  {date}  no snapshot");
        }
    }
    out
}

/// Count distinct cross-boundary edges present in `curr` but not `prev`,
/// keyed on the stable violation `id` — the same identity
/// `commands::drift::compare_snapshots` uses for its added/removed diff.
/// Both sides collapse to sets first, so a snapshot that happens to carry
/// the same `id` twice cannot inflate the count.
fn count_new_edges(prev: &DriftSnapshot, curr: &DriftSnapshot) -> u64 {
    let prev_ids: BTreeSet<&str> = prev.violations.iter().map(|v| v.id.as_str()).collect();
    let curr_ids: BTreeSet<&str> = curr.violations.iter().map(|v| v.id.as_str()).collect();
    let added = curr_ids.difference(&prev_ids).count();
    // usize → u64 is lossless on every supported target; the fallback
    // keeps the conversion cast-free for clippy::pedantic.
    u64::try_from(added).unwrap_or(u64::MAX)
}

/// Render one glyph per week (oldest → newest). Measured weeks scale by
/// `new_edges` relative to the max measured value (integer division —
/// deterministic); no-data weeks render [`SPARK_GAP`].
fn render_sparkline(weeks: &[WeekBucket]) -> String {
    let max = weeks
        .iter()
        .filter(|w| w.has_data)
        .map(|w| w.new_edges)
        .max()
        .unwrap_or(0);
    weeks
        .iter()
        .map(|w| {
            if w.has_data {
                SPARK_BARS[bar_index(w.new_edges, max)]
            } else {
                SPARK_GAP
            }
        })
        .collect()
}

/// Map `value` in `0..=max` to a [`SPARK_BARS`] index in `0..=7` by
/// integer division. `max == 0` (every measured week had zero new edges)
/// maps everything to the lowest bar. `value <= max` holds by construction
/// (`max` is the maximum over the same weeks), so the result never exceeds
/// 7; `saturating_mul` + `.min(7)` make that robust without a panic path.
fn bar_index(value: u64, max: u64) -> usize {
    if max == 0 {
        return 0;
    }
    usize::try_from(value.saturating_mul(7) / max)
        .unwrap_or(7)
        .min(7)
}

/// Bucket index (`0..TREND_WEEKS`) for `ts`, or `None` when `ts` falls
/// outside the window. The window is inclusive at both ends
/// (`window_start..=now`); a snapshot dated exactly `now` lands in the
/// final bucket, which is why `WeekBucket::week_end` is exclusive for
/// every bucket except the last.
fn bucket_index(
    ts: DateTime<Utc>,
    window_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Option<usize> {
    if ts < window_start || ts > now {
        return None;
    }
    let day_offset = (ts - window_start).num_days();
    let week = usize::try_from(day_offset / 7).unwrap_or(0);
    Some(week.min(TREND_WEEKS - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_index_scales_by_integer_division() {
        // max == 0 → everything is the lowest bar (measured zero).
        assert_eq!(bar_index(0, 0), 0);
        // value == max → top bar.
        assert_eq!(bar_index(4, 4), 7);
        // proportional, integer division: (1*7)/4 = 1, (2*7)/4 = 3.
        assert_eq!(bar_index(1, 4), 1);
        assert_eq!(bar_index(2, 4), 3);
        // value 0 with a non-zero max is still the lowest bar.
        assert_eq!(bar_index(0, 9), 0);
    }

    #[test]
    fn count_new_edges_uses_violation_id_identity() {
        let prev = snapshot("2026-05-01T00:00:00Z", &["e1", "e2"]);
        // e2 removed (not counted), e3 + e4 added → 2 new edges.
        let curr = snapshot("2026-05-02T00:00:00Z", &["e1", "e3", "e4"]);
        assert_eq!(count_new_edges(&prev, &curr), 2);
    }

    #[test]
    fn bucket_index_places_window_boundaries() {
        let now = ts("2026-05-29T00:00:00Z");
        let start = now - Duration::days(WINDOW_DAYS);
        // Before the window → None.
        assert_eq!(bucket_index(start - Duration::days(1), start, now), None);
        // Exactly window_start → first bucket.
        assert_eq!(bucket_index(start, start, now), Some(0));
        // Exactly now → final bucket (inclusive).
        assert_eq!(bucket_index(now, start, now), Some(TREND_WEEKS - 1));
        // After now → None.
        assert_eq!(bucket_index(now + Duration::days(1), start, now), None);
    }

    // ── helpers shared with commands::insights::tests in spirit ──────

    pub(super) fn ts(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    pub(super) fn snapshot(created_at: &str, edge_ids: &[&str]) -> DriftSnapshot {
        use crate::commands::drift::{SnapshotMetrics, SnapshotViolation};
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
}
