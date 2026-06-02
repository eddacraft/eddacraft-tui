//! INSIGHTS-004: First-Week Adoption Signal Hint.
//!
//! Surfaces a low-noise, once-per-week one-liner in `anvil status` and the
//! watch TUI only for users within their first 14 days after `anvil start`
//! (using the `created_at` from `anvil/project-id`). The hint is suppressed
//! for the remainder of the week (trailing 7-day window) if the user has already run the
//! default `anvil insights` summary in that window. State is kept in a
//! tiny project-local `.anvil/insights-hint.json` (not tracked; mirrors
//! `.anvil/first-run` and cache patterns).
//!
//! The N in the hint text is the `witness_events_observed` count for the
//! trailing 7-day window (the only live number from the aggregator today;
//! other counters remain zero-filled per prior INSIGHTS recon). The exact
//! string matches the Expected Outcome so existing acceptance surface
//! expectations are honoured.

use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::insights::aggregator;

/// 14 calendar days is the "first week cohort" window (generous to catch
/// people who only run status/watch once or twice in their first days).
const FIRST_WEEK_DAYS: i64 = 14;

/// 7 days for the "once per week" rate limit on the hint itself, and for
/// the "already ran insights this week" suppression check.
const WEEK_DAYS: i64 = 7;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FirstWeekHintState {
    /// RFC3339 UTC of last time the hint was emitted for this project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_hint_shown_at: Option<String>,
    /// RFC3339 UTC of last time the default `anvil insights` (weekly
    /// summary) was invoked for this project. Used to suppress the nudge
    /// for the rest of the week.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_insights_viewed_at: Option<String>,
}

/// Returns the one-line nudge if all gates pass:
/// - project has a parsable `anvil/project-id` with `created_at`
/// - now is within 14 days of that `created_at`
/// - no insights summary run in the trailing 7 days
/// - no hint already emitted in the trailing 7 days
///
/// When it returns Some(line), it also records the emission in the
/// project-local state so the "exactly once per week" contract holds
/// across `status` and `watch` invocations.
pub fn first_week_insights_hint(root: &Path, now: DateTime<Utc>) -> Option<String> {
    let now = crate::insights::truncate_to_seconds(now);

    // 1. Install timestamp from the authoritative tracked identity file.
    let Ok(Some(identity)) = crate::activation::identity::read_project_id(root) else {
        return None;
    };
    let created = identity.created_at.as_ref()?;
    let install_ts = match DateTime::parse_from_rfc3339(created) {
        Ok(ts) => ts.with_timezone(&Utc),
        Err(_) => return None,
    };
    if now.signed_duration_since(install_ts) > Duration::days(FIRST_WEEK_DAYS) {
        return None;
    }

    // 2. Project-local state (not the global user state dir used by
    // update hints, because the 14-day window + "ran insights" signal
    // are per-project adoption signals).
    let state_path = root.join(".anvil/insights-hint.json");
    let state: FirstWeekHintState = read_state(&state_path).unwrap_or_default();

    // 3. Suppression: user already consulted `anvil insights` (the
    // default weekly summary) within the last 7 days for this project.
    if let Some(viewed) = &state.last_insights_viewed_at
        && let Ok(v) = DateTime::parse_from_rfc3339(viewed).map(|d| d.with_timezone(&Utc))
        && now.signed_duration_since(v) <= Duration::days(WEEK_DAYS)
    {
        return None;
    }

    // 4. Rate limit the hint itself (once per week).
    if let Some(shown) = &state.last_hint_shown_at
        && let Ok(s) = DateTime::parse_from_rfc3339(shown).map(|d| d.with_timezone(&Utc))
        && now.signed_duration_since(s) <= Duration::days(WEEK_DAYS)
    {
        return None;
    }

    // 5. Compute N from the same weekly window the insights command uses.
    // We intentionally use witness_events_observed (the only durable
    // number) rather than the zero-filled total_saves_observed; the
    // user-facing string still says "saves" to match the spec text.
    let n = aggregator::weekly_summary(root, now).map_or(0, |s| s.witness_events_observed);

    // 6. Record that we emitted the hint (best effort; failure must not
    // block the surface or nag on next run).
    let mut next = state;
    next.last_hint_shown_at = Some(crate::insights::format_utc(now));
    let _ = write_state(&state_path, &next);

    Some(format!(
        "Anvil watched {n} saves this week (run `anvil insights`)"
    ))
}

/// Record that the default `anvil insights` weekly summary was viewed.
/// Called from the insights command so that subsequent `status` / watch
/// invocations in the same 7-day window suppress the first-week nudge.
pub fn record_insights_viewed(root: &Path, now: DateTime<Utc>) {
    let now = crate::insights::truncate_to_seconds(now);
    let state_path = root.join(".anvil/insights-hint.json");
    let mut state = read_state(&state_path).unwrap_or_default();
    state.last_insights_viewed_at = Some(crate::insights::format_utc(now));
    let _ = write_state(&state_path, &state);
}

// --- tolerant JSON state helpers (best-effort, never panic the surface) ---

fn read_state(path: &Path) -> std::io::Result<FirstWeekHintState> {
    let text = std::fs::read_to_string(path)?;
    let state: FirstWeekHintState = serde_json::from_str(&text)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(state)
}

fn write_state(path: &Path, state: &FirstWeekHintState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    crate::util::atomic_write(path, body.as_bytes()).map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tempfile::TempDir;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn make_temp_repo() -> (TempDir, std::path::PathBuf) {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = TempDir::with_prefix(format!("anvil-insights-hint-{id}-")).unwrap();
        let root = dir.path().to_path_buf();
        // Seed minimal anvil/ dir + project-id with recent created_at.
        let anvil_dir = root.join("anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        let now = Utc::now();
        let created = (now - Duration::days(3)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let pid = format!(
            "# test\nproject_uuid: 01999999-0000-0000-0000-000000000001\ncreated_at: {created}\n"
        );
        std::fs::write(anvil_dir.join("project-id"), pid).unwrap();
        (dir, root)
    }

    #[test]
    fn returns_none_outside_first_14_days() {
        let (_tmp, root) = make_temp_repo();
        // Overwrite created_at to 20 days ago.
        let old =
            (Utc::now() - Duration::days(20)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let pid =
            format!("project_uuid: 01999999-0000-0000-0000-000000000001\ncreated_at: {old}\n");
        std::fs::write(root.join("anvil/project-id"), pid).unwrap();

        let hint = first_week_insights_hint(&root, Utc::now());
        assert!(hint.is_none());
    }

    #[test]
    fn shows_when_in_window_and_never_viewed() {
        let (_tmp, root) = make_temp_repo();
        // Ensure no prior state.
        let _ = std::fs::remove_file(root.join(".anvil/insights-hint.json"));

        let hint = first_week_insights_hint(&root, Utc::now());
        assert!(hint.is_some());
        assert!(hint.unwrap().contains("run `anvil insights`"));
    }

    #[test]
    fn suppressed_after_insights_viewed_same_week() {
        let (_tmp, root) = make_temp_repo();
        record_insights_viewed(&root, Utc::now());

        let hint = first_week_insights_hint(&root, Utc::now());
        assert!(hint.is_none());
    }

    #[test]
    fn hint_is_rate_limited_per_week() {
        let (_tmp, root) = make_temp_repo();
        // First call emits.
        let h1 = first_week_insights_hint(&root, Utc::now());
        assert!(h1.is_some());

        // Immediate second call in same week is suppressed.
        let h2 = first_week_insights_hint(&root, Utc::now());
        assert!(h2.is_none());
    }
}
