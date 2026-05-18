//! AI-tool detection consumer (ADOPT-003, `anvil-run` half).
//!
//! `anvil start` writes the detected AI-tool inventory to
//! `<repo>/.anvil/cache/detected-agents.json`. `anvil-run` reads that
//! cache to cross-reference the `--tool` flag the caller supplied
//! against what was actually visible on the host the last time
//! activation ran. The cache is **advisory** — a missing or stale
//! file is not an error, and `anvil-run` never gates a launch on its
//! contents. The matching cache writer lives at
//! `crates/anvil-cli/src/activation/detect_agents.rs`; the JSON
//! shape is the `AgentInventory` serde representation pinned by the
//! writer crate's `inventory_round_trips_through_json` test.
//!
//! The consumer is deliberately type-light: it returns the
//! kebab-case `kind` ids as `String` so it does not have to depend
//! on (or duplicate) the writer's `AgentKind` enum. The wire
//! contract that holds the two sides together is the kebab-case id
//! pinned by the writer's
//! `agent_kind_id_matches_serde_representation` test.

use std::path::{Path, PathBuf};

/// Path to the cache file written by `anvil start`. Exposed for the
/// rare consumer that wants to surface the path itself (e.g. in a
/// log line); detection consumers should prefer
/// [`load_cached_agent_ids`] which handles missing / malformed
/// cases.
#[must_use]
pub fn cache_path(root: &Path) -> PathBuf {
    root.join(".anvil")
        .join("cache")
        .join("detected-agents.json")
}

/// Read the cached agent-id list from
/// `<root>/.anvil/cache/detected-agents.json`. Returns `None` if the
/// cache is absent or unparseable; returns `Some(vec![])` if the
/// cache parses but lists no detected agents (i.e. the host had no
/// AI tooling visible at last activation).
///
/// Order matches the writer (`AgentKind::all()` order); duplicates
/// are not deduplicated because the writer does not produce them.
#[must_use]
pub fn load_cached_agent_ids(root: &Path) -> Option<Vec<String>> {
    let path = cache_path(root);
    let text = std::fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let detected = value.get("detected")?.as_array()?;
    let mut ids = Vec::with_capacity(detected.len());
    for entry in detected {
        let id = entry.get("kind")?.as_str()?;
        ids.push(id.to_string());
    }
    Some(ids)
}

/// `true` if the cache lists an agent with the given kebab-case id.
/// Returns `false` when the cache is missing or malformed — callers
/// that need to distinguish "absent" from "present-and-not-listed"
/// should use [`load_cached_agent_ids`] directly.
#[must_use]
pub fn cache_lists_agent(root: &Path, agent_id: &str) -> bool {
    load_cached_agent_ids(root).is_some_and(|ids| ids.iter().any(|id| id == agent_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_cache(root: &Path, body: &str) {
        let path = cache_path(root);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
    }

    #[test]
    fn returns_none_when_cache_file_is_absent() {
        let tmp = TempDir::new().unwrap();
        assert!(load_cached_agent_ids(tmp.path()).is_none());
    }

    #[test]
    fn returns_none_when_cache_is_not_valid_json() {
        let tmp = TempDir::new().unwrap();
        write_cache(tmp.path(), "{not json");
        assert!(load_cached_agent_ids(tmp.path()).is_none());
    }

    #[test]
    fn returns_none_when_cache_is_missing_detected_field() {
        let tmp = TempDir::new().unwrap();
        write_cache(tmp.path(), r#"{"unrelated": []}"#);
        assert!(load_cached_agent_ids(tmp.path()).is_none());
    }

    #[test]
    fn returns_empty_vec_when_inventory_lists_no_agents() {
        let tmp = TempDir::new().unwrap();
        write_cache(tmp.path(), r#"{"detected": []}"#);
        assert_eq!(load_cached_agent_ids(tmp.path()), Some(vec![]));
    }

    #[test]
    fn returns_agent_ids_in_writer_order() {
        // The writer emits `kind` in kebab-case; the consumer
        // returns those strings verbatim so callers can compare
        // against `--tool` values without translation.
        let tmp = TempDir::new().unwrap();
        write_cache(
            tmp.path(),
            r#"{
                "detected": [
                    {"kind": "claude-code", "evidence": []},
                    {"kind": "cursor", "evidence": []}
                ]
            }"#,
        );
        assert_eq!(
            load_cached_agent_ids(tmp.path()),
            Some(vec!["claude-code".to_string(), "cursor".to_string()])
        );
    }

    #[test]
    fn ignores_unknown_fields_for_forward_compatibility() {
        // The writer may grow new top-level or per-agent fields; the
        // consumer must keep parsing the agent ids it understands.
        let tmp = TempDir::new().unwrap();
        write_cache(
            tmp.path(),
            r#"{
                "schemaVersion": 2,
                "detected": [
                    {"kind": "aider", "evidence": [], "futureField": 1}
                ],
                "futureTop": "ok"
            }"#,
        );
        assert_eq!(
            load_cached_agent_ids(tmp.path()),
            Some(vec!["aider".to_string()])
        );
    }

    #[test]
    fn cache_lists_agent_short_circuits_on_missing_cache() {
        let tmp = TempDir::new().unwrap();
        // No cache written → returns false rather than panicking.
        assert!(!cache_lists_agent(tmp.path(), "claude-code"));
    }

    #[test]
    fn cache_lists_agent_matches_id_present_in_inventory() {
        let tmp = TempDir::new().unwrap();
        write_cache(
            tmp.path(),
            r#"{"detected": [{"kind": "claude-code", "evidence": []}]}"#,
        );
        assert!(cache_lists_agent(tmp.path(), "claude-code"));
        assert!(!cache_lists_agent(tmp.path(), "cursor"));
    }

    #[test]
    fn cache_path_points_at_anvil_cache_detected_agents_json() {
        // Pin the relative path so the writer (anvil-cli) and the
        // consumer (anvil-run) cannot drift on the cache location.
        let tmp = TempDir::new().unwrap();
        let path = cache_path(tmp.path());
        let rel = path.strip_prefix(tmp.path()).unwrap();
        assert_eq!(rel, Path::new(".anvil/cache/detected-agents.json"));
    }
}
