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

const EXCEPTIONS_FILE: &str = ".anvil/exceptions.json";

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

    /// Loads exceptions from `{workspace_root}/.anvil/exceptions.json`.
    ///
    /// Returns an empty store if the file does not exist.
    pub fn load(workspace_root: &Path) -> Result<Self, ExceptionError> {
        let path = workspace_root.join(EXCEPTIONS_FILE);
        if !path.exists() {
            return Ok(Self::empty());
        }

        let content = std::fs::read_to_string(&path)?;
        let store: Self =
            serde_json::from_str(&content).map_err(|e| ExceptionError::Parse(e.to_string()))?;
        Ok(store)
    }

    /// Saves exceptions to `{workspace_root}/.anvil/exceptions.json`.
    pub fn save(&self, workspace_root: &Path) -> Result<(), ExceptionError> {
        let path = workspace_root.join(EXCEPTIONS_FILE);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            serde_json::to_string_pretty(self).map_err(|e| ExceptionError::Serialise(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
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

/// Simple glob matching supporting `*` (any segment) and `**` (any depth).
///
/// This is intentionally minimal — covers the most common patterns without
/// pulling in a full glob crate.
fn glob_matches(pattern: &str, path: &str) -> bool {
    // Normalise separators
    let pattern = pattern.replace('\\', "/");
    let path = path.replace('\\', "/");

    glob_match_recursive(pattern.as_bytes(), path.as_bytes())
}

fn glob_match_recursive(pattern: &[u8], path: &[u8]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }

    // Handle ** (matches zero or more path segments)
    if pattern.starts_with(b"**") {
        let rest = if pattern.len() > 2 && pattern[2] == b'/' {
            &pattern[3..]
        } else {
            &pattern[2..]
        };

        // ** at end of pattern matches everything
        if rest.is_empty() {
            return true;
        }

        // Try matching rest against every suffix of path
        if glob_match_recursive(rest, path) {
            return true;
        }
        for i in 0..path.len() {
            if path[i] == b'/' && glob_match_recursive(rest, &path[i + 1..]) {
                return true;
            }
        }
        return false;
    }

    // Handle * (matches any characters except /)
    if pattern[0] == b'*' {
        let rest = &pattern[1..];
        // Try matching rest starting from current position through to next /
        for i in 0..=path.len() {
            if i > 0 && path[i - 1] == b'/' {
                break;
            }
            if glob_match_recursive(rest, &path[i..]) {
                return true;
            }
        }
        return false;
    }

    // Handle ? (matches any single character except /)
    if pattern[0] == b'?' {
        if path.is_empty() || path[0] == b'/' {
            return false;
        }
        return glob_match_recursive(&pattern[1..], &path[1..]);
    }

    // Literal match
    if path.is_empty() || pattern[0] != path[0] {
        return false;
    }
    glob_match_recursive(&pattern[1..], &path[1..])
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
}
