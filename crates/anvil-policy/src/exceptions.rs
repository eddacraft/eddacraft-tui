use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::evaluator::Violation;

/// A policy exception that suppresses matching violations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyException {
    /// The policy ID to suppress (e.g. "AP-001").
    pub policy_id: String,
    /// Glob-style file pattern (e.g. "src/legacy/**"). Empty means all files.
    #[serde(default)]
    pub file_pattern: String,
    /// Human-readable justification for the exception.
    pub reason: String,
    /// When the exception was created.
    pub created_at: DateTime<Utc>,
    /// Optional expiry — the exception is ignored after this date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExceptionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("serialisation error: {0}")]
    Serialise(String),
}

/// Tracked store path (ADR-073). Exceptions are durable governance state that
/// must travel with the repository and be visible in PR review, so they live
/// under `anvil/`, not the gitignored `.anvil/` runtime tree.
const EXCEPTIONS_FILE: &str = "anvil/exceptions/store.json";

/// Legacy local store path. Read-only fallback for repositories written before
/// the ADR-073 migration; [`ExceptionStore::save`] never writes here, and
/// [`ExceptionStore::migrate`] performs the one-time, non-destructive move.
const LEGACY_EXCEPTIONS_FILE: &str = ".anvil/exceptions.json";

/// Persistent store for policy exceptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionStore {
    pub exceptions: Vec<PolicyException>,
}

impl ExceptionStore {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            exceptions: Vec::new(),
        }
    }

    /// Loads exceptions, preferring the tracked store
    /// (`{workspace_root}/anvil/exceptions/store.json`) and falling back to the
    /// legacy local store (`{workspace_root}/.anvil/exceptions.json`) for
    /// repositories not yet migrated (ADR-073).
    ///
    /// Read-only: this never writes or migrates. Use [`Self::migrate`] for the
    /// one-time move. Returns an empty store if neither file exists.
    pub fn load(workspace_root: &Path) -> Result<Self, ExceptionError> {
        let tracked = workspace_root.join(EXCEPTIONS_FILE);
        if tracked.exists() {
            return Self::load_from(&tracked);
        }
        let legacy = workspace_root.join(LEGACY_EXCEPTIONS_FILE);
        if legacy.exists() {
            return Self::load_from(&legacy);
        }
        Ok(Self::empty())
    }

    /// Reads and parses a store from an explicit path.
    fn load_from(path: &Path) -> Result<Self, ExceptionError> {
        let content = std::fs::read_to_string(path)?;
        let store: Self =
            serde_json::from_str(&content).map_err(|e| ExceptionError::Parse(e.to_string()))?;
        Ok(store)
    }

    /// Saves exceptions to the tracked store
    /// (`{workspace_root}/anvil/exceptions/store.json`).
    ///
    /// Uses write-temp-then-rename to avoid corruption on interrupted writes.
    pub fn save(&self, workspace_root: &Path) -> Result<(), ExceptionError> {
        let path = workspace_root.join(EXCEPTIONS_FILE);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ExceptionError::Serialise(e.to_string()))?;
        atomic_write(&path, content.as_bytes())?;
        Ok(())
    }

    /// One-time, non-destructive migration of the legacy local store
    /// (`.anvil/exceptions.json`) to the tracked store
    /// (`anvil/exceptions/store.json`), per ADR-073.
    ///
    /// Copies the legacy store to the tracked path when the legacy file exists
    /// and the tracked store does not yet. Idempotent — returns `Ok(false)`
    /// when there is nothing to do. The legacy file is **left in place**;
    /// callers decide when to remove it.
    pub fn migrate(workspace_root: &Path) -> Result<bool, ExceptionError> {
        let tracked = workspace_root.join(EXCEPTIONS_FILE);
        let legacy = workspace_root.join(LEGACY_EXCEPTIONS_FILE);
        if tracked.exists() || !legacy.exists() {
            return Ok(false);
        }
        let store = Self::load_from(&legacy)?;
        store.save(workspace_root)?;
        Ok(true)
    }

    /// Returns only the exceptions that are currently active (not expired).
    #[must_use]
    pub fn active_exceptions(&self) -> Vec<&PolicyException> {
        self.active_exceptions_at(Utc::now())
    }

    /// Returns exceptions active at a specific point in time.
    #[must_use]
    pub fn active_exceptions_at(&self, now: DateTime<Utc>) -> Vec<&PolicyException> {
        self.exceptions
            .iter()
            .filter(|e| !is_expired(e, now))
            .collect()
    }

    /// Adds a new exception.
    pub fn add(&mut self, exception: PolicyException) {
        self.exceptions.push(exception);
    }

    /// Removes all exceptions for the given policy ID.
    pub fn remove_by_policy(&mut self, policy_id: &str) {
        self.exceptions.retain(|e| e.policy_id != policy_id);
    }
}

/// Atomically write `content` to `path` via a temp file + rename.
fn atomic_write(path: &Path, content: &[u8]) -> Result<(), ExceptionError> {
    use std::io::Write;

    let dir = path.parent().ok_or_else(|| {
        ExceptionError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("no parent directory for {}", path.display()),
        ))
    })?;

    let mut tmp = tempfile::Builder::new()
        .tempfile_in(dir)
        .map_err(ExceptionError::Io)?;

    tmp.write_all(content).map_err(ExceptionError::Io)?;
    tmp.flush().map_err(ExceptionError::Io)?;

    let tmp_path = tmp.into_temp_path();

    // On Windows, TempPath::persist uses std::fs::rename, which fails if the
    // destination already exists. Remove the existing file first.
    #[cfg(windows)]
    {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(ExceptionError::Io(e));
            }
        }
    }

    tmp_path
        .persist(path)
        .map_err(|e| ExceptionError::Io(e.error))?;

    Ok(())
}

/// Checks whether a violation is suppressed by any active exception.
#[must_use]
pub fn is_suppressed(violation: &Violation, exceptions: &[PolicyException]) -> bool {
    is_suppressed_at(violation, exceptions, Utc::now())
}

/// Checks suppression at a specific point in time (useful for testing).
#[must_use]
pub fn is_suppressed_at(
    violation: &Violation,
    exceptions: &[PolicyException],
    now: DateTime<Utc>,
) -> bool {
    exceptions.iter().any(|ex| {
        if is_expired(ex, now) {
            return false;
        }
        if ex.policy_id != violation.policy_id {
            return false;
        }
        if ex.file_pattern.is_empty() {
            return true;
        }
        glob_matches(&ex.file_pattern, &violation.file)
    })
}

/// Filters a list of violations, removing any that are suppressed.
#[must_use]
pub fn filter_suppressed(
    violations: Vec<Violation>,
    exceptions: &[PolicyException],
) -> Vec<Violation> {
    violations
        .into_iter()
        .filter(|v| !is_suppressed(v, exceptions))
        .collect()
}

fn is_expired(exception: &PolicyException, now: DateTime<Utc>) -> bool {
    exception.expires_at.is_some_and(|exp| now > exp)
}

/// Glob matching using the `glob` crate's `Pattern` type.
///
/// Normalises path separators to `/` before matching so patterns
/// work consistently across platforms.
fn glob_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.replace('\\', "/");
    let path = path.replace('\\', "/");

    let opts = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    match glob::Pattern::new(&pattern) {
        Ok(p) => p.matches_with(&path, opts),
        Err(e) => {
            eprintln!("warning: invalid exception glob pattern '{pattern}': {e}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_violation(policy_id: &str, file: &str) -> Violation {
        Violation {
            policy_id: policy_id.to_string(),
            file: file.to_string(),
            message: "test violation".to_string(),
            severity: "warning".to_string(),
            category: None,
            fingerprint: None,
        }
    }

    fn make_exception(policy_id: &str, file_pattern: &str) -> PolicyException {
        PolicyException {
            policy_id: policy_id.to_string(),
            file_pattern: file_pattern.to_string(),
            reason: "legacy code".to_string(),
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    #[test]
    fn suppresses_matching_policy_id() {
        let v = make_violation("AP-001", "src/foo.ts");
        let ex = make_exception("AP-001", "");
        assert!(is_suppressed(&v, &[ex]));
    }

    #[test]
    fn does_not_suppress_different_policy() {
        let v = make_violation("AP-001", "src/foo.ts");
        let ex = make_exception("AP-002", "");
        assert!(!is_suppressed(&v, &[ex]));
    }

    #[test]
    fn suppresses_with_matching_glob() {
        let v = make_violation("AP-001", "src/legacy/old.ts");
        let ex = make_exception("AP-001", "src/legacy/**");
        assert!(is_suppressed(&v, &[ex]));
    }

    #[test]
    fn does_not_suppress_non_matching_glob() {
        let v = make_violation("AP-001", "src/new/fresh.ts");
        let ex = make_exception("AP-001", "src/legacy/**");
        assert!(!is_suppressed(&v, &[ex]));
    }

    #[test]
    fn expired_exception_is_ignored() {
        let v = make_violation("AP-001", "src/foo.ts");
        let ex = PolicyException {
            policy_id: "AP-001".to_string(),
            file_pattern: String::new(),
            reason: "temporary".to_string(),
            created_at: Utc::now() - Duration::days(30),
            expires_at: Some(Utc::now() - Duration::days(1)),
        };
        assert!(!is_suppressed(&v, &[ex]));
    }

    #[test]
    fn non_expired_exception_applies() {
        let v = make_violation("AP-001", "src/foo.ts");
        let ex = PolicyException {
            policy_id: "AP-001".to_string(),
            file_pattern: String::new(),
            reason: "temporary".to_string(),
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::days(7)),
        };
        assert!(is_suppressed(&v, &[ex]));
    }

    #[test]
    fn filter_removes_suppressed() {
        let violations = vec![
            make_violation("AP-001", "src/a.ts"),
            make_violation("AP-002", "src/b.ts"),
            make_violation("AP-001", "src/c.ts"),
        ];
        let exceptions = vec![make_exception("AP-001", "")];
        let remaining = filter_suppressed(violations, &exceptions);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].policy_id, "AP-002");
    }

    #[test]
    fn glob_star_matches_single_segment() {
        assert!(glob_matches("src/*.ts", "src/foo.ts"));
        assert!(!glob_matches("src/*.ts", "src/sub/foo.ts"));
    }

    #[test]
    fn glob_doublestar_matches_depth() {
        assert!(glob_matches("src/**", "src/a/b/c.ts"));
        assert!(glob_matches("**/test.ts", "a/b/test.ts"));
        assert!(glob_matches("src/**/test.ts", "src/deep/nested/test.ts"));
    }

    #[test]
    fn glob_question_mark() {
        assert!(glob_matches("src/?.ts", "src/a.ts"));
        assert!(!glob_matches("src/?.ts", "src/ab.ts"));
    }

    #[test]
    fn store_load_save_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = ExceptionStore::empty();
        store.add(make_exception("AP-001", "src/**"));
        store.add(make_exception("AP-003", ""));

        store.save(tmp.path()).unwrap();
        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.exceptions.len(), 2);
        assert_eq!(loaded.exceptions[0].policy_id, "AP-001");
    }

    #[test]
    fn store_load_returns_empty_when_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = ExceptionStore::load(tmp.path()).unwrap();
        assert!(store.exceptions.is_empty());
    }

    #[test]
    fn store_remove_by_policy() {
        let mut store = ExceptionStore::empty();
        store.add(make_exception("AP-001", ""));
        store.add(make_exception("AP-002", ""));
        store.add(make_exception("AP-001", "src/**"));

        store.remove_by_policy("AP-001");
        assert_eq!(store.exceptions.len(), 1);
        assert_eq!(store.exceptions[0].policy_id, "AP-002");
    }

    #[test]
    fn store_active_filters_expired() {
        let mut store = ExceptionStore::empty();
        let now = Utc::now();

        store.add(PolicyException {
            policy_id: "AP-001".to_string(),
            file_pattern: String::new(),
            reason: "still valid".to_string(),
            created_at: now,
            expires_at: Some(now + Duration::days(7)),
        });
        store.add(PolicyException {
            policy_id: "AP-002".to_string(),
            file_pattern: String::new(),
            reason: "expired".to_string(),
            created_at: now - Duration::days(30),
            expires_at: Some(now - Duration::days(1)),
        });

        let active = store.active_exceptions();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].policy_id, "AP-001");
    }

    // --- ADR-073 storage-path migration (EXCEPT-001/002) ---

    fn write_store_at(root: &Path, rel: &str, store: &ExceptionStore) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let content = serde_json::to_string_pretty(store).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn save_writes_tracked_path_not_legacy() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = ExceptionStore::empty();
        store.add(make_exception("AP-001", "src/**"));
        store.save(tmp.path()).unwrap();

        assert!(tmp.path().join("anvil/exceptions/store.json").exists());
        assert!(!tmp.path().join(".anvil/exceptions.json").exists());
    }

    #[test]
    fn load_prefers_tracked_over_legacy() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut tracked = ExceptionStore::empty();
        tracked.add(make_exception("AP-TRACKED", ""));
        write_store_at(tmp.path(), "anvil/exceptions/store.json", &tracked);

        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-LEGACY", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.exceptions.len(), 1);
        assert_eq!(loaded.exceptions[0].policy_id, "AP-TRACKED");
    }

    #[test]
    fn load_falls_back_to_legacy_when_tracked_absent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-LEGACY", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.exceptions.len(), 1);
        assert_eq!(loaded.exceptions[0].policy_id, "AP-LEGACY");
    }

    #[test]
    fn migrate_copies_legacy_to_tracked_non_destructive() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-001", "src/**"));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        let migrated = ExceptionStore::migrate(tmp.path()).unwrap();
        assert!(migrated);

        // Tracked store now exists and carries the data.
        assert!(tmp.path().join("anvil/exceptions/store.json").exists());
        let loaded = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.exceptions.len(), 1);
        assert_eq!(loaded.exceptions[0].policy_id, "AP-001");

        // Legacy file is left in place (non-destructive).
        assert!(tmp.path().join(".anvil/exceptions.json").exists());
    }

    #[test]
    fn migrate_is_idempotent_and_noop_without_legacy() {
        let tmp = tempfile::TempDir::new().unwrap();

        // Nothing to migrate when no legacy store exists.
        assert!(!ExceptionStore::migrate(tmp.path()).unwrap());

        let mut legacy = ExceptionStore::empty();
        legacy.add(make_exception("AP-001", ""));
        write_store_at(tmp.path(), ".anvil/exceptions.json", &legacy);

        // First migration moves the data.
        assert!(ExceptionStore::migrate(tmp.path()).unwrap());
        // Second call is a no-op because the tracked store now exists.
        assert!(!ExceptionStore::migrate(tmp.path()).unwrap());
    }
}
