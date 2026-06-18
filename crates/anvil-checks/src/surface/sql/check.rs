//! Aggregator for the SURFSQL rule pack.
//!
//! `run_surfsql_check` is the single entry point higher-level callers reach
//! for. Discovery is the caller's job: pass the `(path, content)` pairs you
//! want considered (typically the files for which
//! [`is_sql_migration_file`](super::scanner::is_sql_migration_file) is true).
//! The aggregator stays sync and free of `io::Error`, mirroring
//! [`crate::surface::env::run_surfenv_check`].
//!
//! Phase 1 ships SURFSQL-002 (destructive-pattern catalogue). SURFSQL-003
//! (schema hygiene) and SURFSQL-006 (drift baseline) layer in here as later
//! slices per `plans/modules/surface-sql-migrations.aps.md`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::scanner::{SqlFinding, SqlHygieneFinding, scan_sql_file, scan_sql_hygiene};

/// Aggregated result of running every SURFSQL rule against a file set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SurfsqlCheckResult {
    /// SURFSQL-002 — destructive / irreversible operations.
    pub destructive: Vec<SqlFinding>,
    /// SURFSQL-003 — schema-hygiene issues (missing idempotency guards).
    pub hygiene: Vec<SqlHygieneFinding>,
}

impl SurfsqlCheckResult {
    /// Total finding count across every rule, including suppressed findings.
    #[must_use]
    pub fn total_findings(&self) -> usize {
        self.destructive.len() + self.hygiene.len()
    }

    /// Count an operator actually has to action (unsuppressed only).
    #[must_use]
    pub fn unsuppressed_findings(&self) -> usize {
        self.destructive.iter().filter(|f| !f.suppressed).count()
            + self.hygiene.iter().filter(|f| !f.suppressed).count()
    }
}

/// Run every SURFSQL rule against a set of SQL files.
///
/// `sql_files` carries the path and content of each candidate file — the
/// caller has already done discovery and the read.
#[must_use]
pub fn run_surfsql_check(sql_files: &[(PathBuf, String)]) -> SurfsqlCheckResult {
    let mut result = SurfsqlCheckResult::default();
    for (path, content) in sql_files {
        let display = path.to_string_lossy();
        result.destructive.extend(scan_sql_file(&display, content));
        result.hygiene.extend(scan_sql_hygiene(&display, content));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{SurfsqlCheckResult, run_surfsql_check};
    use crate::surface::sql::scanner::DestructiveKind;
    use std::path::PathBuf;

    #[test]
    fn empty_input_yields_default_result() {
        let result = run_surfsql_check(&[]);
        assert_eq!(result.total_findings(), 0);
        assert_eq!(result.unsuppressed_findings(), 0);
    }

    #[test]
    fn aggregates_destructive_findings_across_files() {
        let a = "DROP TABLE old_events;\n".to_string();
        let b = "DELETE FROM audit_log;\nUPDATE users SET active = true;\n".to_string();
        let files = vec![
            (PathBuf::from("db/migrations/099-drop.sql"), a),
            (PathBuf::from("db/migrations/100-bulk.sql"), b),
        ];
        let result = run_surfsql_check(&files);
        let kinds: Vec<DestructiveKind> = result.destructive.iter().map(|f| f.kind).collect();
        assert!(kinds.contains(&DestructiveKind::DropTable));
        assert!(kinds.contains(&DestructiveKind::DeleteWithoutWhere));
        assert!(kinds.contains(&DestructiveKind::UpdateWithoutWhere));
        assert_eq!(result.unsuppressed_findings(), 3);
    }

    #[test]
    fn aggregates_destructive_and_hygiene_findings() {
        // One file with a destructive op and an unguarded CREATE TABLE.
        let sql = "CREATE TABLE events (id int);\nDROP TABLE legacy;\n".to_string();
        let files = vec![(PathBuf::from("db/migrations/050.sql"), sql)];
        let result = run_surfsql_check(&files);
        assert_eq!(result.destructive.len(), 1, "DROP TABLE");
        assert_eq!(result.hygiene.len(), 1, "unguarded CREATE TABLE");
        assert_eq!(result.unsuppressed_findings(), 2);
        assert_eq!(result.total_findings(), 2);
    }

    #[test]
    fn result_round_trips_via_json() {
        let files = vec![(
            PathBuf::from("m.sql"),
            "TRUNCATE events;\nCREATE TABLE t (id int);\n".to_string(),
        )];
        let result = run_surfsql_check(&files);
        let json = serde_json::to_string(&result).expect("serialize");
        let round: SurfsqlCheckResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round.total_findings(), result.total_findings());
        assert_eq!(round.hygiene.len(), result.hygiene.len());
    }
}
