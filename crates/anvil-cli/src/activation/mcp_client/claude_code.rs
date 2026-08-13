//! Claude Code MCP client (LAUNCH-009).
//!
//! Claude Code reads MCP config from `~/.claude.json` (user / local
//! scope) and `.mcp.json` (project / workspace scope). The file is
//! strict JSON in current Claude Code versions; the JSONC concern
//! raised in council was based on a misread of `~/.claude/settings.json`
//! (a different file). The MCP config file specifically is JSON-only —
//! verified against the Claude Code docs.
//!
//! Server entries live under the top-level `mcpServers` key, indexed by
//! a free-form server name (we use `"anvil"`).
//!
//! Reference: <https://docs.anthropic.com/en/docs/claude-code/mcp>

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::super::diagnostic::{McpClientId, McpTier};
use super::{
    AnvilEntry, ConfigCandidate, ConfigScope, DriftClass, McpClient, ParseError, ParsedConfig,
    RenderError, classify_drift_by_args, command_to_string, entries_equivalent, merge_json_mcp,
    parse_json_mcp, render_new_json_mcp,
};

const SERVER_NAME: &str = "anvil";
pub(crate) const ANVIL_MCP_ALLOW_RULE: &str = "mcp__anvil__*";

pub struct ClaudeCode;

impl McpClient for ClaudeCode {
    fn id(&self) -> McpClientId {
        McpClientId::ClaudeCode
    }

    fn config_paths(&self, workspace: &Path, home: Option<&Path>) -> Vec<ConfigCandidate> {
        let mut paths = Vec::with_capacity(2);
        // Workspace-local first. Claude Code's project-scoped MCP file
        // is `.mcp.json`; `~/.claude.json` is user/local only.
        paths.push(ConfigCandidate {
            path: workspace.join(".mcp.json"),
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
        parse_json_mcp(raw, SERVER_NAME)
    }

    fn classify_drift(&self, parsed: &ParsedConfig, fresh: &AnvilEntry) -> DriftClass {
        let Some(existing) = parsed.existing_entry.as_ref() else {
            return DriftClass::NotPresent;
        };
        let fresh_value = match build_entry(fresh) {
            Ok(v) => v,
            Err(e) => {
                return DriftClass::UnsafeDrift {
                    reason: format!("could not build fresh entry: {e}"),
                };
            }
        };
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
        if entries_equivalent(existing, &fresh_value) {
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
fn build_entry(fresh: &AnvilEntry) -> Result<Value, RenderError> {
    match fresh {
        AnvilEntry::Stdio { command, args, env } => {
            let cmd = command_to_string(command)?;
            Ok(json!({
                "type": "stdio",
                "command": cmd,
                "args": args,
                "env": env,
            }))
        }
    }
}

/// Claude Code keeps project MCP servers in `.mcp.json` and user/local
/// servers in `~/.claude.json`. Permission rules live in
/// `.claude/settings.json` next to the MCP file's parent directory.
pub(crate) fn settings_path_for_mcp_config(mcp_config_path: &Path) -> PathBuf {
    mcp_config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".claude")
        .join("settings.json")
}

pub(crate) fn render_settings_with_anvil_allow(
    existing: Option<&str>,
) -> Result<String, RenderError> {
    let mut root = match existing {
        // An empty or whitespace-only file carries no settings to preserve, so
        // treat it the same as a missing file (start from a fresh object)
        // rather than failing the whole install with BadRoot. A bare empty
        // `settings.json` is a common placeholder (interrupted write, another
        // tool's `touch`); rejecting it used to flip activation to a sticky
        // `state: error` on every run (Council M1).
        Some(raw) if raw.trim_start_matches('\u{feff}').trim().is_empty() => {
            Value::Object(serde_json::Map::new())
        }
        Some(raw) => {
            let trimmed = raw.trim_start_matches('\u{feff}').trim();
            serde_json::from_str::<Value>(trimmed)
                .map_err(|e| RenderError::BadSettingsJson(e.to_string()))?
        }
        None => Value::Object(serde_json::Map::new()),
    };

    let obj = root.as_object_mut().ok_or(RenderError::BadRoot)?;
    let permissions = obj
        .entry("permissions".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let permissions_obj = permissions
        .as_object_mut()
        .ok_or(RenderError::BadPermissionsKey)?;
    let allow = permissions_obj
        .entry("allow".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let allow_arr = allow.as_array_mut().ok_or(RenderError::BadAllowKey)?;
    if allow_arr.iter().any(|rule| !rule.is_string()) {
        return Err(RenderError::BadAllowKey);
    }
    if !allow_arr
        .iter()
        .any(|rule| rule.as_str() == Some(ANVIL_MCP_ALLOW_RULE))
    {
        allow_arr.push(json!(ANVIL_MCP_ALLOW_RULE));
    }

    serde_json::to_string_pretty(&root).map_err(|e| RenderError::Serialise(e.to_string()))
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
        let paths = ClaudeCode.config_paths(&ws, Some(&home));
        assert_eq!(paths.len(), 2);
        // Claude Code project scope is `.mcp.json`; user/local scope is
        // `~/.claude.json` (https://docs.anthropic.com/en/docs/claude-code/mcp).
        assert_eq!(paths[0].path, PathBuf::from("/repo/.mcp.json"));
        assert_eq!(paths[0].scope, ConfigScope::Workspace);
        assert_eq!(paths[1].path, PathBuf::from("/home/u/.claude.json"));
        assert_eq!(paths[1].scope, ConfigScope::Global);
    }

    #[test]
    fn parse_invalid_json_returns_invalid_error() {
        let err = ClaudeCode.parse("{not valid").unwrap_err();
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn parse_with_anvil_entry_includes_type_stdio() {
        let raw = r#"{"mcpServers": {"anvil": {"type": "stdio", "command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let parsed = ClaudeCode.parse(raw).unwrap();
        let existing = parsed.existing_entry.as_ref().unwrap();
        assert_eq!(existing.get("type"), Some(&json!("stdio")));
    }

    #[test]
    fn classify_drift_no_existing_is_not_present() {
        let parsed = ClaudeCode.parse(r#"{}"#).unwrap();
        assert_eq!(
            ClaudeCode.classify_drift(&parsed, &fresh()),
            DriftClass::NotPresent
        );
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
    fn settings_merge_adds_anvil_mcp_allow_rule_and_preserves_existing_rules() {
        let raw = r#"{"permissions": {"allow": ["Bash(pnpm test *)"], "deny": ["Read(.env)"]}, "theme": "dark"}"#;
        let rendered = render_settings_with_anvil_allow(Some(raw)).unwrap();
        let v: Value = serde_json::from_str(&rendered).unwrap();
        let allow = v
            .get("permissions")
            .and_then(|p| p.get("allow"))
            .and_then(Value::as_array)
            .unwrap();
        assert!(allow.contains(&json!("Bash(pnpm test *)")));
        assert!(allow.contains(&json!("mcp__anvil__*")));
        assert_eq!(
            v.get("permissions").unwrap().get("deny"),
            Some(&json!(["Read(.env)"]))
        );
        assert_eq!(v.get("theme"), Some(&json!("dark")));
    }

    #[test]
    fn settings_merge_is_idempotent_for_anvil_mcp_allow_rule() {
        let raw = r#"{"permissions": {"allow": ["mcp__anvil__*"]}}"#;
        let rendered = render_settings_with_anvil_allow(Some(raw)).unwrap();
        let v: Value = serde_json::from_str(&rendered).unwrap();
        let allow = v
            .get("permissions")
            .and_then(|p| p.get("allow"))
            .and_then(Value::as_array)
            .unwrap();
        assert_eq!(allow, &[json!("mcp__anvil__*")]);
    }

    #[test]
    fn settings_merge_treats_empty_or_whitespace_file_as_absent() {
        // An empty/whitespace placeholder must not fail the install (Council M1):
        // it should be treated like a missing file and seeded with the anvil rule.
        for raw in ["", "   ", "\n\t\n", "\u{feff}", "\u{feff}  \n"] {
            let rendered = render_settings_with_anvil_allow(Some(raw))
                .unwrap_or_else(|e| panic!("empty input {raw:?} should render Ok, got {e:?}"));
            let v: Value = serde_json::from_str(&rendered).unwrap();
            let allow = v
                .get("permissions")
                .and_then(|p| p.get("allow"))
                .and_then(Value::as_array)
                .unwrap();
            assert_eq!(allow, &[json!("mcp__anvil__*")], "input {raw:?}");
        }
    }

    #[test]
    fn settings_path_sits_under_claude_directory_next_to_mcp_config() {
        let path = settings_path_for_mcp_config(Path::new("/home/u/.claude.json"));
        assert_eq!(path, PathBuf::from("/home/u/.claude/settings.json"));
    }

    #[test]
    fn settings_path_for_workspace_mcp_json_is_project_settings() {
        let path = settings_path_for_mcp_config(Path::new("/repo/.mcp.json"));
        assert_eq!(path, PathBuf::from("/repo/.claude/settings.json"));
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
