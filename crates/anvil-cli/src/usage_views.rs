//! USAGE-003: dev-investment query views over the command-invocation
//! usage log.
//!
//! Read-side counterpart to [`crate::usage`] (the USAGE-001 producer).
//! These pure functions answer the founder's standing questions — "what
//! is being used and what is not" — over the NDJSON sidecar at
//! `<credentials_dir>/kindling/usage.ndjson`, without ad-hoc SQL or `jq`
//! each time. The CLI surface (`anvil kindling usage <view>`) renders
//! them; the runbook in `docs/observability/usage-analytics.md` documents
//! them and the standing caveat: **these views are signal, not evidence**
//! (small populations, flag bias, survivorship — they inform direction,
//! not decisions in isolation).
//!
//! ## Robustness
//!
//! [`load_rows`] is a best-effort consumer of an append-only log: a torn
//! final line (interrupted write) or a row written by a newer, richer
//! producer must never abort a view. Malformed lines are skipped; unknown
//! fields are ignored ([`UsageRow`] models only what the views read, so
//! it stays forward-compatible as the row shape grows).

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};
use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// One usage row, deserialised from a single NDJSON line.
///
/// Deliberately a *subset* of the producer's `CommandInvokedObservation`:
/// only the fields the views consult are modelled, and serde ignores the
/// rest, so an older reader still parses rows from a newer producer.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageRow {
    pub command: String,
    pub principal: String,
    pub timestamp: String,
    #[serde(default)]
    pub flag_set: Vec<FlagEntry>,
}

/// The inline flag-context entry the views need (a subset of the
/// producer's `FlagSetEntry`).
#[derive(Debug, Clone, Deserialize)]
pub struct FlagEntry {
    pub key: String,
    pub variant: String,
    #[serde(default)]
    pub gate_affecting: bool,
}

/// Time window for count-based views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// The trailing 7 days.
    Week,
    /// The trailing 30 days.
    Month,
    /// Everything since launch (no lower bound).
    All,
}

impl Period {
    /// The inclusive lower bound for this window relative to `now`, or
    /// `None` for [`Period::All`].
    fn cutoff(self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Period::Week => Some(now - Duration::days(7)),
            Period::Month => Some(now - Duration::days(30)),
            Period::All => None,
        }
    }
}

/// Whether a row falls within the window. Rows with an unparseable
/// timestamp are kept only for the unbounded ([`Period::All`]) window;
/// a windowed view excludes them rather than guess their age.
fn in_period(row: &UsageRow, cutoff: Option<DateTime<Utc>>) -> bool {
    match cutoff {
        None => true,
        Some(cut) => DateTime::parse_from_rfc3339(&row.timestamp)
            .is_ok_and(|ts| ts.with_timezone(&Utc) >= cut),
    }
}

/// Load usage rows from an NDJSON sidecar, skipping blank, malformed, and
/// non-UTF-8 lines. A missing file is an empty log, not an error (the
/// views answer "nothing recorded yet" cleanly).
///
/// Reads line-by-line through a [`BufReader`] rather than slurping the
/// whole file, so a single corrupt byte sequence (e.g. a torn write that
/// split a multi-byte UTF-8 codepoint) only drops *that* line instead of
/// aborting every view with an `InvalidData` error. Peak memory is the
/// parsed rows plus one line, not the entire file plus the rows. The
/// append-only sidecar is unrotated today (see the runbook's retention
/// note); at the founder-laptop scale this is comfortable, and rotation
/// is the tracked upgrade path before any high-frequency producer lands.
pub fn load_rows(path: &Path) -> std::io::Result<Vec<UsageRow>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let mut rows = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(line) => line,
            // A non-UTF-8 line is corrupt data for that line only (e.g. a
            // torn write that split a multi-byte codepoint) — skip it.
            Err(err) if err.kind() == std::io::ErrorKind::InvalidData => continue,
            // A genuine I/O failure (a read error mid-file) is real and is
            // surfaced, not silently swallowed — the function returns
            // `io::Result` precisely so the caller learns of it.
            Err(err) => return Err(err),
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(row) = serde_json::from_str::<UsageRow>(&line) {
            rows.push(row);
        }
    }
    Ok(rows)
}

/// A command and its invocation count within the chosen window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandCount {
    pub command: String,
    pub count: usize,
}

/// Top commands by invocation count within `period`, most-invoked first.
///
/// Ties break by command name (ascending) so the order is stable across
/// runs. `limit == 0` means no cap.
#[must_use]
pub fn top_commands(
    rows: &[UsageRow],
    period: Period,
    now: DateTime<Utc>,
    limit: usize,
) -> Vec<CommandCount> {
    let cutoff = period.cutoff(now);
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows.iter().filter(|row| in_period(row, cutoff)) {
        *counts.entry(row.command.as_str()).or_default() += 1;
    }
    let mut ranked: Vec<CommandCount> = counts
        .into_iter()
        .map(|(command, count)| CommandCount {
            command: command.to_owned(),
            count,
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.command.cmp(&b.command))
    });
    if limit > 0 {
        ranked.truncate(limit);
    }
    ranked
}

/// Registered commands that have never appeared in the log.
///
/// `registered` is the canonical command surface (from clap
/// introspection at the call site). The result is sorted for a stable
/// view. Note (signal, not evidence): a command recorded under a
/// finer-grained canonical name than its clap name — `auth` runs as
/// `auth-login` / `auth-logout` — can still show here even though a
/// subcommand ran; the runbook documents this.
#[must_use]
pub fn never_invoked(rows: &[UsageRow], registered: &[String]) -> Vec<String> {
    let seen: BTreeSet<&str> = rows.iter().map(|row| row.command.as_str()).collect();
    let mut unused: Vec<String> = registered
        .iter()
        .filter(|name| !seen.contains(name.as_str()))
        .cloned()
        .collect();
    unused.sort();
    unused.dedup();
    unused
}

/// A flag variant and how many invocations carried it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VariantCount {
    pub variant: String,
    pub count: usize,
}

/// A flag-dependent path observed in the log: which flag was active, how
/// often, whether it gates, and the variant breakdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlagUsage {
    pub key: String,
    pub invocations: usize,
    pub gate_affecting: bool,
    pub variants: Vec<VariantCount>,
}

/// Flag-dependent paths *exercised* in the log: every flag key seen in
/// any row's `flag_set`, with the number of **rows** that carried it, the
/// variant breakdown, and whether it was *ever* gate-affecting,
/// most-exercised first.
///
/// Each key is counted at most once per row (the producer already dedups
/// a key within a row; this enforces it defensively so a hand-edited row
/// with a repeated key cannot inflate the count). `gate_affecting` is the
/// OR across all observations — `true` once any row saw the flag gate, so
/// it reflects "ever gated", not the flag's current configuration.
///
/// This reports the *exercised* side only. The complement — manifest
/// flags never observed ("not exercised") — needs the flag catalogue and
/// is documented in the runbook rather than computed here, to avoid
/// coupling the read surface to the manifest.
#[must_use]
pub fn flag_usage(rows: &[UsageRow]) -> Vec<FlagUsage> {
    // key -> (gate_affecting seen, variant -> count)
    let mut by_key: BTreeMap<&str, (bool, BTreeMap<&str, usize>)> = BTreeMap::new();
    for row in rows {
        // Count each key at most once per row.
        let mut seen_in_row: BTreeSet<&str> = BTreeSet::new();
        for entry in &row.flag_set {
            if !seen_in_row.insert(entry.key.as_str()) {
                continue;
            }
            let slot = by_key.entry(entry.key.as_str()).or_default();
            slot.0 |= entry.gate_affecting;
            *slot.1.entry(entry.variant.as_str()).or_default() += 1;
        }
    }
    let mut usage: Vec<FlagUsage> = by_key
        .into_iter()
        .map(|(key, (gate_affecting, variants))| {
            let invocations = variants.values().sum();
            let mut variants: Vec<VariantCount> = variants
                .into_iter()
                .map(|(variant, count)| VariantCount {
                    variant: variant.to_owned(),
                    count,
                })
                .collect();
            variants.sort_by(|a, b| {
                b.count
                    .cmp(&a.count)
                    .then_with(|| a.variant.cmp(&b.variant))
            });
            FlagUsage {
                key: key.to_owned(),
                invocations,
                gate_affecting,
                variants,
            }
        })
        .collect();
    usage.sort_by(|a, b| {
        b.invocations
            .cmp(&a.invocations)
            .then_with(|| a.key.cmp(&b.key))
    });
    usage
}

/// An anonymised principal and its invocation count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrincipalActivity {
    pub principal: String,
    pub invocations: usize,
}

/// Principals by activity level, most-active first.
///
/// The principal is passed through **exactly as recorded** — the
/// anonymised hash, or the literal `anonymous`. Anonymisation is the
/// producer's contract (OQ2): this read surface does not (and cannot)
/// re-derive a raw identity, but it also does not re-validate the stored
/// value, so it surfaces whatever the producer wrote. The producer is the
/// single point that guarantees no raw identity ever lands in a row.
#[must_use]
pub fn principals_by_activity(rows: &[UsageRow]) -> Vec<PrincipalActivity> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows {
        *counts.entry(row.principal.as_str()).or_default() += 1;
    }
    let mut activity: Vec<PrincipalActivity> = counts
        .into_iter()
        .map(|(principal, invocations)| PrincipalActivity {
            principal: principal.to_owned(),
            invocations,
        })
        .collect();
    activity.sort_by(|a, b| {
        b.invocations
            .cmp(&a.invocations)
            .then_with(|| a.principal.cmp(&b.principal))
    });
    activity
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn row(command: &str, principal: &str, ts: &str, flags: &[(&str, &str, bool)]) -> UsageRow {
        UsageRow {
            command: command.to_owned(),
            principal: principal.to_owned(),
            timestamp: ts.to_owned(),
            flag_set: flags
                .iter()
                .map(|(key, variant, gate)| FlagEntry {
                    key: (*key).to_owned(),
                    variant: (*variant).to_owned(),
                    gate_affecting: *gate,
                })
                .collect(),
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-06-14T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn top_commands_ranks_by_count_then_name() {
        let rows = vec![
            row("check", "p1", "2026-06-14T10:00:00Z", &[]),
            row("check", "p1", "2026-06-14T10:01:00Z", &[]),
            row("status", "p1", "2026-06-14T10:02:00Z", &[]),
            row("audit", "p1", "2026-06-14T10:03:00Z", &[]),
        ];
        let top = top_commands(&rows, Period::All, now(), 10);
        assert_eq!(
            top[0],
            CommandCount {
                command: "check".into(),
                count: 2
            }
        );
        // status and audit both have count 1 → name-ascending tiebreak.
        assert_eq!(top[1].command, "audit");
        assert_eq!(top[2].command, "status");
    }

    #[test]
    fn top_commands_respects_limit() {
        let rows = vec![
            row("a", "p", "2026-06-14T10:00:00Z", &[]),
            row("b", "p", "2026-06-14T10:00:00Z", &[]),
            row("c", "p", "2026-06-14T10:00:00Z", &[]),
        ];
        assert_eq!(top_commands(&rows, Period::All, now(), 2).len(), 2);
        assert_eq!(
            top_commands(&rows, Period::All, now(), 0).len(),
            3,
            "0 = no cap"
        );
    }

    #[test]
    fn top_commands_filters_by_window() {
        let rows = vec![
            row("recent", "p", "2026-06-12T10:00:00Z", &[]), // within a week
            row("old", "p", "2026-04-01T10:00:00Z", &[]),    // months ago
        ];
        let week = top_commands(&rows, Period::Week, now(), 10);
        assert_eq!(week.len(), 1);
        assert_eq!(week[0].command, "recent");

        let month = top_commands(&rows, Period::Month, now(), 10);
        assert_eq!(month.len(), 1, "the April row is outside 30 days");

        let all = top_commands(&rows, Period::All, now(), 10);
        assert_eq!(all.len(), 2, "All ignores the window");
    }

    #[test]
    fn windowed_view_excludes_unparseable_timestamp() {
        let rows = vec![row("weird", "p", "not-a-timestamp", &[])];
        assert!(
            top_commands(&rows, Period::Week, now(), 10).is_empty(),
            "an unparseable timestamp is excluded from a windowed view"
        );
        assert_eq!(
            top_commands(&rows, Period::All, now(), 10).len(),
            1,
            "but kept for the unbounded view"
        );
    }

    #[test]
    fn never_invoked_is_registered_minus_seen() {
        let rows = vec![
            row("check", "p", "2026-06-14T10:00:00Z", &[]),
            row("status", "p", "2026-06-14T10:00:00Z", &[]),
        ];
        let registered = [
            "audit".to_string(),
            "check".to_string(),
            "status".to_string(),
            "watch".to_string(),
        ];
        let unused = never_invoked(&rows, &registered);
        assert_eq!(unused, vec!["audit".to_string(), "watch".to_string()]);
    }

    #[test]
    fn flag_usage_groups_by_key_and_variant() {
        let rows = vec![
            row(
                "status",
                "p",
                "2026-06-14T10:00:00Z",
                &[("cli.licence-gate", "enabled", true)],
            ),
            row(
                "status",
                "p",
                "2026-06-14T10:01:00Z",
                &[("cli.licence-gate", "enabled", true)],
            ),
            row(
                "check",
                "p",
                "2026-06-14T10:02:00Z",
                &[("cli.licence-gate", "disabled", true)],
            ),
            row("version", "p", "2026-06-14T10:03:00Z", &[]),
        ];
        let usage = flag_usage(&rows);
        assert_eq!(usage.len(), 1, "one distinct flag key observed");
        let gate = &usage[0];
        assert_eq!(gate.key, "cli.licence-gate");
        assert_eq!(gate.invocations, 3);
        assert!(gate.gate_affecting);
        // enabled (2) ranks before disabled (1).
        assert_eq!(
            gate.variants[0],
            VariantCount {
                variant: "enabled".into(),
                count: 2
            }
        );
        assert_eq!(
            gate.variants[1],
            VariantCount {
                variant: "disabled".into(),
                count: 1
            }
        );
    }

    #[test]
    fn principals_ranked_by_activity() {
        let rows = vec![
            row("a", "alice", "2026-06-14T10:00:00Z", &[]),
            row("b", "alice", "2026-06-14T10:01:00Z", &[]),
            row("c", "bob", "2026-06-14T10:02:00Z", &[]),
            row("d", "anonymous", "2026-06-14T10:03:00Z", &[]),
        ];
        let activity = principals_by_activity(&rows);
        assert_eq!(
            activity[0],
            PrincipalActivity {
                principal: "alice".into(),
                invocations: 2
            }
        );
        // bob and anonymous tie at 1 → name-ascending (anonymous < bob).
        assert_eq!(activity[1].principal, "anonymous");
        assert_eq!(activity[2].principal, "bob");
    }

    #[test]
    fn load_rows_skips_malformed_and_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("usage.ndjson");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"kind":"command.invoked","command":"check","principal":"p","timestamp":"2026-06-14T10:00:00Z","args":[],"flag_set":[]}}"#).unwrap();
        writeln!(f).unwrap(); // blank line
        writeln!(f, "{{ this is not json").unwrap(); // torn / malformed
        writeln!(
            f,
            r#"{{"command":"status","principal":"p","timestamp":"2026-06-14T10:01:00Z"}}"#
        )
        .unwrap(); // missing optional flag_set
        drop(f);

        let rows = load_rows(&path).unwrap();
        assert_eq!(rows.len(), 2, "blank and malformed lines skipped");
        assert_eq!(rows[0].command, "check");
        assert_eq!(rows[1].command, "status");
        assert!(
            rows[1].flag_set.is_empty(),
            "absent flag_set defaults to empty"
        );
    }

    #[test]
    fn load_rows_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.ndjson");
        assert!(load_rows(&path).unwrap().is_empty());
    }

    #[test]
    fn load_rows_skips_non_utf8_line_without_aborting() {
        // A torn write that split a multi-byte codepoint leaves invalid
        // UTF-8 on one line. It must drop only that line, not fail the
        // whole view with an InvalidData error.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("usage.ndjson");
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(
            br#"{"command":"check","principal":"p","timestamp":"2026-06-14T10:00:00Z"}"#,
        );
        bytes.push(b'\n');
        bytes.extend_from_slice(&[0xff, 0xfe, 0x00]); // invalid UTF-8 line
        bytes.push(b'\n');
        bytes.extend_from_slice(
            br#"{"command":"status","principal":"p","timestamp":"2026-06-14T10:01:00Z"}"#,
        );
        bytes.push(b'\n');
        std::fs::write(&path, bytes).unwrap();

        let rows = load_rows(&path).unwrap();
        assert_eq!(rows.len(), 2, "the non-UTF-8 line is skipped, not fatal");
        assert_eq!(rows[0].command, "check");
        assert_eq!(rows[1].command, "status");
    }

    #[test]
    fn flag_usage_counts_each_key_once_per_row() {
        // A hand-edited row with a duplicated key must not inflate the
        // invocation count: the flag was active for one invocation.
        let rows = vec![row(
            "status",
            "p",
            "2026-06-14T10:00:00Z",
            &[
                ("cli.licence-gate", "enabled", true),
                ("cli.licence-gate", "enabled", true),
            ],
        )];
        let usage = flag_usage(&rows);
        assert_eq!(usage.len(), 1);
        assert_eq!(
            usage[0].invocations, 1,
            "duplicate key in one row counts once"
        );
    }
}
