//! EVAL-004 — canonical persistence of eval results.
//!
//! Eval outcomes are stored in an Anvil-owned schema ([`EvalRecord`]) rather
//! than a framework's native format, so historical trend and evidence queries
//! survive a change of eval harness. The on-disk form is append-only NDJSON
//! (one JSON record per line) under an Anvil-managed directory — cheap to
//! append from CI and trivial to read back in chronological order.
//!
//! The record stores the *normalised* summary fields ([`EvalRunSummary`]) plus
//! provenance (run id, timestamp): no framework-specific keys leak into the
//! schema, satisfying "queryable independent of framework format".

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::port::{EvalFinding, EvalRunSummary};

/// The NDJSON history file name within the store root.
const HISTORY_FILE: &str = "history.jsonl";

/// One persisted evaluation outcome in Anvil's canonical schema. Derived from a
/// normalised [`EvalRunSummary`] plus provenance; deliberately free of any
/// framework-native field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalRecord {
    /// Caller-supplied unique id for this run (e.g. timestamp + suite).
    pub run_id: String,
    /// RFC 3339 timestamp the run was recorded at.
    pub recorded_at: String,
    pub suite: String,
    pub schema_version: String,
    pub policy: String,
    pub query: String,
    pub exit_code: i32,
    /// Denormalised verdict + counts, so trend queries need not re-walk
    /// `findings`.
    pub passed: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub findings: Vec<EvalFinding>,
}

impl EvalRecord {
    /// Build a canonical record from a normalised summary and provenance. The
    /// timestamp is supplied by the caller so persistence stays deterministic
    /// and testable.
    pub fn from_summary(
        summary: &EvalRunSummary,
        run_id: impl Into<String>,
        recorded_at: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            recorded_at: recorded_at.into(),
            suite: summary.suite.clone(),
            schema_version: summary.schema_version.clone(),
            policy: summary.policy.clone(),
            query: summary.query.clone(),
            exit_code: summary.exit_code,
            passed: summary.passed(),
            error_count: summary.error_count(),
            warning_count: summary.warning_count(),
            findings: summary.findings.clone(),
        }
    }

    /// Reconstruct an [`EvalRunSummary`] from a stored record — the read path
    /// for regression comparison against a baseline run.
    pub fn to_summary(&self) -> EvalRunSummary {
        EvalRunSummary {
            suite: self.suite.clone(),
            schema_version: self.schema_version.clone(),
            policy: self.policy.clone(),
            query: self.query.clone(),
            findings: self.findings.clone(),
            exit_code: self.exit_code,
        }
    }
}

/// Append-only NDJSON store of [`EvalRecord`]s under `root`.
pub struct EvalResultStore {
    root: PathBuf,
}

impl EvalResultStore {
    /// Open (lazily — nothing touches disk until [`append`](Self::append)) a
    /// store rooted at `root`. The CLI roots this at `<ANVIL_HOME>/eval`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn history_path(&self) -> PathBuf {
        self.root.join(HISTORY_FILE)
    }

    /// Append one record. Creates the store directory on first write and takes
    /// an exclusive advisory lock so concurrent CI writers do not interleave a
    /// line. The append is atomic: a partial write (disk full, I/O error) is
    /// rolled back by truncating to the pre-write length, so a failed append
    /// never leaves a torn line that would make the whole history unreadable.
    pub fn append(&self, record: &EvalRecord) -> Result<(), StoreError> {
        fs::create_dir_all(&self.root)?;
        let mut line = serde_json::to_string(record)?;
        line.push('\n');

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(self.history_path())?;
        file.lock_exclusive()?;
        // Under the exclusive lock no other writer can interleave, so the
        // pre-write length is a stable rollback point.
        let original_len = file.metadata().map(|m| m.len()).ok();
        let result = (&file).write_all(line.as_bytes());
        if result.is_err()
            && let Some(len) = original_len
        {
            let _ = file.set_len(len);
        }
        // Always unlock, even if the write failed.
        let _ = FileExt::unlock(&file);
        result?;
        Ok(())
    }

    /// All records in chronological (append) order. An absent history file is
    /// an empty history, not an error. Takes a shared advisory lock so a read
    /// that races a writer's exclusive lock waits for the line to land rather
    /// than observing a half-written record.
    pub fn all(&self) -> Result<Vec<EvalRecord>, StoreError> {
        let path = self.history_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&path)?;
        file.lock_shared()?;
        let result = Self::read_records(&path, &file);
        let _ = FileExt::unlock(&file);
        result
    }

    /// Parse every record from an already-opened, lock-held history file.
    fn read_records(
        path: &std::path::Path,
        file: &fs::File,
    ) -> Result<Vec<EvalRecord>, StoreError> {
        let mut records = Vec::new();
        for (idx, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record = serde_json::from_str(&line).map_err(|e| StoreError::Corrupt {
                path: path.to_path_buf(),
                line: idx + 1,
                detail: e.to_string(),
            })?;
            records.push(record);
        }
        Ok(records)
    }

    /// Every record for one suite, chronological.
    pub fn for_suite(&self, suite: &str) -> Result<Vec<EvalRecord>, StoreError> {
        Ok(self
            .all()?
            .into_iter()
            .filter(|r| r.suite == suite)
            .collect())
    }

    /// The most recent record for a suite — the baseline a new run regresses
    /// against — or `None` if the suite has no history.
    pub fn latest(&self, suite: &str) -> Result<Option<EvalRecord>, StoreError> {
        Ok(self.for_suite(suite)?.pop())
    }
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("eval store IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not serialise eval record: {0}")]
    Serialise(#[from] serde_json::Error),
    #[error("corrupt eval history at {path} line {line}: {detail}")]
    Corrupt {
        path: PathBuf,
        line: usize,
        detail: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::port::EvalSeverity;
    use tempfile::TempDir;

    fn summary(suite: &str, exit: i32) -> EvalRunSummary {
        EvalRunSummary {
            suite: suite.into(),
            schema_version: "1.0.0".into(),
            policy: "p.rego".into(),
            query: "data.anvil.findings".into(),
            findings: vec![EvalFinding {
                severity: if exit == 0 {
                    EvalSeverity::Warning
                } else {
                    EvalSeverity::Error
                },
                message: "m".into(),
                from: None,
                to: None,
                fingerprint: Some("fp".into()),
            }],
            exit_code: exit,
        }
    }

    #[test]
    fn eval_result_persistence_round_trips_a_record() {
        let dir = TempDir::new().expect("tmp");
        let store = EvalResultStore::new(dir.path());
        let record = EvalRecord::from_summary(&summary("arch", 1), "run-1", "2026-06-30T00:00:00Z");

        store.append(&record).expect("append");
        let read = store.all().expect("read");

        assert_eq!(read.len(), 1);
        assert_eq!(read[0], record);
        assert!(!read[0].passed);
        assert_eq!(read[0].error_count, 1);
    }

    #[test]
    fn eval_result_persistence_latest_returns_most_recent_for_suite() {
        let dir = TempDir::new().expect("tmp");
        let store = EvalResultStore::new(dir.path());
        store
            .append(&EvalRecord::from_summary(
                &summary("arch", 1),
                "r1",
                "2026-06-30T00:00:00Z",
            ))
            .expect("a1");
        store
            .append(&EvalRecord::from_summary(
                &summary("secrets", 0),
                "r2",
                "2026-06-30T00:01:00Z",
            ))
            .expect("a2");
        store
            .append(&EvalRecord::from_summary(
                &summary("arch", 0),
                "r3",
                "2026-06-30T00:02:00Z",
            ))
            .expect("a3");

        let latest = store.latest("arch").expect("latest").expect("some");
        assert_eq!(latest.run_id, "r3");
        assert!(latest.passed);
        assert_eq!(store.for_suite("arch").expect("for_suite").len(), 2);
        assert_eq!(store.for_suite("secrets").expect("for_suite").len(), 1);
    }

    #[test]
    fn eval_result_persistence_empty_history_is_not_an_error() {
        let dir = TempDir::new().expect("tmp");
        let store = EvalResultStore::new(dir.path());
        assert!(store.all().expect("all").is_empty());
        assert!(store.latest("anything").expect("latest").is_none());
    }

    #[test]
    fn eval_result_persistence_record_is_framework_neutral() {
        // The serialised schema must carry only canonical Anvil keys — a
        // regression guard against leaking a framework-native field.
        let record = EvalRecord::from_summary(&summary("arch", 0), "r1", "2026-06-30T00:00:00Z");
        let json: serde_json::Value = serde_json::to_value(&record).expect("ser");
        let keys: Vec<&str> = json
            .as_object()
            .expect("obj")
            .keys()
            .map(String::as_str)
            .collect();
        for required in [
            "run_id",
            "recorded_at",
            "suite",
            "schema_version",
            "exit_code",
            "passed",
            "findings",
        ] {
            assert!(keys.contains(&required), "missing canonical key {required}");
        }
    }

    #[test]
    fn eval_result_persistence_to_summary_reconstructs_run() {
        let original = summary("arch", 1);
        let record = EvalRecord::from_summary(&original, "r1", "2026-06-30T00:00:00Z");
        assert_eq!(record.to_summary(), original);
    }

    #[test]
    fn eval_result_persistence_detects_corrupt_history() {
        let dir = TempDir::new().expect("tmp");
        let store = EvalResultStore::new(dir.path());
        fs::create_dir_all(dir.path()).expect("mkdir");
        fs::write(dir.path().join(HISTORY_FILE), "{ not valid json\n").expect("write");
        let err = store.all().expect_err("corrupt");
        assert!(matches!(err, StoreError::Corrupt { line: 1, .. }));
    }
}
