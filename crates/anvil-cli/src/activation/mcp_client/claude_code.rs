//! Claude Code MCP client (LAUNCH-009).
//!
//! Claude Code reads MCP config from `~/.claude.json` (per-user) and
//! `.claude.json` (per-workspace). The file is strict JSON in current
//! Claude Code versions; the JSONC concern raised in council was based
//! on a misread of `~/.claude/settings.json` (a different file). The
//! MCP config file specifically is JSON-only — verified against the
//! Claude Code 0.x docs.
//!
//! Server entries live under the top-level `mcpServers` key, indexed by
//! a free-form server name (we use `"anvil"`).
//!
//! Reference: <https://docs.anthropic.com/en/docs/claude-code/mcp>

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::super::diagnostic::{McpClientId, McpTier};
use super::{
    AnvilEntry, ConfigCandidate, ConfigScope, DriftClass, McpClient, ParseError, ParsedConfig,
};

const SERVER_NAME: &str = "anvil";

pub struct ClaudeCode;

impl McpClient for ClaudeCode {
    fn id(&self) -> McpClientId {
        McpClientId::ClaudeCode
    }

    fn config_paths(&self, workspace: &Path, home: Option<&Path>) -> Vec<ConfigCandidate> {
        let mut paths = Vec::with_capacity(2);
        // Workspace-local first.
        paths.push(ConfigCandidate {
            path: workspace.join(".claude.json"),
            scope: ConfigScope::Workspace,
        });
        if let Some(h) = home {
            paths.push(ConfigCandidate {
                path: h.join(".claude.json"),
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
            return DriftClass::UpToDate;
        };
        let fresh_value = build_entry(fresh);
        if existing == &fresh_value {
            return DriftClass::UpToDate;
        }
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
            McpTier::RestartRequired
        } else {
            McpTier::ConfigPresent
        }
    }

    fn restart_hint(&self) -> &'static str {
        "Restart Claude Code (exit the CLI and re-launch) so it picks up the new MCP entry."
    }
}

/// Translate an `AnvilEntry` into the JSON shape Claude Code expects.
/// Claude Code uses the same shape as Cursor with an explicit
/// `"type": "stdio"` discriminator added — kept in lockstep with
/// `commands/mcp_config.rs::build_entry` so the activation flow and the
/// standalone `anvil mcp-config` CLI produce drift-compatible entries.
fn build_entry(fresh: &AnvilEntry) -> Value {
    match fresh {
        AnvilEntry::Stdio { command, args, env } => json!({
            "type": "stdio",
            "command": command.to_string_lossy(),
            "args": args,
            "env": env,
        }),
    }
}

fn classify_existing(existing: &Value, fresh: &AnvilEntry) -> DriftClass {
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
        DriftClass::SafeDrift {
            reason: format!(
                "version drift: existing command `{existing_cmd}` differs from fresh `{}`",
                fresh_cmd.display()
            ),
        }
    } else {
        DriftClass::UnsafeDrift {
            reason: format!(
                "existing entry's args do not match anvil's launch shape (existing: {existing_args:?}, fresh: {fresh_args:?})"
            ),
        }
    }
}

#[allow(dead_code)] // used by orchestrator integration in a follow-up commit
pub(crate) fn home_config_dir() -> Option<PathBuf> {
    dirs::home_dir()
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
        let paths = ClaudeCode.config_paths(&ws, Some(&home));
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, PathBuf::from("/repo/.claude.json"));
        assert_eq!(paths[1].path, PathBuf::from("/home/u/.claude.json"));
    }

    #[test]
    fn parse_invalid_json_returns_invalid_error() {
        let err = ClaudeCode.parse("{not valid").unwrap_err();
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn parse_with_anvil_entry_includes_type_stdio() {
        // Claude Code config format uses an explicit `type: "stdio"`
        // discriminator (different from Cursor's untyped shape).
        let raw = r#"{"mcpServers": {"anvil": {"type": "stdio", "command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = ClaudeCode.parse(raw).unwrap();
        let existing = parsed.existing_entry.as_ref().unwrap();
        assert_eq!(existing.get("type"), Some(&json!("stdio")));
    }

    #[test]
    fn classify_drift_matching_entry_is_up_to_date() {
        let raw = r#"{"mcpServers": {"anvil": {"type": "stdio", "command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = ClaudeCode.parse(raw).unwrap();
        assert_eq!(
            ClaudeCode.classify_drift(&parsed, &fresh()),
            DriftClass::UpToDate
        );
    }

    #[test]
    fn classify_drift_different_command_is_safe_drift() {
        let raw = r#"{"mcpServers": {"anvil": {"type": "stdio", "command": "/nix/store/abc/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = ClaudeCode.parse(raw).unwrap();
        assert!(matches!(
            ClaudeCode.classify_drift(&parsed, &fresh()),
            DriftClass::SafeDrift { .. }
        ));
    }

    #[test]
    fn classify_drift_different_args_is_unsafe_drift() {
        let raw = r#"{"mcpServers": {"anvil": {"type": "stdio", "command": "/opt/foo/anvil-shim", "args": ["serve"], "env": {}}}}"#;
        let parsed = ClaudeCode.parse(raw).unwrap();
        assert!(matches!(
            ClaudeCode.classify_drift(&parsed, &fresh()),
            DriftClass::UnsafeDrift { .. }
        ));
    }

    #[test]
    fn merge_and_render_preserves_unrelated_keys() {
        let raw =
            r#"{"mcpServers": {"other-server": {"type": "stdio"}}, "settings": {"theme": "dark"}}"#;
        let parsed = ClaudeCode.parse(raw).unwrap();
        let rendered = ClaudeCode.merge_and_render(&parsed, &fresh()).unwrap();
        let v: Value = serde_json::from_str(&rendered).unwrap();
        assert!(v.get("mcpServers").unwrap().get("other-server").is_some());
        assert!(v.get("mcpServers").unwrap().get("anvil").is_some());
        assert_eq!(
            v.get("settings").unwrap().get("theme"),
            Some(&json!("dark"))
        );
    }

    #[test]
    fn verify_config_tier_matching_entry_is_restart_required() {
        let raw = r#"{"mcpServers": {"anvil": {"type": "stdio", "command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = ClaudeCode.parse(raw).unwrap();
        assert_eq!(
            ClaudeCode.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::RestartRequired
        );
    }

    #[test]
    fn verify_config_tier_no_anvil_entry_is_config_absent() {
        let parsed = ClaudeCode
            .parse(r#"{"mcpServers": {"other": {}}}"#)
            .unwrap();
        assert_eq!(
            ClaudeCode.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::ConfigAbsent
        );
    }
}
