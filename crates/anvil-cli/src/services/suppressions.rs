//! Shared loader for live suppressions.
//!
//! Reads the tracked exception store (`anvil/exceptions/store.json`) and
//! inline `@anvil-ignore` directives. The TypeScript-era
//! `.anvil/suppressions.json` snapshot is not consulted.
//! Consumed by `anvil export` (constraint bundles) and the `anvil dashboard
//! suppressions` surface (TDASH-004).

use std::path::Path;

use anvil_checks::antipattern::{AntipatternCheckConfig, parse_suppression};
use anvil_policy::exceptions::ExceptionStore;
use serde::Serialize;
use walkdir::WalkDir;

use crate::util::is_ignored_dir_name;

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

/// Load active suppressions from the live exception store and `@anvil-ignore`.
pub(crate) fn load_suppressions(workspace_root: &Path) -> Vec<SuppressionEntry> {
    load_suppressions_report(workspace_root).active
}

/// Load the suppression inventory as an active-plus-counts [`SuppressionsReport`].
pub(crate) fn load_suppressions_report(workspace_root: &Path) -> SuppressionsReport {
    let mut active = Vec::new();
    let mut expired = 0usize;

    match ExceptionStore::load(workspace_root) {
        Ok(store) => {
            let now = chrono::Utc::now();
            for exception in store.exceptions {
                if exception.revoked.is_some() {
                    expired += 1;
                    continue;
                }
                if exception.expires_at.is_some_and(|exp| exp < now) {
                    expired += 1;
                    continue;
                }
                active.push(SuppressionEntry {
                    pattern_id: exception.policy_id,
                    file: exception.file_pattern,
                    scope: "exception".to_string(),
                    reason: exception.reason,
                    expires_at: exception
                        .expires_at
                        .map(|exp| exp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                });
            }
        }
        Err(error) => {
            eprintln!("warning: could not load exception store: {error}");
        }
    }

    for inline in collect_inline_ignores(workspace_root) {
        active.push(inline);
    }

    let total = active.len() + expired;
    SuppressionsReport {
        active,
        total,
        expired,
    }
}

fn collect_inline_ignores(workspace_root: &Path) -> Vec<SuppressionEntry> {
    let config = AntipatternCheckConfig::default();
    let mut entries = Vec::new();
    for entry in WalkDir::new(workspace_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir() && is_ignored_dir_name(&e.file_name().to_string_lossy()))
        })
    {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let path_str = path.to_string_lossy();
        if !config
            .extensions
            .iter()
            .any(|ext| path_str.ends_with(ext.as_str()))
        {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = path
            .strip_prefix(workspace_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for (idx, line) in content.lines().enumerate() {
            if let Some((rule, reason)) = parse_suppression(line) {
                entries.push(SuppressionEntry {
                    pattern_id: rule,
                    file: format!("{rel}:{}", idx + 1),
                    scope: "inline".to_string(),
                    reason,
                    expires_at: None,
                });
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_inline_anvil_ignore() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("smelly.ts"),
            "// @anvil-ignore AP-001: legacy\nconst x = 1;\n",
        )
        .unwrap();
        let result = load_suppressions(tmp.path());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pattern_id, "AP-001");
        assert_eq!(result[0].scope, "inline");
        assert!(result[0].file.starts_with("smelly.ts"));
    }

    #[test]
    fn loads_exception_store_and_filters_expired() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store_dir = tmp.path().join("anvil/exceptions");
        std::fs::create_dir_all(&store_dir).unwrap();
        std::fs::write(
            store_dir.join("store.json"),
            r#"{
                "exceptions": [
                    {
                        "schema_version": "anvil.exception.v1",
                        "id": "exc_active",
                        "policy_id": "AP-010",
                        "file_pattern": "src/legacy/**",
                        "reason": "still needed",
                        "created_at": "2026-01-01T00:00:00Z"
                    },
                    {
                        "schema_version": "anvil.exception.v1",
                        "id": "exc_old",
                        "policy_id": "AP-011",
                        "file_pattern": "src/gone.ts",
                        "reason": "expired",
                        "created_at": "2020-01-01T00:00:00Z",
                        "expires_at": "2020-06-01T00:00:00Z"
                    }
                ]
            }"#,
        )
        .unwrap();
        let result = load_suppressions(tmp.path());
        let ids: Vec<&str> = result.iter().map(|s| s.pattern_id.as_str()).collect();
        assert!(ids.contains(&"AP-010"));
        assert!(!ids.contains(&"AP-011"));
    }

    #[test]
    fn typescript_suppressions_json_is_ignored() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("suppressions.json"),
            r#"{ "suppressions": [{ "pattern_id": "AP-001", "file": "a.ts", "scope": "file", "reason": "stale store" }] }"#,
        )
        .unwrap();
        assert!(load_suppressions(tmp.path()).is_empty());
    }

    #[test]
    fn absent_sources_return_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(load_suppressions(tmp.path()).is_empty());
    }
}
