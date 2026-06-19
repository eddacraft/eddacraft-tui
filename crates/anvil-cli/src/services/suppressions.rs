//! Shared loader for `.anvil/suppressions.json`.
//!
//! Reads the suppression store, filtering out expired and malformed entries.
//! Consumed by `anvil export` (constraint bundles) and the `anvil dashboard
//! suppressions` surface (TDASH-004).

use std::path::Path;

use serde::Serialize;

/// One active suppression, projected for serialization and display.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuppressionEntry {
    pub(crate) pattern_id: String,
    pub(crate) file: String,
    pub(crate) scope: String,
    pub(crate) reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) expires_at: Option<String>,
}

#[derive(serde::Deserialize)]
struct SuppressionStore {
    suppressions: Vec<RawSuppression>,
}

#[derive(serde::Deserialize)]
struct RawSuppression {
    pattern_id: String,
    file: String,
    scope: String,
    reason: String,
    expires_at: Option<String>,
}

/// The active suppressions plus the active/expired counts over the whole store.
///
/// `anvil export` and the dashboard only need the active set, but the
/// `anvil://suppressions` MCP resource surfaces the totals too (RMCPF-020), so
/// the report keeps the expired tally that `load_suppressions` discards.
#[derive(Debug, Clone)]
pub(crate) struct SuppressionsReport {
    /// Active (unexpired, well-formed expiry) suppressions.
    pub(crate) active: Vec<SuppressionEntry>,
    /// Total entries in the store (active + expired/malformed).
    pub(crate) total: usize,
    /// Entries dropped because their `expires_at` is in the past or malformed.
    pub(crate) expired: usize,
}

/// Load active suppressions from `.anvil/suppressions.json`.
///
/// Returns an empty vec when the file is absent or unparseable, and filters out
/// entries whose `expires_at` is in the past or malformed (treated as expired).
pub(crate) fn load_suppressions(workspace_root: &Path) -> Vec<SuppressionEntry> {
    load_suppressions_report(workspace_root).active
}

/// Load the suppression store as an active-plus-counts [`SuppressionsReport`].
///
/// Same parse/expiry semantics as [`load_suppressions`] (absent/unparseable →
/// an empty report; past or malformed `expires_at` counts as expired), but it
/// retains the total and expired counts the active-only loader discards.
pub(crate) fn load_suppressions_report(workspace_root: &Path) -> SuppressionsReport {
    let path = workspace_root.join(".anvil").join("suppressions.json");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return SuppressionsReport {
            active: Vec::new(),
            total: 0,
            expired: 0,
        };
    };

    let store: SuppressionStore = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("warning: could not parse {}: {e}", path.display());
            return SuppressionsReport {
                active: Vec::new(),
                total: 0,
                expired: 0,
            };
        }
    };

    let now = chrono::Utc::now();
    let total = store.suppressions.len();
    let active: Vec<SuppressionEntry> = store
        .suppressions
        .into_iter()
        .filter(|s| match s.expires_at.as_ref() {
            None => true,
            Some(exp) => chrono::DateTime::parse_from_rfc3339(exp)
                .is_ok_and(|d| d.with_timezone(&chrono::Utc) >= now),
        })
        .map(|s| SuppressionEntry {
            pattern_id: s.pattern_id,
            file: s.file,
            scope: s.scope,
            reason: s.reason,
            expires_at: s.expires_at,
        })
        .collect();
    let expired = total - active.len();

    SuppressionsReport {
        active,
        total,
        expired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_suppressions_filters_expired_and_malformed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("suppressions.json"),
            r#"{
                "version": 1,
                "suppressions": [
                    { "pattern_id": "AP-001", "file": "a.ts", "scope": "file", "reason": "active", "expires_at": "2099-12-31T00:00:00Z" },
                    { "pattern_id": "AP-002", "file": "b.ts", "scope": "file", "reason": "expired", "expires_at": "2020-01-01T00:00:00Z" },
                    { "pattern_id": "AP-003", "file": "c.ts", "scope": "file", "reason": "malformed date", "expires_at": "not-a-date" },
                    { "pattern_id": "AP-004", "file": "d.ts", "scope": "file", "reason": "no expiry" }
                ],
                "lastUpdated": "2026-01-01T00:00:00Z"
            }"#,
        )
        .unwrap();

        let result = load_suppressions(tmp.path());
        let ids: Vec<&str> = result.iter().map(|s| s.pattern_id.as_str()).collect();
        // Active (future expiry) and no-expiry should be kept; expired and malformed should be dropped.
        assert!(ids.contains(&"AP-001"), "active suppression should be kept");
        assert!(
            ids.contains(&"AP-004"),
            "no-expiry suppression should be kept"
        );
        assert!(
            !ids.contains(&"AP-002"),
            "expired suppression should be filtered"
        );
        assert!(
            !ids.contains(&"AP-003"),
            "malformed date should be treated as expired"
        );
    }

    #[test]
    fn invalid_json_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(anvil_dir.join("suppressions.json"), "not json at all").unwrap();
        assert!(load_suppressions(tmp.path()).is_empty());
    }

    #[test]
    fn absent_file_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(load_suppressions(tmp.path()).is_empty());
    }
}
