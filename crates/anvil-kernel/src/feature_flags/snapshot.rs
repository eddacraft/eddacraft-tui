use anvil_kernel_types::{
    FEATURE_FLAG_SCHEMA_VERSION, FeatureFlagDefinition, FlagValue, FlagValueType,
};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// -------------------------------------------------------------------------
// Snapshot shape
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlagSnapshot {
    pub schema_version: u32,
    pub snapshot_version: u64,
    pub issued_at: String,
    pub flags: Vec<FeatureFlagDefinition>,
}

#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    pub max_age_sec: u64,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self { max_age_sec: 300 }
    }
}

// -------------------------------------------------------------------------
// Snapshot errors
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    InvalidJson(String),
    UnsupportedSchemaVersion {
        got: u32,
        expected: u32,
    },
    MissingFields(String),
    ValueTypeMismatch {
        flag_key: String,
        variant_key: String,
        expected: String,
        got: String,
    },
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(msg) => write!(f, "Invalid JSON: {msg}"),
            Self::UnsupportedSchemaVersion { got, expected } => {
                write!(f, "Unsupported schema version: {got} (expected {expected})")
            }
            Self::MissingFields(msg) => write!(f, "Missing fields: {msg}"),
            Self::ValueTypeMismatch {
                flag_key,
                variant_key,
                expected,
                got,
            } => write!(
                f,
                "Flag \"{flag_key}\" variant \"{variant_key}\" value must be a {expected}, got {got}"
            ),
        }
    }
}

impl std::error::Error for SnapshotError {}

// -------------------------------------------------------------------------
// Snapshot creation
// -------------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering};

static VERSION_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn create_snapshot(flags: &[FeatureFlagDefinition]) -> FeatureFlagSnapshot {
    let version = VERSION_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let issued_at = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format_epoch_secs(d.as_secs()),
        Err(_) => "1970-01-01T00:00:00Z".to_string(),
    };

    FeatureFlagSnapshot {
        schema_version: FEATURE_FLAG_SCHEMA_VERSION,
        snapshot_version: version,
        issued_at,
        flags: flags.to_vec(),
    }
}

pub(super) fn format_epoch_secs(secs: u64) -> String {
    // Simple UTC ISO 8601 without external crate
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since epoch to y-m-d (simplified civil calendar)
    let (year, month, day) = days_to_civil(days);

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn days_to_civil(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// -------------------------------------------------------------------------
// Value-type alignment
// -------------------------------------------------------------------------

fn flag_value_matches_type(value: &FlagValue, vt: FlagValueType) -> bool {
    matches!(
        (value, vt),
        (FlagValue::Boolean(_), FlagValueType::Boolean)
            | (FlagValue::String(_), FlagValueType::String)
            | (FlagValue::Number(_), FlagValueType::Number)
            | (FlagValue::Object(_), FlagValueType::Object)
    )
}

fn flag_value_type_name(value: &FlagValue) -> &'static str {
    match value {
        FlagValue::Boolean(_) => "boolean",
        FlagValue::String(_) => "string",
        FlagValue::Number(_) => "number",
        FlagValue::Object(_) => "object",
    }
}

fn flag_value_type_label(vt: FlagValueType) -> &'static str {
    match vt {
        FlagValueType::Boolean => "boolean",
        FlagValueType::String => "string",
        FlagValueType::Number => "number",
        FlagValueType::Object => "object",
    }
}

// -------------------------------------------------------------------------
// Snapshot loading
// -------------------------------------------------------------------------

pub fn load_snapshot(json: &str) -> Result<FeatureFlagSnapshot, SnapshotError> {
    let snapshot: FeatureFlagSnapshot =
        serde_json::from_str(json).map_err(|e| SnapshotError::InvalidJson(e.to_string()))?;

    if snapshot.schema_version != FEATURE_FLAG_SCHEMA_VERSION {
        return Err(SnapshotError::UnsupportedSchemaVersion {
            got: snapshot.schema_version,
            expected: FEATURE_FLAG_SCHEMA_VERSION,
        });
    }

    if snapshot.snapshot_version < 1 {
        return Err(SnapshotError::InvalidJson(
            "snapshotVersion must be a positive integer".to_string(),
        ));
    }

    if parse_iso_timestamp(&snapshot.issued_at).is_err() {
        return Err(SnapshotError::InvalidJson(
            "issuedAt must be a valid ISO timestamp".to_string(),
        ));
    }

    // Validate variant values match declared value_type
    for flag in &snapshot.flags {
        for variant in &flag.variants {
            if !flag_value_matches_type(&variant.value, flag.value_type) {
                return Err(SnapshotError::ValueTypeMismatch {
                    flag_key: flag.key.clone(),
                    variant_key: variant.key.clone(),
                    expected: flag_value_type_label(flag.value_type).to_string(),
                    got: flag_value_type_name(&variant.value).to_string(),
                });
            }
        }
    }

    Ok(snapshot)
}

// -------------------------------------------------------------------------
// Freshness check
// -------------------------------------------------------------------------

const CLOCK_SKEW_TOLERANCE_SEC: u64 = 60;

pub fn is_snapshot_fresh(snapshot: &FeatureFlagSnapshot, config: &SnapshotConfig) -> bool {
    let Ok(issued) = parse_iso_timestamp(&snapshot.issued_at) else {
        return false;
    };
    let Ok(now) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) else {
        return false;
    };
    let now_secs = now.as_secs();
    // Reject snapshots issued far in the future (beyond clock-skew tolerance)
    if issued > now_secs + CLOCK_SKEW_TOLERANCE_SEC {
        return false;
    }
    let age_secs = now_secs.saturating_sub(issued);
    age_secs <= config.max_age_sec
}

fn parse_iso_timestamp(s: &str) -> Result<u64, ()> {
    // Parse "YYYY-MM-DDThh:mm:ss[.nnn]Z" without external crate
    // C-002: guard against multi-byte UTF-8 before byte-index slicing
    if !s.is_ascii() {
        return Err(());
    }
    // C-001: accept optional fractional seconds (.NNN before Z)
    if s.len() < 20 || !s.ends_with('Z') {
        return Err(());
    }
    // Validate fixed separator positions
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return Err(());
    }
    // If len > 20, expect ".NNN...Z" fractional seconds (skip them)
    if s.len() > 20 && b[19] != b'.' {
        return Err(());
    }

    let year: u64 = s[0..4].parse().map_err(|_| ())?;
    let month: u64 = s[5..7].parse().map_err(|_| ())?;
    let day: u64 = s[8..10].parse().map_err(|_| ())?;
    let hour: u64 = s[11..13].parse().map_err(|_| ())?;
    let min: u64 = s[14..16].parse().map_err(|_| ())?;
    let sec: u64 = s[17..19].parse().map_err(|_| ())?;

    // C-008: reject pre-epoch years to prevent underflow in civil_to_days
    if year < 1970 {
        return Err(());
    }
    // C-009: validate date/time component ranges
    if !(1..=12).contains(&month) || day < 1 || hour > 23 || min > 59 || sec > 59 {
        return Err(());
    }
    // Strict calendar validation for month/day combinations, including leap years
    if day > days_in_month(year, month) {
        return Err(());
    }

    // Convert to epoch seconds (simplified, no leap seconds)
    let days = civil_to_days(year, month, day);
    Ok(days * 86400 + hour * 3600 + min * 60 + sec)
}

fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_month(year: u64, month: u64) -> u64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn civil_to_days(y: u64, m: u64, d: u64) -> u64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::*;

    fn test_flags() -> Vec<FeatureFlagDefinition> {
        vec![FeatureFlagDefinition {
            key: "cli.licence-gate".into(),
            owner: "BAUTH".into(),
            intent: "Gate CLI features".into(),
            class: FlagClass::Entitlement,
            value_type: FlagValueType::Boolean,
            variants: vec![
                FlagVariant {
                    key: "enabled".into(),
                    value: FlagValue::Boolean(true),
                },
                FlagVariant {
                    key: "disabled".into(),
                    value: FlagValue::Boolean(false),
                },
            ],
            default_variant: "disabled".into(),
            status: FlagStatus::Active,
            created_for: "FLAGS-008".into(),
            expiry_or_review_date: None,
            description: None,
            targeting: None,
            primary_group: None,
            tags: None,
            controls_product_features: Vec::new(),
        }]
    }

    // --- create_snapshot ---

    #[test]
    fn create_snapshot_sets_schema_version() {
        let snap = create_snapshot(&test_flags());
        assert_eq!(snap.schema_version, FEATURE_FLAG_SCHEMA_VERSION);
    }

    #[test]
    fn create_snapshot_monotonic_versions() {
        let a = create_snapshot(&test_flags());
        let b = create_snapshot(&test_flags());
        assert!(b.snapshot_version > a.snapshot_version);
    }

    #[test]
    fn create_snapshot_includes_flags() {
        let flags = test_flags();
        let snap = create_snapshot(&flags);
        assert_eq!(snap.flags.len(), 1);
        assert_eq!(snap.flags[0].key, "cli.licence-gate");
    }

    #[test]
    fn create_snapshot_sets_issued_at() {
        let snap = create_snapshot(&test_flags());
        assert!(snap.issued_at.ends_with('Z'));
        assert!(snap.issued_at.len() >= 20);
    }

    // --- load_snapshot ---

    #[test]
    fn load_snapshot_round_trip() {
        let snap = create_snapshot(&test_flags());
        let json = serde_json::to_string(&snap).unwrap();
        let loaded = load_snapshot(&json).unwrap();
        assert_eq!(loaded.schema_version, snap.schema_version);
        assert_eq!(loaded.flags.len(), snap.flags.len());
    }

    #[test]
    fn load_snapshot_invalid_json() {
        let result = load_snapshot("not json");
        assert!(matches!(result, Err(SnapshotError::InvalidJson(_))));
    }

    #[test]
    fn load_snapshot_wrong_version() {
        let snap = create_snapshot(&test_flags());
        let json = serde_json::to_string(&snap).unwrap();
        let mut obj: serde_json::Value = serde_json::from_str(&json).unwrap();
        obj["schemaVersion"] = serde_json::Value::Number(99.into());
        let result = load_snapshot(&obj.to_string());
        assert!(matches!(
            result,
            Err(SnapshotError::UnsupportedSchemaVersion { got: 99, .. })
        ));
    }

    // --- is_snapshot_fresh ---

    #[test]
    fn fresh_snapshot_returns_true() {
        let snap = create_snapshot(&test_flags());
        let config = SnapshotConfig::default();
        assert!(is_snapshot_fresh(&snap, &config));
    }

    #[test]
    fn stale_snapshot_returns_false() {
        let snap = FeatureFlagSnapshot {
            schema_version: FEATURE_FLAG_SCHEMA_VERSION,
            snapshot_version: 1,
            issued_at: "2020-01-01T00:00:00Z".into(),
            flags: test_flags(),
        };
        let config = SnapshotConfig { max_age_sec: 300 };
        assert!(!is_snapshot_fresh(&snap, &config));
    }

    #[test]
    fn custom_max_age() {
        let snap = FeatureFlagSnapshot {
            schema_version: FEATURE_FLAG_SCHEMA_VERSION,
            snapshot_version: 1,
            issued_at: "2020-01-01T00:00:00Z".into(),
            flags: test_flags(),
        };
        // Very large max age should make even an old snapshot fresh
        let config = SnapshotConfig {
            max_age_sec: u64::MAX,
        };
        assert!(is_snapshot_fresh(&snap, &config));
    }

    // --- timestamp helpers ---

    #[test]
    fn civil_round_trip() {
        let epoch_days = civil_to_days(2026, 4, 12);
        let (y, m, d) = days_to_civil(epoch_days);
        assert_eq!((y, m, d), (2026, 4, 12));
    }

    #[test]
    fn parse_iso_known_date() {
        let secs = parse_iso_timestamp("2026-04-12T00:00:00Z").unwrap();
        // 2026-04-12 in epoch seconds
        let expected = civil_to_days(2026, 4, 12) * 86400;
        assert_eq!(secs, expected);
    }

    // --- C-001: fractional seconds ---

    #[test]
    fn parse_iso_with_fractional_seconds() {
        let secs = parse_iso_timestamp("2026-04-12T00:00:00.123Z").unwrap();
        let expected = civil_to_days(2026, 4, 12) * 86400;
        assert_eq!(secs, expected);
    }

    // --- C-002: multi-byte UTF-8 rejection ---

    #[test]
    fn parse_iso_rejects_non_ascii() {
        // Unicode minus sign instead of ASCII hyphen
        assert!(parse_iso_timestamp("2026\u{2212}04\u{2212}12T00:00:00Z").is_err());
    }

    // --- C-008: pre-epoch rejection ---

    #[test]
    fn parse_iso_rejects_pre_epoch() {
        assert!(parse_iso_timestamp("0000-01-01T00:00:00Z").is_err());
        assert!(parse_iso_timestamp("1969-12-31T23:59:59Z").is_err());
    }

    // --- C-009: range validation ---

    #[test]
    fn parse_iso_rejects_invalid_month() {
        assert!(parse_iso_timestamp("2026-13-01T00:00:00Z").is_err());
        assert!(parse_iso_timestamp("2026-00-01T00:00:00Z").is_err());
    }

    #[test]
    fn parse_iso_rejects_invalid_day() {
        assert!(parse_iso_timestamp("2026-01-00T00:00:00Z").is_err());
        assert!(parse_iso_timestamp("2026-01-32T00:00:00Z").is_err());
    }

    #[test]
    fn parse_iso_rejects_invalid_time() {
        assert!(parse_iso_timestamp("2026-01-01T24:00:00Z").is_err());
        assert!(parse_iso_timestamp("2026-01-01T00:60:00Z").is_err());
        assert!(parse_iso_timestamp("2026-01-01T00:00:60Z").is_err());
    }

    // --- C-010: calendar edge cases ---

    #[test]
    fn leap_year_feb_29() {
        // 2024 is a leap year
        let secs = parse_iso_timestamp("2024-02-29T00:00:00Z").unwrap();
        let formatted = format_epoch_secs(secs);
        assert_eq!(formatted, "2024-02-29T00:00:00Z");
    }

    #[test]
    fn century_leap_year_2000() {
        // 2000 is a leap year (divisible by 400)
        let secs = parse_iso_timestamp("2000-02-29T12:30:45Z").unwrap();
        let formatted = format_epoch_secs(secs);
        assert_eq!(formatted, "2000-02-29T12:30:45Z");
    }

    #[test]
    fn century_non_leap_2100() {
        // 2100 is NOT a leap year (divisible by 100 but not 400)
        let secs = parse_iso_timestamp("2100-03-01T00:00:00Z").unwrap();
        let formatted = format_epoch_secs(secs);
        assert_eq!(formatted, "2100-03-01T00:00:00Z");
    }

    #[test]
    fn epoch_boundary() {
        let formatted = format_epoch_secs(0);
        assert_eq!(formatted, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn epoch_round_trip() {
        let formatted = format_epoch_secs(0);
        let secs = parse_iso_timestamp(&formatted).unwrap();
        assert_eq!(secs, 0);
    }

    // --- C-011: strict calendar validation ---

    #[test]
    fn parse_iso_rejects_impossible_dates() {
        // April has 30 days
        assert!(parse_iso_timestamp("2026-04-31T00:00:00Z").is_err());
        // February non-leap year has 28 days
        assert!(parse_iso_timestamp("2026-02-29T00:00:00Z").is_err());
        // June has 30 days
        assert!(parse_iso_timestamp("2026-06-31T00:00:00Z").is_err());
        // September has 30 days
        assert!(parse_iso_timestamp("2026-09-31T00:00:00Z").is_err());
    }

    #[test]
    fn century_non_leap_rejects_feb_29() {
        // 2100 is NOT a leap year
        assert!(parse_iso_timestamp("2100-02-29T00:00:00Z").is_err());
    }

    // --- C-012: load_snapshot validation ---

    #[test]
    fn load_snapshot_rejects_zero_version() {
        let snap = create_snapshot(&test_flags());
        let json = serde_json::to_string(&snap).unwrap();
        let mut obj: serde_json::Value = serde_json::from_str(&json).unwrap();
        obj["snapshotVersion"] = serde_json::Value::Number(0.into());
        let result = load_snapshot(&obj.to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("positive integer"));
    }

    #[test]
    fn load_snapshot_rejects_invalid_issued_at() {
        let snap = create_snapshot(&test_flags());
        let json = serde_json::to_string(&snap).unwrap();
        let mut obj: serde_json::Value = serde_json::from_str(&json).unwrap();
        obj["issuedAt"] = serde_json::Value::String("not-a-date".into());
        let result = load_snapshot(&obj.to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("valid ISO timestamp")
        );
    }

    // --- value-type alignment ---

    #[test]
    fn load_snapshot_rejects_mismatched_value_type() {
        let mut flags = test_flags();
        // Declare boolean but supply a string variant value
        flags[0].value_type = FlagValueType::Boolean;
        flags[0].variants[0].value = FlagValue::String("yes".into());

        let snap = create_snapshot(&flags);
        let json = serde_json::to_string(&snap).unwrap();
        let result = load_snapshot(&json);
        assert!(matches!(
            result,
            Err(SnapshotError::ValueTypeMismatch { .. })
        ));
        let err = result.unwrap_err();
        assert!(err.to_string().contains("boolean"));
        assert!(err.to_string().contains("string"));
    }

    #[test]
    fn load_snapshot_accepts_matching_value_types() {
        // Already covered by load_snapshot_round_trip, but explicit for clarity
        let snap = create_snapshot(&test_flags());
        let json = serde_json::to_string(&snap).unwrap();
        assert!(load_snapshot(&json).is_ok());
    }

    // --- C-013: clock-skew protection ---

    #[test]
    fn future_snapshot_beyond_tolerance_is_stale() {
        // issued_at far in the future (year 2099) should be rejected
        let snap = FeatureFlagSnapshot {
            schema_version: FEATURE_FLAG_SCHEMA_VERSION,
            snapshot_version: 1,
            issued_at: "2099-01-01T00:00:00Z".into(),
            flags: test_flags(),
        };
        let config = SnapshotConfig { max_age_sec: 300 };
        assert!(!is_snapshot_fresh(&snap, &config));
    }
}
