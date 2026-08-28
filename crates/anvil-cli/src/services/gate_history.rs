//! Reader for `.anvil/gate-history.ndjson` — the live Rust gate history
//! written by `anvil gate`.
//!
//! Status Recent Runs and audit historical scores bind here. They must not
//! read the TypeScript `FileCacheProvider` registry at `.anvil/cache/index.json`.

use std::path::Path;

use serde::Deserialize;

/// Project-relative path of the append-only gate history artefact.
pub(crate) const GATE_HISTORY_REL: &str = ".anvil/gate-history.ndjson";

/// One retained gate run, matching the JSON object `anvil gate` appends.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GateHistoryPoint {
    pub recorded_at: String,
    pub score: f64,
    pub status: String,
    #[serde(default)]
    pub warning_count: usize,
    #[serde(default)]
    pub duration_seconds: Option<String>,
    #[serde(default)]
    pub checks_run: Option<String>,
}

impl GateHistoryPoint {
    /// `pass` and `warn` are successful gate outcomes (warnings over blocks).
    pub(crate) fn passed(&self) -> bool {
        matches!(self.status.as_str(), "pass" | "warn")
    }

    pub(crate) fn checks_run(&self) -> usize {
        self.checks_run
            .as_deref()
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(0)
    }

    pub(crate) fn checks_passed(&self) -> usize {
        if self.passed() { self.checks_run() } else { 0 }
    }

    pub(crate) fn duration_ms(&self) -> u64 {
        let Some(seconds) = self
            .duration_seconds
            .as_deref()
            .and_then(|raw| raw.parse::<f64>().ok())
            .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        else {
            return 0;
        };
        let millis = (seconds * 1000.0).min(86_400_000.0);
        // Bounded to one day of milliseconds, so the u64 conversion cannot
        // overflow or take a negative value.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            millis.round() as u64
        }
    }

    /// `YYYY-MM-DD HH:MM` from an RFC3339 `recorded_at` (`…Z`).
    pub(crate) fn timestamp_display(&self) -> String {
        let raw = self.recorded_at.as_str();
        if raw.len() >= 16 && raw.as_bytes().get(10) == Some(&b'T') {
            format!("{} {}", &raw[..10], &raw[11..16])
        } else {
            raw.to_string()
        }
    }
}

/// Load retained gate points, newest first, truncated to `limit`.
///
/// Missing, empty, or unreadable files yield an empty vec. Malformed lines
/// are skipped so a truncated last line cannot hide earlier valid points.
#[must_use]
pub(crate) fn load_recent(root: &Path, limit: usize) -> Vec<GateHistoryPoint> {
    let path = root.join(GATE_HISTORY_REL);
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    let mut points: Vec<GateHistoryPoint> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
        .filter_map(|line| serde_json::from_slice::<GateHistoryPoint>(line).ok())
        .filter(|point| {
            matches!(point.status.as_str(), "pass" | "warn" | "fail") && point.score.is_finite()
        })
        .collect();
    points.sort_by(|left, right| right.recorded_at.cmp(&left.recorded_at));
    points.truncate(limit);
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_history(root: &Path, lines: &[&str]) {
        let dir = root.join(".anvil");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("gate-history.ndjson"), lines.join("\n") + "\n").unwrap();
    }

    #[test]
    fn missing_file_is_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(load_recent(tmp.path(), 5).is_empty());
    }

    #[test]
    fn newest_first_and_truncated() {
        let tmp = tempfile::TempDir::new().unwrap();
        write_history(
            tmp.path(),
            &[
                r#"{"recorded_at":"2026-08-17T19:41:45Z","score":100.0,"status":"pass","status_label":"PASSED","warning_count":0,"duration_seconds":"0.5","checks_run":"1"}"#,
                r#"{"recorded_at":"2026-08-18T10:00:00Z","score":80.0,"status":"fail","status_label":"FAILED","warning_count":2,"duration_seconds":"1.25","checks_run":"4"}"#,
                r#"{"recorded_at":"2026-08-16T00:00:00Z","score":90.0,"status":"warn","status_label":"WARN","warning_count":1,"duration_seconds":"2","checks_run":"3"}"#,
            ],
        );
        let points = load_recent(tmp.path(), 2);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].recorded_at, "2026-08-18T10:00:00Z");
        assert!(!points[0].passed());
        assert_eq!(points[0].checks_run(), 4);
        assert_eq!(points[0].checks_passed(), 0);
        assert_eq!(points[0].duration_ms(), 1250);
        assert_eq!(points[0].timestamp_display(), "2026-08-18 10:00");
        assert_eq!(points[1].recorded_at, "2026-08-17T19:41:45Z");
        assert!(points[1].passed());
        assert_eq!(points[1].checks_passed(), 1);
        assert_eq!(points[1].duration_ms(), 500);
    }

    #[test]
    fn skips_malformed_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        write_history(
            tmp.path(),
            &[
                "not-json",
                r#"{"recorded_at":"2026-08-17T19:41:45Z","score":100.0,"status":"pass","status_label":"PASSED","warning_count":0,"duration_seconds":"0.5","checks_run":"1"}"#,
            ],
        );
        assert_eq!(load_recent(tmp.path(), 5).len(), 1);
    }

    #[test]
    fn ignores_typescript_cache_index() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cache = tmp.path().join(".anvil/cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(
            cache.join("index.json"),
            r#"{"entries":{"gate:plan.md:1710000000":{"passed":true,"score":0.95,"checksRun":8,"checksPassed":8,"durationMs":1850}}}"#,
        )
        .unwrap();
        assert!(load_recent(tmp.path(), 5).is_empty());
    }
}
