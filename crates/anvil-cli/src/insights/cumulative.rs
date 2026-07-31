//! Cumulative insights aggregation over Kindling / usage observation streams.

use std::io::{BufRead, BufReader, Read as _};
use std::path::Path;

use anvil_intercept::kindling_observation::{
    KIND_CONSTRAINT_APPLIED, KIND_GATE_EVALUATED, SAVE_TIME_GATE_ID,
};
use anvil_intercept_rules::secret::SECRET_RULE_ID;
use anvil_witness::{WitnessLine, witness_paths};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::format_utc;

/// The cumulative value aggregate. Counts and window bounds only — see
/// the module privacy contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CumulativeValue {
    /// Earliest recorded event across all sources (RFC 3339). `None`
    /// means nothing has been recorded yet — consumers must render that
    /// honestly, never as a measured zero.
    pub since: Option<String>,
    /// Latest recorded event across all sources (RFC 3339). This is the
    /// evidence window's own end bound; it deliberately replaces any
    /// "generated at" timestamp so output stays deterministic. It labels
    /// the overall evidence span only — the per-stream windows below
    /// anchor to their own stream's bounds, never to this cross-stream
    /// maximum.
    pub as_of: Option<String>,
    /// Earliest witness event (RFC 3339); `None` when the chain holds no
    /// events — consumers must render that honestly, never as a
    /// measured zero.
    pub witness_first_event: Option<String>,
    /// Latest witness event (RFC 3339) — the anchor of the 30/90-day
    /// windows below. Kept per-stream so machine-wide save-time
    /// activity in another repository can never shift this
    /// repository's witness windows.
    pub witness_last_event: Option<String>,
    /// Witness events since the first recorded event (the chain is
    /// append-only, so this is a genuine all-time count).
    pub witness_events_total: u64,
    /// Witness events in the 30 days ending at
    /// [`Self::witness_last_event`].
    pub witness_events_last_30_days: u64,
    /// Witness events in the 90 days ending at
    /// [`Self::witness_last_event`].
    pub witness_events_last_90_days: u64,
    /// Save-time protection counts over the sidecar's retained window.
    pub save_time: SaveTimeCounts,
}

/// Save-time protection counts, valid **only** over the retained
/// sidecar window named by `window_start`/`window_end`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SaveTimeCounts {
    /// Earliest retained save-time / fence row (RFC 3339); `None` when
    /// the sidecar holds no such rows.
    pub window_start: Option<String>,
    /// Latest retained save-time / fence row (RFC 3339).
    pub window_end: Option<String>,
    /// Save-time gate evaluations observed (pass and fail).
    pub evaluations_observed: u64,
    /// Save-time evaluations that raised at least one finding.
    pub risky_writes_flagged: u64,
    /// Flagged evaluations whose enforcement was blocking (an error
    /// severity finding stopped the write).
    pub writes_blocked: u64,
    /// Secret-detection findings raised at save time.
    pub secret_findings_caught: u64,
    /// Protective fence engagements applied by the daemon.
    pub fences_engaged: u64,
}

impl SaveTimeCounts {
    const fn empty() -> Self {
        Self {
            window_start: None,
            window_end: None,
            evaluations_observed: 0,
            risky_writes_flagged: 0,
            writes_blocked: 0,
            secret_findings_caught: 0,
            fences_engaged: 0,
        }
    }

    /// Whether any save-time / fence evidence is present. Requires
    /// BOTH window bounds so the contract matches its render call
    /// sites, which read `window_start` and `window_end` together
    /// (the aggregator always sets the pair from one `(lo, hi)`).
    #[must_use]
    pub const fn has_evidence(&self) -> bool {
        self.window_start.is_some() && self.window_end.is_some()
    }
}

impl CumulativeValue {
    /// Whether any witness-chain evidence is present. Mirrors
    /// [`SaveTimeCounts::has_evidence`] (requiring BOTH bounds, so the
    /// contract matches the render call sites that read the pair):
    /// renders must branch to an honest "no witness events recorded
    /// yet" line when this is `false`, never print measured-looking
    /// zeros.
    #[must_use]
    pub const fn witness_has_evidence(&self) -> bool {
        self.witness_first_event.is_some() && self.witness_last_event.is_some()
    }
}

/// Minimal, forward-compatible view of one sidecar NDJSON row. Only the
/// classification fields are modelled; serde ignores the rest, so the
/// fields that carry paths / principals / reasons are never even
/// deserialised here.
#[derive(Debug, Deserialize)]
struct SidecarRow {
    kind: String,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    gate_id: Option<String>,
    #[serde(default)]
    outcome: Option<String>,
    #[serde(default)]
    enforcement: Option<String>,
    #[serde(default)]
    rules_violated: Option<Vec<String>>,
}

/// Hard cap on bytes read from any single evidence file.
///
/// Matches the sidecar's 64 MiB retention cap (`usage::trim_usage_sidecar`),
/// so a well-formed source never reaches it; the cap exists so a
/// substituted special file (a FIFO, `/dev/zero`, a planted symlink to
/// either) cannot make a reader loop or allocate unboundedly — the
/// repeat-start value receipt reads this module on a thread that is
/// abandoned on timeout, so an unbounded read would otherwise run on
/// unattended. Bytes past the cap in a single file are not counted
/// (the truncated final line fails to parse and is skipped), which is
/// an honest under-count, never a fabricated claim.
const MAX_SOURCE_READ_BYTES: u64 = 64 * 1024 * 1024;

/// Wire value of [`anvil_intercept::kindling_observation::Outcome::Fail`]
/// (kebab-case serde contract).
const OUTCOME_FAIL: &str = "fail";
/// Wire value of
/// [`anvil_intercept::kindling_observation::Enforcement::Blocking`].
const ENFORCEMENT_BLOCKING: &str = "blocking";

/// Compute the cumulative value aggregate from the repository's witness
/// chain plus the user-scoped usage sidecar at `sidecar_path`.
///
/// Missing sources are empty evidence, not errors. Rows and lines with
/// an unparseable timestamp are skipped entirely — an event that cannot
/// be placed in the evidence window cannot back a windowed claim.
pub fn cumulative_value(repo_root: &Path, sidecar_path: &Path) -> anyhow::Result<CumulativeValue> {
    let witness_ts = collect_witness_timestamps(repo_root)?;
    let (save_time, sidecar_bounds) = collect_save_time_counts(sidecar_path)?;

    let witness_bounds = bounds(&witness_ts);
    let since = merge_bound(
        witness_bounds.map(|(lo, _)| lo),
        sidecar_bounds.map(|(lo, _)| lo),
        std::cmp::min,
    );
    let as_of = merge_bound(
        witness_bounds.map(|(_, hi)| hi),
        sidecar_bounds.map(|(_, hi)| hi),
        std::cmp::max,
    );

    // Windows anchor to the witness chain's OWN latest event — never the
    // cross-stream `as_of`. The sidecar is machine-wide, so save-time
    // activity in another repository must not shift this repository's
    // witness windows (council-797f142a major 1).
    let (last_30, last_90) = match witness_bounds {
        None => (0, 0),
        Some((_, anchor)) => {
            let count_from = |days: i64| {
                let start = anchor - Duration::days(days);
                saturating_u64(
                    witness_ts
                        .iter()
                        .filter(|ts| **ts >= start && **ts <= anchor)
                        .count(),
                )
            };
            (count_from(30), count_from(90))
        }
    };

    Ok(CumulativeValue {
        since: since.map(format_utc),
        as_of: as_of.map(format_utc),
        witness_first_event: witness_bounds.map(|(lo, _)| format_utc(lo)),
        witness_last_event: witness_bounds.map(|(_, hi)| format_utc(hi)),
        witness_events_total: saturating_u64(witness_ts.len()),
        witness_events_last_30_days: last_30,
        witness_events_last_90_days: last_90,
        save_time,
    })
}

/// Every parseable witness-event timestamp across the active chain and
/// its archives.
fn collect_witness_timestamps(repo_root: &Path) -> anyhow::Result<Vec<DateTime<Utc>>> {
    let mut out = Vec::new();
    for path in witness_paths(repo_root) {
        let file = std::fs::File::open(&path)?;
        for raw in BufReader::new(file.take(MAX_SOURCE_READ_BYTES)).lines() {
            let raw = match raw {
                Ok(raw) => raw,
                // A torn write can leave one non-UTF-8 line; skip that
                // line rather than abort the aggregate.
                Err(err) if err.kind() == std::io::ErrorKind::InvalidData => continue,
                Err(err) => return Err(err.into()),
            };
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(witness) = WitnessLine::from_ndjson_line(line.as_bytes()) else {
                continue;
            };
            if let Some(ts) = parse_ts(&witness.ts) {
                out.push(ts);
            }
        }
    }
    Ok(out)
}

/// Inclusive (earliest, latest) timestamp bounds of a set of events.
type TsBounds = (DateTime<Utc>, DateTime<Utc>);

/// Classify the retained sidecar rows into [`SaveTimeCounts`], returning
/// the counts plus the (earliest, latest) bounds of the rows counted.
fn collect_save_time_counts(
    sidecar_path: &Path,
) -> anyhow::Result<(SaveTimeCounts, Option<TsBounds>)> {
    let mut counts = SaveTimeCounts::empty();
    let file = match std::fs::File::open(sidecar_path) {
        Ok(file) => file,
        // A missing sidecar is an empty evidence window, not an error.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok((counts, None)),
        Err(err) => return Err(err.into()),
    };

    let mut lo_hi: Option<TsBounds> = None;
    for raw in BufReader::new(file.take(MAX_SOURCE_READ_BYTES)).lines() {
        let raw = match raw {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::InvalidData => continue,
            Err(err) => return Err(err.into()),
        };
        if raw.trim().is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<SidecarRow>(&raw) else {
            continue;
        };
        // Only events that can be placed on the timeline back a claim.
        let Some(ts) = row.timestamp.as_deref().and_then(parse_ts) else {
            continue;
        };

        match row.kind.as_str() {
            KIND_GATE_EVALUATED => {
                // Defensive gate filter: the DPO sink only persists
                // save-time rows, but never scoop up rows from another
                // gate should that change.
                if row.gate_id.as_deref() != Some(SAVE_TIME_GATE_ID) {
                    continue;
                }
                counts.evaluations_observed += 1;
                if row.outcome.as_deref() == Some(OUTCOME_FAIL) {
                    counts.risky_writes_flagged += 1;
                    if row.enforcement.as_deref() == Some(ENFORCEMENT_BLOCKING) {
                        counts.writes_blocked += 1;
                    }
                }
                if let Some(rules) = &row.rules_violated {
                    counts.secret_findings_caught +=
                        saturating_u64(rules.iter().filter(|rule| *rule == SECRET_RULE_ID).count());
                }
            }
            KIND_CONSTRAINT_APPLIED => {
                counts.fences_engaged += 1;
            }
            _ => continue,
        }

        lo_hi = Some(match lo_hi {
            None => (ts, ts),
            Some((lo, hi)) => (lo.min(ts), hi.max(ts)),
        });
    }

    counts.window_start = lo_hi.map(|(lo, _)| format_utc(lo));
    counts.window_end = lo_hi.map(|(_, hi)| format_utc(hi));
    Ok((counts, lo_hi))
}

/// Lossless on every supported target; saturates defensively if `usize`
/// ever exceeds `u64`.
fn saturating_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

fn parse_ts(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|ts| ts.with_timezone(&Utc))
}

fn bounds(ts: &[DateTime<Utc>]) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let lo = ts.iter().min()?;
    let hi = ts.iter().max()?;
    Some((*lo, *hi))
}

fn merge_bound(
    a: Option<DateTime<Utc>>,
    b: Option<DateTime<Utc>>,
    pick: fn(DateTime<Utc>, DateTime<Utc>) -> DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    match (a, b) {
        (Some(a), Some(b)) => Some(pick(a, b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}
