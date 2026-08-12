//! Managed MCP configuration adapters for the ADR-106 client registry.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use crate::activation::agent_registry::{AgentClientId, InstallScope, McpConfigKind};
use crate::util::atomic_write_nofollow;

const SERVER_NAME: &str = "anvil";
const STDIO_ARGS: &[&str] = &["mcp", "serve", "--stdio"];

#[derive(Debug)]
pub(crate) struct InstallReport {
    pub path: PathBuf,
    pub wrote: bool,
    pub changed: bool,
    pub drifted: bool,
    pub entry: Value,
    pub reload_hint: &'static str,
}

pub(crate) fn preview(client: AgentClientId, command: &str) -> Result<String> {
    let adapter = client.entry();
    let kind = adapter
        .mcp_kind
        .context("registry entry is missing an MCP config adapter")?;
    let entry = expected_entry(client, kind, command);
    match kind {
        McpConfigKind::CodexToml | McpConfigKind::GrokToml => {
            let value = json!({ "mcp_servers": { SERVER_NAME: entry } });
            Ok(format!(
                "{}\n",
                toml::to_string_pretty(&json_to_toml(value)?)?
            ))
        }
        _ => {
            let mut root = Value::Object(Map::new());
            insert_json_entry(kind, &mut root, entry)?;
            Ok(format!("{}\n", serde_json::to_string_pretty(&root)?))
        }
    }
}

pub(crate) fn install(
    client: AgentClientId,
    scope: InstallScope,
    root: &Path,
    command: &str,
    verify_only: bool,
    dry_run: bool,
) -> Result<InstallReport> {
    if command.trim().is_empty() {
        bail!("--command must not be empty");
    }

    let adapter = *client.entry();
    let Some(path) = adapter.mcp_path(scope, root) else {
        let supported = if adapter.supports_mcp(InstallScope::Project) {
            "project"
        } else {
            "global"
        };
        bail!(
            "{} does not support {}-scope MCP installation; use --scope {supported}",
            adapter.display_name,
            scope.label(),
        );
    };
    ensure_safe_target(root, &path)?;
    let kind = adapter
        .mcp_kind
        .context("registry entry is missing an MCP config adapter")?;
    let entry = expected_entry(client, kind, command);

    if verify_only {
        let actual = read_entry(kind, &path)?
            .with_context(|| format!("{} has no `{SERVER_NAME}` MCP entry", path.display()))?;
        if !entry_matches(client, kind, &actual, command) {
            bail!(
                "{} has a malformed `{SERVER_NAME}` MCP entry; expected command `{command}` with args `mcp serve --stdio`",
                path.display()
            );
        }
        return Ok(InstallReport {
            path,
            wrote: false,
            changed: false,
            drifted: false,
            entry: actual,
            reload_hint: adapter.reload_hint,
        });
    }

    let current = read_entry(kind, &path)?;
    let changed = current.as_ref() != Some(&entry);
    let drifted = current.is_some() && changed;
    if drifted
        && !current
            .as_ref()
            .is_some_and(|entry| anvil_owned(kind, entry))
    {
        bail!(
            "{} already contains a user-owned or foreign `{SERVER_NAME}` MCP entry; refusing to overwrite it",
            path.display()
        );
    }
    if dry_run || !changed {
        return Ok(InstallReport {
            path,
            wrote: false,
            changed,
            drifted,
            entry,
            reload_hint: adapter.reload_hint,
        });
    }

    // No-follow parent creation + revalidation closes the race where a
    // checked component is swapped for an outside symlink between the
    // initial ensure_safe_target and the write.
    if let Some(parent) = path.parent() {
        crate::util::create_dir_all_nofollow(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    ensure_safe_target(root, &path)?;
    let rendered = merge_document(kind, &path, entry.clone())?;
    // Parent-fd-pinned atomic write: tempfile + rename cannot follow a
    // concurrent parent-directory swap.
    atomic_write_nofollow(&path, rendered.as_bytes())
        .with_context(|| format!("writing MCP config {}", path.display()))?;

    Ok(InstallReport {
        path,
        wrote: true,
        changed: true,
        drifted,
        entry,
        reload_hint: adapter.reload_hint,
    })
}

impl InstallScope {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Project => "project",
        }
    }
}

fn expected_entry(client: AgentClientId, kind: McpConfigKind, command: &str) -> Value {
    match kind {
        McpConfigKind::OpenCodeJson => json!({
            "type": "local",
            "command": [command, "mcp", "serve", "--stdio"],
            "enabled": true,
        }),
        McpConfigKind::ZedContextServersJson => json!({
            "command": {
                "path": command,
                "args": STDIO_ARGS,
                "env": {},
            }
        }),
        _ if client == AgentClientId::CopilotCli => json!({
            "type": "stdio",
            "command": command,
            "args": STDIO_ARGS,
            "env": {},
            "tools": ["*"],
        }),
        _ if client == AgentClientId::ClaudeCode => {
            json!({
                "type": "stdio",
                "command": command,
                "args": STDIO_ARGS,
                "env": {},
            })
        }
        _ => json!({
            "command": command,
            "args": STDIO_ARGS,
            "env": {},
        }),
    }
}

fn entry_matches(client: AgentClientId, kind: McpConfigKind, entry: &Value, command: &str) -> bool {
    match kind {
        McpConfigKind::OpenCodeJson => {
            entry.get("type").and_then(Value::as_str) == Some("local")
                && entry.get("command") == Some(&json!([command, "mcp", "serve", "--stdio"]))
        }
        McpConfigKind::ZedContextServersJson => {
            entry.pointer("/command/path").and_then(Value::as_str) == Some(command)
                && entry.pointer("/command/args") == Some(&json!(STDIO_ARGS))
        }
        _ => {
            entry.get("command").and_then(Value::as_str) == Some(command)
                && entry.get("args") == Some(&json!(STDIO_ARGS))
                && (client != AgentClientId::CopilotCli
                    || entry.get("tools") == Some(&json!(["*"])))
                && match (client, entry.get("type")) {
                    (AgentClientId::ClaudeCode | AgentClientId::CopilotCli, Some(kind)) => {
                        kind.as_str() == Some("stdio")
                    }
                    (AgentClientId::ClaudeCode | AgentClientId::CopilotCli, None) => false,
                    (_, Some(kind)) => kind.as_str() == Some("stdio"),
                    (_, None) => true,
                }
        }
    }
}

fn anvil_owned(kind: McpConfigKind, entry: &Value) -> bool {
    match kind {
        McpConfigKind::OpenCodeJson => {
            let Some(command) = entry.get("command").and_then(Value::as_array) else {
                return false;
            };
            command.len() == 4
                && command
                    .first()
                    .and_then(Value::as_str)
                    .is_some_and(command_is_anvil)
                && command[1].as_str() == Some("mcp")
                && command[2].as_str() == Some("serve")
                && command[3].as_str() == Some("--stdio")
        }
        McpConfigKind::ZedContextServersJson => {
            entry
                .pointer("/command/path")
                .and_then(Value::as_str)
                .is_some_and(command_is_anvil)
                && entry.pointer("/command/args") == Some(&json!(STDIO_ARGS))
        }
        _ => {
            entry
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(command_is_anvil)
                && entry.get("args") == Some(&json!(STDIO_ARGS))
        }
    }
}

fn command_is_anvil(command: &str) -> bool {
    let normalised = command.replace('\\', "/");
    matches!(
        normalised.rsplit('/').next().map(str::to_ascii_lowercase),
        Some(name) if name == "anvil" || name == "anvil.exe"
    )
}

fn read_entry(kind: McpConfigKind, path: &Path) -> Result<Option<Value>> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) if raw.trim().is_empty() => return Ok(None),
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };

    match kind {
        McpConfigKind::CodexToml | McpConfigKind::GrokToml => {
            let parsed: toml::Value = toml::from_str(&raw).with_context(|| {
                format!(
                    "existing config at {} is not valid TOML; refusing to overwrite",
                    path.display()
                )
            })?;
            let Some(entry) = parsed
                .get("mcp_servers")
                .and_then(|value| value.get(SERVER_NAME))
            else {
                return Ok(None);
            };
            Ok(Some(serde_json::to_value(entry)?))
        }
        _ => {
            let parsed: Value = serde_json::from_str(&raw).with_context(|| {
                format!(
                    "existing config at {} is not valid JSON; refusing to overwrite",
                    path.display()
                )
            })?;
            Ok(json_entry(kind, &parsed).cloned())
        }
    }
}

fn merge_document(kind: McpConfigKind, path: &Path, entry: Value) -> Result<String> {
    match kind {
        McpConfigKind::CodexToml | McpConfigKind::GrokToml => {
            let mut root: toml::Value = match fs::read_to_string(path) {
                Ok(raw) if !raw.trim().is_empty() => toml::from_str(&raw).with_context(|| {
                    format!(
                        "existing config at {} is not valid TOML; refusing to overwrite",
                        path.display()
                    )
                })?,
                Ok(_) => toml::Value::Table(toml::map::Map::new()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    toml::Value::Table(toml::map::Map::new())
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("reading {}", path.display()));
                }
            };
            let table = root
                .as_table_mut()
                .context("existing TOML config root is not a table; refusing to overwrite")?;
            let servers = table
                .entry("mcp_servers")
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
                .as_table_mut()
                .context("existing `mcp_servers` is not a table; refusing to overwrite")?;
            servers.insert(SERVER_NAME.to_string(), json_to_toml(entry)?);
            Ok(format!("{}\n", toml::to_string_pretty(&root)?))
        }
        _ => {
            let mut root: Value = match fs::read_to_string(path) {
                Ok(raw) if !raw.trim().is_empty() => {
                    serde_json::from_str(&raw).with_context(|| {
                        format!(
                            "existing config at {} is not valid JSON; refusing to overwrite",
                            path.display()
                        )
                    })?
                }
                Ok(_) => Value::Object(Map::new()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Value::Object(Map::new())
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("reading {}", path.display()));
                }
            };
            insert_json_entry(kind, &mut root, entry)?;
            Ok(format!("{}\n", serde_json::to_string_pretty(&root)?))
        }
    }
}

fn json_entry(kind: McpConfigKind, root: &Value) -> Option<&Value> {
    match kind {
        McpConfigKind::McpServersJson => root.pointer("/mcpServers/anvil"),
        McpConfigKind::ServersJson => root.pointer("/servers/anvil"),
        McpConfigKind::OpenCodeJson => root.pointer("/mcp/anvil"),
        McpConfigKind::ZedContextServersJson => root.pointer("/context_servers/anvil"),
        McpConfigKind::OpenClawJson => root.pointer("/mcp/servers/anvil"),
        McpConfigKind::CodexToml | McpConfigKind::GrokToml => None,
    }
}

fn insert_json_entry(kind: McpConfigKind, root: &mut Value, entry: Value) -> Result<()> {
    let keys: &[&str] = match kind {
        McpConfigKind::McpServersJson => &["mcpServers"],
        McpConfigKind::ServersJson => &["servers"],
        McpConfigKind::OpenCodeJson => &["mcp"],
        McpConfigKind::ZedContextServersJson => &["context_servers"],
        McpConfigKind::OpenClawJson => &["mcp", "servers"],
        McpConfigKind::CodexToml | McpConfigKind::GrokToml => unreachable!(),
    };
    let mut cursor = root
        .as_object_mut()
        .context("existing JSON config root is not an object; refusing to overwrite")?;
    for key in keys {
        let child = cursor
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        cursor = child
            .as_object_mut()
            .with_context(|| format!("existing `{key}` is not an object; refusing to overwrite"))?;
    }
    cursor.insert(SERVER_NAME.to_string(), entry);
    Ok(())
}

fn json_to_toml(value: Value) -> Result<toml::Value> {
    toml::Value::try_from(value).context("converting MCP entry to TOML")
}

fn ensure_safe_target(root: &Path, path: &Path) -> Result<()> {
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut cursor = root.to_path_buf();
    let relative = path.strip_prefix(root).with_context(|| {
        format!(
            "MCP target {} is outside selected root {}",
            path.display(),
            root.display()
        )
    })?;
    for component in relative.components() {
        cursor.push(component);
        let Ok(metadata) = fs::symlink_metadata(&cursor) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            let resolved = cursor.canonicalize().with_context(|| {
                format!("resolving symlinked MCP config path {}", cursor.display())
            })?;
            if !resolved.starts_with(&canonical_root) {
                bail!(
                    "refusing MCP config path through symlink outside selected root: {} -> {}",
                    cursor.display(),
                    resolved.display()
                );
            }
        }
    }
    Ok(())
}
