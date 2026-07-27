use std::path::Path;

use crate::api::{
    DataGap, DataState, PROTECTION_HISTORY_SCHEMA, ProtectionHistory, ProtectionHistoryPoint,
    ProtectionHistoryRange,
};
use crate::{Workspace, WorkspaceReadError};

const HISTORY_ARTEFACT: &str = ".anvil/gate-history.ndjson";

pub fn load_protection_history(workspace: &Workspace) -> ProtectionHistory {
    let bytes = match workspace.read(Path::new(HISTORY_ARTEFACT)) {
        Ok(bytes) => bytes,
        Err(WorkspaceReadError::Missing { .. }) => {
            return unavailable("No retained gate history is available yet.");
        }
        Err(error) => {
            return unavailable(format!("Retained gate history could not be read: {error}"));
        }
    };
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return unavailable("Retained gate history is empty.");
    }

    let mut invalid = 0usize;
    let mut points = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
        .filter_map(
            |line| match serde_json::from_slice::<ProtectionHistoryPoint>(line) {
                Ok(point)
                    if matches!(point.status.as_str(), "pass" | "warn" | "fail")
                        && point.score.is_finite()
                        && (0.0..=100.0).contains(&point.score)
                        && utc_sort_key(&point.recorded_at).is_some() =>
                {
                    Some(point)
                }
                _ => {
                    invalid += 1;
                    None
                }
            },
        )
        .collect::<Vec<_>>();
    if points.is_empty() {
        return unavailable(format!(
            "Retained gate history contains no valid points ({invalid} invalid lines)."
        ));
    }
    points.sort_by(|left, right| left.recorded_at.cmp(&right.recorded_at));
    let excess = points.len().saturating_sub(500);
    if excess > 0 {
        points.drain(..excess);
    }
    let actual_range = Some(ProtectionHistoryRange {
        first_recorded_at: points.first().expect("non-empty").recorded_at.clone(),
        last_recorded_at: points.last().expect("non-empty").recorded_at.clone(),
    });
    let mut gaps = reserved_gaps();
    if excess > 0 {
        gaps.push(DataGap {
            component: "gate-history-cap".to_owned(),
            reason: format!("{excess} older valid points exceeded the 500-point response cap."),
        });
    }
    let (data_state, source_message) = if invalid == 0 && excess == 0 {
        (
            DataState::Complete,
            format!("{} retained gate points are available.", points.len()),
        )
    } else {
        if invalid > 0 {
            gaps.push(DataGap {
                component: "gate-history".to_owned(),
                reason: format!("{invalid} invalid history lines were preserved but omitted."),
            });
        }
        (
            DataState::Partial,
            format!(
                "{} retained gate points are available; {invalid} invalid lines and {excess} excess valid points were omitted.",
                points.len()
            ),
        )
    };
    ProtectionHistory {
        schema_version: PROTECTION_HISTORY_SCHEMA.to_owned(),
        data_state,
        source_message,
        actual_range,
        points,
        gaps,
    }
}

fn utc_sort_key(value: &str) -> Option<&str> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || bytes.iter().enumerate().any(|(index, byte)| {
            !matches!(index, 4 | 7 | 10 | 13 | 16 | 19) && !byte.is_ascii_digit()
        })
    {
        return None;
    }
    let year = value[0..4].parse::<u16>().ok()?;
    let month = value[5..7].parse::<u8>().ok()?;
    let day = value[8..10].parse::<u8>().ok()?;
    let hour = value[11..13].parse::<u8>().ok()?;
    let minute = value[14..16].parse::<u8>().ok()?;
    let second = value[17..19].parse::<u8>().ok()?;
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return None,
    };
    (year > 0 && day > 0 && day <= days_in_month && hour < 24 && minute < 60 && second < 60)
        .then_some(value)
}

fn unavailable(message: impl Into<String>) -> ProtectionHistory {
    let mut gaps = reserved_gaps();
    gaps.insert(
        0,
        DataGap {
            component: "gate-history".to_owned(),
            reason: "No valid retained gate points are available.".to_owned(),
        },
    );
    ProtectionHistory {
        schema_version: PROTECTION_HISTORY_SCHEMA.to_owned(),
        data_state: DataState::Unavailable,
        source_message: message.into(),
        actual_range: None,
        points: Vec::new(),
        gaps,
    }
}

fn reserved_gaps() -> Vec<DataGap> {
    vec![
        DataGap {
            component: "drift-history".to_owned(),
            reason: "Drift history is not produced in this version.".to_owned(),
        },
        DataGap {
            component: "suppression-history".to_owned(),
            reason: "Suppression history is not produced in this version.".to_owned(),
        },
    ]
}
