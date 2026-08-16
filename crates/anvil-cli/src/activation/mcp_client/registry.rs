//! Registry-backed MCP client adapter (CIB-343).
//!
//! Cursor and Claude Code keep specialised impls. Every other first-wave
//! `AgentClientId` is driven from `AgentClient` / `McpConfigKind`.

use std::path::Path;

use serde_json::{Value, json};

use super::super::agent_registry::{AgentClientId, InstallScope, McpConfigKind};
use super::super::diagnostic::{McpClientId, McpTier};
use super::{
    AnvilEntry, ConfigCandidate, ConfigScope, DriftClass, McpClient, ParseError, ParsedConfig,
    RenderError, classify_drift_by_args, command_to_string, entries_equivalent, merge_json_at,
    merge_toml_mcp, parse_keyed_json_mcp, parse_toml_mcp, render_new_json_at, render_new_toml_mcp,
    stdio_entry_from_value,
};

const SERVER_NAME: &str = "anvil";

#[derive(Clone, Copy)]
pub(crate) struct RegistryClient {
    id: AgentClientId,
}

impl RegistryClient {
    const fn new(id: AgentClientId) -> Self {
        Self { id }
    }

    fn kind(self) -> McpConfigKind {
        self.id
            .entry()
            .mcp_kind
            .expect("registry-backed handshake clients always declare mcp_kind")
    }
}

pub(crate) static CODEX: RegistryClient = RegistryClient::new(AgentClientId::Codex);
pub(crate) static OPEN_CODE: RegistryClient = RegistryClient::new(AgentClientId::OpenCode);
pub(crate) static GEMINI_CLI: RegistryClient = RegistryClient::new(AgentClientId::GeminiCli);
pub(crate) static ANTIGRAVITY: RegistryClient = RegistryClient::new(AgentClientId::Antigravity);
pub(crate) static OPEN_CLAW: RegistryClient = RegistryClient::new(AgentClientId::OpenClaw);
pub(crate) static VS_CODE: RegistryClient = RegistryClient::new(AgentClientId::VsCode);
pub(crate) static COPILOT_CLI: RegistryClient = RegistryClient::new(AgentClientId::CopilotCli);
pub(crate) static GROK: RegistryClient = RegistryClient::new(AgentClientId::Grok);
pub(crate) static WARP: RegistryClient = RegistryClient::new(AgentClientId::Warp);
pub(crate) static ZED: RegistryClient = RegistryClient::new(AgentClientId::Zed);

impl McpClient for RegistryClient {
    fn id(&self) -> McpClientId {
        self.id
    }

    fn config_paths(&self, workspace: &Path, home: Option<&Path>) -> Vec<ConfigCandidate> {
        let entry = *self.id.entry();
        let mut paths = Vec::with_capacity(2);
        if let Some(path) = entry.mcp_path(InstallScope::Project, workspace) {
            paths.push(ConfigCandidate {
                path,
                scope: ConfigScope::Workspace,
            });
        }
        if let Some(home) = home
            && let Some(path) = entry.mcp_path(InstallScope::Global, home)
            && paths.iter().all(|candidate| candidate.path != path)
        {
            paths.push(ConfigCandidate {
                path,
                scope: ConfigScope::Global,
            });
        }
        paths
    }

    fn parse(&self, raw: &str) -> Result<ParsedConfig, ParseError> {
        let kind = self.kind();
        if let Some(table) = kind.toml_servers_table() {
            parse_toml_mcp(raw, table, SERVER_NAME)
        } else if let Some(keys) = kind.json_object_path() {
            parse_keyed_json_mcp(raw, keys, SERVER_NAME)
        } else {
            Err(ParseError::UnexpectedShape(format!(
                "{} has no MCP config adapter",
                self.id.display_name()
            )))
        }
    }

    fn classify_drift(&self, parsed: &ParsedConfig, fresh: &AnvilEntry) -> DriftClass {
        let Some(existing) = parsed.existing_entry.as_ref() else {
            return DriftClass::NotPresent;
        };
        let Some(existing) = canonical_entry(self.kind(), existing) else {
            return DriftClass::UnsafeDrift {
                reason: "existing entry is not a recognised anvil stdio shape".to_string(),
            };
        };
        let fresh_value = match fresh_canonical(fresh) {
            Ok(value) => value,
            Err(error) => {
                return DriftClass::UnsafeDrift {
                    reason: format!("could not build fresh entry: {error}"),
                };
            }
        };
        if entries_equivalent(&existing, &fresh_value) {
            return DriftClass::UpToDate;
        }
        classify_drift_by_args(&existing, fresh)
    }

    fn merge_and_render(
        &self,
        parsed: &ParsedConfig,
        fresh: &AnvilEntry,
    ) -> Result<String, RenderError> {
        let kind = self.kind();
        let entry = build_entry(self.id, kind, fresh)?;
        if let Some(table) = kind.toml_servers_table() {
            merge_toml_mcp(parsed, table, SERVER_NAME, entry)
        } else if let Some(keys) = kind.json_object_path() {
            merge_json_at(parsed, keys, SERVER_NAME, entry)
        } else {
            Err(RenderError::Serialise(format!(
                "{} has no MCP config adapter",
                self.id.display_name()
            )))
        }
    }

    fn render_new(&self, fresh: &AnvilEntry) -> Result<String, RenderError> {
        let kind = self.kind();
        let entry = build_entry(self.id, kind, fresh)?;
        if let Some(table) = kind.toml_servers_table() {
            render_new_toml_mcp(table, SERVER_NAME, entry)
        } else if let Some(keys) = kind.json_object_path() {
            render_new_json_at(keys, SERVER_NAME, entry)
        } else {
            Err(RenderError::Serialise(format!(
                "{} has no MCP config adapter",
                self.id.display_name()
            )))
        }
    }

    fn verify_config_tier(&self, parsed: Option<&ParsedConfig>, fresh: &AnvilEntry) -> McpTier {
        let Some(parsed) = parsed else {
            return McpTier::ConfigAbsent;
        };
        let Some(existing) = parsed.existing_entry.as_ref() else {
            return McpTier::ConfigAbsent;
        };
        let Some(existing) = canonical_entry(self.kind(), existing) else {
            return McpTier::ConfigPresent;
        };
        let Ok(fresh_value) = fresh_canonical(fresh) else {
            return McpTier::ConfigPresent;
        };
        if entries_equivalent(&existing, &fresh_value)
            || matches!(
                classify_drift_by_args(&existing, fresh),
                DriftClass::SafeDrift { .. }
            )
        {
            McpTier::RestartRequired
        } else {
            McpTier::ConfigPresent
        }
    }

    fn restart_hint(&self) -> &'static str {
        self.id.entry().reload_hint
    }

    fn installed_stdio_entry(&self, parsed: &ParsedConfig) -> Option<AnvilEntry> {
        let existing = parsed.existing_entry.as_ref()?;
        let canonical = canonical_entry(self.kind(), existing)?;
        stdio_entry_from_value(&canonical)
    }
}

fn build_entry(
    id: AgentClientId,
    kind: McpConfigKind,
    fresh: &AnvilEntry,
) -> Result<Value, RenderError> {
    let AnvilEntry::Stdio { command, args, env } = fresh;
    let cmd = command_to_string(command)?;
    match kind {
        McpConfigKind::OpenCodeJson => {
            let mut command = Vec::with_capacity(1 + args.len());
            command.push(json!(cmd));
            command.extend(args.iter().cloned().map(Value::from));
            Ok(json!({
                "type": "local",
                "command": command,
                "enabled": true,
            }))
        }
        McpConfigKind::ZedContextServersJson => Ok(json!({
            "command": {
                "path": cmd,
                "args": args,
                "env": env,
            }
        })),
        _ if id == AgentClientId::CopilotCli => Ok(json!({
            "type": "stdio",
            "command": cmd,
            "args": args,
            "env": env,
            "tools": ["*"],
        })),
        _ => Ok(json!({
            "command": cmd,
            "args": args,
            "env": env,
        })),
    }
}

fn fresh_canonical(fresh: &AnvilEntry) -> Result<Value, RenderError> {
    let AnvilEntry::Stdio { command, args, env } = fresh;
    Ok(json!({
        "command": command_to_string(command)?,
        "args": args,
        "env": env,
    }))
}

fn canonical_entry(kind: McpConfigKind, entry: &Value) -> Option<Value> {
    match kind {
        McpConfigKind::OpenCodeJson => {
            let command = entry.get("command")?.as_array()?;
            let first = command.first()?.as_str()?;
            let args: Vec<Value> = command.iter().skip(1).cloned().collect();
            Some(json!({
                "command": first,
                "args": args,
                "env": {},
            }))
        }
        McpConfigKind::ZedContextServersJson => {
            let command = entry.pointer("/command/path")?.as_str()?;
            let args = entry.pointer("/command/args").cloned().unwrap_or(json!([]));
            let env = entry.pointer("/command/env").cloned().unwrap_or(json!({}));
            Some(json!({
                "command": command,
                "args": args,
                "env": env,
            }))
        }
        _ => {
            let command = entry.get("command")?.as_str()?;
            let args = entry.get("args").cloned().unwrap_or(json!([]));
            let env = entry.get("env").cloned().unwrap_or(json!({}));
            Some(json!({
                "command": command,
                "args": args,
                "env": env,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fresh() -> AnvilEntry {
        AnvilEntry::local_stdio(PathBuf::from("/usr/local/bin/anvil"))
    }

    #[test]
    fn grok_paths_are_workspace_then_home() {
        let ws = PathBuf::from("/repo");
        let home = PathBuf::from("/home/u");
        let paths = GROK.config_paths(&ws, Some(&home));
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, PathBuf::from("/repo/.grok/config.toml"));
        assert_eq!(paths[0].scope, ConfigScope::Workspace);
        assert_eq!(paths[1].path, PathBuf::from("/home/u/.grok/config.toml"));
        assert_eq!(paths[1].scope, ConfigScope::Global);
    }

    #[test]
    fn zed_is_project_scope_only() {
        let ws = PathBuf::from("/repo");
        let home = PathBuf::from("/home/u");
        let paths = ZED.config_paths(&ws, Some(&home));
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, PathBuf::from("/repo/.zed/settings.json"));
        assert_eq!(paths[0].scope, ConfigScope::Workspace);
    }

    #[test]
    fn grok_toml_matching_entry_is_restart_required() {
        let raw = r#"
[mcp_servers.anvil]
command = "/usr/local/bin/anvil"
args = ["mcp", "serve", "--stdio"]
"#;
        let parsed = GROK.parse(raw).unwrap();
        assert_eq!(
            GROK.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::RestartRequired
        );
        assert_eq!(GROK.classify_drift(&parsed, &fresh()), DriftClass::UpToDate);
    }

    #[test]
    fn vscode_servers_json_matching_entry_is_restart_required() {
        let raw = r#"{"servers":{"anvil":{"command":"/usr/local/bin/anvil","args":["mcp","serve","--stdio"],"env":{}}}}"#;
        let parsed = VS_CODE.parse(raw).unwrap();
        assert_eq!(
            VS_CODE.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::RestartRequired
        );
    }

    #[test]
    fn opencode_array_command_is_handshake_equivalent() {
        let raw = r#"{"mcp":{"anvil":{"type":"local","command":["/usr/local/bin/anvil","mcp","serve","--stdio"],"enabled":true}}}"#;
        let parsed = OPEN_CODE.parse(raw).unwrap();
        assert_eq!(
            OPEN_CODE.verify_config_tier(Some(&parsed), &fresh()),
            McpTier::RestartRequired
        );
        let entry = OPEN_CODE.installed_stdio_entry(&parsed).unwrap();
        match entry {
            AnvilEntry::Stdio { command, args, .. } => {
                assert_eq!(command, PathBuf::from("/usr/local/bin/anvil"));
                assert_eq!(args, vec!["mcp", "serve", "--stdio"]);
            }
        }
    }
}
