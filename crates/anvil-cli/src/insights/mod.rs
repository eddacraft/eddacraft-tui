pub mod aggregator;
pub mod cumulative;
pub mod drift_trend;
pub mod first_week_hint;
pub mod scorecard;
pub mod suppressions;

use chrono::{DateTime, SecondsFormat, Utc};

/// RFC3339 UTC string at whole-second precision — the shared timestamp
/// format across the insights surfaces (weekly summary windows, drift
/// trend week boundaries).
pub(crate) fn format_utc(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Truncate a timestamp to whole seconds so window arithmetic is
/// deterministic regardless of sub-second jitter in `Utc::now()`.
pub(crate) fn truncate_to_seconds(ts: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::from_timestamp(ts.timestamp(), 0).expect("valid UTC timestamp")
}
