//! Live-monitoring timestamp display for the watch dashboard (CIB-266).
//!
//! Kernel events carry UTC ISO8601 (`…Z`). Operators reading a live dashboard
//! misread bare UTC as local wall-clock. Relative ages are shown at render time;
//! short non-ISO stamps (e.g. action results as `%H:%M:%S`) pass through.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Format a stored timestamp for live watch panels.
///
/// - Parseable UTC ISO8601 → relative age (`just now`, `45s ago`, `2m ago`, …)
/// - Everything else (short local times, placeholders) → unchanged
pub fn format_live_timestamp(raw: &str, now: SystemTime) -> String {
    match parse_utc_iso8601(raw) {
        Some(then) => format_relative(then, now),
        None => raw.to_string(),
    }
}

fn format_relative(then: SystemTime, now: SystemTime) -> String {
    let secs = match now.duration_since(then) {
        Ok(d) => d.as_secs(),
        // Future stamp (clock skew): treat as "just now".
        Err(_) => return "just now".to_string(),
    };

    if secs < 5 {
        "just now".to_string()
    } else if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3_600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3_600)
    } else {
        format!("{}d ago", secs / 86_400)
    }
}

/// Parse `YYYY-MM-DDTHH:MM:SS[.frac]Z` or `...[+/-]HH:MM` / `...[+/-]HHMM`.
///
/// Returns `None` for short local times, placeholders, and non-ISO strings so
/// the caller can pass them through unchanged.
fn parse_utc_iso8601(s: &str) -> Option<SystemTime> {
    // Fast reject: short times like "10:30:01" or "t".
    // Minimum parseable form is `YYYY-MM-DDTHH:MM:SSZ` (20 chars).
    if s.len() < 20 {
        return None;
    }

    let bytes = s.as_bytes();
    if bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return None;
    }

    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: u32 = s.get(5..7)?.parse().ok()?;
    let day: u32 = s.get(8..10)?.parse().ok()?;
    let hour: u32 = s.get(11..13)?.parse().ok()?;
    let minute: u32 = s.get(14..16)?.parse().ok()?;
    let second: u32 = s.get(17..19)?.parse().ok()?;

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let rest = s.get(19..)?;
    let (frac_nanos, rest) = if let Some(stripped) = rest.strip_prefix('.') {
        let digits_end = stripped
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(stripped.len());
        if digits_end == 0 {
            return None;
        }
        let digits = &stripped[..digits_end];
        let mut nanos: u32 = 0;
        for (i, c) in digits.chars().take(9).enumerate() {
            let digit = u32::from(c) - u32::from('0');
            #[allow(clippy::cast_possible_truncation)]
            let place = 8u32.saturating_sub(i as u32);
            nanos += digit * 10u32.pow(place);
        }
        (nanos, &stripped[digits_end..])
    } else {
        (0, rest)
    };

    let offset_secs = parse_tz_offset(rest)?;
    let days = days_from_civil(year, month, day)?;
    let day_secs = i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);
    let unix = days * 86_400 + day_secs - offset_secs;
    if unix < 0 {
        return None;
    }
    #[allow(clippy::cast_sign_loss)]
    let unix = unix as u64;
    Some(UNIX_EPOCH + Duration::new(unix, frac_nanos))
}

fn parse_tz_offset(rest: &str) -> Option<i64> {
    if rest == "Z" || rest == "z" {
        return Some(0);
    }
    let (sign, body) = if let Some(b) = rest.strip_prefix('+') {
        (1i64, b)
    } else if let Some(b) = rest.strip_prefix('-') {
        (-1i64, b)
    } else {
        return None;
    };

    let (oh, om) = if body.len() == 5 && body.as_bytes().get(2) == Some(&b':') {
        (
            body.get(0..2)?.parse::<i64>().ok()?,
            body.get(3..5)?.parse::<i64>().ok()?,
        )
    } else if body.len() == 4 {
        (
            body.get(0..2)?.parse::<i64>().ok()?,
            body.get(2..4)?.parse::<i64>().ok()?,
        )
    } else {
        return None;
    };
    if !(0..=23).contains(&oh) || !(0..=59).contains(&om) {
        return None;
    }
    Some(sign * (oh * 3600 + om * 60))
}

/// Civil date → days since Unix epoch (1970-01-01). Howard Hinnant algorithm.
fn days_from_civil(mut y: i64, m: u32, d: u32) -> Option<i64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let m = i64::from(m);
    let d = i64::from(d);
    y -= i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let adj_m = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * adj_m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unix(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    /// Fixed "now": 2026-08-06T12:00:00Z
    fn fixed_now() -> SystemTime {
        // Verified via parse_roundtrip_matches_days_from_civil.
        unix(1_786_017_600)
    }

    fn stamp_secs_ago(ago: u64) -> String {
        let then = fixed_now() - Duration::from_secs(ago);
        system_time_to_iso_z(then)
    }

    fn system_time_to_iso_z(t: SystemTime) -> String {
        let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        // Mirror kernel emitter civil conversion so fixtures match production stamps.
        const SECS_PER_DAY: u64 = 86_400;
        const DAYS_PER_400Y: u64 = 146_097;
        const DAYS_PER_100Y: u64 = 36_524;
        const DAYS_PER_4Y: u64 = 1_461;
        const DAYS_PER_YEAR: u64 = 365;

        let time_of_day = secs % SECS_PER_DAY;
        let hour = time_of_day / 3600;
        let minute = (time_of_day % 3600) / 60;
        let second = time_of_day % 60;

        let mut days = secs / SECS_PER_DAY;
        days += 719_468;
        let era = days / DAYS_PER_400Y;
        let day_of_era = days % DAYS_PER_400Y;
        let year_of_era = (day_of_era - day_of_era / (DAYS_PER_4Y - 1)
            + day_of_era / DAYS_PER_100Y
            - day_of_era / (DAYS_PER_400Y - 1))
            / DAYS_PER_YEAR;
        let mut year = year_of_era + era * 400;
        let day_of_year =
            day_of_era - (DAYS_PER_YEAR * year_of_era + year_of_era / 4 - year_of_era / 100);
        let mp = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        if month <= 2 {
            year += 1;
        }
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    }

    #[test]
    fn seventy_five_seconds_ago_is_one_minute_not_bare_z() {
        let raw = stamp_secs_ago(75);
        assert!(raw.ends_with('Z'), "fixture must be bare UTC ISO: {raw}");
        let out = format_live_timestamp(&raw, fixed_now());
        assert_eq!(out, "1m ago");
        assert!(!out.contains('Z'));
    }

    #[test]
    fn forty_five_seconds_ago_is_seconds() {
        let raw = stamp_secs_ago(45);
        assert_eq!(format_live_timestamp(&raw, fixed_now()), "45s ago");
    }

    #[test]
    fn under_five_seconds_is_just_now() {
        let raw = stamp_secs_ago(3);
        assert_eq!(format_live_timestamp(&raw, fixed_now()), "just now");
    }

    #[test]
    fn exact_now_is_just_now() {
        let raw = stamp_secs_ago(0);
        assert_eq!(format_live_timestamp(&raw, fixed_now()), "just now");
    }

    #[test]
    fn hours_and_days() {
        assert_eq!(
            format_live_timestamp(&stamp_secs_ago(7_200), fixed_now()),
            "2h ago"
        );
        assert_eq!(
            format_live_timestamp(&stamp_secs_ago(172_800), fixed_now()),
            "2d ago"
        );
    }

    #[test]
    fn future_small_delta_clamps_to_just_now() {
        let future = system_time_to_iso_z(fixed_now() + Duration::from_secs(2));
        assert_eq!(format_live_timestamp(&future, fixed_now()), "just now");
    }

    #[test]
    fn short_local_time_passes_through() {
        assert_eq!(
            format_live_timestamp("10:30:01", fixed_now()),
            "10:30:01"
        );
    }

    #[test]
    fn non_iso_placeholder_passes_through() {
        assert_eq!(format_live_timestamp("t", fixed_now()), "t");
        assert_eq!(format_live_timestamp("", fixed_now()), "");
    }

    #[test]
    fn fractional_seconds_z_parsed() {
        let then = fixed_now() - Duration::from_secs(75);
        let base = system_time_to_iso_z(then);
        let with_frac = base.replace('Z', ".123Z");
        assert_eq!(format_live_timestamp(&with_frac, fixed_now()), "1m ago");
    }

    #[test]
    fn plus_zero_offset_parsed() {
        let raw = stamp_secs_ago(90).replace('Z', "+00:00");
        assert_eq!(format_live_timestamp(&raw, fixed_now()), "1m ago");
    }

    #[test]
    fn known_kernel_style_stamp() {
        // Explicit stamp: 2026-08-06T11:58:45Z is 75s before 12:00:00Z
        let raw = "2026-08-06T11:58:45Z";
        assert_eq!(format_live_timestamp(raw, fixed_now()), "1m ago");
    }

    #[test]
    fn parse_roundtrip_matches_days_from_civil() {
        let t = parse_utc_iso8601("1970-01-01T00:00:00Z").unwrap();
        assert_eq!(t, UNIX_EPOCH);
        let t = parse_utc_iso8601("2026-08-06T12:00:00Z").unwrap();
        assert_eq!(t, fixed_now());
    }
}
