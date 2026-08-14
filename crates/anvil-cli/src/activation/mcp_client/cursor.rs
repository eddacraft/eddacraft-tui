//! Cursor MCP client (LAUNCH-009).
//!
//! Cursor reads MCP config from `~/.cursor/mcp.json` (per-user) and
//! `.cursor/mcp.json` (per-workspace). Both files are strict JSON.
//! Server entries live under the top-level `mcpServers` key, indexed by
//! a free-form server name (we use `"anvil"`).
//!
//! Reference: <https://docs.cursor.com/context/model-context-protocol>

use std::path::Path;

use serde_json::{Value, json};

use super::super::diagnostic::{McpClientId, McpTier};
use super::{
    AnvilEntry, ConfigCandidate, ConfigScope, DriftClass, McpClient, ParseError, ParsedConfig,
    RenderError, classify_drift_by_args, command_to_string, entries_equivalent, merge_json_mcp,
    parse_json_mcp, render_new_json_mcp,
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
        parse_json_mcp(raw, SERVER_NAME)
    }

    fn classify_drift(&self, parsed: &ParsedConfig, fresh: &AnvilEntry) -> DriftClass {
        let Some(existing) = parsed.existing_entry.as_ref() else {
            return DriftClass::NotPresent;
        };
        // Build the fresh value once. If the path is invalid UTF-8 we
        // can't even compare, so escalate to UnsafeDrift with a clear
        // reason rather than silently mangling via to_string_lossy.
        let fresh_value = match build_entry(fresh) {
            Ok(v) => v,
            Err(e) => {
                return DriftClass::UnsafeDrift {
                    reason: format!("could not build fresh entry: {e}"),
                };
            }
        };
        // PATH-stable existing `anvil` matches an anvil-shaped fresh
        // command; absolute/versioned existing paths vs preferred
        // `anvil` are drift (MCPLH-001).
        if entries_equivalent(existing, &fresh_value) {
            return DriftClass::UpToDate;
        }
        classify_drift_by_args(existing, fresh)
    }

    fn merge_and_render(
        &self,
        parsed: &ParsedConfig,
        fresh: &AnvilEntry,
    ) -> Result<String, RenderError> {
        let entry = build_entry(fresh)?;
        merge_json_mcp(parsed, SERVER_NAME, entry)
    }

    fn render_new(&self, fresh: &AnvilEntry) -> Result<String, RenderError> {
        let entry = build_entry(fresh)?;
        render_new_json_mcp(SERVER_NAME, entry)
    }

    fn verify_config_tier(&self, parsed: Option<&ParsedConfig>, fresh: &AnvilEntry) -> McpTier {
        let Some(p) = parsed else {
            return McpTier::ConfigAbsent;
        };
        let Some(existing) = p.existing_entry.as_ref() else {
            return McpTier::ConfigAbsent;
        };
        let Ok(fresh_value) = build_entry(fresh) else {
            return McpTier::ConfigPresent;
        };
        if entries_equivalent(existing, &fresh_value)
            || matches!(
                classify_drift_by_args(existing, fresh),
                DriftClass::SafeDrift { .. }
            )
        {
            // Owned anvil-shaped entries stay RestartRequired so status
            // can handshake the *installed* command. Path drift vs
            // preferred `anvil` is still SafeDrift for install rewrite.
            McpTier::RestartRequired
        } else {
            McpTier::ConfigPresent
        }
    }

    fn restart_hint(&self) -> &'static str {
        "Quit Cursor (Cmd-Q / Ctrl-Q) and reopen so it picks up the new MCP entry."
    }
}

/// Translate an `AnvilEntry` into the JSON shape Cursor expects.
fn build_entry(fresh: &AnvilEntry) -> Result<Value, RenderError> {
    match fresh {
        AnvilEntry::Stdio { command, args, env } => {
            let cmd = command_to_string(command)?;
            Ok(json!({
                "command": cmd,
                "args": args,
                "env": env,
            }))
        }
    }
}

#[cfg(test)]
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
    fn parse_no_anvil_entry_returns_existing_none() {
        let parsed = Cursor.parse(r#"{"mcpServers": {"foo": {}}}"#).unwrap();
        assert!(parsed.existing_entry.is_none());
    }

    #[test]
    fn classify_drift_no_existing_is_not_present() {
        let parsed = Cursor.parse(r#"{}"#).unwrap();
        assert_eq!(
            Cursor.classify_drift(&parsed, &fresh()),
            DriftClass::NotPresent
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
        assert!(v.get("mcpServers").unwrap().get("other-server").is_some());
        assert!(v.get("mcpServers").unwrap().get("anvil").is_some());
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
        let raw = r#"{"mcpServers": {"anvil": {"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = Cursor.parse(raw).unwrap();
        assert_eq!(
            Cursor.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::RestartRequired
        );
    }

    #[test]
    fn verify_config_tier_owned_path_drift_is_restart_required() {
        let raw = r#"{"mcpServers": {"anvil": {"command": "/different/path/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = Cursor.parse(raw).unwrap();
        assert_eq!(
            Cursor.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::RestartRequired
        );
    }
}
