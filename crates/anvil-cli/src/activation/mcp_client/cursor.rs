//! Cursor MCP client (LAUNCH-009).
//!
//! Cursor reads MCP config from `~/.cursor/mcp.json` (per-user) and
//! `.cursor/mcp.json` (per-workspace). Both files are strict JSON.
//! Server entries live under the top-level `mcpServers` key, indexed by
//! a free-form server name (we use `"anvil"`).
//!
//! Reference: <https://docs.cursor.com/context/model-context-protocol>

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::super::diagnostic::{McpClientId, McpTier};
use super::{
    AnvilEntry, ConfigCandidate, ConfigScope, DriftClass, McpClient, ParseError, ParsedConfig,
};

/// Stable server-name key. Matches the `SERVER_NAME` constant in
/// `commands/mcp_config.rs` so the activation flow and the standalone
/// `anvil mcp-config` CLI produce drift-compatible entries.
const SERVER_NAME: &str = "anvil";

pub struct Cursor;

impl McpClient for Cursor {
    fn id(&self) -> McpClientId {
        McpClientId::Cursor
    }

    fn config_paths(&self, workspace: &Path, home: Option<&Path>) -> Vec<ConfigCandidate> {
        let mut paths = Vec::with_capacity(2);
        // Workspace-local first — Cursor honours per-repo config when
        // present, and per-repo lets users override per-user defaults.
        paths.push(ConfigCandidate {
            path: workspace.join(".cursor").join("mcp.json"),
            scope: ConfigScope::Workspace,
        });
        if let Some(h) = home {
            paths.push(ConfigCandidate {
                path: h.join(".cursor").join("mcp.json"),
                scope: ConfigScope::Global,
            });
        }
        paths
    }

    fn parse(&self, raw: &str) -> Result<ParsedConfig, ParseError> {
        let trimmed = raw.trim_start_matches('\u{feff}').trim();
        if trimmed.is_empty() {
            return Err(ParseError::Empty);
        }
        let value: Value = serde_json::from_str(trimmed)
            .map_err(|e| ParseError::Invalid(format!("JSON parse error: {e}")))?;
        if !value.is_object() {
            return Err(ParseError::UnexpectedShape(
                "top-level value must be a JSON object".to_string(),
            ));
        }
        let existing = value
            .get("mcpServers")
            .and_then(|m| m.get(SERVER_NAME))
            .cloned();
        Ok(ParsedConfig {
            raw: value,
            existing_entry: existing,
        })
    }

    fn classify_drift(&self, parsed: &ParsedConfig, fresh: &AnvilEntry) -> DriftClass {
        let Some(existing) = parsed.existing_entry.as_ref() else {
            // No existing entry — installing is not drift.
            return DriftClass::UpToDate;
        };
        let fresh_value = build_entry(fresh);
        if existing == &fresh_value {
            return DriftClass::UpToDate;
        }
        // Compare by shape: same `args` + same anvil-shaped command =
        // SafeDrift; everything else = UnsafeDrift.
        classify_existing(existing, fresh)
    }

    fn merge_and_render(
        &self,
        parsed: &ParsedConfig,
        fresh: &AnvilEntry,
    ) -> Result<String, String> {
        let mut root = parsed.raw.clone();
        let entry = build_entry(fresh);
        let obj = root
            .as_object_mut()
            .ok_or_else(|| "config root is not an object".to_string())?;
        let servers = obj
            .entry("mcpServers".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let map = servers
            .as_object_mut()
            .ok_or_else(|| "`mcpServers` is not an object".to_string())?;
        map.insert(SERVER_NAME.to_string(), entry);
        serde_json::to_string_pretty(&root).map_err(|e| format!("serialise: {e}"))
    }

    fn render_new(&self, fresh: &AnvilEntry) -> Result<String, String> {
        let mut servers = Map::new();
        servers.insert(SERVER_NAME.to_string(), build_entry(fresh));
        let mut root = Map::new();
        root.insert("mcpServers".to_string(), Value::Object(servers));
        serde_json::to_string_pretty(&Value::Object(root)).map_err(|e| format!("serialise: {e}"))
    }

    fn verify_config_tier(&self, parsed: Option<&ParsedConfig>, fresh: &AnvilEntry) -> McpTier {
        let Some(p) = parsed else {
            return McpTier::ConfigAbsent;
        };
        let Some(existing) = p.existing_entry.as_ref() else {
            return McpTier::ConfigAbsent;
        };
        if existing == &build_entry(fresh) {
            // Entry matches what we'd install — config is up to date.
            // RestartRequired is always the answer for a freshly-written
            // entry; the orchestrator can probe `ServerStartable` from
            // there.
            McpTier::RestartRequired
        } else {
            // Some anvil-shaped entry exists, but it doesn't match what
            // we'd install. Treat as ConfigPresent — drift handling is
            // separate from the tier ladder.
            McpTier::ConfigPresent
        }
    }

    fn restart_hint(&self) -> &'static str {
        "Quit Cursor (Cmd-Q / Ctrl-Q) and reopen so it picks up the new MCP entry."
    }
}

/// Translate an `AnvilEntry` into the JSON shape Cursor expects.
fn build_entry(fresh: &AnvilEntry) -> Value {
    match fresh {
        AnvilEntry::Stdio { command, args, env } => json!({
            "command": command.to_string_lossy(),
            "args": args,
            "env": env,
        }),
    }
}

/// Classify an existing non-matching entry against a freshly-built one.
/// Pure: no I/O.
fn classify_existing(existing: &Value, fresh: &AnvilEntry) -> DriftClass {
    // Pull the existing command + args; if the shape is unrecognisable,
    // it's UnsafeDrift.
    let Some(obj) = existing.as_object() else {
        return DriftClass::UnsafeDrift {
            reason: "existing entry is not an object".to_string(),
        };
    };
    let existing_args: Vec<String> = obj
        .get("args")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let existing_cmd = obj.get("command").and_then(|c| c.as_str()).unwrap_or("");

    let AnvilEntry::Stdio {
        command: fresh_cmd,
        args: fresh_args,
        ..
    } = fresh;

    if existing_args == *fresh_args {
        // Same args, different command path: classic version-upgrade
        // shape (e.g. user has nix-managed anvil at a different path).
        // Caller can re-validate the existing path before deciding.
        DriftClass::SafeDrift {
            reason: format!(
                "version drift: existing command `{existing_cmd}` differs from fresh `{}`",
                fresh_cmd.display()
            ),
        }
    } else {
        // Different args = unrecognisable, do not touch.
        DriftClass::UnsafeDrift {
            reason: format!(
                "existing entry's args do not match anvil's launch shape (existing: {existing_args:?}, fresh: {fresh_args:?})"
            ),
        }
    }
}

/// Resolve the user-global config root for Cursor.
#[allow(dead_code)] // used by orchestrator integration in a follow-up commit
pub(crate) fn home_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".cursor"))
}

#[cfg(test)]
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use super::*;

    fn fresh() -> AnvilEntry {
        AnvilEntry::local_stdio(PathBuf::from("/usr/local/bin/anvil"))
    }

    #[test]
    fn config_paths_workspace_first_then_home() {
        let ws = PathBuf::from("/repo");
        let home = PathBuf::from("/home/u");
        let paths = Cursor.config_paths(&ws, Some(&home));
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, PathBuf::from("/repo/.cursor/mcp.json"));
        assert_eq!(paths[0].scope, ConfigScope::Workspace);
        assert_eq!(paths[1].path, PathBuf::from("/home/u/.cursor/mcp.json"));
        assert_eq!(paths[1].scope, ConfigScope::Global);
    }

    #[test]
    fn parse_empty_file_returns_empty_error() {
        assert!(matches!(Cursor.parse("   "), Err(ParseError::Empty)));
    }

    #[test]
    fn parse_invalid_json_returns_invalid_error() {
        let err = Cursor.parse("{not valid").unwrap_err();
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn parse_non_object_root_returns_unexpected_shape() {
        let err = Cursor.parse("[1, 2, 3]").unwrap_err();
        assert!(matches!(err, ParseError::UnexpectedShape(_)));
    }

    #[test]
    fn parse_no_anvil_entry_returns_existing_none() {
        let parsed = Cursor.parse(r#"{"mcpServers": {"foo": {}}}"#).unwrap();
        assert!(parsed.existing_entry.is_none());
    }

    #[test]
    fn parse_with_anvil_entry_extracts_it() {
        let raw = r#"{"mcpServers": {"anvil": {"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = Cursor.parse(raw).unwrap();
        assert!(parsed.existing_entry.is_some());
    }

    #[test]
    fn classify_drift_no_existing_is_up_to_date() {
        let parsed = Cursor.parse(r#"{}"#).unwrap();
        assert_eq!(
            Cursor.classify_drift(&parsed, &fresh()),
            DriftClass::UpToDate
        );
    }

    #[test]
    fn classify_drift_matching_entry_is_up_to_date() {
        let raw = r#"{"mcpServers": {"anvil": {"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = Cursor.parse(raw).unwrap();
        assert_eq!(
            Cursor.classify_drift(&parsed, &fresh()),
            DriftClass::UpToDate
        );
    }

    #[test]
    fn classify_drift_different_command_is_safe_drift() {
        // Same args, different binary path = safe (version upgrade).
        let raw = r#"{"mcpServers": {"anvil": {"command": "/nix/store/abc/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = Cursor.parse(raw).unwrap();
        match Cursor.classify_drift(&parsed, &fresh()) {
            DriftClass::SafeDrift { reason } => {
                assert!(reason.contains("/nix/store/abc/bin/anvil"));
            }
            other => panic!("expected SafeDrift, got {other:?}"),
        }
    }

    #[test]
    fn classify_drift_different_args_is_unsafe_drift() {
        // Foreign tool using our key.
        let raw = r#"{"mcpServers": {"anvil": {"command": "/opt/foo/anvil-shim", "args": ["serve", "--port", "1234"], "env": {}}}}"#;
        let parsed = Cursor.parse(raw).unwrap();
        match Cursor.classify_drift(&parsed, &fresh()) {
            DriftClass::UnsafeDrift { reason } => {
                assert!(reason.contains("anvil's launch shape"));
            }
            other => panic!("expected UnsafeDrift, got {other:?}"),
        }
    }

    #[test]
    fn merge_and_render_preserves_unrelated_keys() {
        let raw = r#"{"mcpServers": {"other-server": {"command": "/usr/bin/other"}}, "unrelatedKey": 42}"#;
        let parsed = Cursor.parse(raw).unwrap();
        let rendered = Cursor.merge_and_render(&parsed, &fresh()).unwrap();
        let v: Value = serde_json::from_str(&rendered).unwrap();
        // Both servers present.
        assert!(v.get("mcpServers").unwrap().get("other-server").is_some());
        assert!(v.get("mcpServers").unwrap().get("anvil").is_some());
        // Unrelated key preserved.
        assert_eq!(v.get("unrelatedKey"), Some(&json!(42)));
    }

    #[test]
    fn render_new_produces_minimal_config() {
        let rendered = Cursor.render_new(&fresh()).unwrap();
        let v: Value = serde_json::from_str(&rendered).unwrap();
        assert!(v.is_object());
        assert!(v.get("mcpServers").unwrap().get("anvil").is_some());
    }

    #[test]
    fn verify_config_tier_no_parsed_is_config_absent() {
        assert_eq!(
            Cursor.verify_config_tier(None, &fresh()),
            McpTier::ConfigAbsent
        );
    }

    #[test]
    fn verify_config_tier_no_anvil_entry_is_config_absent() {
        let parsed = Cursor.parse(r#"{"mcpServers": {"other": {}}}"#).unwrap();
        assert_eq!(
            Cursor.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::ConfigAbsent
        );
    }

    #[test]
    fn verify_config_tier_matching_entry_is_restart_required() {
        // Always RestartRequired on a fresh write — we can't observe
        // restart from anvil.
        let raw = r#"{"mcpServers": {"anvil": {"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = Cursor.parse(raw).unwrap();
        assert_eq!(
            Cursor.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::RestartRequired
        );
    }

    #[test]
    fn verify_config_tier_different_anvil_entry_is_config_present() {
        // An anvil entry exists but doesn't match what we'd install.
        // Drift handling is separate from the tier ladder.
        let raw = r#"{"mcpServers": {"anvil": {"command": "/different/path/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = Cursor.parse(raw).unwrap();
        assert_eq!(
            Cursor.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::ConfigPresent
        );
    }
}
